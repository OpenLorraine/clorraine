# Ambiguity Register: Direct Template Formatting

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design (eligible technical decisions delegated by the
> session's `--auto-design` authorization)
> **Root invocation ID**: `template-direct-2026-11-11`
> **Policy version**: 1
> **Status**: ✅ GATE PASSED — all 12 items resolved
> **Last Updated**: 2026-11-11 12:00

Auto-design active — eligible technical decisions are delegated and recorded.
The scope is user-owned: the active objective requires #19's allocation-churn
component, and the explicit next step is to implement #25 under the full gate.

## Decision Records

### AR-01 — Implement #25, not a second shim-only pass

- **Status**: ✅ Resolved
- **Authority**: User — the active directive states that the shim-only change
  left the allocation-churn component unresolved and names the next step:
  "Implement the measured codegen lowering tracked in #25 ... or obtain an
  explicit user ruling". No ruling narrowed the goal, so #25 is the work.
- **Decision**: change the template lowering and the two `String` shims so a
  template with integer placeholders allocates exactly one `String`; close
  #25 and #19 only after the full gate.
- **Evidence**: the pre-plan probe measured 6,003 → 2,002 allocations and
  1,489,187 → 1,132,051 net instructions on `strings`
  (`plans/string-integer-formatting/02-current-state.md` §Probes).
- **Rejected alternatives**: keep the shim-only delivery and ask for a scope
  ruling (the reviewer already identified it as incomplete); a second
  shim-only trick (the temporary `String` must be independent, so it cannot
  be pooled safely).
- **Strongest counterargument**: the codegen change is larger and changes the
  emitted C for every template; the emission-integrity and memory-safety
  reviews must carry it.
- **Confidence**: High — the probe is measured and the scope is directed.
- **Hardening**: one independent design challenger already assessed both
  options in PLAN-28; the reviewer directive resolves the scope. The full
  emission, sanitizer, and benchmark gates are mandatory.
- **Reopen triggers**: the final measurement shows no allocation or
  instruction win, or the user narrows the goal back to shim-only.

### AR-02 — Part representation

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: internal data structure in the shim builder.
- **Objective**: carry scalar values and byte ranges in one array without a
  temporary `String`.
- **Decision**: keep `ClStrPart` at 12 bytes with
  `{ const unsigned char *ptr; unsigned int len; union { void *owned; int i;
  unsigned int u; } value; }`. A scalar piece has `ptr == NULL` and `len` set
  to the kind code (0 = i32, 1 = u32); a byte piece has a pointer and length;
  an owned byte piece (a formatted float) also carries the handle.
- **Evidence**: the probe measured 1,132,051 net instructions and 2,002
  allocations with this shape; the 12-byte size preserves the existing
  compound-literal footprint.
- **Rejected alternatives**: a 20–24-byte tagged union (larger frame, measured
  worse on `matrix` in the earlier probe); a second parallel array (two
  evaluation lists).
- **Strongest counterargument**: encoding the kind in `len` is less obvious
  than a named field. The helper comments and the emitted `CL_STR_KIND_*`
  names make it explicit.
- **Confidence**: High.
- **Hardening**: the emission-integrity and memory-safety reviews.
- **Reopen triggers**: a reviewer finds the encoding ambiguous or a profile
  rejects the designated union initializer.

### AR-03 — Which placeholders use which part

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: lowering mechanism for existing formattable types.
- **Decision**: `i8/i16/i32/isize` → kind 0; `u8/u16/u32/usize` → kind 1;
  `str`/`static str` → a borrowed byte part; `String` → a borrowed byte part
  through `cl_str_part_string` (no copy); `bool` → `cl_str_part_bool`
  (static "true"/"false" bytes); `f32/f64` → `cl_str_part_f64` (the existing
  `%g` converter, owned and freed by the builder). The fallback for any other
  handled type keeps `cl_str_part_owned(cl_text_...)`.
- **Evidence**: the spec's formattable set is exactly these types
  (`spec/04-expressions.md` §12); the existing `text_call` already normalizes
  the integer widths.
- **Rejected alternatives**: hand-formatting floats into the destination
  (would change the `%g` contract); copying a `String` placeholder (the
  allocation this feature removes).
- **Strongest counterargument**: borrowing a `String` placeholder changes the
  old copy; the source is still alive through the full expression and the
  result is independent because the builder copies the bytes.
- **Confidence**: High.
- **Hardening**: ownership and lifetime probes; the existing owned-placeholder
  test.
- **Reopen triggers**: a probe shows a dangling source or a source that is
  consumed.

### AR-04 — Allocation contract

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: performance mechanism.
- **Decision**: a template with no float placeholder makes exactly one
  allocation (the result). A template with a float placeholder makes two (the
  `%g` temporary plus the result); the temporary is freed by the builder.
- **Evidence**: the probe's `strings` count is 2,002 total allocations
  (2,000 results + 2 outside the loop), down from 6,003.
- **Rejected alternatives**: pooling the float temporary (unsafe if the result
  escapes); hand-formatting floats (contract risk).
- **Strongest counterargument**: float templates keep an extra allocation;
  floats are rare and the documented `%g` output is preserved.
- **Confidence**: High.
- **Hardening**: the allocation-count probe and the findings table.
- **Reopen triggers**: the harness or a reviewer shows a missing free.

### AR-05 — Capacity and block invariant

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: internal allocation invariant.
- **Decision**: the builder reserves the sum of the piece capacities (11 for
  i32, 10 for u32, the byte count for bytes), writes in one pass, then sets
  `len` and `cap` to the written count and writes the NUL. A later
  `cl_string_append` still traps because `cap == len`.
- **Evidence**: the existing `cl_string_new` layout and the PLAN-28 capacity
  proof; the probe's direct-write builder.
- **Rejected alternatives**: exact decimal lengths (a second division pass);
  leaving `cap` at the maximum (changes the append trap behavior).
- **Strongest counterargument**: the physical block has up to 7 spare bytes;
  the logical capacity is exact.
- **Confidence**: High.
- **Hardening**: memory-safety review and ASan.
- **Reopen triggers**: a block-layout change or an append probe.

### AR-06 — No language or ABI change

- **Status**: ✅ Resolved
- **Authority**: User — issue #19 and #25 both forbid a language source
  behavior change and an ABI break.
- **Decision**: keep every `cl_text_*`/`cl_string_*` signature; keep the
  template's owned-`String` result and evaluation order; change only the
  emitted builder call and the shim-internal `ClStrPart`.
- **Evidence**: the template is compiler-lowered, not a user-facing ABI; the
  result type and the placeholder types are unchanged.
- **Rejected alternatives**: a new public builder API (ABI surface).
- **Strongest counterargument**: the emitted C changes for every template;
  the emission-integrity auditor verifies no program body changes beyond the
  call.
- **Confidence**: High.
- **Hardening**: `spec/` unchanged; the emission audit.
- **Reopen triggers**: a spec test changes or a reviewer finds an ABI impact.

### AR-07 — Test strategy

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: test strategy.
- **Decision**: new `template_direct_formatting_spec_test.rs` covers the
  boundary outputs again (the existing suites stay), the allocation delta
  around a template (1 vs 3 for two integer placeholders), the `String`/`str`
  borrow, the float two-allocation case, and ASan/UBSan. New
  `template_direct_formatting_impl_test.rs` pins the emitted parts, the
  absence of `cl_text_i32/u32(` in template calls, and the unchanged
  `text`/`toDecimal` converters.
- **Evidence**: the existing `string_integer_formatting` suites are the
  behavior oracle for the converters and stay green.
- **Rejected alternatives**: modifying the immutable PLAN-28 suites (they are
  the oracle).
- **Strongest counterargument**: the allocation test needs a wrapped
  `malloc`; the host is GNU/Linux and the suite is host-only.
- **Confidence**: High.
- **Hardening**: red on the base, green on the head; ASan/UBSan.
- **Reopen triggers**: a test proves brittle or a formattable type is missed.

### AR-08 — Benchmark baseline

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design
- **Eligibility**: measurement method.
- **Decision**: base is the current `main` head `00eb6bc` (the shim-only
  formatter) built clean; head is the feature commit. The harness flags,
  runs, calibration, and 1% guard are unchanged.
- **Evidence**: `8f07e40` → `00eb6bc` already records the shim win; this plan
  isolates the lowering.
- **Rejected alternatives**: base `8f07e40` (mixes the two changes).
- **Strongest counterargument**: the base build costs time; it isolates the
  attribution.
- **Confidence**: High.
- **Reopen triggers**: the harness identity or flags drift.

### AR-09 — Review dispatch

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design (project policy).
- **Decision**: this is a codegen/emission change, so dispatch the
  correctness reviewer, the memory-safety auditor, and the read-only
  emission-integrity auditor. The semantics reviewer is not dispatched: no
  language rule, lend, capture, or evaluation-order rule changes.
- **Evidence**: `AGENTS.md` "CodeOps" phase-review dispatch.
- **Confidence**: High.
- **Reopen triggers**: the lowering changes a language rule.

### AR-10 — Verification commands

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design (project convention).
- **Decision**: `make check`, `make asan`, `make ubsan`, `make bench-shape`,
  `make bench-compare`, and `make verify`; the new suites run under both
  sanitizer legs.
- **Evidence**: `AGENTS.md`.
- **Confidence**: High.

### AR-11 — Closeout

- **Status**: ✅ Resolved
- **Authority**: User — the directive says deliver #19; #25 is the named
  residual.
- **Decision**: close #25 and #19 after the final gate, update the roadmap to
  Done, and record the final table in `05-findings.md`.
- **Evidence**: `gh issue view 19` and `25` are open.
- **Rejected alternatives**: close #19 now with the shim-only note (the
  directive says that is incomplete).
- **Confidence**: High.
- **Reopen triggers**: the final gate fails.

### AR-12 — Naming and files

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by --auto-design.
- **Decision**: plan folder `plans/template-direct-formatting/`; tests
  `compiler/tests/template_direct_formatting_spec_test.rs` and
  `_impl_test.rs`; shim names `cl_str_part_string`, `cl_str_part_bool`,
  `cl_str_part_f64`, `CL_STR_KIND_I32`, `CL_STR_KIND_U32`.
- **Evidence**: the repo's `<feature>_spec_test.rs` convention.
- **Confidence**: High.
