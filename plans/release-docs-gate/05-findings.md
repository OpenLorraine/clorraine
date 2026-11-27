# Findings ledger: release docs gate

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Phase 1 review (correctness)

| ID | Severity | Finding | Disposition |
|----|----------|---------|-------------|
| P1-RV-001 | MAJOR | Allowlist containment untested (19/40 mutants survived overall) | Fixed: two spec cases (token outside the phrase; phrase ending inside a token) |
| P1-RV-002 | MAJOR | `required` rule and quick-start heading untested | Fixed: two spec cases |
| P1-RV-003 | MINOR | version-input variants, changelog window, HEAD-vs-tree placeholder, record header/order, loader checks, `vv` untested | Fixed: spec cases for each |
| P1-RV-004 | MINOR | Untracked review record accepted | Fixed: `review` finding (AR-40) |
| P1-RV-005 | MINOR | Second placeholder line in the working tree unreported; banned wording on the placeholder line skipped | Fixed: count in HEAD and tree; only the placeholder pattern exempt |
| P1-RV-006 | MINOR | Sidebar text and link checked separately | Fixed: one-item match |
| P1-RV-007 | MINOR | Manifest keys beyond 03-01 | Fixed: 03-01 schema table updated (AR-40) |
| P1-RV-008 | MINOR | `required` placeholder loader check missing | Fixed |
| P1-RV-009 | MINOR | Token lookbehind differed from the spec; two-part release refs | Fixed: spec lookbehind; release references (AR-40) |
| P1-RV-010 | MINOR | Link labels could not wrap | Fixed |
| P1-RV-011 | MINOR | Non-UTF-8 or missing tracked files crashed | Fixed: `unreadable` finding; non-git repo exits 2 |
| P1-RV-012 | MINOR | File-level findings without a line; hard-coded manifest path; literal U+200B; upper-case `.MD` | Fixed except file-level findings, which have no single line (accepted) |

## Phase 2 review (correctness)

| ID | Severity | Finding | Disposition |
|----|----------|---------|-------------|
| P2-RV-001 | MAJOR | Timestamp freshness does not fit the real deploy (author dates, shallow clone, non-site HEAD) | Fixed: deployment-based freshness (AR-39) with spec cases |
| P2-RV-002 | MAJOR | 14 live behaviours without a failing control | Fixed: a spec case per behaviour; all 12 re-run mutants now fail the suite |
| P2-RV-003 | MINOR | Page set built from `scan.exclude` | Fixed: the site's `srcExclude` |
| P2-RV-004 | MINOR | Outside git: traceback | Fixed: exit 2 |
| P2-RV-005 | MINOR | Missing `SHA256SUMS` reported as unreachable | Fixed: finding before download |
| P2-RV-006 | MINOR | Prerelease flag not in the summary | Fixed |
| P2-RV-007 | MINOR | HTTPException, bad JSON, malformed href crashed | Fixed |
| P2-RV-008 | MINOR | Trailing punctuation, `releases/latest/download`, other Marketplace items, `gh` outside the budget | Fixed |

## Phase 2 audit (security)

| ID | Severity | Finding | Disposition |
|----|----------|---------|-------------|
| SA-001 | MINOR | Some network/parse errors crashed | Fixed (with P2-RV-007); `gh` output decoded as UTF-8 with replacement |
| SA-002 | MINOR | Userinfo host mismatch | Fixed: userinfo and ports refused |
| SA-003 | MINOR | CDN suffix allowed from any origin | Fixed: only after a Marketplace start |
| SA-004 | MINOR | Per-request and run budgets not enforced | Fixed: wall-clock body reads, per-hop deadline, `gh` within the budget |
| SA-005 | MINOR | Link segments unvalidated before `gh` | Fixed: segment and tag validation, `quote(safe="")` |
| SA-006 | MINOR | GH_HOST/GH_REPO could redirect probes | Fixed: pinned host/repo arguments, variables stripped |
| SA-007 | MINOR | `None == None` freshness pass | Moot: freshness no longer compares timestamps (AR-39) |
| SA-008 | MINOR | Missing SHA256SUMS → unreachable | Fixed (with P2-RV-005) |
| SA-009 | MINOR | Transport and budget properties untested | Fixed: local-server transport tests, hop count, fake-clock budget, off-site sitemap location |

## Phase 3 gate

`make check IMAGE=clorraine-sdk:windows-cli`: passed (4529 Rust tests; 66 release-docs tests at that point).

## Phase 4 proof

- The human review found two real docs gaps (the site's limitations summary lacked screen text clipping and had a stale clipboard sentence); fixed in 9e917a12.
- Replaying the gate at 54b0d9b6 (before the maintainer's post-release fixes) reports exactly the three defects the maintainer found by hand.
- The proof run found one checker bug (blockquote markers split claims) and improved allowlist suggestions.
