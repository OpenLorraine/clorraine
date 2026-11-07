# Ambiguity Register: Compiler Robustness Closeout (PLAN-18)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design active — eligible technical decisions delegated under
> policy version 1; one language-surface decision was ruled by the user
> **Root invocation ID**: `codeops-robustness-20261107`
> **Policy version**: 1

## Register

| ID | Category | Question | Decision | Status / evidence |
|---|---|---|---|---|
| AR-1 | Scope | What does this plan deliver? | The six recorded defect closeouts, a UBSan host leg, and a deterministic differential fuzzer. No grammar, ABI, runtime-shim, stdlib, or golden change. One existing diagnostic (E10202) gains a new trigger by user ruling. | ✅ Resolved — user objective + AR-11; R1–R9 |
| AR-2 | Authority | Which decisions need a user ruling? | Any language, grammar, diagnostic, ABI, or frozen-spec change. The enum overflow rule was ruled by the user (AR-11). Every other fix makes the compiler match an existing rule. | ✅ Resolved — user objective; AR-11 |
| AR-3 | D1 fix site | Where is the missing fill guard emitted? | Once in the pure-fill lowering before the first store. A dedicated `init_fill_guards_emitted` flag set by the module preparation tells the store core that the fill operand was already guarded; the flag covers prefix elements too. The impure branch is gated by the same flag so it does not duplicate. | ✅ Resolved — AI delegated; `function.rs:4222`, `program.rs:1743-1846`; R1 |
| AR-4 | D1 classes | Which guard kinds are in scope? | Bounds (`E11211`), integer division/remainder (`E11210`), and float-to-integer range/NaN (`E11212`). The same early return dropped all of them. | ✅ Resolved — AI delegated; R1.1–R1.6 |
| AR-5 | D2 semantics | Which operations wrap at their result type? | `+`, `-`, `*`, and `<<` are fixed in the renderer. `>>`, `/`, `%`, and bitwise operations are pinned by constant-context agreement cases; a pinned difference is a defect and is fixed the same way. | ✅ Resolved — AI delegated; R2 |
| AR-6 | D2 intermediate type | How is wrapping made safe in C? | For a result narrower than 32 bits, compute in `unsigned int` and cast to the result type. A 32-bit result keeps the existing spelling (unsigned arithmetic is defined and the signed path already casts through `uint32`). Generated C uses `unsigned int`/the emitted `u32` typedef, never `uint32_t`. | ✅ Resolved — AI delegated; R2.2, N1 |
| AR-7 | D2 storage | Are module `const`s materialized as literals now? | No. The const stays a place; the initializer expression wraps. | ✅ Resolved — AI delegated; R2.5, R2.6 |
| AR-8 | D3 status | Is the inline shift divisor still broken? | No. The pre-PLAN-14 base (`0ac87d3e`) traps; the starting head prints `0`. This plan adds the regression test and records the base/head evidence; no new code change is claimed. The test helper accepts `RDC_CLC` so the base run is reproducible. | ✅ Resolved — verified with `build/perf/base-clc`; R3.3 |
| AR-9 | D5 scope | Does the `f32` fix materialize float constants? | No. Rounding is fixed at casts, float arithmetic, and declared `f32` const references. Floats stay places. | ✅ Resolved — AI delegated; R4.6 |
| AR-10 | D5 precision | How is `f32` rounding implemented in Rust? | Round through `f64 -> f32 -> f64` (`value as f32 as f64`). Rust uses IEEE-754 round-to-nearest-even, matching host and m68k binary32. | ✅ Resolved — AI delegated; R4.1–R4.7 |
| AR-11 | D6 auto-increment | What happens when an auto-incremented member exceeds the backing type? | **User ruling: reject with E10202.** A member value is representable in the backing type. `enum E: u8 { A = 255, B }` reports E10202 at `B`, using the registered message `literal does not fit 'u8'` and the existing help. The same rule already applies to explicit values. No new code, message, or help text. | ✅ Resolved — **user ruling 2026-11-07**; R5.2; the folder stops wrapping in the sense that the declaration is rejected; the emitter never sees an out-of-range member |
| AR-12 | D6 explicit values | Does the emitter fold explicit expressions, or only the folder? | Both use one `ConstFolder` walk. The emitter folds explicit values with the declaring module's folder and backing type. This also fixes explicit expressions that were emitted as `0`. | ✅ Resolved — AI delegated; R5.1; spec EN-3 already requires constant expressions |
| AR-13 | D6 ownership | Where does the emitter get the declaring module? | `EnumInfo` gains the module name at collection time. The emitter only reads the folder's value; it does not report diagnostics (the checker already did). | ✅ Resolved — AI delegated; `03-01-defect-fixes.md` |
| AR-14 | UBSan runner | Copy the ASan runner or generalize it? | Add a separate `docker/scripts/ubsan-suite.sh` with the same discovery, per-suite process, timeout, `RDC_UBSAN=1` export, minimum suite count, and `-no-pie` wrapper. | ✅ Resolved — AI delegated; R6.1–R6.3 |
| AR-15 | UBSan coverage | Which suites opt in, and what is swept? | The new robustness spec suite (R1–R5) and a UBSan sweep over `buffer-container`, `file-copier`, and `string-parser`. The six `spec/tests/run` fault programs stay the frozen fault oracle. | ✅ Resolved — AI delegated; R6.4 |
| AR-16 | UBSan in check | Is `make ubsan` part of `make check`? | No. It is verify-only, placed after `check`/`bench-shape`; `make asan` stays separate. The measured runtime is recorded. | ✅ Resolved — user objective "keep make check fast"; R6.5 |
| AR-17 | Fuzzer technology | Python, Rust, or a third-party fuzzer? | Python 3 standard library under `tools/fuzz/`, with package files matching `tools/perf`. No external dependency, no build script, no cargo-fuzz target. | ✅ Resolved — user objective names Python; R7.1, R7.12 |
| AR-18 | Fuzzer model | Coverage-guided or template grammar? | A deterministic typed template grammar distilled from the EBNF and corpus patterns, with committed seeds. Every template family is validated as accepted before the smoke gate. | ✅ Resolved — AI delegated; R7.2, R7.3 |
| AR-19 | Fuzzer oracle | Does the differential alone find the six defects? | No. The differential finds host/target divergence; the committed seeds have goldens and catch shared wrong values. The spec/impl suites remain the defect oracle. | ✅ Resolved — AI delegated; R7.7, R7.9 |
| AR-20 | Fuzzer gates | What runs in the default gate? | `make fuzz-test` and `make fuzz-smoke` (all twelve seeds plus generated `0..=15`, with fixed sanitizer and m68k subsets). `make fuzz` is the longer local target; `make fuzz-smoke-host` is the budget fallback. | ✅ Resolved — AI delegated; R7.10 |
| AR-21 | Fuzzer timeouts | Are wall-clock limits a pass criterion? | No. Timeouts only bound a hang; all seed counts and matrix sizes are fixed integers. | ✅ Resolved — user objective "no flaky wall-clock thresholds"; R7.9, R7.13 |
| AR-22 | Historical pins | May the PLAN-14 test expectations change? | Yes: the narrow value becomes 44; the enum value becomes 255 with an in-range `A = 254, B`; the enum place pin uses the in-range enum; the shift/fill impl pin gains the wrap cast. The narrow-const place pin stays. No `spec/` golden is touched. | ✅ Resolved — AI delegated; R8.3 |
| AR-23 | Commit mode | How is the work landed? | `--auto-commit` after each verified task or coherent fix; documentation commits separate from implementation; push only when the full gate is green. | ✅ Resolved — user objective |
| AR-24 | Licensed media | What if a FS-UAE leg cannot run? | Record the exact blocker and report partial verification; do not claim `verify: all legs passed`. | ✅ Resolved — user objective; R9.5 |
| AR-25 | Completed plan docs | Do the plan documents count as source comments? | They may name plans and requirements. The ban applies to source code and doc comments. | ✅ Resolved — repository standard; R8.6 |
| AR-26 | D6 conversion | Is `Direction(2)` emitted as a C function call? | Yes at the starting head; the spec's own example fails the strict C build. The plan fixes the lowering to a scalar cast (R5.5), because the conversion semantics need the spec form to work. This is codegen conformance, not a language change. | ✅ Resolved — verified with `enum_cast.clo`; `spec/09-enums-unions.md:38` |
| AR-27 | D5 overflow | What happens to a constant float-to-integer NaN or out-of-range conversion? | E10202, using the registered message `literal does not fit '{type}'` and help, with the target type carried in `ConstError`. This is an existing code with a new trigger, matching EX-16. | ✅ Resolved — verified with `enum_nan.clo`; `spec/04-expressions.md:367-370`, `spec/14-diagnostics.md:88`; R4.4 |
| AR-28 | Fuzzer differential traps | How are correct runtime faults compared across profiles? | A seed with `expected-fault.txt` is classified first: the normalized `error CODE:` line must match, and the signal/nonzero status is expected. Only a missing/mismatched line is `golden-mismatch`. | ✅ Resolved — AI delegated; R7.8 |
| AR-29 | Fuzzer safety | Which constructs may the generator emit? | A typed allowlist: integer and narrow arithmetic, shifts, consts, enums, arrays and fills, structs with complete literals, bounded loops, and `std.io.println` text. No `embed`, `ffi`, `unsafe`, raw pointer, file I/O, `assert`, or trap. Programs run in a private working directory with core dumps disabled; the seeds are scanned for the same list. | ✅ Resolved — AI delegated; R7.3, R7.11 |
| AR-30 | Fuzzer flags | Which C flags define a divergence? | Host: `-std=c99 -O0 -g -Wall -Wextra -Werror`. m68k: `-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror`. A warning is `invalid-c`. Sanitizers add `-fsanitize=address,undefined -fno-sanitize-recover=all -no-pie`. | ✅ Resolved — AI delegated; R7.4, R7.5 |
| AR-31 | Gate budget | What is the added fast-gate budget? | The smoke fuzzer process (after `cargo build`) is ≤ 60 seconds on the reference container. If exceeded, use `fuzz-smoke-host` (host + sanitizer only) and move m68k to `verify`. Measure `fuzz-test` and `fuzz-smoke` separately and record both. | ✅ Resolved — AI delegated; R7.10, N6 |
| AR-32 | Emission base | Which commit is the emission-audit base? | `96ae0d4` (code-identical to the starting head `02877f2`). Build it once in 0.1.3 at `build/robustness/base-96ae0d4-clc`; copy the pre-PLAN-14 `0ac87d3e` binary to `build/robustness/base-0ac87d3e-clc` first. Reuse both; do not overwrite. | ✅ Resolved — AI delegated; R9.2 |
| AR-33 | Specialist record | Is the specialist check stale? | Yes: `codeops/specialist-check.json` has no entry for this plan. Rerun the project-level specialist check and record the result; do not run the installer from inside the repository. | ✅ Resolved — repository standard; R8.5 |
| AR-34 | Cross-toolchain gate | How are the new shapes proven on every build leg? | Extend `tools/perf/fixtures/shapes.clo` and add a `-mcrt=nix13` build leg to `bench-shape.sh`, keeping the GCC `nix20`, vbcc, and freestanding legs. | ✅ Resolved — AI delegated; R8.4 |
| AR-35 | Fault parity | How is the fill fault proven on m68k? | Add committed probes under `examples/robustness/` and extend `run-fault-amiga.sh` with a per-case path or a second loop; keep the six frozen `spec/tests/run` cases. Report eight cases. | ✅ Resolved — AI delegated; R9.3 |
| AR-36 | Enum constant cast | Does `E(2)` fold in a constant context? | No. The frozen constant-expression list names scalar casts and integer-to-raw casts, not enum casts; the first attempt was an unauthorized extension and was removed. `E(2)` in an extent or `case` stays E10224. | ✅ Resolved — rejected after the D6 semantics review; R5.6 |
| AR-37 | Handle conversions | Does the same lowering bug affect handle casts? | A handle type import is not recognized as a cast by the current sema resolver, so `Buffer(0)` is a pre-existing limitation recorded in `docs/known-limitations.md`. The lowering still handles a handle name that the sema types as a cast. No language change. | ✅ Resolved — AI delegated; R5.5 |
| AR-38 | Fuzzer determinism proof | Is a same-process determinism test enough? | No. Hash order is stable within a process. The test runs two subprocesses with different `PYTHONHASHSEED` values and compares bytes. | ✅ Resolved — AI delegated; R7.2 |
| AR-39 | Module conditional guard | Does a module-level conditional struct arm get its guard? | The module walk leaves arm guards to the store core. The store core must run them on the selected arm; the fix sets the "guards already emitted" flag false for arm stores. A module-level conditional struct case is added. | ✅ Resolved — AI delegated; R1.3 |
| AR-40 | Phase reviews | When are independent reviews run? | After Phase 1 (correctness + memory-safety), after Phase 2 (those two + emission-integrity), after Phase 4 (correctness + emission-integrity), and after Phase 6 (correctness). A review fix re-runs the affected gates; the final gate runs after all reviews. | ✅ Resolved — repository quality policy; R9.6 |
| AR-41 | Suite-level sanitizer control | How is it proven that the suites' child builds are sanitized? | The robustness suite builds a known-UB C probe through its own flag helper under `RDC_UBSAN` and a known-OOB probe under `RDC_ASAN`, asserting the reports. | ✅ Resolved — AI delegated; R6.7 |
| AR-42 | Smoke fallback target | What is the host-only fallback? | `make fuzz-smoke-host` runs the host + sanitizer matrices without m68k; `make verify` runs the m68k subset separately if needed. The target is approved as a fifth Makefile target. | ✅ Resolved — AI delegated; R7.10 |
| AR-43 | Seed count | How many seeds are committed? | Twelve, each with one golden; the smoke runs all twelve. | ✅ Resolved — AI delegated; R7.7 |
| AR-44 | Struct seed | Does `struct_fill` cover the D1 struct path? | It is a fault seed with an out-of-range runtime index, so the missing guard is observable; a separate `struct_value` seed covers the normal path. | ✅ Resolved — AI delegated; R7.7 |
| AR-45 | Harness class | How is a harness bug classified? | `harness` (exit 2), separate from `compiler-rejected`, `internal`, and the compiler findings. | ✅ Resolved — AI delegated; R7.9 |
| AR-46 | Base test override | How is a test run against the pre-PLAN-14 binary? | The spec helper checks `RDC_CLC` before `CARGO_BIN_EXE_clc`; the recorded D3 command uses it. | ✅ Resolved — AI delegated; R3.3 |
| AR-47 | Core dumps | Can a fault seed leave a core file? | The harness sets `RLIMIT_CORE=0` for every child; the UBSan self-test does the same and asserts no core file remains. | ✅ Resolved — AI delegated; R7.5, R7.11 |
| AR-48 | Same-name enums | Are two modules with the same enum name supported? | The language resolves enum names by the existing bare-name rule; the emitter's map keeps the first declaration. This is a pre-existing collision scope, not newly supported. R5.6 covers a unique cross-module enum only. | ✅ Resolved — AI delegated; R5.6 |
| AR-49 | Cast-site record | How does the lowering know a call is a cast? | The checker records the exact call nodes it resolved through `callee_type` in `SemaOutput::type_casts`; the body and module-initializer lowerings read that set. This is precise for local, module-function, global, and import shadowing and avoids duplicating the name-resolution rule. | ✅ Resolved — AI delegated after the D6 review; R5.5 |
| AR-50 | Enum walk policy | Which module, values, and `sizeof` does the enum walk use? | The declaring module (temporarily bound while explicit values fold), the computed prefix cached so a later member can read an earlier one, no wrap, and the target ABI `sizeof` policy (extents=true) so an extent and the emitted macro agree with the runtime C `sizeof`. | ✅ Resolved — AI delegated after the D6 review; R5.6 |
| AR-51 | D5 range rule | Does the constant float-to-integer check use the truncated or untruncated value? | The untruncated value, matching the runtime `E11212` guard, so a fractional boundary (`i8(127.5)`) is a compile-time E10202 instead of a runtime trap. | ✅ Resolved — AI delegated after the D5 review; R4.4 |

## Delegated Decision Records

### AR-11 — Enum auto-increment overflow (user ruling)

```text
Authority: USER — explicit ruling in this session on 2026-11-07
Question: how should enum E: u8 { A = 255, B } behave when the folder wraps to 0 but the emitter writes 256?
Ruling: reject with E10202 — a member value must fit the backing type, and an auto-incremented value that does not fit reports the existing E10202 at the member
Eligibility: the user owns language-surface decisions; the goal requires an explicit ruling for any diagnostic/language change
Evidence: spec/04-expressions.md:372 "A constant value that does not fit its context type is E10202" (and :306 for literals); spec/14-diagnostics.md:88 registers the message and help; sema/items.rs already applies E10202 to explicit values; the value 256 is not representable in a u8-backed enum and fails the strict store build
Rejected alternatives: wrap at the backing type (silently changes 255/256 to 0); keep 256 in both (an unrepresentable enum value and invalid C99 for the spec store shape)
Confidence: High
Hardening: compile-fail case for enum E: u8 { A = 255, B }; in-range controls; the folder stops wrapping and returns the fit error through the declaration check; the emitter never sees an out-of-range member
Policy version: 1
Root invocation ID: codeops-robustness-20261107
Reopen triggers: a future spec rule that permits out-of-range enumerators
```

### AR-19 — Fuzzer oracle

```text
Authority: AI — delegated by --auto-design
Eligibility: verification tooling; no product behavior, no new dependency
Objective: find codegen and runtime divergences deterministically without claiming more than the oracle can see
Decision: the differential compares host and m68k for generated programs; committed seeds have explicit goldens and catch shared wrong values; the spec/impl suites remain the six-defect oracle
Evidence: five of the six defects produce the same wrong value on both profiles; the committed seeds are the only fuzzer input with a value oracle; the goal asks for host/target diff and sanitizers, not a general value oracle
Rejected alternatives: a full semantic oracle in the fuzzer (duplicates the compiler); no goldens (the smoke would pass on the starting compiler); coverage-guided mutation (new dependency, non-deterministic)
Strongest counterargument: a seeded template grammar only finds the shapes it knows; the counter is that the seeds and templates derive from the actual defect classes and corpus patterns
Confidence: Medium-High
Hardening: unit tests for deterministic generation and every failure class; the smoke gate; the longer local target
Policy version: 1
Root invocation ID: codeops-robustness-20261107
Reopen triggers: a divergence escapes the template families, or a shared wrong value escapes the seed goldens
```

### AR-26 — Enum and handle conversion emission

```text
Authority: AI — delegated by --auto-design
Eligibility: codegen conformance with an existing sema rule; no new language surface
Objective: make the spec's own conversion examples compile and run
Decision: lower a call whose callee names a type (enum or handle) to the scalar cast the sema stage already typed, instead of emitting a C function call; a local or parameter binding with the same name shadows the type and keeps the call path
Evidence: spec/09-enums-unions.md:38 uses Direction(2); sema check_type_cast already accepts it; own::lower_value emits ValueKind::Call; the strict C build fails with an implicit declaration; Buffer(0) has the same shape
Rejected alternatives: leave it and document an accepted limit (the spec example would stay broken); add a new syntax form (a language change); a bare-name check (it would misread a shadowing local)
Strongest counterargument: this is a seventh fix outside the six; the counter is that the conversion requirement cannot be tested without it, and it is local and spec-conformance
Confidence: High
Hardening: spec cases for a literal and a runtime argument, a shadowing case, and an impl pin for the C cast spelling
Policy version: 1
Root invocation ID: codeops-robustness-20261107
Reopen triggers: a cast form the lowering cannot distinguish from a function call
```

### AR-27 — Constant float-to-integer overflow

```text
Authority: AI — delegated by --auto-design
Eligibility: spec conformance; the diagnostic already exists in the frozen rule EX-16
Objective: stop the folder from emitting 0 for a NaN conversion and a saturated value for an out-of-range one
Decision: a constant float-to-integer cast that is NaN or outside the target range reports E10202 with the registered message/help and the target type
Evidence: spec/04-expressions.md:367-370 and EX-16; spec/14-diagnostics.md:88; enum E: i32 { A = i32(NaN const) } currently emits #define E_A 0; the runtime path traps E11212
Rejected alternatives: leave the folder value and rely on the runtime trap (wrong compile-time value, violates EX-16); report E11212 at compile time (the spec says E10202 for the constant case); a second message under E10202 (the registry pins the first line)
Strongest counterargument: this rejects a program that currently compiles; the counter is that the frozen spec mandates it and the runtime would trap anyway
Confidence: High
Hardening: cast, arithmetic, and declared-const cases; enum and array-extent negatives; the robustness spec suite asserts the registered message
Policy version: 1
Root invocation ID: codeops-robustness-20261107
Reopen triggers: a spec clarification that NaN is not an overflow
```
