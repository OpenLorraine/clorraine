# Volatile and MMIO: Clorraine v1 language completion

> **Document**: 03-03-volatile-mmio.md
> **Parent**: [Index](00-index.md)

## Overview

This component completes T7: the six volatile intrinsics, the `volatile`
qualifier on module-level `let`, and the per-chip MMIO wrapper module. Volatile
overlay struct fields already emit C `volatile` members; the six intrinsics
currently lower to the literal `0`, and a `volatile let` global loses its
qualifier (AR-19, AR-20).

## Architecture

### Current Architecture

- The six names are reserved built-ins (`compiler/src/token.rs:176-189`).
- The unsafe gate exists: a volatile intrinsic outside `unsafe` is `E10604`
  (`compiler/src/own/expr.rs:130`, `compiler/src/ffi.rs:159`).
- `sema::check_intrinsic` has no case for them and returns `Ty::unknown()`
  (`compiler/src/sema/expr.rs:599-601`).
- `codegen::intrinsic` has no case and returns `"0".to_owned()`
  (`compiler/src/codegen/calls.rs:204`).
- `emit_struct` writes a `volatile` field qualifier
  (`compiler/src/codegen/program.rs:266`), but `emit_globals` writes
  `static {ty} {symbol} = ...` with no qualifier even when the declaration is
  `volatile let` (`compiler/src/codegen/program.rs:366-380`).

### Proposed Changes

1. Type the six intrinsics.
2. Lower them to a C `volatile`-qualified access.
3. Thread and emit the global `volatile` flag.
4. Add the `std.amiga.custom` wrapper module.

## Implementation Details

### New Functions/Methods

- `sema/expr.rs::check_intrinsic` gains cases:
  - `readVolatile8/16/32(addr)` → requires the argument convertible to
    `raw u8`/`raw u16`/`raw u32` and returns `u8`/`u16`/`u32`.
  - `writeVolatile8/16/32(addr, value)` → requires the address as above and the
    value convertible to the matching unsigned type; returns `void`.
  A mismatched argument is reported with the existing conversion diagnostic.
- `codegen/calls.rs::intrinsic` gains cases:
  - `readVolatileN` → `(*(volatile <uN> *)({addr}))`.
  - `writeVolatileN` → `(*(volatile <uN> *)({addr}) = ({value}))`.
  The C `volatile` qualifier is what forbids folding, caching, and reordering
  (T7); the access is native-width, so no byte swap is emitted.
- `ir/global.rs` carries a `volatile: bool`; `sema/items.rs` sets it from the
  AST `GlobalLetDecl.volatile` (`compiler/src/ast/mod.rs:278`);
  `codegen/program.rs::emit_globals` prefixes the C qualifier when it is set
  (AR-20).
- `stdlib/std.amiga.custom.clo` (new) declares an overlay struct with `volatile`
  fields, a `const` `raw` register base, and one exported safe function per
  register operation (AR-28). `compiler/src/stdlib.rs` embeds the module and
  adds its exported names; `Profile::supports_module` already restricts every
  `std.amiga.*` module to hosted-amiga (`compiler/src/profile.rs:73-79`).

### New Types/Interfaces

No new Rust type is introduced. The module's Clorraine surface follows
`spec/appendix-amiga.md` section 4:

```clorraine
module std.amiga.custom;

struct CustomRegs {
    dmacon: volatile u16;
    cop1lc: volatile u32;
}

const CUSTOM_BASE: raw CustomRegs = raw CustomRegs(0x00DFF000);
const COP_DMA: u16 = 0x0080;

export func setDma(enabled: bool): void {
    unsafe {
        if (enabled) {
            CUSTOM_BASE.dmacon = CUSTOM_BASE.dmacon | COP_DMA;
        } else {
            CUSTOM_BASE.dmacon = CUSTOM_BASE.dmacon & (0xFFFF - COP_DMA);
        }
    }
}

export func setColor(index: u32, value: u16): void {
    unsafe {
        writeVolatile16(raw u16(0x00DFF180) + index, value);
    }
}
```

### Integration Points

- The stdlib module registry is the single place a module becomes importable;
  the per-profile gate already rejects it on `host` and `freestanding`.
- The volatile field qualifier path is shared with overlay structs and is
  unchanged.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Volatile intrinsic or overlay access outside `unsafe` | `E10604` (existing gate) | AR-19 |
| Intrinsic argument is not the matching `raw` pointer or value type | the existing conversion diagnostic | AR-19 |
| `std.amiga.custom` imported on `host` or `freestanding` | `E11200` (existing profile gate) | AR-7 |
| A `volatile` field or global in a misplaced position | `E10208` (existing) | AR-20 |

> **Traceability:** every strategy cites the register entry that resolved it;
> see `00-ambiguity-register.md`.

## Testing Requirements

- Host specification test: `readVolatile8/16/32` and `writeVolatile8/16/32`
  against a real local address, asserting round-trip semantics and that a
  volatile read is not folded across a write (AR-27).
- Host compile-fail: the existing `E10604` golden stays asserted.
- Hosted-amiga: a corpus program under `spec/tests/corpus/volatile-mmio/` that
  imports and calls the `std.amiga.custom` wrapper, wired into the corpus leg's
  running-program set (`docker/scripts/corpus-amiga.sh`) with a matching case in
  `compiler/tests/corpus_spec_test.rs`, and run under `vamos`/FS-UAE (R17, AR-27).
- Implementation tests for the global `volatile` qualifier and the module's
  registered names.
