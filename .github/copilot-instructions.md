# gtd-mcp-rs コーディング規約

## アーキテクチャ概要

RustでGTD（Getting Things Done）タスク管理を実装するMCP（Model Context Protocol）サーバーです。3層アーキテクチャパターンに従っています：

- **`src/main.rs`**: `mcp-attr`の宣言的マクロ（`#[mcp_server]`、`#[tool]`）を使用したMCPサーバーハンドラー
- **`src/gtd.rs`**: TOMLシリアライゼーションを持つコアドメインモデル（`Task`、`Project`、`Context`、`GtdData`）
- **`src/storage.rs`**: `gtd.toml`ストレージのファイル永続化層
- **`src/git_ops.rs`**: Git操作による自動バージョン管理層

データフロー: MCPクライアント → stdio → `GtdServerHandler` → `GtdData`（インメモリ） → `Storage` → `gtd.toml`ファイル

## ドキュメントと言語の規約

**重要**: このプロジェクトでは以下の言語使い分けルールに従ってください：

- **README.md**: 英語で記述（国際的な可読性のため）
- **ソースコード内のdocコメント**: 英語で記述（`///`コメント、`//!`コメント）
- **テストコード内のコメント**: 日本語でも可（例：`// 既存ファイルの上書きテスト`）
- **コミットメッセージ**: 英語推奨だが日本語も可
- **AIへの指示**: 日本語で構いません（このガイドラインは日本語で記述されています）

クラス名、関数名、変数名は英語で記述してください。

## 重要な実装の詳細

### Enum命名規則
**重要**: 全てのenumはTOMLシリアライゼーションに合わせてsnake_case形式を使用します（PascalCaseではありません）：
```rust
#[allow(non_camel_case_types)]
pub enum TaskStatus {
    inbox,           // NOT Inbox
    next_action,     // NOT NextAction
    waiting_for,
    someday,
    done,
    trash,
}
```
これはテスト（例：`test_enum_snake_case_serialization`）で強制されており、必ず守る必要があります。

### MCPツール実装パターン
全てのMCPツールは以下のパターンに従います：
1. Mutexをロック: `let mut data = self.data.lock().unwrap();`
2. `GtdData`に対する操作を実行
3. ロックを解放: `drop(data);`
4. ディスクに保存: `self.save_data()?`
5. エラーには`bail!()`を使用（`mcp_attr::bail`から）

例：
```rust
#[tool]
async fn add_task(&self, title: String, ...) -> McpResult<String> {
    let mut data = self.data.lock().unwrap();
    data.add_task(task);
    drop(data);
    if let Err(e) = self.save_data() {
        bail!("Failed to save: {}", e);
    }
    Ok(format!("Task created with ID: {}", task_id))
}
```

### 日付の扱い
- 日付には`chrono::NaiveDate`を使用（時刻コンポーネントなし）
- パース形式：`YYYY-MM-DD`（`NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")`経由）
- オプショナルな日付は`Option<NaiveDate>`を使用

### データストレージ
- 人間が読みやすいように`toml::to_string_pretty()`経由でTOML形式
- ファイルパス：カレントディレクトリの`gtd.toml`
- バージョン管理に適したGitフレンドリーな形式
- ストレージ操作は`anyhow::Result`を返す

## 開発ワークフロー

### ビルド
```bash
cargo build              # デバッグビルド
cargo build --release    # リリースビルド
```

### テスト
```bash
cargo test              # 全119個のユニットテストを実行
```
テストは`env::temp_dir()`経由で一時ファイルを使用し、後でクリーンアップします。

### サーバーの実行
```bash
cargo run               # stdio MCPサーバーを起動
# または: ./target/release/gtd-mcp-rs
```
サーバーはstdio（JSON-RPC）経由でClaude DesktopなどのMCPクライアントと通信します。

### Claude Desktopとの統合
`claude_desktop_config.json`に以下を追加：
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs"
    }
  }
}
```

## テスト規約

### テストの構成
- 各ファイルの最後に`#[cfg(test)]`モジュール内にテストを配置
- 日本語コメントでテストの目的を説明（例：`// 既存ファイルの上書きテスト`）
- テスト名は説明的に：`test_storage_save_and_load_comprehensive`

### テストデータパターン
- 一時ファイルパスには`get_test_path()`を使用
- テストファイルをクリーンアップ：`let _ = fs::remove_file(&test_path);`
- 最小限のstructと完全に設定されたstructの両方をテスト
- `test_complete_toml_output`でTOML出力が期待される形式と一致することを検証

## 依存関係

- **`mcp-attr` (0.0.7)**: 宣言的MCPサーバー構築（クロスプラットフォーム、`rust-mcp-sdk`の代替）。`schemars 0.8`に依存してJSON Schema生成を提供
- **`tokio`**: MCPサーバー用の非同期ランタイム
- **`toml` (0.9)**: シリアライゼーション（注：可読性のため`toml::to_string_pretty`を使用）
- **`chrono`**: `serde`機能付き日付処理
- **`uuid`**: `v4`機能でタスク/プロジェクトIDを生成（後方互換性のため保持されているが、現在は使用されていない）
- **`anyhow`**: コンテキスト付きエラーハンドリング
- **`git2`**: Git操作による自動バージョン管理

## コードスタイルパターン

- 外部でまだ使用されていないヘルパーメソッドには`#[allow(dead_code)]`を使用
- enumフィルタリングにはmatch式を使用（例：`matches!(task.status, TaskStatus::inbox)`）
- 文字列フォーマット：`format!("- [{}] {} (status: {:?})\n", ...)`
- Mutexパターン：ロック、変更、解放、永続化
- エラー伝播：MCPツールには`bail!()`、内部関数には`?`

## 注意点（落とし穴）

1. **Edition 2024**: `Cargo.toml`は`edition = "2024"`を使用（2021ではない）
2. **クロスプラットフォーム**: Windows互換性のため`rust-mcp-sdk`の代わりに`mcp-attr`を使用
3. **TOML構造**: タスク/プロジェクトは配列（`[[tasks]]`）、コンテキストはテーブル（`[contexts.Name]`）
4. **ID生成**: カウンターベースのID生成システムを使用（タスク: `#1`, `#2`, プロジェクト: `project-1`, `project-2`）。`GtdData::generate_task_id()`と`GtdData::generate_project_id()`メソッドで管理
5. **ステータスフィルタリング**: `list_tasks`の文字列マッチングは動的なenumパースではなくハードコードされたmatchアームを使用
