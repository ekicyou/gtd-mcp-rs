# Release Highlights

This directory holds one highlights file per released version of `gtd-mcp`.
A highlights file tells users what changed in that version and what, if
anything, they need to do. This guide is the single source of rules for
writing one; you should not need any other document.

## Location and name

- Path: `doc/release-notes/v{version}.md`, for example
  `doc/release-notes/v0.8.4.md`. `{version}` is the `[package] version` in
  `Cargo.toml`, without a leading `v` inside the braces.
- Exactly one file per version.
- Encoding: UTF-8, LF line endings, ASCII characters only (no smart quotes,
  no em dashes, no non-ASCII symbols).
- The file is added in the version-bump PR (`chore(release): v{version}`).
  Never edit the highlights file of another version in that PR.

## Language and tone

- Write in English.
- Use plain, neutral prose. No character voice, no jokes, no marketing tone.
- Write facts in short sentences, addressed to the user of `gtd-mcp`.

## Format

- Do not use `#` or `##` headings.
- Each highlight is one `### ` heading followed by its body text, which runs
  until the next `### ` heading or the end of the file.
- There is at least one highlight.
- Every `### ` heading ends with a reference to the change it came from, in
  parentheses:
  - a pull request: `#N`, for example `(#258)`;
  - a direct commit without a pull request: its 7-character short SHA in
    lowercase hex, for example `(fe9dcab)`;
  - several references: separate them with a comma and one space, for
    example `(#257, #258)` or `(#259, dbe9713)`.
- The heading text before the reference is a short summary of the change,
  not a copy of a commit subject.

### Sample

```markdown
### Git sync works with HTTPS remotes authenticated by gh (#258)

Fetch and push in `--sync-git` mode now pass credentials to the HTTPS
remote. Previously, a remote that relied on the GitHub CLI credential helper
failed with an authentication error. No configuration change is needed; if
you worked around the problem with an SSH remote, you can keep using it.

### Internal maintenance (#257, #259, dbe9713)

Development tooling and release automation were updated. There is no change
to the server's behavior or to its data files.
```

## Must include

- Every change that requires users to change their configuration, usage,
  MCP client settings, or data files. Write it in this order, as plain
  statements of fact:
  1. what changed and how;
  2. what the user needs to do;
  3. what happens if they do not.
- For a version that only contains fixes: at least one highlight describing
  a fix that affects users.
- For a version with no user-visible change: one highlight that summarizes
  the internal maintenance as a whole.

## Do not write

- Names of kiro specs.
- Item-by-item lists of internal-only work (CI, skills, steering, specs).
  Summarize such work in one highlight, or leave it out when there are
  user-visible changes to report.
- Links to detailed release notes.
- Copies of commit subjects. Drop the Conventional Commits prefix
  (`feat:`, `fix(...)`, `chore:` and so on) and rephrase for users.

## Number of highlights

Aim for 3 to 5 highlights. The minimum is 1. Merge small related changes into
one highlight rather than listing each of them.

## Source of material

List the changes merged into `main` since the previous release:

```bash
git log --first-parent --format="%h %s" v{previous}..main
```

A squash-merge subject ends with `(#N)`; that `N` is the pull request number.
A line without `(#N)` is a direct commit; use its 7-character short SHA from
the first column as the reference.

If the tag `v{previous}` does not exist (the first release made with this
process), use the commit that set the version to `{previous}` as the starting
point instead of `v{previous}`. It is the first line of:

```bash
git log --first-parent -S'version = "{previous}"' --format=%h main -- Cargo.toml
```

The range then starts from the commit after that one:
`git log --first-parent --format="%h %s" {start}..main`.

## Format check

Run these four checks from the repository root after writing the file. Each
command exits with status 0 when the check passes and 1 when it fails; a
failing check prints the offending lines with their line numbers (check 1
prints nothing). All four must pass.

```bash
F=doc/release-notes/v{version}.md

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

These checks do not cover the language and tone, "Must include", or
"Do not write". Re-read the file against those sections yourself.
