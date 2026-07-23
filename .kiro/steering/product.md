# Product Overview

gtd-mcp は、GTD（Getting Things Done）タスク管理を提供する MCP（Model Context Protocol）サーバー。Claude Desktop などの LLM アシスタントが、実証された GTD 手法でユーザーのタスク・プロジェクト管理を支援できるようにする。

## Core Capabilities

- **完全な GTD ワークフロー**: Capture(inbox) → Review(list) → Clarify(update) → Organize(change_status) → Do → Purge(empty_trash)
- **統一 Nota インターフェース**: タスク・プロジェクト・コンテキストを単一の `Nota` 構造で扱い、5つのツール（`inbox` / `list` / `update` / `change_status` / `empty_trash`）だけで全操作を提供
- **柔軟な ID**: クライアント（LLM）が意味のある文字列 ID を付与（例: `"meeting-prep"`, `"website-redesign"`）
- **繰り返しタスク**: daily / weekly / monthly / yearly の recurrence パターンをサポート
- **バッチ操作**: `change_status` は複数 ID を一括処理（週次レビューの効率化）
- **TOML ストレージ + Git 同期**: 人間が読める形式で保存し、任意で Git による自動バージョン管理

## Target Use Cases

- LLM アシスタント経由の日常タスク管理（「inboxを処理して」「今日のタスクは？」）
- GTD の日次・週次レビュー（statusフィルタ・日付フィルタ・キーワード検索）
- プロジェクト・コンテキスト（@home, @office 等）による整理

## Value Proposition

- **LLM ファースト設計**: ツールの doc comment が LLM への唯一の情報源となる前提で、トークン効率と誘導性を最適化
- **クロスプラットフォーム**: Windows を含む全主要 OS で動作（`mcp-attr` 採用の理由）
- **データの所有権**: ローカルの `gtd.toml` 1ファイル。Git フレンドリーで差分レビュー・履歴管理が容易
- **配布**: crates.io（`cargo install gtd-mcp`）またはソースビルド。MCP クライアント設定はコマンド + データファイルパス引数のみ

---
_created: 2026-07-23 (copilot-instructions.md からの移行を含む)_
