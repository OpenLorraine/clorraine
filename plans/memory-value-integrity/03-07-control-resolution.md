# Control Resolution: `DEF-427` and `DEF-447`: Memory and Value Integrity

> **Document**: 03-07-control-resolution.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-427` (major, false positive), `DEF-447` (major, false positive)
> **Phase**: P7 · **Lenses**: correctness + memory-safety (AR-31; no code change)

## Overview

Two filed majors do not reproduce at the planning head. This phase re-probes them at the
execution head, pins the correct behavior as **green controls**, and closes the rows as false
positives with the probe evidence. No compiler change is planned. If the execution-head probe
shows a red shape, the phase reclassifies the row to a fix and re-enters the ambiguity register
(AR-35(d)); a silent fix is forbidden.

## Frozen Rule

- **`DEF-427`:** Ch 08 SL-2 (`spec/08-arrays-strings.md:72`): `len(xs)` is the element count —
  compile-time for a fixed array, runtime for a slice and `str`. §4 (`:100-111`): indexing a
  `str` yields a `u8` byte (`s[i]`). `len` of a `u8` is not the element's length; there is no
  frozen diagnostic for `len` of a scalar in this batch (AR-27).
- **`DEF-447`:** Ch 16 §2 and BR-1 (`spec/16-borrow-rules.md:51-58`, `:187`): a view loan never
  outlives its owner. The phase-7 C1 correction binds an owning read-base literal at body scope,
  which is the lifetime the row describes. The probe shows the filed shape is leak-free.

## Why No Fix

### `DEF-427`

The filed form is `i32(len(w.items[i][0]))` with `items: str[2]` (the planning reconstruction) or
an equivalent double index. The first index selects the `str` element `w.items[i]`; the second
index selects its first **byte** (`u8`). `len` of that byte is not the element's length. The
"expected 2" in the row note belongs to `len(w.items[i])`, which is green at the head in every
spelling (concrete, generic, borrow, loop alias, `str`/`String` element). No code change can make
the double index report 2 without changing the meaning of indexing a `str` (Ch 08 §4 yields a
`u8`). The row is a false positive (AR-27).

**Probe evidence (`d427p`):**

```clorraine
struct W { items: str[2]; }
func get(w: W, i: i32): i32 { return i32(len(w.items[i][0])) - 2; }
func main(): i32 { let w: W = W { items: ["AA", "CC"] }; return get(w, 1); }
```

Accepted; plain 254; ASan 254; the emitted expression is `(i32)((u32)(i32)(0) - (u32)2)`.
Controls: `d427y` (loop alias `len(w.items[i])`) run 0; `d427l` (`String` element) run 0; the
concrete/generic/borrow/`str`/`String` view-length matrix (`d427a`, `d427e`, `d427i`, `d427j`,
`d427m`, `d427q`, `d427r`, `d427t`, `d427u`, `d427v`, `d427w`) run 0; the single-index member form
`len(w.items[i].name)` runs 0.

### `DEF-447`

The filed inline read-base owning literal and its variants are LSan-clean at the head. The row
was already annotated "not reproduced" by the batch-M phase-7 emission re-audit, and the phase-7
C1 correction (the body-scope read-base binding) is the mechanism it describes. No reproduction
exists to fix; the row is a false positive (AR-28).

**Probe evidence (`d447a`):**

```clorraine
struct G { rows: str[2][2]; tag: String; }
func n1(): str[2] { return ["AA", "BB"]; }
func use(): i32 { return i32(G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]); }
```

Accepted; plain 0; ASan/LSan clean. Variants: the nested-field form (`d447f`), the `String` field
move (`d447c`), the `String[2][2]` row (`d447d`), and the direct `String` read (`d447g`) are all
clean.

## Control Cases (ST-22–ST-23)

The controls are **green at the head** and are written first; they must fail if a future change
regresses the behavior.

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-22 | `DEF-427` | The view-length matrix: `len(w.items[i])` (concrete, generic, borrow, loop alias) and `len(w.items[i].name)`; the byte-index variant `len(w.items[i][0])` | The view-length readings are 2 at every spelling and run 0; the byte variant asserts `len(...) == 0` (the probe's `-2` returns 254, ASan 254, no memory error) |
| ST-23 | `DEF-447` | The filed inline read-base owning literal; the call-row/nested-field, view-copy, and `String`-field variants | All build strictly and run 0; ASan/LSan clean; the body-scope read-base binding is pinned |

`DEF-427`'s control lives in `compiler/tests/array_value_integrity_{spec,impl}_test.rs` (the
fixed-array/`len` origin); `DEF-447`'s control lives in
`compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (the phase-7 literal origin)
(AR-30).

## Closure Procedure

1. Rebuild at the execution head; re-run the filed probes and the variants.
2. If both remain green: record the verdicts in `05-findings.md`, close the two roadmap rows as
   false positives with the probe evidence, and add the controls.
3. If a red shape appears: **stop**, reclassify the row to a fix, add it to the active phase (or a
   new phase) with the new probe, and re-enter the register before code (AR-35(d)).
4. No compiler, spec, or registry edit in either outcome.

## Verification

- `make check`; the array-value and residue suites plain and with `RDC_ASAN=1` (the `DEF-447`
  control is the LSan leg).
- The strict builds of the controls; the view-length matrix reads 2.
- The roadmap/ledger close-out records the false-positive evidence (AR-36).

## Risks and Lenses

- **Correctness** verifies the probes and the control expectations; a false positive closed with
  the wrong evidence would hide a real defect.
- **Memory-safety** re-reads the `DEF-447` LSan evidence and the `DEF-427` byte-index shape (no
  memory error).
- **Risk:** the original filed repro may have been a slightly different shape that is still red;
  the phase re-derives and probes the variants named in the row note before closing. If none
  reproduces, the closure is evidence-based (AR-27, AR-28).
- **Risk:** a future `len`-of-a-scalar checker rule is a separate scope; the control documents
  the current behavior and does not add a rejection.
