---
title: About
description: "Why Clorraine exists, its relationship to LorraineOS, and how the project is developed."
---

# About

Clorraine is the systems language of **LorraineOS**, a bare-metal Amiga OS for
the Motorola 68000 family. The language and the OS are co-designed: LorraineOS
requirements drive Clorraine's feature set, and Clorraine exists to make
LorraineOS and Amiga programs safe and pleasant to write.

Like C and Unix, and later Oberon, the pairing is deliberate. A systems language
should be shaped by the system it builds.

## Why another language?

C is everywhere on the Amiga, and it is not going away. Clorraine does not try to
replace every C file. It targets the problems that make large C programs hard to
change safely:

| C problem | Clorraine response |
|---|---|
| Use-after-free and double-free | One owner, moves, and deterministic drops checked by the compiler |
| Unclear pointer modes | `T`, `T?`, `borrow T`, `take T`, and views are explicit |
| Header and build complexity | One C99 translation unit, a module system, and an embedded standard library |
| Target facts scattered through makefiles | Profiles carry the compiler, runtime, ABI, and package facts |
| Unsafe hardware access hidden in macros | Named wrappers and an explicit `volatile`/`raw` boundary |

The compiler still emits C99 and uses the AmigaPorts m68k GCC toolchain. That is
a feature: the output is inspectable, the toolchain is familiar, and the
generated program does not depend on a new runtime VM.

## Design principles

- **Safety without a collector.** Ownership is checked at compile time. There is
  no tracing GC and no hidden ownership runtime.
- **Amiga first.** The standard library covers graphics, audio, Intuition, Exec,
  DOS, layers, text, ILBM, and RTG. The A500/Kickstart 1.3 profile is a
  first-class target.
- **One language, four targets.** `host`, `hosted-amiga`, `hosted-amiga-13`, and
  `freestanding` share the language and differ in target facts.
- **The specification is the oracle.** The language, grammar, and diagnostics are
  frozen and tested. The implementation follows them.
- **Explicit boundaries.** `ffi`, `raw`, and `unsafe` mark the places where the
  language hands control to C or hardware.
- **Honest status.** The project is a developer preview. The site says so on every
  page.

## Relationship to LorraineOS

LorraineOS is a separate downstream project. Clorraine's `freestanding` profile
defines a provisional 32-bit, big-endian, no-libc ABI that LorraineOS can adopt
or replace. The profile is implemented and tested, but it is not yet a stable
OS ABI.

The hosted profiles target AmigaOS 2.0+ and the A500/Kickstart 1.3 baseline. They
are the practical path for Amiga programs today.

## How the project is developed

Clorraine is developed with AI-assisted tooling under human direction. The
requirements, design decisions, and acceptance decisions remain human-controlled.
Every change is checked against the frozen specification and the project's test
gates. The AI is a tool, not the authority.

The project does not claim that AI assistance makes the compiler correct. The
evidence is the specification, the tests, the sanitizers, the fuzzer, and the
Amiga acceptance runs.

## Independence

Clorraine is an independent implementation. It does not include or derive from
proprietary AmigaOS source code, Kickstart ROMs, or Workbench media. It is not
affiliated with or endorsed by Amiga Corporation or Hyperion Entertainment.

Amiga, AmigaOS, Kickstart, and Workbench are trademarks of Amiga Corporation.

## License

The compiler and site content are Apache 2.0, with the
[Clorraine Output Exception](/project/licensing). Published v1.1.2 artifacts
remain MIT.

## Links

- [OpenLorraine on GitHub](https://github.com/OpenLorraine/clorraine)
- [Project brief](/project/brief)
- [Verification](/project/verification)
- [Roadmap](/guide/roadmap)
