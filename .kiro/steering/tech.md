# Technology Stack

## Architecture

3層アーキテクチャの Rust 製 MCP サーバー（lib + bin クレート構成）:

- **MCP 層**: `GtdServerHandler`（`src/lib.rs`）— `mcp-attr` の宣言的マクロ（`#[mcp_server]`, `#[tool]`）でツールを定義し、`src/handlers/` に処理を委譲
- **ドメイン層**: `src/gtd/` — 統一 `Nota` モデルと `GtdData` コンテナ
- **永続化層**: `src/storage.rs` + `src/git_ops.rs` — TOML ファイル保存と Git 自動コミット

データフロー: MCP クライアント → stdio (JSON-RPC) → `GtdServerHandler` → `GtdData`（インメモリ, Mutex 保護） → `Storage` → `gtd.toml`

## Core Technologies

- **言語**: Rust, **Edition 2024**（2021 ではない点に注意）
- **クレート**: lib 名 `gtd_mcp`（`src/lib.rs`）、bin 名 `gtd-mcp`（`src/main.rs`）

## Key Libraries

- **`mcp-attr` (~0.0.7)**: 宣言的 MCP サーバー構築。Windows 互換性（クロスプラットフォーム）のため `rust-mcp-sdk` の代替として採用。JSON Schema 生成は内部の `schemars` に依存
- **`tokio`**: MCP サーバー用非同期ランタイム
- **`toml` (~1)**: シリアライゼーション。可読性のため `toml::to_string_pretty()` を使用
- **`chrono`**: `serde` 機能付き日付処理（`NaiveDate`、時刻なし）
- **`anyhow`**: コンテキスト付きエラーハンドリング（ストレージ層は `anyhow::Result` を返す）
- **`git2`**: Git 操作による自動バージョン管理
- **`clap` (derive)**: CLI 引数パース（位置引数 `file` と `--sync-git` フラグ）
- **`tempfile`** (dev): テスト用一時ファイル

## Development Standards

### 言語使い分けルール
- **README.md**: 英語（国際的な可読性のため）。日本語版は `README.ja-jp.md`
- **ソースコードの doc コメント**（`///`, `//!`）: 英語
- **テストコード内コメント**: 日本語可（例: `// 既存ファイルの上書きテスト`）
- **コミットメッセージ**: 英語推奨、日本語も可
- **AI への指示・ステアリング**: 日本語で可
- クラス名・関数名・変数名は英語

### 日付の扱い
- `chrono::NaiveDate` を使用（時刻コンポーネントなし）、オプショナルは `Option<NaiveDate>`
- パース形式: `YYYY-MM-DD`（`NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")`）
- 現在日付は `local_date_today()`（ローカルタイムゾーン）を使用

### データストレージ
- TOML 形式、`toml::to_string_pretty()` で人間可読・Git フレンドリーに出力
- データファイルパスは CLI 位置引数で指定（例: `gtd-mcp gtd.toml`）
- 改行コード: 読み込み時に LF へ正規化、書き込み時に OS ネイティブ形式へ変換（Windows は CRLF）
- ストレージ操作は `anyhow::Result` を返す

### エラーハンドリング
- MCP ツール内: `bail!()`（`mcp_attr::bail`）
- 内部関数: `?` で伝播
- パラメータ検証: `src/validation.rs` のヘルパー（`mcp_attr::ErrorCode::INVALID_PARAMS`）

### コードスタイル
- 外部未使用のヘルパーには `#[allow(dead_code)]`
- enum フィルタリングは match / `matches!` 式（例: `matches!(nota.status, NotaStatus::inbox)`）
- Mutex パターン: ロック → 変更 → 解放（`drop`） → 永続化

## Development Environment

### Common Commands
```bash
cargo build              # デバッグビルド
cargo build --release    # リリースビルド
cargo test               # 全テスト実行（270超のテスト）
cargo run -- gtd.toml    # stdio MCP サーバー起動（--sync-git で Git 同期）
```
引数なし起動はヘルプ表示 + exit code 2。

### CI チェック（コミット・PR 前に必須）
GitHub Actions CI と同一のチェックをローカルで実行すること:
```bash
cargo fmt --all -- --check                                # 1. フォーマット
cargo clippy --all-targets --all-features -- -D warnings  # 2. Lint
cargo build --verbose                                     # 3. ビルド
cargo test --verbose                                      # 4. 全テスト
cargo build --release --verbose                           # 5. リリースビルド
```
- `cargo clippy` は必ず `--all-targets --all-features` 付きで実行（テストコード含めて lint。テスト内の未使用 import 等も CI で失敗する）
- フォーマットエラーは `cargo fmt --all` で自動修正
- CI 失敗はマージブロックとなる

## Key Technical Decisions

1. **Edition 2024**: `Cargo.toml` は `edition = "2024"`（2021 と誤認しない）
2. **`mcp-attr` 採用**: Windows 互換性のため `rust-mcp-sdk` を使わない
3. **enum は snake_case**: TOML シリアライゼーションと一致させる（structure.md 参照）
4. **統一 Nota モデル**: Task/Project/Context を単一構造に統合（TiddlyWiki の tiddler に着想）
5. **ID は不変のクライアント指定文字列**: 旧カウンターベース ID（`#1`, `project-1`）はレガシー形式としてマイグレーションで吸収

---
_created: 2026-07-23 (copilot-instructions.md からの移行を含む)_
