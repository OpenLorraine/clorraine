# Preflight Report: String Integer Formatting Throughput

> **Audit target**: `plans/string-integer-formatting/` (plan set)
> **Content hash**: computed at scan start (plan folder, 2026-11-11)
> **Date**: 2026-11-11
> **Mode**: auto-design (technical findings resolved under the delegated
> policy; the scope finding is owned by issue #19's user-authored boundary)
> **⚠️ SAME-SESSION REVIEW**: the plan and this report were produced in the
> same session. The scan used the actual repository and the reproduced probe;
> a fresh-session review is still the stronger independence guarantee.

## Codebase Context Summary

| Area | What was read | What it establishes |
|---|---|---|
| Converters | `compiler/runtime/shim_host.c:352`, `:362`; `compiler/runtime/shim_hosted_amiga.c:969`, `:979` | Current `snprintf` bodies and signatures |
| Block | `shim_host.c:226`, `:242`, `:262`; `shim_hosted_amiga.c:847`, `:863`, `:879` | `[len][cap][bytes][NUL]` layout, allocation size, append guard |
| Callers | `compiler/src/codegen/expr.rs:1556` (`text_call`), `stdlib/std.fmt.clo`, `stdlib/std.text.clo:16` | Template, `text`, `toDecimal`, and `cl_string_append` reachability |
| Profiles | `compiler/src/profile.rs:10`, `:13`, `:31` | Shims are embedded; freestanding has no `String` |
| Harness | `tools/perf/bench.py`, `benchmarks/strings/program.clo` | Metrics, calibration, shipped flags |
| Gates | `Makefile` targets, `docker/scripts/asan-suite.sh`, `ubsan-suite.sh`, `bench-shape.sh` | Sanitizer discovery and strict-C legs |
| Evidence | `build/perf/audit` probe (removed at closeout) | Attribution, allocation count, base/head tables |

## Findings

### PF-001 — The plan delivers the formatting half, not allocation elimination

- **Severity**: 🟠 MAJOR
- **Dimension**: Completeness / scope
- **Evidence**: issue #19's title and Opportunity name temporary `String`
  allocations; the selected shim-only shape keeps 6,003 allocations.
- **Resolution**: the plan states the partial delivery explicitly
  (`01-requirements.md` Scope, `05-findings.md` §7), cites the issue's
  "standard-library and runtime-shim work only" boundary, and records the
  codegen option as a follow-up issue with probe evidence (AR-01, AR-05,
  AR-16). The independent challenger agreed that widening the scope needs a
  user ruling.
- **Status**: ✅ Resolved — user boundary + named follow-up.

### PF-002 — Width constants assume a 32-bit `int` without a guard

- **Severity**: 🟠 MAJOR
- **Dimension**: Feasibility / portability
- **Evidence**: `cl_string_new(11/10)` and the `LONG`→`int` cast rely on
  32-bit `int`; no compile-time check exists.
- **Resolution**: add `#if INT_MAX != 2147483647 || UINT_MAX != 4294967295u`
  `#error` in both shims (`03-runtime-formatting.md` §Width guard; task 3.1.1
  and 3.1.2).
- **Status**: ✅ Resolved.

### PF-003 — Spare capacity would change `cl_string_append` behavior

- **Severity**: 🟠 MAJOR
- **Dimension**: Consistency / edge cases
- **Evidence**: base `cl_string_new` sets `cap == len` after `cl_string_from`;
  a max-width allocation would leave up to 7 spare bytes, so an internal
  append that used to trap would succeed.
- **Resolution**: set `string->cap = written` after the in-place write, so the
  logical capacity is exact and the append guard keeps its behavior
  (`03-runtime-formatting.md`; AR-10). Only the physical allocation slack
  remains, recorded in the findings.
- **Status**: ✅ Resolved.

### PF-004 — "No emitted-C shape change" was imprecise

- **Severity**: 🟡 MINOR
- **Dimension**: Consistency
- **Evidence**: the shim bodies are part of the emitted translation unit, so
  the C text changes even though the program/template shape does not.
- **Resolution**: `01-requirements.md` R4 and the index now say "no language
  or ABI change and no program-shape emission change; only the embedded shim
  converter bodies change".
- **Status**: ✅ Resolved.

### PF-005 — Amiga boundary values are compiled but not executed

- **Severity**: 🟡 MINOR
- **Dimension**: Testability
- **Evidence**: the host suite runs the boundaries; `make bench` exercises
  only the benchmark's values; `make differential` does not include
  `INT_MIN`/`UINT_MAX`.
- **Resolution**: task 4.1.6 builds a boundary program for `hosted-amiga`,
  runs it under `vamos`, and compares it with the host output.
- **Status**: ✅ Resolved.

### PF-006 — Duplicated audit tables across the plan documents

- **Severity**: 🟡 MINOR
- **Dimension**: Consistency / reference-don't-restate
- **Evidence**: the pre-plan tables appeared in both `02-current-state.md` and
  `05-findings.md`, and a summary table in `01-requirements.md`.
- **Resolution**: `02-current-state.md` owns the audit; `05-findings.md` cites
  it with a one-line gloss; `01-requirements.md` cites the attribution section.
- **Status**: ✅ Resolved.

### PF-007 — Impl tests were ordered before implementation

- **Severity**: 🟡 MINOR
- **Dimension**: Ordering & sequencing
- **Evidence**: the first draft put the impl suite in Phase 2.
- **Resolution**: the impl suite is written in Phase 3 after the implementation
  (task 3.1.4); the red case is ST-21 in Phase 2.
- **Status**: ✅ Resolved.

### PF-008 — Task-count and roadmap mismatches

- **Severity**: 🟡 MINOR
- **Dimension**: Consistency
- **Evidence**: the draft said 29 tasks after adding the Amiga run; the
  roadmap row said 0/29.
- **Resolution**: all counts now read 30; the roadmap row says 0/30.
- **Status**: ✅ Resolved.

### PF-009 — The float converter keeps `vfprintf` linked

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Feasibility / performance
- **Evidence**: `cl_text_f64` keeps `snprintf("%g")`, so the `vfprintf` code
  remains in `.text`; the probe still shows `.text` −0.5% on `strings`.
- **Resolution**: accepted — the float path is not hot and `%g` is the
  documented contract (AR-02). A manual `%g` is out of scope.
- **Status**: ✅ Accepted.

### PF-010 — Division loop vs lookup table

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Feasibility
- **Evidence**: the decimal loop calls libgcc division; a 100-entry table
  could trade `.text`/data for fewer divisions.
- **Resolution**: accepted — the measured probe beats `snprintf`; a table is
  larger and not justified by the benchmark (AR-07).
- **Status**: ✅ Accepted.

### PF-011 — ST-21 program's unused locals warn

- **Severity**: 🟡 MINOR
- **Dimension**: Testability
- **Evidence**: the probe source declared `s`, `t`, `u` and did not read them,
  producing Clorraine `W10000` warnings.
- **Resolution**: the test consumes them through `len(...)` (no formatting)
  before printing the counter; the expected count is unchanged.
- **Status**: ✅ Resolved.

### PF-012 — `--wrap=snprintf` is GNU-linker specific

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Feasibility
- **Evidence**: the host test links with GNU ld on Linux; the repo's host CI
  is Linux and the suite is host-only.
- **Resolution**: accepted; the m68k leg is covered by task 4.1.6 and the
  benchmark. If a non-GNU host is ever supported, the test can be gated by a
  build probe.
- **Status**: ✅ Accepted.

## Verdict

`✅ PREFLIGHT PASSED WITH NOTES — 8 resolved, 3 accepted`

- No 🔴 or 🟠 finding remains unresolved.
- The three 🔵 observations are accepted with reasons.
- The plan is ready for execution after the plan-document commit.
