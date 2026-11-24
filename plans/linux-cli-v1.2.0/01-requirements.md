# Requirements: Linux CLI v1.2.0

> **Status**: Draft for preflight
> **Scope**: Linux x86_64, Clorraine 1.2.0
> **Evidence**: `spikes/07-linux-cli-fs-uae/README.md`

## Product statement

A Linux user installs the Clorraine CLI without Docker, downloads the pinned
upstream Amiga toolchain themselves, validates it once, builds a `.clo` program,
and runs it in FS-UAE with their own Kickstart 3.1 A1200 ROM. VS Code uses
the same installed CLI.

The old Python installer/support pipeline is removed only after this path is
proven on a real Linux host.

## Functional requirements

| ID | Priority | Requirement | Acceptance |
|---|---|---|---|
| R1 | Must | `install.sh` installs the versioned CLI archive on Linux x86_64 without Docker, root, or a Python runtime. | ST-18, ST-20 |
| R2 | Must | The installer checks Linux/x86_64, glibc >= 2.38, and the commands it needs before running the downloaded CLI; failures name the missing capability and a package hint. | ST-1, ST-2, ST-3 |
| R3 | Must | Network installs verify a compiled-in archive size and SHA-256 before extraction. `--archive PATH --checksums PATH` installs exact local bytes, verifies the supplied checksum, and records the size; it does not require the release-pinned size. No remote text is piped to a shell. | ST-4, ST-5 |
| R4 | Must | `clorraine setup --toolchain PATH` validates the pinned upstream rc24 root and stores its canonical path. An invalid toolchain never replaces a valid stored one. | ST-7, ST-8, ST-9 |
| R5 | Must | `build` and `run` resolve a toolchain in the order `--toolchain` > `CLORRAINE_TOOLCHAIN` > stored config. Host builds need no toolchain. When none is selected, the marker-gated legacy `PATH` build remains for the Docker SDK; the verified FS-UAE runner requires a native selection. | ST-10, ST-11 |
| R6 | Must | The validator checks `bin/m68k-amigaos-gcc`, the `m68k-amigaos` tree, and links a probe with both `-mcrt=nix20` and `-mcrt=nix13`. | ST-8, ST-9 |
| R7 | Must | With a native toolchain, `run --emulator` requires FS-UAE >= 3.2.0 and a 512 KB Kickstart 3.1 A1200 ROM (KS3.0 untested). A missing or 256 KB ROM is a clear error; AROS is never substituted silently. The legacy `PATH` branch keeps the existing handoff. | ST-12, ST-13 |
| R8 | Must | In the native FS-UAE runner, a compiled guest wrapper records the program return code and writes the status marker only for rc 0. The host requires rc 0 plus status `ok` and reads both before cleanup. | ST-14, ST-15, ST-16 |
| R9 | Must | When a native toolchain is selected, `run --emulator` prints the guest output, returns 0 only for rc 0 plus status `ok`, and returns 1 for a nonzero rc, missing/malformed status or rc, post-spawn emulator failure, read/capture failure, or timeout. Selection/media errors return 2. Empty output is valid. The marker-gated legacy handoff keeps its existing command output and exit 0; it produces no guest status. | ST-15, ST-16 |
| R10 | Should | When a native toolchain is selected, `run --emulator --capture` writes the guest output to the normal `out.txt` artifact and prints its path. The legacy handoff capture remains unchanged. | ST-17 |
| R11 | Must | The VS Code extension discovers the installed CLI, uses `clorraine.toolchain.path` for non-host actions, and passes FS-UAE/ROM settings as CLI flags. `clorraine.sdk.path` is read once with a deprecation warning. | ST-21, ST-22, ST-23 |
| R12 | Must | `make dist` builds a local candidate archive, a separate VSIX, and checksums; `make dist-check` runs hermetic producer/installer tests in `make check`; `make acceptance-check` validates recorded evidence and is not a CI gate. | ST-19, ST-24, ST-25 |
| R13 | Must | The old native Python installer, its tests, the support/distribution producers, and the release workflows are removed only after the local acceptance passes. Docker SDK setup and `tools/distribution/inputs/` remain. Public docs, site, release notes, and changelog match the new path. | ST-26, ST-27, ST-28 |
| R14 | Must | Updates install side-by-side with a unique version directory and an atomic `current` switch; the lock is stale-safe; `--modify-path` is quoted and idempotent; uninstall preserves config/versions unless `--purge` is explicit. | ST-6 |

## Quality requirements

| ID | Requirement |
|---|---|
| Q1 | Emitted C, compiler flags, and both Amiga profiles stay unchanged. The spike built the existing corpus and both profiles with the upstream toolchain. |
| Q2 | Every new behavior has a specification test written before implementation, plus implementation tests for internals. |
| Q3 | The FS-UAE runner uses a private 0700 directory outside the build tree and removes it only after the result has been read or the run has failed safely. |
| Q4 | No toolchain archive, Kickstart ROM, FS-UAE binary, or licensed media is bundled, mirrored, or downloaded by Clorraine. |
| Q5 | The public CLI surface is `setup`, `build`, `run`, `emit`, `--toolchain`, `--fs-uae`, `--kickstart`, `--capture`, and the existing profile/mode flags. No `--sdk-root`, `--installation`, or Python-helper surface remains. |
| Q6 | User-facing errors are one line, name the flag or file, and give the next action. No panic or raw loader error is the normal path. |
| Q7 | Documentation states glibc 2.38+ and the Ubuntu 24.04+ minimum, the rc24 pin, the tested KS3.1 A1200 ROM requirement (KS3.0 untested), the wrapper oracle, and that AROS is manual-only and not an acceptance path. Acceptance records the actual host glibc rather than a constant. |

## Non-goals

- macOS arm64 or Windows x64. They are a separate follow-up plan.
- WSL support or heuristic WSL rejection.
- Downloading or mirroring the AmigaPorts toolchain.
- Bundling FS-UAE or Kickstart ROMs.
- A verified AROS guest run.
- An A500/Kickstart 1.3 CLI emulator recipe; the existing SDK A500 gates remain
  the 1.3 proof.
- Publishing a GitHub release, tagging, or uploading assets. Publication needs
  separate approval.
- A debugger, graphics capture, or editor-managed emulator installation.
- The old support-wheel or native installer compatibility layers.

## Traceability

| Requirement | Design | Tests |
|---|---|---|
| R1–R3 | `03-03-install-release.md` | ST-1–ST-5, ST-18–ST-20 |
| R4–R6 | `03-01-toolchain.md` | ST-7–ST-11 |
| R7–R10 | `03-02-fs-uae.md` | ST-12–ST-17 |
| R11 | `03-04-editor-migration.md` | ST-21–ST-23 |
| R12 | `03-03-install-release.md` | ST-19, ST-24, ST-25 |
| R13 | `03-03-install-release.md`, `03-04-editor-migration.md` | ST-26–ST-28 |
| R14 | `03-03-install-release.md` | ST-6 |
| Q1–Q7 | all design documents | all |
