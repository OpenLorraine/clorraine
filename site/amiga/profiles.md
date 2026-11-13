---
title: Amiga profiles
description: "hosted-amiga, hosted-amiga-13, and freestanding: what each profile provides on 68K Amiga."
---

# Amiga profiles

The target profile decides the compiler, runtime, CPU default, and module
surface. The source language does not name a chip or an address.

## `hosted-amiga`

| Fact | Value |
|---|---|
| Compiler | `m68k-amigaos-gcc` |
| Runtime | libnix `-mcrt=nix20` |
| Target | AmigaOS 2.0+, big-endian, 32-bit |
| Default CPU | 68000 (`-m68020` opt-in) |
| Float ABI | Soft-float (`-m68881` opt-in) |
| Package | Hunk executable |

This is the profile for Workbench 2.x/3.x and the optional services. It includes
`std.amiga.workbench`, `std.amiga.icon`, `std.amiga.expansion`, and the 2.0+
optional modules.

## `hosted-amiga-13`

| Fact | Value |
|---|---|
| Compiler | `m68k-amigaos-gcc` |
| Runtime | libnix `-mcrt=nix13` |
| Target | A500: 68000, OCS, 512 KB chip, Kickstart 1.3 |
| Default CPU | 68000 |
| Float ABI | Soft-float |
| Package | Hunk executable |

The 1.3 profile provides a Kickstart 1.3-safe subset:

`std.io`, `std.mem`, `std.text`, `std.fmt`, `std.amiga.custom`,
`std.amiga.dos`, `std.amiga.graphics`, `std.amiga.intuition`,
`std.amiga.interrupt`, and `std.amiga.exec`.

The 2.0+ modules are rejected with `E11200`. That is deliberate: a program that
uses Workbench 2.0 APIs should not silently produce an A500 binary that fails at
runtime.

## `freestanding`

`freestanding` is implemented with a provisional ABI: 32-bit, big-endian, no
libc, console-only standard library. It has no heap, so `String` and `Box<T>` are
rejected, and the `std.mem` allocator surface is unavailable.

LorraineOS can adopt or replace the ABI. It is not yet a stability promise.

## Building

```sh
clorraine build program.clo -p hosted-amiga
clorraine build program.clo -p hosted-amiga-13
clorraine build program.clo -p freestanding
```

Inside the repository, use the SDK and emulator wrappers:

```sh
make run FILE=program.clo PROFILE=hosted-amiga
make run FILE=program.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

## Module boundary

| Module | 1.3 | 2.0+ |
|---|---|---|
| `std.io`, `std.mem`, `std.text`, `std.fmt` | Yes | Yes |
| `std.amiga.custom`, `dos`, `graphics`, `intuition`, `interrupt`, `exec` | Yes | Yes |
| `std.amiga.video`, `sprite`, `audio`, `input`, `timer`, `text`, `layers` | Yes | Yes |
| `std.amiga.workbench`, `icon`, `expansion` | No (`E11200`) | Yes |
| `std.amiga.asl`, `commodities`, `locale`, `arexx`, `clipboard`, `datatypes`, `bsdsocket`, `picasso96`, and other optional services | Selectively | Yes, where the tracked gate proves the surface |

The SDK coverage page records the exact tracked surface.

## Learn more

- [Profiles in the language tour](/language/profiles)
- [SDK coverage](/amiga/sdk-coverage)
- [Chapter 15 — Platform Profiles](https://github.com/OpenLorraine/clorraine/blob/main/spec/15-profiles.md)
