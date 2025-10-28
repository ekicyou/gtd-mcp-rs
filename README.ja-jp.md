# gtd-mcp

[![CI](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml)

**バージョン 0.8.0**

GTD（Getting Things Done）タスク管理のためのModel Context Protocol（MCP）サーバーです。このサーバーにより、ClaudeなどのLLMアシスタントが実証済みのGTD手法を使用してタスクとプロジェクトの管理を支援できます。

## gtd-mcpとは？

gtd-mcpは、Getting Things Done（GTD）ワークフローを実装したMCPサーバーです。Model Context Protocolを通じてLLMアシスタントとシームレスに連携する、完全なタスク管理システムを提供します。

**主な機能：**
- ✅ 完全なGTDワークフロー対応（受信箱、次のアクション、待機中、いつかやる/多分やる、カレンダー、完了、参考資料、ゴミ箱）
- ✅ **統合notaインターフェース** - タスク、プロジェクト、コンテキストを1つのツールセットで管理
- ✅ プロジェクトとコンテキストの管理
- ✅ **柔軟なタスクID** - クライアント側で任意の文字列を指定可能（例："meeting-prep"、"call-sarah"）
- ✅ 効率的なタスク管理のためのバッチ操作
- ✅ TOML形式のストレージ（人間が読みやすく、Git対応）
- ✅ オプションのGit同期機能

## クイックスタート

### インストール

crates.ioからのインストール：
```bash
cargo install gtd-mcp
```

またはソースからのビルド：
```bash
git clone https://github.com/ekicyou/gtd-mcp-rs.git
cd gtd-mcp-rs
cargo build --release
```

バイナリは`target/release/gtd-mcp`（ソースビルド）または`~/.cargo/bin/gtd-mcp`（cargo install）に配置されます。

### 設定

MCPクライアントの設定（例：Claude Desktopの`claude_desktop_config.json`）に追加します：

`cargo install`でインストールした場合：
```json
{
  "mcpServers": {
    "gtd": {
      "command": "gtd-mcp",
      "args": ["gtd.toml"]
    }
  }
}
```

ソースからビルドした場合：
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp",
      "args": ["gtd.toml"]
    }
  }
}
```

Git同期を有効にする場合：
```json
{
  "mcpServers": {
    "gtd": {
      "command": "gtd-mcp",
      "args": ["gtd.toml", "--sync-git"]
    }
  }
}
```

### 使用方法

設定が完了したら、LLMアシスタントに統合notaインターフェースを使用したタスク管理の支援を依頼できます：

- 「プロジェクト提案書をレビューする新しいタスクを追加して」
- 「次のアクションを見せて」
- 「meeting-prepタスクを更新してメモを追加して」
- 「call-sarahのステータスを完了に変更して」
- 「website-redesignというプロジェクトを作成して」
- 「受信箱に何がある？」
- 「受信箱の処理を手伝って」

## MCPツール

システムは、すべてのGTD操作を処理する5つの統合ツールを提供します：

### 収集とレビュー

**inbox** - 注意が必要なものを収集（GTD収集ステップ）
- 必須：`id`（任意の文字列、例："call-john"、"website-redesign"）、`title`、`status`
- オプション：`project`、`context`、`notes`、`start_date`（YYYY-MM-DD）、`recurrence`（繰り返しパターン）、`recurrence_config`（繰り返し設定）
- statusがタイプを決定：inbox/next_action等→タスク、project→プロジェクト、context→コンテキスト
- GTDワークフローの最初のステップとして使用 - 後で処理するためにすべてを素早く収集

**list** - オプションのフィルターですべてのnotaをレビュー（GTDレビューステップ）
- オプション：`status` - 特定のステータスでフィルタリング（inbox、next_action、waiting_for、later、calendar、someday、done、reference、trash、project、context）
- オプション：`date`（YYYY-MM-DD） - calendarステータスの場合、start_date <= この日付のタスクを表示
- オプション：`exclude_notes`（boolean） - notesを除外してトークン使用量を削減
- オプション：`keyword` - ID、タイトル、ノートでキーワード検索（大文字小文字を区別しない）
- オプション：`project` - プロジェクトIDでフィルタリング
- オプション：`context` - コンテキスト名でフィルタリング
- システムを最新の状態に保つために定期的に（毎日/毎週）レビュー

## GTDステータスカテゴリ

システムは、GTD手法に従って以下のステータスカテゴリをサポートします：

### 実行可能な項目
- **inbox**：明確化が必要な未処理の項目（ここから開始）
- **next_action**：注意が必要な実行準備完了のタスク
- **waiting_for**：他の誰かまたは外部イベントによってブロックされている項目
- **later**：最終的に実行する予定の延期されたタスク
- **calendar**：日付または時刻が指定されたタスク
- **someday**：将来の潜在的なアクション（まだコミットされていない）

### 実行不可能な項目
- **reference**：将来の参照用に保存された実行不可能な情報 - 重要な文書、メモ、または後で必要になる可能性があるが、アクションを必要としない情報
- **done**：完了したタスク（記録保持とレビュー用）
- **trash**：破棄された項目（empty_trashで永久に削除可能）

### 組織構造
- **project**：複数のアクションを必要とする複数ステップの成果物
- **context**：アクションを実行できる環境、ツール、または状況（例：@office、@home、@computer）

### 整理と実行

**update** - notaの詳細を明確化し整理（GTD明確化/整理ステップ）
- 必須：`id`
- オプション：`title`、`status`、`project`、`context`、`notes`、`start_date`
- ステータスを変更してタイプを変換可能（タスク→プロジェクト、タスク→コンテキストなど）
- オプションフィールドをクリアするには空文字列""を使用
- 受信箱に収集した後、これを使用してコンテキストを追加し、次のステップを明確化

**change_status** - GTDワークフローステージを通じてnotaを移動（GTD実行/整理ステップ）
- 必須：`ids`（バッチ操作の場合は配列、単一項目の場合は単一ID）、`new_status`
- オプション：`start_date`（YYYY-MM-DD、calendarステータスに移動する際に必須）
- タイプ変換を含むすべてのワークフロー遷移をサポート
- 一般的なワークフロー：inbox → next_action → done、またはinbox → waiting_for、またはinbox → trash

### メンテナンス

**empty_trash** - ゴミ箱に入れたすべてのnotaを永久に削除（GTD削除ステップ）
- パラメータは不要
- 不可逆的な操作 - GTDレビューの一環として週次で実行
- 壊れたリンクを防ぐために参照を自動的にチェック

## 繰り返しタスク機能

バージョン0.8.0から、繰り返しタスク機能が追加されました：

- **daily**：毎日繰り返し
- **weekly**：特定の曜日に繰り返し（例：月曜日、水曜日、金曜日）
- **monthly**：月の特定の日に繰り返し（例：1日、15日、25日）
- **yearly**：年の特定の月日に繰り返し（例：1月1日、12月25日）

繰り返しタスクを作成するには、`inbox`ツールで`recurrence`と`recurrence_config`パラメータを使用します。

## データストレージ

タスクはTOML形式（デフォルト：`gtd.toml`）で保存されます。この形式は人間が読みやすく、Git対応です：

```toml
format_version = 3

[[inbox]]
id = "#1"
title = "プロジェクト提案書をレビューする"
project = "q1-marketing"
context = "Office"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[project]]
id = "q1-marketing"
title = "Q1マーケティングキャンペーン"

[[context]]
name = "Office"
notes = "デスクとコンピュータがある作業環境"
```

サーバーは、データファイルをロードする際に古い形式バージョン（v1、v2）を現在のバージョン（v3）に自動的に移行します。

### Git統合

`--sync-git`フラグで自動Git同期を有効にします。サーバーは以下を実行します：
- ロード前に最新の変更をpull
- 説明的なメッセージで変更をコミット
- 保存後にリモートにpush

設定：
```bash
git init
git config user.name "Your Name"
git config user.email "your@email.com"
git remote add origin https://github.com/yourusername/gtd-data.git
```

## ドキュメント

- **[FEATURES_JA.md](FEATURES_JA.md)** - 実装機能の詳細な技術仕様（日本語）
- **[IMPLEMENTATION.md](doc/IMPLEMENTATION.md)** - 技術的な実装詳細とアーキテクチャ（英語）
- **[GTD_ASSESSMENT.md](doc/GTD_ASSESSMENT.md)** - 機能評価と拡張ロードマップ（英語）
- **[RELEASE.md](RELEASE.md)** - すべてのバージョンのリリースノート（新しい順、英語）
- **[MCP_TOOLS.md](doc/MCP_TOOLS.md)** - MCPツールの詳細なドキュメント（英語）

## 開発

```bash
# ビルド
cargo build

# テスト実行
cargo test

# コード品質チェック
cargo fmt --check
cargo clippy -- -D warnings
```

CI/CDの詳細については[CI_SUMMARY.md](doc/CI_SUMMARY.md)を参照してください。

## ライセンス

MITライセンス - 詳細はLICENSEファイルを参照してください。

## 関連リンク

- [English README](README.md) - 英語版README
- [GitHub リポジトリ](https://github.com/ekicyou/gtd-mcp-rs)
- [crates.io](https://crates.io/crates/gtd-mcp)
