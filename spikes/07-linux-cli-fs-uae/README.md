# Linux CLI and FS-UAE spike

This spike tested the product path before the plan was rewritten: a Linux user
downloads the upstream Amiga toolchain, Clorraine validates it, builds a
program, and FS-UAE runs it with the user's Kickstart ROM.

Date: 2026-11-24. Host: Ubuntu 26.04 x86_64, glibc 2.43, FS-UAE 3.2.35.

## Pinned inputs

| Input | Value |
|---|---|
| Toolchain | AmigaPorts `m68k-amigaos-gcc` v16.2-rc24 |
| Asset | `m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz` |
| URL | `https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz` |
| Bytes | `69,868,548` |
| SHA-256 | `b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100` |
| Layout | `m68k-amigaos-gcc-16.2/{bin,m68k-amigaos,m68k-kick13,lib,libexec,share}`; `m68k-amigaos/` contains `include`, `lib`, `libnix`, `ndk-include`, and `ndk13-include` |
| Kickstart used | `amiga-os-310-a1200.rom`, 524,299 bytes, SHA-256 `541f7467bce4f4b07a6eb5250d4586a4679c2da51ad30b48ba93421a901b1bb0` |
| FS-UAE | 3.2.35-2 (Ubuntu package); `fs-uae --version` prints `3.2.35` |

GitHub's asset metadata reports the same SHA-256 digest; no separate checksum
file is attached. The documentation and acceptance pin the size and hash above;
`setup` pins the rc24 version string and probes the extracted tree, so the
contract does not depend on API metadata.

## Host compatibility

The rc24 host tools are dynamically linked and require **glibc 2.38**.

| Host | glibc | `m68k-amigaos-gcc --version` |
|---|---|---|
| Ubuntu 24.04 | 2.39 | `m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b` |
| Ubuntu 22.04 | 2.35 | fails: `version 'GLIBC_2.38' not found` |

The Linux minimum is therefore Ubuntu 24.04 or another distribution with
glibc 2.38 or newer. Ubuntu 22.04 cannot run this toolchain.

The compiler works from an absolute path with an empty environment and does not
need its `bin` directory on `PATH`. Both `-mcrt=nix20` and `-mcrt=nix13` link
against the toolchain's own libnix.

Ubuntu 24.04 was tested for the toolchain probe only; the full product path ran
on Ubuntu 26.04. Ubuntu 24.04 is the minimum because of the toolchain, not a
fully exercised host in this spike. Acceptance records the actual host and must
repeat the full path on the release baseline.

## Build results

`clc` emitted the translation unit and the upstream compiler built it with the
driver's existing flags:

```
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g \
  -Wall -Wextra -Werror -o hello hello.c -lamiga -lm
```

| Program | Profile | Result |
|---|---|---|
| `examples/hello/hello.clo` | hosted-amiga | build and run pass |
| `examples/hello/hello.clo` | hosted-amiga-13 | build and run pass |
| `file-copier` | hosted-amiga | build and guest output pass |
| `string-parser` | hosted-amiga | build and guest output pass |
| `buffer-container` | hosted-amiga | build and guest output pass |
| `volatile-mmio` | hosted-amiga | build and guest output pass |
| freestanding `print` probe | freestanding | build-only pass (no LorraineOS to run) |

The release line (`-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections
-s`) also built a 17,548-byte hello. A freestanding `print` probe built with
`-ffreestanding -fno-builtin -nostdlib -lg` and produced a 7,928-byte AmigaOS
executable; it was not run because there is no LorraineOS target. No `-Werror`
diagnostic came from the emitted C.

## Guest run recipe

FS-UAE boots an A1200 with a directory hard drive. A simple two-line recipe was
tested and rejected:

```
CNEXT:program >CNEXT:result.txt
echo marker >CNEXT:status.txt
```

A program returning 1 continued and wrote the marker, so the marker was not
success-only. `FailAt` is not available on the bare directory drive; the
command fails and aborts the script before it can lower the failure level.

The reliable recipe uses a small wrapper compiled with the same toolchain:

```c
#include <proto/dos.h>
#include <stdio.h>

int main(void) {
    LONG rc = SystemTagList("CNEXT:program >CNEXT:clorraine-result.txt", NULL);
    FILE *f = fopen("CNEXT:clorraine-rc.txt", "w");
    if (f) { fprintf(f, "%ld\n", (long)rc); fclose(f); }
    if (rc == 0) {
        f = fopen("CNEXT:clorraine-status.txt", "w");
        if (f) { fputs("ok\n", f); fclose(f); }
    }
    return 0;
}
```

The drive contains `runner`, `program`, and `s/startup-sequence` with only
`CNEXT:runner`. The wrapper runs the program with its output redirected, records
the return code, and writes the status only for zero. The host polls the
status/rc files, then kills and waits for FS-UAE, then reads both and removes
the private run.

| Program return | Status | rc file | Result |
|---|---|---|---|
| 0 | `ok` | `0` | pass |
| 1 | absent | `1` | fail |
| 5 | absent | `5` | fail |
| 9 | absent | `9` | fail |

The wrapper was 20,224 bytes with `-mcrt=nix20`. Boot-to-result took about
6 seconds. The rc file can appear before the status file; the host must wait for
both, not either. The earlier corpus and `hello13` runs used the two-line
recipe; all programs returned zero, so their output evidence is still valid,
but the product oracle is the wrapper above. A `hosted-amiga-13` build of
`file-copier` also ran with the wrapper on A1200/KS3.1: rc 0, status `ok`, and
the output matched its expected file.

The four corpus programs matched their `expected-output.txt` byte-for-byte after
CR removal. `hello13` (the A500 profile binary) also ran correctly on the
A1200/KS3.1 machine.

## AROS observation

With no Kickstart file, FS-UAE 3.2.35 booted its built-in AROS and the earlier
two-line directory-drive script produced correct hello output. This is recorded
as an observation only: the product decision remains that a real Kickstart ROM is
required for a verified guest result, and the CLI must not silently substitute
AROS for a missing ROM.

## Current driver gaps

| Area | Current behavior | Consequence |
|---|---|---|
| Toolchain selection | `--sdk-root` requires `<root>/sdk/amiga`, `installation.json`, `.ndk-prepared.json`, and a paired `sdk/vamos` venv | The upstream rc24 root is rejected; a PATH-only build reports "this command needs the pinned SDK container" |
| Build | The compiler path comes from the old native SDK assembly | The upstream `bin/m68k-amigaos-gcc` cannot be selected |
| FS-UAE | The runner writes only `program\n`, never reads a guest result, and deletes storage after the emulator stops | A launch cannot distinguish success from failure |
| FS-UAE options | `--capture` is rejected for native emulator runs | Scripts cannot collect guest output |
| ROM | Any readable ROM is accepted; the recipe is hard-coded A1200 | A 256 KB Kickstart 1.3 ROM fails opaquely |
| Profile | The extension and CLI accept both Amiga profiles for FS-UAE | The A1200 recipe works for both; no rejection is needed |
| glibc | No host check | An old host gets a loader error instead of an actionable message |
| Installer | The old Python/native installer expects the superseded SDK layout | It cannot install or validate this toolchain |

## Plan consequences

1. Replace the old native SDK layout with a small upstream-toolchain root
   (`bin/m68k-amigaos-gcc` plus `m68k-amigaos/`). Validate it with a two-crt
   compile probe.
2. Set the Linux minimum to glibc 2.38 (Ubuntu 24.04 class) and check it before
   running the CLI.
3. Require a 512 KB Kickstart 3.1 A1200 ROM for `run --emulator`; KS3.0 is
   untested. Do not auto-launch AROS.
4. Make the guest oracle explicit: the compiled wrapper writes
   `clorraine-rc.txt` and, only for rc 0, `clorraine-status.txt`; read both
   before cleanup, with a timeout.
5. Keep the existing compiler flags and both Amiga profiles; the A1200 recipe
   runs both.
6. Do not include publication in the next plan. Build a local candidate archive
   and prove it with `--archive/--checksums`; publication needs separate
   approval.

## Limitations

- One Linux host and one Ubuntu 24.04 container; no other distributions.
- One licensed KS3.1 A1200 ROM; no KS3.0 test.
- No installer, archive, editor, or release run in this spike.
- No m68k debugger, graphics capture, or A500/KS1.3 boot.
- No macOS or Windows evidence.

## Re-run

The spike artifacts were temporary. To repeat the build check, extract the
pinned asset, emit hello with `clc`, and use the command in
[Build results](#build-results). To repeat the guest check, stage the directory
drive and startup script from [Guest run recipe](#guest-run-recipe) and pass
`--amiga-model=A1200 --hard-drive-0=... --kickstart-file=...` to FS-UAE under
`xvfb-run`.
