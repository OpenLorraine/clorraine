# Requirements Preflight Report: Constant Array Initialization (PLAN-25)

> **Document**: 00-preflight-report-requirements.md
> **Parent**: [Index](00-index.md)
> **Audit target**: `plans/constant-array-initializer/01-requirements.md`
> **Context documents**: `plans/constant-array-initializer/00-index.md`,
> `plans/constant-array-initializer/00-ambiguity-register.md`
> **Scope mode**: strict
> **Audit target hashes**:
> - revision 1 (first scan): `64964f4506b910497d67ae3bf337ab23a1df5ceb9c398c50a79a7d2fed3194bd`
> - revision 2 (after the first fix pass): `a8a6740b8780737a81bad748e74aac0501551343066b2d583f2730c788eb4e29`
> - revision 3 (after the profile fix): `753bc570413f0b0adb233379a5a15f78522085e072dcd67e9d02884eba73c80a`
> - final (after the minor doc edits): `c6c7af0c6e46cb8b40bb1f477b6b5739ce3c8e648000570c4a372448a0c65b8a`
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved;
> the remaining minor notes were fixed in the final revision.

> ⚠️ **Same-session review.** The requirements were authored in this session.
> The first scan and the two re-scans were dispatched to independent
> read-only auditors; the four phase reviews later provide a separate
> implementation-period check. A future reader should still treat the
> requirements as same-session work.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| The current global declaration is a writable empty state | `emit_globals` `compiler/src/codegen/program.rs:1134-1160` |
| Global initializers run in `cl_init_<module>` in dependency order | `emit_init_functions` `program.rs:1163-1193`; `global_init_order` `compiler/src/codegen/init.rs:69-120` |
| The array-literal initializer goes through `emit_global_array_fill` | `emit_global_init` `program.rs:1244-1290`; `fills_element_wise` `program.rs:4714-4725` |
| The current path writes one element per statement | `emit_filled_array` `program.rs:1926-1952` |
| The width-correct constant evaluator is `constant_integer` | `compiler/src/codegen/expr.rs:2036-2124` |
| The typed value renderer is `store_text` | `compiler/src/codegen/function.rs:3984` |
| `GlobalInfo` carries the `volatile` flag | `compiler/src/codegen.rs:49-63`; `record_global` `:616-640` |
| Module `const` arrays with explicit lists are rejected | `compiler/src/sema/items.rs:150-156`; `compiler/src/consts/mod.rs:168-204`; E10224 |
| Front-end-invalid shapes | zero extent E10226; slice E10200; handle/owned array E11106; untyped out-of-range literal E10202 |
| The harness records compiler identity, C size, sections, host time, instructions | `tools/perf/bench.py` `compiler_identity`, `measure`, `size_sections`, `trace_vamos` |
| The compare gate enforces output/identity and 1% no-regression | `tools/perf/bench.py` `command_compare` |
| Base tag and current head | `v1.1.0` = `c0e34568f423a1ff2994dddd6f2f654e50f338d6`; current head = `a589353`; `v1.1.0..HEAD` changes 25 compiler files from the already-landed work |
| Probe on the committed benchmark | `lookup` current 17,025 instructions / 117,440 `.text`; block-scope template 12,842 / 92,976; output `lookup: 134217728` equal |

## Findings and Rulings

The five-cluster first scan produced 41 local PA findings; duplicates are
merged below by root cause. Every entry is resolved in the final requirements.

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🔴 | A short/empty explicit list was eligible; a template built from only the listed elements plus `sizeof(global)` would over-read and corrupt the zero tail. | R2.1 now requires `elements.len() == extent`; R2.4 excludes short/empty lists; R3.1/R3.2 add behavior and emission cases. **Plan-preflight update:** the final design uses an extent-sized template declarator, so a short list would zero-fill rather than over-read; the exclusion remains because the tail policy is unmeasured, not because of an over-read. |
| PF-002 | 🔴 | `constant_integer` accepts `MIN / -1`, float-to-integer casts, and short-circuit forms whose current emission carries a trap or an unevaluated operand; folding them would remove a guard or emit UB. | R2.1 now requires a guard-free, effect-free structural predicate (literals; non-float integer casts; `Neg`/`BitNot`/`Not` only); R2.4/R3.1 add the exclusion and trap cases; R2.8 forbids guard removal. |
| PF-003 | 🔴 | The base/head identity rule forbade the cumulative `v1.1.0` comparison that R1.5 requires, and “head” named two different commits. | A “Two Comparisons” section defines cumulative (`v1.1.0`→final; prior compiler changes expected/enumerated) and isolated (benchmark commit→final; only the feature diff); the blocker applies to unexpected differences. |
| PF-004 | 🟠 | Module-level `const` arrays with explicit lists are rejected with `E10224`, so the eligible `const` branch and the `const` run case were unreachable. | R2.1 is `let`-only; `const` arrays are recorded as front-end-invalid/out of scope; R3.1 uses a compile-fail case. |
| PF-005 | 🟠 | The `lookup` checksum was payload-blind: the sine table summed to zero, so an all-zero or skipped template printed the same output. | The table is offset by 32768 to 1..65535 and the sum is 134,217,728; the README and R1.3 state that a skipped/zeroed/partial copy changes the output. |
| PF-006 | 🟠 | The rule had no lower bound and would add a template plus `memcpy` to one-element arrays where it is not a measured win. | R2.1 adds a provisional 16-element/64-byte lower bound; Phase 1 measures and finalizes it. |
| PF-007 | 🟠 | The volatile check was placed in `emit_global_array_fill`, whose signature does not carry the flag. | R2.7 places the check in `emit_global_init` (where `GlobalInfo.volatile` is available) or passes it explicitly; R3.2 adds the volatile exclusion. |
| PF-008 | 🟠 | A template name could collide with a global or an init local and shadow it inside `cl_init`. | R2.2 uses the reserved `__cl_ctmpl_` prefix plus a per-function counter; R3.2 adds collision tests with a global and an earlier `__cl_itmp` local. |
| PF-009 | 🟠 | `usize`/`isize` are eligible but were absent from the behavior and emission cases; their width differs between host and m68k. | R3.1/R3.2 add `usize`/`isize` behavior and emission pins. |
| PF-010 | 🟠 | The retained `build/perf` baseline was stale, the base identity was not checked, and no isolated-base compiler or isolated compare existed. | R1.4 removes stale documents, checks the base `1.1.0` identity, and retains `build/perf/isolated-base-clc`; R1.5/R4.1 require the isolated compare command and its output. |
| PF-011 | 🟠 | The review record count was undefined and could be unsatisfiable after a codegen-changing fix. | R6.1 defines four reviewers per codegen-changing phase; the plan defines the phase set; R6.3 requires a final focused re-review. |
| PF-012 | 🟡 | The probe table mixed a synthetic `table4096` program with the committed `lookup` output. | The probe was re-run on `benchmarks/lookup/program.clo`; the requirements now record 17,025→12,842 and 117,440→92,976 with the matching output. |
| PF-013 | 🟡 | The narrative said “about 40 KB of `.text`” while the measured delta is about 24 KB. | The text now says “about 24 KB of `.text` for the initialization statements”. |
| PF-014 | 🟡 | “An out-of-range/typed constant” conflated an eligible wrapping cast with a compile error. | R3.1 uses `u8(300)`/`i8(200)` as run cases and an untyped out-of-range literal as an `E10202` compile-fail case. |
| PF-015 | 🟡 | The baseline/commit boundary was described as one commit and as a separate commit list. | R1/R4.5 define the order: plan docs, benchmark/tooling, baseline evidence, implementation, fix, closeout. |
| PF-016 | 🟡 | “Unique within the function” did not clearly cover globals or init locals the function references. | R2.2 names the reserved prefix and the symbols the name must avoid; R3.2 pins the collision. |
| PF-017 | 🟡 | The zero-extent, slice, handle/owned, and `const`-array “old stores” cases cannot reach codegen. | R2.4 classifies them as compile-fail/eligibility cases; the “old spelling” suite is limited to reachable shapes. |
| PF-018 | 🟡 | The template value rendering was ambiguous between folded `constant_integer` and typed `store_text`, and `-1` would render badly if folded. | R2.2/R3.2 require the exact `store_text` rendering and pin `-1`, `i8(-128)`, `u8(255)`, `u32(4294967295)`, `usize`, `isize`, and `bool`. |
| PF-019 | 🟡 | The `bool` leaf would be silently excluded by `is_integer_scalar`. | R2.1 uses an explicit scalar list including `bool`; R3.1/R3.2 add bool cases. |
| PF-020 | 🟡 | The benchmark README’s “four scored” prose and baseline-location note would be stale. | R1.3/R5.2 require updating the table, the scored count, and the baseline note. |
| PF-021 | 🟡 | The emission audit omitted `calibration` and did not distinguish the isolated/cumulative diffs. | R4.3 covers all six benchmark programs (five scored plus calibration), the profile per source set, the isolated lookup-only diff, and the two benchmark diff commands. |
| PF-022 | 🟡 | The out-of-scope table was presented as complete but omitted several excluded surfaces. | R2/R4 add the missing rows and state that the Scope Boundary is authoritative and the table is illustrative. |
| PF-023 | 🟡 | “The net instruction delta is larger” was true only for the percentage, not the absolute delta. | The text now says the net percentage delta is larger and the absolute delta is unchanged. |
| PF-024 | 🟡 | The shape-fixture case’s eligibility was unspecified and could silently exercise the old spelling. | R3.6 requires an eligible one-extent explicit-list non-volatile `let` scalar array read by `main`, with the hosted legs exercising the template. |
| PF-025 | 🟡 | `make bench-test` does not run `lookup`; the R3 acceptance could pass without a benchmark run. | R3.7 splits the harness unit tests from a targeted `make bench`/`bench.py run --bench lookup` acceptance. |
| PF-026 | 🟡 | The benchmark could be changed between baseline and head while keeping the same expected output. | R1.6/R4.3 require the benchmark-source diff and SHA-256 record; the only addition is `lookup`. |
| PF-027 | 🟡 | “The base compiler” meant both `v1.1.0` and the pre-change head, and the red run had no reproducible mechanism. | R3.3 defines the pre-change head compiler (`isolated-base-clc`) and the `RDC_CLC` seam; the v1.1.0 compiler is an independent check. |
| PF-028 | 🟡 | No baseline-evidence commit was required before implementation. | R4.5 adds the baseline evidence commit after the baseline run and before the implementation. |
| PF-029 | 🟡 | The roadmap progress count and plan index status were not updated at closeout. | R5.3 requires the PLAN-25 row, the progress count, and the index Status/Last Updated. |
| PF-030 | 🟡 | A 4,096-value initializer line would exceed C99’s guaranteed logical-line limit, and the toolchain legs did not see a wrapped template. | R2.2 requires eight values per line; R3.6 requires a fixture large enough to exercise the wrap on every leg. |
| PF-031 | 🟡 | The freestanding `memcpy` is a byte loop; applying the template to multi-byte freestanding tables is an unmeasured runtime trade-off. | R2.1/R2.4 gate the template to host, hosted-amiga, and hosted-amiga-13; freestanding keeps the stores; R5.1/R5.2 record the limit. |
| PF-032 | 🟡 | The target declaration and `memcpy` position/order were not pinned. | R3.2 adds the target-declaration, position, and order pins; R3.1 adds the later-initializer and address-taken cases. |
| PF-033 | 🟠 | `hosted-amiga-13` was eligible in R2.1 but excluded by R2.4 and omitted from tests/limits (iteration 2). | R2.4 now excludes only freestanding; R2.6/R3.2/R3.6/R4.3/R5.1 and the index/register include `hosted-amiga-13`; R3.2 adds the profile case. |
| PF-034 | 🟡 | The base version check compared `compiler.version` to `1.1.0`, but the harness records `clc 1.1.0`. | R1.4 requires a version line containing `1.1.0`. |
| PF-035 | 🟡 | `build/perf/base-clc` is the v1.1.0 compiler, not the benchmark-commit compiler. | R1.4/R3.3/R4.3 retain and use `build/perf/isolated-base-clc`; `base-clc` is an independent v1.1.0 check only. |
| PF-036 | 🟡 | The table range was written `0..65535`; the actual minimum is 1. | R1.3 and the benchmark docs say “offset by 32768 to 1..65535”. |
| PF-037 | 🟡 | “Byte-identical … except” was self-contradictory, and the 64-byte threshold was target-dependent for `usize`/`isize`. | N3 now says “unchanged apart from”; R2.1 uses 16 elements and 64 bytes under the profile’s type table. |
| PF-038 | 🟡 | The guard-bearing cases were described as normal-output run cases. | R3.1 classifies them as emission/trap cases with the same guard/fault as the base compiler. |
| PF-039 | 🟡 | The accepted limit called the template data “`.rodata`” while `m68k-amigaos-size` counts it in `.text`. | R5.1 says “read-only template data”. |
| PF-040 | 🟡 | R4.3 allowed “an expected prior change” in the isolated diff, where no prior change belongs. | R4.3 now says the isolated diff contains only the `lookup` template; the cumulative diff carries the prior changes. |

## Iteration Record

| Iteration | Scope | Result |
|---|---|---|
| 1 | Five-cluster full scan of revision 1 | 3 critical, 9 major, 26 minor local findings (merged above) |
| 2 | Re-scan of revision 2, scoped to the fix diff | One new major (PF-033, `hosted-amiga-13` consistency); all prior root causes verified fixed; probe numbers verified |
| 3 | Re-scan of revision 3, scoped to PF-033 and its minors | **PASS** — no critical/major; the remaining minor doc notes were fixed in the final revision |

## Scope and Complexity

- Strict scope was used. No optional addition was proposed or kept.
- The requirements add no new crate, build script, harness code, specialist
  agent, runtime-shim change, or language/ABI surface.
- The only material measurement surface is the committed `lookup` benchmark,
  explicitly allowed by the goal’s Scope In 1 and justified as a real
  sine/lookup-table workload (the in-repo `cube` example uses the same shape).
- Auto-design was used only for eligible technical decisions (mechanism,
  eligibility, tests, performance thresholds); product scope and the v1.1.0
  base remain user-owned.

## Final Verdict

**✅ PREFLIGHT PASSED** — all critical/major findings resolved, all minor
notes fixed, no unresolved scope-expansion proposal, and the requirements are
ready for `make-plan`.
