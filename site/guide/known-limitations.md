---
title: Known limitations
description: "The accepted limitations of Clorraine v1.2, summarised for users and linked to the authoritative list."
---

# Known limitations

Clorraine is a developer preview. This page summarises the limitations that
matter when you evaluate or use it. The authoritative list, including compiler
edge cases and their workarounds, is
[docs/known-limitations.md](https://github.com/OpenLorraine/clorraine/blob/main/docs/known-limitations.md).

## Project-wide

| Limitation | Impact | Guidance |
|---|---|---|
| Developer preview | Interfaces, the installer, and platform support may change | Use it for experiments and feedback, not production work |
| Linux x86_64 only | v1.2.0 installs on Linux x86_64 only; WSL2 and Remote-WSL are not claimed | Windows 11 ships with the next release ([quick start](/guide/quick-start#install-on-windows)); macOS is on the way |
| Host floor | glibc 2.38+; Ubuntu 24.04 is the minimum baseline | Ubuntu 22.04 cannot run the rc24 host tools |
| Toolchain and emulator | rc24 only; FS-UAE 3.2.0+; a licensed Kickstart 3.1 A1200 ROM for Amiga runs | KS3.0 is untested; AROS is manual-only and not an acceptance path |
| Emulator runs end after 90 seconds | `clorraine run --emulator` stops a program that has not finished, Amiga boot included | Programs that run forever, such as the showcase demos, play [in the browser](/examples/) |
| No Workbench in the emulator run | The FS-UAE run boots straight into your program, without a Workbench screen | Programs that open windows on Workbench need a Workbench setup of your own |
| VS Code extension is not in a marketplace | The extension comes from the release | The installers install it when VS Code is detected |
| Windows: host floor | Windows 11 x64; Windows 10 and PowerShell 7 are untested; ARM64 is refused | Use Windows 11 x64 with Windows PowerShell 5.1 |
| Windows: execution policy | Scripts do not run by default; Group Policy takes precedence over the bypass | Run the installer with the one-time `-ExecutionPolicy Bypass` |
| Windows: administrator step | Installing rc24 asks for UAC approval once | Approve it, or pass `-Toolchain` with an existing rc24 |
| Windows: Clang debug builds | Clang rejects the emitted C under `-Werror` | Use GCC (w64devkit or MSYS2 UCRT64) or set `CLORRAINE_CC` |
| Windows: SSH logons | The rc24 compiler does not start from an SSH logon | Run the CLI in the signed-in desktop session |
| Windows: interrupted emulator runs | Ctrl+C leaves the FS-UAE window and `%TMP%\clorraine-emulator-*` | Close the window and delete the folder |
| Windows: path shapes | UNC workspaces, paths over 260 characters, and a comma in the `%TMP%` path are unsupported | Use a local folder with a short path |
| Windows: unsigned binaries | SmartScreen or Defender may ask before the first run | Compare the downloads with the release `SHA256SUMS` first |
| Host-only CI | The GitHub Actions runner does not execute the Amiga legs | Run `make verify` locally for full acceptance |
| Amiga gates need licensed media | Advanced Workbench gates cannot run on a bare machine | Supply your own media; no ROM or Workbench image is distributed |

## Platform profiles

| Profile | Limitation |
|---|---|
| `hosted-amiga` | Targets AmigaOS 2.0+ through libnix `-mcrt=nix20` |
| `hosted-amiga-13` | A500 baseline: 68000, OCS, 512 KB chip, Kickstart 1.3; the 2.0+ modules are rejected with `E11200` |
| `freestanding` | Provisional ABI for LorraineOS; no heap, so `String` and `Box<T>` are unavailable |
| `host` | For development and tests; it is not an Amiga artifact |

## Amiga surface

- **RTG is Picasso96 on UAEGFX and the emulated Picasso IV.** CyberGraphX,
  CGX4, PIP/video overlay, CLUT palette editing, hardware sprites on RTG, and
  RTG text/font rendering are out of scope.
- **Exec worker tasks have no DOS process context.** A task created by
  `std.amiga.exec.createTask` may use arithmetic, module-level scalars, and
  `reportTaskResult`; it must not call DOS, Intuition, `std.io`, allocate, drop an
  owning value, or block. Use a message port and a DOS process for that work.
- **Some optional services are narrower than their APIs.** Clipboard proves the
  write/update path; serial, parallel, and printer prove typed open/close;
  datatypes proves parsing but not visible `DTM_DRAW` rendering; network proves
  a TCP echo through `tcpEcho`. The full list is in the authoritative document.
- **The NDK is not redistributed.** The SDK image builds from public upstream
  sources and verifies downloaded inputs.

## Compiler edge cases

The compiler has a small set of accepted-source edge cases outside the
robustness closeout. They are pre-existing, are not regressions, and do not
appear in the validation corpus. Examples include:

- An imported enum or handle type used as a cast can emit a C call instead of a
  cast.
- A non-integer explicit enum member is accepted without a type check.
- A runtime integer converted to a handle can fail the strict host build.
- Some volatile-array copies and prefix-row shapes can emit `memcpy` into
  volatile storage and fail the strict build.
- A module `const` that does not fold in a `case` label is rejected by the C
  compiler.

Each case has a workaround in the
[authoritative limitations document](https://github.com/OpenLorraine/clorraine/blob/main/docs/known-limitations.md).
If you hit one, a minimal reproduction in an issue is valuable.

## v1.1 ownership surfaces

| Surface | Limitation |
|---|---|
| `Box<T>` allocation | Allocation failure traps; `Box<T>` is not nullable. Use `Box<T>?` as an optional slot |
| `Box<T>` moves | A boxed value cannot be moved out of its box; use `boxSet` or `boxWith` |
| `Box<T>` FFI | A direct box crossing is `E10105`; use `boxWith` or a C-facing struct |
| Capturing closures | Ephemeral and inline-only; they cannot be stored, returned, or captured again |
| `take` captures | Rejected in v1 |

## Verification limits

- AddressSanitizer child programs in the SDK image may need a non-PIE link.
  `make asan` handles this automatically.
- A known GCC 15 loop-condition warning can appear for `for`, `do-while`, or
  conditions whose operands are calls. The emitted C remains valid C99 and GCC
  12 in the SDK image accepts it.
- Benchmark instruction counts are guest CPU instructions, not cycle counts.
  They are useful for deterministic comparisons, not for claiming hardware
  performance.
- ASan does not instrument the m68k-only optional C shim; that code is covered
  by static review, host record models, and m68k runtime gates.

## Reporting a limitation

If a limitation is not listed, open an issue with a minimal source file, the
command, the profile, and the compiler version. The project treats a failing
specification test as an implementation bug, never as a reason to weaken the
test.
