# Plan Preflight Report: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit target**: the complete plan folder
> `plans/v1.1.2-release-closeout/`
> **Context documents**: [`01-requirements.md`](01-requirements.md),
> [`00-ambiguity-register.md`](00-ambiguity-register.md),
> [`00-preflight-report-requirements.md`](00-preflight-report-requirements.md),
> `RELEASE.md`, `Makefile`, `AGENTS.md`
> **Scope mode**: strict
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved; every
> minor finding was fixed, not accepted.

> ⚠️ **Same-session review.** The plan was authored in this session. Two
> independent read-only auditors scanned it against the real repository and a
> third re-scanned the fixes. The two reserved decisions (version and
> publication) are user rulings.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| v1.1.1 is the tagged release | annotated tag `144e53b` → commit `2d1eb30` |
| 31 pre-plan commits have landed | `git log --oneline v1.1.1..HEAD` |
| Version surfaces are complete | `compiler/Cargo.toml:5`, `Cargo.lock:6-7`, `tools/vscode/package.json:5`, `package-lock.json:3,9`, `README.md:54` |
| The extension version is test-enforced | `tools/vscode/test/manifest.test.ts`, `packaged.test.ts` compare to Cargo |
| `bench-base` records the base commit | `docker/scripts/bench-base.sh` passes `--compiler-git "$COMMIT"` |
| `bench`/`flags.json` record the head identity | `tools/perf/bench.py`, `tools/perf/flag_audit.py` |
| `git archive` reproduces a published source asset | the v1.1.1 archive hash matched |
| `gh` supports the draft/verify/publish flow | `gh release create --draft`, `view --json isDraft,assets`, `edit --draft=false` |
| The repository is private | `gh repo view --json visibility` = PRIVATE |
| `make verify` includes the A600/A1200 gates | `Makefile:217-218` |
| `make verify` regenerates tracked RTG PNGs | `docker/scripts/rtg-smoke.sh:320` |
| `bench-base` keeps only `base-clc` | `docker/scripts/bench-base.sh` trap removes the worktree |

## Scan 1 — Grounding and Delivery

### Major findings

| ID | Finding | Resolution |
|---|---|---|
| PP-001 / PP-101 | The plan pushed C1 before any gate ran. | 2.1.4 commits C1 without pushing; 3.1.11 pushes `main` and the tag only after the gates and artifact audit. |
| PP-002 / PP-101 | The tag was pushed before the release build and archive audit; the failure table's "recreate the local tag" was then impossible. | 3.1.8 creates the tag locally; 3.1.9 builds; 3.1.10 packages and audits; 3.1.11 pushes. The failure row now says "after the push, rebuild from the same tag; never move it". |
| PP-003 / PP-107 | The changelog template carried only the isolated old-flag numbers and omitted the release-tree row, flag ranges, LSP exclusion, and §13 name required by R2.2/R2.3/R2.7. | The template now carries all three bases, the LSP sentence, and the §13 change. |
| PP-102 | The release build was not validated before the tag; task 2.1.3 did not name `--release`, and the only release build was after the tag. | 2.1.3 is `cargo build --release --workspace` with `target/release` version/ELF/not-stripped/hello checks; 3.1.9 rebuilds from the tag. |
| PP-103 / PP-006 | The runbook design listed six sections and dropped failure handling, so a literal rewrite would omit R3.5. | The runbook table now includes Failure handling; task 3.1.3 names it. |
| PP-104 | The claims were reviewed only after publication, with no `gh release edit --notes-file` recovery. | 3.1.6 is a pre-publication claim review; 4.1.5 is the final record review. |
| PP-105 | "All scored" mixed the five scored benchmarks with the 1.3 sieve and overlapped the corpus range. | The five scored set is `.text` 57.1–69.3% / binaries 64.2–74.7%; the 1.3 audit set is 49.1–88.9%, stated separately. |

### Minor findings

| ID | Finding | Resolution |
|---|---|---|
| PP-004 / PP-106 | Phase 2 said 13 tasks but had 14; totals were inconsistent. | Phase 2 is 14; Phase 3 is 13; total is 40. |
| PP-005 / PP-114 | The specialist evidence listed `THIRD_PARTY_NOTICES.md` as changed, but it is only shipped. | Removed from the closeout diff list; it stays in the artifact contents. |
| PP-007 | The Comparisons section still said "57–74%". | Replaced with the five-scored/1.3-audit ranges. |
| PP-008 / PP-121 | "Retained worktree/base crate" was wrong. | Uses the temporary worktree and retained `build/perf/base-clc`, with the rerun command. |
| PP-009 / PP-123 | The release-tree row did not say `hosted-amiga` `lookup` or "same harness flags". | Labeled in the register and design; the host range stays separate. |
| PP-010 | "Public release page" is inaccurate for a private repository. | Uses "published (non-draft) release page". |
| PP-011 | AR-09's rationale overstated the v1.1.1 improvement. | Reworded to the actual change: changelog/notes after measurement, tag local until the audit. |
| PP-012 | AR-09 said three commits, but a plan commit precedes C1. | Now four commits (C0–C3) and a tag, shown in the design diagram. |
| PP-013 | Node is not on the default `PATH` for `make vscode-check`. | The runbook and gate note require Node 22 on `PATH`. |
| PP-108 | `gh release view --json assets` does not return the draft state. | Uses `--json isDraft,assets`. |
| PP-109 | RT-16 and the current-state doc said a regenerated RTG PNG may be committed. | Restore unintended changes; investigate a material difference. |
| PP-110 | The gate section implied everything runs in the container. | The top-level `make` targets run on the host; `vscode-check` needs Node; `gh` runs on the host. |
| PP-111 | The hello-world `clc` invocation was missing; `clc` is emit-only. | Exact `clc`/`cc` and `clorraine` commands added. |
| PP-112 | The specialist refresh appeared in both Phase 1 and Phase 4. | Only Phase 4 refreshes it to 74. |
| PP-113 | The plan-preflight report link did not exist. | This report supplies it before the `Plan Preflighted` row is committed. |
| PP-115 | "31 commits" becomes stale after the plan commit. | "31 pre-plan commits plus this plan's record commits". |
| PP-116 | Task 1.1.5 implied no doc named `1.1.1`; the current docs do. | It now checks that the allowlisted current-version files are the only ones and no test pins it. |
| PP-117 | The review-rulings and accepted-limits findings sections had no owner. | Task 4.1.2 records them. |
| PP-118 | "A gate fails: fix" could imply a compiler change, violating scope. | Environmental/tooling fixes are allowed; compiler/test failures need a user ruling. |
| PP-119 | No remote-sync or non-fast-forward handling. | 1.1.2 asserts `main == origin/main`; a rejected push stops and does not rebase. |
| PP-120 | No existing-draft, auth, or permission failure rows. | Added to the failure table. |
| PP-122 | "Benchmarks/spec unchanged" ignored the release diff. | "Unchanged by the closeout; the release diff adds lookup and the two §13 rows." |
| PP-124 | "No test needed" overstated the extension coverage. | Narrowed to the extension/Cargo surface; the release cases cover the rest. |

## Re-scan

A third read-only auditor re-checked the revised plan against the same
evidence. It found no new critical/major issue after the fixes. The remaining
observations were resolved in place:

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RS-101 | MINOR | The plan preflight report was missing; the roadmap could not be set to `Plan Preflighted`. | This report is committed in 1.1.1. |
| RS-102 | MINOR | The changelog template's line wrapping and R2.3 wording had to match exactly. | Template rewritten with all three bases. |
| RS-103 | MINOR | The phase total after adding the claim review needed to be 40. | Table and all counters now say 40. |

## Final Verification

- Every critical/major finding is resolved and traced into a task.
- Every minor finding is fixed or explicitly reworded; none is silently
  accepted.
- The plan has 40 tasks in 4 phases, with C0–C3 sequencing, a pre-publication
  claim review, and a local tag before the artifact audit.
- The requirements, register, design, testing, execution, and register agree
  on the version, scope, measurement bases, artifacts, tag, and publication.
- The user's two reserved decisions have explicit rulings: version `1.1.2`
  and publication when green.

**Verdict: ✅ PREFLIGHT PASSED — 33 findings (7 major, 26
minor/observation) across two scans and one re-scan; all resolved, 0 open.**
