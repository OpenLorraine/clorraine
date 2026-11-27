# Testing strategy

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

Specification tests live in `tools/release/tests/docs_check_spec.py` and implementation tests in
`docs_check_impl.py`. Both are hermetic:
- **Fixture repository:** each case builds a small git repository in a temporary directory, with a
  manifest, the release-facing files and a review record. A clean fixture passes; each case then
  changes one thing.
- **Injected inputs:** the clock, the HTTP fetcher and the `gh` runner are passed in.
- **Assertions:** each case checks the exit code and the reported rule id, and the message where the
  spec fixes the wording.

Commands:
- **Task verify:** `make release-docs-test`.
- **Phase gate:** `make check` (+ `make site-check` when `site/` changes) (AR-36).

## Specification test cases

| ST | Input (one change to the clean fixture) | Expected | Trace |
|----|-----------------------------------------|----------|-------|
| ST-1 | Clean fixture, VERSION `1.3.0`, today = record date | exit 0, "0 findings" | FR-1 |
| ST-2 | VERSION `1.3`, `v1.3.0`, `1.3.0-rc.1`, empty | `1.3`, `1.3.0-rc.1` and empty: exit 2; `v1.3.0` accepted as `1.3.0` | AR-26 |
| ST-3 | `compiler/Cargo.toml` says 1.2.0 | exit 1, `version-input` | AR-26 |
| ST-4 | A page says "v1.2.0 is the current release" | exit 1, `stale-version` with `path:line`, and a paste-ready allowlist entry with `"reason": ""` | AR-2, AR-27 |
| ST-5 | Same, with an allowlist entry for that phrase (reviewed_for 1.3.0) | exit 0 | AR-3 |
| ST-6 | Allowlist entry whose phrase is reworded in the file | exit 1, `allowlist-unused` and `stale-version` | AR-3, AR-20 |
| ST-7 | Allowlist entry with `reviewed_for: 1.2.0` | exit 1, `allowlist-review` | AR-14 |
| ST-8 | Allowlist entry with empty reason | exit 2 | AR-28 |
| ST-9 | "limitations of Clorraine v1.2" | exit 1, `stale-version` (two-part) | AR-12 |
| ST-10 | `v1.3.10` and `v1.3.0` in one file, VERSION 1.3.0 | only `v1.3.10` is reported | AR-26 |
| ST-11 | FS-UAE `3.2.35` without a `v` | exit 0 | AR-2 |
| ST-12 | `**v1.2.0**` wrapped across two lines inside an allowlisted phrase | exit 0 (normalization) | AR-20 |
| ST-13 | Home page lacks the Windows install command | exit 1, `platform` naming Windows and the file | AR-11, AR-34 |
| ST-14 | Sidebar lacks `Install on Windows` | exit 1, `platform` naming `config.ts` | AR-34 |
| ST-15 | An install command pinned to `v1.2.0` | exit 1, `stale-version` | FR-2 |
| ST-16 | "ships with the next release" in a page | exit 1, `banned` | AR-13 |
| ST-17 | `RELEASE-NOTES.md` with zero, or two, `PENDING-CHECKSUMS:` lines | exit 1, `placeholder` | AR-13 |
| ST-18 | Brief dated 15 days before today; dated tomorrow | exit 1, `dated` both | AR-4, AR-25 |
| ST-19 | Old changelog section mentions v1.2.0; the top section mentions v1.2.0 | the old section passes; the top section reports `stale-version` | AR-18 |
| ST-20 | Changelog top heading `[1.3.0]` without a date | exit 1, `changelog` | AR-18 |
| ST-21 | A tracked `docs/macos-cli.md` not in scan/excluded | exit 1, `unclassified` | AR-19 |
| ST-22 | Review record: missing, one Before item unticked, an item edited, an extra item, an e-mail address, `Reviewer: gevik` (no `@`) | exit 1, `review` each | AR-32 |
| ST-23 | Review record with Before ticked and After unticked | static exit 0; live reports `review` | AR-9 |
| ST-24 | Live: sitemap newest lastmod older than the last site commit | exit 4, `deploy-not-current`, no page fetched | AR-24 |
| ST-25 | Live: a rendered page contains `v1.2.0` that is not allowlisted for its source file; and one allowlisted only for a theme file that renders on every page | first reported, second allowed | AR-20, AR-38 |
| ST-26 | Live: an install URL only in a code block points at a missing asset | exit 1, the download URL reported | AR-21, AR-22 |
| ST-27 | Live: Discord invite API returns 404 | exit 1 | AR-16 |
| ST-28 | Live: Marketplace package for X.Y.Z returns 404 | exit 1 | AR-16 |
| ST-29 | Live: published body contains `PENDING-`, or a hash that differs from `SHA256SUMS` | exit 1 | AR-13 |
| ST-30 | Live: a probe times out on every try (no findings); a redirect to a non-allowed host | timeout → exit 3 `unreachable` after 3 attempts; redirect → refused, reported `unreachable` | AR-17, AR-23 |

## Implementation tests

These cover:
- the normalization edge cases: entities, U+200B, link rewriting, offset-to-line mapping;
- the HTML extraction of inline versus block elements and skipped tags;
- the manifest loader's error messages;
- the month table;
- `Retry-After` handling;
- deduplication of findings across pages;
- the CLI usage text.

## Proof (Phase 4)

These runs use the real repository rather than fixtures:
- `make release-docs-check VERSION=1.3.0`;
- the same with `LIVE=1`.

Each finding is either fixed in the docs or allowlisted with a true reason. Then the maintainer
confirms the v1.3.0 record, its After-deploy section is ticked, and the live run passes.
