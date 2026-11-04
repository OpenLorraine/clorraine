# Chapter 10 - Modules

> **Status:** stable (v1.0)
> **Depends on:** Ch 03 (ownership), Ch 06 (functions), Ch 12 (FFI and contracts), Ch 14 (diagnostics), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 2.1-2.3 (source structure, module declaration, imports), 3.5 (module-level values), 3.6 (FFI blocks)
> **Ledger:** [decisions.md](decisions.md) section 12 (M1-M10); plus 3 (E6), 4 (V2), 5 (O10), 11 (X6), 15 (P9)

A program is a set of files. Each file belongs to a **module**, and the module
name is the namespace for the declarations in it. A filename carries no
meaning. Declarations live at module level and executable code lives inside
functions. This chapter defines the module system and the initialization order
that runs before `main`.

## 1. The module declaration

Every file begins with exactly one module declaration, and it must be the first
declaration in the file (M1).

```ebnf
module_decl = "module" , qualified_name , ";" ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-1 | One `module A.B;` declaration per file, as the first declaration | E11105 |
| MOD-2 | The name is a dotted path (`A.B`); filenames have no semantic meaning | - |

```clorraine
module Demo.MinMax;
```

A missing declaration, or one that is not first, is E11105. The dotted name is
the full module identity: two files declare the same module only when they
write the same dotted name (section 3).

## 2. Declarations and visibility

Module level holds declarations only: `func`, `comptime func`, `struct`,
`enum`, `handle`, module `let` and `const`, `ffi` blocks, and `intrinsic`
declarations. Statements are not allowed there; run code inside functions.

`export` marks a declaration public. The default is module-private: another
module sees the name only after an import, and only if the declaration is
exported (M3).

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-3 | Module level holds declarations only; executable code lives in functions | - |
| MOD-4 | `export` makes a declaration public; the default is module-private | - |
| MOD-5 | A name imported from another module must be exported by that module | E11102 |
| MOD-6 | `ffi` blocks are module-scoped and are never exported | E10103 |

```clorraine
module Demo.MinMax;

export const LIMIT: i32 = 100;      // visible to importers

func tooBig(n: i32): bool {         // module-private
    return n > LIMIT;
}
```

A `handle` declaration is public only if it is exported, because application
code must name the type. [Ch 12](12-ffi-contracts.md) owns the `ffi` and
`handle` rules; ownership of the resource is [Ch 03](03-ownership.md).

## 3. Declaration merging

Several files may declare the same module. Their declarations merge into one
module (M2). Because a filename has no meaning, the merge is defined by the
`module` declarations, not by the directory. A name declared twice in the
merged module is E11101.

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-7 | Files that declare the same module name merge into one module | - |
| MOD-8 | A name declared twice in the merged module is an error | E11101 |

```clorraine
// File: util_math.clo
module Demo.Util;

export func add(a: i32, b: i32): i32 { return a + b; }
```

```clorraine
// File: util_text.clo
module Demo.Util;

export func trim(s: str): str { return s; }   // same module, merged
```

The two files form one module `Demo.Util` with `add` and `trim`.

## 4. Imports

An import brings named declarations into scope. Only a named list is allowed;
there are no wildcard imports. `as` renames a binding and is a contextual
keyword (grammar 10.3).

```ebnf
import_stmt = "import" , "{" , import_list , "}" , "from" , qualified_name , ";" ;
import_item = identifier , [ "as" , identifier ] ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-9 | Imports use named lists: `import { X, Y as Z } from A.B;` | - |
| MOD-10 | There is no wildcard import | - |
| MOD-11 | An imported name must be declared and exported in the target module | E11102 |
| MOD-12 | Binding one name twice in a module is an error; rename with `as` | E11103 |
| MOD-13 | Qualified access `A.B.f()` is always available, with or without an import | - |
| MOD-14 | Circular imports are allowed; declarations resolve globally | - |

```clorraine
module Demo.App;

import { clamp, LIMIT as MAX } from Demo.MinMax;

let seed: i32 = 7;
```

An unused import warns as W10006. Qualified access needs no import:

```clorraine
let low: i32 = Demo.MinMax.clamp(-5);   // no import required
```

Because declarations resolve globally (M10), two modules may import each other.
A circular import creates no initialization cycle by itself; only
runtime-initialized globals can do that (section 6).

## 5. Name resolution

Names resolve in lexical scopes. A scope is a block (a function body, an `if`,
a loop, or a brace block), and function parameters belong to the body's scope.
The innermost enclosing scope wins, and a name must be declared before the point
of use in its block; an earlier reference is E10219. A local may shadow an outer
local or a module name.

```clorraine
module Demo.Shadow;

let LIMIT: i32 = 100;

func pick(LIMIT: i32): i32 {       // the parameter shadows the module value
    let doubled: i32 = LIMIT * 2;  // LIMIT here is the parameter
    let LIMIT: i32 = doubled;      // the local shadows the parameter
    return LIMIT;
}
```

At module level, declarations are order-independent (M10): a function may call
one declared later in the same module, and an initializer may name a later
declaration. Imports bind names in module scope, may rename a binding with `as`,
are order-independent, and each binding must resolve to a declared, exported
name (E11102, E11103); section 4 owns the import syntax. Qualified access
`A.B.x` is always available, with or without an import (MOD-13). Files that
declare the same module share one namespace, so a duplicate name is E11101
(section 3).

Built-in intrinsic names are reserved: only an `intrinsic func` declaration may
bind them, and an ordinary declaration of a reserved name is E20006
([Ch 01](01-lexical.md), [Ch 06](06-functions.md)). Dot-call (`x.f(y)`) resolves
under the candidate rule in [Ch 04](04-expressions.md) section 8.

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-15 | Names resolve in lexical scopes; the innermost enclosing scope wins, and a name is declared before use in its block | E10219 |
| MOD-16 | A local may shadow an outer local or a module name | - |
| MOD-17 | Module-level declarations are order-independent; imports bind names in module scope, may rename with `as`, are order-independent, and must resolve | E11102, E11103 |
| MOD-18 | Built-in intrinsic names are reserved; only an `intrinsic func` declaration binds them | E20006 |

## 6. Module-level values and initialization

Module-level `let` and `const` declarations require an initializer (grammar
3.5). `let` is a mutable global; `const` is immutable and, when its initializer
is a constant expression, a compile-time value (V2).

There are **no global constructors** and no C constructor attributes (M5). The
compiler collects module-level initializers into generated
`cl_init_<module>` functions. A generated entry runs them before `main`, in the
topological order of init dependencies: a module initializes after the modules
it imports and after every module its initializer expressions name, transitively.
Independent modules are ordered by their fully qualified module name, compared
byte-wise (lexicographic), so the order is deterministic. A reference inside a
called function body does not create an init edge.

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-19 | There are no global constructors and no C constructor attributes | - |
| MOD-20 | Module initializers become generated `cl_init_<module>` functions run before `main` | - |
| MOD-21 | Initialization follows the topological order of init dependencies (imports plus modules named by initializer expressions, transitively); ties break on the fully qualified module name, byte-wise | - |
| MOD-22 | References inside called function bodies do not create init edges | - |
| MOD-23 | A dependency cycle involving runtime-initialized globals is an error | E11100 |
| MOD-24 | A cycle of `const`-only modules is allowed | - |
| MOD-25 | A global must be duplicable or `const`; a module-level `let` with a non-duplicable initializer is an error | E11106 |
| MOD-26 | Module-level initializer expressions must be non-nullable (M7a) | E10200 |
| MOD-27 | Runtime work that can fail belongs in an explicit `init()` function called from `main` | - |

A `const` whose initializer is constant folds at compile time, so it adds no
runtime step. A module-level `let` must hold a duplicable value (O10): a handle
or other move-only value cannot be a global, because a global has no drop point
at exit. A module-level initializer may not produce an empty (`null`) value:
startup either yields a real value, or the failure is reported by an explicit
`init()` function. That function returns `bool` or a status struct and is
called from `main` (E6).

```clorraine
module Demo.Config;

export let debug: bool = false;   // duplicable; set before main
export const MAX: i32 = 256;      // compile-time value
```

```clorraine
module Demo.Storage;

let ready: bool = false;

export func init(): bool {
    // runtime work that can fail lives here, not in an initializer
    ready = true;
    return true;
}
```

```clorraine
module Demo.App;

import { init as startStorage } from Demo.Storage;

func main(): i32 {
    if (!startStorage()) {
        return 1;
    }
    return 0;
}
```

## 7. Entry points and builds

An executable has exactly one entry function, `func main(): i32` (M8). A
missing, duplicated, or mistyped `main` is E11104. A library build has no
`main`; it exports a generated entry that the host calls, and that entry runs
the module initializers first.

| Rule | Statement | Diagnostic |
|---|---|---|
| MOD-28 | An executable has exactly one `func main(): i32` | E11104 |
| MOD-29 | A library build exports a generated init entry the host calls | - |

The generated names and the emitted C are in [Ch 17](17-codegen.md).

## 8. FFI blocks

An `ffi` block declares external C functions. It is module-scoped and is never
exported (M9); an `export` on the block is E10103. Share external calls through
an exported wrapper function. The contract vocabulary is in
[Ch 12](12-ffi-contracts.md).

## 9. Worked example

Two files, one module pair:

```clorraine
// File: minmax.clo
module Demo.MinMax;

export const LIMIT: i32 = 100;

export func clamp(n: i32): i32 {
    if (n < -LIMIT) {
        return -LIMIT;
    }
    if (n > LIMIT) {
        return LIMIT;
    }
    return n;
}
```

```clorraine
// File: app.clo
module Demo.App;

import { clamp, LIMIT as MAX } from Demo.MinMax;

let seed: i32 = 7;                        // module-level let: duplicable

func main(): i32 {
    let a: i32 = clamp(seed);                  // imported name
    let b: i32 = Demo.MinMax.clamp(-200);      // qualified access, no import
    let c: i32 = seed * MAX;                   // imported under an alias
    if (a + b + c == 0) {
        return 1;
    }
    return 0;
}
```

## 10. Diagnostics

| Code | Trigger |
|---|---|
| E10103 | An `ffi` block is exported or leaks into application code |
| E10200 | Module-level initializer is nullable |
| E10219 | A name is used before its declaration in the block |
| E11100 | Initializer dependency cycle |
| E11101 | Duplicate declaration in a merged module |
| E11102 | Import name not found or not exported |
| E11103 | Duplicate import binding |
| E11104 | Entry-point problem |
| E11105 | Module declaration missing or not first |
| E11106 | Module-level `let` with a non-duplicable initializer |
| E20006 | Reserved built-in name redeclared |
| W10006 | Imported name is never used |

All message text lives in [Ch 14](14-diagnostics.md).
