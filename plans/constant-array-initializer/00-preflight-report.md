# Plan Preflight Report: Constant Array Initialization (PLAN-25)

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit target**: `plans/constant-array-initializer/` — `00-index.md`,
> `01-requirements.md`, `02-current-state.md`, `03-design.md`,
> `05-findings.md`, `07-testing-strategy.md`, `99-execution-plan.md`,
> `00-ambiguity-register.md`
> **Context**: `00-preflight-report-requirements.md` (requirements preflight
> already passed)
> **Scope mode**: strict
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved; the
> remaining minors are fixed or accepted.

> ⚠️ **Same-session review.** The plan was authored in this session. The first
> scan and the two re-scans were dispatched to independent read-only auditors;
> the four phase reviews later provide a separate implementation-period check.
> A future reader should still treat the plan as same-session work.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| The global declaration is a writable empty state | `emit_globals` `compiler/src/codegen/program.rs:1134-1160` |
| Initializers run in `cl_init_<module>` in dependency order | `emit_init_functions` `program.rs:1163-1193`; `global_init_order` `compiler/src/codegen/init.rs:69-120` |
| The array-literal initializer goes through `emit_global_array_fill` | `emit_global_init` `program.rs:1244-1290`; `fills_element_wise` `program.rs:4714-4725` |
| The current path writes one element per statement | `emit_filled_array` `program.rs:1926-1952` |
| The width-correct constant evaluator is `constant_integer` | `compiler/src/codegen/expr.rs:2036-2124` |
| The typed value renderer is `store_text` | `compiler/src/codegen/function.rs:3984` |
| `GlobalInfo` carries the `volatile` flag | `compiler/src/codegen.rs:49-63` |
| `Profile` has host, hosted-amiga, hosted-amiga-13, freestanding | `compiler/src/profile.rs:35-60` |
| `usize`/`isize` are 32-bit typedefs on every profile | `compiler/src/codegen/program.rs:179-180`; `TypeTable::size_of` `compiler/src/types/table.rs:538` |
| Module `const` arrays with explicit lists are rejected | `compiler/src/sema/items.rs:150-156`; E10224 |
| Front-end-invalid shapes | zero extent E10226; slice E10200; handle/owned array E11106; untyped out-of-range literal E10202 |
| The harness records `dirty` and the compiler identity | `tools/perf/bench.py:125-143` |
| The compare gate enforces output and the 1% guard | `tools/perf/bench.py:625-700` |
| The base tag and current head | `v1.1.0` = `c0e34568f423a1ff2994dddd6f2f654e50f338d6`; current = `a589353` |

## Findings and Rulings

The first scan used five clusters; duplicates are merged by root cause. Every
entry is resolved in the final plan docs.

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-P-001 | 🔴 | Every positive/exclusion ST table was 1–4 elements, below the plan's own 16-element/64-byte floor; the expected template could never appear and the exclusions were confounded by the threshold. | ST-1..ST-41 now use eligible sizes (32×`i32`, 64×`u8`/`i8`/`bool`, 32×`i16`/`u16`, 16×`u32`, 32×`usize`/`isize`); ST-32 is the labelled below-threshold control. |
| PF-P-002 | 🟠 | The threshold was “measured in Phase 1” with no executor. | 1.1.5 measures `i32[4]/[8]/[16]/[32]` and `u8[64]` with the isolated-base compiler and records the final bound before Phase 2 sizes the tests; 05 has the threshold table. |
| PF-P-003 | 🟠 | ST-26/27 wrote `false && f()` / `false && (x > 0)` into `i32` tables; bool does not implicitly convert. | ST-27/ST-28 use `bool[64]`; the short-circuit cases assert no call/read and no template. |
| PF-P-004 | 🟠 | ST-25 expected an `E11212` trap from the in-range `i32(f32(1.5))`. | ST-25 is the in-range output-1 case; ST-26 is the out-of-range `i32(f32(3.0e9))` `E11212` case. |
| PF-P-005 | 🟠 | `unique_local("__cl_ctmpl")` returns `__cl_ctmpl` first, not `__cl_ctmpl_0`. | 03-design uses an explicit per-init-function `__cl_ctmpl_<n>` counter; R2.2, ST-43/ST-44, and 05 pin `_0`/`_1`. |
| PF-P-006 | 🟠 | The isolated comparison had no `--require`, so the feature's own win was not gated. | R1.5, R4.1, 3.1.6, 5.1.1, and 05 add `--require lookup:instructions:10 --require lookup:text:10` to the isolated comparison. |
| PF-P-007 | 🟠 | The plan committed a standalone red test-only tree, violating the green-commit rule. | R3.3/R4.5 keep the red tests uncommitted; they land with the green implementation commit; 05 owns the red evidence. |
| PF-P-008 | 🟠 | Phase 4/5 could change codegen after a commit/gate with no loop-back. | 3.1.8/4.1.6/5.1.7 add commit, rebuild `clc`, `make bench`, re-audit, and re-review loops. |
| PF-P-009 | 🟠 | ASan/UBSan discovery markers and `RDC_CLC` path resolution were missing. | 2.1.1 adds the markers, sanitizer flags, and workspace-root resolution; 07 documents the red command. |
| PF-P-010 | 🟠 | The base/isolated evidence table lacked isolated `.text`, C bytes, and the three-row identity table. | 05 has instructions, `.text`/C-byte tables, and base/isolated/head identity rows with path, SHA, commit, version, and dirty. |
| PF-P-011 | 🟠 | The emission audit lacked an explicit profile matrix. | R4.3/5.1.2 define the corpus/benchmark/Amiga profile matrix, including host and freestanding. |
| PF-P-012 | 🟠 | The short-list rationale claimed a shorter template would over-read, contradicting the extent-sized declarator. | 01/02/03/05/AR now use the extent-sized zero-fill / unmeasured tail-policy rationale; the exclusion is a scope choice, not an over-read guard. |
| PF-P-013 | 🟠 | ST-4/6/7/8/9/21 used the Clorraine fill form `[value; count]` while expecting the template. | The positive cases use explicit lists; the only fill case is ST-17 (old stores); 07 defines the ellipsis as a list continuation. |
| PF-P-014 | 🟠 | The plan could run `make bench` with uncommitted red/green, review, roadmap, or index edits, making `dirty=true`. | 3.1.5/4.1.6 commit the evidence/roadmap/index/review records; 3.1.6/5.1.1 confirm a clean tree and `dirty=false`. |
| PF-P-015 | 🟠 | ST-24/31 used a constant `MIN / -1`/`% -1` that fails the strict `-Werror=overflow` build. | R3.1 and ST-24/ST-31 use module-level runtime `min`/`neg` operands; the trap still asserts the same guard/fault. |
| PF-P-016 | 🟡 | `moves` was called “GCC inlines” in 00/02/AR while the prior plan says the by-value copies need an ABI change. | 00/02/AR classify `moves` as out of scope (internal ABI). |
| PF-P-017 | 🟡 | `usize`/`isize` were described as profile-width-dependent; the emitted typedefs are 32-bit everywhere. | 01/02/03/05 say “checked type table; `usize`/`isize` are 32-bit”; the threshold is not target-width-dependent. |
| PF-P-018 | 🟡 | The design said `constant_integer` “supplies the value used by `store_text`”; `store_text` renders the original IR. | 03-design says `constant_integer` is only the eligibility/value gate; the template always renders with `store_text`. |
| PF-P-019 | 🟡 | ST-19/29/30/35/39/46 had ambiguous rows. | ST-19 lists all four rows; ST-29 returns 7; ST-30 uses `c = true` → 1; ST-35 says “zero elsewhere”; ST-39 is module-level; ST-46 leaves the emission assertion to the impl suite. |
| PF-P-020 | 🟡 | `bench-shape` legs were misnamed, and the fixture task did not say module-level. | The docs say hosted-amiga (m68k GCC), hosted-amiga-13 (nix13), vbcc, freestanding; R3.6/2.1.3 require a module-level fixture. |
| PF-P-021 | 🟡 | The roadmap row, plan-index status, comment hygiene, and `lookup-smoke.json` task were missing or late. | 1.1.2 creates the smoke JSON; 3.1.5 sets the roadmap/index Executing; 5.1.4 checks comment hygiene; 6.1.1 sets Done. |
| PF-P-022 | 🟡 | The benchmark checksum is not permutation-sensitive. | Accepted: it detects skipped/zeroed/partial copies; the behaviour/emission suites cover exact values and order; 05 records the limit. |
| PF-P-023 | 🟡 | The task count/phase table was stale. | The plan is 37 tasks across 6 phases, matching the phase table. |
| PF-P-024 | 🟡 | The base identity used a short SHA, and some `bench.py` line references were stale. | 05 uses the full `v1.1.0` SHA; 02 cites the current `bench.py` functions/lines. |

## Iteration Record

| Iteration | Scope | Result |
|---|---|---|
| 1 | Five-cluster full scan | 1 critical + 8 major + many minor findings (merged above) |
| 2 | Re-scan of the first fix pass | B1 fill shorthand, B2 dirty-tree ordering, B3 strict-build overflow, B4 short-list wording; all resolved |
| 3 | Re-scan of the second fix pass | One major remained: R3.1 still listed the constant `MIN / -1` form; fixed by aligning R3.1 with the runtime `min`/`neg` ST cases. A targeted grep/consistency check confirms no constant-form reference and no fill shorthand in the positive cases. |
| Final | Targeted consistency check | PASS — no critical/major finding; the remaining minors are fixed or accepted |

A fourth full audit is not required by the shared protocol: iteration 3's one
major was the same root cause as B3 and was fixed by a direct wording
alignment; the targeted check covers its direct dependency surface.

## Scope and Complexity

- Strict scope was used; no optional addition was proposed or kept.
- The plan adds no new crate, build script, harness code, specialist agent,
  runtime-shim change, or language/ABI surface.
- The only measurement surface is the committed `lookup` benchmark, explicitly
  allowed by the goal and justified as a real sine/lookup-table workload.
- Auto-design was used only for eligible technical decisions; product scope
  and the v1.1.0 base remain user-owned.

## Specialist Check

**Specialist check: None —** the project already has the four reviewers this
codegen change needs (correctness, memory safety, emission integrity,
performance); no new domain, dependency, protocol, or infrastructure gap was
found. Evidence: `AGENTS.md` phase-review dispatch and
`codeops/specialist-check.json`.

## Final Verdict

**✅ PREFLIGHT PASSED** — all critical/major findings resolved, the remaining
minor notes fixed or accepted, no unresolved scope-expansion proposal, and the
plan is ready for `exec-plan --auto-commit --auto-design`.
