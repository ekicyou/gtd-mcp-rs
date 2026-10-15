# Research & Design Decisions: crates-io-release

---
**Purpose**: 要件（requirements.md）と既存のコードベースの差（ギャップ分析）を記録し、設計フェーズの判断材料にする。
---

## Summary
- **Feature**: `crates-io-release`
- **Discovery Scope**: New Feature（リポジトリ運用の新設。Rust のソースコードには触れない）＋ 既存の CI・文書との境界調整（設計フェーズ: Light discovery。結果は「Design Phase Discovery」以下）
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
  | 段 4 PR・main 移動時の取り込み・squash | 保つ。PR の CI は待たない（OQ-2 で確定） | 5 |
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

## Design Phase Discovery（設計フェーズの追加調査）

- **Discovery Type**: Light（統合に寄せた調査）。移植元 areka の本番運用中の形が在り、外部依存（Trusted Publishing・`crates-io-auth-action@v1`・sparse index）は areka で実証済みのため、Web 調査は行わず移植元の読み取りとリポジトリの実測で判断した。
- **Consulted skills / rules**: `kiro-spec-design`（`rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）、テンプレート `.kiro/settings/templates/specs/design.md`・`research.md`。steering `product.md`・`tech.md`・`structure.md`（CI チェック 5 種を手元の確かめの基準に採用、言語の使い分け）。

### 6. 移植元のワークフローと手順の再読（設計用）
- **Sources Consulted**: `areka/.github/workflows/crates-io.yml`（181 行）、`areka/.claude/skills/release/SKILL.md`・`procedure.md`（286 行）、`areka/doc/crates-io-publish.md`、`areka/doc/release-notes/README.md`、`areka/tools/crates-io.ps1`（冒頭）。
- **Findings**:
  - areka の公開の段は「版の形 → release を待つ → 取り出し → Release の確認 → 道具 → 公開前の確認 → 残りの判定 → 鍵 → 公開 → 記録」。gtd-mcp では「release を待つ」「Release の確認」を落とし（待ち合わせない）、「タグの確かめ」（手動起動のみ。要件 11.5）を足し、残りの判定を単一クレートの 3 値（present/absent/unreadable）に縮める。
  - areka は「公開前の確認」を「残りの判定」の前に置くが、要件 12.1〜12.3 は「公開済みなら鍵を取らずに成功」「無いときに包む確かめ → 鍵 → 公開」の順を求めるため、gtd-mcp では公開済み判定を先に置く（公開済みの版の起動し直しが組み立て無しで終わる利点もある）。
  - タグ名・入力は `env:` で受けてから正規表現で確かめる（要件 11.6）。areka の形をそのまま使う。
  - areka の「記録」は索引の反映を待たず、反映待ちは文書の切り分け（Re-run）に任せている。
  - areka の手順は「判定は主文脈が出力を読む・スクリプトを足さない」で、道具（`tools/*.ps1`）は既存の物だけを使う。gtd-mcp には道具が無いので、`cargo`・`git`・`gh`・`curl`・`grep` のコマンドを直接書く。
- **Implications**: スクリプト無しで全判定を書ける。areka の段の名前の流儀（日本語の短い名前）を保つと、Publishing Guide の切り分け表と 1 対 1 にできる。

### 7. gtd-mcp の実測（設計用）
- **Sources Consulted**: `Cargo.toml`、`git ls-files`、`src/` の `include_str!` の有無、`.github/dependabot.yml`、`.github/workflows/ci.yml`、`git log --first-parent -S'version = "0.8.3"' origin/main -- Cargo.toml`、`gh pr list --state open`。
- **Findings**:
  - `src/` に `include_str!`・`include_bytes!` は無い。包みは `src/**/*.rs`・`Cargo.toml`・`Cargo.lock`・`README.md`・`LICENSE` で足りる（`[lib]`・`[[bin]]` は `src/` 内）。
  - `git log --first-parent -S'version = "0.8.3"' origin/main -- Cargo.toml` は `fe9dcab` 1 行を返す。初回の起点をこのコマンドで機械的に見つけられる（要件 3.3）。
  - `dependabot.yml` は cargo の週次更新を設定しており、`Cargo.toml`・`Cargo.lock` を触る PR が開きうる（調査時点で開いているのは `chore(ci)` の 2 本だけで、版の置き場には当たらない）。要件 2.5 により、cargo の更新 PR が開いている間は `/release` が段 1.2 で止まる。
  - 既存 `ci.yml` は `actions/checkout@v5` と `dtolnay/rust-toolchain` を使う。公開の走りは checkout の版を揃え、ツールチェーンはランナーの `rustup update stable --no-self-update`（areka と同じ。第三者のアクションを増やさない）にする。
  - タグ `v0.8.0` のコミットには `Cargo.lock` が無い（`dbe9713` で追跡開始）。手動起動で `0.8.0` を渡す公開を伴わない確かめを成り立たせるため、版の一致の段は `--locked` を付けない `cargo metadata --no-deps` で版を読む。

## Synthesis（設計の統合）

- **Generalization**: 「crates.io に `gtd-mcp` {版} が在るか」は、スキルの初回判定（要件 2.8）・緑の判定（7.3）、ワークフローの公開済み判定（12.1, 12.2）と記録（12.5）、文書の確かめ方（13.3）の 5 か所で同じ問いである。1 つの契約（sparse index・`vers` の字のとおりの一致・`yanked` は問わない・3 値）として design.md に定義し、各所はそれを使う。同様に「見どころの形」は Highlights Guide が定め、スキルの段 3.2 と見どころの直しが同じ確かめを使う。
- **Build vs. Adopt**: 公開の認証は `rust-lang/crates-io-auth-action@v1`（採用）。版上げは `cargo-edit` を使わず `Cargo.toml` の 1 行の Edit＋`cargo update --workspace`（単一クレートで 1 行だけのため、道具の有無の分岐を無くす）。release-plz 等の外部ツールは brief で退け済み。判定用のスクリプトは作らない（brief の優先、判定が小さい）。
- **Simplification**: areka の段のうち、申し送り・`dist/README.txt`・`THIRD-PARTY-NOTICES.md`・起動確認・Release の走りと本文の比較・winget・複数クレートの残りの判定・`release.yml` の待ち合わせ・`actions: read` を落とした。版上げのやり方は 1 通りにした。見どころの形の機械の確かめは道具を作らず `grep` 4 つにした。

## Design Decisions

### Decision: 判定ロジックの置き場
- **Context**: 要件 4.2・4.4・11.3・11.4・12.1。
- **Alternatives Considered**:
  1. ワークフローの `run:` と `procedure.md` のコマンドに直接書く。
  2. 小さなスクリプト 1 本に集め、手元と CI の両方から呼ぶ。
- **Selected Approach**: 1（仮置き。design.md OPEN QUESTION 3）。
- **Rationale**: 単一クレートで判定は「正規表現・`cargo metadata` の 1 欄・索引の 1 行の一致」だけ。スクリプトにすると Windows（手元）と Linux（CI）の両方で動く言語の選択と保守が増える。
- **Trade-offs**: 手元と CI の判定は「同じ契約を別の場所に書く」形になり、契約の変更時は 2 か所を直す（Revalidation Triggers に記載）。

### Decision: `release.yml` との並走
- **Context**: 要件 12.9、境界の隣接期待。
- **Alternatives Considered**: 1. 待ち合わせない（brief の基本方針）、2. areka と同様に `release.yml` の緑を待つ。
- **Selected Approach**: 1（仮置き。OPEN QUESTION 2）。`release.yml` が赤でも crates.io には出ることを Publishing Guide 3 節に明記する。
- **Rationale**: 開発者の指示は「crates.io に登録できれば良い」。`release.yml` は非推奨アクションを使い範囲外で、その赤に公開を巻き込まない。権限も `actions: read` を足さずに済む。
- **Trade-offs**: GitHub Release とバイナリが欠けたまま crates.io だけが進む状態があり得る。

### Decision: 公開ワークフローの runner と手元の確かめの道具
- **Context**: 開発環境は Windows（pwsh / Git Bash）、既存 CI は ubuntu 中心。
- **Alternatives Considered**: ubuntu-latest＋bash／windows-latest＋pwsh。
- **Selected Approach**: `ubuntu-latest`＋bash（仮置き。OPEN QUESTION 1）。手元の確かめは steering の CI チェック 5 種＋`cargo publish --dry-run --locked --allow-dirty`（OPEN QUESTION 10）と `git`・`gh`・`curl`・`grep`。
- **Rationale**: 公開に OS 依存が無く、Linux ランナーは起動が速い。既存 CI と揃う。`curl`・`jq` がランナーに在る。
- **Trade-offs**: areka の pwsh の行をそのままは写せず、bash に書き直す手間が要る。

### Decision: 見どころの参照の書き方（PR 番号の無い変更）
- **Context**: 要件 14.3。main には PR 番号の無い直接コミットが混ざる。
- **Alternatives Considered**: 1. PR 番号が在れば `(#N)`、無ければ 7 桁の短い SHA、2. PR 番号の在る変更だけを見出しにする。
- **Selected Approach**: 1（仮置き。OPEN QUESTION 4）。正規表現 `^### .+ \((#[0-9]+|[0-9a-f]{7})(, (#[0-9]+|[0-9a-f]{7}))*\)$`。
- **Rationale**: 直接コミットで入った利用者向けの修正（例: `fe9dcab` の `--sync-git` の修正）を見出しにできないと「必ず載せるもの」と衝突する。
- **Trade-offs**: リポジトリのファイル表示では `#N` も SHA も自動リンクにならない（参照は文字として読む）。

### Decision: 見どころの形の機械の確かめ
- **Context**: 要件 4.4・14.5。areka の `release-notes.ps1 -Check` は移植しない。
- **Selected Approach**: `grep` で 4 つ（`### ` 行が 1 以上・`^#{1,2} ` が 0・見出し末尾の参照・非 ASCII が 0）（仮置き。OPEN QUESTION 5）。英語・普通の文体・「必ず載せるもの」は読み直しで確かめる。
- **Trade-offs**: ASCII のみの決まりは記号（en dash・矢印など）の自由度を失う代わりに、日本語の混入を機械で検出できる。

### Decision: 公開の走りの特定と待ち方
- **Context**: 要件 7.1・7.2・7.3。
- **Selected Approach**: `gh run list --workflow crates-io.yml --event push --json databaseId,headSha,headBranch,status,conclusion` を `headBranch`＝`v{版}`・`headSha`＝`{sha}` で絞る（areka と同じ。タグの push の走りの `headBranch` はタグ名）。30 秒ごと最長 5 分で見つからなければ止まる。`gh run watch {id} --exit-status --interval 30` を裏で回して待つ。緑は `conclusion`＝`success` かつ公開済み判定 `present`（`absent` は 30 秒ごと最長 5 分読み直す。OPEN QUESTION 8）。
- **Trade-offs**: Re-run は同じ `databaseId` の新しい試行になるので再開時も同じ走りを読める。手動起動の走りは `--event push` に出ないため、手動起動で回復した版は「push の走りは赤・crates.io は present」と見え、開発者の判断（7.4）に回る。

### Decision: 初回の変更一覧の起点
- **Context**: 要件 3.3（OQ-1 で (i) に確定）。
- **Selected Approach**: `git log --first-parent -S'version = "{旧版}"' --format=%H {確かめた main} -- Cargo.toml` の 1 行目（最も新しいコミット）を `{起点}` とし、`{起点}..{確かめた main}` を読む。実測で `fe9dcab` を返す。

### Decision: 包みの範囲
- **Context**: 要件 16（OQ-7 で「絞る」に確定。一覧は設計で決める）。
- **Alternatives Considered**: 1. `include`（許す一覧）、2. `exclude`（外す一覧）。
- **Selected Approach**: 1、`include = ["src/**/*.rs", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE"]`（仮置き。OPEN QUESTION 6）。
- **Rationale**: 今後 `.claude/` 等に何が足されても包みに混ざらない。`tests/` は利用に要らない。
- **Trade-offs**: 新しい種類のファイル（`build.rs`・`include_str!` の対象）を足すときは `include` も直す必要がある（Revalidation Triggers）。

### Decision: 版上げのやり方
- **Context**: 要件 3.1。
- **Selected Approach**: `Cargo.toml` の `version = "{旧版}"` 1 行を Edit で直し、`cargo update --workspace` で `Cargo.lock` を揃える（仮置き。OPEN QUESTION 7）。正しく上がったかは差分の範囲の判定（段 3.1）が持つ。
- **Rationale**: 単一クレートで 1 行だけ。`cargo-edit` の有無による分岐を無くす。

### Decision: 公開済みの判定順と「記録」の待ち
- **Context**: 要件 12.1〜12.5。
- **Selected Approach**: 版の一致の直後に公開済み判定を置き、`present` なら包む確かめ・鍵・公開を飛ばす。「記録」は `always()` で公開済み判定を行い、「公開」が成功した回だけ 30 秒ごと最長 5 分読み直す（OPEN QUESTION 8）。
- **Rationale**: 要件の順序（公開済みなら鍵を取らない）に従う。反映の遅れによる偽の赤を減らす。

## Implementation Complexity & Risk
- **Effort: M（3〜7 日）** — 新設が 4 ファイル＋スキル 2 ファイルで、どれも雛形（areka）が在る。ただし手順書（`procedure.md`）の縮約と再開の表の作り直し、初回の扱いの分岐、見どころの形の決まりの英語化に手間がかかる。
- **Risk: Low〜Medium** — Trusted Publishing の形は同じオーナーの areka で本番運用中（Low）。一方、取り消せない操作（タグ・公開）を承認待ち無しで回す手順であり、初回は前のタグ不在・Trusted Publisher 未登録・包みの中身の問題が重なる（Medium 寄り）。実装後の最初の `/release` が実地の検証になる。

## Risks & Mitigations
- Trusted Publisher 未登録のままタグを押す — 鍵の受け取りで止まり何も公開されない（安全側）。crates.io の Trusted Publisher の設定は手元から確かめる手段が無い（設計判断: スキルの前提に入れず、Publishing Guide 2 節で順序を案内し、Re-run で回復）。
- 初回の前タグ不在で手順が止まる — 要件 2.8 の分岐で回避（起点は `git log -S` で求める）。
- 包みに開発用ファイルが入る — `include` で絞る（要件 16）。
- `release.yml`（非推奨アクション）が赤になる — 範囲外。待ち合わせないので公開には影響しない。
- 版上げ PR を CI 未完了でマージ — OQ-2 で「待たない」に確定。Linux・macOS だけの失敗は公開前に検出されないリスクを受け入れる。
- dependabot の cargo 更新 PR が開いていると段 1.2 で止まる — 要件 2.5 のとおり。運用で先に片づける（緩めるなら要件の変更。design.md OPEN QUESTION 9）。
- `concurrency` は待ちを 1 本しか保てない — 冪等なので Run workflow で回復できる。

## Research Needed（設計フェーズでの結論）
- 見どころの形の検査を道具無しで行えるか → `grep` 4 つで行う（上の Decision）。
- `cargo publish --dry-run --locked` と `cargo package --locked` → 手元・CI とも `cargo publish --dry-run --locked`（手元は作業木が汚れているので `--allow-dirty`）。包みの範囲の確かめには `cargo package --list` を使う。
- 走りを特定する条件 → `headBranch`＝`v{版}`・`headSha`＝`{sha}`（上の Decision）。
- Trusted Publisher の登録状態を手元から確かめる手段 → 前提に入れない（Risks & Mitigations）。
- 初回の起点 → `fe9dcab`（`git log -S` で機械的に求める。`.cargo_vcs_info.json` は使わない）。
- `include` を足した後の包みの中身 → 実装時に `cargo package --list` で確かめる（design.md Testing Strategy）。

## Requirement-to-Asset Map

| 要件 | 既存の資産 | ギャップ | 種別 |
|---|---|---|---|
| 1 起動と次の版 | `cargo set-version`（手元に在る） | `/release` スキル本体 | Missing |
| 2 前提の確かめ | `git`・`gh`・タグ（`v0.8.0` まで） | 手順・初回の前タグ不在の扱い | Missing / Constraint（OQ-1） |
| 3 版上げと見どころ | 版の置き場 2 か所 | 見どころの置き場・変更一覧の出し方・初回の起点 | Missing / Unknown |
| 4 手元の確かめ | steering の CI チェック 5 種（`cargo` のみ） | 公開前の包みの確認の手順化、見どころの形の検査手段、差分範囲の判定 | Missing / Unknown |
| 5 PR と squash | `gh`、ブランチ保護無し | PR の CI は待たない（OQ-2 で確定） | Constraint（OQ-2） |
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

**設計フェーズの選択**: 案 B（新設のみ・判定は各ファイルに直接書く）。案 C のスクリプトは、単一クレートで判定が小さく、手元（Windows）と CI（Linux）の両方で動かす保守が見合わないため採らない（Design Decisions「判定ロジックの置き場」）。

## References
- `C:\home\maz\git\areka\.claude\skills\release\SKILL.md`・`procedure.md` — 移植元の手順（読み取りのみ）
- `C:\home\maz\git\areka\.github\workflows\crates-io.yml` — Trusted Publishing の本番の形
- `C:\home\maz\git\areka\doc\crates-io-publish.md` — 設定・やり直し・予備の手順の雛形
- `C:\home\maz\git\areka\doc\release-notes\README.md` — 見どころの決まりの雛形（日本語・口調付き）
- `https://index.crates.io/gt/d-/gtd-mcp` — crates.io の sparse index（0.8.3 まで公開済みを確認）
