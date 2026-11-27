# Static check: manifest, rules, review record

> **Document**: 03-01-static-check.md
> **Parent**: [Index](00-index.md)

## Modules (AR-30)

| Module | Owns |
|--------|------|
| `tools/release/docs_rules.py` | manifest loading and validation, `normalize()`, version tokens, allowlist matching, date parsing, review-record parsing |
| `tools/release/docs_check.py` | CLI (`--version`, `--live`, `--today`, `--repo`, `--manifest`), static mode, reporting, exit codes |
| `tools/release/docs_live.py` | live mode (03-02) |
| `tools/release/docs-manifest.json` | the data |

Std-only Python 3.10+; no network in static mode; files read from the
working tree but enumerated with `git ls-files` (AR-19).

## CLI and exit codes (AR-27)

`python3 tools/release/docs_check.py --version X.Y.Z [--live] [--today YYYY-MM-DD]`.
Exit 0 pass, 1 findings, 2 usage or manifest error, 3 unreachable (live), 4
deploy not current (live). All findings are collected, deduplicated and
printed sorted as `path:line [rule] message` (static) or `URL [rule] message`
(live), then a one-line count. An unallowlisted token also prints a
ready-to-paste entry:
`{"file": "<path>", "phrase": "<±40 chars of normalized context>", "reason": "", "reviewed_for": "X.Y.Z"}`.

## Manifest schema (AR-28)

| Key | Shape | Meaning |
|-----|-------|---------|
| `site_url` | string | `https://openlorraine.github.io/clorraine/` |
| `scan` | `{files: [path], globs: [glob], exclude: [glob]}` | release-facing set (AR-1, AR-19) |
| `excluded` | `[{path, reason}]` | tracked root/`docs/` Markdown deliberately not scanned |
| `platforms` | `[{name, command, guide, sidebar_text, anchor, heading}]` | `command` contains `{version}` (AR-34) |
| `install_surfaces` | `[path]` | files that must carry every platform command |
| `required` | `[{file, pattern, why}]` | literal text with `{version}` / `{minor}` placeholders (AR-35) |
| `banned` | `[{pattern, message}]` | case-insensitive literal phrases (`next release`, `vX.Y.Z`, `X.Y.Z`, `not in a marketplace`, `PENDING-`) |
| `placeholder` | `{file, line_prefix}` | `RELEASE-NOTES.md`, `PENDING-CHECKSUMS:` (AR-13) |
| `dated` | `[{file, pattern}]` | regex with named groups `day`, `month`, `year` (AR-25) |
| `changelogs` | `[{file, heading}]` | regex for a released heading with group `version` (+ `date`) (AR-18) |
| `theme_files` | `[{file, pages}]` | `"all"` or `"home"` (AR-38) |
| `allowlist` | `[{file, phrase, reason, reviewed_for}]` | AR-3, AR-14 |

Loader errors (exit 2): unknown key, a listed file that is not tracked, an
empty `reason`, an allowlist phrase without a version token, a malformed
regex, a `required`/`command` pattern without a placeholder where one is
needed.

## Version tokens (AR-2, AR-12, AR-26)

- **Version:** `VERSION` must match `^\d+\.\d+\.\d+$` after stripping one
  leading `v`; `minor` is `X.Y`.
- **Tokens:**
  - `v`-tokens: `(?<![\w.])v\d+\.\d+(?:\.\d+)?(?:-[0-9A-Za-z.]+)?(?![\w])`, not followed by `.\d`.
  - Release references: `releases/(?:tag|download)/v\d+\.\d+\.\d+` and `clorraine-\d+\.\d+\.\d+`.
- **Valid tokens:** a three-part token is valid when it equals `vX.Y.Z`. A two-part token is valid when it equals `vX.Y`. Release references must carry `X.Y.Z`.
- **Allowlist entries:** an entry allows the tokens inside each occurrence of its phrase in the normalized text of its file.
- **Ignored:** third-party numbers without a `v` (FS-UAE `3.2.35`).

## Normalization (AR-20)

Drop `**`, `__` and backticks. Rewrite `[text](url)` to `text url`. Decode HTML entities. Drop U+200B. Collapse every whitespace run, newlines included, to one space. A map from normalized offsets back to the raw line numbers lets each finding report `path:line`.

## Static rules (each finding gets a rule id)

| Rule | Check | AR |
|------|-------|----|
| `version-input` | VERSION equals `compiler/Cargo.toml` `[package]` version, `tools/vscode/package.json` version, both changelog top released headings, and the `RELEASE-NOTES.md` title contains `vX.Y.Z` | AR-26 |
| `unclassified` | every tracked root `*.md` and `docs/*.md` is in `scan` or `excluded` | AR-19 |
| `stale-version` | a token that is neither valid nor allowlisted | AR-2, AR-12 |
| `allowlist-unused` | an entry whose phrase no longer occurs | AR-20 |
| `allowlist-review` | an entry whose `reviewed_for` ≠ VERSION | AR-14 |
| `required` | a required pattern is missing | AR-35 |
| `platform` | a platform command missing from an install surface or its guide; the sidebar item (`text` + `link` to `/guide/quick-start#<anchor>`) missing from `config.ts`; the quick-start `## Install on <Platform>` heading missing | AR-34 |
| `banned` | banned wording anywhere in the scan set, the one placeholder line excepted | AR-13 |
| `placeholder` | the committed `RELEASE-NOTES.md` (read with `git show HEAD:`) has ≠ 1 `PENDING-CHECKSUMS:` line | AR-13 |
| `dated` | a dated line is missing, unparsable, in the future, or older than 14 days | AR-4, AR-25 |
| `changelog` | the top released heading is not `[X.Y.Z] - YYYY-MM-DD` (extension changelog: `## X.Y.Z`) within the date window; sections below it are exempt from `stale-version` only | AR-18 |
| `review` | the record fails the rules below for the `before` section | AR-5, AR-9, AR-32 |

## Review record (AR-5, AR-9, AR-10, AR-32)

- **Template:** `docs/release-reviews/TEMPLATE.md`, with `## Before tag` and `## After deploy` sections of `- [ ] item` lines (AR-33).
- **Record:** `docs/release-reviews/vX.Y.Z.md` must contain:
  - the heading `# Release review: vX.Y.Z`;
  - the lines `Prepared by: …`, `Reviewer: @handle` and `Date: YYYY-MM-DD`;
  - the same two sections, whose checkbox lines match the template's item texts exactly, in order.
- **Notes:** non-checkbox lines and indented sub-bullets are allowed.
- **Static mode:** requires every `Before tag` item to be ticked (`- [x]`), plus `Prepared by`, `Reviewer` and `Date`.
- **Live mode:** also requires every `After deploy` item to be ticked.
- **Failures:**
  - a missing or extra item;
  - an edited item text;
  - an e-mail address anywhere in the record;
  - a `Reviewer` value that is not an `@handle`.
- **Scope:** only the record for VERSION is validated.
