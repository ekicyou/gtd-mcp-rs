# crates.io への公開の手順

リリースのたびに `gtd-mcp` を、リリースと同じ版で crates.io へ出す。ふだんは GitHub Actions のワークフロー `.github/workflows/crates-io.yml`（以下「公開の段」）が自動で出すので、開発者がすることは、最初に一度だけの Trusted Publishing の設定と、止まったときのやり直しだけである。

crates.io に一度出した版は、消すことも差し替えることもできない（できるのは取り下げ＝yank だけ）。

## 1. 何を出すか

- 出すのは `gtd-mcp` の 1 つだけ。
- 包みに入るファイルは `Cargo.toml` の `include` で決まる（`src/` の `.rs`・`Cargo.toml`・`Cargo.lock`・`README.md`・`LICENSE`）。

## 2. Trusted Publishing の設定

Trusted Publishing は、GitHub Actions から、長く使える鍵を置かずに crates.io へ出す仕組みである。公開の段はこれだけで認証する。

**最初のタグの push より前に済ませておく。** 済んでいないとタグを push しても公開の段は段「鍵」で止まり、何も公開されない。その場合は登録してから 5 節の「Re-run」で回復できる。

crates.io に持ち主のアカウントでログインし、`gtd-mcp` で次を行う。

1. クレートの頁の **Settings** を開き、**Trusted Publishing** の欄で追加を選ぶ。
2. 発行元に **GitHub** を選び、次のとおり入れる。

   | 欄 | 入れる値 |
   |---|---|
   | Repository owner | `ekicyou` |
   | Repository name | `gtd-mcp-rs` |
   | Workflow filename | `crates-io.yml` |
   | Environment name (optional) | 空のまま |

3. 保存し、一覧に 1 行増えたことを確かめる。

Environment を空にするのは、公開の段が GitHub の environment（人の承認を挟む仕組み）を使わないためである。ワークフローのファイル名を変えたら、この設定も直す。

## 3. いつもの流れ

1. 版を上げる PR を squash マージし、そのコミットに `v{版}` のタグを打って push する（ふだんは `/release` が行う）。
2. タグの push で、GitHub Release を作る `release.yml` と公開の段 `crates-io.yml` が並走する。互いを呼ばず、待たない。**`release.yml` が赤でも crates.io への公開は進む。**
3. 公開の段は次の段を上から順に行う。どれかが失敗したら、そこから先は「記録」以外を飛ばす。走りの題は `crates-io v{版}` になる。

| 段 | すること |
|---|---|
| 版の形 | タグが `v` と数字 3 つの形か（手で起動したときは、入力 `version` が `v` 無しの数字 3 つの形か）確かめる |
| タグの確かめ | 手で起動したときだけ。タグ `v{版}` が GitHub に在るか確かめる |
| 取り出し | タグ `v{版}` のコミットを取り出す |
| 版の一致 | `Cargo.toml` の版が `{版}` と同じか確かめる |
| 公開済み判定 | crates.io の索引を読み、`{版}` が在れば `present`、無ければ `absent`、読めなければ `unreadable`（失敗） |
| 包む確かめ | `absent` のときだけ。`cargo publish --dry-run --locked` |
| 鍵 | `absent` のときだけ。Trusted Publishing で短命の鍵を受け取る |
| 公開 | `absent` のときだけ。`cargo publish --locked --no-verify` |
| 記録 | 必ず走る。索引を読み直し（公開した直後は 30 秒ごとに最長 5 分）、`present`・`absent`・`unreadable` と各段の結果を走りの要約に書く。`present` でなければ失敗にする |

既に `present` なら、段「包む確かめ」「鍵」「公開」を飛ばして緑で終わる。

タグの push の走りは、タグのコミットに在る `crates-io.yml` で動く。このファイルが入る前のコミットに打ったタグでは動かない。

## 4. 出たことの確かめ方

crates.io の索引に、その版の行が在るかを見る（公開の段の「公開済み判定」「記録」と同じ判定）。

```bash
curl -fsS https://index.crates.io/gt/d-/gtd-mcp | grep -cF '"vers":"0.8.4"'
```

- `1` なら `present`（出ている）。`0` なら `absent`（出ていない）。`curl` がエラーを出したら、`0` が出ていても `unreadable`（読めない）。
- 公開の段の走りの要約（段「記録」が書く）でも `present`・`absent` を見られる。
- 公開した直後は索引への反映が遅れることがある。`absent` なら時間を置いてもう一度読む。

## 5. 止まったときのやり直し

公開の段は、同じ版で何度起動し直してもよい。既に出ていれば段「公開済み判定」が `present` を読み、何も上げずに緑で終わる。起動し直しは GitHub の Actions の画面から行う。

- **Re-run**: 止まった `crates-io` の走りを開き、「Re-run jobs」→「Re-run all jobs」を押す。元の走りと同じコミット（タグの push の走りならタグのコミット、Run workflow の走りなら起動した時点の `main`）の `crates-io.yml` で、最初の段から走る。
- **Run workflow**: 左の一覧で `crates-io` を選び、「Run workflow」を押す。「Use workflow from」は `main` のまま、`version` に `v` を付けない版（例 `0.8.4`）を入れて起動する。**`main` に在る** `crates-io.yml` で動き、段「タグの確かめ」でタグ `v{版}` が在ることを確かめてから、そのタグのコミットを取り出す。

どこで止まったかは、走りの段ごとの結果と要約で見る。前の段が赤なら「記録」も赤になるので、「記録」以外で最初に赤になった段で次の表を引く。

| 止まった段 | 見えたこと・原因 | やり直し |
|---|---|---|
| 「版の形」 | タグ名か入力 `version` の形が外れている。何も公開されていない | 入力の誤りなら、正しい版で Run workflow。タグ名の誤りなら、そのタグは出す版ではない（タグは動かさず消さない） |
| 「タグの確かめ」 | 入力した版のタグ `v{版}` が GitHub に無い | 版を確かめて Run workflow |
| 「取り出し」 | タグのコミットを取り出せない | 一時的なら Re-run |
| 「版の一致」 | タグのコミットの `Cargo.toml` の版がタグと違う。タグのコミットの誤り | 同じ版では出さない。`main` で直して次の版で出す |
| 「公開済み判定」 | `unreadable`。crates.io の索引を読めない | 時間を置いて Re-run |
| 「包む確かめ」 | 包めない・組み立てられない | 通信などの一時的な失敗なら Re-run。タグのコミットのコードや設定の誤りなら、`main` で直して次の版で出す |
| 「鍵」 | 鍵を受け取れない。Trusted Publishing が未登録か、登録値が違う | 2 節のとおり登録・修正してから Re-run |
| 「公開」 | `cargo publish` が失敗した | 一時的なら Re-run |
| 「記録」 | 「公開」は緑で「記録」だけ赤。索引への反映待ち | 時間を置いて Re-run（何も上げずに緑で終わる） |

- **ワークフローのファイルの誤り**（段の書き方・権限・入力の扱いなど）: Re-run はタグのコミットのファイルで動くので、`main` で直しても効かない。`main` で直してから Run workflow で同じ版を渡す。
- `release.yml` の赤は公開の段に関係しない。`release.yml` が赤でも公開の段は待たずに進み、上の表だけで切り分ける。
- **`/release` の再開**: Run workflow で出し直して緑になったら、`/release {版}` を再開する。段 6.2 が題 `crates-io v{版}` の手動の走りの緑を認め、報告まで進む。

## 6. 予備の手順

公開の段が使えないとき（Trusted Publishing の不具合など）に、手元の cargo から出す。先に 4 節で確かめ、既に `present` なら出す物は無い。

1. タグのコミットに移り、作業木がきれいなことを確かめる。

   ```bash
   git switch --detach v0.8.4
   git status --porcelain   # 何も出ないこと
   ```

2. steering（`.kiro/steering/tech.md`）の CI チェック 5 つが緑であることを確かめる。

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo build --verbose
   cargo test --verbose
   cargo build --release --verbose
   ```

3. crates.io の **Account Settings** → **API Tokens** で新しい鍵を作る。期限はその日まで、対象のクレートは `gtd-mcp` だけ、できることは `publish-update` だけにする。
4. 鍵を手元の cargo に渡す。鍵は `cargo login` に聞かれてから貼る（コマンドの後ろに書かない）。

   ```bash
   cargo login
   ```

5. 出す。

   ```bash
   cargo publish --locked
   ```

6. 鍵を手元から消す。

   ```bash
   cargo logout
   ```

7. crates.io の **API Tokens** で、作った鍵を取り消す（Revoke）。
8. 4 節の確かめ方で `present` になったことを確かめる。

## 7. してはいけない操作

接続先の URL や認証の情報は、画面にもログにも印字しない。接続先の URL には認証の情報が含まれていることがある。次の操作は、この手順のどこでも行わない。

- `git remote -v`
- `git config --get remote.origin.url`
- `.git/config` の中身の表示
- cargo の認証のファイル（`credentials.toml`）の中身の表示
- `cargo login` の後ろに鍵を書くこと、鍵を環境変数に入れて表示すること
- crates.io の鍵を、リポジトリのファイルや GitHub の秘密の置き場（Secrets）に置くこと
