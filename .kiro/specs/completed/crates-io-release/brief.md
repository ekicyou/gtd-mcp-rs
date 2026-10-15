# Brief: crates-io-release

## Problem
gtd-mcp の開発者（ekicyou）は、リリースのたびに版上げ・タグ打ち・`cargo publish` を手で行っている。crates.io には 0.8.3 まで出ているが、Git のタグは `v0.8.0` までしか無く、crates.io の版とタグ・リポジトリの状態が食い違っている。手作業のため、版の付け忘れ・タグの打ち忘れ・手元のトークンでの公開といった事故の余地が残る。

## Current State
- `Cargo.toml` は単一クレート `gtd-mcp`（lib `gtd_mcp` + bin `gtd-mcp`）、版 0.8.3。crates.io の `gtd-mcp` は 0.8.3 まで公開済み（初版の未公開状態ではない）。
- `.github/workflows/` には `ci.yml`・`release.yml`（5 ターゲットのバイナリを作って GitHub Release に添える。非推奨の `actions/create-release@v1` 等を使用）・`security-audit.yml` が在る。crates.io へ公開する CI は無い。
- `RELEASE.md` は v0.8.0 までの英語のリリースノート（1668 行）。以後の版の記録は無い。
- `.claude/skills/` には kiro 系のスキルだけが在り、リリース用のスキルは無い。
- 移植元: `C:\home\maz\git\areka\.claude\skills\release`（`SKILL.md` + `procedure.md`）。areka の手順は、版上げ PR → squash マージ → タグ `v{版}` の push → CI の見守り → 報告、を承認待ち無しで回す。areka 固有の道具（`tools/test-all.ps1`・`package.ps1`・`crates-io.ps1`・`release-notes.ps1`）、複数クレート、`THIRD-PARTY-NOTICES.md`・`dist/README.txt`、zip 梱包・起動確認・winget、Claudia 口調の見どころに依存している。

## Desired Outcome
- `/release`（引数省略で 3 つ目の数字 +1、または `X.Y.Z` 指定）で、gtd-mcp のリリース 1 回分が承認待ち無しで回る: 版上げ → 見どころファイル → 手元の確かめ → 版上げ PR → squash マージ → タグ `v{版}` の push → crates.io 公開の走りの見守り → 見どころ全文を含む最終報告。
- タグ `v{版}` の push で、crates.io への公開が CI から Trusted Publishing（OIDC）で行われる。リポジトリにも手元にも crates.io のトークンを置かない。
- 同じ版の再実行は安全（公開済みの版は飛ばす）。
- crates.io 側の Trusted Publishing の設定手順（開発者が crates.io の画面で一度だけ行う作業）が文書になっていて、開発者がその手順に沿って設定を終えられる。

## Approach
areka の `release` スキルを gtd-mcp 用に**削って**移植し、CI は **crates.io への公開ワークフロー 1 本だけ**を新しく作る。

- **スキル**: `.claude/skills/release/SKILL.md` + `procedure.md`。areka の段の骨組み（前提確認・版上げ・手元確認・版上げ PR・タグ・見守り・報告、止まり方の決まり、再開のときの事実の読み方、外へ出るコマンドは 1 呼び出し 1 行）は保つ。areka 固有の段（`THIRD-PARTY-NOTICES.md`・`dist/README.txt`・起動確認・zip・winget・Release 本文の比較・`dola` 行・複数クレート）は落とす。
- **CI**: `.github/workflows/crates-io.yml` を新設。きっかけはタグ `v*` の push と `workflow_dispatch`（版を入力）。タグと `Cargo.toml` の版の一致を確かめ、sparse index（`https://index.crates.io/gt/d-/gtd-mcp`）で公開済みかを読み、未公開のときだけ `rust-lang/crates-io-auth-action@v1` で鍵を取って `cargo publish --locked` する。権限は `contents: read`・`id-token: write`。`secrets.` は参照しない。
- **見どころ**: 英語・普通の文体で `doc/release-notes/v{版}.md` を版上げ PR に含める。書き方の決まりは `doc/release-notes/README.md` に置く。
- **crates.io 設定の文書**: `doc/crates-io-publish.md` に Trusted Publishing の設定手順・いつもの流れ・止まったときのやり直し・トークンを使う予備の手順を書く。
- 手元の確かめの道具は最小にする（`cargo fmt --check`・`cargo clippy -D warnings`・`cargo test`・`cargo publish --dry-run` など既存の `cargo` で足りる物を優先し、スクリプトを足すのは必要と設計で判断した場合だけ）。

**選んだ理由**: 開発者の指示「crates.io に登録できれば良い。それ以外の CI は areka 用であり、不要」。areka の Release 用 CI・梱包・ライセンス表の仕組みは持ち込まない。areka で本番運用中の Trusted Publishing の形（同じオーナー `ekicyou`）を流用できるので、実現性の懸念は小さい。

**退けた案**:
- areka の道具（pwsh スクリプト 2 本・Release CI）を丸ごと簡略移植 — 開発者が crates.io 以外の CI を不要とした。
- release-plz 等の外部ツールに任せる — areka の手順とかけ離れ、スキルの移植にならない。
- API トークンを GitHub Secrets に置く — 漏洩と期限の管理が要る。Trusted Publishing を選択済み。

## Scope
- **In**:
  - `.claude/skills/release/SKILL.md`・`procedure.md`（gtd-mcp 用に移植）
  - `.github/workflows/crates-io.yml`（新設。Trusted Publishing、冪等、`workflow_dispatch` での出し直し）
  - `doc/crates-io-publish.md`（crates.io の Trusted Publishing 設定手順・運用・やり直し・予備の手順・してはいけない操作）
  - `doc/release-notes/README.md`（見どころの書き方の決まり、英語）
  - `RELEASE.md` の冒頭に「以後は `doc/release-notes/` と GitHub Releases を参照」の注記（本文は凍結）
  - 開発者の設定（自動モードの許可ルールの文例）を手順書に載せる（設定ファイル自体は触らない）
  - 開発者が crates.io の画面で Trusted Publisher を登録する作業の案内
- **Out**:
  - 既存の `release.yml`（バイナリ配布）・`ci.yml`・`security-audit.yml` の変更
  - GitHub Release の本文を見どころファイルから作る仕組み、Release の資産の検査
  - areka の `tools/*.ps1` の移植（zip 梱包・起動確認・ライセンス表・全体テストスクリプト）
  - winget 等の他の配布先
  - 過去の版（0.8.1〜0.8.3）のタグや見どころの後追い作成
  - crates.io の画面操作そのもの（開発者が行う。手順の案内まで）

## Boundary Candidates
- 公開の CI（`crates-io.yml`）: タグ → 版一致の確認 → 公開済み判定 → OIDC で鍵 → publish → 記録
- リリース手順のスキル（`release` スキル）: 手元での版上げから見守り・報告まで。CI の中身には触れず、走りの結果だけを読む
- 文書: crates.io 設定と運用（`doc/crates-io-publish.md`）、見どころの決まり（`doc/release-notes/README.md`）

## Out of Boundary
- バイナリの組み立てと GitHub Release の作成（既存の `release.yml` が持つ。この spec は触らない）
- 通常の CI（`ci.yml`）の検査内容
- 開発者の Claude Code 設定ファイル（`settings.json`）の書き換え（文例を示すだけ）

## Upstream / Downstream
- **Upstream**: crates.io の Trusted Publishing（GitHub Actions の OIDC）、`rust-lang/crates-io-auth-action@v1`、`gh` CLI、既存の `Cargo.toml` の版の置き場（`[package] version` の 1 行）、ステアリング（`tech.md`・`structure.md`）
- **Downstream**: 以後の全リリース（`/release` の運用）。将来 Release 本文の自動化や配布先の追加をするなら、別 spec でこの手順の「見直すきっかけ」を更新する

## Existing Spec Touchpoints
- **Extends**: なし（`.kiro/specs/` は空）
- **Adjacent**: なし。既存の `release.yml` はタグ `v*` で同時に走るが、この spec の CI とは独立（待ち合わせない設計を基本とし、要否は設計で決める）

## Constraints
- crates.io の Trusted Publishing は `workflow_run` きっかけを受け付けないため、`push`（タグ）と `workflow_dispatch` をきっかけにする。ワークフローのファイル名は crates.io 側の設定と一致させる（`crates-io.yml`）。
- crates.io 側の Trusted Publisher 登録（owner `ekicyou`・repo `gtd-mcp-rs`・workflow `crates-io.yml`・environment 空）は開発者が crates.io の画面で行う。登録前にタグを打つと鍵の取得で止まり、何も公開されない。
- 押したタグは動かさない・消さない。同じ版を二度出さない。`git push --tags`・`--follow-tags`・`--force` は使わない。手元からの `cargo publish` は予備の手順のときだけ。
- 外へ出るコマンド（`git push`・`gh pr create`・`gh pr merge`）は 1 呼び出し 1 行（許可ルールに当てるため）。
- 開発環境は Windows（pwsh / Git Bash）。手元の確かめは `cargo` だけで回せる形を優先する。
- Edition 2024、`Cargo.lock` はリポジトリ管理下（`cargo publish --locked` が使える）。
- 版の置き場は `Cargo.toml` の `[package] version` 1 行と `Cargo.lock` の `gtd-mcp` の 1 行。
- 文書の言語: スキル・手順書・spec は日本語、見どころファイルと `doc/release-notes/README.md` は英語。
- crates.io 上の 0.8.3 とタグ `v0.8.0` の食い違いがあるため、初回の `/release` の「前のタグが在るか」の確かめが外れる。初回の扱い（`v0.8.3` を後付けで打つか、初回だけ前のタグの確かめを緩めるか）は要件で決める。
