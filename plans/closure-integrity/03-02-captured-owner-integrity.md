# Captured-Owner Integrity: Closure Integrity

> **Document**: 03-02-captured-owner-integrity.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 2 closes the two ownership holes in the capture path: a lambda that returns its by-value
non-duplicable capture double-frees (DEF-104), and a called func-typed capture whose name matches
a module lender leaks its owned result (DEF-109). Both fixes reuse established mechanisms — the
per-instance duty audit and the callee-name masking — and neither changes the language surface.

## Architecture

### Current Architecture

- A capture's mode is chosen at the generic definition: `ty.is_unknown() || duplicable` → `Copy`,
  else a read-only borrow (`sema/expr.rs:157-172`); a bare `T` is duplicable
  (`types/table.rs:404-424`). The per-instance audit (`own/instance_check.rs:12-16`) has no
  capture-ownership rule, so an owning instance keeps the copy: the environment field and the
  source alias one allocation and the body's return (`own/lower.rs:468-483`) copies `__env->x`
  out, releasing the same storage twice.
- `declare_captures` (`own/lower.rs:575-625`) pushes the capture binding with `ir_name` set to the
  source name (`:598-600`), bypassing the masking `declare`/`unique_ir_name` perform
  (`own.rs:836-898`, `masks_callee` at `:892`). A call through the capture resolves by
  `bindings[id].ir_name` (`own/call.rs:170`) and the may-lend gate looks the name up in the
  program's callee map (`own/lower.rs:1564-1589`, `own/program.rs:150-163`), so a capture named
  like a module lender selects the lender, skips binding the owned result, and leaks it.

### Proposed Changes

1. **The captured-owner return (DEF-104).** The return of a by-value non-duplicable capture is a
   move of the capture, which L-3 rejects:
   - the concrete spelling (`func outer(x: Wrap): Wrap { return ([x] () => x)(); }`) is judged at
     the declaration by the existing E10401 rule (`sema/expr.rs:256-276`, reaching the return
     position of the lambda body);
   - the generic spelling records the judgment as a per-instance duty at the recorded lambda site
     and the audit substitutes each instance: an owning, non-nullable, non-view instance reports
     `E10401` (`cannot move captured '{name}' in a lambda`); a duplicable instance stays accepted
     (the copy is sound), and the nullable/view carve-outs follow the capture-mode rule.
   The duty mirrors `RequiresStorableSlot`: recorded once at the site, audited once per instance,
   reported once for the first offending instance (AR-5, AR-11).
2. **The capture name mask (DEF-109).** `declare_captures` computes the capture's IR name through
   the same masking path as every other binding (`declare`/`unique_ir_name`), so the masked name
   never collides with a module lender; the may-lend gate, symbol resolution, and the codegen
   environment lookup (`codegen/expr.rs:604-661`) then resolve the capture itself. The false
   comment at `own/lower.rs:598-599` is corrected: the spelling *is* read for a called func-typed
   capture.

### Check order

The capture-return judgment runs where the lambda body's value is returned, before the result's
transfer is assumed; a rejected return leaves no partial state. The masking change is
spelling-only and must leave diagnostics naming the source name.

## Implementation Details

### New Functions/Methods

The duty follows the existing record/audit pattern (three small pieces, mirroring
`storable_slot_requirements`):

```rust
/// Records that a lifted lambda returns a by-value capture.
///
/// The capture's mode is not final while the enclosing function is still
/// generic: a bare `T` copies, but an owning substitution makes the copy an
/// alias the return would release twice. The site is recorded once and the
/// per-instance audit applies the concrete judgment after substitution.
fn record_capture_return(&mut self, site: usize, capture: &CaptureInfo, span: Span)

/// Reports the capture-return judgment at each recorded instance.
///
/// A duplicable instance copies and stays accepted; an owning instance
/// reports `E10401` exactly as the concrete spelling does. One duty reports
/// once — the first offending instance.
fn check_capture_return_requirements(/* the existing duty inputs */)
```

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `sema/expr.rs` | the return-position judgment in the E10401 rule |
| `own/lower.rs` | the lambda-body return path (`:468-483`) records the duty; `declare_captures` masks |
| `own/instance_check.rs` | the new audit beside `check_storable_slot_requirements` |
| `own.rs` | `declare`/`unique_ir_name` are reused, not changed |
| `codegen/expr.rs` | the existing environment lookup is reused unchanged |
| the suites | `borrow_generic_integrity_*`, `call_value_integrity_*`, `v1_language_*` stay green |

## Code Examples

```clorraine
struct Wrap { v: String; }

func outer<T>(x: T): T { return ([x] () => x)(); }   // accepted at T = i32 (copy)

let w: Wrap = Wrap { v: stringOf("abcd") };
outer<Wrap>(w);                                        // E10401: cannot move captured 'x' in a lambda
                                                       // (the author writes `return x;` directly)

func run(borrow s: String): String { return stringOf("mod"); }
let run: func(borrow String): String = run;
let r: String = ([run] () => run(stringOf("ab")))();   // accepted, ASan-clean
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owning instance returns a by-value capture | one `E10401`, naming the capture | AR-5, AR-11 |
| Duplicable instance, same shape | accepted (copy) | AR-5 |
| Concrete owning spelling | the same `E10401`, judged directly | AR-11 |
| Called func-typed capture named like a module lender | masked IR name; the result releases once | AR-7, AR-10 |
| A real module-lender call outside a capture | unchanged (the module callee map is untouched) | AR-10 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-10 – ST-13 ([07-testing-strategy.md](07-testing-strategy.md)).
- Host ASan probes for the DEF-104 and DEF-109 shapes (AR-13): the owning shape is rejected (no
  translation unit); the func-typed capture runs with leak detection on.
- Implementation pins: one report per site with the substituted instance first, the masked IR name
  and the unchanged source-name diagnostics, the release counts on the DEF-109 shape, and
  byte-identical re-emission.
