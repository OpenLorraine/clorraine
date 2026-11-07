# Preflight Report: Generated-C Optimization Requirements

> **Status**: ✅ PREFLIGHT PASSED — 27 findings resolved (1 critical, 13 major, 13 minor)
> **Iteration**: 2 (fix verification and bounded re-scan)
> **Artifact**: Requirements set at `plans/generated-c-optimization/`
> **Audit target**: `00-index.md`, `01-requirements.md`, `00-ambiguity-register.md`, `02-current-state.md`
> **Context documents**: `AGENTS.md`, `spec/17-codegen.md`, `compiler/src/codegen/`, `compiler/src/own/`, `compiler/src/consts/`, `compiler/src/clorraine/toolchain.rs`, `Makefile`, `docker/`
> **Codebase Grounded**: 24 source files and 11 probes examined across two independent audits
> **Mode**: auto-design active — eligible technical resolutions delegated under policy version 1
> **Last Updated**: 2026-11-07

## Same-Session Review

The requirements were created in this session. Two independent `general`
auditors performed the first scan: one on document soundness and codebase
grounding (dimensions 1–6), one on delivery and fit (dimensions 7–13). Their
findings are consolidated below. The author applied the resolutions and
re-scanned the changed requirements; the same-session bias is recorded here as
required.

## Codebase Context Summary

**Tech stack:** one std-only Rust workspace; `clc` emits one C99 translation
unit; `m68k-amigaos-gcc` 16.2.0b, `vamos`, host `cc` (Debian GCC 12.2.0) in the
pinned image; Python 3.11.2.

**Architecture facts verified:**

| Fact | Evidence |
|---|---|
| Statement fill emitter | `compiler/src/codegen/function.rs:3971` (`emit_array_assign_to`) |
| Deferred expression fill emitter | `compiler/src/codegen/guards.rs:1169` (`array_assign_expr`) |
| Bounds condition | `compiler/src/codegen/guards.rs:2157`, `compiler/src/codegen/render.rs:20` |
| Division guard | `compiler/src/codegen/guards.rs:2108` |
| Function-body const read | `compiler/src/own/lower.rs:18` |
| Module-initializer lowering | `compiler/src/codegen/program.rs::lower_global` |
| Constant folder | `compiler/src/consts/mod.rs:105` |
| Module const storage | emitted by `compiler/src/codegen/program.rs`; order by `compiler/src/codegen/init.rs` |
| Unsigned guard already omits `< 0` | `render.rs:20-26`; probe `usize` index emits no lower check |
| Address-of const accepted | `unsafe &N` probe emits `p = &(cl_N);` and a raw write changes the value |
| Trace determinism | `vamos --instr-trace` counts identical on repeat; stderr carries `instr:` lines |

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 2 | 🟠 MAJOR |
| 2 | Implicit assumptions | 5 | 🔴 CRITICAL |
| 3 | Logical contradictions | 4 | 🟠 MAJOR |
| 4 | Completeness gaps | 6 | 🟠 MAJOR |
| 5 | Dependency issues | 3 | 🟠 MAJOR |
| 6 | Feasibility | 3 | 🟠 MAJOR |
| 7 | Testability | 6 | 🟠 MAJOR |
| 8 | Security blind spots | 1 | 🟡 MINOR |
| 9 | Edge cases | 4 | 🟠 MAJOR |
| 10 | Scope creep | 1 | 🟠 MAJOR |
| 11 | Ordering & sequencing | 2 | 🟠 MAJOR |
| 12 | Consistency | 4 | 🟡 MINOR |
| 13 | Codebase alignment | 5 | 🟠 MAJOR |

## Summary by Severity

| Severity | Count | Status |
|---|---:|---|
| CRITICAL | 1 | All resolved |
| MAJOR | 13 | All resolved |
| MINOR | 13 | All resolved |
| OBSERVATION | 0 | — |

## Findings and Resolutions

### PF-REQ-001 — Const folding is unsound when a const's address is taken [🔴 CRITICAL]

**Dimension:** 2, 6, 13
**Location:** `01-requirements.md` R4.1–R4.6; `00-ambiguity-register.md` AR-7
**Codebase evidence:** `compiler/src/own/lower.rs:201` lowers unary operands
through `lower_value`; `compiler/src/codegen/expr.rs:754` renders `&(operand)`.
At HEAD, `const N: i32 = 5; let p: raw i32 = unsafe &N; unsafe { *p = 6; }
return N - 5;` compiles, emits `p = &(cl_N); *p = 6; return cl_N - 5;`, and
exits 1. Folding the read would return 0.

**Problem:** R4.2 said folding applies in every position and assumed only
"accepted borrow forms" matter. Raw address-of is accepted and can mutate the
storage between reads. Folding would change behavior, violating N1.

**Resolution:** R4.3 adds a whole-program address-taken pre-pass; any const
whose address is taken in a function body or module initializer is never
folded. AR-18 records the mechanism. R8.1 adds the address-taken/raw-write
spec case. Rejected alternative: make the storage read-only — a language/ABI
change that would break the accepted program.

**Confidence:** High. **Hardening:** the probe is reproducible at HEAD; the
pre-pass is conservative.

---

### PF-REQ-002 — A local can shadow a module const in the folder's name lookup [🟠 MAJOR]

**Dimension:** 2, 13
**Location:** R4.2; `02-current-state.md` "Function-body const read"
**Evidence:** `consts/mod.rs:360-381` resolves names in the module context;
`own/place.rs:3710-3726` resolves a local first. A probe with a local `N`
shadowing a module `const N` shows the extent folder already uses the module
value, a pre-existing bug.
**Problem:** A naive `ConstFolder` call in `lower_value` could fold the local
read to the module const.
**Resolution:** R4.2 requires the resolved `place_target` to be a bare
`Global` const before folding; locals, parameters, and projections stay places.
A shadowing spec case is required in R8.1. The pre-existing extent-shadowing
bug is out of scope and recorded as such.

**Confidence:** High.

---

### PF-REQ-003 — Imported/expression initializer chains cannot fold with the caller's folder [🟠 MAJOR]

**Dimension:** 5, 6
**Location:** R4.2/R4.4; `02-current-state.md`
**Evidence:** `consts/mod.rs:414-449` folds a module const's initializer in the
caller's module context; `const A = 5; const B = A;` imported from another
module fails to fold as an extent.
**Problem:** R4.2 promised imported/qualified/expression folding; the named
mechanism cannot deliver it.
**Resolution:** R4.4 requires constructing the `ConstFolder` for the
**declaring module**. R8.1 adds an imported/expression-initializer case. The
pre-existing extent folder bug is not changed; the feature folds reads, not
extents.

**Confidence:** High.

---

### PF-REQ-004 — Negative integers and `f32` are not rendered as typed literals [🟠 MAJOR]

**Dimension:** 6, 7
**Location:** R4.5; `02-current-state.md`
**Evidence:** `codegen/expr.rs:40` casts a folded `i128` to `u64`;
`int_literal` at `558-574` appends `u` for large values, so `-1` would become
`18446744073709551615u`. `float_literal` at `render.rs:104-111` has no type
suffix, so an `f32` fold could compare as a double.
**Problem:** R4.5's "same as a source literal" was not implementable through
the existing literal path.
**Resolution:** R4.5 now requires a typed unary negation for negative
integers and a `float` cast for `f32`; R8.1 adds negative-integer and `f32`
behavior cases. Rejected alternative: restrict folding to non-negative
integers and `f64` — it would leave common constants unfolded.

**Confidence:** High.

---

### PF-REQ-005 — Module-level initializers bypass the function-body lowering [🟠 MAJOR]

**Dimension:** 4, 5
**Location:** R4.2, R4.7
**Evidence:** `codegen/program.rs::lower_global` turns `Name`/`Qualified` into
`ValueKind::Place`; `own/lower.rs:18` is the function-body path. A probe
`const N = 5; let X: i32 = N;` emits `X = cl_N`.
**Problem:** R4.2 named "initializers" without saying which path; the named
hook does not reach module initializers.
**Resolution:** R4.7 explicitly excludes module-level initializers and
`lower_global` stays unchanged. Rejected alternative: implement both paths —
no measured need and more surface.

**Confidence:** High.

---

### PF-REQ-006 — R6's unsigned/evidence claim is already implemented [🟠 MAJOR]

**Dimension:** 2, 3, 13
**Location:** R6.1/R6.5; `02-current-state.md` "R6" row; AR-8
**Evidence:** `render.rs:20-26` omits `< 0` for an unsigned index. A `usize`
probe emits no lower check. The sieve uses signed `i32`. A slice/`str`
constant index still emits `(1 < 0)`.
**Problem:** R6's "unsigned index" trigger and acceptance were green before
the change and the recorded evidence was false.
**Resolution:** R6 is rewritten around the real dead case: a non-negative
constant index into a slice or `str`. Unsigned and fixed-array constant forms
are regression assertions. AR-8 and the current-state table are corrected.

**Confidence:** High.

---

### PF-REQ-007 — Negative slice/string literals are runtime traps, not compile errors [🟠 MAJOR]

**Dimension:** 2, 4
**Location:** R6.2
**Evidence:** `compiler/src/sema/expr.rs:1033-1055` rejects a negative
constant only for a known-extent fixed array. A slice probe `s[-1]` compiles
and emits a runtime `E11211` guard. The existing run-fault golden covers only
a fixed array with a runtime `usize`.
**Problem:** R6.2's blanket "negative literal is rejected" could lead an
implementer to drop the lower check for a slice/string literal, turning an
accepted trap into an out-of-bounds read.
**Resolution:** R6.2 now states the distinction; R8.1 adds slice and string
negative and upper-out-of-range trap cases in the new suite, without editing
a frozen golden.

**Confidence:** High.

---

### PF-REQ-008 — One workload cannot serve both a 20-repetition trace and a 2,000-repetition host run [🟠 MAJOR]

**Dimension:** 3, 7, 9
**Location:** R1.2/R1.5; `02-current-state.md` baseline tables; R2.3/R2.4
**Evidence:** the 3,044,035-instruction count is for 20 repetitions; the host
table is 2,000 repetitions. A single committed program must produce one
output and one workload. The auditor measured `vamos --instr-trace` at about
37 s and 109 MB per 1,004,028 instructions; a 3M-instruction trace is about
112 s and 330 MB.
**Problem:** The baseline mixed two workloads, and the trace budget was
marginal with a log file.
**Resolution:** R2 defines one workload per program. R2.4 raises the trace
budget to 300 s and R1.7 streams and counts the trace instead of writing a
log. R1.5 makes host time advisory. `make bench-fast` skips tracing for
iteration. The committed baseline is re-recorded with the committed program.

**Confidence:** High.

---

### PF-REQ-009 — No reproducible base/head path [🟠 MAJOR]

**Dimension:** 4, 11, 13
**Location:** R3.1, R8.4; AR-3
**Evidence:** `benchmarks/` and `tools/perf/` do not exist at HEAD; the
exploratory source is under ignored `build/`; AR-3 rejected a committed
baseline JSON; nothing produces the base result document.
**Problem:** R8.4 could not be executed from a fresh checkout.
**Resolution:** R1.9 adds `make bench-base COMMIT=<ref>`: a detached worktree
under `build/perf/`, a base `clc` build, the current harness and benchmarks,
and `build/perf/base.json`. `make bench` writes the head; `make
bench-compare` compares. AR-21 records it.

**Confidence:** High.

---

### PF-REQ-010 — The exploratory host numbers used a different compiler [🟠 MAJOR]

**Dimension:** 2, 5
**Location:** `02-current-state.md` environment and host table
**Evidence:** the exploratory host values were measured with Ubuntu GCC 15.2;
the harness runs in the pinned image with Debian GCC 12.2.0.
**Problem:** The baseline was not the harness baseline.
**Resolution:** `02-current-state.md` is re-measured inside the pinned image
with GCC 12.2.0; R1.4 records the toolchain identity; the harness runs there.

**Confidence:** High.

---

### PF-REQ-011 — A 2% host-time gate contradicts the metric policy and the noise [🟠 MAJOR]

**Dimension:** 3, 12
**Location:** R1.5; AR-2, AR-11; `02-current-state.md` host table
**Evidence:** AR-2 says host time is not the acceptance metric; the recorded
30-run spread is 6–10%; the manual-check variant is 35.0 ms median vs 31.0 ms
baseline in the container.
**Problem:** A valid change could fail on noise.
**Resolution:** R1.5 and AR-11 make host time advisory; the hard gate is
`net_instructions` and `.text`. The 1% allowance is retained for the
deterministic metrics.

**Confidence:** High.

---

### PF-REQ-012 — The spec suite cannot be red for behavior-preserving changes [🟠 MAJOR]

**Dimension:** 7
**Location:** R8.1/R8.2; AGENTS.md spec/impl rule
**Evidence:** R4–R7 preserve behavior, so a spec test derived from behavior
passes before the change; the repository says a failing spec test means the
implementation is wrong.
**Problem:** "Run the spec suite red" was unexecutable and invited weakening
the oracle.
**Resolution:** R8.1 makes the spec suite a green-before/green-after
regression oracle; R8.2 makes the impl suite the red-first driver for shapes
that are actually absent at HEAD (large fill, slice `< 0`, `% 2` guard).

**Confidence:** High.

---

### PF-REQ-013 — Prefix and nested-row compaction have no measured baseline [🟠 MAJOR]

**Dimension:** 6, 10
**Location:** R5.3/R5.4; R3.2
**Evidence:** the only measured fill is a one-extent `u8[8192]`; nested rows
already use a row temp and one `memcpy` per row; no prefix or struct baseline
exists.
**Problem:** The requirements exceeded the measured need.
**Resolution:** R5 is narrowed to one-extent scalar statement fills. Nested
rows, struct-literal fills, array copies, and the deferred expression form
keep their spelling and are in Won't Have. R5.3 keeps prefix support because
it is the same loop range, but the acceptance is the one-extent sieve.

**Confidence:** High.

---

### PF-REQ-014 — The existing fault goldens do not pin the literal/negative/slice cases [🟠 MAJOR]

**Dimension:** 4, 7
**Location:** R6.5, R7.5; R8.1
**Evidence:** `spec/tests/run/E11210_division_by_zero.clo` uses a runtime
zero; `E11210_min_neg_one.clo` uses a runtime `-1`; `E11211_index_out_of_range.clo`
uses a fixed array and a runtime `usize`. A literal `-1` divisor is represented
as `Unary(Neg, Int(1))`, not `ValueKind::Int(-1)`.
**Problem:** R6/R7 could regress a literal/negative/slice trap and still pass
the existing gate.
**Resolution:** R8.1 adds literal-zero, literal `MIN / -1`, slice/string
negative, and slice/string upper trap cases in the new spec suite; no frozen
golden is edited. R7 uses `constant_integer` for constant-divisor detection.

**Confidence:** High.

---

### PF-REQ-015 — Fill threshold, `bool`, `memset`, and volatile evaluation were ambiguous [🟡 MINOR]

**Dimension:** 1, 9, 12
**Location:** R5.1–R5.5; R8.2
**Evidence:** `bool` is not byte-sized on the target; `init_operand_is_pure`
treats a place read as pure; the threshold could mean `extent` or the covered
range.
**Resolution:** R5.1 uses `extent - prefix`; R5.2 uses `memset` only for
`u8`/`i8` constants and a loop for every other scalar, including `bool`;
R5.4 keeps the per-element expression in the loop for a pure fill, preserving
a volatile read count. R8.1 adds a volatile fill case.

---

### PF-REQ-016 — The new suites should opt into ASan [🟡 MINOR]

**Dimension:** 4
**Location:** R8.3
**Evidence:** `make asan` discovers suites that reference `RDC_ASAN`; the new
fill path uses computed `memset` sizes.
**Resolution:** R8.3 requires the new spec suite to opt into `RDC_ASAN` and
cover fill/const memory cases.

---

### PF-REQ-017 — `bench-test` placement and CI coverage [🟡 MINOR]

**Dimension:** 4, 5
**Location:** R1.6
**Evidence:** `make check` runs host shell steps and container cargo tests;
the container has Python 3.11.2; CI runs only the spec check and workspace
tests.
**Resolution:** R1.6 runs `bench-test` inside the pinned container as part of
`make check`; the requirement no longer claims PR coverage that the host-only
workflow does not provide.

---

### PF-REQ-018 — Headline host numbers did not reconcile with the table [🟡 MINOR]

**Dimension:** 3, 12
**Location:** `00-index.md`, AR-7, `02-current-state.md`
**Evidence:** the index said 42→27 ms and 39→27 ms; the table said 36.9→22.5
and 21.6→23.0; the "checks removed made host faster" claim was false against
the const-folded run.
**Resolution:** all headline numbers now use the container medians
(31.0→22.0 ms; the manual check removal is rejected on m68k and is not a host
win). The comparison base is explicit.

---

### PF-REQ-019 — `char`, the `init.rs` owner, and tool versions were wrong or missing [🟡 MINOR]

**Dimension:** 2, 4
**Location:** R4.1; `02-current-state.md`; R1.4
**Evidence:** `char` is not declared (E10219); character literals are `u8`;
`init.rs` orders initializers while `program.rs` emits them; the schema did
not record tool versions.
**Resolution:** R4.1 uses the integer types and says character literals are
`u8`; `02` names `program.rs` as the emitter and `init.rs` as the order; R1.4
records the toolchain identity.

---

### PF-REQ-020 — Review set, findings document, freestanding leg, and struct-size metric [🟡 MINOR]

**Dimension:** 4, 10
**Location:** R8.5–R8.7; `00-index.md`
**Evidence:** AGENTS.md assigns the semantics reviewer to language-rule and
evaluation-order changes; the index did not list `05-findings.md`; the roadmap
names "struct sizes" but R1.2 did not.
**Resolution:** R8.6 adds the semantics reviewer and performance auditor;
the index lists `05-findings.md`; R8.5 adds the freestanding compile leg; R8.7
requires recording the struct-size metric drop or restoring it.

---

### PF-REQ-021 — Harness shell and path safety were unspecified [🟡 MINOR]

**Dimension:** 8, 9
**Location:** R1.1, N6
**Evidence:** new Python code under `tools/perf/`; no CLI or cleanup contract.
**Resolution:** R1.10 and N8 require argument-list subprocesses, benchmark-name
validation, an output path confined to `build/perf/`, and cleanup limited to
the created run directory.

---

### PF-REQ-022 — Timeouts, determinism, and failure cases were not enforced or tested [🟡 MINOR]

**Dimension:** 7, 9
**Location:** R1.6/R1.7, R2.4
**Evidence:** no timeout requirement; the two-run determinism and fake-compiler
failures were not in the unit-test list.
**Resolution:** R1.7 adds per-step timeouts; R2.4 records each duration;
R1.6 adds the determinism, fake-emit-failure, and output-mismatch cases.

---

### PF-REQ-023 — Frozen guard formulas could be read as forbidding the simplification [🟡 MINOR]

**Dimension:** 13
**Location:** `spec/17-codegen.md:304,417`; N7
**Evidence:** the spec writes the guard as a formula; omitting a constant-false
disjunct is behaviorally equivalent but not literally the same text.
**Resolution:** N7 states that the guard rows are behavioral, a constant-false
disjunct may be omitted, and the trap code/message/exit status stay identical.
No `spec/` file is edited. The spec owner can still rule otherwise; if so, R6/R7
are dropped without affecting R4/R5.

---

### PF-REQ-024 — Result schema omitted output, runs, flags, and toolchain [🟡 MINOR]

**Dimension:** 7, 9
**Location:** R1.2/R1.4
**Evidence:** `compare` checks output but the schema had no output hash; the
m68k run strips `\r`; host `usize` is 64-bit and m68k `usize` is 32-bit.
**Resolution:** R1.4 records output SHA-256, run count, profile, flags, and
toolchain; R2.3 requires fixed-width checksum types; the harness normalizes
`\r\n` to `\n`.

---

### PF-REQ-025 — The second fill emitter and nested-row facts were missing [🟡 MINOR]

**Dimension:** 13
**Location:** `02-current-state.md` codebase facts; R5
**Evidence:** `guards.rs::array_assign_expr` emits the deferred expression
form; the nested-row path already uses a temp and one `memcpy` per row.
**Resolution:** `02` names both emitters; R5 excludes the deferred/nested forms
from compaction.

---

### PF-REQ-026 — R7.3 described a nonexistent `if` guard for a literal zero [🟡 MINOR]

**Dimension:** 7, 13
**Location:** R7.3
**Evidence:** `division_guard_condition` returns `None` for `Int(0)`; the value
renderer emits `((void)(x), cl_trap("E11210", ...), 0)`.
**Resolution:** R7.3 now says the trap expression is unchanged and no separate
`if` guard is emitted; the spec suite tests the trap, not the guard shape.

---

### PF-REQ-027 — Acceptance-threshold authority was ambiguous [🟡 MINOR]

**Dimension:** 10, 12
**Location:** AR-11, Reserved Decisions
**Evidence:** The register reserved acceptance criteria but delegated numeric
thresholds.
**Resolution:** The register now distinguishes product acceptance criteria
(reserved) from the technical pass thresholds that implement the user's
explicit measured-need/"make it shine" direction. AR-11 records the rationale
and the advisory host-time rule. No product behavior is delegated.

## Iteration 2 Verification

| Check | Result |
|---|---|
| Address-taken const probe | `unsafe &N` + raw write returns 1 at HEAD; R4.3 preserves it and R8.1 pins it |
| Shadowing/local gate | R4.2 requires a bare `Global` const; spec case added |
| Imported/expression initializer | R4.4 uses the declaring-module folder; spec case added |
| Negative/`f32` typed literal | R4.5 uses unary negation and an `f32` cast; spec cases added |
| Module initializer boundary | R4.7 explicit; no `lower_global` change |
| R6 real case | Slice/`str` constant `1` emits `(1 < 0)`; R6 is red on that shape |
| R6 negative trap | R6.2 distinguishes fixed-array rejection from slice/string runtime traps |
| Trace workload and budget | One workload; 300 s; streamed; `make bench-fast` |
| Base/head path | R1.9 `make bench-base`; AR-21; `build/perf/base.json` |
| Host metric | Advisory in R1.5/AR-11; container numbers in `02` |
| Spec red-first | Green regression spec; red-first impl pins in R8.2 |
| Fill scope | One-extent scalar only; `u8`/`i8` memset; `bool` loop; nested/struct/deferred excluded |
| Fault coverage | Literal zero, `-1`, slice/string negative and upper traps in R8.1 |
| ASan / bench-test | R8.3 and R1.6 |
| Frozen spec | N7; no `spec/` edit; guard behavior unchanged |
| Harness safety | R1.10 and N8 |
| Docs | `05-findings.md` in the index; known-limitations mandatory; struct-size metric recorded |
| Toolchain / schema | R1.4 records compiler, toolchain, runs, flags, output hash |

## Verdict

`✅ PREFLIGHT PASSED — all 27 findings resolved.` All critical and major
findings are closed in the requirements; the minor findings are resolved in
the same edit pass. No product-scope, acceptance-criteria, security, legal, or
publication decision was delegated. Iteration 3 is not triggered.
