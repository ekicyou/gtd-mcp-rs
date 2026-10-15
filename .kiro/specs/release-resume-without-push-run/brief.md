# Brief: release-resume-without-push-run

> 起票元: crates-io-release の完了時の棚卸し（2026-10-16）。`/kiro-discovery` は開発者だけが起動できるため、棚卸しの結果をこの brief に直接書いた。着手前に `/kiro-discovery` か `/kiro-start` で中身を確かめること。

## Problem
`/release` の再開は、タグの push で起きた `crates-io` の走りが 1 本も無いと、段 6.1 で「走りが見つからない」と止まる。開発者が Run workflow で同じ版を出し直して緑にしても、`/release {版}` を打ち直すと報告まで進めない。

## Current State
- 要件 7.7（completed/crates-io-release）は「タグの push の走りが赤でも、手で起動した走りが緑なら認める」場合だけを決めている。
- procedure.md 段 6.1 は push の走りを `headBranch == "v{版}"`・`headSha == "{sha}"` で探し、見つからなければ止まる（要件 7.2）。段 6.2 の手動の走りの確認は push の走りが見つかった後にしか届かない。
- Publishing Guide（`doc/crates-io-publish.md`）5 節は、push の走りが無いときは段 6.1 で止まると書き、crates.io での確かめで終える道を示している。
- 起きるのは、GitHub がタグの push で走りを起こさなかったときに限られる。concurrency で取り消された走りは `cancelled` として残るので当たらない。

## Desired Outcome
再開の回で push の走りが無くても、題が `crates-io v{版}` の手動の走りが緑で crates.io に `present` なら、`/release` が緑として報告まで進む。

## Approach
要件 7.7 を「push の走りが赤、または無い」に広げ、procedure.md 段 6.1 の「見つからない」の扱いを、再開の回に限り段 6.2 の手動の走りの確認へ回す。Publishing Guide 5 節の再開の説明を合わせて直す。

## Scope
- **In**: 要件 7.7 の改訂、procedure.md 段 6.1・6.2、SKILL.md の要約（必要なら）、Publishing Guide 5 節の 1 行。
- **Out**: `crates-io.yml` の変更、初回の `/release`（再開でない回）の扱いの変更。

## Boundary Candidates
- `/release` 手順の段 6（見守り）

## Out of Boundary
- Publish Workflow（`crates-io.yml`）の段・きっかけ
- `release.yml`

## Upstream / Downstream
- **Upstream**: completed/crates-io-release（要件 7.2・7.7、design の Release Skill 段 6、State Management）
- **Downstream**: なし

## Existing Spec Touchpoints
- **Extends**: completed/crates-io-release の要件 7.7
- **Adjacent**: なし

## Constraints
- 走りを自分からやり直さない（`gh run rerun` の禁止）・外へ出るコマンドは単独の 1 行、の決まりは変えない。
