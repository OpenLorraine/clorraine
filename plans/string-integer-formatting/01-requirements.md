# Requirements: String Integer Formatting Throughput

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Revision**: final — all ambiguity-register items resolved

## Feature Overview

Issue #19 identifies the `strings` benchmark as the largest remaining runtime
cost in the scored suite: 2,964,051 net m68k instructions, about 3.6× the
next benchmark. The issue asks for a measurement audit first, then one
measured change inside the standard library and runtime shims, with no
language source behavior change and no ABI break.

The audit (`02-current-state.md` §Attribution) shows the cost is dominated by
C library formatting: the `snprintf` family holds 65.4% and `malloc`/`free`
only 11.8%. Two `cl_text_u32` calls per iteration each format through
`snprintf("%u")`
into a stack buffer, allocate a `String`, and copy the bytes. Replacing only
the integer conversion with a bounded decimal writer that fills the allocated
block in place removes the dominant cost and stays inside the issue's
boundary.

The pre-plan probe measures `strings` at 1,464,668 net instructions
(−50.6%), with unchanged output, 6,003 allocations (unchanged), and a smaller
`.text`. The other four scored benchmarks improve or stay within 0.06% of
baseline. The residual allocation work is recorded for a follow-up issue.

## Scope Boundary

**In scope:** the two `String` runtime shims (`shim_host.c`,
`shim_hosted_amiga.c`); the `cl_text_i32` and `cl_text_u32` converter bodies;
their shared decimal helper; behavior and implementation tests for every
`i32`/`u32` boundary and template shape; the benchmark base/head measurement;
the sanitizer and strict-C legs; the phase reviews; and the documentation,
roadmap, and follow-up-issue closeout.

**Out of scope:** the template lowering and `ClStrPart` builder; direct
formatting into the template destination; eliminating the per-template
allocations; `cl_text_f64`/`cl_text_bool`/`cl_text_str`/padding/`cl_text_hex_u32`
behavior; the standard-library `.clo` sources; language syntax, semantics,
grammar, diagnostics, ABI, and the frozen `spec/`; codegen, IR, symbols, and
emitted C shape; the freestanding profile; new crates, build scripts, harness
code, or toolchain flags; and safety trade-offs.

Authoritative boundary: issue #19's "Scope and constraints" section. The
codegen lowering is a measured follow-up option, not planned work (AR-01,
AR-05, AR-16).

## Functional Requirements

### R1 — Integer conversion without `snprintf`

- `cl_text_i32` and `cl_text_u32` must not call `snprintf`, `vsnprintf`,
  `sprintf`, or any other C library formatter.
- The decimal text must be byte-identical to the prior `%d`/`%u` output for
  every 32-bit value, including `0`, `1`, `-1`, `2147483647`,
  `-2147483648`, and `4294967295`.
- A program that formats integers through templates, `text`, and
  `toDecimal` must make zero `snprintf` calls (AR-03, AR-08).

### R2 — One allocation, written in place

- Each conversion allocates exactly one `String` block through
  `cl_string_new`, as before.
- The digits are written into the block's payload; no stack buffer and no
  `cl_string_append` copy is used.
- The block keeps the `[len][cap][bytes][NUL]` invariant: `len` is the number
  of written bytes, `payload[len] == '\0'`, and `len == cap`.
- The block is allocated at the maximum decimal width (11 for `i32`, 10 for
  `u32`); after the write, `cap` is reset to the written count so the logical
  capacity stays exact and `cl_string_append` keeps its old behavior
  (AR-04, AR-06, AR-10).

### R3 — Unchanged conversion behavior

- `{str}`, `{String}`, `{bool}`, `{f64}`, and literal template parts keep
  their current output and ownership behavior.
- `cl_text_f64` keeps `%g` through `snprintf`; `cl_text_bool`,
  `cl_text_str`, `cl_text_hex_u32`, and the padding helpers are unchanged.
- `std.fmt.toDecimal`, `std.fmt.toHex`, `text`, and `println` keep their
  signatures and output (AR-02).

### R4 — Profile and toolchain parity

- The host and hosted-amiga shims implement the same helper and the same
  observable behavior, using each file's native types.
- The emitted C still compiles warning-free under the host strict flags,
  m68k GCC `-Wall -Wextra -Werror`, `hosted-amiga-13`, vbcc, and the
  freestanding build leg.
- No language, ABI, or emitted-C-shape change (AR-09, AR-20).

### R5 — Measured win and no scored regression

- `strings` net instructions must drop by at least 30% against the clean
  `8f07e40` base under the shipped flags.
- No scored benchmark may exceed the 1% net-instruction or `.text`
  regression guard, and every output hash must match.
- Record the base/head table and the allocation count in `05-findings.md`
  (AR-12, AR-13).

### R6 — Memory safety and sanitizers

- The new suite runs under AddressSanitizer with leak detection and under
  UndefinedBehaviorSanitizer with `-fno-sanitize-recover=all`.
- The capacity arithmetic and the `INT_MIN` magnitude are reviewed by the
  memory-safety auditor (AR-14, AR-19).

### R7 — Documentation

- The shim comments state the block invariant, the maximum width, the
  `INT_MIN` handling, and why the write is in bounds.
- `05-findings.md` carries the attribution, the probe, the final table, and
  the residual.
- The roadmap and `docs/known-limitations.md` (if a limit is user-visible)
  are updated (AR-15).

### R8 — Closeout

- `make check`, `make asan` (new suite), `make ubsan` (new suite),
  `make bench-shape`, the benchmark comparison, and `make verify` are green.
- A follow-up issue records the codegen direct-to-destination option with the
  probe evidence (AR-16, AR-17).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Which layer changes | Shim only; codegen lowering; both | Shim only | Issue #19's explicit boundary; shim removes 65% of the measured cost | AR-01 |
| Integer algorithm | `snprintf`; manual decimal; lookup table | Manual decimal | Removes the dominant library path; small bounded code | AR-02, AR-03, AR-07 |
| Write target | Stack buffer + copy; direct into block | Direct into block | Measured 3.2% further win and no allocation change | AR-06 |
| Capacity | Exact length; maximum width | Maximum width | Avoids a second division pass; ≤7 spare bytes | AR-06, AR-10 |
| Float formatting | Manual `%g`; keep `snprintf` | Keep `snprintf` | Preserves the documented six-significant-digit contract; not hot | AR-02 |
| Allocation churn | Fix now; follow-up | Follow-up | Outside the issue's scope; recorded with evidence | AR-05, AR-16 |

> **Traceability:** every decision cites its owning Ambiguity Register entry.

## Acceptance Criteria

1. [ ] `cl_text_i32`/`cl_text_u32` contain no `snprintf` call and the
       `--wrap=snprintf` probe counts zero calls for an integer-only program.
2. [ ] Every ST case in `07-testing-strategy.md` passes on host; the template
       outputs are byte-identical to the base.
3. [ ] The zero-formatter-call spec case fails on the base compiler (red) and
       passes after the change (green); the behavior cases pass on both, with
       the no-change oracle justified in `05-findings.md`. The implementation
       suite is written after the change.
4. [ ] ASan and UBSan legs are clean.
5. [ ] `make bench-compare` is green; `strings` net instructions improve by
       at least 30%; no scored benchmark regresses above 1% in instructions
       or `.text`.
6. [ ] `make check`, `make bench-shape`, and `make verify` are green.
7. [ ] `05-findings.md` records the attribution, probe, final table, and
       residual; the follow-up issue exists.
8. [ ] No `spec/` or `stdlib/` file changes.
