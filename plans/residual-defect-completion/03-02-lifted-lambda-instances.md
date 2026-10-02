# Lifted-Lambda Instances: Residual Defect Completion

> **Document**: 03-02-lifted-lambda-instances.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-52: a generic call inside a lifted lambda in a generic body is never
instantiated. Lowering lifts the lambda into a separate IR function whose types still mention the
enclosing type parameter; the instance walk never visits it, and emission writes it once without a
substitution, so the C names `ClT` and the build fails.

The approved policy is to specialize the lifted lambda per enclosing instance (AR-2). Nothing new
is needed for how an instance is emitted as a C function; the work is provenance plus walking the
lifted body with the instance bindings and emitting one copy per needed instance.

## Architecture

### Current Architecture

- Lowering registers each lifted function with `lambda_site: Some(site)` and appends it to the
  function list immediately after its enclosing declared function
  (`compiler/src/own.rs:114-117`, `compiler/src/own/lower.rs:453-473`,
  `compiler/src/ir.rs:548-554`). No field names the enclosing function.
- `collect_ir_instances` walks every IR function, then re-walks each recorded instance's declared
  function with its bindings until the instance set closes
  (`compiler/src/own/generics.rs:453-491`). Lifted functions are never walked with bindings.
- Emission writes every declared instance with a substitution
  (`compiler/src/codegen/program.rs:740-755`) and every lifted function exactly once without one
  (`:756-766`). `emit_lifted_function` resolves the symbol from the site alone
  (`compiler/src/codegen/function.rs:119-135`); closure environment structs and lifted prototypes
  are emitted once per site (`program.rs:230-260`, `:447-462`).
- The closure value, its call, and its environment are rendered from the site alone
  (`compiler/src/codegen/expr.rs:574-580`, `:598-619`, `:629-632`), and its C type is
  `ClClosure_<id>` (`compiler/src/codegen/types.rs:298-300`, `:60`). Both must become
  instance-aware for a specialization.

### Proposed Changes

1. **Carry the enclosing-function provenance.** When lowering a lambda, record the enclosing
   declared function's name on the lifted `Function` (a new field beside `lambda_site`). The
   lowerer knows it at `lower.rs:453-473`, where the enclosing function's name is available.
2. **Walk lifted bodies in the instance closure.** In `collect_ir_instances`, when an instance is
   processed, also walk every lifted function whose enclosing name matches the instance's declared
   function, with the same bindings, and record which (site, function, args) specializations are
   needed. This records nested generic calls (the DEF-52 symptom) and marks the specializations.
3. **Emit one specialization per needed (site, args).** A lifted function whose types, calls, or
   captures mention no enclosing parameter keeps the single shared emission exactly as today
   (byte-identical for existing programs). A lifted function that mentions a parameter is emitted
   once per marked instance with the substitution installed, under a deterministic symbol
   `cl_lambda_<site>__<instance type key>`; the site-only symbol remains for the shared case.
4. **Environment structs follow the same keying.** A capturing lambda whose capture types mention
   an enclosing parameter gets one `ClEnv_<site>__<key>` struct and callable type per
   specialization; a non-parameter capture keeps today's `ClEnv_<site>`. Closure creation sites
   inside an instance body reference that instance's environment and lifted symbol.

The emitter already installs a substitution for declared instances; the specialization reuses the
same mechanism (`emit_function`), so the lifted body is not re-checked, only re-substituted.

## Implementation Details

### New/changed data

- `compiler/src/ir.rs` — `Function` gains the enclosing declared function's name next to
  `lambda_site` (a `Option<String>` or an id, implementation's choice; the behavior is what
  matters).
- `compiler/src/own/lower.rs` — set the provenance when pushing the lifted function.
- `compiler/src/own/generics.rs` — `collect_ir_instances` gains the lifted re-walk and the
  specialization marking. Suggested shape: `Monomorphization` gains a recorded set of
  `(site, args, enclosing)` specialization keys, deduplicated like `Instance`.
- `compiler/src/codegen/program.rs` — the lifted emission loop emits shared functions once and
  specializes marked ones per instance; `emit_closure_declarations` and the closure-creation
  paths resolve the per-instance environment when marked; `private_function_references`
  (`program.rs:807-828`) references each specialized `static` symbol.
- `compiler/src/codegen/function.rs` — `emit_lifted_function`/`lifted_signature` accept the
  instance substitution and the specialized symbol; the no-substitution path is unchanged.
- `compiler/src/codegen/types.rs` — the callable-type spelling becomes instance-aware: a marked
  specialization names its callable `ClClosure_<site>__<key>` (and its environment
  `ClEnv_<site>__<key>`) so the instance body's local type matches the specialized function
  pointer; `closure_type`/`c_type` gain the active-specialization spelling.
- `compiler/src/codegen/expr.rs` — `closure_value` (`:574-580`), `closure_call_with_destination`
  (`:598-619`), and `environment_literal` (`:629-632`) render the active specialization's
  `cl_lambda_<site>__<key>` and `ClClosure_<site>__<key>`/`ClEnv_<site>__<key>` spellings instead
  of the site-only names.

### Integration Points

- `ProgramInfo.functions` keys declared functions by name; lifted functions stay a separate list.
  The specialization key must be deterministic and must not disturb the declared-instance order
  (byte-identical re-emission).
- The closure `TyKind::Closure` payload carries only a site id (`compiler/src/types/mod.rs:122-129`);
  the instance spelling is chosen by the emitter's active specialization when rendering, so the
  type itself is not rewritten. Either a rendering-context field or a key-carrying remap is
  acceptable; the required behavior is the emitted per-instance name.

## Code Examples

```clorraine
func apply<T>(x: T): T { return x; }

func outer<T>(x: T): T {
    let f: func(T): T = (v: T) => apply<T>(v);
    return f(x);
}

func main(): i32 { return outer<i32>(3); }
```

After the fix the emitted C contains an `outer` instance calling a specialized lambda that calls
`cl_apply__i32`; the program builds and returns `3`.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| The enclosing instance names no lifted function | No specialization is emitted; the shared path is untouched | AR-2 |
| A lifted body mentions no enclosing parameter | The single shared emission stays; existing emitted C is byte-identical | AR-2, R9 |
| Recursion through a lambda instance | The same instance dedup logic and `E10305` cap apply; the walk only records, never recurses by hand | AR-2 |
| Two instances specialize the same site | Distinct keys by instance type key; both bodies emitted | AR-2 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-7 – ST-9 (07-testing-strategy.md): capture-less primary repro, two
  coexisting instances, and a capturing lambda with a parameter-typed capture.
- Implementation tests: specialization symbol shape, per-instance environment struct and
  callable type, closure value/call/env rendering for both the capture-less and the capturing
  case, shared emission unchanged for a non-generic lambda, and byte-identical re-emission.
