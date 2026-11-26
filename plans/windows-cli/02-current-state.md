# Current State: Windows CLI and installer

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The Linux CLI path is complete (plans `linux-cli-v1.2.0`, `public-preview-v1.2.0`): `install.sh`,
`package.sh`, `make dist`/`dist-check`/`acceptance`/`acceptance-check`, the native toolchain
selection and the verified FS-UAE runner in the driver, and the editor's CLI discovery. The spike
(`spikes/08-windows-cli/README.md`) ran the same driver on Windows 11 after cross-building it and
recorded every gap below.

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `docker/Dockerfile`, `docker/Makefile:27` | Pinned SDK image; `IMAGE ?= clorraine-sdk:dev` | MinGW + Windows target (AR-20); branch tag (AR-37) |
| `Makefile:58-106` | `check`, `dist`, `dist-check` | Windows `cargo check`, Windows staging, PowerShell assets |
| `compiler/src/codegen.rs:811-816`, `compiler/src/codegen/cleanup.rs:319-331` | `#line` directives | Escape the path (AR-16) |
| `compiler/runtime/shim_freestanding.c:27, :123` | `uintptr_t` typedef; x86_64 syscall branch | Guards (AR-44) |
| `compiler/src/clorraine/toolchain.rs:170-235` | Host flags, `"cc"`, `native_syscalls()` | Compiler order (AR-29); Linux-only syscalls (AR-23) |
| `compiler/src/clorraine/setup.rs:116-200, 216-244, 352-377` | Validator, config dir, glibc check | `.exe`, `%APPDATA%`, display paths |
| `compiler/src/clorraine/environment.rs:111-156` | PATH search, missing-tool hints, executable check | `.exe`, Windows hints |
| `compiler/src/clorraine/emulator/select.rs:38-128` | FS-UAE lookup and version | `fs-uae.exe`, PE version (AR-14) |
| `compiler/src/clorraine/emulator/storage.rs:66-112` | Private run directory | Display paths; inherited ACLs on Windows |
| `compiler/src/clorraine/artifacts.rs:32-44` | Artifact names | `.exe` host names (AR-45) |
| `compiler/src/clorraine/new.rs:95-112`, `compiler/templates/starter/tasks.json` | Starter launcher | `.exe` lookup, stable path (AR-32) |
| `compiler/src/lsp/uri.rs:48-90`, `lsp/workspace.rs`, `lsp/source_files.rs` | URI parse/format, document keys | Windows paths (AR-38) |
| `tools/vscode/src/discovery.ts:85-118`, `actions.ts:67-115` | CLI discovery, saved-source checks | Windows location, path comparison (AR-38) |
| `tools/setup/package.sh` | Linux candidate assembly | Windows zip, `install.ps1`, five-asset `SHA256SUMS` (AR-33) |
| `tools/setup/acceptance.py:21-213` | Linux-only evidence schema | Platform-keyed schema (AR-46) |
| `tools/setup/install.sh` | Linux installer | Reference for `install.ps1` parity (AR-24) |
| `docs/linux-cli.md`, `site/`, `README.md`, `docs/known-limitations.md` | User docs | Windows pages (W17) |

### Code Analysis

- `codegen.rs:815` and `cleanup.rs:330` write `#line {line} "{path}"` without escaping; a `\` or
  `"` in the path produces an invalid or wrong C string (spike: `check\hello.clo`).
- `setup.rs:157` looks for `bin/m68k-amigaos-gcc`; `setup.rs:129` keeps the verbatim
  `canonicalize` result, which reaches the config and messages as `\\?\C:\…`.
- `setup.rs:224-244` resolves the config only from `XDG_CONFIG_HOME`/`HOME`.
- `environment.rs:121-123` joins bare tool names; `select.rs:53` looks for `fs-uae`.
- `select.rs:89-128` parses `fs-uae --version` stdout; the Windows build is a GUI-subsystem
  program and prints nothing to a pipe.
- `toolchain.rs:213-215` returns true for any x86_64 host; `shim_freestanding.c:123` selects the
  Linux syscall branch on any x86_64, and `:27` makes `uintptr_t` 32-bit on LLP64 Windows.
- `artifacts.rs:33` names the host program `<stem>`; MinGW writes `<stem>.exe`.
- `new.rs:99-112` joins the bare `clorraine` name on PATH.
- `uri.rs:58-69` turns `file:///c%3A/x` into `/c:/x`; `uri.rs:73-84` percent-encodes `\`.
- `discovery.ts:97` looks only in `~/.local/bin/<name>` without `.exe`.
- `acceptance.py:209` requires `platform == linux` and `:213` a glibc version.

## Gaps Identified

### Gap 1: No Windows build
**Current Behavior:** The SDK image builds Linux binaries only.
**Required Behavior:** `make dist` produces the Windows binaries from the pinned image (AR-20).
**Fix Required:** MinGW and the Windows target in the image; Windows staging in `make dist`.

### Gap 2: The driver assumes Unix names and directories
**Current Behavior:** See Code Analysis (setup, environment, select, artifacts, new).
**Required Behavior:** W11.
**Fix Required:** A platform-parameterized helper layer (03-02).

### Gap 3: Emitted C is wrong for some paths and for freestanding on Windows
**Current Behavior:** Unescaped `#line`; Linux syscalls and a 32-bit `uintptr_t` on Windows.
**Required Behavior:** W12, W13.
**Fix Required:** 03-01.

### Gap 4: Editor and language server break on Windows paths
**Current Behavior:** Wrong URIs, verbatim paths in `sourceFiles`, strict path comparison.
**Required Behavior:** W14. **Fix Required:** 03-03.

### Gap 5: No Windows installer, assets, or evidence
**Current Behavior:** Linux only. **Required Behavior:** W1–W10, W15, W16.
**Fix Required:** 03-04, 03-05.

## Dependencies

### Internal Dependencies
- The pinned SDK image and `make dist` (Linux candidate flow).
- The verified FS-UAE runner and `emulator_runner.c` (unchanged).

### External Dependencies
- AmigaPorts rc24 Windows setup and FS-UAE 3.2.35 Windows zip (pinned, AR-1, AR-13).
- Windows 11 `curl.exe`, `tar.exe`, Windows PowerShell 5.1.
- The user's Windows 11 machine over SSH, with the desktop-session runner (spike).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Defender or SmartScreen flags the unsigned binaries | Med | Med | Watch during testing; revisit signing (AR-18) |
| The parallel session changes shared files (`Makefile`, roadmap, image) | High | Med | Own image tag; shared-file edits at merge (AR-37) |
| A PowerShell 5.1 behaviour differs from the auditors' assumptions | Med | Med | Spec tests run on the real host before implementation is accepted |
| The UAC prompt or standard account behaves unlike the elevated spike | Med | High | Acceptance on a standard account with a person approving UAC (AR-46, AR-49) |
| Emission changes alter Linux output | Low | High | Emission-integrity review; pins updated only for the intended change (AR-16, AR-44) |
