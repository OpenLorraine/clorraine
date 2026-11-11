# Ambiguity Register: String Integer Formatting

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design (eligible technical decisions delegated by the
> session's `--auto-design` authorization)
> **Root invocation ID**: `string-format-2026-11-11`
> **Policy version**: 1
> **Status**: ✅ GATE PASSED — all 16 items resolved
> **Last Updated**: 2026-11-11 09:39

Auto-design active — eligible technical decisions are delegated and recorded.
Product scope, acceptance criteria, and the issue boundary remain user-owned.
Issue #19 states the scope explicitly: standard-library and runtime-shim work
only, no language source behavior change, no ABI break.

## Decision Records

### AR-01 — Which part of #19 is in scope

- **Status**: ✅ Resolved
- **Authority**: User — issue #19 "Scope and constraints" says
  "Standard-library and runtime-shim work only; no language source behavior
  change and no ABI break."
- **Objective**: cut the measured runtime cost of integer formatting without
  changing the language or the emitted C shape.
- **Decision**: in scope are the integer text converters in the two profiles
  that own a `String`: `cl_text_i32` and `cl_text_u32` in
  `compiler/runtime/shim_host.c` and
  `compiler/runtime/shim_hosted_amiga.c`. The template lowering and the
  `ClStrPart` builder are out of scope.
- **Evidence**: the measurement audit in `02-current-state.md` attributes
  1,940,000 of 2,964,051 net instructions (65.4%) to the `snprintf`
  family, and only 348,677 (11.8%) to `malloc`/`free`. Removing
  `snprintf` from the integer path is the dominant lever and stays inside
  the stated boundary.
- **Rejected alternatives**: a codegen lowering that formats placeholders
  directly into the destination buffer (measured a further 332,617
  instructions on `strings`, and 6,003 → 2,002 allocations). It changes the
  emitted C for every template, so it exceeds the issue's scope. It is
  recorded as a follow-up issue (AR-16), not implemented here.
- **Strongest counterargument**: the issue title names allocation churn, and
  this plan leaves the per-iteration allocation count unchanged. The
  measured attribution shows `snprintf` is 5.5× the allocation cost, so the
  smaller change still removes the dominant cost; the residual is named and
  tracked.
- **Confidence**: High (0.85) — the probe reproduces on the scored benchmark
  and the attribution buckets agree; medium that the issue author intended
  allocation elimination.
- **Hardening**: one blind independent design challenger reviewed both
  options without the parent's preference. Verdict: Option A is legitimate
  under the literal scope but is a partial delivery of the title; Option B is
  the better engineering decision only after a user scope ruling. The
  challenger's risks (32-bit width assumption, `INT_MIN`, capacity slack,
  Amiga boundary execution, retained-memory slack) are addressed by the width
  guard, the `cap = written` reset, the ST boundary cases, task 4.1.6, and the
  findings note.
- **Reopen triggers**: the final harness shows no `strings` win, or the user
  rules that the codegen lowering belongs in #19.

### AR-02 — Which formatters change

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: implementation mechanism inside the accepted formatting
  boundary; no product or scope change.
- **Objective**: remove `snprintf` from the hot integer path only.
- **Decision**: `cl_text_i32` and `cl_text_u32` use one shared decimal writer.
  `cl_text_hex_u32` already writes digits directly and is unchanged.
  `cl_text_f64` keeps `%g` through `snprintf`, because the documented
  six-significant-digit conversion is the C library's contract and floats
  are not in the hot path. `cl_text_bool`, `cl_text_str`, and the padding
  helpers are unchanged.
- **Evidence**: `strings` formats two `u32` placeholders per iteration; the
  `f64`/`bool`/`str` converters are not called by the scored suite.
- **Rejected alternatives**: hand-rolling `%g` (risk of a different digit
  string; out of scope); changing `toHex` (already manual).
- **Strongest counterargument**: leaving `%g` on `snprintf` keeps the
  `vfprintf` code linked, so `.text` does not drop as much as it could.
  Runtime is the measured objective, and the float path is rare.
- **Confidence**: High.
- **Hardening**: the review packet checks that no other converter silently
  changed.
- **Reopen triggers**: a scored benchmark turns out to format floats in its
  hot loop.

### AR-03 — Decimal output must be byte-identical

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: correctness mechanism for an accepted behavior.
- **Objective**: preserve the exact `%d` / `%u` output contract.
- **Decision**: emit an optional `-`, then the magnitude in decimal with no
  leading zeros, using `0u - (unsigned int)value` for the magnitude. This
  covers `INT_MIN` without signed overflow and matches `%d`. `u32` values
  print as unsigned.
- **Evidence**: C99 `%d` on a 32-bit two's-complement target prints
  `-2147483648` for `INT_MIN`; the unsigned magnitude is the standard
  portable spelling. `%u` has no sign.
- **Rejected alternatives**: `labs`/`abs` (undefined for `INT_MIN`);
  negating in signed arithmetic (undefined for `INT_MIN`); a lookup table of
  all values (absurd).
- **Strongest counterargument**: a manual formatter can diverge on a
  boundary; the ST cases cover 0, ±1, ±max, `INT_MIN`, and `UINT_MAX`.
- **Confidence**: High.
- **Hardening**: the spec suite runs the boundaries on host and on m68k via
  the benchmark/differential legs.
- **Reopen triggers**: any output hash differs in the base/head comparison.

### AR-04 — Allocation failure and error behavior

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: unchanged failure mechanism.
- **Objective**: preserve the allocator contract.
- **Decision**: the converters still allocate exactly one block through
  `cl_string_new`, which traps with `error E1: out of memory` on failure.
  The direct-write path adds no new error branch.
- **Evidence**: `cl_string_new` already traps for a failed allocation and
  rejects a capacity that cannot fit the modeled 32-bit size.
- **Rejected alternatives**: none viable.
- **Strongest counterargument**: bypassing `cl_string_append` removes its
  capacity trap; the fixed maximum-width capacity makes the write provably
  in bounds.
- **Confidence**: High.
- **Hardening**: the memory-safety auditor checks the capacity arithmetic
  and runs ASan.
- **Reopen triggers**: a sanitizer report or a reviewer finds an
  out-of-bounds write.

### AR-05 — Allocation-churn residual

- **Status**: ✅ Resolved
- **Authority**: User — issue #19 scope limits this plan to the shim.
- **Decision**: this plan does not change the number of allocations per
  template. The measured 6,003 allocations for `strings` stay in the
  findings as the residual, and a follow-up issue records the codegen
  lowering that removes two of the three per-iteration allocations.
- **Evidence**: the codegen probe measured 2,002 allocations and 1,132,051
  net instructions, versus 1,464,668 for the in-scope shim change.
- **Rejected alternatives**: implementing the codegen lowering in this plan
  (scope).
- **Strongest counterargument**: the issue title mentions allocation churn;
  the follow-up keeps that work visible.
- **Confidence**: High.
- **Hardening**: the closeout verifies the follow-up issue exists.
- **Reopen triggers**: the user asks to expand #19.

### AR-06 — Write digits directly into the String block

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: internal allocation/copy mechanism.
- **Objective**: remove the stack buffer plus `cl_string_append` copy from
  each integer conversion.
- **Decision**: allocate the maximum width (`11` for `i32`, `10` for `u32`),
  write the digits into the payload, set `len` and `cap` to the written
  count, and write the trailing NUL. The physical block may have up to 7
  spare payload bytes; the logical `cap` is exact, so `cl_string_append`
  keeps its capacity-exceeded behavior.
- **Evidence**: the probe measured 1,512,668 net instructions with a stack
  buffer plus copy and 1,464,668 with direct writes (−3.2% of the base), and
  `.text` 24,092 → 24,080. Both keep the same one allocation.
- **Rejected alternatives**: exact-size formatting requires a digit-count
  pass and a second division loop; the probe shows it is slower. Keeping
  `cl_string_from` keeps the copy.
- **Strongest counterargument**: over-allocating up to 7 bytes per string
  increases peak memory slightly. The maximum object is 12 payload bytes,
  so the overhead is bounded and small. The logical `cap` is reset to the
  written count, so the block API's capacity behavior does not change.
- **Confidence**: High.
- **Hardening**: the memory-safety auditor checks the write bound; ASan
  covers the block.
- **Reopen triggers**: a memory profile shows the spare capacity matters, or
  a reviewer finds a length/NUL defect.

### AR-07 — Division cost on 68000

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: performance mechanism.
- **Objective**: confirm that a `% 10` / `/ 10` loop beats `snprintf` on the
  68000 despite the libgcc division calls.
- **Decision**: use the modulo/division loop. `snprintf` already performed
  the same division internally through `vfprintf`; the measured win proves
  the direct loop is cheaper.
- **Evidence**: `strings` net instructions 2,964,051 → 1,464,668; the
  `vfprintf`/`putc`/`ulldivus`/`udivsi3` buckets are removed from the hot
  path.
- **Rejected alternatives**: reciprocal multiplication (needs 32-bit
  multiply, which is also a libgcc call on 68000); a 100-entry digit table
  (more `.text` and data).
- **Strongest counterargument**: a 68000 division is slow; the measurement
  includes the division calls and still wins.
- **Confidence**: High.
- **Hardening**: the benchmark table is the gate.
- **Reopen triggers**: a different target profile shows the loop loses.

### AR-08 — Boundary and template cases

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: test strategy.
- **Objective**: prove the formatter on every boundary and every template
  shape that reaches it.
- **Decision**: cover `i32` `0`, `1`, `-1`, `2147483647`, `-2147483648`;
  `u32` `0`, `1`, `4294967295`; `usize` max; adjacent placeholders; literal
  text before/between/after; empty `str`; `String` placeholder; `bool`;
  `f64`; `toDecimal`; `toHex`; and a loop that formats 2,000 values.
- **Evidence**: the existing `codegen_run_spec_test` covers some template
  behavior but not the new boundaries.
- **Rejected alternatives**: rely on the benchmark checksum alone (too
  coarse).
- **Strongest counterargument**: many cases duplicate existing coverage;
  the boundary cases are new.
- **Confidence**: High.
- **Hardening**: the spec suite is the oracle; the ST table is in
  `07-testing-strategy.md`.
- **Reopen triggers**: a reviewer finds an uncovered formattable type.

### AR-09 — Profile parity and strict C

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: cross-profile integration.
- **Objective**: keep the host and hosted-amiga shims behaviorally identical
  and compile them under every accepted toolchain.
- **Decision**: implement the same helper and bodies in both shims, keeping
  each file's native types (`int`/`unsigned int` on host, `LONG`/`ULONG`
  signatures on Amiga). Freestanding has no `String` and is untouched.
  `make bench-shape` exercises GCC, vbcc, and the freestanding profile.
- **Evidence**: `compiler/src/profile.rs` embeds the three shims; the
  freestanding shim has no `cl_string_*`/`cl_text_*` symbols.
- **Rejected alternatives**: a shared header (the shims are standalone
  translation-unit text, and the host/Amiga types differ).
- **Strongest counterargument**: duplicated code can drift; the impl suite
  checks both emitted shims for the same helper names.
- **Confidence**: High.
- **Hardening**: `make check` and `make verify` cover both profiles.
- **Reopen triggers**: a profile compiles a different body or a strict C
  warning appears.

### AR-10 — String-block invariant

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: internal data-layout invariant.
- **Objective**: keep `[len][cap][bytes][NUL]` valid after a direct write.
- **Decision**: after writing `written` digits at the payload start, set
  `len = written` and `payload[written] = '\0'`. `cap` stays the maximum
  width (11 or 10), so `written <= cap` and the NUL fits the extra byte
  `cl_string_new` always reserves.
- **Evidence**: `cl_string_new(cap)` allocates `sizeof(block) + cap + 1` and
  writes the initial NUL. The maximum decimal lengths are 10 (`u32`) and 11
  (`i32` with sign).
- **Rejected alternatives**: call `cl_string_append` after a zero-length
  block (reintroduces the copy).
- **Strongest counterargument**: a future change to the block layout would
  need this path updated; the comment states the invariant.
- **Confidence**: High.
- **Hardening**: memory-safety review and ASan.
- **Reopen triggers**: the block layout changes.

### AR-11 — Security surface

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: security analysis of an internal numeric formatter.
- **Objective**: confirm no new input, injection, or data-protection surface.
- **Decision**: the only input is a 32-bit integer passed by value. There is
  no parsing, no format string from data, no allocation size from untrusted
  input, and no new file/network path. The block size is a fixed constant.
- **Evidence**: the converters take `int`/`unsigned int`; the format string
  is gone entirely.
- **Rejected alternatives**: none viable.
- **Strongest counterargument**: removing `snprintf` removes a
  well-audited library path; the replacement is a bounded 10/11-byte write
  with a proven capacity.
- **Confidence**: High.
- **Hardening**: correctness and memory-safety review.
- **Reopen triggers**: the converter gains a new input source.

### AR-12 — Acceptance thresholds

- **Status**: ✅ Resolved
- **Authority**: User — issue #19 requires a proven 10–30% instruction
  reduction and no scored regression above 1%.
- **Decision**: accept the change only if `strings` net instructions drop by
  at least 30% and every scored benchmark stays within the 1% instruction
  and `.text` limits, with identical outputs.
- **Evidence**: the probe measures `strings` −50.6%; the other four
  benchmarks −0.04% to −35.4%; `.text` −0.5% to −? on `strings` and smaller
  elsewhere.
- **Rejected alternatives**: landing a smaller win (does not justify the
  surface) or a larger codegen change (AR-01).
- **Strongest counterargument**: the 30% floor is stricter than the issue's
  lower bound; the probe is far above it.
- **Confidence**: High.
- **Hardening**: the final harness comparison is the gate.
- **Reopen triggers**: the final measurement misses the floor.

### AR-13 — Benchmark base and flags

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: measurement method.
- **Objective**: compare the same emission under the shipped flags.
- **Decision**: the base is the current `main` head `8f07e40` (`clc 1.1.2`)
  built from a clean tree; the harness uses its recorded 30 runs, host and
  m68k release flags, and the calibration subtraction. `make bench` writes
  `head.json`; `make bench-base COMMIT=8f07e40` writes the base document.
- **Evidence**: the harness records compiler identity and dirty state; the
  v1.1.2 release used the same method.
- **Rejected alternatives**: reusing `build/perf/base.json` (v1.1.1, not the
  current emission).
- **Strongest counterargument**: building the base from the same commit adds
  time; it removes the version drift.
- **Confidence**: High.
- **Hardening**: the harness compare rejects mismatched toolchains/flags.
- **Reopen triggers**: the compiler identity or flags differ.

### AR-14 — Native and sanitizer verification

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: verification strategy.
- **Objective**: prove the change on the host, the Amiga target, and under
  sanitizers.
- **Decision**: run the new host suites, `cargo test --workspace`,
  `make check`, `make asan` for the new suite, `make ubsan` for the new
  suite, `make bench-shape` (GCC, vbcc, freestanding), the benchmark
  comparison, and the full `make verify`.
- **Evidence**: the repo's `make asan`/`make ubsan` discover suites by their
  `RDC_ASAN`/`RDC_UBSAN` markers; `bench-shape` covers vbcc.
- **Rejected alternatives**: host-only verification (misses the target
  types).
- **Strongest counterargument**: `make verify` is long; it is the acceptance
  gate.
- **Confidence**: High.
- **Hardening**: the phase reviews check the exact commands.
- **Reopen triggers**: a gate leg fails.

### AR-15 — Naming and file layout

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: naming and internal structure.
- **Objective**: keep names consistent with the existing conventions.
- **Decision**: the plan folder is `plans/string-integer-formatting/`; the
  test suites are
  `compiler/tests/string_integer_formatting_spec_test.rs` and
  `compiler/tests/string_integer_formatting_impl_test.rs`; the shared
  helpers are `cl_format_u32` and `cl_format_i32` in each shim. The findings
  document is `05-findings.md`, matching the recent performance plans.
- **Evidence**: the repo's `<feature>_spec_test.rs` /
  `<feature>_impl_test.rs` convention; prior plans carry `05-findings.md`.
- **Rejected alternatives**: putting the plan under `stdlib/` (it is a
  compiler plan); naming the helper `cl_text_u32_into` (it formats, not
  appends).
- **Strongest counterargument**: none material.
- **Confidence**: High.
- **Reopen triggers**: a reviewer prefers a different name before the
  implementation starts.

### AR-16 — Residual work is a follow-up issue

- **Status**: ✅ Resolved
- **Authority**: User — the session rule says remaining optimization options
  are registered as GitHub issues rather than bundled.
- **Decision**: after the implementation lands, open a follow-up issue for
  the codegen lowering that formats template placeholders directly into the
  destination, with the probe evidence (6,003 → 2,002 allocations;
  `strings` 1,464,668 → 1,132,051 net instructions) and the scope note.
- **Evidence**: `gh issue list` shows #13–#23 are the deferred list; a new
  issue keeps the option visible.
- **Rejected alternatives**: bundling it (scope); dropping it (loses the
  measured residual).
- **Strongest counterargument**: another issue adds backlog; the evidence
  makes it cheap to pick up later.
- **Confidence**: High.
- **Hardening**: the closeout verifies the issue number.
- **Reopen triggers**: the user declines the follow-up issue.

### AR-17 — Full verify command

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design (project convention).
- **Decision**: the project's acceptance command is `make verify`, with
  `make check` as the fast gate; every plan Verify line names the matching
  subset and the final task names `make verify`. Commands run from the
  repository root inside the pinned container through the Makefile.
- **Evidence**: `AGENTS.md` "Commands" and "Testing" sections.
- **Confidence**: High.

### AR-18 — Base tree state

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design.
- **Decision**: the base is `8f07e40` on `main`, clean worktree, no untracked
  files outside `build/`; the plan's first task confirms it and records the
  compiler identity.
- **Evidence**: `git status --porcelain` is empty and `main == origin/main`.
- **Confidence**: High.

### AR-19 — Phase-review dispatch

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design (project policy).
- **Decision**: because the change writes into a heap block but does not
  change ownership, drop, IR, or emitted C shape, dispatch the correctness
  reviewer plus the read-only memory-safety auditor for the implementation
  phase. The emission-integrity auditor and semantics reviewer are not
  dispatched: no IR/codegen/symbol/language-rule change.
- **Evidence**: `AGENTS.md` "CodeOps" phase-review dispatch rules.
- **Confidence**: High.
- **Reopen triggers**: the implementation grows into codegen.

### AR-20 — No frozen-spec change

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design.
- **Decision**: no `spec/` chapter or golden changes. The template
  desugaring contract in `spec/17-codegen.md` §9 still holds: the builder is
  shim code and the allocation strategy stays there. The integer converters
  are shim internals.
- **Evidence**: the spec describes the builder concept, not the converter
  bodies; no spec test pins `snprintf`.
- **Confidence**: High.

## Resolution Notes

**AR-1:** The issue's scope boundary is the deciding authority. The codegen
probe is retained as evidence for the follow-up, not as planned work.

**AR-6:** The probe's direct-write variant is `build/perf/audit` evidence; the
implementation reuses the same shape in both shims.

**AR-12:** The 30% floor is stricter than the issue's 10–30% hypothesis and is
measured against the same harness the release uses.
