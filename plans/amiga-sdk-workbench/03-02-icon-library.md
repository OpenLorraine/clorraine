# Design: Icon Library

> **Document**: 03-02-icon-library.md
> **Parent**: [Index](00-index.md)
> **Covers**: R2, R8 (icon part), AR-6

## Public API

```clorraine
module std.amiga.icon;

export handle DiskObject freedBy cl_free_disk_object;

export const ICON_DISK: i32 = 1;
export const ICON_DRAWER: i32 = 2;
export const ICON_TOOL: i32 = 3;
export const ICON_PROJECT: i32 = 4;
export const ICON_GARBAGE: i32 = 5;
export const ICON_DEVICE: i32 = 6;
export const ICON_KICK: i32 = 7;
export const ICON_APPICON: i32 = 8;

ffi stdicon {
    func cl_get_disk_object(name: cstring): DiskObject?;
    func cl_put_disk_object(name: cstring, borrow icon: DiskObject): bool;
    func cl_free_disk_object(take icon: DiskObject): void;
    func cl_get_def_disk_object(kind: i32): DiskObject?;
    func cl_disk_object_type(borrow icon: DiskObject): i32;
    func cl_set_disk_object_type(borrow icon: DiskObject, kind: i32): bool;
}
```

The module is available only on `hosted-amiga`.

## Semantics

| Call | Behavior |
|---|---|
| `getDiskObject(name)` | `GetDiskObject(name)`; reads `<name>.info`; `null` on a null name, missing file, or allocation failure |
| `putDiskObject(name, icon)` | `PutDiskObject(name, icon)`; writes `<name>.info`; `false` on a null name/icon or a write failure |
| `freeDiskObject(take icon)` | `FreeDiskObject` exactly once; null-safe |
| `getDefDiskObject(kind)` | `GetDefDiskObject(kind)` for `kind` 1..8; `null` for another kind or an allocation failure |
| `diskObjectType(borrow icon)` | `do_Type`, or 0 for a null handle |
| `setDiskObjectType(borrow icon, kind)` | sets `do_Type` for `kind` 1..8; `false` for null or an out-of-range kind |
| drop (generated) | frees an unfreed `DiskObject`; null-safe |

`get`/`put` use the base name without `.info`, matching the icon-library API.
The caller owns the `DiskObject`; the library does not retain it after
`putDiskObject` returns. `PutDiskObject` writes the `.info` file directly, so a
failed write is reported but the original file is not guaranteed untouched.

## Shim Details

- Open `icon.library` 37 lazily and assign the NDK `IconBase` global; a
  failure returns `null`/`false`.
- `cl_get_disk_object(char *name)` rejects a null name.
- `cl_put_disk_object(char *name, void *icon)` rejects a null name or icon and
  forwards to `PutDiskObject`.
- `cl_free_disk_object(void *icon)` is null-safe.
- `cl_get_def_disk_object(LONG kind)` rejects a kind outside 1..8 before calling
  `GetDefDiskObject`.
- `cl_disk_object_type` reads `((struct DiskObject *)icon)->do_Type`.
- `cl_set_disk_object_type` writes the field after the range check.

The icon functions live in the 2.0+ shim section; the 1.3 build never compiles
or links them.

## Failure Values

| Call | Failure value |
|---|---|
| `getDiskObject`, `getDefDiskObject` | `null` |
| `putDiskObject`, `setDiskObjectType` | `false` |
| `diskObjectType` | 0 for a null handle |

## Test Hooks

The specification test compiles the surface on `hosted-amiga` and cross-builds
it with `nix20`. The implementation test pins the shim calls, the `do_Type`
range check, and the drop routing. The A600 and A1200 gates perform a real
default-icon -> `put` -> `get` -> type -> free round-trip on a writable volume.
