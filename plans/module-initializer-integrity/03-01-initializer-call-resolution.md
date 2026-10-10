# Initializer Call Resolution: Module Initializer Integrity

> **Document**: 03-01-initializer-call-resolution.md
> **Parent**: [Index](00-index.md)

## Overview

DEF-131/132/133 are three faces of one gap: `lower_global` (`codegen/program.rs:1560-1691`)
lowers initializer calls without the body path's callee resolution. A lambda-literal callee has
no lifted function (the empty name renders `(__cl_itmp0);`), a func-typed module global spells
its source name instead of the mangled C symbol, a generic call declares no instance, and a dot
call drops its receiver (`receiver: None`, the receiver folded or lost). The fix records what the
own stage must know — generic instances and lifted init lambdas — in a new initializer walk, and
mirrors the body's receiver semantics in the init Call arm.

## Architecture

### Current Architecture

- `generics::collect` (`own/generics.rs:30-45`) walks `Item::Func` bodies only
  (`walk_block`/`walk_expr`/`record_call`, `:180-431`); `collect_struct_instances` (`:71-76`)
  records a global's declared type only. Initializer calls produce no instance.
- `lift_lambda` (`own/lower.rs:458-592`) is reachable only from `lower_lambda` (`:372-409`),
  itself reachable only from a `FunctionChecker` lowering a body.
- `lower_global`'s Call arm (`codegen/program.rs:1668-1688`) uses
  `callee_name(callee).unwrap_or_default()` and a hard-coded `receiver: None`; the body path
  uses `is_closure_callee` (`own/expr.rs:235-238`), `func_field_receiver` (`own/call.rs:112-155`),
  and `lower_dot_receiver` (`:33-74`).

### Proposed Changes

1. **Initializer walk (own stage, PD-2).** Extend the AST pass to walk every
   `Item::GlobalLet`/`GlobalConst` value with the existing `walk_expr`, so `record_call` records
   initializer calls into the same call list the instance machinery consumes (DEF-132). The walk
   also collects each lambda literal that appears in an initializer expression (the filed shape
   is a callee; a lambda value is collected the same way) keyed by its site id.
2. **Init lambda lift.** For each collected lambda site, lift it once through `lift_lambda`
   reusing a **synthetic minimal checker context**: a `FunctionChecker` constructed with a
   synthesized module-init `FuncDecl` (name `__cl_init_<module>`, no parameters, no body
   lowering) whose only use is the lift; the synthetic context's own IR is never emitted. Init
   lambdas are necessarily capture-less for the liftable shapes: a capture entry can only name a
   local (there are none in an initializer) or a module name (already `E10406` from the
   closure-integrity rules). The lifted functions append to the normal lifted set so the
   emitter's lifted-function pass emits them (`codegen/program.rs:1949-2028`).
3. **Init Call resolution (codegen).** `lower_global`'s Call/ClosureCall arms mirror the body:
   - a lambda-literal callee resolves to the lifted identity `cl_lambda_<site>` recorded in
     step 2;
   - a name resolving to a module function spells its emitted symbol (as today) and, for a
     generic call, the instance keyed by the recorded type arguments (DEF-132);
   - a name resolving to a func-typed module global spells the global's mangled C symbol
     (DEF-131), and the call is a value call through that global's signature;
   - a dot call sets `receiver` for a func-typed field (mirroring `func_field_receiver`) and
     otherwise passes the lowered receiver as argument zero (mirroring `lower_dot_receiver`),
     with the dot-`len` structural path kept in 03-02 (DEF-133).
4. **Shape gating.** Resolution changes apply only to the shapes that differ; an initializer
   whose calls already resolve emits byte-identically (AR-13).

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/generics.rs` | the initializer walk extends `collect`; `record_call` unchanged |
| `own/lower.rs` | `lift_lambda` reused via the synthetic context |
| `own.rs` | the own-stage driver calls the walk and carries lifted functions into the output |
| `codegen/program.rs` | `lower_global` Call/ClosureCall arms resolve callee + receiver |
| `codegen.rs` | the lifted-function emission pass already consumes the lifted set |

### New Types/Interfaces

- The walk's collected lambda record: the lambda's site id plus its module, feeding the lift.
- No new public types; the synthetic `FuncDecl` is internal to the own stage.

## Code Examples

```clorraine
// DEF-131 (R1): lambda callee
let g: i32 = (() => 3)();

// DEF-131 (R2): func-typed global callee
func ready(): i32 { return 7; }
let f: func(): i32 = ready;
let g2: i32 = f();

// DEF-132 (R3): generic call
func pick<T>(a: T, b: T): T { return a; }
let g3: i32 = pick<i32>(1, 2);

// DEF-133 (R4): callable-field receiver
struct Box { get: func(): i32; }
let b: Box = Box { get: ready };
let g4: i32 = b.get();
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Lambda with a capture entry | Existing rules report (`E10406` for module names; no locals exist) | AR-3 |
| Unresolved callee after the fix | Remains a strict-build failure — the test oracle; no new code | AR-12 |
| Instance still missing for an initializer call | A spec failure of R3, never a runtime wrong symbol | AR-13 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-F1 – ST-F4 ([07-testing-strategy.md](07-testing-strategy.md)): each
  builds under the strict flags and runs to its pinned result.
- Implementation pins: the lifted `cl_lambda_<site>` definition and callable use; the mangled
  global symbol; the declared generic instance symbol; the receiver argument in the emitted
  init call.
- Regression: the existing init pins in `eval_emission_integrity_impl_test.rs` /
  `store_emission_integrity_impl_test.rs` stay untouched; the corpus differential shows only
  the four defect shapes plus new cases.
