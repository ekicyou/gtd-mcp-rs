# Research & Design Decisions: crates-io-release

---
**Purpose**: 要件（requirements.md）と既存のコードベースの差（ギャップ分析）を記録し、設計フェーズの判断材料にする。
---

## Summary
- **Feature**: `crates-io-release`
- **Discovery Scope**: New Feature（リポジトリ運用の新設。Rust のソースコードには触れない）＋ 既存の CI・文書との境界調整
- **Key Findings**:
  - リリース用のスキル・crates.io 公開の CI・見どころの置き場は**すべて未整備**（Missing）。移植元の areka には本番運用中の同等物があり、形の大部分は流用できる。
  - **crates.io に上がる包みの中身が絞られていない**: `Cargo.toml` に `include`/`exclude` が無く、現状の `cargo package --list` には `.claude/`（54 ファイル）・`.kiro/`（23）・`.github/`・`CLAUDE.md`・`doc/` 等が入る。この spec で `.claude/skills/release/` 等を足すと、さらに公開物に混ざる。
  - **初回の前のタグが無い**: リモートのタグは `v0.2.0`・`v0.3.0`・`v0.8.0` だけ。crates.io の索引では 0.8.1（2026-02-17）・0.8.2（2026-07-21）・0.8.3（2026-07-25）が公開済み。版を 0.8.3 にしたコミットは `fe9dcab`、その後 `dbe9713`（`Cargo.lock` の追跡開始）が続く。
  - **main はブランチ保護されていない**（`protected: false`、必須チェック無し）。`gh pr merge --squash` は PR の CI を待たずに通るため、CI を待つかどうかは手順側で決める必要がある（OQ-2）。
  - 既存の `release.yml` はタグ `v*` で同時に走り、非推奨の `actions/create-release@v1`・`actions/upload-release-asset@v1` を使う。範囲外だが、タグの push のたびに並走する隣接物である。

## Research Log

### 1. 既存資産の棚卸（gtd-mcp 側）
- **Context**: 要件 1〜15 に対し、再利用できる物と欠けている物を確かめる。
- **Sources Consulted**: `Cargo.toml`、`Cargo.lock`、`.github/workflows/{ci,release,security-audit}.yml`、`RELEASE.md`、`README.md`・`README.ja-jp.md`、`doc/`、`.claude/skills/`、`.kiro/steering/`、`git ls-remote --tags origin`、`gh api repos/ekicyou/gtd-mcp-rs/branches/main`、`https://index.crates.io/gt/d-/gtd-mcp`
- **Findings**:
  - 版の置き場は `Cargo.toml` の `[package] version`（1 行、`0.8.3`）と `Cargo.lock` の `name = "gtd-mcp"` の直後の `version` 行（374〜375 行目付近）の 2 か所だけ。単一クレートなので areka の「`[workspace.package]` と `dola` の 2 行」「`N` 個のパッケージ」の判定は不要になる。
  - `Cargo.lock` は追跡済み（`dbe9713`）。`--locked` を前提にできる。
  - `cargo set-version`（cargo-edit 0.13.13）は開発者の手元に在る。`gh` 2.102.0 も在る。
  - `doc/` には英語・日本語混在の設計メモ（`BRANCH_PROTECTION.md`・`CI_SUMMARY.md` 等）が在るが、`doc/release-notes/` と `doc/crates-io-publish.md` は無い。
  - `.claude/skills/` には kiro 系・ponytail 系だけが在り、`release` は無い。
  - steering には roadmap が無い（areka 手順の 1.4「申し送り」を受ける場所が無い → OQ-5）。
  - `README.md`（199 行目）・`README.ja-jp.md`（213 行目）が `RELEASE.md` を「全版のリリースノート」と案内している（→ OQ-6）。
  - 最近の main への変更には PR 番号の無い直接コミットが混ざる（例: `fe9dcab`・`dbe9713`・`1e848e0`）。areka の見どころの「見出し末尾に PR 番号」の決まりはそのままでは当てはまらない。
- **Implications**: スキル・CI・文書の 4 種は新設。既存ファイルの変更は `RELEASE.md` 冒頭の注記だけ（＋ OQ-7 で決まれば `Cargo.toml` の包みの範囲）。

### 2. 移植元（areka）との差分
- **Context**: brief は「areka の release スキルを削って移植」。何を保ち、何を落とすかを要件に照らす。
- **Sources Consulted**: `areka/.claude/skills/release/SKILL.md`・`procedure.md`、`areka/.github/workflows/crates-io.yml`、`areka/tools/crates-io.ps1`、`areka/doc/crates-io-publish.md`、`areka/doc/release-notes/README.md`（読み取りのみ）
- **Findings**:

  | areka の段・物 | gtd-mcp での扱い | 対応要件 |
  |---|---|---|
  | 読み方・してはいけない事・止まり方の決まり | 保つ（語の置き換えのみ） | 9 |
  | 1.1 作業木・main の取り込み・「確かめた main」 | 保つ | 2.1〜2.4 |
  | 1.2 根の 3 ファイルを触る開いた PR | 対象を `Cargo.toml`・`Cargo.lock` の 2 つに縮める | 2.5 |
  | 1.3 版 2 行の一致・`v{旧版}` の存在 | 版 1 行。`v{旧版}` 不在時の初回の緩め（OQ-1）を足す | 2.7〜2.9 |
  | 1.4 roadmap の申し送り | 落とす（OQ-5） | — |
  | 1.5 次の版・`v{版}` 不在 | 保つ | 1.1〜1.4, 2.6 |
  | 2.1 `cargo set-version --workspace` | 単一クレートなので `--workspace` 不要。代わりのやり方（Edit＋`cargo update -w`）も保てる | 3.1 |
  | 2.2 `dist/README.txt` の日付 | 落とす | — |
  | 2.3 `tools/release-notes.ps1 -Notes` で変更一覧 | 道具は移植しない。`git log --first-parent {起点}..{確かめた main}` を直接読む形が候補 | 3.2, 3.3 |
  | 3.1 `tools/test-all.ps1 -License`（約 19 分） | steering の CI チェック 5 種の `cargo` コマンドに置き換え | 4.1 |
  | 3.2 差分の範囲の表 | 3 ファイル（`Cargo.toml`・`Cargo.lock`・見どころ）に縮める | 4.3 |
  | 3.3 起動確認（窓）・`package.ps1` | 落とす | — |
  | 3.4 `crates-io.ps1 -Verify` | `cargo publish --dry-run --locked`（または `cargo package --locked`）に置き換え候補 | 4.2 |
  | 3.5 `release-notes.ps1 -Check` | 道具を移植しないため、形の検査の手段が Missing（下の「Research Needed」） | 4.4 |
  | 段 4 PR・main 移動時の取り込み・squash | 保つ。CI 待ちの追加は OQ-2 | 5 |
  | 段 5 タグ | 保つ | 6 |
  | 6.1/6.2 Release と crates-io の 2 本の見守り・Release 本文比較・winget | crates-io の 1 本だけに縮める（OQ-4） | 7 |
  | 6.3 赤の表 | `release.yml` の行を落とし、公開の走りの行を中心に作り直す | 7.4〜7.6 |
  | 段 7 報告（見どころ全文） | 保つ | 8 |
  | 公開後の見どころの直し（`gh release edit` 含む） | `gh release edit` を落とし、main のファイルの直し PR だけにする（OQ-3） | 10 |
  | 再開の表 | 保つ（題 `chore(release): v` の PR を基準） | 9.1〜9.4 |
  | 開発者の設定（autoMode.allow の文例） | 保つ。`gh release edit` 等を外した文例に直す | 9.10 |
  | `crates-io.yml` の「release を待つ」「Release の確認」 | brief の基本方針は「待ち合わせない」。設計で要否を決める | 12.9 |
  | `crates-io.yml` の判定を `tools/crates-io.ps1` に集める形 | スクリプトは原則足さない（brief）。判定（版の形・版の一致・索引での公開済み判定）をワークフロー内に直接書くか、最小のスクリプトを足すかは設計で決める | 11, 12 |
  | 見どころ README（日本語・Claudia 口調・PR 番号必須） | 英語・普通の文体に書き直す。PR 番号の無い直接コミットの扱いを決める | 14 |

- **Implications**: 段の骨組みはほぼそのまま移せる。新しく考える必要があるのは (a) 初回の前のタグ不在、(b) 見どころの形の検査手段、(c) CI の判定ロジックの置き場、(d) `release.yml` との並走の扱い、の 4 点。

### 3. crates.io Trusted Publishing と公開ワークフローの形
- **Context**: 要件 11・12（タグ／手動起動・冪等・OIDC・Secrets 不使用・最小権限・直列化）。
- **Sources Consulted**: `areka/.github/workflows/crates-io.yml`（本番運用中、同じオーナー `ekicyou`）、`areka/doc/crates-io-publish.md` 2 節・5 節
- **Findings**:
  - Trusted Publishing はきっかけとして `workflow_run` を受け付けない（brief の制約）。`push`（tags `v*`）＋`workflow_dispatch`（input `version`）の 2 つで areka と同じ形にできる。
  - 鍵の受け取りは `rust-lang/crates-io-auth-action@v1`、権限は `contents: read`・`id-token: write`。`release.yml` の状態を読まない設計なら areka の `actions: read` は不要。
  - 公開済み判定は sparse index（`https://index.crates.io/gt/d-/gtd-mcp`、1 行 1 版の JSON）を読めばよい。今の索引は 0.8.3 まで在り、`yanked: false`。
  - Trusted Publishing はクレートが既に crates.io に在ることが前提（最初の版は手で出す必要）。`gtd-mcp` は既存なので問題ない。
  - タグの push で動く走りと、その「Re-run」は**タグのコミットに在る**ワークフローを使う。`workflow_dispatch` は既定の枝（main）のファイルで動く。ワークフローの誤りを直した後のやり直しは「Run workflow」で行う、という areka の切り分けがそのまま当てはまる。
  - `workflow_dispatch` はワークフローが main に在る間だけ使える。このフィーチャーの実装 PR が main に入った後の最初のタグから自動公開が効く。
  - areka は `windows-latest`＋pwsh。gtd-mcp の `release.yml`・`ci.yml` は `ubuntu-latest` が中心。公開には OS 依存の要素が無いので、どちらでも成り立つ（設計で選ぶ）。
- **Implications**: CI の実現性リスクは低い。判定の置き場（YAML 内のシェル vs 小さなスクリプト）と runner の OS が設計の主な選択。

### 4. crates.io に上がる包みの中身
- **Context**: 要件 4.2・12.3（公開できる形に包めること）。
- **Sources Consulted**: `cargo package --list --allow-dirty`（手元、ビルド無し）
- **Findings**: 現状の包みに、`.claude/`（54）・`.kiro/`（23）・`.github/`（4）・`CLAUDE.md`・`doc/`（6）・`FEATURES_JA.md`・`MCP_PROTOCOL_TEST_RESULTS.md`・`RELEASE.md`・`tests/` 等が入る。`Cargo.toml` に `include`/`exclude` が無いため。0.8.3 以前の公開物にも同様に入っていた可能性が高い。
- **Implications**: 機能上の不具合ではないが、開発用の skill・spec・steering が crates.io に配られ続ける。直すには `Cargo.toml` に `include`（または `exclude`）を足す変更が要り、brief の「版の置き場は `[package] version` の 1 行」とは別の、このフィーチャーの実装 PR での一度きりの変更になる。範囲に入れるかを決める必要がある（OQ-7）。

### 5. 初回の `/release` の扱い（OQ-1 の材料）
- **Context**: 要件 2.7〜2.9・3.3。
- **Findings**:
  - 案 A（タグ後付け）: `v0.8.3` を `fe9dcab`（または 0.8.3 を実際に包んだコミット）に開発者が手で打つ。タグの push で、そのコミットに在る `release.yml` が走り GitHub Release `v0.8.3` とバイナリが作られる（副作用。brief の Out「過去の版のタグの後追い作成」に当たる）。そのコミットには `crates-io.yml` が無いので公開の走りは起きない。
  - 案 B（初回だけ緩める・仮置き）: 「`v{旧版}` 無し・`chore(release): v{旧版}` のマージ済み PR 無し・crates.io に {旧版} 在り」の 3 条件が揃うときだけ進む。2 回目以降は `v{旧版}` が必ず在るので、この分岐は通らない。変更一覧の起点は別途決める必要がある。
  - 起点の候補: (i) 版を {旧版} にしたコミット（`git log -S'version = "0.8.3"' -- Cargo.toml` → `fe9dcab`）、(ii) 最後に在るタグ `v0.8.0`（0.8.1〜0.8.3 の変更も一覧に混ざる）、(iii) crates.io の公開物に入っている `.cargo_vcs_info.json` の `git.sha1`（公開時の正確なコミット。取得には crate のダウンロードが要る）。
- **Implications**: 案 B は手順の分岐が 1 つ増えるだけで副作用が無い。起点は (i) が最も軽い。決定は要件ディスカッションへ。

## Requirement-to-Asset Map

| 要件 | 既存の資産 | ギャップ | 種別 |
|---|---|---|---|
| 1 起動と次の版 | `cargo set-version`（手元に在る） | `/release` スキル本体 | Missing |
| 2 前提の確かめ | `git`・`gh`・タグ（`v0.8.0` まで） | 手順・初回の前タグ不在の扱い | Missing / Constraint（OQ-1） |
| 3 版上げと見どころ | 版の置き場 2 か所 | 見どころの置き場・変更一覧の出し方・初回の起点 | Missing / Unknown |
| 4 手元の確かめ | steering の CI チェック 5 種（`cargo` のみ） | 公開前の包みの確認の手順化、見どころの形の検査手段、差分範囲の判定 | Missing / Unknown |
| 5 PR と squash | `gh`、ブランチ保護無し | CI 待ちの有無（保護が無いので手順側で担保） | Constraint（OQ-2） |
| 6 タグ | — | 手順 | Missing |
| 7 見守り | `gh run list/watch/view` | 公開の走りの見つけ方・緑の判定（索引） | Missing |
| 8 最終報告 | — | 手順 | Missing |
| 9 再開・止まり方 | areka の再開の表 | gtd-mcp 用への縮約、autoMode の文例 | Missing |
| 10 見どころの直し | — | 手順（Release 本文は触らない） | Missing（OQ-3） |
| 11 きっかけと版の確かめ | `release.yml` が同じ `v*` で並走 | `crates-io.yml` 新設 | Missing / Constraint（並走） |
| 12 冪等・OIDC | crates.io の既存クレート（0.8.3） | 公開済み判定・鍵・公開・記録・直列化、包みの中身（OQ-7） | Missing / Constraint |
| 13 公開手順書 | areka の同書（雛形） | gtd-mcp 用の書き直し | Missing |
| 14 見どころの決まり | areka の同書（日本語・口調） | 英語・普通の文体・PR 番号無しの参照の扱い | Missing / Unknown |
| 15 RELEASE.md 凍結 | `RELEASE.md`（v0.8.0 まで） | 冒頭の注記。README の案内（OQ-6） | Missing / Constraint |

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|---|---|---|---|---|
| A: 既存を拡張 | `release.yml` に crates.io 公開ジョブを足し、手順は README 等に追記 | ファイル数が最少 | brief の Out（既存 `release.yml` を触らない）に反する。Trusted Publishing の workflow 名が `release.yml` になり、Release 側の失敗と公開が結合する | 採らない前提 |
| B: 新設のみ（brief の方針） | `crates-io.yml`・`.claude/skills/release/`・`doc/crates-io-publish.md`・`doc/release-notes/README.md` を新設し、既存は `RELEASE.md` の注記だけ | 既存 CI と完全に独立。areka の本番の形を流用できる | 判定ロジックの置き場を決める必要。`release.yml` とタグで並走する | 推奨の基本線 |
| C: ハイブリッド | B に加えて、判定（版の形・版の一致・公開済み判定・見どころの形の検査）を 1 本の小さなスクリプトに集め、手元と CI で同じ物を使う | 手元と CI の判定が一致。areka の `crates-io.ps1` の考え方に近い | brief の「スクリプトは設計で必要と判断した場合だけ」に照らして要否の説明が要る。pwsh 前提だと CI の runner が Windows に寄る | 見どころの形の検査を機械化したい場合の候補 |

## Design Decisions（設計フェーズへの持ち越し）

### Decision: 判定ロジックの置き場
- **Context**: 要件 4.2・4.4・11.3・11.4・12.1。
- **Alternatives Considered**:
  1. ワークフローの YAML の中に直接書く（版の形は正規表現、版の一致は `cargo metadata` か `Cargo.toml` の読み取り、公開済みは索引を `curl` で読む）。手元の確かめは `cargo` コマンドと主文脈の読み取りだけ。
  2. 小さなスクリプト 1 本に集め、手元と CI の両方から呼ぶ。
- **Selected Approach**: 未決（設計で決める）。brief は 1 を優先。
- **Follow-up**: 見どころの形の検査（要件 4.4）を機械で行うか、主文脈の読み取りだけで行うか。

### Decision: `release.yml` との並走
- **Context**: 要件 12.9、境界の隣接期待。
- **Alternatives Considered**: 1. 待ち合わせない（brief の基本方針）、2. areka と同様に `release.yml` の緑を待つ。
- **Selected Approach**: 未決。待ち合わせない場合、`release.yml` が赤でも crates.io には出る（Release とバイナリが欠けたまま crates.io だけ進む状態があり得る）ことを手順書に書く必要がある。

### Decision: 公開ワークフローの runner と手元の確かめの道具
- **Context**: 開発環境は Windows（pwsh / Git Bash）、既存 CI は ubuntu 中心。
- **Alternatives Considered**: ubuntu-latest＋bash／windows-latest＋pwsh。
- **Selected Approach**: 未決。

### Decision: 見どころの参照の書き方（PR 番号の無い変更）
- **Context**: 要件 14.3。main には PR 番号の無い直接コミット（`fe9dcab`・`dbe9713` 等）が混ざる。areka の「見出しの末尾に `(#N)` 必須」はそのままでは当てはまらない。
- **Alternatives Considered**: 1. PR 番号が在れば `(#N)`、無ければ短いコミット SHA で参照する、2. PR 番号の在る変更だけを見どころにする。
- **Selected Approach**: 未決（設計で決め、Highlights Guide に書く）。

### Decision: 公開の走りの特定と待ち方
- **Context**: 要件 7.1・7.2。
- **Alternatives Considered**: `gh run list --workflow crates-io.yml --event push` を `headBranch`＝`v{版}`・`headSha`＝squash のコミットで絞る（areka と同じ）／`gh run watch --exit-status` で待つ。
- **Selected Approach**: 未決（Research Needed の結果で決める）。

### Decision: 初回の変更一覧の起点
- **Context**: 要件 3.3（OQ-1 の決定に従う）。
- **Alternatives Considered**: 5 節の (i)〜(iii)。
- **Selected Approach**: 要件ディスカッションで (i) に確定（`Cargo.toml` の版を {旧版} にしたコミット＝初回は `fe9dcab` の次から数える。要件 3.3）。設計では、そのコミットを手順の中でどう見つけるか（例: `git log -S` で版の行を追う）を決める。

## Implementation Complexity & Risk
- **Effort: M（3〜7 日）** — 新設が 4 ファイル＋スキル 2 ファイルで、どれも雛形（areka）が在る。ただし手順書（`procedure.md`）の縮約と再開の表の作り直し、初回の扱いの分岐、見どころの形の決まりの英語化に手間がかかる。
- **Risk: Low〜Medium** — Trusted Publishing の形は同じオーナーの areka で本番運用中（Low）。一方、取り消せない操作（タグ・公開）を承認待ち無しで回す手順であり、初回は前のタグ不在・Trusted Publisher 未登録・包みの中身の問題が重なる（Medium 寄り）。実装後の最初の `/release` が実地の検証になる。

## Risks & Mitigations
- Trusted Publisher 未登録のままタグを押す — 鍵の受け取りで止まり何も公開されない（安全側）。手順書（要件 13.2）と、スキルの前提確認に「登録済みか」を開発者に確かめる手段が無い点を設計で検討（crates.io の設定は API から読めない可能性 → Research Needed）。
- 初回の前タグ不在で手順が止まる — 要件 2.8 の仮置き（OQ-1）で回避。
- 包みに開発用ファイルが入る — OQ-7 で範囲を決める。
- `release.yml`（非推奨アクション）が赤になる — 範囲外。公開とは独立のまま、最終報告で触れるかは OQ-4。
- 版上げ PR を CI 未完了でマージ — ブランチ保護が無いため手順側で待つ（OQ-2 の仮置き）。

## Research Needed（設計フェーズで調べる）
- 見どころの形の検査を `cargo`・`git`・`gh` だけで行えるか（要件 4.4）。
- 手元の公開前確認として `cargo publish --dry-run --locked` と `cargo package --locked` のどちらを使うか（所要時間・索引へのアクセス・dirty な作業木での振る舞い）。
- `gh run list --workflow crates-io.yml` でタグの push の走りを確実に特定する条件（`headBranch`＝タグ名、`headSha`＝squash のコミット）。
- crates.io の Trusted Publisher の登録状態を手元から確かめる手段の有無。
- 初回の変更一覧の起点（`fe9dcab` か、公開物の `.cargo_vcs_info.json` の sha か）。
- `include`/`exclude` を足した場合の包みの中身（README・LICENSE・`src/`・`Cargo.lock` が残ること）。

## References
- `C:\home\maz\git\areka\.claude\skills\release\SKILL.md`・`procedure.md` — 移植元の手順（読み取りのみ）
- `C:\home\maz\git\areka\.github\workflows\crates-io.yml` — Trusted Publishing の本番の形
- `C:\home\maz\git\areka\doc\crates-io-publish.md` — 設定・やり直し・予備の手順の雛形
- `C:\home\maz\git\areka\doc\release-notes\README.md` — 見どころの決まりの雛形（日本語・口調付き）
- `https://index.crates.io/gt/d-/gtd-mcp` — crates.io の sparse index（0.8.3 まで公開済みを確認）
