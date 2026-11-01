# Residual Hardening: Clorraine v1.0 Production Readiness

> **Document**: 03-01-residual-hardening.md
> **Parent**: [Index](00-index.md)

## Overview

Five residual classes are release-blocking: the explicit null diagnostic and four initializer
leak shapes. They either give the wrong diagnostic for a frozen rule or leave accepted programs
with unreleased owned values. The rest of the report-only residuals are accepted and documented
in `docs/known-limitations.md` (AR-4, AR-14).

The design follows the existing ownership and emission patterns: the compiler already has an
initializer-owner registry, a fresh-projection-root binder, and a nullability diagnostic. This
component extends those paths; it adds no new layer, dependency, or generalized mechanism.

The original examples in this document used a move-only fill and a local element move. Both are
rejected by the frozen language (`E10215` for a move-only fill, `E10026` for an element move), so
they could not drive a red test. The reproductions below are the accepted shapes that actually
leak or report the wrong diagnostic at the current revision.

## Architecture

### Current Architecture

- **Null conversions.** `let b: bool = null;` already reports `E10012` through the nullability rule.
  The explicit cast spelling, `bool(null)`, reports `E10201` at
  `compiler/src/sema/expr.rs:1109-1125`, and `!null` reports `E10218` through
  `compiler/src/sema/expr.rs:1616-1628`. **User ruling 2026-11-01:** both explicit spellings report
  E10012; `spec/04-expressions.md` records the null-cast exception to TY-14. There is no new bool
  conversion.
- **Initializer view roots.** `compiler/src/codegen/program.rs:1133-1430` binds the fresh owning
  roots that a retained initializer view reads. `retain_view_array_roots` (`:1399-1423`) walks the
  top-level elements and the fill value, but it does not recurse into a nested `ArrayLit`; a nested
  fill row therefore keeps only the selected row's roots. The module-initializer path is the same
  shape at a wider scope.
- **Multi-element initializer.** A module-level view global whose initializer projects one element
  of a multi-element view array retains the selected element's root and leaks the unselected
  element's root (11 bytes at the current revision).
- **Initializer field call root.** A local struct field initialized from a call through a fresh
  projection (`Box { n: sink(mk().xs[0]) }`) binds only the selected `xs[0]`; the sibling `xs[1]`
  is never released (11 bytes).
- **Indexed call-root projection.** A module-level view global `let g: str = mk()[pick()];` is
  accepted and binds a fresh projection root, but a run-time index is not part of the constant
  accessor chain, so the rebound root loses the index and the unselected slot leaks (11 bytes).
  A local owning move (`let s: String = mk()[pick()]`) is rejected by `E10026` and is not the
  residual.

### Proposed Changes

1. **Null-to-bool diagnostic (user ruled E10012).** Route the null-shaped cast and the unary
   `!null` condition through the existing nullability check, and update the cast table in
   `spec/04-expressions.md` to name the null-cast exception to TY-14. Keep
   `let b: bool = null;` as the already-correct control. Do not add a bool conversion or change
   any accepted program.
2. **Initializer view retention.** Extend the initializer retention walk to:
   - recurse into a nested `ArrayLit`, so every nested fill row retains its fresh fill root;
   - retain every fresh element root in a multi-element view-array initializer, not only the
     selected element;
   - bind the whole fresh call root when an initializer field or element projects through a call
     result, so the sibling fields release at init-function exit;
   - keep the existing init-owner registry as the release authority; each retained root is
     registered once and released in reverse order.
3. **Indexed module-initializer view root.** Carry the run-time index through the module
   initializer's view-root binding, so the bound temporary is indexed by the same evaluated value
   and the release covers the unselected slots. Evaluate the index once and share it between the
   read and the release.
4. **Known limitations.** Do not write the document here; `03-05` is the sole owner. `03-01` supplies
   the verified residual facts.

## Implementation Details

### New Types/Interfaces

No new public types. The work extends existing internal helpers:

- `compiler/src/sema/expr.rs` — null/cast and condition classification (E10012 ruling).
- `compiler/src/codegen/program.rs` — init-owner retention, nested array recursion, and projection
  rebinding.
- `compiler/src/own/lower.rs` — ownerless projection binding where the module initializer path
  shares it.
- `compiler/tests/v1_production_readiness_spec_test.rs` — the ST-1 – ST-6 cases.
- `compiler/tests/v1_production_readiness_impl_test.rs` — emitted shapes and release counts.

### New Functions/Methods

No new exported API. Internal helper changes are described in the proposed changes above. Any
new helper must be private to its module and documented with the existing module style.

### Integration Points

- The spec suite uses the existing test helpers for emit, strict C build, and `RDC_ASAN` runs.
- The implementation suite uses the existing byte-identity and emitted-C pin style.
- The phase review dispatches `correctness-reviewer`, `memory-safety-auditor`,
  `emission-integrity-auditor`, and `semantics-reviewer`; the change touches ownership,
  diagnostics, codegen, and language rules. The emission lens includes the base/head corpus
  emission diff and the strict C build; the semantics lens includes the `spec/17-codegen` ABI
  review where lowering changes.

## Code Examples

### Null-to-bool

The three spellings are separate compilations; a semantic error in one suppresses the later
nullability reports in a combined program.

```clorraine
// control: already E10012 at current revision
func main(): i32 {
    let b: bool = null;
    return 0;
}
```

```clorraine
// cast: E10201 today; E10012 after the Phase 1 fix
func main(): i32 {
    let b: bool = bool(null);
    return 0;
}
```

```clorraine
// condition: E10218 today; E10012 after the Phase 1 fix
func main(): i32 {
    let b: bool = !null;
    return 0;
}
```

### Nested fill at module scope (red: 11-byte leak)

```clorraine
struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(len(first)) - 2;
}
```

The inner `[; ...]` rows are nested `ArrayLit` values; the current walk retains only the selected
outer row and leaks the other row's fill root.

### Multi-element view array at module scope (red: 11-byte leak)

```clorraine
struct H { v: String; w: String; }
struct Box { names: str[2]; tag: String; }

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(picked)) - 2;
}
```

The sibling owning `tag` field forces the initializer onto the store path; without it the literal
stays inline and the roots bind correctly. The red fixture must keep the `tag` field.

### Initializer field call root (red: 11-byte leak)

```clorraine
struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let b: Box = Box { n: sink(mk().xs[0]) };
    return b.n - 2;
}
```

The direct call and a func-typed callee twin both leak the sibling `xs[1]`; the fix must bind the
whole `mk()` root, not only the selected element.

### Indexed call-root projection at module scope (red: 11-byte leak)

```clorraine
func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { return 1; }

let selected: str = mk()[pick()];

func main(): i32 {
    return i32(len(selected)) - 2;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `bool(null)` / `!null` | Report E10012; record the exception in `spec/04-expressions.md`; keep the annotated-let control | AR-4 |
| Nested fill row | Recurse into every nested `ArrayLit` and register each fresh fill root | AR-4 |
| Multi-element view array | Retain every element root; release the unselected roots at init exit | AR-4 |
| Initializer field call root | Bind the whole call root and release the sibling fields | AR-4 |
| Indexed module view | Carry the evaluated index through the view-root binding; release the unselected slots | AR-4 |
| Local owning element move | Keep the frozen `E10026` rejection; it is not a residual | AR-4 |
| Move-only fill | Keep the frozen `E10215` rejection; use a view array for the test | AR-4 |
| Residual outside the list | File a new DEF row; do not absorb it into this batch | AR-4 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-1 – ST-6 in `07-testing-strategy.md`. ST-1 compiles the control and each
  explicit spelling independently; a combined program would suppress the later nullability
  reports.
- ASan probes: every new memory case runs with `RDC_ASAN=1` and a bounded timeout; the Phase 1
  Verify command also sets `RDC_ASAN=1` and runs the suite serially.
- Implementation tests: emitted release counts, root binding, index carry-through, and byte
  identity where pinned.
- Regression: the full `make check` suite; `make asan` after the target lands.
