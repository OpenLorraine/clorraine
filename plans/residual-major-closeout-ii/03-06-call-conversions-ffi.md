# Call Conversions and `ffi` Typing: Residual Major Closeout II

> **Document**: 03-06-call-conversions-ffi.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are follow-ons of the batch-K phase-4 conversion duties: an inferred call whose
null-shaped argument is not judged because a *sibling* argument fixes the parameter to a
concrete non-nullable type, and the inverted conversion routes (a caller value at a
parameter-typed destination through an explicit type argument, and the caller's own type
parameter as an explicit argument) that record no duty. The incidentally fixed `DEF-350` is the
phase's control: the batch-K F1 correction typed the view/slice `ffi` result as the ABI pointer,
and its spellings are pinned.

## Design

### 1. The sibling-fixed null argument (`DEF-344`)

`pair(1, x)` with `let x = null;` against `pair<T>(a: T, b: T)` is accepted and emits `void *`
where `i32` is declared (`dn1`, strict build fails); the explicit twin `pair<i32>(1, x)` reports
one `E10012`, and `pair(x, 1)` reports one `E10012` while `pair(x, x)` reports two (the recorded
count question). The null rule (`own/call_check.rs:397`) fires when an *inferred parameter
resolved to the null shape*; when a sibling fixes the parameter concrete, the null-shaped
argument is never judged.

After the inference resolves the parameter, judge every null-shaped argument against the
resolved parameter type: a concrete non-nullable parameter reports the frozen `E10012` once per
argument-use site (settling the count question per the frozen per-site rules — the phase
measures `pair(x, 1)` and `pair(x, x)` against the concrete twins and records the ruling). The
legal null-capable instances (`T = String?` etc.) stay accepted.

### 2. The inverted conversion routes (`DEF-351`)

Four spellings are accepted and fail the strict build: `pick<cstring>()`'s `return 5`
(`reverse1`), `consume2<cstring>(x)` with `x: str` (`gencallee`), `sink<U>(a)` at `U = cstring`
with `a: i32` (`genmixed`), and `bs01`'s explicit argument that is the caller's own type
parameter (`inner<U>(c)` with `c: cstring`, `U = raw u8`). The concrete twins report `E10200`.
The batch-K duty records parameter-*mentioning* sources; the destination-side trigger (a caller
literal/name into a parameter-typed parameter) and the explicit-type-argument route are
unrecorded.

Extend the duty record to the generic-call route and the destination-side trigger: a written
argument/return whose type is concrete while the resolved parameter is a concrete incompatible
destination records a conversion duty, and the instance audit reports the frozen `E10200` once
per site. The legal instances (`T = cstring`/`str`/`String`, the identity spellings) stay
accepted; `genlit`'s `E10200` and the concrete twins are the controls.

### 3. The control: the `ffi` view/slice result (`DEF-350`)

The batch-K phase-4 F1 correction types a view/slice `ffi` result as the ABI pointer, so at the
head the view/slice destinations report one `E10200` (`h8`/`h9`/`fr17c`/`fr17f`), the template
reports `E10500` (`fr17l`), `len(Make())` and the slice length build and run 0
(`fr17b`/`fr17e`), and the cstring return/index compile (`fr17d`/`fr17h`). The residual
`.ptr`/`.len`-on-a-raw-pointer spellings are the separately filed minor `DEF-352`, and the
inferred local's unused-local failure is the `len`-surface family; neither is this batch's
scope. The phase pins the fixed spellings as controls with a per-spelling table.

## Integration Points

- `compiler/src/own/call_check.rs:397` (the inferred-null rule), the call inference path.
- `compiler/src/sema/call.rs` (the `OpaqueConversion` duty record), `compiler/src/own/instance_check.rs`
  (the duty audit and the frozen `E10200` report helper).
- `compiler/src/codegen/function.rs`/`calls.rs` (the sanctioned casts already emitted).
- Controls: `K4S-sem/genlit.clo`, the concrete twins (`K4S-sem/*_conc.clo`), the batch-K
  conversion cases, the `ffi` spellings of `emission-audit/k4/variants/fr17*`.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A null-shaped argument at a null-capable resolved parameter | The existing acceptance (comparisons, nullable instances) | — |
| A legal conversion at a generic destination | The duty does not record; the legal instances stay accepted | — |
| A conversion duty at a site whose source is unknown | No duty; the existing unknown-shape rules govern | — |
| A `ffi` extern without a definition in a memory run | The filed evidence is the strict build; the recorded stubs link where they exist | AR-11 |

## Testing Requirements

- Spec: the sibling-fixed null argument (the rejected spelling, the two count controls, the
  legal nullable instances; one report per site), the inverted conversions (the return, the
  argument, the explicit type argument as the caller's parameter, the b-s pair; the legal
  instances accepted and running), and the `DEF-350` control table.
- Impl: the duty records and the per-instance reports (code, count, span), the legal-instance
  emissions, the `ffi` prototype typing, the rejection shapes; a re-emission
  determinism pin.
