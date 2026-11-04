# Chapter 15 - Platform Profiles

> **Status:** stable (v1.0; v1.1 adds the A500/Kickstart 1.3 profile)
> **Depends on:** Ch 00 (axioms), Ch 12 (contracts), Ch 14 (diagnostics), Ch 17 (codegen) *(informational)*
> **Grammar:** none; profiles carry target facts, not surface syntax (see [grammar.ebnf.md](grammar.ebnf.md))
> **Ledger:** [decisions.md](decisions.md): A5, X3, P14, P15, R2 (C3 is an appendix topic); diagnostics N1-N3, W10004

A machine that runs Clorraine has facts the language must not know about: which
compiler to call, which runtime to link, how wide a pointer is, and how a
program is packaged. A **profile** is a machine-readable bundle of those facts.
The core language never mentions a chip, an address, or a library (A5). A
source file names a profile, and the profile supplies the rest.

## 1. What a profile contains

A profile is data, not syntax. The compiler and the SDK read it; a source file
that uses only supported features compiles unchanged under any profile that
provides them.

| Fact | Example |
|---|---|
| Toolchain | compiler command, CPU flags, ABI flags |
| Runtime | C runtime and startup, or the OS entry points |
| Target | default CPU, endianness, pointer size |
| Entry | how the program starts and returns |
| Package | the artifact the target runs |
| Float ABI | how `f32`/`f64` arguments and results cross a call |
| Stack policy | the stack-frame warning threshold used by W10004 |

The rules below make the profile the single place these facts live.

| Rule | Statement | Diagnostic |
|---|---|---|
| PR-1 | All target facts live in the profile; source files do not name them | - |
| PR-2 | Contracts are profile-scoped: each profile names its API surface (X3; [Ch 12](12-ffi-contracts.md)) | - |
| PR-3 | A construct the selected profile cannot lower is rejected | E11200 |
| PR-4 | Profiles are versioned with the compiler; the SDK records the active profile in its provenance (R2) | - |
| PR-5 | An unknown profile name is rejected | E11201 |
| PR-6 | A profile that is declared but not implemented is rejected | E11200 |

The compiler never guesses a profile from the host machine. The build selects
one profile, and that selection is recorded.

## 2. The v1 profiles

v1 ships `hosted-amiga`. v1.1 adds `hosted-amiga-13`, the A500/Kickstart 1.3
baseline. The `host` profile exists for fast tests. `freestanding` is implemented
provisionally: because LorraineOS does not exist yet, this project defines a thin
ABI (m68k, 32-bit, big-endian, no libc) that LorraineOS can adopt or replace
later.

| Profile fact | `hosted-amiga` (v1) | `host` (tests) | `freestanding` (provisional) |
|---|---|---|---|
| Compiler | `m68k-amigaos-gcc`, `vc +aos68k` | host `gcc`/`clang` | `m68k-amigaos-gcc -nostdlib` |
| Runtime | libnix `-mcrt=nix20` | host shim | self-contained freestanding shim |
| Default CPU | `-m68000` (opt-in `-m68020`) | native | `-m68000` (m68k) |
| Endianness | big | little | big |
| Pointer size | 32-bit | host | 32-bit |
| Entry | `main(): i32` exported to C | `main(): i32` | `_start` calls `main(): i32` |
| Package | hunk executable (dir drive/ADF) | host binary | raw m68k hunk |

Two codegen facts need a separate row because they have a policy, not just a
value:

| Profile fact | `hosted-amiga` | `host` | `freestanding` |
|---|---|---|---|
| Float ABI | soft-float; the GCC and vbcc ABIs are a planned matrix leg, not yet verified (P15) | native IEEE `float`/`double` | soft-float through `libg.a` (`-lg`) |
| FPU build leg | opt-in `-m68881` | not applicable | not applicable |
| Stack-frame warning (W10004) | 1024 bytes (v1 default, W10004a) | 16384 bytes (v1 default, W10004a) | 1024 bytes |

The soft-float ABI is the compatibility choice: it works on every 68000 and
needs no FPU. The `-m68881` leg trades that portability for speed and is
opt-in. The stack-frame thresholds are v1 defaults; a frame above the
threshold is a warning, not an error, so a large frame is a deliberate choice.

### 2.1 A500 baseline (`hosted-amiga-13`, v1.1)

The lowest supported baseline is a stock A500: 68000, OCS, 512 KB chip, and
Kickstart 1.3.

| Profile fact | `hosted-amiga-13` |
|---|---|
| Compiler | `m68k-amigaos-gcc` |
| Runtime | libnix `-mcrt=nix13` |
| Default CPU | `-m68000` |
| Endianness | big |
| Pointer size | 32-bit |
| Entry | `main(): i32` exported to C |
| Package | hunk executable (dir drive/ADF) |
| Float ABI | soft-float |
| Stack-frame warning | 1024 bytes |

The profile provides the Kickstart 1.3-safe standard-library subset:
`std.io`, `std.mem`, `std.text`, `std.fmt`, `std.amiga.custom`,
`std.amiga.dos`, `std.amiga.graphics`, `std.amiga.intuition`, and
`std.amiga.interrupt`. `std.amiga.exec` is a 2.0+ surface and reports
`E11200` on this profile until the Exec bundle adds the fallback.

## 3. Build matrix

P14 fixes the build matrix. The exact commands are normative in
[Ch 17](17-codegen.md) section 13; the table below is the map.

| Element | Value |
|---|---|
| Libraries | `-lamiga`, `-lm` |
| C runtime | `-mcrt=nix20` (hosted-amiga); `-mcrt=nix13` (hosted-amiga-13) |
| CPU | `-m68000` default; opt-in `-m68020` |
| FPU | opt-in `-m68881` (P15) |
| Runners | `vamos` with GCC and vbcc; FS-UAE directory drive and ADF |

Phase 0 verified the hosted matrix
([findings](../docs/findings-phase0.md)): GCC 68000 on `vamos` and both FS-UAE
media, GCC 68020 build-only, and vbcc on `vamos` only. No float code was
exercised, so the float ABI legs remain planned and unverified (P15).

## 4. Cross-profile testing

| Profile | When it runs | What it proves |
|---|---|---|
| `host` | every commit | the emitted C compiles and the semantics are correct |
| `hosted-amiga` | emulator CI (`vamos`, FS-UAE) | the real target and runtime behave the same |
| `hosted-amiga-13` | A500 emulator (`vamos`, FS-UAE with Kickstart 1.3) | the pre-2.0 runtime and the A500 baseline behave the same |
| `freestanding` | the build and host-run legs of the check suite | the artifact links with no C runtime; the same unit runs through the host no-libc layer |

The `host` profile is the fast gate. The emulator legs catch target-only
problems: endianness, pointer width, the AmigaOS runtime, and packaging.

## 5. Diagnostics

| Code | Trigger |
|---|---|
| E11200 | a construct is not lowerable on the selected profile, or the profile is not implemented |
| E11201 | the profile name is unknown |
| W10004 | a stack frame exceeds the profile threshold |

Message text lives in [Ch 14](14-diagnostics.md). W10004 is a warning: it is
deniable by default and escalable with `--deny-warnings` (N6).

## 6. Open questions

| Question | Current position |
|---|---|
| Profile file format | Compiled-in tables first; data files later |
| `freestanding` ABI | Implemented provisionally (m68k, 32-bit, big-endian, no libc); LorraineOS may adopt or replace it |
| ARM64 profile | Not planned; the v1 target is m68k/AmigaOS-compatible |
| Stack-frame thresholds | v1 defaults; revisit after the first target measurements |
