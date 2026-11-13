---
title: Standard library
description: "The 43 embedded Clorraine standard-library modules, grouped by purpose and profile."
---

# Standard library

The standard library is embedded in the compiler. It ships as `.clo` sources
under `stdlib/`, so the generated C includes only the modules the program uses.

There are **43 modules**: six core modules and 37 Amiga modules.

## Core modules

| Module | Purpose |
|---|---|
| `std.io` | Console output, files, and byte streams |
| `std.mem` | Buffers, byte views, `Array<T>`, `Box<T>`, and allocation |
| `std.text` | `String`, string views, conversion, and NUL-terminated views |
| `std.fmt` | Hex and numeric formatting |
| `std.math` | Integer and floating-point helpers, trigonometry |
| `std.iff` | IFF parsing and writing |

## Amiga modules

### Core OS surface

| Module | Purpose |
|---|---|
| `std.amiga.custom` | Custom chip registers and per-chip wrappers |
| `std.amiga.dos` | Files, directories, paths, delays, and volume information |
| `std.amiga.exec` | Tasks, ports, messages, libraries, and worker tasks |
| `std.amiga.graphics` | Screens, drawing, colours, and rastports |
| `std.amiga.input` | Mouse, keyboard, and joystick input |
| `std.amiga.interrupt` | Interrupt handlers and servers |
| `std.amiga.intuition` | Screens, windows, gadgets, menus, and requesters |
| `std.amiga.layers` | Layers and clipping |
| `std.amiga.sprite` | Hardware sprites and collision |
| `std.amiga.text` | Topaz fonts and text rendering |
| `std.amiga.timer` | Timer devices and delays |
| `std.amiga.video` | Chip memory, bitmaps, blitter, copper, and displays |

### Advanced and optional services

| Module | Purpose |
|---|---|
| `std.amiga.workbench` | AppIcons, AppMessages, and Workbench startup |
| `std.amiga.icon` | Icon and disk-object handling |
| `std.amiga.expansion` | Expansion board information |
| `std.amiga.args` | Argument parsing |
| `std.amiga.asl` | ASL file and font requesters |
| `std.amiga.audio` | Audio channels and sample playback |
| `std.amiga.boopsi` | BOOPSI classes and objects |
| `std.amiga.bsdsocket` | TCP/IP sockets through a third-party stack |
| `std.amiga.clipboard` | Clipboard device access |
| `std.amiga.commodities` | Commodities broker and hotkeys |
| `std.amiga.datatypes` | Datatypes library |
| `std.amiga.devices` | Device opening and I/O |
| `std.amiga.eclock` | Elapsed-time clock |
| `std.amiga.gadtools` | GadTools gadgets |
| `std.amiga.amigaguide` | AmigaGuide documents |
| `std.amiga.arexx` | ARexx message ports |
| `std.amiga.ilbm` | ILBM image loading |
| `std.amiga.locale` | Locale and language information |
| `std.amiga.lowlevel` | Low-level library access |
| `std.amiga.nonvolatile` | Nonvolatile storage |
| `std.amiga.picasso96` | Picasso96 RTG surfaces |
| `std.amiga.pool` | Memory pools |
| `std.amiga.regions` | Regions and clipping |
| `std.amiga.semaphore` | Semaphores and mutual exclusion |
| `std.amiga.utility` | Utility library helpers |

## Profile availability

| Module group | `host` | `hosted-amiga-13` | `hosted-amiga` | `freestanding` |
|---|---|---|---|---|
| `std.io`, `std.mem`, `std.text`, `std.fmt`, `std.math`, `std.iff` | Yes | Yes, except heap-dependent APIs | Yes | Console subset; no heap |
| Core Amiga modules | No | Yes | Yes | No |
| `workbench`, `icon`, `expansion` | No | No (`E11200`) | Yes | No |
| Optional services | No | Selectively | Yes, where the tracked gate proves the surface | No |

The exact API surface is in
[`spec/appendix-stdlib.md`](https://github.com/OpenLorraine/clorraine/blob/main/spec/appendix-stdlib.md).
The source of each module is under
[`stdlib/`](https://github.com/OpenLorraine/clorraine/tree/main/stdlib).

## Importing

```clorraine
import { println } from std.io;
import { String, stringOf } from std.text;
import { setColor } from std.amiga.custom;
```

Imports are explicit. The compiler resolves the module from the embedded
library, and the generated C includes only what the program reaches.

## Generating API documentation

`clorraine --doc` renders `///` comments from Clorraine sources as linked
Markdown. It works on your sources as well as the standard library:

```sh
clorraine --doc -o build/api examples/hello/hello.clo
```

The output directory contains one Markdown page per module plus an index.
