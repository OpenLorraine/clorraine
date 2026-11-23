# Current State: Cross-platform CLI and FS-UAE delivery

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

- `clc`, `clorraine` and `clorraine-lsp` are one std-only Rust workspace. They
  build only for the host platform today (Linux in CI).
- The driver already builds all four profiles and launches FS-UAE for the two
  Amiga profiles. `compiler/src/clorraine/emulator.rs` stages a private A1200
  directory drive, writes a config, passes literal path arguments and returns
  the emulator process status.
- The driver already discovers an SDK/toolchain (`environment.rs`), a
  Kickstart ROM and FS-UAE (`emulator.rs`), and supports `vamos` for plain
  terminal runs.
- The VS Code extension already discovers the installed driver/LSP
  (`tools/vscode/src/discovery.ts`) and runs Build/Run/Run in FS-UAE through the
  driver (`tools/vscode/src/actions.ts`).
- The superseded plan added a Python installer (`tools/setup/install.sh`,
  `install_native.py`, `native_archive.py`, `native_state.py`), a support build
  and audit pipeline (`tools/distribution/`), a lock and wheels, two release
  workflows and a large acceptance suite.
- Upstream AmigaPorts publishes prebuilt toolchains:
  `v16.2-rc24` has Linux x64, macOS arm64 and Windows x64 assets; `v16.2-rc25`
  currently lacks Windows. The archives contain a relocated `bin/` tree and the
  prepared NDK 3.2 headers.

### Relevant Files

| File | Purpose | Change |
|---|---|---|
| `compiler/src/clorraine/cli.rs` | CLI options and command dispatch | Add `setup` command and flags |
| `compiler/src/clorraine/mod.rs` | Command execution and required tools | Setup path; optional `vamos` |
| `compiler/src/clorraine/environment.rs` | SDK/tool discovery | Toolchain root from config; no bundled venv requirement |
| `compiler/src/clorraine/emulator.rs` | FS-UAE launch | Windows/macOS discovery; optional ROM/AROS fallback |
| `compiler/src/clorraine/artifacts.rs` | Build dirs and permissions | Windows directory/permission port |
| `compiler/src/clorraine/native_metadata.rs` | Metadata/stat handling | Windows-safe file checks |
| `compiler/src/clorraine/setup.rs` (new) | Validate toolchain/config | New |
| `tools/setup/install.sh` | Linux bootstrap | Rewrite for new archive |
| `tools/setup/install.ps1` (new) | Windows follow-up bootstrap | Later plan |
| `tools/vscode/src/discovery.ts` | CLI discovery | Linux home/PATH |
| `tools/vscode/src/actions.ts` | Editor actions | Use CLI commands and config |
| `.github/workflows/release.yml` (new) | Linux release build | Replace old workflows |
| `tools/setup/install_native.py`, `native_archive.py`, `native_state.py`, `tools/distribution/` code, old workflows | Superseded path | Remove after new Linux proof; retain licence inputs if referenced |

### Code Analysis

- `emulator.rs:35-75` requires a Kickstart ROM unconditionally. The new design
  must allow no ROM and let FS-UAE use AROS, with a warning.
- `emulator.rs:214,267` and `mod.rs:522` use Unix-only permissions/flags;
  `artifacts.rs:107` uses `DirBuilderExt`. For Linux v1.2.0 these paths are
  already valid; the follow-up adds Windows alternatives.
- `environment.rs:115-149` requires the bundled `sdk/vamos` venv and
  `wheels.json` for plain Amiga runs. This becomes an optional PATH lookup.
- `discovery.ts` currently describes the Linux extension host and searches
  `~/.clorraine/bin`; Windows home/PATH spelling and `clorraine.exe` need tests.
- `install_native.py` is Python-based and Ubuntu-only; it is not a viable
  cross-platform base.

## Gaps Identified

| Gap | Current Behavior | Required Behavior | AR |
|---|---|---|---|
| No cross-platform setup | Linux/Python installer only | `clorraine setup` on three OSes | AR-3/5 |
| No upstream toolchain path | Own audited build and lock | Download pinned upstream release | AR-3/20 |
| No Windows port | Unix-only file/process details | Windows x64 build and FS-UAE launch | AR-2 |
| ROM required | FS-UAE fails without Kickstart | Optional ROM; AROS fallback warning | AR-12 |
| Bundled vamos required | Installed venv expected | Optional PATH `vamos`; FS-UAE primary | AR-18 |
| Ubuntu/Python pin | Rejects non-Ubuntu hosts | Capability check | AR-9 |
| Heavy release pipeline | Self-hosted runner, support lock | One hosted matrix | AR-22 |
| Old paths remain | Python/support code still present | Remove after new proof | AR-16 |

## Dependencies

### Internal

- Existing compiler, language server, driver, FS-UAE staging and editor actions.
- Existing Docker SDK and `make verify` for the compiler gates.
- Existing version 1.2.0 records.

### External

- AmigaPorts `m68k-amigaos-gcc` release assets and checksums.
- System `curl` and `tar` (plus `xz` support).
- FS-UAE, user-provided; user Kickstart ROM or AROS.
- VS Code (extension client only).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Upstream toolchain incompatible with our emitted C | Medium | High | Spike before implementation; pin exact tag; stop and revise if the upstream corpus leg fails |
| Upstream asset disappears | Low | High | Pin exact tag/checksum; `--toolchain` escape hatch; do not mirror |
| Unsigned binaries trigger warnings | High | Low | Script installs first; document; signing later |
| FS-UAE A1200/KS3.1 behavior differs from expectations | Medium | Medium | Phase 1 user-ROM run; stop if it fails |
