# Design

## Phase 1 — Generic dot-call inference and bounds (`DEF-397`)

`check_generic_dot_call` already computes the receiver/argument bindings. After
the final `unbound` set, run the direct call's judgments:

1. when `unbound` is empty, the recursive-instantiation check (`E10305`);
2. otherwise `E10301`, exactly as `check_generic_call`;
3. `check_callable_bounds` (`E10303`);
4. `check_ephemeral_bindings` for the chosen instance.

The arity-mismatch early return keeps its current behavior: the count report is
the first cause, and the argument walk still reports its own errors.

## Phase 2 — Qualified function values and intrinsics (`DEF-398`, `DEF-399`)

### Function value

`codegen/expr.rs::path` gains a module-qualified function branch before the
global-binding branch: join the segments, resolve the function through
`resolve_function` (which already accepts `module.name`), and emit
`function_symbol(module, name)` for a non-generic declaration. A generic
function value without an instance keeps its previous spelling.

### Intrinsics

- The `std.mem` export surface gains the compiler intrinsic helpers `len`,
  `rawSlice`, and `rawStr`, so the resolver accepts the qualified spelling just
  as it accepts `std.mem.slice`.
- `sema::check_call`, in the qualified branch, resolves the target module's
  declaration kind after the function-table miss:
  - `len` → walk the argument(s), return `usize`;
  - `slice`/`rawSlice` → `check_view_builder`;
  - `rawStr` → walk the argument(s), return `str`;
  - `ffi` → read the target module's `ffi` signature, check the written count
    and `cstring` arguments, and return the ABI result type.
- `own::lower_intrinsic_call` matches the simple (last) segment after its
  user/ffi-shadow guard, and handles the free `len` form by lowering its first
  argument (the dot form still lowers the receiver).

### Cross-module ffi

The resolver treats an `Intrinsic` or `Ffi` declaration in a non-own module as
qualified-visible even without `export`. The sema route above uses the target
module's `ffi` signature; the emitter's existing `ffi_callee_signature` already
resolves `module.name` and the simple C symbol.

## Phase 3 — Module-path precedence (`DEF-400`)

The own call checker skips the field/function-binding probes when the callee is
a qualified name whose full dotted spelling resolves to a user or `ffi` module
function, and takes the named-call branch. A lexical local of the head is still
handled by `callee_name`'s receiver-is-value test, which checks the lexical
table, so a local keeps the dot-call reading. Sema and the resolver already
prefer the module path (batch R phase 5 and the resolver's first split).
