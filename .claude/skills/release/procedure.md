# リリースの手順（`/release`）

この文書が `/release` の正本である。段・コマンド・判定・表・してはいけない事・止まり方はここに従い、`SKILL.md` はその要約に過ぎない。

## 読み方

- 段は 0 から 7 の順に回す。どの段から始めるかは、毎回、段 0 の再開の表で決める。
- 各段は「行うこと（コマンド）」と「判定」から成る。判定はコマンドの出力を主文脈が読み、書かれた条件に当てはめて決める。**判定に当たらなければ止まる**（「止まり方の決まり」）。判定は記憶や推測ではなく、その場で取り直した出力だけで行う。
- コマンドは bash の形で書く（Windows では Claude Code の Bash ツール＝Git Bash で回す）。`{…}` は「置き換える語」の値に置き換えてから回す。
- 主文脈で実行する。サブエージェント・進み具合のファイル・記録用のコミットを作らない。スクリプトも足さない。
- 順調な経路では開発者の承認を待たない。
- 頼みが公開済みの版の見どころの直しのときは、段 0〜7 を回さず「見どころの直し」だけを行う。

## 置き換える語

| 語 | 中身 | 決まる所 |
|---|---|---|
| `{remote}` | `git remote` が出す名前（1 行）。`-v` は付けない | 段 0 |
| `{枝}` | `git branch --show-current` が出す作業の枝 | 段 0 |
| `{引数}` | `/release` に付けた引数（無ければ空） | 起動 |
| `{旧版}` | 上げる前の版（`git show HEAD:Cargo.toml` の `[package] version`） | 段 1.3 |
| `{版}` | 今回出す版（`X.Y.Z`、`v` を付けない） | 段 1.4 |
| `{確かめた main}` | 前提の確かめで取り込んだ `{remote}/main` の先端の SHA | 段 1.1 |
| `{起点}` | 変更の一覧の起点（タグ `v{旧版}` か、初回は版を `{旧版}` にしたコミット） | 段 1.3 |
| `{番号}` | 版上げ PR の番号 | 段 0・段 4.1 |
| `{本文}` | 版上げ PR の本文を書く、作業木の外の一時ファイルの絶対パス | 段 4.1 |
| `{sha}` | 版上げ PR の squash のコミット | 段 0・段 4.4 |
| `{id}` | Publish Workflow の走りの番号 | 段 6.1 |

## してはいけない事

- タグを動かさない・消さない（手元のタグもリモートのタグも）。
- 同じ版を二度出さない。リモートにタグ `v{版}` が在る版や、crates.io に在る版で版上げを始めない。
- `--tags`・`--follow-tags`・`--force`・`--admin` を使わない。main へ直接 push しない。
- 手元から `cargo publish` しない（段 3.4 の `cargo publish --dry-run` は除く）。公開は Publish Workflow だけが行う。
- `gh run rerun` や Run workflow など、走りを自分からやり直さない。やり直すかは開発者が決める。
- 版上げ PR に版上げ以外を混ぜない（`Cargo.toml`・`Cargo.lock`・`doc/release-notes/v{版}.md` の 3 ファイルだけ）。
- リモートの接続先 URL や認証情報を印字しない。`git remote -v` を使わない。`git fetch`・`git push`・`git ls-remote` には `--quiet` を付けて標準エラーを捨て（`2>/dev/null`）、成否は終了コードと読み直しで決める。
- リリースの途中で Publish Workflow・既存のワークフロー・ソースコードを書き換えない（整形の自動修正もしない）。他の版の見どころファイルと `RELEASE.md` も変えない。範囲外の問題は直さず最終報告に書く。
- 進み具合のファイル・記録用のコミットを作らない。

## 止まり方の決まり

- 止まるのは次の 4 つのときだけ: 確かめ（判定）が外れたとき・走りが赤のとき・手順に無い状態を見たとき・許可の判定にコマンドを断られたとき。
- 許可の判定にコマンドを断られたら、言い換えず、別の道具で回避せず、そのコマンドと断られた理由を示して止まる。
- 外へ出るコマンド（`git push`・`gh pr create`・`gh pr merge`）は、1 回の呼び出しにつき、他のコマンドと `&&`・`;`・`|` でつながない単独の 1 行で回す。
- 段 0・段 1 で止まるときは何も変更せずに止まる（段 1.1 の取り込みが衝突したときは `git merge --abort` で取り込みの前に戻してから止まる）。段 2・3 で止まるときは PR を出さず、作業木を消さずに残す。main にも GitHub にも何も出ていない。
- 止まったら、止まった段の名前・見えた事実（判定に使ったコマンドの出力）・次に開発者が決めることを日本語で報告する。

## 段 0 再開の読み方

進み具合はどこにも書かない。`/release` のたびに、段 1 の前に事実を読み直し、下の表で始める段を決める。

### 0.1 事実を読む

```bash
git remote
git branch --show-current
gh pr list --head {枝} --state all --json number,state,mergeCommit,title --jq '[.[] | select(.title | test("^chore[(]release[)]: v[0-9]+[.][0-9]+[.][0-9]+$"))] | sort_by(.title | ltrimstr("chore(release): v") | split(".") | map(tonumber)) | last // empty | "\(.number)\t\(.state)\t\(.mergeCommit.oid // "-")\t\(.title)"'
git status --porcelain
git log -1 --format=%s
```

- `git remote` が 1 行でなければ止まる（`{remote}` が決まらない）。1 行ならそれを `{remote}` とする。
- `gh pr list` の出力が「この枝の版上げ PR」（題が `chore(release): v{X.Y.Z}` の PR のうち版が最も新しい 1 本）。空なら「版上げ PR が無い」。1 行なら `番号・状態（OPEN／MERGED／CLOSED）・squash のコミット・題` で、題の版をこの表の中の `{版}` とする。
- 版上げ PR が `MERGED` のときだけ、リモートのタグを読む。

```bash
git ls-remote --quiet --tags {remote} refs/tags/v{版} 2>/dev/null
```

  - 終了コードが 0 でなければ止まる。0 行なら「タグが無い」、1 行ならその SHA を控える。

- 「3 ファイルの内側」とは、`git status --porcelain` のどの行のパスも `Cargo.toml`・`Cargo.lock`・`doc/release-notes/v{X.Y.Z}.md` のどれかであること。`{X.Y.Z}` は作業木の `Cargo.toml` の `[package] version` で、次で読む。

```bash
sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' Cargo.toml
git rev-parse -q --verify MERGE_HEAD
```

  - 取り込みの途中（`git rev-parse -q --verify MERGE_HEAD` が SHA を出す、または `git status --porcelain` に状態欄に `U` を含む行か `AA`・`DD` の行が在る）のときは「3 ファイルの内側」に当たらない。

### 0.2 再開の表

上から順に当てはめ、最初に当たった行に従う。

| 見えた事実 | 始める段 | 要件 |
|---|---|---|
| 版上げ PR が無く、作業木の変更が 3 ファイルの内側だけ（変更が 1 行以上在る） | PR の前に切れた回。`{版}` は作業木の `Cargo.toml`、`{旧版}` は `git show HEAD:Cargo.toml`、`{確かめた main}` は `git merge-base {remote}/main HEAD`（先に `git fetch --quiet {remote} main 2>/dev/null`）。1.2・1.3・1.4 のタグの確かめを回してから 2.2 の読み直しへ（見どころが在れば書き直さない）、続けて段 3 | 9.1 |
| 版上げ PR が無く、作業木がきれいで、先頭のコミットの題が `chore(release): v` で始まる | コミットの後・PR の前。`{版}` は `git show HEAD:Cargo.toml`。4.1 の push から。`{確かめた main}` は覚えていない物として 4.3 の 2 以降を 1 回行う | 9.1 |
| 版上げ PR が無く、作業木がきれいで、上に当たらない | 段 1 から（新しい回） | 9.1 |
| 版上げ PR が開いている | 作業木がきれいでなければ止まる。`{確かめた main}` は覚えていない物として 4.3 の 2 以降を 1 回行ってから 4.4 | 9.1 |
| 版上げ PR が `MERGED` で、リモートのタグ `v{版}` が無い | 段 5 から。`{sha}` は PR の `mergeCommit`。版上げ・確かめ・PR をやり直さない | 9.2 |
| 版上げ PR が `MERGED` で、リモートのタグが在る（引数が無いか同じ版） | 段 6 から（タグが `{sha}` を指すことを確かめる）。新しい版を始めない | 9.3 |
| 版上げ PR が `MERGED` で、リモートのタグが在り、引数がそれより新しい版 | 前の回の 6.2 を当て、緑なら段 1 から新しい回、緑でなければ前の回の段 6 から | 9.3 |
| 上のどれにも当たらない | 何も変更せずに止まり、見えた事実を報告する | 9.4 |

- 「引数」の版は先頭の `v` を除いて比べる。「それより新しい」は段 1.4 と同じ比べ方（数 3 つを数として比べる）。
- 頼みが公開済みの版の見どころの直しのときは、この表を当てはめず「見どころの直し」だけを行う（1.5）。

## 段 1 前提の確かめ

段 1 では何も変更しない（1.1 の main の取り込みを除く）。どの判定が外れても、その場で止まる。

### 1.1 枝・作業木・main

```bash
git branch --show-current
git status --porcelain
git fetch --quiet {remote} main 2>/dev/null
git merge-base --is-ancestor {remote}/main HEAD
git merge --no-edit {remote}/main
git diff --name-only {remote}/main...HEAD
git rev-parse {remote}/main
```

1. `git branch --show-current` が `main` でない。`main` なら止まり、「ハーネスのワークツリーの枝で `/release` を打ち直す」と示す（2.1）。
2. `git status --porcelain` が 0 行。在れば止まり、その行を示す（2.2）。
3. `git fetch` の終了コードが 0。0 でなければ止まる。
4. `git merge-base --is-ancestor {remote}/main HEAD` の終了コードが 0 なら取り込み済みで、`git merge` は回さない。1 なら `git merge --no-edit {remote}/main` を回す（2.3）。
5. `git merge` が衝突したら、衝突を自分で解かない。`git diff --name-only --diff-filter=U` で衝突したファイルを控え、`git merge --abort` で取り込みの前に戻してから止まり、控えたファイルを示す（2.4）。
6. `git diff --name-only {remote}/main...HEAD` が 0 行（作業の枝に main 以外の変更が無い）。在れば止まり、そのファイルを示す（2.4）。
7. `git rev-parse {remote}/main` の値を `{確かめた main}` として覚える（2.3）。

### 1.2 開いた PR

```bash
gh pr list --state open --limit 200 --json number,title,files,author --jq '.[] | select(.author.login != "app/dependabot") | select((.files | length) >= 100 or any(.files[]; .path == "Cargo.toml" or .path == "Cargo.lock")) | "\(.number)\t\(.files | length)\t\(.title)"'
```

- 作者が dependabot（`author.login` が `app/dependabot`）の PR は数えない（確定: 設計判断 9）。
- 出た行のうち、ファイル数が 100 の行は `files` が切り詰められているので、`gh pr diff {番号} --name-only` で読み直し、根の `Cargo.toml` か `Cargo.lock` を含むかを決める。
- 判定: 版の置き場（根の `Cargo.toml`・`Cargo.lock`）を触る開いた PR が 0 本。在れば止まり、その PR の番号と題を示す（2.5）。

### 1.3 旧版と前のタグ

```bash
git show HEAD:Cargo.toml | sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}'
git ls-remote --quiet --tags {remote} refs/tags/v{旧版} 2>/dev/null
```

- 1 行目の出力（1 行）を `{旧版}` とする。1 行でなければ止まる。
- `git ls-remote` の終了コードが 0 でなければ止まる。
- 1 行（タグ `v{旧版}` が在る）なら `{起点}` を `v{旧版}` として段 1.4 へ。
- 0 行（タグが無い）なら次の 2 つを読み、下の分岐表に当てはめる。

```bash
gh pr list --state merged --search "chore(release): v{旧版} in:title" --json number,title,mergeCommit --jq '.[] | select(.title == "chore(release): v{旧版}") | "\(.number)\t\(.mergeCommit.oid)\t\(.title)"'
curl -fsS https://index.crates.io/gt/d-/gtd-mcp | grep -cF '"vers":"{旧版}"'
```

- `gh pr list` は題が字のとおり `chore(release): v{旧版}` の PR だけを残す。1 行以上なら「在る」、0 行なら「無い」。
- 2 つ目は公開済み判定（公開済み判定の契約と同じ情報源・同じ `vers` の字のとおりの一致）。`curl` がエラーを出したら、数に関わらず `unreadable`。エラーが無く 1 以上なら `present`、0 なら `absent`。

| マージ済みの `chore(release): v{旧版}` | 公開済み判定（`{旧版}`） | 扱い | 要件 |
|---|---|---|---|
| 在る | 問わない | 前の回が「マージの後・タグの前」で止まっている、と PR の番号と squash のコミットを示して止まる。版を上げない | 2.7 |
| 無い | `present` | 初回（`/release` 導入前に手で出した版）。`{起点}` を下のコマンドの 1 行目（最も新しいコミット。初回は `fe9dcab`）とし、先へ進む。タグは作らない。起点と「前のタグが無かった」ことを報告に書く | 2.8, 3.3 |
| 無い | `absent` または `unreadable` | 事実を示して止まる | 2.9 |

```bash
git log --first-parent -S'version = "{旧版}"' --format=%H {確かめた main} -- Cargo.toml
```

- 初回の分岐で、このコマンドの出力が 0 行なら止まる（起点が決まらない）。

### 1.4 次の版

```bash
printf '%s' '{引数}' | grep -Ex 'v?[0-9]+\.[0-9]+\.[0-9]+'
printf '%s\n' '{旧版}' '{版}' | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1
git ls-remote --quiet --tags {remote} refs/tags/v{版} 2>/dev/null
```

1. `{引数}` が空なら、`{旧版}` の 3 つ目の数に 1 を足した版を `{版}` とする（例 `0.8.3` → `0.8.4`）（1.1）。
2. `{引数}` が在れば、1 行目の `grep` が 1 行を出す（終了コード 0）こと。外れたら止まり、受け付ける形 `X.Y.Z` か `vX.Y.Z`（X・Y・Z は数字）を示す（1.3）。当たれば先頭の `v` を除いた値を `{版}` とする（1.2）。
3. `{版}` が `{旧版}` より新しい: 2 行目の出力が `{版}` で、かつ `{版}` と `{旧版}` が字のとおり同じでない（数 3 つを数として比べる。`0.8.10` は `0.8.9` より新しい）。外れたら止まり、2 つの版を示す（1.4）。
4. `git ls-remote` の終了コードが 0 で、出力が 0 行（リモートにタグ `v{版}` が無い）。終了コードが 0 でなければ止まる。1 行なら同じ版を出し直さずに止まり、そのタグの SHA を示す（2.6）。

- 1.4 で止まるときは何も変更していない。

## 段 2 版上げと見どころ

段 2 では `Cargo.toml`・`Cargo.lock`・`doc/release-notes/v{版}.md` の 3 ファイルだけを変える。他の版の見どころファイルと `RELEASE.md` は変えない（3.4）。段 2 で止まるときは PR を出さず、作業木を消さずに残す。

### 2.1 版上げ

1. 根の `Cargo.toml` の `[package]` の `version = "{旧版}"` の 1 行を、Edit ツールで `version = "{版}"` に直す。他の行は触らない。
2. `Cargo.lock` を揃える。

```bash
cargo update --workspace
sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' Cargo.toml
```

- `cargo update --workspace` の終了コードが 0。0 でなければ止まる。
- 2 行目の出力が `{版}` の 1 行（3.1）。外れたら止まる。差分の範囲（`Cargo.lock` が版の 1 行だけ変わったか）の判定は段 3.1 が持つ。

### 2.2 見どころ

1. `{起点}` がタグ `v{旧版}` のときだけ、そのタグを手元に取る（初回の分岐で `{起点}` がコミットのときは回さない）。

```bash
git fetch --quiet {remote} refs/tags/v{旧版}:refs/tags/v{旧版} 2>/dev/null
```

  - 終了コードが 0。0 でなければ止まる。

2. 変更の一覧を読む（3.2, 3.3）。

```bash
git log --first-parent --format="%h %s" {起点}..{確かめた main}
```

  - 一覧は `{起点}` の次のコミットから `{確かめた main}` まで。初回の分岐では `{起点}`（版を `{旧版}` にしたコミット）自体は入らない（3.3）。
  - 題の末尾が `(#N)` の行は squash でマージした PR で、`#N` が参照。`(#N)` の無い行は直接のコミットで、1 列目の 7 桁の短い SHA が参照（見どころの形の契約）。
  - 出力が 0 行なら止まる（見どころに書く変更が無い）。

3. Highlights Guide（`doc/release-notes/README.md`）を全部読み、その決まりに従って `doc/release-notes/v{版}.md` を Write ツールで新しく書く（3.2）。英語・普通の文体・ASCII だけ・LF。見出しは `### ` だけで、各見出しの末尾に参照を置く。
  - 再開（段 0 の表の 1 行目）で `doc/release-notes/v{版}.md` が既に在るときは、1・2（タグの取り込みと一覧）を回し、3 は書き直さず 4 の読み直しへ進む。
4. 書いたファイルを Highlights Guide の「Must include」と「Do not write」に照らして、2 の一覧と突き合わせて読み直す（3.5）。設定・使い方・MCP クライアントの設定・データファイルの変更を利用者に求める変更が一覧に在れば、それが「何が変わったか・利用者がすること・しないとどうなるか」の順ですべて載っていること。
  - 漏れや「Do not write」に当たる物が在れば、このファイルだけを直して読み直す。ファイルが在り、読み直しで漏れが無ければ段 3 へ。

## 段 3 手元の確かめ

3.1 から 3.4 を順に、1 つずつ回す。どれか 1 つでも判定が外れたら、**版上げ PR を出さずに止まり**、通らなかった確かめの名前（「3.1 差分の範囲」など）と、判定に使った出力を示す。作業木は消さずに残す（4.5）。

- 手元の確かめでソースを書き換えない。整形の自動修正（`cargo fmt --all`）も、`cargo clippy --fix` も回さない（4.6）。見どころファイルの形が外れたときも直さずに止まる。

### 3.1 差分の範囲

```bash
git diff --numstat HEAD
git diff -U0 HEAD -- Cargo.toml
git diff -U0 HEAD -- Cargo.lock
git status --porcelain
```

1. `git diff --numstat HEAD` がちょうど 2 行で、`1	1	Cargo.lock` と `1	1	Cargo.toml`（列はタブ区切り。足 1 行・消 1 行）（4.3）。
2. `git diff -U0 HEAD -- Cargo.toml` と `git diff -U0 HEAD -- Cargo.lock` のどちらも、`---`・`+++` の行を除いた `-`・`+` の行が `-version = "{旧版}"` と `+version = "{版}"` の 2 行だけ（4.3, 3.1）。
3. `git status --porcelain` がちょうど 3 行で、` M Cargo.toml`・` M Cargo.lock`・`?? doc/release-notes/v{版}.md`。新しいファイルは見どころの 1 つだけで、他の版の見どころファイルと `RELEASE.md` は出ない（4.3, 3.4）。

### 3.2 見どころの形

Highlights Guide の「Format check」の 4 つの確かめを、そのまま 1 つずつ回す（4.4）。Guide の `{version}` をこの文書の `{版}` に置き換えただけで、コマンドは同じ。

```bash
F=doc/release-notes/v{版}.md

# 1. At least one "### " heading.
grep -q '^### ' "$F"

# 2. No "#" or "##" headings.
! grep -nE '^#{1,2} ' "$F"

# 3. Every "### " heading ends with a reference: (#N), (abc1234), or a
#    ", "-separated list of them.
! grep -n '^### ' "$F" | grep -vE '^[0-9]+:### .+ \((#[0-9]+|[0-9a-f]{7})(, (#[0-9]+|[0-9a-f]{7}))*\)$'

# 4. ASCII characters only.
! LC_ALL=C grep -nP '[^\x00-\x7F]' "$F"
```

- 4 つとも終了コード 0。1 つでも 1 なら止まり、外れた確かめの番号と、出た行（行番号つき。1 は何も出さない）を示す。
- 英語・普通の文体・「Must include」・「Do not write」は機械では見ない。段 2.2 の 4 の読み直しが持つ。

### 3.3 CI と同じ確かめ

steering `tech.md` の CI チェック 5 種を、この順に 1 つずつ回す（4.1）。`&&` でつながず、1 つの終了コードを見てから次を回す。

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --verbose
cargo test --verbose
cargo build --release --verbose
```

- 5 つとも終了コード 0。0 でなければ、そこで止まり、そのコマンドと失敗の出力の要点を示す。後のコマンドは回さない。
- `cargo fmt --all -- --check` が外れても `cargo fmt --all` で直さない（4.6）。

### 3.4 公開前の確かめ

```bash
cargo publish --dry-run --locked --allow-dirty
```

- 終了コード 0（4.2）。0 でなければ止まり、失敗の出力の要点を示す。
- `--dry-run` を外さない。公開は Publish Workflow だけが行う。`--allow-dirty` は、3 ファイルがこの時点でまだコミットされていないために付ける（確定: 設計判断 10）。
- 3.1〜3.4 がすべて通ったら段 4 へ。

## 段 4 版上げ PR とマージ

段 4 から先は GitHub に出る。外へ出るコマンド（`git push`・`gh pr create`・`gh pr merge`）は、1 回の呼び出しにつき単独の 1 行で回す。マージの前（4.1〜4.3）で止まるときは、PR を閉じず、枝と作業木を消さずに残す。打ち直せば段 0 の再開の表の「版上げ PR が開いている」行（または「先頭のコミットの題が `chore(release): v` で始まる」行）から続く。PR の CI の結果は待たない（5.5）。

### 4.1 コミットと PR

1. 3 ファイルだけを名前を挙げて足し、題 `chore(release): v{版}` でコミットする（5.1, 5.7）。`git add -A`・`git add .`・`git commit -a` は使わない。

```bash
git add Cargo.toml Cargo.lock doc/release-notes/v{版}.md
git commit -m "chore(release): v{版}"
git status --porcelain
git show --numstat --format= HEAD
```

  - `git commit` の終了コードが 0。0 でなければ止まる。
  - `git status --porcelain` が 0 行。
  - `git show --numstat --format= HEAD` がちょうど 3 行で、`1	1	Cargo.lock`・`1	1	Cargo.toml`・`{N}	0	doc/release-notes/v{版}.md`（列はタブ区切り。`{N}` は見どころファイルの行数で、1 以上）。外れたら止まる（5.7）。

2. 作業の枝を push する（再開の表の 2 行目はここから）。

```bash
git push --quiet {remote} HEAD 2>/dev/null
```

```bash
git ls-remote --quiet {remote} refs/heads/{枝} 2>/dev/null
git rev-parse HEAD
```

  - `git push` の終了コードが 0 で、`git ls-remote` の 1 行の SHA が `git rev-parse HEAD` と同じ。外れたら止まる。`{枝}` が `main` でないことは段 1.1 で確かめてある（main へ直接 push しない）。

3. PR の本文を、Write ツールで作業木の外の一時ファイル `{本文}`（Claude Code の scratchpad の絶対パス。無ければ `/tmp/release-v{版}-pr-body.md`）に書く。パスはスラッシュ区切りで書き、コマンドでは引用符で囲む（Git Bash ではクォートしないバックスラッシュが消える）。作業木の中には書かない（作業木がきれいでなくなる）。本文には版・変更ファイル（1 の `git show --numstat --format= HEAD` の出力 3 行）・手元の確かめ 3.1〜3.4 の結果・見どころファイルの所在を書く（5.3）。形は次のとおり。

```text
Release v{版}.

Files changed (git show --numstat --format= HEAD):

    1	1	Cargo.lock
    1	1	Cargo.toml
    {N}	0	doc/release-notes/v{版}.md

Local checks (all passed):

- 3.1 diff scope
- 3.2 highlights format
- 3.3 CI checks: cargo fmt --all -- --check / cargo clippy --all-targets --all-features -- -D warnings / cargo build --verbose / cargo test --verbose / cargo build --release --verbose
- 3.4 cargo publish --dry-run --locked --allow-dirty

Highlights: doc/release-notes/v{版}.md
```

  - 3.1〜3.4 は通ったときだけコミットまで来るので「all passed」と書く。再開の表の 2 行目から来たときも同じ（コミットは段 3 が通った後にしか作られない）。

4. main 宛てに PR を出す。

```bash
gh pr create --base main --head {枝} --title "chore(release): v{版}" --body-file "{本文}"
```

5. 出た PR を読み直し、`{番号}` と URL を覚える。`gh pr create` の終了コードや出力ではなく、この読み直しで決める。

```bash
gh pr list --head {枝} --state open --json number,url,title --jq '.[] | select(.title == "chore(release): v{版}") | "\(.number)\t\(.url)"'
```

  - ちょうど 1 行。1 列目を `{番号}` とし、2 列目の URL を最終報告のために覚える。0 行なら止まる（PR が開いていない）。

### 4.2 PR のファイル

```bash
gh pr view {番号} --json headRefOid,files --jq '.headRefOid, (.files[] | .path)'
git rev-parse HEAD
```

- 1 行目（PR の先頭のコミット）が `git rev-parse HEAD` と同じ。違えば止まる（読んだファイルが今の作業の枝のものでない）。
- 2 行目以降がちょうど 3 行で、`Cargo.lock`・`Cargo.toml`・`doc/release-notes/v{版}.md` の 3 つ（順は問わない）（5.2）。それ以外のパスが在るか、3 行でなければ、**マージせずに止まり**、出たパスを示す。PR は閉じない。

### 4.3 main の移動

PR を出した後に main が `{確かめた main}` から進んでいたら、新しい main を取り込み、確かめ直してからマージへ進む（5.4）。

1. main の先端を読む。

```bash
git fetch --quiet {remote} main 2>/dev/null
git rev-parse {remote}/main
```

  - `git fetch` の終了コードが 0。0 でなければ止まる。
  - `git rev-parse {remote}/main` が `{確かめた main}` と同じなら、取り込む物は無い。4.4 へ。違えば 2 から 8 を順に回す。

2. 新しく入った変更を控える。`{確かめた main}` を覚えていないとき（再開の表の 2 行目・4 行目から来たとき）は、取り込みの前の `git merge-base {remote}/main HEAD` の値を `{確かめた main}` とする。このときは先に 1 の `git fetch` を回す。

```bash
git merge-base {remote}/main HEAD
git log --first-parent --format="%h %s" {確かめた main}..{remote}/main
```

  - 出た行（題と参照）を最終報告のために控える（8.3）。0 行でも 3 以降を回す。

3. 新しい main を取り込む。

```bash
git merge --no-edit {remote}/main
```

  - 衝突したら、衝突を自分で解かない。`git diff --name-only --diff-filter=U` で衝突したファイルを控え、`git merge --abort` で取り込みの前に戻してから止まり、控えたファイルを示す。PR は閉じない。

4. 2 で控えた変更を Highlights Guide（`doc/release-notes/README.md`）に照らし、見どころに反映するかを決める。「Must include」に当たる物（設定・使い方・MCP クライアントの設定・データファイルの変更を利用者に求める変更など）が在れば、`doc/release-notes/v{版}.md` だけを Edit ツールで足し、段 3.2 の 4 つの確かめを回し、見どころだけのコミットを足す。当たる物が無ければ見どころは変えず、コミットも足さない。

```bash
git add doc/release-notes/v{版}.md
git commit -m "chore(release): v{版} highlights update"
```

  - 足したかどうかを最終報告のために控える（8.3）。

5. 段 3.3（CI と同じ確かめの 5 つ）と段 3.4（公開前の確かめ）を、書かれたとおり 1 つずつ回し直す。外れたら、マージせずに止まり、通らなかった確かめの名前と出力を示す。

6. 段 3.1（差分の範囲）を `{remote}/main` 相手に読み替えて回し直す。

```bash
git diff --numstat {remote}/main HEAD
git diff -U0 {remote}/main HEAD -- Cargo.toml
git diff -U0 {remote}/main HEAD -- Cargo.lock
git status --porcelain
```

  - `git diff --numstat {remote}/main HEAD` がちょうど 3 行で、`1	1	Cargo.lock`・`1	1	Cargo.toml`・`{N}	0	doc/release-notes/v{版}.md`。
  - `Cargo.toml` と `Cargo.lock` の差分のどちらも、`---`・`+++` の行を除いた `-`・`+` の行が `-version = "{旧版}"` と `+version = "{版}"` の 2 行だけ。`{旧版}` を覚えていないときは、`-version = "…"` の行の版を `{旧版}` とし、それが `{版}` より古いことを段 1.4 の 3 と同じ比べ方で確かめる。
  - `git status --porcelain` が 0 行。
  - 外れたら、マージせずに止まり、出力を示す。

7. 作業の枝を push し、4.1 の 2 の読み直しと 4.2 を回し直す。

```bash
git push --quiet {remote} HEAD 2>/dev/null
```

8. 1 で読んだ `{remote}/main` の値を `{確かめた main}` として覚え直し、1 へ戻る。1 で先端が `{確かめた main}` と同じになるまで繰り返す。

### 4.4 squash マージ

1. マージの直前に、もう一度 4.3 の 1 を回す。先端が `{確かめた main}` と違えば 4.3 の 2 へ戻る。
2. 版上げ PR を squash でマージする。PR の CI の結果は待たず、開発者にも聞かない（5.5）。`--admin`・`--auto`・`--delete-branch` は付けない（作業の枝は消さない）。

```bash
gh pr merge {番号} --squash
```

3. PR の状態を読み直す。`gh pr merge` の終了コードや出力ではなく、この読み直しだけでマージの成否を決める（5.6）。

```bash
gh pr view {番号} --json state,mergeCommit --jq '"\(.state)\t\(.mergeCommit.oid // "-")"'
```

  - 1 列目が `MERGED` で、2 列目が 40 桁の SHA。当たれば 2 列目を `{sha}` として覚え、段 5 へ。
  - `MERGED` でなければ止まり、出た行を示す。マージをやり直さない（`--admin` を足して打ち直す・`--auto` で待つ、などをしない）。PR は開いたまま残り、打ち直せば再開の表の「版上げ PR が開いている」行から続く。

## 段 5 タグの push

段 5 では、squash のコミット `{sha}` に注釈なしのタグ `v{版}` を 1 本だけ打ち、そのタグ 1 本だけを push する。タグを動かさない・消さない（手元のタグもリモートのタグも）。`--tags`・`--follow-tags`・`--force` を使わない（6.4, 6.5）。

段 5 で止まるときは、main の版は `{版}` に上がったままになる。止まったら、見えた事実に加えて「打ち直せば再開の表の『版上げ PR が `MERGED` で、リモートのタグ `v{版}` が無い』行から、同じ `{sha}` にタグを打てる」ことを報告する（6.6）。手元に打ったタグは残す。

### 5.1 タグの確かめ

```bash
git fetch --quiet {remote} main 2>/dev/null
git merge-base --is-ancestor {sha} {remote}/main
git show {sha}:Cargo.toml | sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}'
git ls-remote --quiet --tags {remote} refs/tags/v{版} 2>/dev/null
git rev-parse -q --verify refs/tags/v{版}
```

4 つとも当たったら 5.2 へ。どれか 1 つでも外れたら、**push せずに止まる**（6.1, 6.2）。

1. `git fetch` の終了コードが 0 で、`git merge-base --is-ancestor {sha} {remote}/main` の終了コードが 0（squash のコミットが main に在る）。1 や 128 なら止まる。
2. `git show {sha}:Cargo.toml` から読んだ版が `{版}` の 1 行（そのコミットの版が今回の版と一致する）。
3. `git ls-remote` の終了コードが 0 で、出力が 0 行（リモートにタグ `v{版}` が無い）。1 行なら同じ版を二度出さずに止まり、そのタグの SHA を示す。
4. 手元のタグ `v{版}` が無い（`git rev-parse -q --verify refs/tags/v{版}` が何も出さず終了コード 1）か、出た SHA が `{sha}` と同じ。別の SHA（別のコミットや注釈つきのタグ）なら止まり、その SHA を示す。手元のタグを消さない・動かさない（6.2）。

### 5.2 タグの push

1. 手元にタグ `v{版}` が無いときだけ、`{sha}` に注釈なしのタグを打つ（`-a`・`-s`・`-m`・`-f` を付けない）。在るとき（5.1 の 4 で `{sha}` を指していたとき）は打たない。

```bash
git tag v{版} {sha}
git rev-parse -q --verify refs/tags/v{版}
```

  - `git tag` の終了コードが 0 で、`git rev-parse` の出力が `{sha}`。外れたら止まり、出た値を示す。打ったタグは消さない。

2. そのタグ 1 本だけを push する（6.1, 6.5）。

```bash
git push --quiet {remote} refs/tags/v{版} 2>/dev/null
```

3. リモートのタグを読み直す。`git push` の終了コードに関わらず、この読み直しで成否を決める（6.3）。

```bash
git ls-remote --quiet --tags {remote} refs/tags/v{版} 2>/dev/null
```

  - 終了コードが 0 で、ちょうど 1 行の `{sha}	refs/tags/v{版}`（SHA が `{sha}`）。当たれば段 6 へ。
  - 0 行（タグを打てなかった）なら、push をやり直さずに止まる。手元のタグは残し、段 5 の冒頭の報告をする（6.6）。
  - 1 行でも SHA が `{sha}` と違えば止まり、その SHA を示す。リモートのタグに触れない（6.4）。

## 段 6 公開の見守り

段 6 では Publish Workflow（`.github/workflows/crates-io.yml`。ワークフロー名 `crates-io`、走りの題 `crates-io v{版}`）の走りを読むだけで、書き換えない。`gh run rerun` も Run workflow もしない。手元から `cargo publish` しない（7.5）。

段 0 の再開の表から段 6 に入ったときは、`{sha}` を版上げ PR の `mergeCommit` とし、0.1 で読んだリモートのタグ `v{版}` の SHA が `{sha}` と同じことを確かめる。違えば止まり、2 つの SHA を示す（タグに触れない）。

### 6.1 走りの特定と待ち

1. タグの push の走りのうち、`headBranch` が `v{版}` で `headSha` が `{sha}` の走りを探す（7.1）。見つからなければ 30 秒置いて読み直し、最長 10 回（5 分）。待ちは前景の `sleep` を使わず、下の 1 本を Bash ツールの裏で回し（`run_in_background`）、終わりの知らせを待つ。

```bash
for i in 1 2 3 4 5 6 7 8 9 10; do out=$(gh run list --workflow crates-io.yml --event push --limit 20 --json databaseId,headSha,headBranch,status,conclusion --jq '.[] | select(.headBranch == "v{版}" and .headSha == "{sha}") | "\(.databaseId)\t\(.status)\t\(.conclusion)"') || { echo "gh-error"; break; }; if [ -n "$out" ]; then printf '%s\n' "$out"; break; fi; sleep 30; done
```

  - 出力が `番号	状態	結論`（タブ区切り）のちょうど 1 行なら、1 列目を `{id}` とする。
  - 何も出ない（10 回読んでも見つからない）なら止まり、「タグ `v{版}`（`{sha}`）の `crates-io` の走りが見つからない」と報告する（7.2）。`gh-error` か 2 行以上なら、その出力を示して止まる。

2. 2 列目が `completed` でなければ、走りが終わるまで待つ。これも Bash ツールの裏で回し、終わりの知らせを待つ。終了コードでは判定せず、成否は 6.2 の読み直しで決める。

```bash
gh run watch {id} --exit-status --interval 30
```

### 6.2 緑の判定

1. 走りの結論を読む。

```bash
gh run view {id} --json status,conclusion,url --jq '"\(.status)\t\(.conclusion)\t\(.url)"'
```

  - 1 列目が `completed` で 2 列目が `success` なら 2 へ。3 列目の URL を最終報告のために覚える。
  - `completed` でなければ 6.1 の 2 へ戻る。
  - `success` でないとき: 段 0 の再開から段 6 に入った回（再開の表の「リモートのタグが在る」2 行）に限り、同じ版を開発者が手で起動した走りを読む（7.7）。そうでない回は 6.3 へ。

```bash
gh run list --workflow crates-io.yml --event workflow_dispatch --limit 20 --json databaseId,displayTitle,status,conclusion,createdAt --jq '[.[] | select(.displayTitle == "crates-io v{版}" and .status == "completed")] | sort_by(.createdAt) | last // empty | "\(.databaseId)\t\(.conclusion)"'
```

  - 題が `crates-io v{版}` で `completed` の最も新しい手動の走りが出る。2 列目が `success` なら、1 列目で `{id}` を置き換え、1 から同じ判定を行う。0 行か `success` でなければ、タグの push の走りのまま 6.3 へ。

2. crates.io に `{版}` が在るかを読む（公開済み判定の契約。段 1.3 と同じ情報源・同じ一致）。

```bash
curl -fsS https://index.crates.io/gt/d-/gtd-mcp | grep -cF '"vers":"{版}"'
```

  - `curl` がエラーを出したら、数に関わらず `unreadable`。エラーが無く 1 以上なら `present`、0 なら `absent`。
  - `present` なら緑。段 7 へ（7.3）。
  - `absent` か `unreadable` なら、すぐに赤としない。30 秒置いて読み直し、最長 10 回（5 分）。待ちは 6.1 と同じく下の 1 本を裏で回す。

```bash
for i in 1 2 3 4 5 6 7 8 9 10; do sleep 30; if body=$(curl -fsS https://index.crates.io/gt/d-/gtd-mcp); then n=$(printf '%s\n' "$body" | grep -cF '"vers":"{版}"'); echo "$n"; [ "$n" -ge 1 ] && break; else echo unreadable; fi; done
```

  - 最後の行が 1 以上なら `present` で緑。それ以外（`0` か `unreadable`）なら赤として 6.3 へ。

### 6.3 赤の扱い

走りが `success` で終わらなかったとき、または 6.2 の上限まで読んでも crates.io に `{版}` が無いときは、**自分からやり直さずに止まる**（7.4, 7.5）。`gh run rerun`・Run workflow・手元の `cargo publish` をしない。やり直すかは開発者が決める。

1. 走りの段ごとの結果を読む。

```bash
gh run view {id} --json jobs --jq '.jobs[].steps[] | "\(.name)\t\(.conclusion)"'
```

2. 「記録」以外で最初に `failure`（または `cancelled`）になった段を、Publishing Guide（`doc/crates-io-publish.md`）5 節「止まったときのやり直し」の切り分け表の「止まった段」に当てはめる。走りが `success` なのに crates.io に無いとき、または「記録」だけが赤のときは、表の「記録」の行（索引への反映待ち）に当てる。どの段も赤でない（走りが取り消された、など）ときは、その事実をそのまま示す。
3. 止まり、次を示す（段 7 の「止まった」報告）。
  - 走りの番号・URL・結論、1 の出力のうち赤の段、当てはめた切り分け表の行（「見えたこと・原因」と「やり直し」）。
  - やり直しは表のとおり Re-run（タグのコミットの `crates-io.yml` で動く）か Run workflow（`main` の `crates-io.yml` で動き、入力 `version` に `{版}`）で、開発者が GitHub の Actions の画面から行うこと。緑になった後に `/release {版}` を打ち直せば、段 0 から段 6 に入り、6.2 がその走りを認めて報告まで進むこと。
  - 赤の原因がタグのコミットの中身に在る（表の「版の一致」の行、「包む確かめ」でコードや設定の誤りのとき）なら、**同じ版では出し直さず、`main` で直して次の版で出す必要がある**と書く（7.6）。タグは動かさず消さない。

## 段 7 報告

回が終わったら（緑で終わった、または止まった）、日本語の報告を 1 つ出す（8.1）。記録用のコミットや進み具合のファイルは作らない（8.6）。

1. 必ず載せる（8.2）。その時点でまだ無い物（段 4 より前で止まって PR が無い、など）は「まだ無い」と書く。
  - 版 `{版}`
  - タグのコミット `{sha}`
  - 版上げ PR の番号と URL（URL を覚えていないとき・再開で来たときは `gh pr view {番号} --json url --jq .url` で読む）
  - Publish Workflow の走りの番号 `{id}`・URL・結論（手動の走りを認めたときはそのことも）
  - crates.io での `{版}` の `present`／`absent`（6.2 の 2 の最後の読み取り。`unreadable` ならそう書く）
2. 該当するときに載せる（8.3）。
  - 途中で main を取り込んだか: 段 4.3 で控えた題と参照、見どころに足したか。
  - 初回の扱い: 前のタグ `v{旧版}` が無かったこと、選んだ `{起点}`（段 1.3）。
  - 途中で気付いた範囲外の問題（直さずに書く。9.9）。
3. 緑で終わったとき（8.4）: タグのコミットの見どころの全文を、下の出力のまま載せる。その後に「直しが要れば言ってください。main のファイルを直します」と添える。

```bash
git show {sha}:doc/release-notes/v{版}.md
```

4. 止まったとき（8.5）: 止まった段の名前（「6.3 赤の扱い」など）・見えた事実（判定に使ったコマンドの出力）・次に開発者が決めること。

## 見どころの直し

公開済みの版 `{版}` の見どころの直しを頼まれたときだけ行う。段 0〜7 は回さない（1.5）。直すのは main の `doc/release-notes/v{版}.md` の 1 ファイルだけで、タグ・タグのコミット・crates.io の公開物・GitHub Release には触れない（10.2）。外へ出るコマンドは単独の 1 行で回す。判定に当たらなければ止まる。

1. 枝・作業木・main。

```bash
git branch --show-current
git status --porcelain
git fetch --quiet {remote} main 2>/dev/null
git merge --no-edit {remote}/main
git ls-files --error-unmatch doc/release-notes/v{版}.md
```

  - `git branch --show-current` が `main` でない（main へ直接 push しない）。`git status --porcelain` が 0 行。`git fetch` の終了コードが 0。
  - `git merge` が衝突したら、段 1.1 の 5 と同じく `git merge --abort` で取り込みの前に戻して止まる。
  - `git ls-files --error-unmatch` の終了コードが 0（直すファイルが main に在る）。

2. `doc/release-notes/v{版}.md` を頼まれたとおり Edit ツールで直し、段 3.2 の 4 つの確かめを回す。どれか外れたら止まる。

```bash
git status --porcelain
```

  - ちょうど 1 行の ` M doc/release-notes/v{版}.md`。

3. コミットし、push する。

```bash
git add doc/release-notes/v{版}.md
git commit -m "docs(release): v{版} の見どころの直し"
```

```bash
git push --quiet {remote} HEAD 2>/dev/null
```

```bash
git ls-remote --quiet {remote} refs/heads/{枝} 2>/dev/null
git rev-parse HEAD
```

  - `git push` の終了コードが 0 で、`git ls-remote` の 1 行の SHA が `git rev-parse HEAD` と同じ。外れたら止まる。

4. PR を出し、読み直す。

```bash
gh pr create --base main --head {枝} --title "docs(release): v{版} の見どころの直し" --body "Fix the highlights of v{版}. Only doc/release-notes/v{版}.md is changed."
```

```bash
gh pr list --head {枝} --state open --json number,url,title --jq '.[] | select(.title == "docs(release): v{版} の見どころの直し") | "\(.number)\t\(.url)"'
gh pr view {番号} --json headRefOid,files --jq '.headRefOid, (.files[] | .path)'
git rev-parse HEAD
```

  - `gh pr list` がちょうど 1 行。1 列目を `{番号}` とする。
  - `gh pr view` の 1 行目が `git rev-parse HEAD` と同じで、2 行目以降がちょうど 1 行の `doc/release-notes/v{版}.md`（10.1）。外れたらマージせずに止まる。PR は閉じない。

5. squash でマージし、読み直す。

```bash
gh pr merge {番号} --squash
```

```bash
gh pr view {番号} --json state,mergeCommit --jq '"\(.state)\t\(.mergeCommit.oid // "-")"'
```

  - 1 列目が `MERGED`。外れたらマージをやり直さずに止まる。
  - 報告に PR の番号・URL・squash のコミットと、直した後のファイルの全文を載せる。

## 見直すきっかけ

次の変更があったら、この手順（段 1.3・2.2・3.2・6・見どころの直し）と Publishing Guide を合わせて見直す。

- ワークフローのファイル名 `crates-io.yml`・ワークフロー名 `crates-io`・`run-name`（走りの題 `crates-io v{版}`）の変更（段 6.1・6.2 の走りの見つけ方。ファイル名は crates.io の Trusted Publisher の設定も直す）。
- きっかけ（タグ `v*` の push・`workflow_dispatch` の入力 `version`）や段の名前の変更（段 6.3 と Publishing Guide 5 節の切り分け表）。
- 版の置き場の変更（`Cargo.toml` の `[package] version` 1 行・`Cargo.lock` の 1 行。ワークスペース化・クレートの追加を含む）。
- 見どころファイルの置き場・名前・形の決まりの変更（段 2.2・3.2）。
- `release.yml` を待ち合わせる方針への変更、GitHub Release 本文の自動化の導入。
- 公開済み判定の情報源（`https://index.crates.io/gt/d-/gtd-mcp`）の変更。

## 開発者の設定

自動モードで承認待ち無しに回すには、開発者が自分の Claude Code の設定の `autoMode.allow` に、次の文を 1 つの文字列として足す（9.10）。`/release` はどの設定ファイルも書き換えない。

```text
While running the /release skill in the gtd-mcp-rs repository, allow only these operations: pushing the current working branch (never main) with `git push <remote> HEAD`; creating a pull request into main from that branch titled `chore(release): vX.Y.Z` and squash-merging it with `gh pr merge <number> --squash`; creating a lightweight tag `vX.Y.Z` on the squash commit of that merged `chore(release): vX.Y.Z` pull request and pushing only that one tag with `git push <remote> refs/tags/vX.Y.Z`; creating a pull request into main from that branch titled `docs(release): vX.Y.Z の見どころの直し` that changes only `doc/release-notes/vX.Y.Z.md` and squash-merging it with `gh pr merge <number> --squash`. Never allow: `--admin`, `--force`, `--tags`, `--follow-tags`, pushing directly to main, moving or deleting any tag, merging any other pull request, `cargo publish` (only `cargo publish --dry-run` is allowed), or re-running workflows (`gh run rerun`).
```
