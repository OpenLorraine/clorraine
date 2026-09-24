# Screen Lifecycle: Clorraine Amiga Graphics

> **Document**: 03-01-screen-lifecycle.md
> **Parent**: [Index](00-index.md)

## Overview

`std.amiga.intuition` owns the `Screen` handle: opening a custom screen and
releasing it. It holds no drawing calls; those live in
[03-02-drawing.md](03-02-drawing.md). The module reaches the profile C shim
through `ffi`, so application code never writes `ffi` and never touches
`unsafe`. (AR-5, AR-6)

## Architecture

### Current Architecture

The spike (`spikes/05-graphics/`) fused `openScreen`, `setColor`, `fill`, and the
`Screen` handle into one provisional module. Nothing is registered in the
production `SOURCES`/`MODULES` tables.

### Proposed Changes

Create `stdlib/std.amiga.intuition.clo` with the `Screen` handle and
`openScreen`, and register it in `compiler/src/stdlib.rs`. Drawing moves to the
companion module ([03-02](03-02-drawing.md)). (AR-1, AR-5)

## Implementation Details

### New Types/Interfaces

```clorraine
module std.amiga.intuition;

/// An owned custom screen. The caller releases it through the generated drop.
export handle Screen freedBy cl_close_screen;

ffi stdintuition {
    func cl_open_screen(width: i32, height: i32, depth: u8, title: cstring): Screen?;
    func cl_close_screen(take screen: Screen): void;
}

/// Opens a custom screen, or returns `null` on failure.
export func openScreen(width: i32, height: i32, depth: u8, title: cstring): Screen? {
    return cl_open_screen(width, height, depth, title);
}
```

### New Functions/Methods

| Name | Signature | Contract |
|---|---|---|
| `openScreen` | `(width: i32, height: i32, depth: u8, title: cstring): Screen?` | Opens a custom screen; `null` on failure. `width`/`height` are pixels; `depth` is the bitplane count. (AR-8, AR-12, AR-16) |
| drop (generated) | `cl_close_screen(take screen: Screen): void` | Exactly once; null-safe. (AR-6, AR-17) |

### Integration Points

- `compiler/src/stdlib.rs` — `SOURCES` gains the embedded body; `MODULES` gains
  the exports `["Screen", "openScreen"]`. (AR-5)
- `compiler/src/profile.rs` — no change; `supports_module` already rejects
  `std.amiga.*` on host and freestanding. (AR-25)
- [03-03-runtime-shim.md](03-03-runtime-shim.md) — `cl_open_screen` and
  `cl_close_screen`. (AR-9, AR-23)

## Code Examples

### Example 1: open and drop

```clorraine
import { Screen, openScreen } from std.amiga.intuition;

func main(): i32 {
    let screen: Screen? = openScreen(320, 256, 4, "Clorraine");
    if (screen == null) {
        return 1;
    }
    return 0;   // the screen drops here
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Screen open fails | `openScreen` returns `null`; the caller null-checks (existing nullable rules) | AR-16 |
| Null handle used before the check | Existing `E10010` nullable-use diagnostic | AR-16 |
| Screen used on the host profile | `E11200` profile-surface rejection before codegen | AR-20, AR-25 |
| Drop runs on a `null` screen | `cl_close_screen` is null-safe | AR-17 |

> **Traceability:** Every error-handling strategy and design choice references
> the Ambiguity Register entry that resolved it; see
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: host-profile rejection of the corpus program (`E11200`);
  hosted-amiga slice compiles with no diagnostics. ([07](07-testing-strategy.md) ST-1–ST-3)
- Implementation tests: the `stdlib` table exports and embedded-source
  consistency (already covered by the existing `stdlib` unit tests) plus a new
  check that `std.amiga.intuition` is registered. (AR-11, AR-22)
