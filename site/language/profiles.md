---
title: Profiles
description: "The four Clorraine target profiles and what each one provides."
---

# Profiles

A profile is a machine-readable bundle of target facts: compiler, runtime,
pointer width, endianness, entry point, package format, and stack policy. The
core language never names a chip or an address. You select a profile; the profile
supplies the rest.

## The four profiles

| Profile | Compiler and runtime | Target | Entry | Package |
|---|---|---|---|---|
| `host` | Host `gcc` or `clang` with the host shim | Native, little-endian | `main(): i32` | Host executable |
| `hosted-amiga` | `m68k-amigaos-gcc`, libnix `-mcrt=nix20` | AmigaOS 2.0+, big-endian, 32-bit | `main(): i32` | Hunk executable |
| `hosted-amiga-13` | `m68k-amigaos-gcc`, libnix `-mcrt=nix13` | A500: 68000, OCS, 512 KB chip, Kickstart 1.3 | `main(): i32` | Hunk executable |
| `freestanding` | `m68k-amigaos-gcc -nostdlib` with the freestanding shim | m68k, big-endian, 32-bit, no libc | `_start` calls `main(): i32` | Raw m68k hunk |

The default CPU is `-m68000`. The `-m68020` and `-m68881` legs are opt-in. The
soft-float ABI is the compatibility default because it works on every 68000.

## Choosing a profile

| You want to | Use |
|---|---|
| Run a fast test on the development machine | `host` |
| Build for AmigaOS 2.0 or later, including Workbench 2.x/3.x | `hosted-amiga` |
| Build for a stock A500 with Kickstart 1.3 | `hosted-amiga-13` |
| Build a no-libc image for LorraineOS bring-up | `freestanding` |

The `host` profile is for tests. It is not an Amiga artifact and does not prove
that a program runs on AmigaOS.

## Commands

```sh
clorraine run hello.clo
clorraine run hello.clo -p hosted-amiga
clorraine build hello.clo -p hosted-amiga-13
clorraine emit hello.clo -p host -o build/hello.c
clorraine build hello.clo -p freestanding
```

Inside the repository, the Makefile wrappers set up the SDK and the emulator:

```sh
make run FILE=examples/hello/hello.clo
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

## The 1.3 subset

The `hosted-amiga-13` profile provides a Kickstart 1.3-safe standard-library
subset:

`std.io`, `std.mem`, `std.text`, `std.fmt`, `std.amiga.custom`,
`std.amiga.dos`, `std.amiga.graphics`, `std.amiga.intuition`,
`std.amiga.interrupt`, and `std.amiga.exec`.

The 2.0+ modules (`std.amiga.workbench`, `std.amiga.icon`, and
`std.amiga.expansion`) are available only on `hosted-amiga`; the 1.3 profile
rejects them with `E11200`.

## Freestanding

`freestanding` is implemented with a provisional ABI: 32-bit, big-endian, no
libc, console-only standard library. It has no heap, so the built-in `String`
and `Box<T>` are rejected with `E11200`, and the `std.mem` allocator surface is
not available.

The ABI is a thin contract that LorraineOS can adopt or replace. It is not a
promise of ABI stability across releases yet.

## Unsupported constructs

A construct the selected profile cannot lower is rejected, not silently
emulated. The diagnostic is `E11200`. An unknown profile name is `E11201`.

## Learn more

- [Chapter 15 — Platform Profiles](https://github.com/OpenLorraine/clorraine/blob/main/spec/15-profiles.md)
- [Chapter 17 — Code Generation](https://github.com/OpenLorraine/clorraine/blob/main/spec/17-codegen.md)
- [Amiga profiles](/amiga/profiles)
