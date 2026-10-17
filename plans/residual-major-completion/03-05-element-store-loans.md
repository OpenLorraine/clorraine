# Element and Store Loans: Residual Major Completion

> **Document**: 03-05-element-store-loans.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are missing loan transfers on view-like stores: an element-read store loses the loan the
direct argument keeps, and the conditional/array-literal/single-view copy sources transfer no
element loans. Both are accepted programs with a host ASan heap-use-after-free; the frozen rule
requires the loan to follow the copied view, so the later out-slot store reports `E10030`. The
third row (`DEF-374`) is verified green and is the phase's control.

## Design

### 1. The element-read store (`DEF-372`)

`let s: str = xs[0];` where `xs[0]` holds a view derived from an owned `String` element
(`xs[0] = a.v;`) followed by `fill(s, borrow slot)` is accepted and reads freed storage (ASan
heap-use-after-free). The direct element argument `fill(xs[0], ...)` and the member twin
`fill(a.v, ...)` report one `E10030`. Frozen: Ch 16 §2
(`spec/16-borrow-rules.md:51-58`) — binding a local `str` view creates a shared loan of the
`String`, and "the loan ends at the last use of the derived view or any alias of it"; Ch 16 §6
(`:233`) — a use reachable only through the owner's release is rejected (`E10030`); Ch 16 BR-1
(`:187`).

Fix: the view-like store/let transfer adopts the source element's loan instead of dropping it:
when the stored value is a view read from an element (or a projection of one) that carries a
loan, record the loan against the destination binding so its later aliases and out-slot stores
are judged. The `let` transfer is `own/statements.rs:391-399` (the carrier-copy block); the
assignment spelling shares the same transfer. The direct argument's lend path and the whole-array copy transfer (`DEF-315`) are the
shape references. Expected: one `E10030` at the `fill` site, matching the direct and member
twins; no TU. The store itself and an in-scope read-back stay accepted.

### 2. The conditional and array-literal copy sources (`DEF-373`)

Four accepted copy shapes lose their source's element loans and read freed storage at the later
fill (ASan heap-use-after-free):

- `let ys: str[2] = (c ? xs : zs);` where `xs[0] = a.v;` / `zs[1] = b.w;` record the loans;
- `let ys: str[2] = [xs[0], xs[1]];` where the elements hold loans (**including the fill
  form**);
- `let s: str = c ? xs[0] : zs[0];` (the single-view conditional);
- a conditional whose arm is itself an array literal.

The mixed-position probe (`preflight/m373_union.clo`) has arm A loan `ys[0]` and arm B loan
`ys[1]`; both `fill` sites must report, but the current copy records no loans at all (run 38 +
ASan UAF). The direct twin `fill(c ? a.v : b.v, borrow slot)` reports one `E10030`; the
member-source, out-slot, return, and global destinations already reject (the row note's
controls). Frozen: the same Ch 16 §2 loan rule, Ch 03 §2
(`spec/03-ownership.md:70-71`: a view lend copies the view, and the storage it points into is
shared), and Ch 16 BR-4 (`spec/16-borrow-rules.md:190`: a move of the owner requires no live
loan). The copy itself is legal; the escape is the later out-slot store.

Fix: extend the `DEF-315` whole-array loan-transfer machinery to the copy sources, and write the
destination's loan record as a **union**: `move_element_loans` (`own/place.rs:1184`) snapshots the
source and replaces the destination record at `:1202` through `replace_element_loans` (`:1103`),
so a per-arm call would lose the first arm's positions. Snapshot **both arms** and write the
destination's union once; translate each element of an array-literal source (fill form included)
and both arms of a single-view conditional; a conditional whose arm is an array literal unions
the arm's element positions. The lifecycle keeps the existing per-position drop (the batch-J
`DEF-290` fix); an overwritten copy drops the stale loans. Expected: the mixed-position case
reports at both `fill` sites; one `E10030` per affected fill; no TU; the copies used in-scope stay
accepted; the direct conditional twin keeps its report.

### 3. The control (`DEF-374`)

`eat(take (c ? v.name : w.name))` over a `for (borrow ...)` element is verified green: the generic
and concrete twins both report two `E10026` at the same span (`probes/m374_condelem.clo` =
`probes/m374_concrete.clo`, 12:24). Frozen: Ch 07 STR-11/12 (`spec/07-structs.md:106-107`: partial
moves are limited to top-level fields; element moves are `E10026`). The phase pins the count and
span parity; no code change (AR-32). A regression turns the row red in the phase.

## Integration Points

- `compiler/src/own/place.rs:1184` (`move_element_loans`), the destination replace at `:1202`,
  `replace_element_loans` (`:1103`); the per-position drop (the batch-J `DEF-290` helper);
  `compiler/src/own/statements.rs:391-399` (the `let`/assignment copy transfer);
  `compiler/src/own/expr.rs` (the conditional value transfer and the element read).
- The batch-L `DEF-315` copy-transfer path and the `E10030` report helper.
- Controls: `probes/m372_direct.clo`, `m372_member.clo`, `m373_direct.clo`,
  `preflight/m373_union.clo`, `m374_condelem.clo`, `m374_concrete.clo`; the batch-L loan matrix
  (`hp_alias_copy`/`hp_alias_return`/`hp_alias_param` shapes).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An element-read store whose loan escapes through an out-slot | One `E10030` at the fill (the direct twin's rule) | AR-17 |
| A conditional/array-literal/single-view copy whose loans escape | One `E10030` per fill site; the destination record is the union of both arms | AR-18 |
| A mixed-position copy (arm A `ys[0]`, arm B `ys[1]`) | Both `fill` sites report; no arm's positions are lost | AR-18 |
| A copy used in-scope (no escape) | Accepted; no report | AR-17/18 |
| An overwritten copy or destination | Drop the stale loans (the batch-J per-position drop) | AR-17/18 |
| A borrow-element conditional take | The control's two `E10026` (no code change) | AR-19 |
| A loan record that changes an accepted emission | Byte-identity controls and the ASan matrix | AR-17/18 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`):
  - `should_report_an_escaping_element_read_store` (`DEF-372`): the store spelling reports one
    `E10030` at the fill; the direct and member twins keep their report; an in-scope read-back
    runs 0.
  - `should_report_an_escaping_copy_from_a_conditional_or_literal_source` (`DEF-373`): the
    conditional array copy, the array-literal copy (fill form included), the single-view
    conditional, and a conditional whose arm is an array literal each report one `E10030`; the
    mixed-position case (arm A `ys[0]`, arm B `ys[1]`) reports at both fills; the direct
    conditional twin keeps its report; the in-scope copies run 0.
  - `should_report_the_borrow_element_conditional_take_twice` (`DEF-374` control): the generic
    and concrete twins each report two `E10026` at the same span.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the loan records and the report
  code/count/span per shape, the union record (both arms, mixed positions), the accepted copy
  emissions, the overwritten-loan drop, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and with `CMD="RDC_ASAN=1 cargo test
  --test conditional_projection_residue_spec_test --test
  conditional_projection_residue_impl_test"`; the host `-fsanitize=address` runs of every red
  shape (the reds are rejected after the fix, so the ASan leg re-runs the accepted controls and
  the direct twins).
- **Risks/lenses**: correctness + memory-safety + semantics (AR-27). The memory lens owns the
  loan/origin matrix, the mixed-position union, and the ASan leg; the semantics lens owns the
  Ch 16 reading. No emission lens for this phase (AR-27).
- **Risks/lenses**: correctness + memory-safety + semantics (AR-27). The memory lens owns the
  loan/origin matrix and the ASan leg; the semantics lens owns the Ch 16 reading.
