# FS-UAE and editor integration

> **Document**: 03-03-fs-uae-editor.md
> **Parent**: [Index](00-index.md)
> **AR**: AR-11/12/13/18/21/29/32

## Overview

FS-UAE is the supported Amiga run path. The driver stages a private A1200
directory drive; this component extends discovery/ROM handling and keeps the VS
Code extension a thin client of the installed CLI.

## FS-UAE discovery (single precedence)

1. `--fs-uae PATH` on `run --emulator` or `setup`;
2. `CLORRAINE_FS_UAE`;
3. `fs_uae=` in the Clorraine config;
4. PATH (`fs-uae`, `fs-uae.exe`);
5. platform defaults:
   - macOS: `/Applications/FS-UAE.app/Contents/MacOS/fs-uae`, then
     `~/Applications/FS-UAE.app/Contents/MacOS/fs-uae`;
   - Windows: `Program Files`/`Program Files (x86)` FS-UAE installations;
   - Linux: no extra default beyond PATH.

Do not search Documents/Downloads. An explicit invalid path fails; no fallback.
FS-UAE is user-provided and never bundled.

## Kickstart and AROS

ROM precedence: `--kickstart` > `CLORRAINE_KICKSTART` > `kickstart=` config > no
ROM. A configured but missing/unreadable ROM is a hard error.

With no ROM, the driver prints one warning and launches FS-UAE without a
`kickstart_file` override, allowing FS-UAE's built-in AROS ROM. AROS is a
launch-only fallback (AR-29): it is not proof that the guest program ran and is
not used for guest-success acceptance. Guest-success evidence requires a user
Kickstart ROM plus an observable program result.

## Run behavior

- `run -p hosted-amiga --emulator FILE...` builds, stages the A1200 drive, writes
  `run.fs-uae`, launches FS-UAE and returns the emulator process status. It never
  claims the guest exit status.
- `run -p hosted-amiga-13 --emulator` is a usage error in v1.2.0: the native
  recipe is A1200/KS3.1 only. An A500 model recipe is follow-up scope.
- Plain `run` on an Amiga profile uses `vamos` only if found on PATH. If absent,
  exit 1 with amitools/`--emulator` guidance (AR-18).
- `--fs-uae`/`--kickstart` are usage errors except on `setup` and
  `run --emulator`.
- Host runs directly; freestanding keeps the existing host-emulation path.

## Staging and path guards

- Private run directory under the system temp dir, outside the build tree.
- Copy only the program and `s/startup-sequence`; the startup script runs the
  program with output redirected to `result.txt`, and the host reads that file
  after the emulator exits. `result.txt` is the guest-success marker.
- Mount only that directory; no shell/INI interpolation.
- Preserve the existing guards: FS-UAE mount/INI commas are rejected in the run
  path, a ROM filename ending in a space is copied privately, control characters
  are rejected. Add tests for these in the platform port.
- Windows: isolate `APPDATA`/`LOCALAPPDATA`/FS-UAE base/cache; macOS: keep
  `HOME`/XDG isolation; Linux: keep `HOME`/XDG and X authority.
- Clean only after confirmed emulator termination.

## VS Code extension

| Behavior | Contract |
|---|---|
| Discovery | Machine setting, `<home>/bin`, trusted dev builds, PATH. |
| Config | Reads the same `<home>/config`; no second toolchain setting. |
| Build/Run | Call the installed `clorraine` with explicit argv. |
| Settings | Remove `clorraine.sdk.path`, `fsUae.path`, `kickstart.path` with a one-time warning that the CLI config is authoritative; do not keep two sources. |
| Missing CLI | Actionable install/verify command; no bundled compiler. |
| Platform | Linux desktop first; Windows/macOS follow-up; no WSL. |
| VSIX | GitHub release asset first; Marketplace/Open VSX later. |

The extension uses the existing machine setting, `<home>/bin`, trusted dev
builds and PATH. It never edits PATH, installs the CLI or downloads the
toolchain.

## Error handling

| Case | Behavior | AR |
|---|---|---|
| FS-UAE missing | Build succeeds; run --emulator explains | AR-11 |
| No ROM | Warn, launch AROS; not guest proof | AR-12/29 |
| Explicit bad ROM | Exit 1 before launch | AR-12 |
| No `vamos` | Exit 1 with amitools/FS-UAE choices | AR-18 |
| Bad mount/ROM path | Exit 2 with path remedy | AR-23 |
| Editor CLI missing | No task; show install command | AR-13 |

## Testing requirements

- Rust precedence/AROS/staging tests; exact argv.
- Node discovery/action tests, including missing CLI and config migration.
- Linux FS-UAE gate; Windows/macOS real-machine evidence before follow-up release.
