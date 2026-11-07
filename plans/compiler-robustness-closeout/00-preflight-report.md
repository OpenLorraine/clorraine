# Preflight Report: Compiler Robustness Closeout Plan

> **Status**: ✅ PREFLIGHT PASSED — 45 findings resolved (1 critical, 24 major, 16 minor, 4 observation)
> **Iteration**: 2 (two independent audits, then a resolution pass and re-scan)
> **Artifact**: Plan set at `plans/compiler-robustness-closeout/`
> **Audit target**: `00-index.md`, `01-requirements.md`,
> `00-ambiguity-register.md`, `02-current-state.md`,
> `03-01-defect-fixes.md`, `03-02-verification-infrastructure.md`,
> `07-testing-strategy.md`, `99-execution-plan.md`
> **Context documents**: `AGENTS.md`, the compiler source cited by the design,
> `Makefile`, `docker/scripts/`, `tools/perf/`, the test suites
> **Codebase Grounded**: 22 source and script files plus 30 probes across two
> independent audits
> **Mode**: auto-design active + one user ruling on the enum overflow rule
> **Last Updated**: 2026-11-07

## Same-Session Review

Two independent `general` auditors performed the first plan scan: one on
feasibility and codebase alignment (dimensions 1–6), one on testability and
delivery (dimensions 7–13). The author reproduced every critical and major
finding before resolving it. The enum overflow decision was escalated to the
user, who ruled E10202. The author then applied the resolutions and re-scanned
the changed documents; the same-session bias is recorded here.

## Summary by Severity

| Severity | Count | Status |
|---|---:|---|
| CRITICAL | 1 | Resolved by user ruling |
| MAJOR | 24 | All resolved |
| MINOR | 16 | All resolved |
| OBSERVATION | 4 | All resolved |

## Critical Finding

### PF-PLAN-001 — Enum overflow rule needs the user

Both auditors independently identified that the plan's first enum resolution
(backing-type wrap) changed a frozen-spec construct's value and was not
delegable. The user ruled **reject with E10202**. The plan now uses the
existing code, message, and help, updates the PLAN-14 enum pins to an in-range
control, adds a compile-fail case, and records the ruling in AR-11. No wrap is
implemented.

## Findings and Resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-PLAN-001 | 🔴 CRITICAL | Enum auto-increment needs a language decision; wrap, 256, and E10202 were all viable | User ruled E10202; AR-11, R5.2–R5.5, the pins, and the seeds updated |
| PF-PLAN-002 | 🟠 MAJOR | The E10202 message text in the design was not the registered message | R4.4, AR-27, and D5 use `literal does not fit '{type}'` and the registered help; `ConstError` carries the target scalar |
| PF-PLAN-003 | 🟠 MAJOR | The module guard-dedup design relied on `init_walked_guards`, which is false on the module path; prefix and impure branches still duplicated | R1.3/AR-3 and D1 use a dedicated `init_fill_guards_emitted` flag, cover prefix elements, gate the impure branch, and add module conditional/prefix cases |
| PF-PLAN-004 | 🟠 MAJOR | A conditional pure fill repeats per element, so “exactly once” was false | R1.2 scopes the once rule to non-conditional operands and requires a guard at each selected arm site; the condition stays once |
| PF-PLAN-005 | 🟠 MAJOR | A whole-unit `cl_trap` count includes the shim | R1.6 counts only inside the generated function/init body and includes a zero control |
| PF-PLAN-006 | 🟠 MAJOR | The `fill-shift` impl pin changes with the shift wrap but was not listed | R8.3, the testing strategy, and task 2.1.2 include the pin update |
| PF-PLAN-007 | 🟠 MAJOR | The D2 design changed 32-bit arithmetic and used `uint32_t`, which the freestanding profile lacks | R2.2 limits the new intermediate to results narrower than 32 bits, keeps the existing 32-bit spelling, and uses `unsigned int`/`u32`; a 32-bit control and a freestanding shape check are added |
| PF-PLAN-008 | 🟠 MAJOR | The runner self-test proves the runner, not the suites' child builds | R6.7 adds suite-level known-ASan and known-UBSan probes through the suite's own flag helper |
| PF-PLAN-009 | 🟠 MAJOR | The UBSan activation probe exits 1 under `-fno-sanitize-recover=all`, so `set -e` would fail it | R6.3 captures the status with `set +e`, asserts the report text, and treats 1/134 as expected |
| PF-PLAN-010 | 🟠 MAJOR | A correct host fault seed (SIGABRT) would be classified `crash` before its golden | R7.8/AR-28 puts the `expected-fault.txt` check first; only a missing/mismatched line is `golden-mismatch` |
| PF-PLAN-011 | 🟠 MAJOR | Same-process determinism does not prove cross-process determinism | R7.2 and the unit test run two subprocesses with different `PYTHONHASHSEED` values |
| PF-PLAN-012 | 🟠 MAJOR | `struct_fill` as a normal seed does not exercise the missing guard | AR-44/R7.7 make it a fault seed with an out-of-range index; `struct_value` covers the normal path |
| PF-PLAN-013 | 🟠 MAJOR | The base-compiler paths collide with `bench-base` and the D3 binary | AR-32/R9.2 copy `0ac87d3e` first, build `96ae0d4` once in 0.1.3 at `build/robustness/`, and reuse both |
| PF-PLAN-014 | 🟠 MAJOR | `run-fault-amiga.sh` builds every source under `spec/tests/run`; the new probes live elsewhere | R9.3/AR-35 adds a per-case path or a second `PROBES` loop and reports eight cases |
| PF-PLAN-015 | 🟠 MAJOR | “`verify` runs `ubsan` after `asan`” was false; `verify` has no `asan` | R6.5/03-02 place `ubsan` after `check`/`bench-shape`; `asan` stays a separate gate |
| PF-PLAN-016 | 🟠 MAJOR | Independent reviews ran only after the full gate, leaving stale evidence | AR-40 and tasks 1.1.5, 2.1.5, 4.1.4, 6.1.5, 7.1.6 dispatch per phase; R9.6 re-gates after final review fixes |
| PF-PLAN-017 | 🟠 MAJOR | The enum/handle conversion is a seventh fix outside the six and was absent from the index baseline | The index minimum baseline and scope now list type-name conversion emission; the final report labels it a spec-conformance fix with its evidence |
| PF-PLAN-018 | 🟠 MAJOR | Two current-state D2 rows were wrong (`i16` positive multiply, `u16 << 15`) | `02-current-state.md` marks `i16 * i16` as a control, records `u16 << 15` as 2147450880, and only `u16 * u16` as undefined |
| PF-PLAN-019 | 🟠 MAJOR | Several requirement rows had no named test | The testing strategy's requirement-coverage table names a case for every R1–R9 row |
| PF-PLAN-020 | 🟠 MAJOR | The gate matrix credited `bench-shape` with a host leg it does not have | The matrix now lists GCC `nix20`, GCC `nix13`, vbcc, and freestanding; the host strict build is credited to the spec suites |
| PF-PLAN-021 | 🟠 MAJOR | R8.4 required a narrow-fill shape the fixture did not have | 03-02 §3.1 adds `let nf: u8[4] = [; u8(200) + u8(100)];` and uses it |
| PF-PLAN-022 | 🟡 MINOR | The progress count and task sum disagreed; red-first tasks could look committed red | The plan is 36 tasks; red-first tests are committed with their fix, and every commit is green |
| PF-PLAN-023 | 🟡 MINOR | Golden file names differed between requirements and design | Both use `<name>.expected.txt` / `<name>.expected-fault.txt` |
| PF-PLAN-024 | 🟡 MINOR | `fuzz-test` could silently skip template acceptance without `clc` | The target builds the workspace first; the test fails if the binary is absent |
| PF-PLAN-025 | 🟡 MINOR | Seeds were not scanned against the generator allowlist | R7.3/R7.11 add a unit test that scans the seeds |
| PF-PLAN-026 | 🟡 MINOR | Fault seeds could leave core files | R7.5/R7.11 and the UBSan runner disable core dumps and assert no core file |
| PF-PLAN-027 | 🟡 MINOR | The `verify` row claimed full green even when licensed media is blocked | The gate matrix and R9.5 require naming every blocked leg |
| PF-PLAN-028 | 🟡 MINOR | The ASan suite count would be 27 or 28 depending on the impl suite marker | The spec suite alone opts in; the count is 27 |
| PF-PLAN-029 | 🟡 MINOR | Baseline counts (26 ASan suites, 66 plan folders) become stale | R8.5 and the findings update 27 and 67; the current-state doc notes the 26→27 change |
| PF-PLAN-030 | 🔵 OBS | The UBSan corpus did not name the three portable programs | R6.4/03-02 name `buffer-container`, `file-copier`, and `string-parser` |
| PF-PLAN-031 | 🔵 OBS | The shape fixture's `pick` cannot take a fixed array and was implied to be called | 03-02 §3.1 states it is uncalled and kept by the emitter's `sizeof` use |
| PF-PLAN-032 | 🟠 MAJOR | The emitter reimplemented the folder's enum walk; `wrap_int`/`FoldValue` are private | R5/03-01 add `ConstFolder::enum_member_values`; both the checker and emitter use it |
| PF-PLAN-033 | 🔵 OBS | The enum map is keyed by bare name; cross-module same-name enums collide | AR-48 scopes R5.7 to a unique cross-module enum and records the pre-existing collision rule |
| PF-PLAN-034 | 🟡 MINOR | The module dedup mechanism was left as “flag or reuse” | 03-01 pins the dedicated `init_fill_guards_emitted` flag and its sites |
| PF-PLAN-035 | 🔵 OBS | The enum conversion test used only a literal; a literal-only special case could pass | The test set adds a runtime argument and a shadowing binding |
| PF-PLAN-036 | 🟡 MINOR | The scope boundary omitted R1/R4/R5.6 emission changes | `00-index.md` lists fill guards, float-derived values, enum values/conversions, and the E10202 trigger |
| PF-PLAN-037 | 🟡 MINOR | The smoke budget included `cargo build` and had no fallback target | R7.10 defines the 60 s budget after the build and adds `make fuzz-smoke-host` |
| PF-PLAN-038 | 🟡 MINOR | The golden and extracted fault line were not explicitly normalized | R7.7/03-02 normalize both and add a CRLF fault-line unit test |
| PF-PLAN-039 | 🟡 MINOR | The classifier had no harness class | R7.9/AR-45 add `harness` with exit 2 |
| PF-PLAN-040 | 🟡 MINOR | The D3 test cannot run against the base binary through `CARGO_BIN_EXE_clc` | R3.3/AR-46 add a `RDC_CLC` override to the spec helper |
| PF-PLAN-041 | 🟠 MAJOR | Handle conversions (`Buffer(0)`) have the same call-emission bug | R5.5/AR-37 extend the type-name lowering to handles and add a case |
| PF-PLAN-042 | 🟠 MAJOR | The sema fit check only covers explicit enum values, so the auto-increment rule was unimplemented | R5.2/03-01 walk members with a running value and apply `int_fits` to both |
| PF-PLAN-043 | 🟡 MINOR | `E(2)` still does not fold in a constant context, so the fuzzer's enum-extent family could generate rejects | R5.6/AR-36 add the folder case for an enum-type call |
| PF-PLAN-044 | 🟡 MINOR | The task count was 31 while the header said 29 | The plan is 36 tasks across eight phases; the header and phase table agree |
| PF-PLAN-045 | 🟠 MAJOR | A module-level conditional struct arm could still reach the store with the guard flag true | R1.3/AR-39 set the flag false for arm stores and add the module conditional case |

## Re-scan

| Check | Result |
|---|---|
| D1 flag and every path | `function.rs:4222`, `:4238`, `program.rs:1743-1846` re-read; dedicated flag defined |
| D2 intermediate on host and m68k | `unsigned int` is 32-bit on both; no `uint32_t` in the design |
| D5 error payload | `ConstError` can carry `ScalarType`; `report` formats the registered message |
| D6 sema and folder walk | `sema/items.rs` and `consts/mod.rs` share `enum_member_values`; the emitter reuses it |
| Enum conversion shadow safety | `lower_call`'s binding checks are named; a shadowing test is added |
| UBSan runner activation | GCC 12 reports the probe with exit 1; the script captures it |
| Fuzzer determinism and safety | SHA-256 decisions, two-process test, seed scan, core dumps disabled |
| Review ordering | Per-phase review tasks and a final re-gate |
| Task count | 36 |
| Scope honesty | type-name conversion, fill guards, float-derived values, and E10202 documented |

## Fix Verification (Iteration 2)

A focused re-scan verified the resolutions above and found four partial
resolutions plus ten minor inconsistencies. They are resolved here.

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-FIX-001 | 🟠 MAJOR | The D6 design still described a per-member emitter fold beside the shared walk | 03-01 D6 now makes `enum_member_values` the sole value source for the checker, folder, and emitter; the emitter no longer re-folds or keeps a counter |
| PF-FIX-002 | 🟠 MAJOR | A statement pure conditional fill repeats its condition per element, so the “condition once” claim was false | R1.2/D1 no longer claim condition-once for the statement path; the condition guard is emitted once and the repetition is recorded as a pre-existing evaluation-order limitation in `docs/known-limitations.md` |
| PF-FIX-003 | 🟠 MAJOR | The execution plan had no D3 task | Phase 2 gained Step 2.2 (D3 tests + base evidence) and the review moved to 2.3.1; the plan is 38 tasks |
| PF-FIX-004 | 🟠 MAJOR | The index named only the enum E10202 trigger, omitting the float-to-integer EX-16 trigger | The index baseline and scope list both E10202 triggers |
| PF-FIX-005 | 🟡 MINOR | The scalar conditional arm is already guarded, so the case was not red | The case is now a control; the module conditional struct case is the red conditional case |
| PF-FIX-006 | 🟡 MINOR | R5.4 said `E.A` but quoted 6/1 (`E.B`) | R5.4 now says folder 5 / emitter 0 → 5 for `E.A` |
| PF-FIX-007 | 🟡 MINOR | R8.3 named both the 32-bit shift pin and `fill-shift`; only the latter changes | R8.3 names only `should_keep_a_shifting_fill_on_the_unrolled_path`; the 32-bit pin is unchanged |
| PF-FIX-008 | 🟡 MINOR | The preflight count was 42 but the table has 45 rows | Status and summary now read 1/24/16/4 = 45 |
| PF-FIX-009 | 🟡 MINOR | Progress said 0/36 while 0.1.1 was checked | Progress is 1/38 |
| PF-FIX-010 | 🟡 MINOR | No runtime-argument enum conversion case | The D6 and testing tables add `should_convert_a_runtime_value_to_an_enum` |
| PF-FIX-011 | 🟡 MINOR | The module dedup wording read as zero prefix guards | 03-01 states one fill guard plus one per prefix element, no duplicates |
| PF-FIX-012 | 🟡 MINOR | The new fill-guard flag had no save/restore rule | 03-01 requires save/restore around every walk and adds a mixed module+statement count case |
| PF-FIX-013 | 🟡 MINOR | The smoke-host fallback duplicated host work in verify | 03-02 states `make check` uses `fuzz-smoke-host`; `make verify` adds only the m68k subset |
| PF-FIX-014 | 🔵 OBS | AR-11 cited the float-overflow line for the general fit rule | AR-11 cites `spec/04-expressions.md:372` (and :306) |

## Verdict

**PASSED.** Every critical and major finding from both audit iterations is
resolved. The plan may proceed to task 0.1.2.
