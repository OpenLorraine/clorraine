# Chapter 13 - Data Inclusion

> **Status:** stable (v1.0)
> **Depends on:** Ch 08 (arrays and strings), Ch 14 (diagnostics), Ch 15 (profiles), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) section 7 (intrinsic calls)
> **Ledger:** GF22 (staged `comptime`), P13 (determinism); [grammar-deltas.md](grammar-deltas.md) sections 3 and 5

## 1. The `embed` intrinsic

`embed` produces a fixed byte array whose contents are read from a file during
the build and baked into the binary. Nothing is loaded at run time.

```ebnf
embed_expr = "embed" , "(" , string_literal , [ "," , string_literal ] , ")" ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| DI-1 | `embed("path")` produces a fixed array of bytes from the named file | - |
| DI-2 | `embed("path", "key")` carries a caller-chosen key with the same bytes | - |
| DI-3 | The file is read at build time; there is no runtime loader | - |
| DI-4 | `embed` is allowed only in a constant context | E10213 |
| DI-5 | The path is relative to the source file and inside the project root (an absolute path or `..` escape is rejected), and the file must be readable at build time | E10214 |
| DI-6 | One call embeds one file; directories are not accepted | - |

The optional second string is a caller-chosen label; the grammar accepts it,
and the ledger assigns it no further semantics. The Blend65 `loadable const`
form is gone ([grammar-deltas.md](grammar-deltas.md) section 3); a module
`const` covers the use case.

An embedded path is resolved relative to the source file that contains the
`embed` call and must stay inside the project root. The **project root** is the
directory of the root module — the build's source root. An absolute host path
or a `..` escape is E10214; the restriction keeps the build deterministic and
portable across machines (P13).

A module-level `const` writes an explicit type (V2); a local `const` infers its
type from the embedded array:

```clorraine
// A module-level const needs an explicit type (V2, M7).
const LEVEL: u8[1024] = embed("data/level1.lvl");
const SPRITE: u8[512] = embed("gfx/sprite.bin", "player");

// A local const infers the fixed array type from the file.
func checksum(): u32 {
    const logo = embed("images/logo.bin");
    let sum: u32 = 0;
    for (b in logo) {
        sum = sum + b;
    }
    return sum;
}
```

A module-level initializer is also a constant context (E10213), so an embedded
asset can back a module-level `let`:

```clorraine
// A module-level let must be duplicable and non-nullable (M7, O10).
let palette: u8[64] = embed("gfx/palette.bin");
```

## 2. Compile-time size and duplicability

`embed` yields a fixed array, so `len` is a compile-time constant (GF14):

```clorraine
const LEVEL: u8[1024] = embed("data/level1.lvl");
const LEVEL_SIZE: usize = len(LEVEL);    // compile-time; no runtime cost
```

An embedded array is an ordinary fixed array of bytes, so it is duplicable (V8)
and composes with `sizeof`, `offsetof`, and constant indexing. Bound with
`const`, it lands in the read-only data segment; bound with a module-level
`let`, it stays writable. Because the extent is fixed, `embed` never needs a
fill literal and never raises E10215, the diagnostic for a fill literal without
a fixed extent (see [Ch 08](08-arrays-strings.md)).

## 3. Determinism

Embedded bytes come from the file, not from the environment. The generated C is
deterministic: the same source, the same files, and the same profile produce
byte-identical output, with no timestamps and no absolute host paths (P13,
[Ch 17](17-codegen.md) section 11).

## 4. `comptime` transforms

A `comptime func` can transform embedded data at compile time. The syntax is
frozen now; the semantics are staged, so the semantic stage reports E90001
(GF22).

```clorraine
comptime func bake(data: u8[]): u8[] {
    // E90001: comptime semantics are not implemented yet
    return data;
}
```

## 5. Large assets

An embedded asset becomes part of the image, so its size is a build decision.

| Rule | Statement | Diagnostic |
|---|---|---|
| DI-7 | A build-time size report lists every embedded blob and the total | - |
| DI-8 | There is no compression in v1 | - |
| DI-9 | Each call embeds exactly one file; no directory embedding | - |

The size report makes the cost visible before the image is linked.

## 6. Placement

Placement and profiles are [Ch 15](15-profiles.md) territory. The hosted
profile puts read-only embedded data in the normal data segment. A later
profile feature may place data in specific regions (the Amiga appendix).

## 7. Diagnostics

| Code | Trigger |
|---|---|
| E10213 | `embed` outside a constant context |
| E10214 | an embedded path is absolute or escapes the project root, or the file cannot be read at build time |
| E10215 | array fill literal without a fixed extent (never raised by `embed`; see Ch 08) |
| E90001 | `comptime` semantics are staged |

All message text lives in [Ch 14](14-diagnostics.md).
