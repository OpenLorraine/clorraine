# Requirements: native Linux delivery

> **Parent**: [Index](00-index.md)
> **Type**: Standalone feature

## Functional requirements

| ID | Required outcome | Decision |
|---|---|---|
| R1 | Deliver v1.2.0 Linux x86_64 binaries compatible with Ubuntu 22.04 and tested on 22.04/24.04. | AR-12 |
| R2 | Provide the three compiler binaries and a compatible, reusable prebuilt Amiga toolchain; installation never compiles dependencies. | AR-13/18 |
| R3 | A successful native installation supports host build/run and Amiga build/run through vamos. Missing Ubuntu prerequisites produce an actionable command and no false success. | AR-26/27 |
| R4 | Obtain the pinned NDK upstream and prepare its required headers with prebuilt helpers; verify integrity and preserve notices. | AR-18/28 |
| R5 | Install/update in the selected user-owned root, defaulting to `~/.clorraine`, with explicit reruns and exact release selection. | AR-15/20 |
| R6 | Verify compatibility and integrity before activating a complete installation. Failures preserve a working installation; concurrent installation is rejected. | AR-20 |
| R7 | Build/run from the CLI with installed SDK discovery and configurable SDK/emulator/ROM paths. Preserve existing host, emit, freestanding, vamos and Docker behavior. | AR-19/29 |
| R8 | Native FS-UAE Run stages a program, launches the user's emulator as A1200/KS3.1, waits, and cleans its own run data. Its status describes FS-UAE, not the guest process. | AR-14/19 |
| R9 | Provide trusted-workspace VS Code Build, Run and Run in FS-UAE actions over the saved active entry file and its imported user sources, sharing driver behavior. | AR-21 |
| R10 | Make language-server and driver discovery work in desktop-launched VS Code without inherited terminal PATH. Provide the VSIX and explicit install guidance. | AR-20/21 |
| R11 | Support and verify Linux install/build plus VS Code Remote-WSL on WSL2. | AR-23/24 |
| R12 | Prebuild support independently, reuse exact support identities across compiler releases, and allow support-only distribution revisions without recompiling an unchanged core. | AR-13/22 |
| R13 | Publish only after explicit approval and complete acceptance evidence; release assets and manifests are versioned and immutable. | AR-22/24 |
| R14 | Update release version records, notices, runbook, installation/WSL/editor guidance and roadmap without losing historical release facts. | AR-25 |

## Boundaries

The initial native emulator target is a deployment recipe, not a change to the
language profiles or m68k CPU flags. Both `nix13` and `nix20` builds remain
required. The existing SDK acceptance matrix remains authoritative for older
Amiga systems.

Native Windows/macOS packaging, Linux/Windows ARM artifacts, automatic
WSL-to-Windows emulator launch, bundled FS-UAE/media, native FS-UAE capture,
Workbench provisioning and automatic shell-profile/extension changes are outside
this delivery under AR-12/14/16/19/20/23. The two parked roadmap features remain
parked. Existing Docker capture and host/vamos capture remain supported.

## Compatibility and security requirements

- Keep compiler emission, frozen language specification, target ABI and existing
  compilation flags unchanged; installation selects tools, not new language rules.
- User input and downloaded metadata are data, never shell expressions.
  Validate versions, schemas, paths, archive members/links and compatibility.
- Keep untrusted workspaces from causing Build/Run or selecting executable
  overrides. Use direct process argument arrays.
- Use TLS and the approved published-hash trust model. A compromised publisher
  remains outside the protection provided by an unsigned manifest; do not
  describe hashes as independent publisher authentication.
- Never log tokens or upload ROMs, Workbench data, user paths or private run
  contents as public release evidence. Publication write permission belongs only
  to the approved release job.
- Do not fetch missing runtime dependencies from a package index during
  installation or fall back to a source build. Use the verified local wheel set.
- Bound network operations and malformed metadata/archive processing. The
  installer must finish with a failure rather than wait indefinitely.

## Acceptance

Acceptance requires every applicable case in [07](07-testing-strategy.md), the
approved project gates and proof from the actual candidate archives. A working
developer checkout/container does not substitute for a clean installed-package
test.

Required evidence includes clean Ubuntu installations, relocation with the
original prefix unavailable, host and both Amiga-profile output, native FS-UAE
execution, desktop VS Code, actual WSL2, update/failure recovery, complete source
records and notices, and a truthful quick-start walkthrough.

Coverage is by required behavior and negative controls, not an invented
line-coverage percentage. Every requirement maps to concrete specification
cases. A first-phase feasibility failure stops execution for a user ruling;
it does not permit changing R1–R14 silently.
