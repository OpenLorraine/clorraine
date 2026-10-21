# Current State — Global Borrow and Move Conformance (Batch U)

Baseline `513fd90`.

## `DEF-533` — the module-qualified carve-out and unchecked view sources

The body checker exempted a module-qualified receiver from `E10038`
(`compiler/src/own/call_check.rs`, the `!receiver` guard). The initializer walk
had the same exemption plus an unmarked scalar/view exemption
(`compiler/src/own/program.rs`). The view builders (`slice`, `bytes`,
`toCString`, `rawSlice`, `rawStr`) were unchecked in an initializer. Frozen
BR-9 (`spec/16-borrow-rules.md:195`) says a global may not be borrowed.

The conflicting oracles were the module-qualified member-call case, the four
initializer cases (`slice(xs)`, `v(sl)`, `a.add1()`, `maybe(d)`), and several
emission pins that used a global array as a module-level `slice` source.

## `DEF-537` — a func-typed initializer call

`let f: func(N): i32 = ...; let g = f(n);` and `h.cb(n)` pass a global to an
unmarked parameter without a report. The initializer walk resolves named
user/`ffi` functions; it does not read a func-typed global's or field's
signature.

## `DEF-425` — a generic element move

`copy<T>(borrow w: T, i, j) { w.items[i].name = w.items[j].name; }` is accepted
while the concrete `borrow` twin reports `E10034` and the `take` twin
`E10026`. The pinned batch-M acceptance cases are in
`conditional_projection_residue_spec_test.rs` and
`conditional_projection_residue_impl_test.rs`; the fix direction is the
`RequiresNotLend` projection path (`project_place_path` +
`check_not_lend_requirements`).
