# gtd-mcp コーディング規約

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

### MCPツールのdoc comment規約
**重要**: `#[tool]`と`#[mcp_server]`のdoc commentはMCPクライアント（Claude DesktopなどのLLM）に直接公開され、ツールの使い方を理解するための唯一の情報源となります。以下のルールに従ってください：

#### doc commentの公開範囲
- **`#[tool]`関数のdoc comment**: MCPクライアントにツールの説明として表示されます
- **`#[mcp_server]`実装のdoc comment**: MCPサーバー全体の説明として表示されます
- **関数引数のdoc comment**: 各パラメータの説明としてMCPクライアントに表示されます
- これらのdoc commentはLLMがツールを使用する際の唯一の情報源であり、LLMのトークン消費量に直接影響します

#### doc comment記述ルール

1. **関数の説明は関数名に沿った動詞を使用**
   - 例：`delete_context`関数には「Delete a context」（"Remove"ではなく"Delete"を使用）
   - 関数名と説明の動詞を一致させることで、MCPクライアントの理解を容易にします

2. **`Option<T>`引数には必ず"Optional"を明記**
   - 悪い例：`/// New title for the task`
   - 良い例：`/// Optional new title for the task`
   - MCPクライアントがオプショナルであることを理解できるようにします

3. **task_ids引数の形式は`["#1", "#2", "#3"]`のみを記載**
   - プレーン数字（`["1", "2", "3"]`）については記載しない
   - システムが内部で自動修正しますが、MCPクライアントには`#`付き形式のみを推奨します

4. **project_id引数には意味のある略称を推奨**
   - 悪い例：`/// Project ID (e.g., "project-1", "project-2")`
   - 良い例：`/// Project ID - use meaningful abbreviation (e.g., "website-redesign", "q1-budget")`
   - 単純な連番ではなく、プロジェクトを表す説明的なIDを使用するよう誘導します

5. **GTDワークフローのコンテキストを含める**
   - 単なる機能説明だけでなく、GTD手法の中でいつ、なぜこのツールを使うべきかを説明します
   - 例：「Use this as the first step in GTD workflow - quickly capture anything that needs attention.」

6. **トークン消費を最小化**
   - 簡潔で明確な表現を使用し、冗長な説明を避けます
   - 重要な情報を優先し、自明な情報は省略します
   - 箇条書きや記号（→、/）を活用して情報密度を高めます

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
cargo test              # 全132個のユニットテストを実行
```
テストは`env::temp_dir()`経由で一時ファイルを使用し、後でクリーンアップします。

### サーバーの実行
```bash
cargo run               # stdio MCPサーバーを起動
# または: ./target/release/gtd-mcp
```
サーバーはstdio（JSON-RPC）経由でClaude DesktopなどのMCPクライアントと通信します。

### Claude Desktopとの統合
`claude_desktop_config.json`に以下を追加：
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp/target/release/gtd-mcp"
    }
  }
}
```

### CIチェックの実行
**重要**: コードを変更した後は、必ず以下のCIチェックを実行してください。これらはGitHub ActionsのCIで自動実行されるチェックと同じです：

```bash
# コードフォーマットチェック（必須）
cargo fmt --check

# Lintチェック（必須）
cargo clippy -- -D warnings

# 全テストの実行（必須）
cargo test
```

これらのチェックが全て通ることを確認してからコミット・プルリクエストを作成してください。CIで失敗すると、マージがブロックされます。

フォーマットエラーが出た場合は、`cargo fmt`で自動修正できます。

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
- **`anyhow`**: コンテキスト付きエラーハンドリング
- **`git2`**: Git操作による自動バージョン管理
- **`clap`**: コマンドライン引数のパース（`--file`や`--sync-git`オプション用）

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
