# Requirements: Freestanding profile

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

This plan implements no upstream RD, so this document is the owning
requirements doc (standalone full form).

## Feature Overview

The `freestanding` profile is the third platform profile. It is the profile a
program selects when no operating system is present: the emitted C is linked
without libc and enters through a runtime that this project supplies. The
specification reserves the profile for LorraineOS (`spec/15-profiles.md:47`),
but LorraineOS does not exist, so this plan defines a **provisional ABI** the
project controls and LorraineOS can adopt later.

Implementing the profile now means three things: the compiler selects and emits
for it, a no-libc runtime starts and runs the program, and the project can prove
both on the toolchain that exists. Because there is no target OS, the proof is
split: the m68k artifact must build and link with no unresolved symbols, and the
same translation unit must run on the development machine through a raw-syscall
entry.

The profile deliberately supports a small standard-library surface. Console
output works; the heap and formatting modules and the AmigaOS modules are
rejected at compile time; the file functions of `std.io` are present but trap at
runtime. Float types and operations are supported.

## Functional Requirements

### Must Have

- [x] R1 — The compiler accepts `--profile freestanding` and emits a
  translation unit for it; unknown profiles remain `E11201` (AR-19, AR-30).
- [x] R2 — `Profile` is the single source of truth for the profile name and its
  facts; the driver no longer keeps a second name list (AR-19).
- [x] R3 — A freestanding translation unit embeds the freestanding runtime shim
  and no host or hosted-amiga runtime surface (AR-5, AR-13).
- [x] R4 — The runtime defines `_start`, which calls the generated `main` and
  then exits (AR-13).
- [x] R5 — The runtime provides `memset`, `memcpy`, `memcmp`, and `strlen`, so a
  `-nostdlib` link resolves every symbol (AR-9).
- [x] R6 — The m68k build compiles and links with
  `-ffreestanding -nostdlib -lg`; the link succeeds with every symbol resolved
  (the linker fails on any unresolved symbol) (AR-7, AR-10, AR-27).
- [x] R7 — The host run compiles the same translation unit with the
  arch-guarded raw-syscall layer and executes it; on architectures without a
  syscall layer, the run links the platform C runtime instead (AR-7, AR-8).
- [x] R8 — `std.io` console output (`print`, `println`) works on freestanding;
  `std.mem`, `std.text`, `std.fmt`, and `std.amiga.*` are rejected with
  `E11200` (AR-11).
- [x] R9 — `std.io`'s `Open`, `Close`, `Read`, and `Write` compile but trap at
  runtime with the new `E11214` (AR-12, AR-28). Reaching them needs the
  `static str` literal to `cstring` widening (AR-31), because `std.text`, the
  only `toCString` producer, is rejected on freestanding.
- [x] R10 — Float types and operations are allowed; the m68k link appends `-lg`
  and the host run uses native floats (AR-10).
- [x] R11 — The runtime fault and panic helpers write their whole message to
  fd 2 in one write and then `_exit(1)` on the host-run leg, and halt on m68k
  (AR-14, AR-29).
- [x] R12 — The driver builds the m68k artifact for `build --profile
  freestanding`, and `run --profile freestanding` compiles and runs the host
  no-libc artifact (AR-20).
- [x] R13 — The freestanding `W10004` stack-frame threshold is 1024 bytes
  (AR-16).
- [x] R14 — The specification reflects an implemented freestanding profile: PR-6
  is reworded, the profile tables and codegen matrix are updated, `E11214` is
  registered, and the obsolete rejection golden is replaced (AR-21, AR-30).

### Should Have

- [x] R15 — The host-run leg's arch guard covers x86_64 and aarch64, with a
  libc-linked fallback elsewhere (AR-8).
- [x] R16 — The freestanding build adds `-fno-builtin` and is verified at `-O2`
  as well as `-O0`, so a release build cannot recurse into the runtime's own
  `memcpy`/`memset` (AR-9).
- [x] R17 — Root `make run FILE=… PROFILE=freestanding` works through the
  existing driver passthrough (AR-20, AR-22).

### Won't Have (Out of Scope)

An ARM64 profile or 64-bit `usize`/`isize` (AR-25); a freestanding heap
allocator and the text/format modules (AR-26); a profile-scoped FFI contract
table; a custom soft-float library; a general per-profile facts table; running
the m68k artifact under an emulator; any change to `clc`'s emit-only role or the
`hosted-amiga`/`host` profiles.

## Technical Requirements

### Performance

The profile adds no compiler work beyond selecting a different shim and tool
flags. The freestanding runtime is small and allocation-free, so every helper is
`static inline` or a short loop; the emitted C keeps the existing determinism
guarantee (same source, same profile, same flags produce byte-identical C).

### Compatibility

The emitted C stays the strict C99 subset of `spec/17-codegen.md` §1 and must
compile under `-std=c99 -Wall -Wextra -Werror`. The m68k leg's exact command is
normative in `spec/17-codegen.md` §13. The freestanding ABI is provisional and
adoptable by LorraineOS; it is m68k, 32-bit, big-endian, soft-float.

### Security

The profile parses no new input. The console hook is a direct write of an
already-validated byte range; the runtime bounds-checks nothing new because the
emitter already emits bounds, division, and conversion guards. The host syscall
layer uses only `write` and `exit`, takes no path or shell, and adds no network,
credentials, or filesystem access. The emitter continues to invoke external
tools with an argument array only.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| ABI ownership | wait for LorraineOS / define provisional | define provisional now | The OS does not exist; the profile would otherwise stay blocked | AR-3 |
| Verification | build-only / host run / vamos | build+link plus host no-libc run | Proves both self-containment and semantics without an OS | AR-7, AR-27 |
| stdlib surface | console only / heap+format | console only | Keeps the runtime small and honest; heap is a later plan | AR-11 |
| File I/O | contract gate / new module / runtime trap | runtime trap | Avoids a new cross-cutting compiler mechanism | AR-12, AR-28 |
| Floats | reject / link SDK archive | link `-lg` | One self-contained archive; keeps a v1 feature | AR-10 |
| Entry | keep `main` / emit `_start` | keep `main`, add `_start` in the shim | Smallest codegen change; matches the spec's `main` stub | AR-13 |
| Profile facts | duplicate match / single source | single source on `Profile` | Removes the duplication that made freestanding a special case | AR-19 |
| Driver run | m68k build + host run / build only | m68k build, host no-libc run | Gives a one-command execution path without an OS | AR-20 |
| `usize` width | profile-driven / keep 32-bit | keep 32-bit | ARM64 width is a separate port concern | AR-17, AR-25 |

> **Traceability:** every row cites the Ambiguity Register entry that resolved
> it; see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] `clc --profile freestanding -o out.c hello.clo` exits 0 and the C
   contains the freestanding runtime and `_start`.
2. [x] The m68k artifact for a freestanding program links cleanly with
   `-nostdlib -lg` (every symbol resolved).
3. [x] A freestanding `println("hello")` program prints `hello` and exits 0
   through the host no-libc run.
4. [x] A program importing `std.mem`, `std.text`, `std.fmt`, or `std.amiga.*`
   is rejected with `E11200` under freestanding.
5. [x] A freestanding program that calls `openFile` traps with `E11214` and a
   non-zero exit.
6. [x] A freestanding float program links on m68k and returns the expected
   value through the host no-libc run.
7. [x] The compiler's `hosted-amiga` and `host` behaviour and the frozen
   specification's remaining rules are unchanged.
8. [x] `make check` and `make verify` stay green, including the new freestanding
   tests.
9. [x] `AGENTS.md` documents the profile; `plans/00-roadmap.md` carries the
   updated `PLAN-03` row.
