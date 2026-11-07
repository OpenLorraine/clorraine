# Preflight Report: Compiler Robustness Closeout Requirements

> **Status**: ✅ PREFLIGHT PASSED — 36 findings resolved (5 critical, 19 major, 10 minor, 2 observation)
> **Iteration**: 2 (two independent audits, then a resolution pass and re-scan)
> **Artifact**: Requirements set at `plans/compiler-robustness-closeout/`
> **Audit target**: `00-index.md`, `01-requirements.md`,
> `00-ambiguity-register.md`, `02-current-state.md`
> **Context documents**: `AGENTS.md`, `spec/02-types.md`,
> `spec/04-expressions.md`, `spec/09-enums-unions.md`, `spec/17-codegen.md`,
> `compiler/src/codegen/`, `compiler/src/consts/`, `compiler/runtime/`,
> `Makefile`, `docker/scripts/`
> **Codebase Grounded**: 18 source files and 20 probes examined across two
> independent audits
> **Mode**: auto-design active — eligible technical resolutions delegated under
> policy version 1
> **Last Updated**: 2026-11-07

## Same-Session Review

The requirements were created in this session. Two independent `general`
auditors performed the first scan: one on document soundness and codebase
grounding (dimensions 1–6), one on delivery, testability, and fit (dimensions
7–13). Every critical and major finding was reproduced by the author before it
was resolved. The author applied the resolutions and re-scanned the changed
requirements; the same-session bias is recorded here as required.

## Codebase Context Summary

**Tech stack:** one std-only Rust workspace; `clc` emits one C99 translation
unit; `m68k-amigaos-gcc` 16.2.0b, `vamos`, host `cc` (Debian GCC 12.2.0) in the
pinned image; Python 3.11.2.

**Architecture facts verified:**

| Fact | Evidence |
|---|---|
| Pure-fill early return | `compiler/src/codegen/function.rs:4222` |
| Impure-fill guard site | `compiler/src/codegen/function.rs:4238` |
| Module-fill per-element guard | `compiler/src/codegen/program.rs:1842` |
| Binary renderer | `compiler/src/codegen/expr.rs:968-1045` |
| Folder cast | `compiler/src/consts/mod.rs:292` |
| Folder float op | `compiler/src/consts/arithmetic.rs:180` |
| Folder module const | `compiler/src/consts/mod.rs:414` |
| Folder enum member | `compiler/src/consts/mod.rs:456` |
| Enum emitter | `compiler/src/codegen/program.rs:911` |
| Enum cast sema | `compiler/src/sema/call.rs:1685` |
| Enum cast lowering | `compiler/src/own/lower.rs` `ExprKind::Call` branch |
| Host trap | `compiler/runtime/shim_host.c:39-44` (stderr + `abort`) |
| Amiga trap | `compiler/runtime/shim_hosted_amiga.c:543-548` (stdout + `exit(20)`) |
| ASan runner | `docker/scripts/asan-suite.sh` |
| Corpus differential | `docker/scripts/differential-seed.sh` |
| Shape gate | `docker/scripts/bench-shape.sh` |
| Frozen enum rule | `spec/09-enums-unions.md:42-45` |
| Frozen float rule | `spec/04-expressions.md:367-370`, EX-16 |
| m68k NaN caveat | `spec/17-codegen.md:381-384` |

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 6 | 🟠 MAJOR |
| 2 | Implicit assumptions | 5 | 🔴 CRITICAL |
| 3 | Logical contradictions | 7 | 🔴 CRITICAL |
| 4 | Completeness gaps | 7 | 🟠 MAJOR |
| 5 | Dependency issues | 3 | 🟠 MAJOR |
| 6 | Feasibility | 4 | 🟠 MAJOR |
| 7 | Testability | 7 | 🔴 CRITICAL |
| 8 | Security blind spots | 1 | 🟠 MAJOR |
| 9 | Edge cases | 4 | 🟠 MAJOR |
| 10 | Scope creep | 2 | 🟡 MINOR |
| 11 | Ordering & sequencing | 3 | 🟠 MAJOR |
| 12 | Consistency | 4 | 🟠 MAJOR |
| 13 | Codebase alignment | 3 | 🟠 MAJOR |

## Summary by Severity

| Severity | Count | Status |
|---|---:|---|
| CRITICAL | 5 | All resolved |
| MAJOR | 19 | All resolved |
| MINOR | 10 | All resolved |
| OBSERVATION | 2 | All resolved |

## Findings and Resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-REQ-001 | 🔴 CRITICAL | Enum auto-increment `256` cannot satisfy R5.2–R5.4 and valid C99: a `u8`-backed enum cannot store 256, and the spec's `let x: E = E.B` fails `-Werror=overflow` | **User ruling 2026-11-07: reject with E10202.** A member value must fit the backing type; `enum E: u8 { A = 255, B }` reports E10202 at `B` with the registered message/help. No new diagnostic. R5.2–R5.5, the PLAN-14 enum pins, and the seeds are updated to the rejection and in-range controls; AR-11 records the ruling |
| PF-REQ-002 | 🔴 CRITICAL | Host and m68k traps differ (stderr + SIGABRT vs stdout + exit 20), so the fuzzer would call a correct trap a divergence | R7.8 defines fault normalization: compare the extracted `error CODE:` line and a fault/normal class; do not compare raw exit status across profiles |
| PF-REQ-003 | 🔴 CRITICAL | A host/target differential cannot detect the five shared wrong-value defects; the seeds had no golden | R7.7 gives every committed seed `expected.txt` or `expected-fault.txt`; a mismatch is a `golden-mismatch`. AR-19 states the differential finds divergence and the seed goldens catch shared wrong values; the spec/impl suites remain the defect oracle |
| PF-REQ-004 | 🔴 CRITICAL | Constant float-to-integer NaN/out-of-range folds to 0 or saturation; EX-16 requires E10202 | R4.4 and AR-27 require E10202 in every constant context; the folder gets a range/NaN error, and enum/extent/const negatives are added |
| PF-REQ-005 | 🔴 CRITICAL | The UBSan sweep targeted `spec/tests/run` (six intentional-fault programs), not a normal corpus | R6.4 sweeps the three portable `spec/tests/corpus` programs; the six fault programs stay the frozen fault oracle |
| PF-REQ-006 | 🟠 MAJOR | The pure-fill fix omitted `E11212` float-to-integer guards | R1.1 and R1.4 add the `E11212` class and a red fill case |
| PF-REQ-007 | 🟠 MAJOR | R1.2 was false in both directions: struct/conditional paths emitted zero guards and the module path emitted N+1 | R1.2 is a path matrix; R1.5 counts `cl_trap` per path. AR-3 covers the module deduplication |
| PF-REQ-008 | 🟠 MAJOR | R7.4 said every program runs on m68k while R7.6 said a subset | R7.4 and R7.6 now state the exact matrix: host all, sanitizers on a fixed subset, m68k on a fixed subset |
| PF-REQ-009 | 🟠 MAJOR | No seed counts, matrix sizes, or gate budget were quantified | R7.10 fixes the smoke counts (`all seeds`, generated 0–15, sanitizer 0–3, m68k 0–3) and the 60 s budget; `make fuzz` is 0–255 with every eighth seed on m68k |
| PF-REQ-010 | 🟠 MAJOR | The fuzzer's sanitizer builds omitted the SDK image's `-no-pie` workaround | R7.5 makes `-no-pie` mandatory; R7.4 pins `ASAN_OPTIONS` and `UBSAN_OPTIONS` |
| PF-REQ-011 | 🟠 MAJOR | “Compiles” did not pin the strict flags, so D4's `-Werror=overflow` would be invisible | R7.4 pins the host, m68k, and sanitizer flags; a warning is `invalid-c` |
| PF-REQ-012 | 🟠 MAJOR | `rejected` mislabelled a grammar-valid compiler rejection as a generator failure | R7.9 splits `compiler-rejected` and `internal` from `generator`-side failures; R7.3 makes the generator a typed template grammar |
| PF-REQ-013 | 🟠 MAJOR | Deterministic non-terminating programs could block the smoke gate; timeouts conflicted with “no wall-clock thresholds” | R7.3 forbids traps and requires bounded loops; R7.9/R7.10 state timeouts only bound a hang and the matrix is fixed. `hang` is a class, not a performance threshold |
| PF-REQ-014 | 🟠 MAJOR | R8.3 named three pins; only one actually changed under the old 256 decision | Under the new wrap decision exactly two change: `narrow: 300 300 → 44 44` and `enum: 256 256 → 0 0`. The narrow-const place pin stays. R8.3 is a table |
| PF-REQ-015 | 🟠 MAJOR | R9.2 conflated the PLAN-14 base `0ac87d3e` with this plan's base | R9.2 uses `96ae0d4` (code-identical to `02877f2`) and names `0ac87d3e` only for D3's red evidence. AR-32 records both |
| PF-REQ-016 | 🟠 MAJOR | R4.3 had no reproduction and its `fold_module_const` site was missing | `02-current-state.md` D5B records the `const A: f32 = 16777217.0` case; R4.3 and the design name `fold_module_const` |
| PF-REQ-017 | 🟠 MAJOR | R4.6 claimed m68k IEEE NaN agreement, contradicting `spec/17-codegen.md` | R4.7 scopes the claim to finite values, `-0.0`, infinities, and the existing `cl_isnan`/conversion checks; NaN ordering keeps the recorded m68k divergence |
| PF-REQ-018 | 🟠 MAJOR | `Direction(2)` is emitted as a C function call, so R5.3's “like the explicit cast” premise was untestable | R5.6 and AR-26 add the enum-conversion lowering fix. The spec example compiles; a spec case and an impl pin cover it |
| PF-REQ-019 | 🟠 MAJOR | Three new Rust files and four Makefile targets were undercounted in the index | The index complexity section is corrected to three suites and four targets; R7.10 and R8.1 name them |
| PF-REQ-020 | 🟠 MAJOR | The fuzzer's security allowlist and working directory were unspecified | R7.11 and AR-29 allowlist the generated constructs (no `embed`, `ffi`, `unsafe`, raw, file I/O, traps) and require a private working directory |
| PF-REQ-021 | 🟠 MAJOR | The UBSan robustness suite did not opt into ASan, so R1.3's memory claim was untestable | R8.2 requires the robustness suite to carry both `RDC_ASAN` and `RDC_UBSAN`; R1.4 has the red ASan case |
| PF-REQ-022 | 🟠 MAJOR | The new 16-bit multiply/shift UB cases were missing | R2.4 adds `u8*u8`, `u16*u16`, `u16<<15`, and `i16*i16`; R2.2 computes in `unsigned int` so the fix is defined |
| PF-REQ-023 | 🟠 MAJOR | `bench-shape` did not cover the new shapes and had no `nix13` leg | R8.4 extends `tools/perf/fixtures/shapes.clo` and adds a `-mcrt=nix13` build leg |
| PF-REQ-024 | 🟠 MAJOR | The fuzzer's `f32` rounding test could not distinguish per-operation rounding from cast-only rounding | R4.2 requires width-carrying folds; the design's cancellation case (`(16777216.0 + 1.0) - 16777216.0` is 0 at f32, 1 at f64) distinguishes them |
| PF-REQ-025 | 🟠 MAJOR | The UBSan runner had no activation control; a suite that silently skipped the flags would pass | R6.3 adds a known-UB probe self-test; R6.1 fails when fewer than two suites are discovered |
| PF-REQ-026 | 🟠 MAJOR | The D3 exception was not stated in R8.1 | R8.1 names D3 as the exception with pre-PLAN-14 red evidence |
| PF-REQ-027 | 🟠 MAJOR | The fault contract was tested only on the host, not on m68k | R9.3 adds committed fill-fault probes under `examples/robustness/` and extends `run-fault-amiga.sh` |
| PF-REQ-028 | 🟠 MAJOR | The count of suites/targets and `make fuzz` placement were inconsistent | R7.10 puts `fuzz-test` and `fuzz-smoke` in `make check`; `make fuzz` stays local; R9.1 records the gates; the index count is fixed |
| PF-REQ-029 | 🟡 MINOR | R2.1's conditional operator scope was circular | R2.3 names the fixed operators and the agreement-tested set |
| PF-REQ-030 | 🟡 MINOR | R3.2 named “the commit that introduced the failure” | R3.3 names `0ac87d3e` (traps) and the head (prints 0); the positions are enumerated |
| PF-REQ-031 | 🟡 MINOR | R2.3 covered only the module const strict build | R2.5 adds local, fill, and struct-field strict builds |
| PF-REQ-032 | 🟡 MINOR | Minimum suite and seed counts were reported but not enforced | R6.1 requires ≥2 UBSan suites; R7.7/R7.10 fix the seed set and a unit test asserts it is non-empty |
| PF-REQ-033 | 🟡 MINOR | Failure artifacts and seed ordering were underspecified | R7.2 sorts seeds by filename and uses SHA-256 decisions; R7.9 makes emitted C optional for rejects/internal failures |
| PF-REQ-034 | 🟡 MINOR | The determinism claim was not enforceable across Python versions | R7.2 replaces `random` with a SHA-256 decision stream; a unit test compares two processes byte for byte |
| PF-REQ-035 | 🟡 MINOR | The scope boundary said m68k output changed only for narrow arithmetic and enums, omitting R1/R4 | `00-index.md` scope now lists fill guards, float-derived constant values, and enum values |
| PF-REQ-036 | 🔵 OBS | The index listed documents that did not exist yet; the specialist check was stale; the baseline was one docs commit behind | The design, plan, and testing documents were written before this report was finalized; R8.5 reruns the specialist check; R9.2 uses `96ae0d4` |

## Re-scan

After the resolutions, the author re-read every changed requirement against the
code and the frozen spec. The following checks were repeated:

| Check | Result |
|---|---|
| Enum overflow rule is the user ruling | AR-11 records the explicit user ruling; a compile-fail case asserts E10202 |
| Fuzzer trap normalization covers host and Amiga | `shim_host.c`, `shim_hosted_amiga.c`, `run-fault-amiga.sh` read |
| Float-to-int range rule matches EX-16 | `spec/04-expressions.md:367-370` read |
| UBSan corpus is a normal-exit corpus | three portable programs named |
| Strict flags match the repository | `differential-seed.sh`, `bench-shape.sh`, test helpers read |
| ASan/UBSan opt-in both present | R8.2 |
| Base identity is unambiguous | `96ae0d4` for the audit, `0ac87d3e` for D3 |
| No frozen `spec/` edit | R2.6, R5.3, R8.3 |

## Verdict

**PASSED.** Every critical and major finding is resolved in the requirements,
the ambiguity register, and the current-state study. Implementation may proceed
to the plan preflight.
