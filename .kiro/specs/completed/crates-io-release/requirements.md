# Requirements Document

## Project Description (Input)
gtd-mcp の開発者（ekicyou）は、リリースのたびに版上げ・タグ打ち・`cargo publish` を手作業で行っている。crates.io には 0.8.3 まで公開済みだが Git のタグは `v0.8.0` までしか無く、crates.io の版とタグ・リポジトリの状態が食い違っている。手作業のため、版の付け忘れ・タグの打ち忘れ・手元のトークンでの公開といった事故の余地が残る。

これを、areka の `release` スキルを gtd-mcp 用に削って移植した `/release` スキル（版上げ → 見どころファイル → 手元の確かめ → 版上げ PR → squash マージ → タグ `v{版}` の push → crates.io 公開の走りの見守り → 最終報告を承認待ち無しで回す）と、タグ push をきっかけに Trusted Publishing（OIDC）で crates.io へ冪等に公開する CI ワークフロー 1 本（`.github/workflows/crates-io.yml`）に置き換える。あわせて crates.io 側の Trusted Publishing 設定手順・運用・やり直し・予備手順の文書、英語の見どころファイルの書き方の決まり、`RELEASE.md` を凍結する注記を用意する。既存の `release.yml`・`ci.yml`・`security-audit.yml` は変更しない。詳細は `brief.md` を参照。

## Introduction
本仕様は、gtd-mcp のリリース 1 回分（版上げから crates.io への公開確認まで）を、開発者の承認待ち無しに安全に回せるようにする。成果物は次の 6 つである。

- **Release Skill**: `/release` スキル（`.claude/skills/release/SKILL.md` と `procedure.md`）。手元での版上げから見守り・最終報告までを担う。
- **Publish Workflow**: crates.io への公開ワークフロー（`.github/workflows/crates-io.yml`）。タグ `v{版}` の push を受けて、長期の鍵を使わずに crates.io へ冪等に公開する。
- **Publishing Guide**: crates.io 設定と運用の文書（`doc/crates-io-publish.md`）。
- **Highlights Guide**: 見どころファイルの書き方の決まり（`doc/release-notes/README.md`、英語）。
- **Release History Note**: `RELEASE.md` 冒頭の凍結の注記と、`README.md`・`README.ja-jp.md` の案内の修正。
- **gtd-mcp package**: `Cargo.toml` の包みの範囲の指定（crates.io に上がる中身を絞る一度きりの変更）。

本文中の「{版}」は今回出す版（`X.Y.Z`）、「{旧版}」は上げる前の版、「版上げ PR」は題が `chore(release): v{版}` の PR を指す。「（仮置き: OQ-n）」と付けた受け入れ基準は、要件ディスカッションで確定させる未決事項に基づく仮の記述である（末尾「未決事項と仮置き」参照）。

## Boundary Context
- **In scope**:
  - `/release` による 1 回分のリリースの実行（前提の確かめ・版上げ・見どころファイル・手元の確かめ・版上げ PR・squash マージ・タグ push・公開の走りの見守り・最終報告）と、途中で止まった回の再開
  - タグ `v{版}` の push、または版を指定した手動起動による crates.io への冪等な公開（Trusted Publishing のみで認証）
  - crates.io 側の Trusted Publisher 登録手順・いつもの流れ・やり直し・予備の手順・してはいけない操作の文書化
  - 見どころファイル（英語・普通の文体）の書き方の決まり
  - `RELEASE.md` の凍結と、以後の記録の置き場への案内（`README.md`・`README.ja-jp.md` の `RELEASE.md` への案内の 1 行ずつの修正を含む）
  - crates.io に上がる包みの中身を、利用に要るファイルだけに絞る一度きりの変更（`Cargo.toml` の包みの範囲の指定。このフィーチャーの実装で 1 回だけ行い、版上げ PR では触らない）
  - 開発者の自動モード許可ルールの文例の提示（設定ファイル自体は書き換えない）
- **Out of scope**:
  - 既存の `release.yml`（バイナリ配布と GitHub Release 作成）・`ci.yml`・`security-audit.yml` の変更
  - GitHub Release の本文を見どころファイルから作る仕組み、Release の資産の検査
  - zip 梱包・起動確認・ライセンス表・全体テストスクリプト等の areka 固有の道具の移植、winget 等の他の配布先
  - 過去の版（0.8.1〜0.8.3）のタグや見どころの後追い作成
  - crates.io の画面操作そのもの、開発者の Claude Code 設定ファイルの書き換え
- **Adjacent expectations**:
  - タグ `v*` の push では既存の `release.yml` も同時に走る。Publish Workflow は `release.yml` を呼ばず、`release.yml` も Publish Workflow を呼ばない。Publish Workflow が `release.yml` の完了を待ち合わせるかどうかは設計で決める（brief の基本方針は「待ち合わせない」）。Release Skill は `release.yml` の走りを見守らず、合否の判定にも含めない（OQ-4 で確定）。
  - crates.io 側で Trusted Publisher（owner `ekicyou`・repo `gtd-mcp-rs`・workflow `crates-io.yml`・environment 空）が登録済みであることを前提とする。登録は開発者が行う。
  - GitHub 操作には開発者の手元で認証済みの `gh` を使う。手元の確かめは steering（`tech.md`）の「CI チェック」と同じ内容を基準とする。

## Requirements

### Requirement 1: リリースの起動と次の版の決定
**Objective:** As a gtd-mcp の開発者, I want `/release` の一言で次の版が決まること, so that 版の付け間違いを避けて 1 回分のリリースを始められる

#### Acceptance Criteria
1. When 開発者が引数なしで `/release` を実行した, the Release Skill shall {旧版} の 3 つ目の数字に 1 を足した版を {版} とする
2. When 開発者が `X.Y.Z` または `vX.Y.Z` の形の引数を付けて `/release` を実行した, the Release Skill shall 先頭の `v` を除いたその値を {版} とする
3. If 引数が数字 3 つを点でつないだ形でない, the Release Skill shall 何も変更せずに止まり、受け付けられる形を示す
4. If 指定された {版} が {旧版} より新しくない, the Release Skill shall 何も変更せずに止まり、2 つの版を示す
5. When 開発者が公開済みの版の見どころの直しを頼んだ, the Release Skill shall リリースの段を回さず、要件 10 の直しの手順だけを行う
6. The Release Skill shall 開発者の明示的な `/release` の実行でだけ動き、会話の流れから自動で起動しない

### Requirement 2: 前提の確かめ
**Objective:** As a gtd-mcp の開発者, I want 取り返しのつく段階で不整合を見つけて止まること, so that main や crates.io に不完全な状態を出さない

#### Acceptance Criteria
1. If 作業中の枝がリポジトリの既定の枝（`main`）である, the Release Skill shall 何も変更せずに止まり、ハーネスのワークツリーの枝で実行し直すよう示す
2. If 作業木に未コミットの変更がある, the Release Skill shall 何も変更せずに止まる
3. When 前提を確かめる, the Release Skill shall 最新の main を作業の枝に取り込み、取り込んだ main の先端を「確かめた main」として覚える
4. If main の取り込みで衝突が起きた、または取り込み後に作業の枝と main の間に差分が残る, the Release Skill shall 止まって事実を報告する
5. If 版の置き場（`Cargo.toml` の `[package] version`、`Cargo.lock`）を変更する開いた PR（作者が `dependabot[bot]` の PR を除く。dependabot は依存の版だけを変え、`gtd-mcp` 自身の版を変えない）が 1 本以上ある, the Release Skill shall その PR の番号と題を示して止まる
6. If リモートにタグ `v{版}` が既に在る, the Release Skill shall 同じ版を出し直さずに止まる
7. If リモートにタグ `v{旧版}` が無く、題が `chore(release): v{旧版}` のマージ済みの PR が在る, the Release Skill shall 前の回が「マージの後・タグの前」で止まっていると示し、版を上げずに止まる
8. While タグ `v{旧版}` が無く、題が `chore(release): v{旧版}` のマージ済みの PR も無く、crates.io に {旧版} が公開済みである（`/release` 導入前に手作業で出した版）, the Release Skill shall タグを後から作らずに止まらず先へ進み、前のタグが無かったことを最終報告に書く
9. If 上の 7・8 のどちらにも当たらない形でタグ `v{旧版}` が無い, the Release Skill shall 止まって事実を報告する

### Requirement 3: 版上げと見どころファイル
**Objective:** As a gtd-mcp の開発者, I want 版の置き場と見どころファイルが 1 回の版上げで揃うこと, so that crates.io の版・タグ・リポジトリの版が一致する

#### Acceptance Criteria
1. When 版を上げる, the Release Skill shall `Cargo.toml` の `[package] version` と `Cargo.lock` の `gtd-mcp` の版だけを {旧版} から {版} に変える
2. When 版を上げる, the Release Skill shall {旧版} のリリース以後に main に入った変更の一覧を読み、Highlights Guide の決まりに従って `doc/release-notes/v{版}.md` を新しく書く
3. While タグ `v{旧版}` が無い初回の回（要件 2.8 に当たる回）である, the Release Skill shall `Cargo.toml` の `[package] version` を {旧版} にしたコミット（初回は `fe9dcab`）を変更の一覧の起点とし、その次のコミットから数え、起点を最終報告に書く
4. The Release Skill shall 他の版の見どころファイルと `RELEASE.md` の本文を変更しない
5. When 見どころファイルを書き終えた, the Release Skill shall Highlights Guide の「必ず載せるもの」に当たる変更がすべて載っているかを読み直す

### Requirement 4: 手元の確かめ
**Objective:** As a gtd-mcp の開発者, I want PR を出す前に CI と同じ確かめと公開前の確かめが通ること, so that 取り消せないタグと公開の前に失敗を見つけられる

#### Acceptance Criteria
1. When 版上げと見どころファイルが揃った, the Release Skill shall steering（`tech.md`）の「CI チェック」と同じ確かめ（整形・lint・ビルド・全テスト・リリースビルド）を 1 つずつ順に回す
2. When CI と同じ確かめが通った, the Release Skill shall {版} が crates.io へ公開できる形に包めることを、公開せずに確かめる
3. When 手元の確かめを回す, the Release Skill shall 変更されたファイルが `Cargo.toml`・`Cargo.lock`・`doc/release-notes/v{版}.md` の 3 つだけであり、`Cargo.toml` と `Cargo.lock` の差分が版の 1 行ずつだけであることを確かめる
4. When 手元の確かめを回す, the Release Skill shall 見どころファイルが Highlights Guide の形の決まりに合うことを確かめる
5. If 手元の確かめのどれかが通らない, the Release Skill shall 版上げ PR を出さずに止まり、通らなかった確かめの名前を報告し、作業木を消さずに残す
6. The Release Skill shall 手元の確かめでソースを自動で書き換えない（整形の自動修正を含む）

### Requirement 5: 版上げ PR と squash マージ
**Objective:** As a gtd-mcp の開発者, I want 確かめが全部通った版上げが承認待ち無しで main に入ること, so that 手作業の待ちを減らせる

#### Acceptance Criteria
1. When 手元の確かめがすべて通った, the Release Skill shall 3 つのファイルだけを名前を挙げてコミットし、題 `chore(release): v{版}` の PR を main 宛てに出す
2. When 版上げ PR を出した, the Release Skill shall PR に含まれるファイルを読み直し、3 つのファイル以外が含まれていれば止まる
3. When 版上げ PR を出す, the Release Skill shall PR の本文に版・変更ファイル・手元の確かめの結果・見どころファイルの所在を書く
4. If PR を出した後に main が「確かめた main」から進んでいた, the Release Skill shall 新しい main を取り込み、新しく入った変更を見どころに反映するかを判断し、手元の確かめを回し直してからマージに進む
5. When 版上げ PR のファイルの確かめ（5.2）と main の取り込みの確かめ（5.4）が済んだ, the Release Skill shall PR の CI の結果を待たずに、開発者に聞かずに版上げ PR を squash マージする
6. When squash マージした, the Release Skill shall PR の状態を読み直して `MERGED` と squash のコミットを確かめ、その読み直しだけでマージの成否を決める
7. The Release Skill shall 版上げ PR に版上げ以外の変更を混ぜない

### Requirement 6: タグの push
**Objective:** As a gtd-mcp の開発者, I want 正しいコミットに正しいタグが 1 本だけ押されること, so that crates.io の版とタグが必ず一致する

#### Acceptance Criteria
1. When 版上げ PR が `MERGED` になった, the Release Skill shall squash のコミットが main に含まれ、そのコミットの `Cargo.toml` の版が {版} と一致し、リモートにタグ `v{版}` が無いことを確かめてから、開発者に聞かずにタグ `v{版}` をそのコミットに打って、そのタグ 1 本だけを push する
2. If 手元に在るタグ `v{版}` が squash のコミット以外を指している, the Release Skill shall push せずに止まり、手元のタグも消さない
3. When タグを push した, the Release Skill shall リモートのタグ `v{版}` が squash のコミットを指すことを読み直して確かめる
4. The Release Skill shall 押したタグを動かさず、消さず、同じ版を二度出さない
5. The Release Skill shall すべてのタグを一度に押し出す操作・強制の push・main への直接の push を行わない
6. If マージの後にタグを打てなかった, the Release Skill shall main の版を上げたまま止まり、再開すれば同じ squash のコミットにタグを打てることを報告する

### Requirement 7: 公開の見守りと赤の扱い
**Objective:** As a gtd-mcp の開発者, I want タグの push の後に公開の結果まで見届けること, so that 公開できたかどうかを自分で調べずに済む

#### Acceptance Criteria
1. When タグを push した, the Release Skill shall タグ `v{版}` と squash のコミットに対応する Publish Workflow の走りを見つけ、終わるまで待つ
2. If 待っても Publish Workflow の走りが見つからない, the Release Skill shall 止まってその事実を報告する
3. When Publish Workflow の走りが終わった, the Release Skill shall 走りが成功で終わり、かつ crates.io に `gtd-mcp` {版} が在ることをもって緑と判定する
4. If 走りが赤で終わった、または crates.io に {版} が見つからない, the Release Skill shall 見えた事実と Publishing Guide の切り分けのどれに当たるかを示して止まり、やり直すかどうかを開発者に決めてもらう
5. The Release Skill shall 自分から走りをやり直さず、手元から crates.io へ公開しない
6. If 赤の原因がタグのコミットの中身に在る, the Release Skill shall 同じ版で出し直さず、次の版で出す必要があることを報告する
7. When 再開して見守りの段に入り、タグの push の走りが赤でも、開発者が同じ版を手動で起動した Publish Workflow の走りが成功で終わっている, the Release Skill shall その手動の走りを 7.3 の走りとして扱う

### Requirement 8: 最終報告
**Objective:** As a gtd-mcp の開発者, I want 1 回分の結果と見どころの全文を 1 つの報告で読めること, so that 公開内容をその場で確かめ、直しを頼める

#### Acceptance Criteria
1. When リリースの回が終わった（緑で終わった、または止まった）, the Release Skill shall 日本語の報告を 1 つ出す
2. The Release Skill shall 報告に、版・タグのコミット・版上げ PR・Publish Workflow の走りの結果・crates.io での {版} の有無を含める
3. The Release Skill shall 報告に、途中で main を取り込んだかどうか、前のタグが無かった初回の扱い（該当時）、途中で気付いた範囲外の問題を含める
4. When 回が緑で終わった, the Release Skill shall 報告に `doc/release-notes/v{版}.md` の全文をそのまま載せ、直しが要れば頼めることを添える
5. If 回が途中で止まった, the Release Skill shall 報告に、止まった段の名前・見えた事実・次に開発者が決めることを含める
6. The Release Skill shall リリースのために記録用のコミットや進み具合のファイルを作らない

### Requirement 9: 再開と止まり方の決まり
**Objective:** As a gtd-mcp の開発者, I want 途中で切れた回を `/release` の打ち直しで安全に続けられること, so that マージやタグの push を二度行う事故を防げる

#### Acceptance Criteria
1. When `/release` が実行された, the Release Skill shall 段 1 の前に、作業の枝の版上げ PR の有無と状態・リモートのタグ・作業木の変更・先頭のコミットを読み、事実から始める段を決める
2. While 作業の枝の版上げ PR が `MERGED` でリモートのタグ `v{版}` が無い, the Release Skill shall 版上げ・確かめ・PR をやり直さず、その PR の squash のコミットにタグを打つ段から続ける
3. While 作業の枝の版上げ PR が `MERGED` でリモートのタグ `v{版}` が在る, the Release Skill shall 新しい版を始めず、見守りと報告の段から続ける
4. If 読んだ事実が決められたどの状態にも当たらない, the Release Skill shall 何も変更せずに止まり、見えた事実を報告する
5. The Release Skill shall 順調な経路では開発者の承認を待たず、確かめが外れたとき・走りが赤のとき・手順に無い状態を見たとき・許可の判定にコマンドを断られたときだけ止まる
6. If 許可の判定にコマンドを断られた, the Release Skill shall コマンドを言い換えたり別の道具で回避したりせず、そのコマンドと理由を示して止まる
7. The Release Skill shall 外部へ出るコマンド（push・PR の作成・PR のマージ）を、1 回の呼び出しにつき他のコマンドとつながない単独の 1 行で実行する
8. The Release Skill shall リモートの接続先 URL や認証情報を印字する操作を行わない
9. The Release Skill shall リリースの途中で Publish Workflow・既存のワークフロー・ソースコードを書き換えず、範囲外の問題は最終報告に書く
10. The Release Skill shall 開発者が自動モードで承認待ち無しに回すための許可ルールの文例を手順書に載せ、開発者の設定ファイルは書き換えない

### Requirement 10: 公開後の見どころの直し
**Objective:** As a gtd-mcp の開発者, I want 最終報告を読んで見どころの直しを頼めること, so that 公開済みの版の記述の誤りを main で直せる

#### Acceptance Criteria
1. When 開発者が公開済みの版 {版} の見どころの直しを頼んだ, the Release Skill shall 最新の main を取り込み、`doc/release-notes/v{版}.md` の 1 ファイルだけを直す PR を題 `docs(release): v{版} の見どころの直し` で出し、そのファイルだけを含むことを確かめて squash マージする
2. The Release Skill shall 見どころの直しでタグ・タグのコミット・crates.io の公開物・GitHub Release を変更しない

### Requirement 11: 公開ワークフローのきっかけと版の確かめ
**Objective:** As a gtd-mcp の開発者, I want タグの push で crates.io への公開が自動で始まること, so that 手元から公開せずに済む

#### Acceptance Criteria
1. When タグ `v*` が push された, the Publish Workflow shall そのタグのコミットを対象に公開の走りを始める
2. When 開発者が版を入力して手動で起動した, the Publish Workflow shall タグ `v{入力の版}` のコミットを対象に公開の走りを始める
3. If タグ名が `v` と数字 3 つの形でない、または手動起動の入力が数字 3 つの形でない, the Publish Workflow shall 何も公開せずに失敗で終わり、受け付けられる形を示す
4. If 対象のコミットの `Cargo.toml` の版がタグ（または入力）の版と一致しない, the Publish Workflow shall 何も公開せずに失敗で終わり、2 つの版を示す
5. If 手動起動で入力された版のタグ `v{入力の版}` がリモートに無い, the Publish Workflow shall 何も公開せずに失敗で終わり、タグが無いことを示す
6. The Publish Workflow shall タグ名や入力の値を、形を確かめるまで処理の文に埋め込まない
7. The Publish Workflow shall 他のワークフローの完了をきっかけにして起動しない

### Requirement 12: 冪等な公開と認証
**Objective:** As a gtd-mcp の開発者, I want 同じ版で何度起動し直しても安全に公開されること, so that 止まった公開をボタン 1 つでやり直せる

#### Acceptance Criteria
1. When 公開の走りが版を確かめた, the Publish Workflow shall crates.io に `gtd-mcp` {版} が既に在るかを読む
2. If crates.io に {版} が既に在る, the Publish Workflow shall 認証の鍵を取らず、何も公開せずに成功で終わる
3. When crates.io に {版} が無い, the Publish Workflow shall 公開できる形に包めることを確かめてから、Trusted Publishing で短命の鍵を受け取り、`Cargo.lock` に固定した依存のまま {版} を公開する
4. If 包む確かめ・鍵の受け取り・公開のどれかが失敗した, the Publish Workflow shall 失敗で終わり、どの段で止まったかを走りの結果に残す
5. When 公開の走りが終わる, the Publish Workflow shall 成否にかかわらず、crates.io での {版} の有無を走りの要約に書き、{版} が無ければ失敗で終わる
6. The Publish Workflow shall リポジトリの秘密の置き場（Secrets）や長期の API トークンを参照しない
7. The Publish Workflow shall 内容の読み取りと Trusted Publishing の鍵の受け取りに要る権限だけを持つ
8. While 公開の走りが動いている, the Publish Workflow shall 同じワークフローの別の走りを並行させず、順に処理する
9. The Publish Workflow shall 既存の `release.yml` を呼び出さず、変更もしない（`release.yml` の完了を待ち合わせるかどうかは設計で決める）

### Requirement 13: crates.io の設定と運用の文書
**Objective:** As a gtd-mcp の開発者, I want crates.io 側の一度きりの設定と止まったときの手順を 1 つの文書で読めること, so that 文書だけを見て設定とやり直しを終えられる

#### Acceptance Criteria
1. The Publishing Guide shall crates.io の画面で `gtd-mcp` に Trusted Publisher を登録する手順と、入れる値（owner `ekicyou`・repo `gtd-mcp-rs`・workflow `crates-io.yml`・environment 空）を示す
2. The Publishing Guide shall 登録を最初のタグの push より前に済ませる必要があること、済ませずにタグを押すと鍵の受け取りで止まり何も公開されないことを示す
3. The Publishing Guide shall タグの push から公開・記録までのいつもの流れと、公開されたことの確かめ方を示す
4. The Publishing Guide shall 止まり方ごとの切り分けと、同じ版での「Re-run」と版を入力した手動起動によるやり直しの手順を示す
5. The Publishing Guide shall Publish Workflow が使えないときの予備の手順として、短期限・対象クレート限定の鍵で手元から公開し、終わったら鍵を手元から消して取り消す手順を示す
6. The Publishing Guide shall してはいけない操作（接続先 URL や認証情報の印字、鍵のリポジトリや Secrets への保存、コマンド行への鍵の直書き）を列挙する
7. The Publishing Guide shall 日本語で書く

### Requirement 14: 見どころファイルの決まり
**Objective:** As a gtd-mcp の利用者, I want 版ごとの注目すべき変更を英語の短い文章で読めること, so that 更新の要点と必要な対応がわかる

#### Acceptance Criteria
1. The Highlights Guide shall 見どころファイルの置き場所と名前（`doc/release-notes/v{版}.md`、版ごとに 1 つ）と文字コードを定める
2. The Highlights Guide shall 見どころファイルを英語・普通の文体（キャラクターの口調を使わない）で書くことを定める
3. The Highlights Guide shall 見どころファイルの形（見出しの階層、1 つの見どころの構成、元になった変更への参照の書き方）を定める
4. The Highlights Guide shall 「必ず載せるもの」（利用者が設定や使い方を変える必要のある変更を含む）と「書かないもの」を定める
5. The Highlights Guide shall 英語で書き、この文書だけを読めば見どころファイルを書けるようにする

### Requirement 15: RELEASE.md の凍結
**Objective:** As a gtd-mcp の利用者, I want 古いリリースノートと新しい記録の置き場の関係がわかること, so that 以後の版の記録を迷わず探せる

#### Acceptance Criteria
1. The Release History Note shall `RELEASE.md` の冒頭に、以後の版の記録は `doc/release-notes/` と GitHub Releases を参照する旨の注記を英語で置く
2. The Release History Note shall `RELEASE.md` の既存の本文（v0.8.0 までの記録）を変更しない
3. The Release History Note shall `README.md`・`README.ja-jp.md` の `RELEASE.md` を全版のリリースノートとする案内を、v0.8.0 までは `RELEASE.md`、以後は `doc/release-notes/` と GitHub Releases を参照する案内に、それぞれの文書の言語で 1 か所ずつ直す

### Requirement 16: crates.io に上がる包みの中身
**Objective:** As a gtd-mcp の利用者, I want crates.io から取得するパッケージに利用に要るファイルだけが入っていること, so that 開発用のスキル・仕様・CI の設定を受け取らずに済む
**Out of boundary for this Release Skill:** この変更はこのフィーチャーの実装で 1 回だけ行い、版上げ PR（要件 3〜5）では版の置き場以外を触らない。

#### Acceptance Criteria
1. The gtd-mcp package shall crates.io へ包む対象を、ビルドと利用に要るファイル（ソース・`Cargo.toml`・`Cargo.lock`・README・ライセンス等。具体的な一覧は設計で決める）に限る
2. The gtd-mcp package shall 包みに `.claude/`・`.kiro/`・`.github/`・`CLAUDE.md` を含めない
3. When 包みの範囲を変えた, the gtd-mcp package shall 包んだ中身からビルドできることを公開せずに確かめられる

## 未決事項と仮置き (Open Questions)
要件ディスカッションで確定させる。各項目の「仮置き」は、上の受け入れ基準に反映済みの暫定案である。

- **OQ-1: 初回の `/release` の前のタグ（`v0.8.3`）が無い問題** — **確定（個別議論・推奨案）**: タグを後付けせず、「`v{旧版}` が無い・`chore(release): v{旧版}` のマージ済み PR も無い・crates.io に {旧版} が在る」ときだけ前のタグの確かめを緩めて進む（要件 2.8）。変更の一覧の起点は `Cargo.toml` の版を {旧版} にしたコミット（初回は `fe9dcab`）とし、その次から数える（要件 3.3）。退けた案: 初回の前に `v0.8.3` を手で打つ（既存の `release.yml` が走り GitHub Release が作られる副作用）、`v0.8.0` を起点にする（公開済みの 0.8.1〜0.8.3 の変更が混ざる）。
- **OQ-2: 版上げ PR の CI を待ってからマージするか** — **確定（個別議論・開発者の判断）**: 待たない。手元の確かめ（要件 4）と PR のファイルの確かめが済んだら、PR の CI の結果を待たずに squash マージする（要件 5.5）。areka の手順と同じ。受け入れたリスク: Linux・macOS だけで起きる失敗は公開前に検出されない（手元の確かめは Windows のみ）。退けた案: PR の全チェックの緑を待つ（約 5〜6 分の待ちと、版上げと無関係な `cargo audit` の赤で止まる不便）。
- **OQ-3: 公開後の見どころの直しを範囲に含めるか** — **確定（推奨案）**: 含める。直すのは main の見どころファイルだけで、GitHub Release の本文は触らない（要件 10）。
- **OQ-4: 既存 `release.yml` の走りを見守り・報告に含めるか** — **確定（推奨案）**: 含めない。合否の判定は Publish Workflow の走りと crates.io の有無だけで行う（Boundary Context）。
- **OQ-5: 次のリリースへの申し送り（areka の roadmap の節）の扱い** — **確定（推奨案）**: 移植しない（gtd-mcp の steering に roadmap が無いため）。
- **OQ-6: README の RELEASE.md への案内** — **確定（推奨案）**: 範囲に入れ、`README.md`・`README.ja-jp.md` の案内を 1 か所ずつ直す（要件 15.3）。
- **OQ-7: crates.io に上がる包みの中身を絞るか** — **確定（推奨案）**: 絞る。このフィーチャーの実装で `Cargo.toml` の包みの範囲を一度だけ指定し、版上げ PR は版の置き場以外を触らない前提を保つ（要件 16）。
