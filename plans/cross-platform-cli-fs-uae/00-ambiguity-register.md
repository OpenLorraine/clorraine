# Ambiguity Register: Cross-platform CLI and FS-UAE delivery

> **Status**: ✅ GATE PASSED — all 33 items resolved
> **Last Updated**: 2026-11-23 16:10

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|---|---|---|---|---|
| 1 | Scope / feature | Primary workflow and product boundary | Installer-first CLI only / VS Code only / CLI-first with VS Code as a client | CLI-first: build and run from a terminal or Makefile; VS Code is a client of the installed CLI | ✅ Resolved |
| 2 | Scope / platform | Which operating systems and architectures are in the first delivery | All at once / Linux first then macOS + Windows / Linux + macOS only | Linux x64 first, then macOS arm64 and Windows x64; no WSL, Linux ARM, Windows ARM or Intel Mac | ✅ Resolved |
| 3 | Technical / dependency | Where the m68k toolchain comes from | Build it ourselves per OS / upstream prebuilt releases / require the user to install it | Upstream AmigaPorts prebuilt toolchain; baseline pin `v16.2-rc24` (it has Linux, macOS arm64 and Windows assets); upgrade only to a tag with all three assets after a passing spike; no binary mirror | ✅ Resolved |
| 4 | Technical / dependency | How the install scripts fetch our CLI while the workspace stays std-only | Add crates / system tools / scripts do all downloads | Bootstrap scripts use the platform `curl`/`tar` and hash tool; no crates. Toolchain acquisition is manual (AR-26). | ✅ Resolved — superseded download detail by AR-26/31 |
| 5 | Naming / interface | The setup command surface | `clorraine setup` / separate setup binary / install only through scripts | `clorraine setup`, re-runnable, with `--toolchain`, `--fs-uae` and `--kickstart` overrides | ✅ Resolved |
| 6 | Data & state | Installation root and layout across operating systems | one portable home / OS-conventional directories | One portable home: `~/.clorraine` on Linux/macOS and `%LOCALAPPDATA%\Clorraine` on Windows; toolchain shared across CLI versions | ✅ Resolved |
| 7 | UX / presentation | Whether installation edits PATH | always / never / Windows automatic with Unix opt-in | Windows adds the user PATH automatically; Linux/macOS print the export line and support `--modify-path` opt-in | ✅ Resolved |
| 8 | Scope / naming | Release version | keep 1.2.0 / 1.3.0 / 2.0.0 | Keep `1.2.0` because no asset from the superseded plan was published | ✅ Resolved |
| 9 | Compatibility | Linux host acceptance policy | Ubuntu-only / capability check / wider glibc floor | Capability check: x86_64 Linux, glibc >= 2.35, `curl` and `tar`; tested on Ubuntu 22.04/24.04 and Debian 12; other distros best-effort | ✅ Resolved |
| 10 | Behavioral | Host C compiler handling | require it always / check only for host profile / install it silently | Check only when the host profile is used; print the OS package command; never run `sudo` | ✅ Resolved |
| 11 | Integration / scope | Who provides FS-UAE | bundle it / user installs it / setup installs it | User installs FS-UAE; `clorraine setup` only discovers, validates and records its path | ✅ Resolved |
| 12 | Behavioral / edge | Kickstart ROM requirement | require user ROM / AROS fallback / always AROS | Use the user's licensed ROM when configured; otherwise try FS-UAE's built-in AROS ROM with a warning | ✅ Resolved |
| 13 | Integration | VS Code extension delivery and role | client only / client plus bundled toolchain / editor first | Extension is a CLI client; first release publishes the VSIX as a GitHub release asset, Marketplace/Open VSX later | ✅ Resolved |
| 14 | Technical / platform | Windows toolchain installation | silent install / manual run / ask upstream for zip | User downloads and runs the upstream Windows installer (or extracts the compiler) and passes `--toolchain`; Clorraine does not run it. | ✅ Resolved — superseded by AR-26 |
| 15 | Security / distribution | Code signing and package-manager manifests | sign before release / unsigned first / package managers first | Unsigned archives and script installs first; browser-download warnings accepted; signing and package managers later | ✅ Resolved |
| 16 | Scope / migration | Removal of the old Python installer and support pipeline | remove now / remove later / keep | Remove `install.pyz`, the support/support-lock pipeline and the release workflows after the new Linux path is proven; no dead paths left | ✅ Resolved |
| 17 | Non-functional / acceptance | macOS and Windows platform proof | CI only / real-machine evidence | Build on hosted runners, but require dated real-machine FS-UAE evidence before claiming macOS or Windows support | ✅ Resolved |
| 18 | Technical / dependency | Bundled `vamos` for terminal Amiga runs | keep bundled wheels / optional PATH only / remove | Do not ship or build `vamos`; use it only if the user already has it on `PATH`, otherwise FS-UAE is the run path | ✅ Resolved |
| 19 | Technical / data | Configuration and update semantics | config file + re-run installer / stores in each tool / no config | One text `config` in the Clorraine home records toolchain/FS-UAE/ROM; re-running install updates the CLI and reuses the shared toolchain; uninstall deletes the folder and PATH entry | ✅ Resolved |
| 20 | Security / compliance | Toolchain/NDK licence handling | mirror assets / download from upstream / build ourselves | User downloads the upstream toolchain; Clorraine never mirrors or redistributes it and shows the upstream terms; `--toolchain` records the user's copy. | ✅ Resolved |
| 21 | Integration / data | Editor discovery and configuration source | extension bundles paths / reads CLI config / duplicate settings | Extension reads the installed CLI through the same config/discovery as the terminal; no conflicting editor-only toolchain settings | ✅ Resolved |
| 22 | Scope / release | Release automation | self-hosted runner / GitHub-hosted matrix / manual only | One GitHub-hosted Linux build for v1.2.0's CLI archive/VSIX; follow-up releases add macOS/Windows. No Docker or self-hosted runner in the user or producer path. | ✅ Resolved |
| 23 | Security | Download/archive validation | hash only / hash + size + safe extraction / trust TLS | Exact SHA-256 and size from a compiled-in pin; HTTPS with redirect checks; safe extraction reject absolute/escaping paths; no shell evaluation | ✅ Resolved |
| 24 | Naming / scope | Plan slug and command names | `cross-platform-cli-fs-uae` / another slug | Plan folder `plans/cross-platform-cli-fs-uae/`; public commands remain `build`, `run`, `emit`, `setup` | ✅ Resolved |
| 25 | Scope / out-of-scope | Explicit exclusions | — | No Docker requirement, no own toolchain build, no emulator/ROM bundling, no WSL, no ARM/Intel Mac, no Marketplace/signing in the first release, no old support pipeline | ✅ Resolved |
| 26 | Technical / dependency | How the upstream toolchain reaches the user | setup downloads/installs it / user downloads it and setup validates it | User downloads the upstream archive for their OS and passes `--toolchain PATH` (or a detected path); setup validates it and builds a smoke program. Clorraine does not download, extract or install the toolchain. | ✅ Resolved |
| 27 | Scope / release | What v1.2.0 contains | all three platforms / Linux only now | v1.2.0 is Linux x64 only. macOS arm64 and Windows x64 are follow-up releases after their real-machine evidence; their archives are not attached to v1.2.0. | ✅ Resolved |
| 28 | Security / UX | How the install scripts are delivered | pipe to shell / download, verify, run | Publish `install.sh`/`install.ps1` and their SHA-256; docs download, verify and run them. No remote text is piped into a shell. | ✅ Resolved |
| 29 | Behavioral / acceptance | AROS fallback proof | guest proof / launch-only | AROS is a launch-only fallback with a warning. Guest-success acceptance requires a user Kickstart ROM and an observable program result. | ✅ Resolved |
| 30 | Data & state | CLI update activation | in-place / side-by-side | Install each CLI version under `versions/<version>`, write a completion marker, then switch `bin` with copy/rename. Keep the previous version until the new one passes. | ✅ Resolved |
| 31 | Technical / security | Hash verification backend for the CLI bootstrap | in-tree SHA-256 / system hasher | Bootstrap uses the platform hash tool: `sha256sum` (Linux), `shasum -a 256` (macOS), `Get-FileHash` (Windows), invoked literally. The toolchain is not downloaded by us (AR-26). | ✅ Resolved |
| 32 | Compatibility / claim | WSL and minimum OS versions | detect/reject WSL / treat WSL as untested | WSL is unsupported and untested; no heuristic rejection. Documented minimums: Ubuntu 22.04/Debian 12, macOS 14 arm64, Windows 10 1809+ x64. | ✅ Resolved |
| 33 | Technical / acceptance | How draft/offline candidate bytes are installed without exposing a production URL override | hidden test env var / local archive option / public prerelease | `install.sh --archive PATH --checksums PATH` installs exact local candidate bytes; it is a documented offline option, not a URL/hash bypass. | ✅ Resolved |

## Resolution Notes

**AR-1–17, 21–25:** Confirmed through the user's 2026-11-23 answers and the
earlier Linux-only/cross-platform discussion.

**AR-18:** Confirmed: no bundled `vamos`; optional PATH use only.

**AR-19:** Confirmed: text config plus re-run-installer update model.

**AR-20:** Confirmed: upstream download only; no mirror or redistribution.

**AR-26–32:** Added after the 2026-11-23 preflight audit. The user chose the
smallest viable path: manual upstream toolchain, Linux-only v1.2.0,
download/verify/run scripts, launch-only AROS, side-by-side updates, system hash
tools and no WSL detection.

**AR-33:** Added for candidate/offline acceptance: `install.sh --archive` uses
exact local bytes and still verifies the supplied checksums; it is not a
network URL or hash bypass.
