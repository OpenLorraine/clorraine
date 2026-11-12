# Requirements: Generated-C Robustness Bundle

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Functional Requirements

### Must Have

- [ ] **R1 — Module-path precedence.** `std.fmt.text(v)` is the intrinsic,
  or a real `std.fmt.text` function/`ffi`/global declaration, when `std` is a
  module-level or imported value, in a function body **and** in a module
  initializer. The module path is
  always available (MOD-13); only a lexical `std` binding may shadow it
  (MOD-16).
- [ ] **R2 — Lexical and global member paths preserved.** A lexical `std` value
  with a `fmt.text` field keeps the member-call behavior. A module-level global
  whose head is not a module path (`box.text(7)`) keeps its member call.
- [ ] **R3 — Owned text argument released.** `text(stringOf("x"))`,
  `text($"{s}")`, and their alias/qualified spellings do not leak the temporary
  argument in a function body. A named-place argument
  (`let s = ...; text(s);`) is a copy and stays readable after the call. The
  module-initializer path already releases its argument and must stay green.
- [ ] **R4 — Init-local reservation.** A user `ffi __cl_itmp0` does not collide
  with a module-initializer temporary; the generated name advances past taken
  symbols. Ordinary programs keep `__cl_itmp0` when it is free.
- [ ] **R5 — Initializer parity.** The module-initializer walk applies the same
  `std.fmt.text` precedence as a function body: a module-level or imported
  `std` value does not hide the module path. The oracle uses a discriminating
  argument (`text(1234)` → length 4, not the member's `"u"` length 1).
- [ ] **R6 — No regression except the intended pin update.** The #24/#26/#27/
  #28/#29 diagnostics and emission pins stay green, except the #29 IT-13
  module-level value-head pin, which #31 supersedes and this bundle updates.
  The conformance suites and generated-C shape stay green.

### Won't Have (Out of Scope)

- Any `spec/`, `stdlib/`, or ABI edit.
- A general revision of module-vs-value precedence; only the
  `std.fmt.text`/module-path case is corrected.
- The deferred-duty simple-name resolution and the right-literal null-at-scalar
  shape (recorded by #29).
- A new diagnostic or runtime helper.

## Technical Requirements

### Performance

- The changes are one name scan, one synthetic mode, an initializer reorder,
  and gate removals. No benchmark program uses a user `__cl_itmp0` ffi, an
  owned `text` argument, or a module-level `std` value.
- `make bench-compare` runs against `0fdb96c`; every scored output equal and no
  scored metric above a 1% regression.

### Compatibility

- R1/R2 follow the frozen module rules. The body spelling was previously invalid
  C; the initializer spelling previously resolved to the member field and is
  changed to the module path per MOD-13.
- R3 changes ownership of a temporary argument; named-place behavior is
  unchanged; the initializer already releases its argument.
- R4 changes a generated name only when a user symbol already occupies it.

### Security

- No new input, parser, FFI, or `unsafe` surface. No user data or secrets are
  involved.

## Acceptance Criteria

1. [ ] R1–R6 verified by the ST/IT cases in
   [07-testing-strategy.md](07-testing-strategy.md).
2. [ ] #16, #30, and #31 are closed with the delivered evidence.
3. [ ] `make check`, `make asan`, `make ubsan`, `make verify`,
   `make bench-shape`, and `make bench` + `make bench-compare` are green.
4. [ ] All scored outputs equal; no scored metric above 1%.
5. [ ] No critical/major review finding open.
