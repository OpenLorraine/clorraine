# Preflight Report: Generated-C Optimization Plan

> **Status**: ✅ PREFLIGHT PASSED — 31 findings resolved (1 critical, 12 major, 18 minor)
> **Iteration**: 2 (fix verification and bounded re-scan)
> **Artifact**: Implementation plan at `plans/generated-c-optimization/`
> **Audit target**: `03-01-benchmark-harness.md`, `03-02-generated-c-changes.md`, `07-testing-strategy.md`, `99-execution-plan.md`
> **Context documents**: `00-index.md`, `01-requirements.md`, `00-ambiguity-register.md`, `00-preflight-report-requirements.md`, `02-current-state.md`, `AGENTS.md`, compiler sources, Makefile, docker scripts
> **Codebase Grounded**: 30+ source files and 14 probes examined across two independent audits
> **Mode**: auto-design active — eligible technical resolutions delegated under policy version 1
> **Last Updated**: 2026-11-07

## Same-Session Review

The plan was created in this session. Two independent `general` auditors
performed the first plan scan: one on implementation feasibility and codebase
alignment, one on delivery, testability, and fit. Their findings are
consolidated below. The author applied the resolutions and re-scanned the
changed plan. Same-session bias is recorded.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| Fill emitter and callers | `compiler/src/codegen/function.rs:3971`; `guards.rs:218` materialized caller; `array_assign_expr` at `guards.rs:1169` |
| Bounds condition | `guards.rs:2157`, `render.rs:20`; unsigned already omits `< 0` |
| Division guard | `guards.rs:2117`; `Int(0)` trap expression |
| Constant folder | `consts/mod.rs:105` (`fold`), `:140` (`integer_at`), `:151` (`integer_in`) |
| Const read lowering | `own/lower.rs:18`; module initializer `codegen/program.rs::lower_global` |
| Address-taken | `unsafe &N` emits `&(cl_N)` and a raw write changes the value |
| GlobalInfo | `own/program.rs:72`; `collect` runs once before lowering |
| Fill targets | `ArrayTarget` at `function.rs:4314`; adjusted parameter and `__cl_ret` verified |
| Toolchain | `m68k-amigaos-gcc` 16.2.0b, host `cc` Debian 12.2.0, Python 3.11.2, `vamos` instruction trace deterministic, `m68k-amigaos-size` parses |
| Base worktree | detached worktree under `build/` builds `clc` in 27 s; stale registration needs `git worktree prune` |
| `vamos --version` | not supported; use `importlib.metadata.version("amitools")` |

## Summary by Severity

| Severity | Count | Status |
|---|---:|---|
| CRITICAL | 1 | Resolved |
| MAJOR | 12 | Resolved |
| MINOR | 18 | Resolved |
| OBSERVATION | 0 | — |

## Major Findings

### PF-PLAN-001 — The R4 fold lacks the declared integer width [🔴 CRITICAL]

**Location:** `03-02` §R4; R4.5
**Evidence:** `ConstFolder::fold` calls `fold_value(expr, None)`; a shift
masks to 128 bits without a context. Probe: `const SHIFT: i32 = 1 << 31;`
runs as `-2147483648`, but the folder returns `2147483648`; the plan's typed
`Int(2147483648)` would render `2147483648u`, so `SHIFT < 0` would be false.
**Problem:** A final wrap does not reproduce shift masking; `1 << 32` must be
`1`, not `0`.
**Resolution:** R4.5 and `03-02` now require the typed `integer_in` fold at the
const's declared integer width. ST-36/ST-37 and IP-17 pin `1 << 31`, `1 << 32`,
and a wrapping add.

### PF-PLAN-002 — The float path does not round at `f32` casts and can emit invalid C [🟠 MAJOR]

**Location:** `03-02` §R4; R4.1/R4.5
**Evidence:** `fold_cast` keeps an unrounded `f64` value for an `f32` target;
`float_literal` spells `inf.0`/`NaN.0`, which are not C tokens.
**Problem:** A folded `f32` arithmetic value can differ from runtime; a
non-finite fold breaks the strict build.
**Resolution:** R4.1/R4.5 restrict float folding to a finite literal, const
name, or cast chain; a float arithmetic or non-finite initializer stays a
place. An `f32` value is rounded through `f32` before the cast. ST-3 uses a
precision-sensitive literal cast; ST-38/IP-18 cover non-finite.

### PF-PLAN-003 — The guard constant helper is not width-aware [🟠 MAJOR]

**Location:** `03-02` §R6/§R7; R6.1/R7.1
**Evidence:** `constant_integer` evaluates binary operations in i128 without
the operand type's wrap. A divisor `i32(2147483647)+i32(2147483647)+i32(1)`
is C `-1`, but the helper reports `4294967295`; removing the guard would lose
the `MIN / -1` trap.
**Resolution:** R6.1/R7.1 and `03-02` restrict the guard constant to a
literal, `bool`/character, cast, or unary form; a binary expression keeps the
full guard. ST-41 pins the wrapping divisor.

### PF-PLAN-004 — R5 also changes the materialized fill path [🟠 MAJOR]

**Location:** R5.1; `03-02` §R5
**Evidence:** `guards.rs:218` calls `emit_array_assign_to` for a materialized
fixed-array literal; the shared function would compact that form too, although
R5.1 excludes it.
**Resolution:** `emit_array_assign_to` gains a `compact` flag; the materialized
caller passes `false`. IP-16 pins the scope.

### PF-PLAN-005 — The volatile-fill cases contradicted R5.4 [🟠 MAJOR]

**Location:** R8.1; `07` IP-14; `03-02` §R5
**Evidence:** an intrinsic fill is impure and evaluated once into `__cl_fill`;
a volatile place read is pure and repeats per element.
**Resolution:** R8.1, ST-16/ST-16b, IP-14/IP-15 now distinguish the two: an
impure intrinsic is evaluated once into a temporary, a pure volatile place is
read per element. IP-14 and IP-15 pin both shapes.

### PF-PLAN-006 — `bench-fast` and `bench-compare` were incompatible [🟠 MAJOR]

**Location:** `03-01`; `99` 3.1.4/4.1.4/5.1.4
**Evidence:** `--no-trace` omits `instructions_net`, which `compare` rejects;
`bench-fast` overwrote `head.json`.
**Resolution:** `bench-fast` writes a separate `head-fast.json`; every
per-phase acceptance uses the full `make bench`; the compare is only against a
full head. `03-01` and the execution tasks are updated.

### PF-PLAN-007 — The red-first pin claim and the task duplication [🟠 MAJOR]

**Location:** `07`; `99` 2.2.1/3.1.1/4.1.2/5.1.1/5.1.2
**Evidence:** IP-4, IP-8, IP-11, IP-12, IP-13, IP-16, IP-18, IP-19 already
pass at HEAD; only the other pins are red. Several pins were to be authored in
both Phase 2 and later phases.
**Resolution:** 2.2.1 authors all pins and records red/green; phases 3–5 only
run them. `07` lists the red and green sets explicitly.

### PF-PLAN-008 — `05-findings.md` was missing and the baseline owner conflicted [🟠 MAJOR]

**Location:** `00-index`, R3.1, `02`, `99`
**Evidence:** the index linked a missing file; R3.1/AR-3 named `02`, while 02
named `05`; no task recorded the committed baseline before Phase 3.
**Resolution:** `05-findings.md` now owns the committed base/head evidence;
`02` owns the exploratory study; task 1.1.6 records the baseline after
`make bench-base`.

### PF-PLAN-009 — The base worktree cleanup could not recover from a crash [🟠 MAJOR]

**Location:** `03-01` §Base/Head
**Evidence:** deleting a worktree without removing its registration makes the
next add fail (`missing but already registered`).
**Resolution:** `03-01` uses `git worktree remove --force`, `git worktree
prune`, refuses a symlink, and traps cleanup on failure.

### PF-PLAN-010 — The base result recorded the head commit [🟠 MAJOR]

**Location:** R1.4/R1.9; `03-01`
**Evidence:** `compiler.git` was `git rev-parse HEAD` of the harness, not the
base compiler.
**Resolution:** `--compiler-git` is passed explicitly; `bench-base.sh` passes
the resolved base commit, `make bench` passes HEAD; `harness_git` is recorded
separately.

### PF-PLAN-011 — The vbcc/freestanding shape leg and the C-byte gate were missing [🟠 MAJOR]

**Location:** R8.5/R8.7; `03-01` compare; `99`
**Evidence:** `corpus-amiga.sh` vbcc legs do not build a `[;` fill; the compare
did not hard-gate `c_bytes`.
**Resolution:** 6.1.3 adds a focused vbcc/freestanding shape leg; the compare
hard-gates `sieve.c_bytes <= 0.75 * base`, `sieve.text <= 0.90 * base`, and
`sieve.instructions_net <= 0.80 * base`.

### PF-PLAN-012 — The base compiler was deleted before the emission diff [🟠 MAJOR]

**Location:** R8.5; `03-01`; `99` 6.2.1
**Evidence:** the base worktree/target was removed; the emission auditor needs
a base `clc`.
**Resolution:** `bench-base.sh` retains `build/perf/base-clc`; 6.2.1 emits the
corpus with base and head and attributes every difference.

## Minor Findings (all resolved)

| ID | Finding | Resolution |
|---|---|---|
| PF-PLAN-013 | ST rows omitted module const types, `export`, `: void`, `slice` import, and used `1` for a bool | `07` source conventions; ST-2/3/5/17/21 fixed; ST-36–ST-41 added |
| PF-PLAN-014 | Trap cases had no execution path | `07` reuses the host build/run pattern and asserts the exact first stderr line |
| PF-PLAN-015 | Execution plan count and Phase 6 row were wrong | 30 tasks; Phase 6 has 6; progress header updated |
| PF-PLAN-016 | Missing Dependencies/Success Criteria/Lenses/Verify/Specialist Agents/Related Files | Added to `00-index` and `99`; 2.1.x/2.2.1 marked `[spec-author]` |
| PF-PLAN-017 | Headline numbers were superseded | Container medians 31.0→22.0 ms; 24.3%; host claim qualified |
| PF-PLAN-018 | Qualified const vs qualified member was undefined | R4.2 corrected; ST-42/IP-20 added |
| PF-PLAN-019 | Calibration had no expected output/README | `03-01` and task 1.1.3 add them for all five programs |
| PF-PLAN-020 | Self-tests/timeouts missed path, label, unknown bench, `\r`, timeout | `03-01` lists 2/15/300 s timeouts and the additional unit cases |
| PF-PLAN-021 | The spec suite did not opt into ASan | 2.1.1/2.1.2 and `07` reference `RDC_ASAN` |
| PF-PLAN-022 | Worktree symlink/cleanup was unspecified | `03-01` refuses a symlink and uses worktree remove/prune |
| PF-PLAN-023 | The struct-size metric was dropped without a record | AR-22 records the drop; R8.7/`00-index` updated |
| PF-PLAN-024 | The debug timing contract disagreed with the schema | R1.2 says debug runs once; release records best/median |
| PF-PLAN-025 | `make bench-base COMMIT=…` reached a shell | Ref resolved in make, validated 40-hex, forwarded via `PASSTHROUGH` |
| PF-PLAN-026 | `--label` was unvalidated and cleanup could escape | R1.10/`03-01` allowlist and `build/perf/` confinement |
| PF-PLAN-027 | The address-taken walker was under-specified | R4.3/`03-02` enumerate every body, statement, and expression; ST-39/IP-19 nested cases |
| PF-PLAN-028 | `constant_integer` visibility was not addressed | `03-02` exposes a shared `constant_guard_integer` subset |
| PF-PLAN-029 | Typed shift/wrap and f32 precision cases were missing | ST-36/37/38, IP-17/18 |
| PF-PLAN-030 | The calibration was in the Files table without expected/README | `03-01` adds both |
| PF-PLAN-031 | Phase 0 status/deliverable and the plan preflight link were stale | 0.1.2 marked `[~]`; this report is the link; final task marks it `[x]` |

## Iteration 2 Verification

| Check | Result |
|---|---|
| Typed integer fold | R4.5 uses `integer_in`; ST-36/37 and IP-17 pin shift/wrap |
| Float path | R4.1/R4.5 restrict to finite literal/cast/name; ST-3/ST-38, IP-18 |
| Guard constants | R6.1/R7.1 restrict to non-binary forms; ST-41 pins the wrap case |
| Materialized fill | `compact=false` caller; IP-16 pins the scope |
| Volatile fill | ST-16/16b, IP-14/15 distinguish impure and pure |
| Fast/full benchmark | `head-fast.json` separate; phases use `make bench` |
| Red/green pins | `07` lists the exact sets; 2.2.1 authors all |
| Baseline evidence | `05-findings.md` created; 1.1.6 records the committed baseline |
| Base/head identity | `--compiler-git`; `base-clc` retained |
| Safe commit/label | Ref validated and passed via `PASSTHROUGH`; label allowlist |
| Plan structure | 30 tasks, Phase 6 = 6, Dependencies/Success Criteria/Lenses/Verify/Specialist Agents/Related Files |
| ST syntax | Types, `export`, `: void`, `slice` import, bool text, trap path fixed |
| Acceptance coverage | 6.1.3 vbcc/freestanding; 6.1.4 full gates; 6.2.1 emission diff; 6.2.2 five reviewers |
| Numbers | Container medians and 24.3% in index/AR/02 |

## Verdict

`✅ PREFLIGHT PASSED — all 31 findings resolved.` The plan is executable: the
critical typed-fold and guard-width issues are closed, the harness has a
reproducible base/head path and safe inputs, the test oracle is green-before
with an explicit red set, and the acceptance matrix covers the full repository.
No product-scope, acceptance-criteria, security, legal, or publication decision
was delegated. Iteration 3 is not triggered.
