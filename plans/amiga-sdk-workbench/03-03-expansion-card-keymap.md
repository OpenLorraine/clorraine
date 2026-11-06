# Design: Expansion, Card, and Keymap

> **Document**: 03-03-expansion-card-keymap.md
> **Parent**: [Index](00-index.md)
> **Covers**: R3, R8 (expansion part), AR-7, AR-8

## Public API

```clorraine
module std.amiga.expansion;

ffi stdexpansion {
    func cl_expansion_board_count(): i32;
    func cl_expansion_board_manufacturer(index: i32): u16;
    func cl_expansion_board_product(index: i32): u8;
    func cl_expansion_board_serial(index: i32): u32;
    func cl_expansion_board_kind(index: i32): u8;
    func cl_card_present(): bool;
    func cl_card_status(): u32;
    func cl_map_raw_key(code: u16, qualifier: u16): String?;
}
```

The module is available only on `hosted-amiga`.

## Expansion Boards

`cl_expansion_board_count` opens `expansion.library` 37 lazily, assigns the NDK
`ExpansionBase` global, and iterates `FindConfigDev(NULL, -1, -1)` with
`FindConfigDev(previous, -1, -1)`. A missing library returns 0.

The accessors walk the list to `index`; an out-of-range or negative index
returns 0. The fields are:

| Accessor | `ConfigDev` field |
|---|---|
| `expansionBoardManufacturer` | `cd_Rom.er_Manufacturer` |
| `expansionBoardProduct` | `cd_Rom.er_Product` |
| `expansionBoardSerial` | `cd_Rom.er_SerialNumber` |
| `expansionBoardKind` | `cd_Rom.er_Type` |

The probe found two Commodore boards on both baselines (manufacturer 2011,
products 82 and 4, serial 1). The values are hardware-dependent; the A600 gate
pins the count and the first board's manufacturer/product in the `svc` line.

## Card Resource

`cl_card_present` opens `card.resource` with `OpenResource`, assigns the NDK
`CardResource` global, records whether it was found, and calls `RemResource`.
`cl_card_status` opens the resource, assigns `CardResource`, calls
`ReadCardStatus`, calls `RemResource`, and returns the status; a missing
resource returns 0. The resource is optional, so absence is not an error.
On both probes the resource was present and the status was 0 (no card inserted);
the `svc` line records both the presence and the status.

## Keymap

`cl_map_raw_key(code, qualifier)`:

1. Open `keymap.library` 37 lazily and assign the NDK `KeymapBase` global; a
   failure returns `NULL`.
2. Fill a stack `struct InputEvent` with `ie_Code = code`,
   `ie_Qualifier = qualifier`, and a null `ie_NextEvent`.
3. `keymap = AskKeyMapDefault()`.
4. `length = MapRawKey(&event, buffer, sizeof buffer, keymap)`.
5. A non-positive length returns `NULL`; otherwise allocate a `String` with
   `cl_string_new`/`cl_string_append` and return it.

The `InputEvent` is zeroed and its class set to `IECLASS_RAWKEY`; the ROM
default keymap maps the demo's raw key, so the demo reports `key 1` on both
baselines. A Workbench installation with additional keymaps can map more.

## Shim Guards

The expansion/icon/Workbench shim code is compiled only for `hosted-amiga`:
the emitter appends `compiler/runtime/shim_hosted_amiga_20.c` **after** the
generated program. The 1.3 build never sees the suffix, so the 1.3 NDK does not
need the 2.0-only headers or symbols, and the 2.0 headers' macros cannot rewrite
user identifiers. The shared shim still compiles for both profiles.

## Failure Values

| Call | Failure value |
|---|---|
| `expansionBoardCount` | 0 when the library is missing |
| `expansionBoardManufacturer`, `...Product`, `...Serial`, `...Kind` | 0 for an out-of-range index |
| `cardPresent` | `false` when the resource is missing |
| `cardStatus` | 0 when the resource is missing |
| `mapRawKey` | `null` when the library is missing or the key is unmapped |

## Test Hooks

The specification test compiles the surface on `hosted-amiga` and cross-builds
it with `nix20`. The implementation test pins the `FindConfigDev` loop, the
field reads, the card-resource open/close, the `MapRawKey` call, and the
bounds checks. The A600 and A1200 gates print the board count, the first board's
manufacturer/product, the card presence and status, and the keymap result:
`svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1`.
