# Phase 3: Call, Member, and CString Surfaces

> **Document**: 03-03-call-member-cstring-surfaces.md
> **Parent**: [Index](00-index.md)

## Overview

Four surfaces where the checker accepts a call the frozen rules reject or misses a rejection
the registry already names: the `cstring` view crossing (DEF-144), the call arity (DEF-147,
plus the plan-created direct-call twin DEF-213 — **gated on the AR-2 user ruling**), the
unknown member callee (DEF-148), and the `toCString` stdlib widening (DEF-154).

## Current Arc

- **DEF-144** — `check_args_impl` (`sema/call.rs:575-605`) special-cases a `static str` literal
  at a `cstring` parameter (accepted, emitted directly) and otherwise runs the ordinary
  conversion check. A `String` handle at a `cstring` parameter builds (the emission resolves
  the handle's pointer), but a `str` view or a slice value is **also accepted** and emitted as
  its view struct where `char *` is declared — the strict build fails (probed). The frozen
  conversion table (`types/mod.rs:595`) defines no view→`cstring` implicit conversion.
- **DEF-147/DEF-213** — `check_args_impl`'s unequal-length branch (`sema/call.rs:601-605`)
  checks the argument values against no expected type and reports nothing; the comment at
  `:536-538` records the deliberate status ("An arity mismatch is left alone: no diagnostic in
  this phase owns it"). The frozen registry has no value-argument code; Ch 04 §8 requires a
  dot-call candidate to match the argument list. **The phase waits for the AR-2 ruling
  (recommended: a new `E10308` row).**
- **DEF-148** — `check_dot_call`'s `None` arm (`sema/call.rs:440-470`) reports `E10212` only
  for a bare type-parameter field; a raw or plain-struct receiver whose name resolves to no
  field, function, or intrinsic falls through silently and the emitter produces an implicit
  call to an undeclared symbol (probed for both receiver kinds).
- **DEF-154** — `stdlib/std.text.clo:45` declares `toCString(s: String)` while
  `spec/appendix-stdlib.md:249` and `spec/08-arrays-strings.md:202` document `String` or
  `str`; `toCString(view)` reports `E10200`. The String→`str` implicit widening
  (`types/mod.rs:610`, PF-022) keeps String callers working under a widened signature, and
  the standard library already reads a `str` view's pointer in `print(s: str)`
  (`stdlib/std.io.clo:64`).

## Proposed Changes

> **Reverted (Phase 3 fix round):** an attempt to cross a view at a `cstring` parameter as
> `(char *)view.ptr` was reverted after the semantics review: a `cstring` parameter is
> contracted (an uncontracted parameter is a mode-less handle or aggregate, Ch 12 §117-118),
> CN-9 checks the declared types, and the conversion table is exhaustive — so the rejection
> below stands, extended to a `raw` or fixed-array argument by the same rule. See the ledger's
> fix round.

### 1. Reject a view at a `cstring` parameter (DEF-144) — `sema/call.rs`

In the `cstring`-parameter argument path, keep the literal allowance and the `String`-handle
case, and report `E10200` (`cannot convert 'str' to 'cstring' implicitly`) for a `str` or
slice argument; the help names the explicit route (`toCString`). One report per site; the
accepted shapes' emission is untouched.

### 2. Check the argument count (DEF-147/DEF-213) — `sema/call.rs` *(AR-2-gated)*

Under the ruled code: the dot-call candidate path checks the written argument count against
the remaining parameter list (the receiver is not written), and the direct-call path against
the whole list; one report per site at the call span. The matched case, the markers, and the
out-slot rules are unchanged.

### 3. Report every unresolved member callee (DEF-148) — `sema/call.rs`

The `None` arm reports `E10212` (`'{name}' is not a member of '{type}'`) for every receiver
kind whose surface resolves nothing — raw, plain struct, and the existing bare-parameter case
— one report per site. A receiver that resolves a non-callable field keeps today's behavior
(the separate filed class DEF-214).

### 4. Widen `toCString` (DEF-154) — `stdlib/std.text.clo`

Declare `toCString(s: str): cstring` and adapt the body to read the view's pointer the way the
standard library already reads `s.ptr`; the in-phase probe pins the exact body so a `String`
caller's emission is byte-identical (the implicit widening provides the view). Literal,
`String`, and view arguments all build and run.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `cstringliteral` | a literal at a `cstring` parameter | accepted; byte-identical emission |
| `cstringowned` | a `String` handle | one `E10200` (revised in the fix round) |
| `cstringview` | a `str` variable (DEF-144) | one `E10200` |
| `cstringslicearg` | a `u8[]` value | one `E10200` |
| `cstringraw` | a `raw u8` value | one `E10200` (added in the fix round) |
| `dotarity` | `x.addOne(5)` (DEF-147) | one arity report (AR-2 code) |
| `directarity` | `f(1, 2)` (DEF-213) | one arity report (AR-2 code); the generic (`g<i32>(1, 2)`) and `ffi` twins join the same rule |
| `arityok` | exact counts, both forms | accepted; byte-identical emission |
| `memberunknown` | `s.foo(5)` (DEF-148) | one `E10212` |
| `memberraw` | `p.foo(5)` on a `raw` receiver | one `E10212` |
| `memberfield` | a callable field and a same-named free function | accepted (the field-wins rule) |
| `cstringviewconv` | `toCString(view)` (DEF-154) | accepted; builds and runs |
| `cstringownedconv` | `toCString(string)` | accepted; byte-identical emission |

## Acceptance

- Each row's reproduction reports exactly the pinned diagnostic (or builds and runs); every
  case failed at the phase baseline.
- The references (`cstringliteral`, `arityok`, `memberfield`) keep their own emission; the
  `toCString` cases (`cstringviewconv`, `cstringownedconv`) carry the intended stdlib delta,
  and `cstringowned` (`String` at a `cstring` parameter) reports `E10200` (revised in the fix
  round, with the register's AR-15 note reverted).
- The stdlib change re-emits only `std.text`'s `toCString` shape; the corpus differential
  lists it as the one intended delta.
