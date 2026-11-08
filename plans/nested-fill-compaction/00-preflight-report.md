# Plan Preflight Report: Nested Fill Compaction (PLAN-22)

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit date**: 2026-11-08
> **Auditor**: independent plan-preflight subagent (read-only), two passes
> **Verdict**: all 24 first-pass findings and all 14 re-check findings resolved;
> no open critical, major, or minor item.

## Scope

The audit checked every design and execution claim against the current code:
the nested fill branch, `ArrayTarget`, `fill_target_compactable`,
`emit_compact_scalar_fill`, `constant_fill_integer`, `TyKind::Array`,
`ValueKind`, the comparator and its tests, the Makefile targets, the benchmark
scripts, the shape fixture, the existing optimization suites, and the UBSan
pattern.

## First Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-PLAN-001 | CRITICAL | The `prefix >= extent` early return was unreachable because the helper rejected zero covered rows | The helper accepts a uniform zero fill with `covered_rows == 0`; the 64-slot threshold applies only to a non-empty region; IP-17 and a unit case pin it | Resolved |
| PF-PLAN-002 | CRITICAL | The baseline, red-pin, fixture, and findings records had no commits before the head benchmark, so the tree would be dirty | Added commits: plan docs (0.3), baseline (1.5), tooling (2.4), codegen with suites and fixture (4.4), red pins/tooling review (4.5); every `make bench` follows a clean-tree check | Resolved |
| PF-PLAN-003 | CRITICAL | The final acceptance table was filled before the gates and never committed, leaving a dirty tree at push | Order is now findings without gate rows (6.1), closeout docs commit (6.2), gates (6.3), gate evidence plus roadmap Done (6.4), push (6.5) | Resolved |
| PF-PLAN-004 | MAJOR | The gate re-derivation had no executable Phase 1 task | Added 1.3.3 with the exact probe commands; the final `REQUIRE` values are recorded before implementation | Resolved |
| PF-PLAN-005 | MAJOR | Attribution and the emission audit were not reproducible | Added 1.3.1/1.3.4 and 5.2.1 with exact emit, trace-bucket, probe-patch, build, and diff commands; artifacts stay under `build/perf/audit` | Resolved |
| PF-PLAN-006 | MAJOR | IP-18's slice-view emission test is not constructible | IP-18 is now a `function.rs` unit case for a synthetic missing extent, overflow, and zero covered; no emission test | Resolved |
| PF-PLAN-007 | MAJOR | The calibration row could not be filled from the JSON | Removed from the benchmark table; separate line with `calibration.instructions` and `calibration.host_release_ns` only | Resolved |
| PF-PLAN-008 | MAJOR | Comparator tests did not exercise the CLI, stdout, or Makefile | Added `parse_args`, shared-validator, captured-stdout, and `make -n` tests; the named failure is asserted | Resolved |
| PF-PLAN-009 | MINOR | Exclusion guards used exactly 64 slots and had no same-size control | All exclusion guards use at least 72 slots; IP-21 is the 72-slot control; 63/64 pins the boundary | Resolved |
| PF-PLAN-010 | MINOR | `Materialize` was not followed through an inner `ArrayLit` | `uniform_zero_fill` follows `Materialize` first and recurses through every inner value | Resolved |
| PF-PLAN-011 | MINOR | Conditional hidden-return arms were described as non-compact | R3.1, 03-02, AR-28, IP-19, and the accepted limit now distinguish assignment stores from hidden-return arms | Resolved |
| PF-PLAN-012 | MINOR | A colon in a benchmark name was not handled | Parse with `rsplitn(3, ':')`; a colon-name test is listed | Resolved |
| PF-PLAN-013 | MINOR | The Makefile expansion had no automated check | `make -n` tests for empty and two-value `REQUIRE` | Resolved |
| PF-PLAN-014 | MINOR | R7.2 said the PLAN-22 row moves from Executing, but no row existed | R7.2 says to add the row; 6.2 adds it as Executing and 6.4 sets Done after the gates | Resolved |
| PF-PLAN-015 | MINOR | The AGENTS row rode the feature commit | AGENTS is queued for the closeout docs commit | Resolved |
| PF-PLAN-016 | MINOR | Zero-test variants were not pinned | Unit cases for `0u`, `i32(0)`, `-0`, `false`, `Char(0)`, `Materialize`, non-zero, float leaf, and `i32(f32(0.0))`; ST-16/IP-20 for the cast zero | Resolved |
| PF-PLAN-017 | MINOR | The exclusion-removal integrity check had no task | Phase 4.3.3 removes each exclusion in turn and records the guard failure | Resolved |
| PF-PLAN-018 | MINOR | The eligibility helper had no unit tests | Phase 4.1.3 covers 63/64, missing extent, overflow, and zero covered | Resolved |
| PF-PLAN-019 | MINOR | The volatile-global case used a module initializer that does not reach the nested branch | ST-07/IP-11 use a function-body assignment to a volatile global | Resolved |
| PF-PLAN-020 | MAJOR | A post-measurement codegen fix left a stale head document | 5.3.4 and 5.4.3 force a fresh benchmark and emission audit after any review or sanitizer fix; the head table is replaced | Resolved |
| PF-PLAN-021 | MINOR | "Commit per verified task" was not implementable | Seven explicit phase-boundary commits in the commit plan; AR-23 records the cadence | Resolved |
| PF-PLAN-022 | MAJOR | The requirements-preflight closure was premature | PF-REQ-004/015/018/021/026/031/032 were reopened and corrected; the PF-R verdict now records both rounds | Resolved |
| PF-PLAN-023 | MINOR | The bytes table could be read as `target.bytes` | The table names `element(prefix).address/bytes` and warns against `target.bytes`; IP-15 pins it | Resolved |
| PF-PLAN-024 | MINOR | The strict fixture had no nested `bool` case | R5.4, 07, 99 4.2.2, and IP-22 add it | Resolved |

## Re-check Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-PLAN2-001 | MAJOR | R3.1 and the accepted limit still required 64 slots without the zero-covered exception | R3.1 adds the exception; the accepted limit says the threshold applies only to a non-empty region | Resolved |
| PF-PLAN2-002 | MINOR | `Materialize` was still not followed before an inner `ArrayLit` | The inner branch calls `uniform_zero_fill(inner) || constant_fill_integer(inner) == Some(0)` | Resolved |
| PF-PLAN2-003 | MINOR | The accepted limit still said all conditional callers are non-compact | It now says conditional-assignment stores are non-compact and hidden-return arms compact (IP-19) | Resolved |
| PF-PLAN2-004 | MAJOR | A sanitizer-driven codegen fix had no rerun task | Added 5.4.3: commit, rerun 5.1/5.2, replace the head table/audit, re-review once | Resolved |
| PF-PLAN2-005 | MAJOR | The probe, PC buckets, and emission audit lacked exact commands | Added the "Reproducible Audit Commands" section with emit, trace-bucket, matrix-patch, build, size, and diff commands | Resolved |
| PF-PLAN2-006 | MINOR | Invalid-metric tests were at the argparse layer; red-first rationale was imprecise | Split parser-shape tests from shared-validator semantic tests; the red-first note now checks the named failure, not only the exit code | Resolved |
| PF-PLAN2-007 | MINOR | The dependency graph skipped 1.5, 2.4, 4.4, 4.5, 6.4, and 6.5 | Rewrote the graph to include every step in order | Resolved |
| PF-PLAN2-008 | MINOR | R5.4 omitted the nested `bool` fixture case | Added to R5.4 | Resolved |
| PF-PLAN2-009 | MINOR | IP-18's helper is private and cannot be called from the integration suite | IP-18 is labeled a `function.rs` `#[cfg(test)]` unit test | Resolved |
| PF-PLAN2-010 | MINOR | 05 hardcoded the 5/3 requirement | 05 uses `<final requirements>` and notes the Phase 1 ruling | Resolved |
| PF-PLAN2-011 | MAJOR | PF-R still claimed a clean closure while contradictions remained | PF-R records both rounds and the corrections; the remaining contradictions are fixed | Resolved |
| PF-PLAN2-012 | MINOR | AR-26 said 64 slots while the pins use 72 | AR-26 says at least 72 with IP-21 | Resolved |
| PF-PLAN2-013 | MINOR | ST-11's slot count mixed outer elements and leaf slots | ST-11 says 72 outer / 288 leaf and notes the threshold does not apply to struct rows | Resolved |
| PF-PLAN2-014 | MINOR | The roadmap row was set Done before the gates | 6.2 adds it as Executing; 6.4 sets Done after the gates | Resolved |

## Verified Claims

| Claim | Result |
|---|---|
| The insertion point is type-correct and has access to `fill`, `element`, `extents`, `target`, and `compact` | Holds |
| `constant_fill_integer` returns 0 for `0u`, `i32(0)`, `-0`, `false`; rejects `i32(f32(0.0))` | Holds |
| `ArrayTarget::element(prefix)` gives the row address and row size for locals, parameters, and return slots | Holds |
| `target.bytes` is the whole-array count for a parameter and must not be multiplied by rows | Holds |
| The `compact` flag is true for a local and false for field/DerefTemp/volatile/materialized/conditional-assignment/self-read callers; true for a hidden-return conditional arm | Holds |
| 63 slots reject and 64 compact for `i32[7][9]` / `i32[8][8]` | Holds |
| The Makefile `addprefix` expansion is correct for empty and controlled multi-value input | Holds |
| `make verify` includes `check`, `bench-shape`, and `ubsan`; `make asan` is separate | Holds |
| The seven corpus programs split into four portable and three hosted-only | Holds |
| The shape fixture has no nested fill before the change | Holds; Phase 4.2 adds it |
| The harness records identity, dirty, C size, sections, host time, and instructions; only instructions and `.text` are gated | Holds |
| The probe instruction and `.text` deltas are internally consistent | Holds; the absolute C bytes are exploratory and Phase 1 records the artifact |

## Verdict

The plan is grounded in the current code, every shape is testable, the commit
sequence keeps the tree clean at every benchmark, and the gate evidence is
recorded before the push. Both preflight passes are closed. `exec-plan` may
start after the plan documents are committed.
