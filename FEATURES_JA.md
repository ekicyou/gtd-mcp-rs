# gtd-mcp-rs 実装機能の詳細説明

## 概要

gtd-mcp-rsは、RustでModel Context Protocol (MCP)を実装したGTD（Getting Things Done）タスク管理サーバーです。ClaudeなどのLLMアシスタントと連携し、David AllenのGTD手法に基づいた包括的なタスク管理システムを提供します。

## アーキテクチャ

3層アーキテクチャパターンを採用：

1. **MCPレイヤー** (`src/main.rs`, `src/lib.rs`)
   - `mcp-attr`クレートの宣言的マクロ（`#[mcp_server]`、`#[tool]`）を使用
   - stdio（標準入出力）経由でJSON-RPC通信
   - 非同期処理にTokioランタイムを使用

2. **ドメインレイヤー** (`src/gtd/`)
   - コアドメインモデル：`Nota`（統合概念）、`NotaStatus`、`RecurrencePattern`
   - `GtdData`：インメモリデータ構造
   - ビジネスロジックとクエリ機能

3. **永続化レイヤー** (`src/storage.rs`, `src/git_ops.rs`)
   - TOML形式でのファイル保存（人間が読みやすい）
   - オプションのGit自動バージョン管理
   - データマイグレーション機能（v1→v2→v3）

## 実装されている5つのMCPツール

### 1. inbox（収集）

**目的**: GTDワークフローの第一段階「収集」。頭に浮かんだことを素早く収集します。

**パラメータ**:
- `id` (必須): 任意の文字列ID（例："call-john"、"meeting-prep"）
- `title` (必須): 項目の簡単な説明
- `status` (必須): ステータス（inbox/next_action/waiting_for/later/calendar/someday/done/reference/project/context/trash）
- `project` (オプション): 親プロジェクトID
- `context` (オプション): コンテキスト（@home、@officeなど）
- `notes` (オプション): Markdown形式の詳細メモ
- `start_date` (オプション): 開始日（YYYY-MM-DD形式）
- `recurrence` (オプション): 繰り返しパターン（daily/weekly/monthly/yearly）
- `recurrence_config` (オプション): 繰り返し設定の詳細

**実装の特徴**:
- statusによってタイプが自動決定（task/project/context）
- 自動的にタイムスタンプ（created_at、updated_at）を付与
- IDは不変（後から変更不可）
- データ検証とエラーハンドリング

**実装場所**: `src/handlers/inbox.rs`

### 2. list（レビュー）

**目的**: すべての項目を一覧表示し、フィルタリングしてレビューします。日次・週次レビューに不可欠。

**パラメータ**:
- `status` (オプション): ステータスでフィルタリング
- `date` (オプション): 日付フィルター（calendarステータス用）
- `exclude_notes` (オプション): notesを除外してトークン使用量削減
- `keyword` (オプション): キーワード検索（ID、タイトル、ノート）
- `project` (オプション): プロジェクトIDでフィルタリング
- `context` (オプション): コンテキスト名でフィルタリング

**実装の特徴**:
- 複数のフィルター条件を組み合わせ可能
- calendarステータスの日付フィルタリング（start_date <= 指定日）
- キーワード検索は大文字小文字を区別しない
- フォーマット済みの読みやすい出力（`src/formatting.rs`）

**実装場所**: `src/handlers/list.rs`

### 3. update（明確化）

**目的**: 収集した項目の詳細を追加・更新。GTDの「明確化」「整理」ステップ。

**パラメータ**:
- `id` (必須): 更新する項目のID（変更不可）
- `title` (オプション): 新しいタイトル
- `status` (オプション): 新しいステータス（タイプ変換可能）
- `project` (オプション): プロジェクトリンク（""でクリア）
- `context` (オプション): コンテキストタグ（""でクリア）
- `notes` (オプション): Markdownノート（""でクリア）
- `start_date` (オプション): 開始日（""でクリア）

**実装の特徴**:
- 空文字列("")でオプションフィールドをクリア可能
- ステータス変更によるタイプ変換（task→project、task→contextなど）
- 自動的にupdated_atタイムスタンプを更新
- 参照整合性のチェック（プロジェクト、コンテキストの存在確認）

**実装場所**: `src/handlers/update.rs`

### 4. change_status（整理・実行）

**目的**: GTDワークフローステージ間で項目を移動。バッチ操作対応。

**パラメータ**:
- `ids` (必須): 変更する項目IDの配列（バッチ操作可能）
- `new_status` (必須): 新しいステータス
- `start_date` (オプション): 開始日（calendarステータス時に必須）

**実装の特徴**:
- 複数の項目を一度に処理（バッチ操作）
- calendarステータスへの移動時にstart_dateを必須チェック
- すべてのワークフロー遷移をサポート
- タイプ変換もサポート（statusがproject/contextの場合）
- 各項目の処理結果を個別に報告

**実装場所**: `src/handlers/change_status.rs`

### 5. empty_trash（削除）

**目的**: ゴミ箱の項目を完全に削除。GTD週次レビューの一部。

**パラメータ**: なし

**実装の特徴**:
- trashステータスの項目のみを削除
- 参照整合性チェック（他の項目から参照されていないか確認）
- 削除前に警告と削除予定項目のリスト表示
- 不可逆的操作のため、慎重な実装

**実装場所**: `src/handlers/empty_trash.rs`

## コアデータモデル

### Nota（統合概念）

Task、Project、Contextを統一した単一のデータ構造：

```rust
pub struct Nota {
    pub id: String,                              // 不変のID
    pub title: String,                           // タイトル
    pub status: NotaStatus,                      // ステータス（タイプを決定）
    pub project: Option<String>,                 // 親プロジェクト
    pub context: Option<String>,                 // コンテキスト
    pub notes: Option<String>,                   // Markdownノート
    pub start_date: Option<NaiveDate>,           // 開始日
    pub created_at: NaiveDate,                   // 作成日
    pub updated_at: NaiveDate,                   // 更新日
    pub recurrence_pattern: Option<RecurrencePattern>,  // 繰り返しパターン
    pub recurrence_config: Option<String>,       // 繰り返し設定
}
```

**実装場所**: `src/gtd/nota.rs`

### NotaStatus（11種類）

GTD手法に基づくステータス：

**実行可能な項目**:
- `inbox` - 未処理（開始地点）
- `next_action` - 次のアクション（実行準備完了）
- `waiting_for` - 待機中（ブロック状態）
- `later` - 後で（延期）
- `calendar` - カレンダー（日付指定）
- `someday` - いつかやる/多分やる

**実行不可能な項目**:
- `reference` - 参考資料（アクション不要の情報）
- `done` - 完了
- `trash` - ゴミ箱

**組織構造**:
- `project` - プロジェクト（複数ステップの成果物）
- `context` - コンテキスト（場所、ツール、状況）

**実装の特徴**:
- snake_case命名（TOMLシリアライゼーションに対応）
- `FromStr`トレイト実装（文字列からのパース）
- 詳細なエラーメッセージ

**実装場所**: `src/gtd/nota.rs`

### RecurrencePattern（繰り返しパターン）

バージョン0.8.0で追加された繰り返しタスク機能：

- `daily` - 毎日
- `weekly` - 週次（特定の曜日）
- `monthly` - 月次（特定の日）
- `yearly` - 年次（特定の月日）

**実装場所**: `src/gtd/nota.rs`

## ストレージとデータ永続化

### TOML形式

人間が読みやすく、Git対応の形式：

```toml
format_version = 3

[[inbox]]
id = "review-proposal"
title = "プロジェクト提案書をレビューする"
status = "inbox"
project = "q1-marketing"
context = "Office"
notes = "詳細な分析が必要"
start_date = "2024-01-15"
created_at = "2024-01-01"
updated_at = "2024-01-10"

[[project]]
id = "q1-marketing"
title = "Q1マーケティングキャンペーン"
status = "project"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[context]]
name = "Office"
title = "Office"
status = "context"
notes = "デスクとコンピュータがある作業環境"
created_at = "2024-01-01"
updated_at = "2024-01-01"
```

**実装場所**: `src/storage.rs`

### データマイグレーション

古いバージョンのデータファイルを自動的に最新形式に変換：

- **v1→v2**: タスクIDの正規化（#プレフィックスの追加）
- **v2→v3**: Nota統合への移行（Task/Project/Context → Nota）

**実装の特徴**:
- 後方互換性の維持
- 段階的な移行
- エラーハンドリングと検証
- 行末文字の正規化（CRLF/LF）

**実装場所**: `src/migration/`

### Git統合

オプションの自動バージョン管理：

**機能**:
- ロード前に最新の変更をpull
- 保存時に変更をコミット
- シャットダウン時にリモートにpush
- 説明的なコミットメッセージ

**設定**:
```bash
cargo run -- gtd.toml --sync-git
```

**実装の特徴**:
- `git2`クレートを使用
- エラーハンドリング（非Gitリポジトリでも動作）
- 自動プッシュ（`Drop`トレイトで実装）

**実装場所**: `src/git_ops.rs`

## データ検証

### ID正規化

ユーザーの入力を自動的に正規化：

```rust
// "#1" も "1" も "#1" に正規化
normalize_task_id("1") => "#1"
normalize_task_id("#1") => "#1"
```

**実装場所**: `src/validation.rs`

### 参照整合性チェック

- プロジェクトIDの存在確認
- コンテキスト名の存在確認
- 削除時の参照チェック（orphan防止）

**実装場所**: `src/gtd/gtd_data.rs`

## テストカバレッジ

### 統合テスト

- **storage_test.rs**: ストレージ機能の包括的テスト（22テスト）
  - TOML保存・ロード
  - 行末文字の正規化
  - Gitシンク機能
  - エラーハンドリング

- **integration_test.rs**: MCPハンドラーの統合テスト
  - 全ツールの動作確認
  - ワークフロー全体のテスト
  - エッジケースの処理

- **migration_test.rs**: データマイグレーション機能のテスト（6テスト）
  - v1→v2移行
  - v2→v3移行
  - フィールド名の変換

- **git_ops_test.rs**: Git操作のテスト
  - コミット・プッシュ・プル
  - エラーハンドリング

**テスト場所**: `tests/`

### ユニットテスト

- `src/gtd/gtd_data.rs`: プライベートメソッドのテスト
- エッジケースと境界値のテスト

## コード品質

### Linting

- **cargo fmt**: コードフォーマットの統一
- **cargo clippy**: Rust のベストプラクティスチェック

### CI/CD

GitHub Actionsによる自動チェック：
- すべてのプラットフォームでのビルド（Linux, macOS, Windows）
- 全テストの実行
- Lintチェック
- コードカバレッジ

**設定場所**: `.github/workflows/ci.yml`

## 依存関係

### 主要クレート

- **mcp-attr (0.0.7)**: 宣言的MCPサーバー構築
  - クロスプラットフォーム対応
  - JSON Schema自動生成
  
- **tokio**: 非同期ランタイム
  - MCPサーバーの非同期処理
  
- **toml (0.9)**: データシリアライゼーション
  - `to_string_pretty`で可読性の高い出力
  
- **chrono**: 日付・時刻処理
  - `NaiveDate`（タイムゾーンなし日付）
  - `serde`機能でシリアライゼーション対応
  
- **anyhow**: エラーハンドリング
  - コンテキスト付きエラー伝播
  
- **git2**: Git操作
  - ネイティブGitライブラリのバインディング
  
- **clap**: コマンドライン引数パース
  - `--sync-git`フラグなど

## MCPプロトコル実装

### 宣言的マクロ

`mcp-attr`クレートの`#[mcp_server]`と`#[tool]`マクロを使用：

```rust
#[mcp_server]
impl McpServer for GtdServerHandler {
    #[tool]
    pub async fn inbox(&self, id: String, ...) -> McpResult<String> {
        // 実装
    }
}
```

**利点**:
- ボイラープレートコードの削減
- 自動的なJSON Schema生成
- LLMクライアント用の型安全なインターフェース

### stdio通信

JSON-RPCプロトコルを標準入出力経由で通信：

```rust
serve_stdio(handler).await?;
```

**実装場所**: `src/main.rs`

## 実装パターン

### MCPツールの標準パターン

```rust
#[tool]
async fn some_tool(&self, ...) -> McpResult<String> {
    // 1. Mutexをロック
    let mut data = self.data.lock().unwrap();
    
    // 2. GtdDataに対する操作
    data.some_operation()?;
    
    // 3. ロックを解放
    drop(data);
    
    // 4. ディスクに保存
    if let Err(e) = self.save_data_with_message("commit message") {
        bail!("Failed to save: {}", e);
    }
    
    // 5. 結果を返す
    Ok(format!("Success: ..."))
}
```

### エラーハンドリング

- MCPツール: `bail!()`マクロ（`mcp_attr::bail`）
- 内部関数: `?`演算子
- `anyhow::Result`でコンテキスト付きエラー

### メモリ管理

- `Mutex<GtdData>`でスレッドセーフなアクセス
- ロック期間の最小化
- 明示的な`drop()`でロック解放

## ドキュメンテーション

### doc comment規約

- 関数のdoc commentはMCPクライアントに公開される
- LLMのトークン消費を最小化
- GTDコンテキストを含める
- `Option<T>`には"Optional"を明記

**例**:
```rust
/// **Capture**: Quickly capture anything needing attention. First GTD step.
/// **When**: Something crosses your mind? Capture immediately.
#[tool]
pub async fn inbox(&self, 
    /// Unique string ID (e.g., "call-john", "web-redesign")
    id: String,
    ...
) -> McpResult<String>
```

## パフォーマンス最適化

### トークン使用量の削減

- `exclude_notes`オプションでnotesを除外
- 簡潔なフォーマット
- 必要な情報のみを返す

### バッチ操作

- `change_status`で複数項目を一度に処理
- 1回のディスク書き込みで複数の変更を保存

## セキュリティ

### 参照整合性

- 削除前に参照チェック
- orphanレコードの防止
- データ整合性の保証

### データ検証

- 日付フォーマットの検証
- ステータスの検証
- IDの正規化と検証

## まとめ

gtd-mcp-rsは、以下の特徴を持つ完成度の高いGTDタスク管理システムです：

**技術的な強み**:
- 3層アーキテクチャによる明確な関心の分離
- 宣言的MCPサーバー実装
- 包括的なテストカバレッジ
- 堅牢なエラーハンドリング
- 自動データマイグレーション

**機能的な強み**:
- 完全なGTDワークフロー対応
- 統合notaインターフェース
- 繰り返しタスク機能
- Git統合による自動バージョン管理
- 人間が読みやすいTOML形式

**ユーザビリティ**:
- LLMアシスタントとの自然な対話
- 柔軟なフィルタリングとクエリ
- バッチ操作のサポート
- 詳細なドキュメンテーション

現在のバージョン0.8.0では、本格的なGTDタスク管理に必要なすべての機能が実装されており、Claude DesktopなどのMCPクライアントと連携して効果的に使用できます。
