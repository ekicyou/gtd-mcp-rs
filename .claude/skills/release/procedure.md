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
