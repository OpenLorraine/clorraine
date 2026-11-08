# Hook Design: C-Hook Trampolines (L5)

> **Document**: 03-03-hook-trampolines.md
> **Parent**: [Index](00-index.md)
> **Requirements**: R10-R12

## 1. Surface

An `ffi` parameter of function type with mode `borrow` is a synchronous
callback:

```clorraine
ffi fixture {
    func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32;
}
```

The mode is semantic: the C prototype is still a plain function pointer. The
contract is: the callee may call the callback zero or more times during the
call and must not store it, return it, or call it after the call returns.

The marker is deliberately the existing `borrow` mode. X3 removed the draft
clause words (`borrows`, `consumes`, `value`, `nullable`) from the FFI
surface; the mode vocabulary already expresses a call-scoped loan. An
unmarked, `const`, or `take` `func` parameter has no synchronous contract and
accepts only a capture-less lambda.

## 2. Sema check

`check_ffi_arguments` (`compiler/src/sema/call.rs:1546-1603`) gains a function
type branch. It needs the parameter modes, so `ffi_parameter_types` returns
`Vec<(Ty, Mode)>` instead of `Vec<Ty>`. Both the simple and the qualified
`ffi` call paths (`sema/call.rs:100-119` and `218-231`) must pass the modes;
changing only the cache type leaves the qualified path mode-blind.

The call-node address is available in `check_expr`, where the existing
`type_casts` record is keyed (`sema/expr.rs:57-70`). Thread that address (or a
site id) through `check_call` into `check_ffi_arguments`; do not try to key
the record from a name inside the argument walk.

For a parameter whose type is `Func(expected)`:

| Argument | `borrow` parameter | other modes |
|---|---|---|
| `Func(found)` | signature match, else `E10210` | signature match, else `E10210` |
| `Closure(found)` | match plus ABI-identical check, record the hook; else `E10210` | `E10400` |
| any other type | `E10200` (not callable) | `E10200` |

The signature match compares parameter types and the return type, ignoring
the outer `borrow` marker and the callback's own parameter modes, exactly as
`check_callable_bounds` compares a capturing closure to a callable bound
(`compiler/src/sema/call.rs:785-800`). The **ABI-identical check** then
rejects, with `E10210`:

- a `str`, `static str`, slice, or fixed-array callback parameter (the language
  spelling is `ClStr`/`ClSlice`, the FFI spelling is `APTR`);
- a `borrow` parameter of a non-duplicable struct (the language spelling is
  `T *`, the FFI spelling is by value);
- a fixed-array return (the language spelling is a hidden out-parameter, the
  FFI spelling is `APTR`);
- a generic by-value struct instance (the `ClWrap`/`ClWrap__i32` mismatch).

The accepted set is scalar, `bool`, enum, `raw`, `cstring`, handle, non-generic
by-value struct, and function-pointer parameters and returns. This is the
narrowest set that is ABI-identical to the lifted lambda; a broader set would
emit incompatible C function pointers.

A capturing hook inside a generic function body is `E10400`; the site would
need a per-instance wrapper that v1.1 does not add. A capture-less hook inside
a generic body is a plain pointer and is accepted. A capturing lambda at a
generic callable-bound parameter (`F: func(T): R`) keeps the existing
acceptance and is not a hook.

The record lives in `SemaOutput`:

```rust
pub struct HookCall {
    pub arg: usize,
    pub closure: ClosureTy,
}
pub hook_calls: BTreeMap<usize, Vec<HookCall>>, // key: call expression address
```

The map is deduplicated by `(call address, arg index)` because a generic body
or an audit may visit the same call twice.

## 3. Lowering

`lower_call` processes the call normally. The capturing callback argument is a
`ValueKind::Closure { site, captures }`; its captures are evaluated at the
argument's source position and materialized by the ordinary argument rules.
After `lower_call` returns, `lower_value` checks the hook record for the call
node's address and wraps the call:

1. For each hook argument `i`, materialize the closure value into a statement
   temporary **at that argument's source position**, inside the argument loop.
   The materialization must happen before any later argument's effects run; a
   post-processing step at the end of `lower_call` is too late. The emitted C
   for a call such as `hook(bump(n), [n] (x) => x + n)` must read `n` at the
   callback's position, not after `bump`.
2. Replace `args[i]` with a plain `Path(cl_hooktramp_<id>)` value of the
   callback's function type. The materialized closure temporary is kept as an
   extra argument at the end of the argument list.
3. Append the closure temporaries in hook order as extra arguments. The
   emitter renders each extra argument as `(<temp>).env`.
4. Build a `HookWrapper` record and push it onto `OwnershipOutput`:

```rust
pub struct HookCallback {
    pub arg: usize,
    pub closure: ClosureTy,
    pub hook_id: usize,
}
pub struct HookWrapper {
    pub name: String,          // cl_hookwrap_<id>
    pub callee: String,        // the original ffi C name
    pub params: Vec<Ty>,       // original ffi parameter types
    pub modes: Vec<Mode>,      // original modes, for the language call
    pub returns: Ty,           // original ffi return type
    pub callbacks: Vec<HookCallback>,
}
```

4. Rename the call to `cl_hookwrap_<id>`, clear its type arguments, and leave
   the original modes in `LoweredCall.modes` so `take` arguments still move.
   The extra closure arguments are beyond the modes and have no move-out.
The wrapper id is a monotonically increasing counter in the ownership stage;
it is deterministic because functions are lowered in source order.

The original argument evaluation order is preserved: the closure value is
lowered and materialized at its own position, and the extra argument reads
that materialized value after all original arguments. No capture expression
is re-evaluated at the call.

## 4. Emission

### 4.1 Environment stack

Emitted once if any wrapper exists:

```c
struct ClHookEnvNode {
    unsigned long id;
    void *env;
    struct ClHookEnvNode *prev;
};
static struct ClHookEnvNode *cl_hook_env_top;
static void cl_hook_env_push(struct ClHookEnvNode *node, unsigned long id, void *env)
{
    node->id = id;
    node->env = env;
    node->prev = cl_hook_env_top;
    cl_hook_env_top = node;
}
static void cl_hook_env_pop(struct ClHookEnvNode *node)
{
    cl_hook_env_top = node->prev;
}
static void *cl_hook_env_find(unsigned long id)
{
    struct ClHookEnvNode *node = cl_hook_env_top;
    while (node) {
        if (node->id == id) { return node->env; }
        node = node->prev;
    }
    return 0;
}
```

The stack is a linked list of caller-frame nodes; no heap. A nested or
recursive call pushes a new node on top; `find` walks by id, so each
trampoline finds its own environment even when two callbacks are active in
one C call.

### 4.2 Trampoline

One per hook argument, emitted next to the lifted lambda:

```c
static LONG cl_hooktramp_7(LONG x)
{
    struct ClEnv_3 *env = (struct ClEnv_3 *)cl_hook_env_find(7);
    if (!env) {
        cl_trap("E11215", "callback invoked after its C hook returned");
    }
    return cl_lambda_3(env, x);
}
```

The C signature is exactly the declared `func(...)` signature. The lifted
lambda already takes the environment as its first parameter. A capture-less
lambda never reaches this path; it is passed as `cl_lambda_<site>` directly.

### 4.3 Wrapper

One per hook call site, emitted after the closure declarations:

```c
static LONG cl_hookwrap_12(LONG (*p0)(LONG), LONG p1, struct ClEnv_3 *env0)
{
    struct ClHookEnvNode n0;
    cl_hook_env_push(&n0, 7, env0);
    LONG r = cl_fixture_apply(cl_hooktramp_7, p1);
    cl_hook_env_pop(&n0);
    return r;
}
```

- The wrapper's parameters are the original `ffi` parameter types in their
  **ABI** spelling plus one `struct ClEnv_<site> *` per callback. The ABI
  spelling comes from `abi::ffi_type` (a contracted handle is `BPTR`, a struct
  is `ClStruct`, a scalar is its fixed-width typedef). The accepted hook
  signature set excludes views, so the language spelling and the ABI spelling
  agree for every accepted parameter; a function-pointer parameter needs a
  named ABI declarator helper. The wrapper forwards those values to the `ffi`
  function, so the outer call ABI is unchanged.
- The language-level call to the wrapper is rendered with the original `ffi`
  callee's ABI rules: `call_arguments`/`callee_modes` currently resolve only
  user and `ffi` callees, so the emitter must look up the wrapper record
  explicitly, render the original arguments with the original ffi callee's
  ABI, and append the environment pointers as `(<temp>).env`. This is why the
  wrapper record carries the original callee name and parameter types.
- The wrapper id is deterministic and unique; the wrapper record is looked up
  by name in both the prototype/definition pass and the call-rendering pass.
- The callback argument in the wrapper call is the trampoline pointer. The
  wrapper's `ffi` call uses the original C symbol.
- Push order follows the hook argument order; pop order is the reverse. A
  `take` argument was already moved by the language-level call, and the
  wrapper forwards the value; no second move occurs.
- A void-returning `ffi` function emits a void wrapper with no `r`.

## 5. Lifetime and drop

The environment is the existing per-lambda environment struct. Its fields are:

- a copied value for a duplicable capture (no drop);
- a pointer for a read-only or writable borrow (no drop).

It owns no droppable resource, so there is no environment drop glue and no
heap allocation. The borrow captures' loans end when the C call returns; the
caller's cleanup ladder then handles any ordinary temporaries. The stack node
is a caller-frame local and dies at the same point.

The C contract is synchronous. If C stores the callback and calls it after
returning, `cl_hook_env_find` returns null and the trampoline reports
`E11215` before touching the environment. This turns a contract violation into
a clean fault instead of a use-after-free.

## 6. Edge cases

| Case | Behavior |
|---|---|
| capture-less lambda at a `borrow` hook | plain lifted function pointer; no wrapper/trampoline/env/push |
| capturing lambda at an unmarked/`const`/`take` `func` | `E10400` |
| capturing lambda at a user `func` parameter | `E10400` |
| capturing lambda at `newHook`/`createTask`/`addHandler` | `E10400`; those APIs store the callback |
| two capturing callbacks in one C call | two trampolines, ids, nodes; wrapper pushes both and pops in reverse |
| nested hook calls | linked stack; each trampoline finds its own id |
| reentrant hook (C calls back into Clorraine, which calls another hook) | same linked stack; ids remain unique |
| callback invoked after return | `E11215` |
| hook call in a short-circuit/conditional operand | the call is effectful and materializes on the evaluated path; the wrapper runs only there |
| hook call in a loop body | one env temp and node per iteration; no accumulation |
| hook call in a module initializer | capture-less only; a capturing capture is `E10406` |
| capturing hook in a generic body | `E10400`; use a non-generic wrapper |
| capturing lambda at a generic callable bound (`boxWith`) | accepted; not a C hook |
| callback parameter is a view/fixed array, or a fixed-array return | `E10210`; not ABI-identical |
| callback parameter is a `borrow` move-only struct | `E10210`; language ABI is `T *` |
| effectful sibling argument before/after the callback | the capture is materialized at its own position; the sibling's order is preserved |
| hook signature mismatch | `E10210` |
| argument not callable | `E10200` |

## 7. Host and m68k proof

A test-only C fixture lives under `compiler/tests/fixtures/`. It defines
synchronous functions that call the callback immediately:

| Fixture | Purpose |
|---|---|
| `cl_fixture_apply` | one callback, one scalar argument |
| `cl_fixture_apply2` | two callbacks in one call |
| `cl_fixture_reenter` | callback that calls a Clorraine function which calls another hook |
| `cl_fixture_repeat` | call the same callback twice |
| `cl_fixture_defer` | store the callback and call it after returning (contract violation; expects `E11215`) |

The host leg compiles the emitted C and the fixture with `cc -std=c99 -Wall
-Wextra -Werror` and runs it. The m68k leg builds the same pair with
`m68k-amigaos-gcc` (`-mcrt=nix20`, `-m68000`) and runs it under `vamos` when
the toolchain is present; the fixture selects `LONG`/`UBYTE` from
`<exec/types.h>` on m68k and fixed-width `int` types on the host. A missing
toolchain skips the leg with a note, as the existing Amiga suites do.

## 8. Rejected alternatives

| Alternative | Why rejected |
|---|---|
| A new FFI clause keyword | X3 removed the draft clause words; `borrow` already means call-scoped |
| A hidden global "current environment" | Breaks nesting and two callbacks in one call |
| A heap environment | The goal forbids it |
| An extra environment parameter in the C callback ABI | Breaks every existing C hook and the frozen function-pointer type |
| Copying the environment per trampoline | Unnecessary; the caller frame already owns it |
| Emitting push/pop as raw IR statements | The IR has no raw statement form and guarded operands would duplicate the sequencing; a generated wrapper keeps the language call normal |
| Supporting capturing hooks in generic bodies | Requires a per-instance wrapper/trampoline record; deferred with a clear `E10400` |

## 9. Blast radius

| Area | Sites |
|---|---|
| Sema | `ffi_parameter_types` mode vector, `check_ffi_arguments` func branch, `SemaOutput::hook_calls` |
| Lowering | `lower_call` post-processing, `HookWrapper` record, wrapper id counter |
| IR/output | `OwnershipOutput::hook_wrappers` |
| Emitter | env stack, trampolines, wrappers, wrapper-call argument rendering, prototypes |
| ABI | named ABI declarators for wrapper parameters |
| Diagnostics | `E10400` help, `E11215` registry, `E10105` FFI aggregate check |
| Tests | spec suite behavior, impl suite emission pins, C fixtures, host and m68k builds |
