# FreedBy Design: Ordinary Clorraine Deleters (X1)

> **Document**: 03-02-freedby.md
> **Parent**: [Index](00-index.md)
> **Requirements**: R7-R9

## 1. Current check

`compiler/src/ffi.rs:65-77` validates a handle's deleter through
`program.ffi_function` only:

```rust
fn deleter_is_valid(deleter: &str, handle: &str, program: &ProgramInfo) -> bool {
    let Some(signature) = program.ffi_function(deleter) else {
        return false;
    };
    signature.params.len() == 1
        && signature.params[0].mode == Mode::Take
        && type_name_matches(&signature.params[0].ty, handle)
        && matches!(signature.returns.kind, TypeKind::Void)
        && !signature.returns.nullable
}
```

An ordinary function with the same signature is rejected with `E10102`.

## 2. Resolution

The check becomes a module-scoped lookup, not a program-wide one:

1. The handle's declaring module is recorded in `HandleInfo` (the collector
   already iterates modules; add `module`).
2. A qualified deleter resolves in the named module.
3. A simple deleter resolves in the declaring module's own functions first,
   then its `import` bindings (`module.import(name)` gives the original name
   and target module), then a unique program-wide ordinary function. More than
   one candidate is invalid.
4. The `ffi` path keeps the existing module-qualified lookup.

The validator and the emitter share one recorded identity:

```rust
pub struct HandleInfo<'a> {
    pub name: String,
    pub module: String,
    pub deleter: DeleterRef<'a>, // { module, simple_name, kind: User|Ffi }
    pub span: Span,
    pub decl: &'a HandleDecl,
}
```

`ProgramInfo::collect` resolves the deleter once and stores it. The validator
uses the resolved declaration; the emitter uses its `(module, simple_name,
kind)`. There is no second lookup in the emitter and no caller-scope fallback.

The parameter type is compared as a resolved `Ty`, not by spelling:
`resolve_ty_folded(&param.ty, &[], table, declaring_module, loaded)` must equal
`Ty::handle(name)`. This accepts an import alias and rejects a same-named type
from another module. The return type is checked as `void` and non-nullable.

The validator is shared over both declaration shapes. The ordinary
`FuncDecl` stores `params: Vec<Param>` and `returns: Type`; the `ffi`
`FuncSignature` stores the same shapes.

Generic functions (`type_params` non-empty) are rejected in v1.1: the handle
name supplies no type arguments, so the instance is not determined. The
generic-instance coverage comes from `Box<T>`, whose generated drop helper
calls the compiler-generated release path for each concrete element.

## 3. One declaration per type

`load/mod.rs` reports duplicates per module only, and the sema collision check
covers structs but not handles. The current `ProgramInfo::collect` inserts
handles with last-wins, while `deleter_symbol` uses the first module's
declaration. Two modules can declare the same handle with different deleters;
no `E11101` is reported, and the validator can check one declaration while the
emitter calls the other.

The fix records handle origins like `struct_origins`:
`TypeTable` gains a `handle_origins: BTreeMap<String, Vec<(String, Span)>>` (or
`ProgramInfo` gains the equivalent), and the sema/load duplicate check reports
`E11101` at the second declaration from a different module. The first
declaration's resolved deleter is the one stored and used.

## 3.1 Qualified names and emission

`HandleInfo.deleter` stores `(module, simple_name, kind)` rather than a dotted
string or a flattened `segments.join("_")`. The emitter's `deleter_symbol`:

1. for a user function, calls `function_symbol(module, simple_name)` directly;
2. for an `ffi` function, uses the simple C name;
3. for `String`, keeps `FreeString`.

It must not pass a dotted name to `function_symbol` (which would produce
`cl_A_B_A_B_freeX`) and must not emit a dotted name as a C symbol. The current
`handle_deleter` first-module scan and the `resolve_function` caller-scope
fallback are removed; the recorded identity is authoritative.

## 4. Drop path

The drop path is unchanged for a handle:

```c
if (value) { <deleter>(value); }
value = NULL;
```

The only difference is the symbol:

| Deleter kind | Symbol |
|---|---|
| same-module ordinary | `cl_<module>_<name>` |
| cross-module ordinary | `cl_<module>_<name>` |
| `ffi` | the declared C name |
| `String` | `FreeString` |

The call is null-safe and appears once in the cleanup ladder. A move-only
handle remains move-only; the deleter is not part of duplicability.

## 5. Deleter body

The deleter body is an ordinary function body, checked by the same ownership,
borrow, and drop rules as any other function. Two shapes are supported and
tested:

1. **Forwarding deleter.** The body calls the underlying `ffi` release:

   ```clorraine
   func freeNode(take node: NodeHandle): void {
       cl_free_node(node);
   }
   ```

2. **Structural deleter.** The body moves the owned payload out of the opaque
   handle and lets the ordinary drop walk release its fields:

   ```clorraine
   func freePayload(take p: PayloadHandle): void {
       unsafe {
           let payload: Payload = *raw Payload(p);
           // payload drops at scope end, releasing its String
       }
       cl_free_payload(p);
   }
   ```

The second shape is the X1 answer to "a deleter that itself drops owned
fields". The raw dereference is `unsafe`; the local's drop is the ordinary
structural drop; the final `ffi` call releases the block. ASan proves there is
no leak and no double free.

## 6. Rejection table

All rejected forms keep the registered `E10102` first line
`'{name}' is not a valid deleter for '{type}'` and gain the help
`declare a function taking 'take T' and returning 'void'`.

| Form | Trigger |
|---|---|
| unresolved name | `E10102` |
| `intrinsic` | `E10102` |
| zero/two params | `E10102` |
| non-`take` mode | `E10102` |
| parameter type not `T` | `E10102` |
| non-`void` return | `E10102` |
| nullable return | `E10102` |
| generic function | `E10102` |
| second handle declaration | `E11101` |
| `freedBy` on a struct | `E10104` |

`E10102`'s registry trigger/help is updated; the code and first line stay
stable.

## 7. Interaction with `Box<T>`

`Box<T>` uses the compiler-generated release path
(`cl_box_drop__<key>` → `cl_box_free`), not a textual `freedBy` declaration.
The X1 amendment is still exercised by:

- the standard library's `Buffer` and other handles remain `ffi` deleters;
- the new tests define a user handle with an ordinary deleting function;
- the recursive `Node` test uses an ordinary deleter for the user handle and
  `Box<Node>?` for the link.

This keeps the two surfaces independent: a Box works without a user handle,
and a user handle works without a Box.

## 8. Test matrix

| Case | Expectation |
|---|---|
| same-module ordinary deleter | accepted; drop calls the mangled symbol |
| exported cross-module deleter (`A.B.freeX`) | accepted; drop calls `cl_A_B_freeX` |
| cross-module simple-name deleter via import | accepted; resolves in the handle's module scope |
| aliased parameter type (`take f: F` where `F` is imported `File`) | accepted; resolved `Ty` comparison |
| same-named type from another module | `E10102` |
| two modules declare the same handle with different deleters | `E11101` |
| `ffi` deleter (regression) | accepted; drop calls the C symbol |
| wrong mode / wrong type / extra param | `E10102` |
| non-void / nullable return | `E10102` |
| unresolved / `intrinsic` name | `E10102` |
| generic function | `E10102` |
| second handle declaration in one module | `E11101` |
| ordinary deleter drops a `String` field | ASan clean, one release |
| handle copied after declaration | `E10020` |
| deleter body calls an `ffi` release | run, ASan clean |
