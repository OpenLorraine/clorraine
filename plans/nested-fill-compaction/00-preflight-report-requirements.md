# Requirements Preflight Report: Nested Fill Compaction (PLAN-22)

> **Document**: 00-preflight-report-requirements.md
> **Parent**: [Index](00-index.md)
> **Audit date**: 2026-11-08
> **Auditor**: independent preflight subagent (read-only)
> **Verdict**: all 33 findings resolved; no open critical, major, or minor item.

## Scope

The audit read the active goal, the requirements set (`00-index.md`,
`01-requirements.md`, `02-current-state.md`, `00-ambiguity-register.md`), and
the repository facts they rely on: `tools/perf/bench.py`,
`compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs`,
`compiler/src/types/mod.rs`, the existing optimization suites, `AGENTS.md`,
`docs/known-limitations.md`, the matrix benchmark, the corpus, and the shape
fixture.

## Findings and Resolutions

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-REQ-001 | CRITICAL | PLAN-20 was already the v1.1.0 release row; the plan reused the ID | Renamed the feature to PLAN-22 everywhere; the roadmap row is new | Resolved |
| PF-REQ-002 | CRITICAL | Positive pins used shapes below the 64-slot threshold | IP-01/02 `i32[8][8]`, IP-03 `i32[4][4][4]`, IP-04 `i32[16][8]` with 8 prefix rows; ST cases resized | Resolved |
| PF-REQ-003 | MAJOR | "Every assertion fails without the fix" is impossible for exclusion guards | Split positive pins (red on base) from exclusion guards (green on both; fail when the exclusion is removed) | Resolved |
| PF-REQ-004 | MAJOR | Exclusion pins could pass because of the threshold, not the exclusion | Every exclusion pin uses at least 72 slots and records the same-size positive control IP-21; the boundary is pinned by IP-05/IP-06. Reopened by the plan preflight and corrected | Resolved |
| PF-REQ-005 | MAJOR | The behaviour oracle mostly exercised below-threshold shapes | Resized the behaviour cases; added below-threshold controls ST-12/ST-15 | Resolved |
| PF-REQ-006 | MAJOR | "Compare with the base compiler" is not available to `cargo test` | Expected output is recorded from a run on the unmodified compiler; the suite spawns only `CARGO_BIN_EXE_clc` | Resolved |
| PF-REQ-007 | MAJOR | The suite builder had no UBSan flags | Added `-fsanitize=undefined -fno-sanitize-recover=all` when `RDC_UBSAN` is set | Resolved |
| PF-REQ-008 | MAJOR | `bench-shape` fixture has no nested fill | Phase 4.2 updates `tools/perf/fixtures/shapes.clo`; R5.4 and 07 name the cases | Resolved |
| PF-REQ-009 | MAJOR | Final gate ran `bench-compare` without the declared requirements | Final gate uses `REQUIRE="<final requirements>"` in R5.5, 05, and 99 | Resolved |
| PF-REQ-010 | MAJOR | Removing the C-byte target breaks an existing unit test | The C-byte test is retired/rewritten as an invariant case; R2.6 and 03-01 record it | Resolved |
| PF-REQ-011 | MAJOR | Untracked plan docs make the harness record `dirty=true` | Commit the plan docs first; base source `b68dae5`, head plan-docs commit with no compiler change; `dirty=false` | Resolved |
| PF-REQ-012 | MAJOR | Requiring equal base/head SHA-256 is likely to fail on debug path embedding | Record both hashes; require source identity, not hash equality; investigate a source-level difference | Resolved |
| PF-REQ-013 | MAJOR | Absolute probe C bytes did not reproduce | Marked C bytes exploratory; Phase 1 re-runs with `build/perf/base-clc`; the gate uses instructions and `.text` | Resolved |
| PF-REQ-014 | MAJOR | "No prefix elements at any level" contradicted the allowed top-level prefix | The empty-element rule applies to the fill chain only; prefix examples resized to 64 covered slots | Resolved |
| PF-REQ-015 | MAJOR | `prefix >= extent` still emitted the row temporary, contrary to AR-13 | The helper returns true for a uniform zero fill with zero covered rows; the early return is reachable. Reopened and corrected by the plan preflight; IP-17 pins it | Resolved |
| PF-REQ-016 | MAJOR | The tooling review was scheduled but had no task | Added Phase 2.5 with `correctness-reviewer` and a verdict gate before Phase 3 | Resolved |
| PF-REQ-017 | MAJOR | The codegen commit had no benchmark comparison recorded | Commit first, measure immediately after, record in the findings; a failed comparison blocks closeout; AR-22 records the ruling | Resolved |
| PF-REQ-018 | MAJOR | The 5%/3% gate was fixed from an unreproducible probe | Gate is provisional; Phase 1 task 1.3.3 re-runs the probe with `build/perf/base-clc`, records the exact commands and numbers, and records the final requirement. Reopened and corrected by the plan preflight | Resolved |
| PF-REQ-019 | MINOR | Only `extents[1..]` was checked for missing extents | Require every extent, including the first, to be `Some`; IP-18 | Resolved |
| PF-REQ-020 | MINOR | Slot-count overflow was not specified | `checked_mul`; overflow rejects; R3.1 | Resolved |
| PF-REQ-021 | MINOR | The zero predicate was underspecified for casts and `Materialize` | Exact predicate documented; `uniform_zero_fill` follows `Materialize` before the `ArrayLit` match and recurses through every inner value; float-to-int casts rejected; unit tests. Reopened by the plan preflight and corrected | Resolved |
| PF-REQ-022 | MINOR | `EnvField`/`Temp` targets were unmentioned | Allowed by the inherited gate; recorded as an accepted limit with the real-storage argument | Resolved |
| PF-REQ-023 | MINOR | The harness was said to reject every missing metric | Corrected: only `instructions_net` and `.text` are required; C bytes and host time are advisory | Resolved |
| PF-REQ-024 | MINOR | "Four corpus programs" was wrong | All seven are audited (four portable, three hosted-only) | Resolved |
| PF-REQ-025 | MINOR | The threshold rationale overclaimed PLAN-14 | Corrected: 64 is a conservative floor, not a measured nested-`memset` optimum | Resolved |
| PF-REQ-026 | MINOR | The attribution lacked trace evidence | Sieve trace buckets recorded; strings/moves labeled code inspection. Phase 1 keeps emitted C and trace logs under `build/perf/audit` and records the commands. Reopened and corrected by the plan preflight | Resolved |
| PF-REQ-027 | MINOR | "The nested path never reaches compaction" was wrong | Corrected: rows of 64+ slots already compact; the outer memset supersedes them | Resolved |
| PF-REQ-028 | MINOR | "No-regression only" and "no measurement code" were imprecise | Corrected to "invariants only" and "no measurement path change; compare interpretation only" | Resolved |
| PF-REQ-029 | MINOR | The execution mode omitted `--auto-design` | Added to the plan header and decisions | Resolved |
| PF-REQ-030 | MINOR | The preflight reports were missing from the document index | Added PF-R and PF-P rows | Resolved |
| PF-REQ-031 | MINOR | The helper unit-test task could be skipped silently | The predicate is a free function with a `#[cfg(test)]` module and pinned cases; the eligibility helper also gets 63/64, missing-extent, overflow, and zero-covered cases. Reopened and extended by the plan preflight | Resolved |
| PF-REQ-032 | MINOR | The `compact=false` caller paths were incomplete | R3.1 and the design distinguish conditional-assignment stores (false) from hidden-return conditional arms (true, IP-19); the accepted limit is aligned. Reopened by the plan preflight and corrected | Resolved |
| PF-REQ-033 | MINOR | The strict-build leg description was inaccurate for freestanding | Named the exact gates: `bench-shape` build-only, `corpus-amiga` hosted/vbcc, freestanding run in `make check` | Resolved |

## Verified Claims

| Claim | Result |
|---|---|
| HEAD and `origin/main` at `b68dae5`; count zero | Holds |
| 64 commits since PLAN-14 closeout | Holds |
| Roadmap 565/567, 2 parked | Holds |
| PLAN-20 is already used | Held; renamed to PLAN-22 |
| Harness records identity, dirty, C size, sections, host time, instructions | Holds |
| Harness hardcodes the sieve 20/10/25 rules and the generic improvement rule | Holds |
| 1% no-regression guard exists for instructions and `.text` | Holds |
| Nested fill branch emits row temp + stores + memcpy | Holds |
| `FILL_COMPACT_THRESHOLD=16`, `FILL_LOOP_THRESHOLD=64` | Holds |
| `constant_fill_integer` is the width-correct helper | Holds |
| `ArrayTarget::element` gives the row size for parameters/return slots | Holds |
| Existing c-byte test conflicts with R2.3 | Held; test retired |
| `bench-shape` fixture has no nested fill | Held; Phase 4.2 adds it |
| Probe instruction and `.text` deltas reproduce | Provisional: the exploratory numbers are internally consistent; Phase 1 task 1.3.3 re-runs with `build/perf/base-clc` and records the artifact |
| Probe absolute C bytes reproduce | Do not; marked exploratory and re-run in Phase 1 |
| Current `make bench-compare` forwards `REQUIRE` | Does not; designed in 03-01 |

## Verdict

The requirements preflight is clean after the resolutions above. The plan
preflight reopened PF-REQ-004, 015, 018, 021, 026, 031, and 032; this revision
corrects them:

- exclusion shapes now use 72 slots with the same-size control IP-21;
- the zero-covered early return is reachable because the helper accepts
  `covered_rows == 0` and the 64-slot threshold applies only to a non-empty
  region;
- Phase 1 task 1.3.3 re-runs the probe with `build/perf/base-clc`, and 07
  records the exact emit, patch, trace-bucket, and emission-diff commands;
- `uniform_zero_fill` follows `Materialize` before the `ArrayLit` match and
  recurses through every inner value;
- the conditional-caller description distinguishes assignment stores from
  hidden-return arms;
- the eligibility helper has 63/64, missing-extent, overflow, and zero-covered
  unit cases.

A second plan re-check found four major and ten minor items (probe command
detail, sanitizer-fix rerun, R3.1 threshold exception, PF-R accuracy); all are
corrected in the current documents. No critical, major, or minor finding
remains open. The plan preflight may proceed.
