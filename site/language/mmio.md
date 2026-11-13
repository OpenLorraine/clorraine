---
title: Volatile and MMIO
description: "Overlay types, volatile fields, raw access, and the per-chip wrappers that keep Amiga hardware access auditable."
---

# Volatile and MMIO

Amiga programming eventually reaches hardware registers. Clorraine keeps those
accesses explicit and auditable instead of making every pointer volatile by
default.

## Volatile fields

An **overlay type** is a struct with at least one `volatile` field. The field is
emitted as a C `volatile` member; the compiler does not fold, cache, or reorder
the access. `volatile` in any other position is rejected (`E10208`).

```clorraine
struct Custom {
    volatile color: u16;
    volatile dmacon: u16;
}
```

An overlay type is never an ordinary value. It is reached through `raw` at the
hardware boundary, which keeps the address arithmetic in a small, reviewable
place.

## Per-chip wrappers

The standard library wraps the common registers in named functions. Application
code calls the wrapper, not the raw address:

```clorraine
module demo.Mmio;

import { setColor } from std.amiga.custom;

func main(): i32 {
    setColor(0, 0x0F00);
    return 0;
}
```

`setColor` writes the volatile register inside `std.amiga.custom`. The `unsafe`
block and the raw address stay in the library:

```clorraine
export func setColor(index: u32, value: u16): void {
    unsafe {
        writeVolatile16(raw u16(0x00DFF180) + index, value);
    }
}
```

## What you get

| Guarantee | Meaning |
|---|---|
| No elision | A volatile write is not removed because nothing reads it |
| No reordering | The compiler does not move the access across another volatile access |
| One address | The register address appears once, in the wrapper |
| Auditable boundary | Every `unsafe` and `raw` use is visible in the module that owns it |

## Profile availability

The hardware wrappers are Amiga-profile APIs. The `host` profile does not
provide them, and the 1.3 profile provides the subset that Kickstart 1.3
supports. Selecting an unavailable construct is `E11200`.

## Learn more

- [Chapter 02 — Types](https://github.com/OpenLorraine/clorraine/blob/main/spec/02-types.md)
- [Chapter 07 — Structs](https://github.com/OpenLorraine/clorraine/blob/main/spec/07-structs.md)
- [Chapter 15 — Platform Profiles](https://github.com/OpenLorraine/clorraine/blob/main/spec/15-profiles.md)
- [Chapter 17 — Code Generation](https://github.com/OpenLorraine/clorraine/blob/main/spec/17-codegen.md)
