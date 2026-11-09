# Plan Preflight Report: Volatile Nested Fill Fix (PLAN-23)

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit date**: 2026-11-09
> **Auditor**: independent plan-preflight subagent (read-only), three passes
> **Verdict**: all 19 first-pass, 9 second-pass, 6 third-pass, and 3 fourth-pass
> findings are resolved; a final re-check confirmed the revisions. No open
> critical, major, or minor item.

## Scope

The audit read the active goal, the plan set, the release runbook, the
roadmap, and the repository facts the design relies on: the nested fill
branch, `ArrayTarget`, `fill_target_compactable`,
`emit_fixed_array_assign_to`, `emit_move_place`, `emit_struct_literal_fields`,
`place()`, `field_type()`, `element_type()`, the struct tables, the existing
optimization suites, the shape fixture, the benchmark harness, and the v1.1.0
release history.

## First Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-VOL-001 | CRITICAL | The fix changed only the fill-row copy; a place-valued prefix row still reached `emit_fixed_array_assign_to` and `memcpy` | The plan adds a dedicated `emit_prefix_row_assign_to` wrapper for the prefix loop (ST-V5/IP-V5). The struct-literal prefix element is a separate limitation; R1.6 is narrowed to the nested fill path | Resolved |
| PF-VOL-002 | MAJOR | A `for borrow` alias or a `borrow` parameter could alias volatile storage but the syntactic predicate cannot see it | The alias and borrow shapes are probed and recorded as separate limitations: the `for borrow` conversion and the call argument discard the qualifier before the fill; a global borrow is rejected (`E10038`) | Resolved |
| PF-VOL-003 | MAJOR | The benchmark and emission base was `b68dae5`, which predates PLAN-22, so the comparison and audit included the prior optimization | The plan builds a fresh base at `7cdbb71`, runs `make bench`, and uses the bare `make bench-compare` 1% guard; the emission audit uses the same base | Resolved |
| PF-VOL-004 | MAJOR | Pins used bare global names while the emitted symbol is `cl_<module>_<name>` | Every global pin uses the full mangled symbol; the existing guard is replaced | Resolved |
| PF-VOL-005 | MAJOR | The source tarball was scheduled before the tag it is built from | The tag is created in 5.5.1 and the artifacts in 5.5.2 | Resolved |
| PF-VOL-006 | MAJOR | Publication had no explicit authorization gate | 5.6.1 asks for explicit session authorization; if not granted, the tag stays local and the artifact hashes are recorded | Resolved |
| PF-VOL-007 | MAJOR | The requirements said the release was out of scope while the execution plan included it | The plan states the row mapping: Phases 0-4 deliver PLAN-23 (R1-R4); Phase 5 is the PLAN-24 checklist following `RELEASE.md` | Resolved |
| PF-VOL-008 | MINOR | `.len` was skipped unconditionally | The predicate mirrors `place()`: fold `.len` only on a fixed array; IP-V6 pins a struct field named `len` | Resolved |
| PF-VOL-009 | MINOR | Review-driven code changes had no rerun or commit task | 3.7.1 commits any fix, reruns focused tests, `make bench`, and the emission audit on the new clean commit, and re-reviews once | Resolved |
| PF-VOL-010 | MINOR | Phase task counts did not match the tasks | The plan is 43 tasks; the phase table matches the task lists | Resolved |
| PF-VOL-011 | MINOR | R4.3 said "replace" a known-limitations line that does not exist | R4.3 says to add the fixed statement and the remaining limitations | Resolved |
| PF-VOL-012 | MINOR | The roadmap progress header was not updated | 0.2.1, 4.2.1, and 5.7.1 set the header counts and status explicitly | Resolved |
| PF-VOL-013 | MINOR | The VS Code manifest stayed at 1.1.0 and the release gate omitted `make vscode-check` | 5.1.1 bumps `tools/vscode/package.json` and the root lock; 5.4.1 runs `make vscode-check` | Resolved |
| PF-VOL-014 | MINOR | The overlay fixture helper could be unused and the overlay shape had no host compile check | The helper is called from `main`; ST-V6 is a strict host build-only case | Resolved |
| PF-VOL-015 | MINOR | The runbook lacked gate-evidence, licence-audit, exact-name, and historical tasks | 5.2.2, 5.4.2, 5.5.2, and 5.6.1 cover them, including `--notes-file` | Resolved |
| PF-VOL-016 | MINOR | The performance auditor and compiler SHA/dirty evidence were missing | 3.1.1/3.1.2 record both identities; 3.6.1 dispatches the performance auditor | Resolved |
| PF-VOL-017 | MINOR | The current-state document said the tree was clean while the plan docs were untracked | It now says the tracked tree is clean and the plan set is untracked until the Phase 0 commit | Resolved |
| PF-VOL-018 | MINOR | The full gate ran before the closeout commit, so it tested a dirty tree | 4.3 commits the closeout, then 4.4 runs the full gate on the clean commit | Resolved |
| PF-VOL-019 | MINOR | The spec header, audit sentence, and focused command were imprecise | The spec header update is an explicit 1.1.1 task; the audit sentence names the committed corpus/benchmarks/examples; the focused command uses the container form | Resolved |

## Second Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-VOL2-001 | MAJOR | Copy Site 2 was the shared `!needs_drop` branch, so it would also change whole-array copies (`g = other`), contradicting the declared scope | The design replaces the shared-core edit with a dedicated `emit_prefix_row_assign_to` wrapper used only by the nested fill prefix loop. The shared core is unchanged. IP-V12 pins the unchanged whole-array `memcpy` | Resolved |
| PF-VOL2-002 | MAJOR | Publication was treated as already authorized by the plan request | 5.6.1 is an explicit authorization gate; without a grant, the tag stays local and publication is recorded as blocked | Resolved |
| PF-VOL2-003 | MAJOR | The alias/borrow limitation was global-only; a volatile field through a local `raw` pointer was not recorded | The plan records both: a `for borrow` over volatile storage fails at the borrow conversion; a `borrow` call argument to a volatile field fails at the call site; a global borrow is `E10038` | Resolved |
| PF-VOL2-004 | MINOR | Owning explicit elements were scoped out but not listed as an accepted limit, and `E10215` does not cover an owning prefix row | The plan records that a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`), so no fill form is representable. An owning explicit row copy (no fill) is recorded as a separate shared-core limitation | Resolved |
| PF-VOL2-005 | MAJOR | The index and preflight report still described the narrower, pending plan | The index status, scope, decisions, and specialist table are revised; this report records both passes and the final verdict | Resolved |
| PF-VOL2-006 | MINOR | Some examples and IP-V5/IP-V4 descriptions still used bare `g` or vague store text | The examples use the mangled symbol where they show emitted C; IP-V4 and IP-V5 give concrete store strings | Resolved |
| PF-VOL2-007 | MINOR | "A failed gate blocks the commit" was wrong because the closeout commit precedes the gate | The testing strategy now says a failed gate blocks the push and requires a follow-up fix commit | Resolved |
| PF-VOL2-008 | MINOR | The spec header update was not an explicit task | 1.1.1 now includes the header update: existing cases pass on base; the new volatile cases are red on base | Resolved |
| PF-VOL2-009 | MINOR | The release phase was embedded in the fix plan without a row mapping | The execution plan states that Phase 5 is the PLAN-24 checklist and that Phases 0-4 are PLAN-23; the index records the same mapping | Resolved |

## Third Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-VOL3-001 | MAJOR | "No representable in-scope shape" for owning elements was asserted without a probe; an owning explicit row copy into a volatile field could still reach the shared core | The plan no longer claims no shape exists. It records an owning explicit row copy (no fill) as a separate shared-core limitation, and states that a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`) | Resolved |
| PF-VOL3-002 | MINOR | The index said the final preflight was pending while the report said it was complete | The index says "Preflight clean (three passes); execution ready"; the report records three passes | Resolved |
| PF-VOL3-003 | MINOR | Some emitted-C examples still used bare `g`; IP-V4 and IP-V12 were imprecise | The examples use the mangled symbols; IP-V4 and IP-V12 give exact store and `memcpy` strings; IP-V12 also asserts no leaf store | Resolved |
| PF-VOL3-004 | MINOR | "Update IP-11" collided with the new IP-V11 | The execution plan says to replace the legacy `should_keep_a_volatile_global_target` test (old IP-11) with IP-V1..V12 and states that the old IP-11 and new IP-V11 are different pins | Resolved |
| PF-VOL3-005 | MINOR | The conditional/call/closure prefix fallback was not listed as an accepted limit | It is recorded in the requirements, current state, design, and findings | Resolved |
| PF-VOL3-006 | MINOR | The release task named a nonexistent root `package-lock.json` | 5.1.1 names `tools/vscode/package.json` and the root package entry in `tools/vscode/package-lock.json` | Resolved |

## Fourth Pass

| ID | Severity | Finding | Resolution | Status |
|---|---|---|---|---|
| PF-VOL4-001 | MINOR | The index Document Index row still said "two-pass" while the report recorded three | The row says "Three-pass adversarial audit and rulings" | Resolved |
| PF-VOL4-002 | MINOR | The index out-of-scope list omitted conditional/call/closure prefix rows | The index lists them, matching the requirements and findings | Resolved |
| PF-VOL4-003 | MINOR | The preflight's PF-VOL2-004 resolution still used the old absolute owning-element claim | It now says no fill form is representable and records the owning explicit row copy as a separate shared-core limitation | Resolved |

## Final Re-check

| Item | Result |
|---|---|
| Both nested fill copy sites are designed and pinned | Holds: fill rows (ST-V1..V4, IP-V1..V4) and prefix place rows (ST-V5, IP-V5) |
| The shared core is unchanged | Holds: the prefix wrapper is the only new call; IP-V12 pins the whole-array `memcpy` |
| The predicate mirrors `place()` for `.len` and raw pointer fields | Holds: the fixed-array guard and the raw layer walk; IP-V6 pins a field named `len` |
| The non-volatile path is unchanged | Holds: the flag defaults false; IP-V7 and IP-V8 guard the `memcpy` and `memset` spellings |
| The benchmark comparison isolates the fix | Holds: base `7cdbb71`, head fix commit, bare 1% compare |
| The emission audit isolates the fix | Holds: base compiler from `7cdbb71`; no committed corpus/benchmark/example shape is volatile |
| The release follows the v1.1.0 pattern | Holds: release commit, gates, tag, artifacts from the tag, authorization, publication, post-tag publication record |
| The tree is clean at every benchmark and at the tag | Holds: the fix commit precedes 3.1; the closeout commit precedes 4.4; the release commit precedes 5.4; the tag follows 5.4 |
| The authorization and scope are explicit | Holds: PLAN-23 fix, PLAN-24 release, publication gate in 5.6.1 |
| The out-of-scope volatile copies are recorded | Holds: whole-array copy, struct-literal prefix, alias/borrow indirection, owning elements |

## Verdict

The plan is grounded in the current code. It fixes both copies the nested fill
path uses, keeps the shared store core and every non-volatile unit unchanged,
isolates the benchmark and emission evidence to the fix's parent, records the
remaining volatile-copy classes as separate limitations, and gates publication
on explicit authorization. The preflight is clean; execution may start after
the plan documents are committed.
