# Project Structure

## Organization Philosophy

レイヤード構成。MCP ツールの「宣言」（`src/lib.rs`）と「実装」（`src/handlers/`）を分離し、ドメインロジックは `src/gtd/` に集約する。横断的な補助ロジック（整形・検証・マイグレーション）は専用モジュールに切り出す。

## Directory Patterns

### MCP サーバー表面
**Location**: `src/lib.rs`
**Purpose**: `GtdServerHandler` 定義と `#[mcp_server]` ブロック。各 `#[tool]` は doc comment（LLM 向け仕様）を持ち、実装は `handlers/` の `handle_*` へ委譲
**Example**: `pub async fn inbox(...) -> McpResult<String> { self.handle_inbox(...).await }`

### ツールハンドラー
**Location**: `src/handlers/`
**Purpose**: MCP ツール 1 つにつき 1 ファイル（`inbox.rs`, `list.rs`, `update.rs`, `change_status.rs`, `empty_trash.rs`）
**Example**: 新ツール追加時は `handlers/` に実装ファイルを作り、`lib.rs` の `#[mcp_server]` に宣言を追加

### ドメイン層
**Location**: `src/gtd/`
**Purpose**: コアモデルとビジネスロジック
- `nota.rs`: 統一 `Nota` 構造・`NotaStatus`・`RecurrencePattern`
- `gtd_data.rs`: `GtdData` コンテナと全 GTD 操作
- `queries.rs`: クエリ・互換メソッド
- `serde_impl.rs`: `GtdData` のカスタム Serialize/Deserialize（マイグレーション統合点）

### 永続化・補助層
**Location**: `src/storage.rs`, `src/git_ops.rs`, `src/formatting.rs`, `src/validation.rs`
**Purpose**: TOML ファイル I/O（改行正規化含む）/ Git 自動コミット / 表示・フィルタ整形 / パラメータ検証

### マイグレーション
**Location**: `src/migration/`
**Purpose**: データフォーマットの世代間移行。レガシー型（`Task`/`Project`/`Context`）はここに隔離
**Pattern**: 新フォーマット導入時は (1) `migrate_vN_to_vN+1` 関数を追加、(2) `migrate_to_latest` に連鎖を追加、(3) 移行パスのテストを追加

### エントリポイント
**Location**: `src/main.rs`
**Purpose**: 薄い CLI エントリのみ（clap パース → `GtdServerHandler::new` → `serve_stdio`）。ロジックを置かない

## Naming Conventions

- **enum バリアント**: **snake_case**（PascalCase ではない）。TOML シリアライゼーション形式と一致させるため `#[allow(non_camel_case_types)]` を付与
  ```rust
  #[allow(non_camel_case_types)]
  pub enum NotaStatus {
      inbox, next_action, waiting_for, later, calendar,
      someday, done, reference, context, project, trash,
  }
  ```
  この規則はテストで強制されており、必ず守ること
- **Nota ID**: クライアント指定の kebab-case 文字列（例: `"fix-io-button"`）。**ID は不変**（変更不可）
- **識別子**: クラス名・関数名・変数名は英語
- **テスト名**: 説明的に（例: `test_storage_save_and_load_comprehensive`）

## TOML データ構造

- ルートに `format_version`、ステータス別配列（`[[inbox]]`, `[[next_action]]`, `[[project]]`, `[[context]]` 等）でシリアライズ。空のステータス配列は出力しない
- レガシー形式（v1: `[[projects]]` 配列、v2: `[projects.id]` テーブル、旧 Task/Project/Context 型、カウンターベース ID）は読み込み時に `migration/` で自動移行される
- ステータスフィルタリングは動的 enum パースではなくハードコードされた match アーム（`FromStr for NotaStatus`）を使用

## Test Organization

- **原則: `/src/` 配下にテストコードを置かない。テストは `/tests/` に配置する**
- 統合テスト: `tests/integration_test.rs`（MCP ハンドラー）、`tests/storage_test.rs`、`tests/migration_test.rs`、`tests/git_ops_test.rs`、`tests/gtd_data_test.rs`
- 例外: private フィールド/メソッドへのアクセスが必要な場合のみ、対象ファイル末尾の `#[cfg(test)]` モジュールに配置（例: `src/gtd/gtd_data.rs`）
- 共通ヘルパーは `tests/common/`
- テストデータパターン:
  - 一時ファイルは `env::temp_dir()` / `tempfile` / `get_test_path()` を使用し、終了時にクリーンアップ（`let _ = fs::remove_file(&test_path);`）
  - 最小構成 struct と全フィールド設定 struct の両方をテスト
  - TOML 出力が期待形式と一致することを検証するテストを維持（例: `test_complete_toml_output`）
- テストコメントは日本語可（例: `// 既存ファイルの上書きテスト`）

---
_created: 2026-07-23 (copilot-instructions.md からの移行を含む)_
