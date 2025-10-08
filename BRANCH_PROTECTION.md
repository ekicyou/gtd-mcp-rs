# Branch Protection Setup Guide

このドキュメントでは、mainブランチを保護するための設定手順を説明します。

## GitHub Actions CI ワークフロー

プロジェクトには以下のCIチェックが自動で実行されます：

### 1. CI ワークフロー (`.github/workflows/ci.yml`)

#### 実行タイミング
- mainブランチへのpush時
- Pull Request作成・更新時

### チェック内容

#### 1. テストジョブ (test)
複数のプラットフォームで実行：
- **Ubuntu (Linux)**: コードフォーマット、Clippy lintチェック、ビルド、テスト
- **macOS**: ビルド、テスト
- **Windows**: ビルド、テスト

実行項目：
- `cargo fmt --check`: コードフォーマットの検証
- `cargo clippy`: Lintチェック（警告もエラーとして扱う）
- `cargo build`: デバッグビルドの検証
- `cargo test`: 全テスト（56個）の実行
- `cargo build --release`: リリースビルドの検証

#### 2. セキュリティ監査 (security-audit)
- `cargo audit`: 依存パッケージのセキュリティ脆弱性をチェック

#### 3. MSRV チェック (msrv)
- Minimum Supported Rust Version (最小サポートRustバージョン) の検証
- Edition 2024のため、Rust 1.85.0以上が必要

### 2. セキュリティ監査ワークフロー (`.github/workflows/security-audit.yml`)

#### 実行タイミング
- 毎日 00:00 UTC に自動実行（スケジュール）
- 手動実行も可能（workflow_dispatch）

#### チェック内容
- `cargo audit`: 依存パッケージのセキュリティ脆弱性をチェック
- 脆弱性が検出された場合、自動的にIssueを作成

### 3. Dependabot (`.github/dependabot.yml`)

#### 機能
- **Cargo依存パッケージ**: 毎週月曜日に自動チェック
- **GitHub Actions**: 毎週月曜日に自動チェック
- セキュリティアップデートは自動的にPRを作成
- パッチアップデートをグループ化して1つのPRにまとめる

## Branch Protection ルールの設定方法

GitHubのリポジトリでmainブランチを保護するには、以下の手順で設定してください：

### 1. リポジトリ設定へ移動
1. GitHubリポジトリページを開く
2. 「Settings」タブをクリック
3. 左サイドバーの「Branches」をクリック

### 2. Branch Protection Ruleの追加
1. 「Add branch protection rule」ボタンをクリック
2. Branch name patternに `main` を入力

### 3. 推奨設定

#### 必須設定：
- [x] **Require a pull request before merging**
  - mainへの直接pushを禁止し、必ずPull Requestを経由
  - [x] **Require approvals**: 1人以上のレビュー承認を必須にする（推奨）

- [x] **Require status checks to pass before merging**
  - CIチェックの成功を必須にする
  - [x] **Require branches to be up to date before merging**
  - 以下のステータスチェックを必須に設定：
    - `Test on ubuntu-latest (stable)`
    - `Test on macos-latest (stable)`
    - `Test on windows-latest (stable)`
    - `Security Audit`
    - `Minimum Supported Rust Version`

#### 追加の推奨設定：
- [x] **Require conversation resolution before merging**
  - レビューコメントの解決を必須にする

- [x] **Do not allow bypassing the above settings**
  - 管理者も含めて、上記ルールの回避を禁止

- [ ] **Require linear history** (オプション)
  - マージコミットを禁止し、rebaseまたはsquashマージのみ許可

### 4. 設定の保存
- 「Create」または「Save changes」ボタンをクリック

## ローカル開発でのCIチェック実行

Pull Requestを作成する前に、ローカルで以下のコマンドを実行してCIエラーを防ぐことができます：

```bash
# コードフォーマット
cargo fmt

# フォーマットチェック
cargo fmt --check

# Lintチェック
cargo clippy --all-targets --all-features -- -D warnings

# テスト実行
cargo test

# リリースビルド
cargo build --release

# セキュリティ監査（cargo-auditが必要）
cargo install cargo-audit
cargo audit
```

## CI失敗時のトラブルシューティング

### フォーマットエラー
```bash
# 自動修正
cargo fmt
```

### Clippy警告
```bash
# 警告を表示
cargo clippy

# コードを修正後、再チェック
cargo clippy -- -D warnings
```

### テスト失敗
```bash
# 詳細なテスト出力
cargo test -- --nocapture

# 特定のテストのみ実行
cargo test test_name
```

### セキュリティ脆弱性
```bash
# 詳細情報を表示
cargo audit

# 依存パッケージの更新
cargo update
```

## 参考リンク

- [GitHub Docs: Branch Protection Rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
