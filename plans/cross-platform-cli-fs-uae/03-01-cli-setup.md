# CLI, setup and installation

> **Document**: 03-01-cli-setup.md
> **Parent**: [Index](00-index.md)
> **AR**: AR-4/5/6/7/8/9/19/26/28/30/31/32

## Overview

One CLI family, one setup command, one portable home. Users download the
upstream Amiga toolchain themselves; `clorraine setup` validates it, discovers
FS-UAE and writes the config. The bootstrap scripts download only our CLI.

## Public commands

```text
clorraine setup [--toolchain PATH] [--fs-uae PATH] [--kickstart PATH]
clorraine build -p PROFILE FILE...
clorraine run   -p PROFILE [--emulator] FILE...
clorraine emit  -p PROFILE FILE...
clorraine --version
clorraine-lsp --version
```

`setup` is idempotent and setup-only. It never downloads a toolchain, runs
`sudo`, installs system packages or edits PATH. `--toolchain` is accepted only by
`setup` and writes the selected path into config; `build`/`run`/`emit` use the
config or the legacy `--sdk-root`/`CLORRAINE_SDK`. `--fs-uae`/`--kickstart` on
other commands remain usage errors; `run --emulator` is the only run form that
uses them.

## Installation layout

```text
<home>/
  bin/                       current clc, clorraine, clorraine-lsp
  versions/1.2.0/            version payload + completion marker
  config                     toolchain=, fs_uae=, kickstart=
```

`<home>` is `~/.clorraine` on Linux/macOS and `%LOCALAPPDATA%\Clorraine` on
Windows. The toolchain lives wherever the user extracted it; `config` records
its absolute path. There is no Clorraine toolchain cache.

The config is a text `key=value` file with absolute paths only. It contains no
secrets, ROM data or archive bytes. Invalid/relative values fail setup. Setup
writes config only after a successful build smoke; a failed setup leaves the
previous config untouched. The installer holds an exclusive home lock across
unpack/validate/setup/switch. Setup itself does not publish version directories.

## Toolchain acquisition and validation

The user downloads the upstream AmigaPorts release for their OS from
<https://github.com/AmigaPorts/m68k-amigaos-gcc/releases> and extracts it. The
baseline documented version is `v16.2-rc24`; the exact tag and hashes are shown
in the quick-start for the supported platforms (AR-3/26). Clorraine never
mirrors or redistributes the toolchain or NDK.

`clorraine setup --toolchain PATH` (or a previously recorded path) validates:

- `PATH/bin/m68k-amigaos-gcc` exists (`m68k-amigaos-gcc.exe` on Windows) and is
  an ordinary executable;
- it reports a m68k-amigaos GCC version;
- a generated hello program builds for `hosted-amiga` and `hosted-amiga-13`
  and the output starts with the Amiga hunk magic `0x000003F3`.

A failed validation names the failing path/step and writes no config. A user who
already has the compiler on PATH may pass its root explicitly; no implicit scan
of Documents/Downloads is performed. Setup prints the upstream release/licence
URL and records it in the runbook (AR-20).

## Driver toolchain resolution

The driver must accept the upstream toolchain root as the selected SDK. The
precedence is one contract (AR-26):

1. explicit `--sdk-root PATH` (legacy) or `CLORRAINE_TOOLCHAIN`/`CLORRAINE_SDK`;
2. `toolchain=` in `<home>/config`;
3. an existing installed/SDK location (development compatibility).

An explicit invalid path never falls back. The upstream root is accepted when
its `bin/` compiler exists; the old `sdk/amiga` + `installation.json` layout
remains accepted for existing installs. The bundled `sdk/vamos` venv is no
longer required for Amiga builds; plain `run` uses PATH `vamos` (AR-18).

## Host capability check

| Platform | Required | Optional |
|---|---|---|
| Linux | x86_64, glibc >= 2.35, `curl`, `tar`, `sha256sum` | host `cc`/headers only for `-p host` |
| macOS | arm64, macOS >= 14, `curl`, `tar`, `shasum` | host `cc` only for `-p host` |
| Windows | x64, Windows 10 1809+, `curl.exe`, `tar.exe`, PowerShell `Get-FileHash` | host C compiler only for `-p host` |

Missing system tools print the platform command; setup never runs `sudo`.
Missing host `cc` does not block Amiga builds (AR-9/10). WSL is untested and
unsupported; no heuristic rejection is attempted (AR-32).

## Bootstrap script (Linux v1.2.0)

The docs publish `install.sh` and its SHA-256. Users download, verify and run it;
no remote text is piped to a shell (AR-28). `install.ps1` belongs to the
macOS/Windows follow-up plan, not v1.2.0.

`install.sh`:

- require x86_64 Linux; reject other platforms with the supported list;
- download `clorraine-1.2.0-linux-x64.tar.gz` and `SHA256SUMS`, or accept
  `--archive PATH --checksums PATH` for a downloaded draft/offline install;
- verify with `sha256sum` before unpacking;
- list the archive with `tar -tvf`, reject absolute paths, `..`, links and
  non-regular members, then extract into a private staging directory;
- run `staging/bin/clorraine --version` by absolute path;
- run `staging/bin/clorraine setup ...` by absolute path (first install) and
  require success;
- only after setup succeeds, move the complete directory to `versions/1.2.0`
  and switch `bin` with copy/rename; on any failure remove staging and keep the
  old `bin`/config;
- hold the exclusive home lock across the whole operation;
- print the PATH export; edit the profile only with `--modify-path`.

The script fails closed on HTTP/hash/member/marker errors and never leaves a
partial version active. A failed setup leaves the previous `bin`/config in place.

## Update and uninstall

- Update: re-run the installer. A new version is unpacked into private staging,
  validated (version + setup), then moved into `versions/<version>` and `bin`
  switches. The previous version remains until the new one passes.
- Same-version re-run: build the replacement in private staging, validate, then
  replace `versions/<version>`; a failure leaves the old directory and config.
- Uninstall: delete `<home>`; remove the opt-in profile block (Linux). No service
  or registry key beyond PATH.

## Error handling

| Case | Behavior | AR |
|---|---|---|
| Unsupported OS/arch | Exit 1, list supported platforms | AR-2/32 |
| glibc/macOS/Windows too old | Exit 1 with required/observed | AR-9/32 |
| `curl`/`tar`/hash tool missing | Exit 1, print install command | AR-9/31 |
| Invalid `--toolchain` | Exit 1 naming the path; no fallback | AR-26 |
| Toolchain smoke fails | Delete/no config; previous config remains | AR-19 |
| FS-UAE missing | Setup succeeds with warning; run --emulator explains | AR-11 |
| ROM missing | Setup records none; launch-only AROS fallback | AR-12/29 |
| Interrupted install | Previous bin/config remains; new version not active | AR-30 |
| Config invalid/foreign-owned | Exit 1; no executable selection | AR-19/23 |

## Testing requirements

- Rust tests for toolchain validation, config parse/write, resolver precedence,
  PATH `vamos`, missing tool messages and failed-smoke rollback.
- Shell tests use fake `curl`/hash tools on PATH with local fixed bytes;
  production still invokes the real `curl`/hash tools with literal argv.
- Tests cover download/hash/marker/absolute invocation, re-run, lock contention
  and uninstall.
- No production download of the toolchain; fixture toolchains only.
