## Ambiguity Register: Windows CLI and installer

> **Status**: ✅ GATE PASSED — all 53 items resolved; AR-54 – AR-63 added and resolved during execution
> **Last Updated**: 2026-11-26 18:30
> **Evidence**: `spikes/08-windows-cli/README.md`

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Technical | How does the installer obtain the rc24 toolchain, given the upstream Windows asset is a per-machine Inno Setup `.exe`? | Run the pinned setup silently behind one UAC prompt / detect only / ask upstream for a zip | Run the pinned upstream setup silently: download, verify size and SHA-256, run `/VERYSILENT … /DIR=…` behind one UAC prompt; an existing root is also accepted through `--toolchain` | ✅ Resolved |
| 2 | Technical | Which host C compiler serves the host profile and the PC hello on Windows? | GCC/Clang on `PATH` / installer offers LLVM-MinGW / also support MSVC `cl` | GCC/Clang-compatible compiler on `PATH` (`cc`, `gcc`, `clang`) or `CLORRAINE_CC`; the PC hello is skipped when none is found | ✅ Resolved |
| 3 | Scope | Which release carries Windows support? | Decide at release time / v1.2.x / v1.3.0 | Decide at release time; plan against the next version, coordinated with the parallel enhancement work | ✅ Resolved |
| 4 | Scope | Where do the spike and acceptance run? | The user's real Windows machine / CI only | The user's real Windows 11 x64 machine, with FS-UAE and the user's KS3.1 ROM; driven remotely over SSH | ✅ Resolved |
| 5 | Scope | Where does development happen? | Linux worktree / move development to Windows | Linux, in the `windows-cli` worktree branch, merged into `main` through a GitHub pull request; no Windows build environment | ✅ Resolved |
| 6 | UX & presentation | How do the docs handle the PowerShell execution policy? | Document a one-time process-scope bypass / document a permanent policy change | Document the process-scope bypass (`powershell -ExecutionPolicy Bypass -File …` or `Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass`), the Group Policy lock, the rc24 UAC prompt, and `Unblock-File` for browser downloads; never a permanent policy change | ✅ Resolved |
| 7 | Scope | What form does the Windows installer take? | `install.ps1` download-then-run / MSI or EXE installer | User accepted recommendation: install.ps1: a PowerShell script with the same download-then-run model and steps as install.sh; no installer framework | ✅ Resolved |
| 8 | Naming | Plan folder and feature name | `plans/windows-cli/` / other | User accepted recommendation: `plans/windows-cli/` (matches the branch name) | ✅ Resolved |
| 9 | Scope | Which Windows versions are supported? | Windows 11 x64 only / Windows 10 22H2 and 11 x64 | User accepted recommendation: Windows 11 x64 only; the docs state Windows 10 is untested | ✅ Resolved |
| 10 | Data & state | Where do the CLI, config, and toolchain live? | Proposed layout / other | User accepted recommendation: CLI in `%LOCALAPPDATA%\Programs\Clorraine\versions\<version>-<sha12>` with a `current` junction and its `bin` on the user PATH; config `%APPDATA%\clorraine\config`; rc24 in `C:\Program Files\Clorraine\amiga-gcc-16.2-rc24` | ✅ Resolved |
| 11 | Edge cases | What happens when an `amiga-gcc` registration already exists (one upstream AppId per machine)? | Reuse rc24, stop for other versions / always replace / always install beside | User accepted recommendation: Reuse and validate an existing rc24 registration; never replace another version without an explicit yes or flag; a non-interactive run stops with a clear message | ✅ Resolved |
| 12 | Technical | How is the active version switched, given a junction cannot be replaced atomically? | Junction with delete-then-rename and repair / per-user `PATH` points at the version directory | User accepted recommendation: Junction with delete-then-rename; the next installer run finishes an interrupted switch; PATH and the editor keep one stable path | ✅ Resolved |
| 13 | Scope | Does the installer offer FS-UAE? | Offer the pinned official zip with consent / detect only (Linux parity) | User accepted recommendation: With consent, download the pinned official FS-UAE 3.2.35 zip, verify size and SHA-256, unzip per-user; an existing FS-UAE is detected first | ✅ Resolved |
| 14 | Technical | How does the driver read the FS-UAE version on Windows, where `--version` prints nothing to a pipe? | PE version resource / `Version.txt` beside the executable | User accepted recommendation: Read the PE version resource of `fs-uae.exe` (small std-only parser) | ✅ Resolved |
| 15 | Scope | Clang as the host compiler fails debug builds under `-Werror` | GCC-only on Windows for now, Clang follow-up / make the emitted C Clang-clean now / relax Clang flags | User accepted recommendation: GCC first: the docs recommend GCC (w64devkit or MSYS2); Clang debug builds are a known limitation with a separate follow-up issue | ✅ Resolved |
| 16 | Scope | The unescaped `#line` path defect affects every host | Fix it in this plan / separate plan | User accepted recommendation: Fix in this plan: escape the `#line` path as a C string literal; emission-integrity review | ✅ Resolved |
| 17 | Behavioral | What do uninstall and purge remove on Windows? | Proposed behaviour / other | User accepted recommendation: `-Uninstall` removes the active version link, the shims, and the PATH entry; `-Purge` also removes all versions and the config; rc24 and FS-UAE are never removed by Clorraine, and the installer prints how to remove them | ✅ Resolved |
| 18 | Security & compliance | Are the Windows binaries code-signed? | Unsigned first release, watch Defender / sign now | User accepted recommendation: Unsigned first release; watch Defender/SmartScreen during testing; revisit signing if they react | ✅ Resolved |
| 19 | Behavioral | Running the CLI from an SSH logon cannot start the rc24 compiler | Document as unsupported / investigate and support | User accepted recommendation: Document as unsupported; tests use the desktop-session runner | ✅ Resolved |
| 20 | Technical (complexity escalation) | Cross-building the Windows binaries needs MinGW-w64 and the Rust Windows target in the pinned SDK image | Smaller: unpinned throwaway image at release time / Larger: pinned image change + `make dist` Windows build + `cargo check` step in `make check` | Approved larger (see AR-20 evidence) | ✅ Resolved |
| 21 | Technical (complexity escalation) | Where do the Windows installer and driver tests run? | Larger: `make windows-check WINHOST=…` gate and/or GitHub Actions `windows-latest` job / Simplified: Linux-tested plain functions, plain PowerShell spec script, Windows evidence via `make acceptance-check`, non-gating remote helper | Revised (simplified design, items 1–4) (see AR-21 evidence) | ✅ Resolved |
| 22 | Scope | Is a standard (non-administrator) account supported? | Supported when admin credentials are available / administrators only | User accepted recommendation: Supported with admin approval: Clorraine installs per-user; only the rc24 step needs an administrator to approve UAC, or an existing toolchain is passed with `--toolchain`; tested on a standard account | ✅ Resolved |
| 23 | Scope | Freestanding profile on Windows | Fix the syscall selection and support it / mark unsupported | User accepted recommendation: Fix and support: the raw syscall layer becomes Linux-only so Windows uses the C-runtime fallback; tested with GCC | ✅ Resolved |
| 24 | Feature gaps | install.ps1 option surface and spelling (install.sh has 17 options; AR-1 and AR-17 mixed `--x` and `-X` spellings) | See resolution notes | User accepted recommendation: PowerShell parameters mapped one-to-one: `-Prefix`, `-ModifyPath`, `-Archive`/`-Checksums`, `-Toolchain`, `-NoToolchain`, `-FsUae`, `-Kickstart`, `-NoEmulator`, `-NoHello`, `-Starter`/`-NoStarter`, `-VSCode`/`-NoVSCode`, `-Uninstall`/`-Purge`; the same conflict rules; a parity table in the plan | ✅ Resolved |
| 25 | Naming & terminology | AR-17 removes "the shims", but AR-10 puts `current\bin` on PATH directly: do shims exist? | See resolution notes | User accepted recommendation: No shims: one PATH entry `…\Clorraine\current\bin` with the real `.exe` files; AR-17's "shims" means that PATH entry | ✅ Resolved |
| 26 | Security & compliance | How the user PATH is written and removed, and whether it needs consent | See resolution notes | User accepted recommendation: Read `HKCU\Environment` `Path` unexpanded, keep `REG_EXPAND_SZ`, add or remove one normalized exact entry, broadcast `WM_SETTINGCHANGE`, never `setx`; asked interactively (default yes), non-interactive only with `-ModifyPath` | ✅ Resolved |
| 27 | Feature gaps | How a non-interactive run consents to the rc24 (UAC) and FS-UAE downloads | See resolution notes | User accepted recommendation: Explicit opt-in flags `-InstallToolchain` and `-InstallFsUae`; a non-interactive run without them never downloads | ✅ Resolved |
| 28 | Feature gaps | Where install.ps1 looks for FS-UAE and Kickstart ROMs, and where the FS-UAE zip is unpacked | See resolution notes | User accepted recommendation: FS-UAE: `-FsUae`, env, config, PATH, `%LOCALAPPDATA%\Programs\Clorraine\fs-uae\3.2.35` (download target), `Program Files\FS-UAE`; ROM: `-Kickstart`, env, config, `~\amiga\roms`, `Documents\FS-UAE\Kickstarts`, the Amiga Forever shared rom folder; FS-UAE version from the PE resource | ✅ Resolved |
| 29 | Technical unknowns | Driver resolution order for the host C compiler (AR-2), its reach beyond Windows, and its messages | See resolution notes | User accepted recommendation: `CLORRAINE_CC` on every host; then Linux keeps `cc` (unchanged) and Windows tries `gcc`, `cc`, `clang`; not stored in the config; the Windows hint names w64devkit/MSYS2 GCC and `CLORRAINE_CC`; install.ps1 uses the same order | ✅ Resolved |
| 30 | Scope ambiguities | install.ps1 behaviour on Windows 10, ARM64, 32-bit, and PowerShell 7 | See resolution notes | User accepted recommendation: Refuse non-x64 (native architecture); Windows 10 prints "untested" and continues; written for PowerShell 5.1, also runs under PowerShell 7 (documented as untested) | ✅ Resolved |
| 31 | UX & presentation | install.ps1 source encoding and console glyph/colour fallback under Windows PowerShell 5.1 | See resolution notes | User accepted recommendation: ASCII-only source; glyphs from `[char]` codes, shown only on a VT-capable UTF-8 console; `NO_COLOR` honoured; plain `[ok]`/`[!!]` otherwise; `package.sh` asserts ASCII | ✅ Resolved |
| 32 | Integration points | `clorraine new` launcher lookup, `tasks.json`, and next-step text on Windows | See resolution notes | User accepted recommendation: `new.rs` uses the same `.exe` PATH search; `tasks.json` gets the absolute `…\Clorraine\current\bin\clorraine.exe`; Windows next-step text (no `~/.profile`; open a new terminal, restart VS Code) | ✅ Resolved |
| 33 | Naming & terminology | Windows asset names, zip layout, SHA256SUMS, stage version check, documented download/verify commands | See resolution notes | User accepted recommendation: `clorraine-<v>-x86_64-windows.zip` with top folder `clorraine-<v>-x86_64-windows`; `install.ps1` beside `install.sh`; one `SHA256SUMS` over all five assets; zip built with Python `zipfile`; the stage version check reads the version embedded in the cross-built exe; docs use `curl.exe -o` plus `Get-FileHash` | ✅ Resolved |
| 34 | Security & compliance | Where the rc24 setup is verified and stored before the elevated run (check-to-use race across the privilege boundary) | See resolution notes | User accepted recommendation: The elevated child copies `setup.exe` into a fresh admin-only folder (`C:\Program Files\Clorraine\.staging-<random>`), checks size and SHA-256 there, runs it with `/LOG` in that folder, then deletes it; a mismatch never runs | ✅ Resolved |
| 35 | Security & compliance | Junction-safe deletion for uninstall, purge, and the version switch | See resolution notes | User accepted recommendation: Remove a junction without recursion; `current` is ours only when its target resolves inside `…\Clorraine\versions`; tree deletion never follows reparse points; never `Remove-Item -Recurse` on such trees | ✅ Resolved |
| 36 | Security & compliance | Downloader and transport rules (HTTPS-only redirects, size cap, timeouts) under PowerShell 5.1 | See resolution notes | User accepted recommendation: `curl.exe` with the Linux rules: HTTPS-only including redirects, size cap, connect and total timeouts; a size or SHA-256 mismatch deletes the file and stops | ✅ Resolved |
| 37 | Stakeholder conflicts | Isolation of the shared SDK image and shared files from the parallel session until merge | See resolution notes | User accepted recommendation: The worktree builds `IMAGE=clorraine-sdk:windows-cli`; `clorraine-sdk:dev` is rebuilt once after merge; roadmap and AGENTS.md edits are made at merge time | ✅ Resolved |
| 38 | Integration points | Canonical path spelling between the language server, its URIs, and the extension's source-path checks on Windows; UNC paths | See resolution notes | User accepted recommendation: Server: no `\\?\` prefix, uppercase drive, backslashes, for document keys, index keys, and `sourceFiles` results; URIs out as `file:///c%3A/…`; the extension compares paths case-insensitively on Windows; UNC/network-share workspaces documented as unsupported | ✅ Resolved |
| 39 | Behavioral gaps | Interrupting `run --emulator` on Windows leaves the GUI FS-UAE window and the private run behind | See resolution notes | User accepted recommendation: Known limitation: close the FS-UAE window; the docs name the leftover `%TEMP%\clorraine-emulator-*` folder; acceptance records the actual behaviour | ✅ Resolved |
| 40 | Edge cases | Installer lock, interruption rollback, and a declined UAC prompt | See resolution notes | User accepted recommendation: Named mutex `Local\Clorraine-Install`; `try`/`finally` rollback mirroring Linux; a declined UAC prompt keeps the installed CLI, prints the `-Toolchain` next step, and exits 1 | ✅ Resolved |
| 41 | Edge cases | `-Purge` while `clorraine-lsp.exe` or a build runs from the install tree | See resolution notes | User accepted recommendation: Before deleting anything, detect running `clorraine*.exe` from the install tree and stop with "close VS Code and running builds" | ✅ Resolved |
| 42 | Edge cases | Supported user-profile and project path shapes (spaces, non-ASCII, OneDrive, >260 characters, commas in TEMP) | See resolution notes | User accepted recommendation: The standard-account test user has a space in its name; a project path with spaces and non-ASCII is in the acceptance set; OneDrive folders work when they are ordinary paths; >260-character paths and commas in TEMP are documented as unsupported | ✅ Resolved |
| 43 | Data & state | Config precedence on Windows when MSYS2/Git Bash export HOME | See resolution notes | User accepted recommendation: Always `%APPDATA%` on Windows; `HOME` and `XDG_CONFIG_HOME` are ignored there; acceptance isolates by overriding `APPDATA` | ✅ Resolved |
| 44 | Technical unknowns | Freestanding on Windows also needs C shim changes (x86_64 syscall branch, 32-bit `uintptr_t` on LLP64) | See resolution notes | User accepted recommendation: Keep AR-23 with minimal guards: the syscall branch requires `__linux__`, and `uintptr_t` is 64-bit only on `_WIN64`; compiled output on existing targets is unchanged, the emitted C text changes (pins updated, emission-integrity review) | ✅ Resolved |
| 45 | Naming & terminology | Host artifact names on Windows (`<stem>` vs `<stem>.exe`) | See resolution notes | User accepted recommendation: `<stem>.exe` and `<stem>.host.exe` on Windows | ✅ Resolved |
| 46 | Data & state | The `platform: windows` acceptance evidence schema and its Windows producer | See resolution notes | User accepted recommendation: `acceptance.py` gains a shared core with per-platform pins (Windows: `os_build` instead of `glibc`, the rc24 setup pin, the FS-UAE zip pin, `install.ps1`, `.zip`; version from the candidate); `tools/setup/acceptance.ps1` runs on Windows (the fs-uae-check set, host hello with GCC, a standard-account install, manual UAC and editor confirmations) and writes the JSON that `make acceptance-check` validates | ✅ Resolved |
| 47 | Technical | Which commands verify each phase and the plan? | Proposed commands / different | User accepted recommendation: phase gate on Linux `make check IMAGE=clorraine-sdk:windows-cli` (AR-37); on Windows `tools/setup/tests/install.spec.test.ps1` through the remote helper; final gate `make verify` on Linux plus the Windows acceptance record validated by `make acceptance-check` | ✅ Resolved |
| 48 | Technical | AR-21 named one PowerShell test script, but the spec/impl split needs two | Two plain scripts / one script with two sections | User accepted recommendation: `install.spec.test.ps1` (from the ST cases, before the installer) and `install.impl.test.ps1` (internals and edge cases); same no-Pester runner; both run on Windows with one documented command | ✅ Resolved |
| 49 | Scope | Who creates the standard test account (AR-22, AR-42)? | Agent over SSH / user | User accepted recommendation: the agent creates local standard user `Clorraine Test` over SSH with a random password given to the user; the user signs in once and approves UAC with the admin account; removed after acceptance | ✅ Resolved |
| 50 | Scope | The spike's machine-wide rc24 registration before acceptance | Keep it to exercise AR-11, then uninstall / remove first | User accepted recommendation: keep it to exercise the existing-registration path once, then uninstall it with its own uninstaller and run the clean-install acceptance; `clorraine-spike` is deleted at the end | ✅ Resolved |
| 51 | Behavioral | install.ps1's PC hello when the only host compiler is Clang | Skip with a GCC note / run with `--release` | User accepted recommendation: skip with `Clang found; the PC hello needs GCC (w64devkit or MSYS2), or set CLORRAINE_CC`; the step counts as skipped, not failed | ✅ Resolved |
| 52 | Behavioral | `-Purge` and the FS-UAE copy install.ps1 downloaded into the Clorraine folder | Purge removes our own copy / never remove | User accepted recommendation: purge removes only the FS-UAE that install.ps1 downloaded into `…\Clorraine\fs-uae\3.2.35`; any other FS-UAE and rc24 are never removed (refines AR-17) | ✅ Resolved |
| 53 | Naming & terminology | Flag for replacing another amiga-gcc registration (AR-11) | `-ReplaceToolchain` / interactive only | User accepted recommendation: `-ReplaceToolchain`; interactive runs ask (default No) | ✅ Resolved |
| 54 | Technical (runtime) | The full image build cannot run: the toolchain stage clones vbcc's `master` branch, which upstream no longer has, and the build cache that AR-20 assumed is gone | Layer the committed Windows lines on the existing `clorraine-sdk:dev` / fix the toolchain stage now / add a Makefile target for the layered build | Layer the committed lines on `clorraine-sdk:dev` (see AR-54 resolution) | ✅ Resolved |
| 55 | Technical (runtime) | The driver modules are private, so the integration spec suites cannot call `native_syscalls_for`, `validate_for`, `fs_uae_version_for`, or `launcher_for` as 03-01/03-02 name them | Re-export exactly those items from `clc::clorraine` and make `platform` public / make the whole modules public / move the spec cases into unit tests | Re-export the named items from `clc::clorraine`; `pub mod platform` (see AR-55 resolution) | ✅ Resolved |
| 56 | Technical (runtime) | The freestanding shim's `offsetof` casts a pointer to `unsigned long`, which MinGW rejects under `-Werror` on Win64 (32-bit `long`), so freestanding programs using `offsetof` cannot build on Windows (W13) | Cast through the shim's `uintptr_t` / a second `_WIN64` branch for the macro / leave it as a limitation | Cast through `uintptr_t` (see AR-56 resolution) | ✅ Resolved |
| 57 | Technical (runtime) | A `#line` path containing `??` forms a C trigraph under `-std=c99` and fails the strict build (pre-existing; found by the Phase 2 review), which W12's "valid C string literal" excludes | Escape every `?` that follows a `?` as `\?` / leave it (Windows forbids `?` in file names) | Escape every `?` that follows a `?` (see AR-57 resolution) | ✅ Resolved |
| 58 | Technical (runtime) | The 64 MiB read limit for the FS-UAE executable (03-02, ST-23) rejects the pinned FS-UAE 3.2.35 `fs-uae.exe`, which is 69,273,425 bytes, so W11's PE version read fails on the real host | Raise the limit to 256 MiB / stream-scan the file with no full read / read only the resource section | Raise the limit to 256 MiB (see AR-58 resolution); 03-02 and ST-23 updated | ✅ Resolved |
| 59 | Technical (runtime) | 03-04 names four test seams (`Invoke-Download`, `Invoke-ElevatedSetup`, `Get-Rc24Registration`, the PATH key), but ST-40, ST-46, ST-47, ST-58, ST-61, and ST-63 also stub the host architecture, OS build, interactivity, console capability, the extracted version check, and the compiler probe, and every case needs a callable entry point | Name the extra seams and an `Invoke-Installer` entry point that returns the exit code / run every case as a separate script with environment overrides | Named seams and `Invoke-Installer` (see AR-59 resolution) | ✅ Resolved |
| 60 | Technical (runtime) | The elevated step's mismatch exit 3 (03-04, ST-54) is also Inno Setup's own "fatal error preparing" code, so a tampered setup cannot be told apart from a setup failure (Phase 6 security review SA-004) | Own codes outside Inno's 0-8 range / keep 3 and word the message for both | Codes 101 mismatch, 102 staging name taken, 103 untrusted admin folder, 104 other error (see AR-60 resolution); 03-04 and ST-54 updated | ✅ Resolved |
| 61 | Technical (runtime) | Which existing `Program Files\Clorraine` the elevated step trusts: an owner check alone passes a folder other accounts may change, and the reviewer proposed also trusting the elevated user's own SID (Phase 6 security re-review SA-006) | Owner and access-list check, fail closed with a repair hint / also trust the current user SID | Owner must be Administrators, SYSTEM, or TrustedInstaller; no allow rule with write-type rights for any other account; the current user is not trusted; 103 names the `icacls /setowner` repair (see AR-61 resolution) | ✅ Resolved |
| 62 | Technical (runtime) | The Windows evidence record's exact JSON shape and how the producer obtains the rc24 and FS-UAE pins, which 03-05 names but does not spell out | Fixed shape below; pins from the installer text after the registration/config confirm use / re-download both files to hash them | Shape and pin source per the AR-62 resolution | ✅ Resolved |
| 63 | Technical (runtime) | The Windows docs need an install URL, but v1.2.0 is published with Linux assets only and AR-3 leaves the carrying release to release time; GitHub's `latest` never resolves to a prerelease (Phase 8 docs review DR-801) | Versioned URLs filled in at release / `latest` URLs with a full-release requirement / v1.2.0 URLs | User ruling 2026-11-26: versioned at release; the docs write `vX.Y.Z` until then (see AR-63 resolution) | ✅ Resolved |

## Coverage Map

> **Coverage**: 4/4 clusters · 12/12 categories · 30 candidates · 4 residual limits · Effort: high/xhigh→inherited · Independence: mixed-effort
> **Freshness**: Coverage as of 2026-11-26; artifacts 00-ambiguity-register.md (AR-1–AR-23), spikes/08-windows-cli/README.md, driver/LSP/extension/installer sources

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | Installer parity, starter, messages, assets | install.sh, package.sh, new.rs, starter templates, environment.rs, setup.rs, toolchain.rs, docs/linux-cli.md, README/site greps | acceptance internals, emulator, LSP (other clusters) | 8 → AR-24, 25, 27, 28, 29, 30, 31, 32, 33 | general-agent fallback for the auditor role | high |
| C2 Behavior and edges | Emulator lifecycle, LSP/extension round trip, installer state | emulator/*, emulator_runner.c, setup.rs, new.rs, artifacts.rs, lsp/*, tools/vscode/src/*, install.sh | no Windows runtime checks; client.ts/extension.ts partly | 8 → AR-26, 34, 38, 39, 40, 41, 42, 43 | general-agent fallback | medium-high |
| C3 Technical and data | cfg sites, config, artifacts, packaging, evidence | every cfg/target site, shim_freestanding.c, codegen casts, lsp keys, package.sh, acceptance.{sh,py}, Dockerfile | observe.rs, lsp/server.rs, tests, site | 8 → AR-25, 26, 29, 33, 38, 43, 44, 45, 46 | general-agent fallback | high |
| C4 Risk and stakeholders | Privilege boundary, deletion, PATH, transport, shared state | install.sh, package.sh, acceptance.{sh,py}, select.rs, storage.rs, docker/Makefile, worktrees, roadmap policy | no Windows runtime checks | 6 → AR-25, 26, 34, 35, 36, 37 | refuted: reproducible zip, timeout, licensing (upstream-fetch policy), signing (AR-18) | high |

**Source note:** C1 CV-001→AR-24/25, CV-002→AR-27, CV-003→AR-28, CV-004→AR-29, CV-005→AR-30, CV-006→AR-31, CV-007→AR-32, CV-008→AR-33; C2 CV-001→AR-38, CV-002→AR-39, CV-003→AR-43, CV-004→AR-26, CV-005→AR-40, CV-006→AR-41, CV-007→AR-34, CV-008→AR-42; C3 CV-001→AR-29, CV-002→AR-43, CV-003→AR-44, CV-004→AR-45, CV-005→AR-25/26, CV-006→AR-38, CV-007→AR-33/31, CV-008→AR-46; C4 CV-001→AR-34, CV-002→AR-35, CV-003→AR-26, CV-004→AR-36, CV-005→AR-25, CV-006→AR-37. Residual limits: no auditor ran anything on Windows; claims about PowerShell 5.1, .NET PATH writes, Node path spelling, and toolchain code pages are unverified until execution.

### Resolution Notes

**AR-1 – AR-6:** decided by the user during the Windows planning discussion on 2026-11-26, before
the spike. The spike confirmed the AR-1 mechanism: silent install in 44 s, exit 0, no restart,
per-machine registration under `HKLM`.

**AR-7 – AR-19, AR-22, AR-23:** the user accepted each recommendation individually in one
batch on 2026-11-26, and confirmed the scope: IN — driver Windows support, the `#line` fix, LSP
URIs and editor discovery, `install.ps1`, packaging, docs/site/README, Windows acceptance; OUT —
publication, MSVC, Windows on ARM, WSL.

**AR-20 (complexity escalation evidence):**

```text
Original goal: Ship a Windows installer built entirely on Linux.
Extra system or support code: gcc-mingw-w64-x86-64 and the Rust x86_64-pc-windows-gnu target in
  the pinned SDK image (docker/Dockerfile); `make dist` cross-builds the Windows zip; `make check`
  gains `cargo check --target x86_64-pc-windows-gnu`.
Why it may be needed: release bytes must come from the pinned image so acceptance evidence binds
  their hashes (Makefile:84, tools/setup/acceptance.py:97-126); without the check step no gate
  compiles the Windows-only code.
Evidence: the spike's throwaway image built working binaries in about a minute; they import only
  Windows system DLLs (spikes/08-windows-cli/README.md).
Smallest solution that still works: an unpinned throwaway image at release time.
Extra cost: about three Dockerfile lines appended after the existing layers; a larger image;
  about one more minute of `make dist` and seconds of `make check`; the shared clorraine-sdk:dev
  image is rebuilt once, when the parallel session is idle.
Independent verdict: Justified — an unpinned build cannot meet the bar the Linux zip meets, and the
  cfg(windows) code needs a compiling gate; noted separately that the image's Rust toolchain is
  unpinned (pre-existing, out of scope).
Direct user decision: approved larger
```

**AR-21 (complexity escalation evidence):**

```text
Original goal: Test the Windows driver and installer to the project's quality bar.
Extra system or support code: a `make windows-check WINHOST=…` gate and/or a GitHub Actions
  windows-latest job.
Why it may be needed: Windows behaviour must be exercised on Windows.
Evidence: hosted Windows runners have no Kickstart ROM and no signed-in desktop session, which
  cc1.exe and FS-UAE need; most integration tests use /bin/sh fakes; a WINHOST gate would be the
  first Makefile gate tied to one machine.
Smallest solution that still works: (1) Windows-specific logic as plain functions tested by the
  Linux `cargo test`; (2) one plain PowerShell 5.1 script tools/setup/tests/install.spec.test.ps1
  (no Pester) run on Windows with one documented command; (3) acceptance evidence extended to
  `platform: windows` with the zip and install.ps1 hashes, validated by `make acceptance-check` on
  Linux; (4) a non-gating helper that copies a candidate to a Windows host and runs commands in the
  desktop session.
Extra cost: one PowerShell test file, an evidence-format extension, one helper script; no CI job
  and no Makefile gate.
Independent verdict: Simplify — the CI job or WINHOST gate adds maintenance without more assurance
  than a manual Windows run whose evidence is hash-checked; `make acceptance-check` already
  implements that pattern.
Direct user decision: revised — the simplified design, items 1–4
```

**AR-24 – AR-28, AR-30, AR-31, AR-34 – AR-36, AR-40, AR-41:** coverage-hunt batch 1; the user
accepted each recommendation individually on 2026-11-26.

**AR-29, AR-32, AR-33, AR-37 – AR-39, AR-42 – AR-46:** coverage-hunt batch 2; the user accepted
each recommendation individually on 2026-11-26. AR-44 keeps AR-23 and adds the shim guards the
hunt found necessary.

**Gate:** the user confirmed the complete register (AR-1 – AR-46) and the verify commands (AR-47)
on 2026-11-26.

**AR-48 – AR-50:** surfaced while authoring the plan; the user accepted each recommendation on
2026-11-26.

**AR-51 – AR-53:** surfaced while writing the installer specification; the user accepted each
recommendation on 2026-11-26.

**AR-54 (runtime, auto-design):**

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (how the branch image is produced); stays inside AR-20's
  approved change and changes no product behaviour, scope, or the committed Dockerfile content
Objective: a `clorraine-sdk:windows-cli` image whose content is the committed docker/Dockerfile
Decision: build the branch image as `FROM clorraine-sdk:dev` plus the committed Windows lines,
  taken verbatim from docker/Dockerfile; the committed Dockerfile keeps the appended layer
Evidence: `make -C docker image` fails at the toolchain stage ("Remote branch master not found",
  vbcc clone, amiga-gcc Makefile:1315) on main as well; docker/Dockerfile, versions.env and
  smoke-test.sh are unchanged since 2026-09-14 and clorraine-sdk:dev was built from them on
  2026-10-08, so the layered image equals the full build of the committed file
Rejected alternatives: fixing the toolchain stage now (outside this plan, changes the shared
  toolchain build and every Amiga gate's compiler); a new Makefile target for the layered build
  (new machinery for a temporary upstream breakage)
Strongest counterargument: the branch image is not proven buildable from scratch; a later
  from-source rebuild could differ (it already could, since rustup and amitools are unpinned)
Confidence: High — would change if docker/Dockerfile's earlier layers change on main before merge
Hardening: no challenger; low-impact, reversible local build with no committed machinery
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: the toolchain stage builds again; main changes the Dockerfile before merge
```

The post-merge rebuild of `clorraine-sdk:dev` (AR-37) uses the same layering until the upstream
vbcc breakage is fixed; the merge notes and a follow-up issue draft record it.

**AR-55 (runtime, auto-design):**

```text
Authority: AI — delegated by --auto-design
Eligibility: internal interface and testing mechanism; the functions and their contracts are
  already specified, only their reachable path is chosen
Objective: the spec suite in compiler/tests calls the specified functions directly
Decision: `pub mod platform` in compiler/src/clorraine/mod.rs (03-02 already makes its items pub),
  plus `pub use` re-exports there of `toolchain::native_syscalls_for`, `setup::{validate_for,
  Toolchain, SetupError, Validation}`, `emulator::select::fs_uae_version_for`, and
  `new::launcher_for`; tests call them as `clc::clorraine::<name>`
Evidence: compiler/src/clorraine/mod.rs:14-24 declares every driver module private; lsp::uri is
  already public; no compiler/tests suite reaches a driver internal today; AGENTS.md puts spec
  suites in compiler/tests
Rejected alternatives: making setup/toolchain/new/emulator public (exposes every pub item and
  silences dead-code warnings for them); unit tests inside src (contradicts the plan's spec file
  and the project's spec-suite location)
Strongest counterargument: the library's public surface grows by a few test-motivated names
Confidence: High — would change if the project forbids test-only exports
Hardening: no challenger; internal crate, reversible
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a reviewer finds the facade exposes behaviour beyond the named functions
```

**AR-56 (runtime, auto-design):**

```text
Authority: AI — delegated by --auto-design
Eligibility: necessary correction inside AR-23/AR-44 (freestanding works on Windows with minimal
  shim guards); compiler/runtime mechanism only
Objective: freestanding programs build under -Werror with MinGW on Win64; Linux and m68k compiled
  output unchanged
Decision: `#define offsetof(type, member) ((uintptr_t)&(((type *)0)->member))`
Evidence: x86_64-w64-mingw32-gcc -std=c99 -ffreestanding -fno-builtin -Wall -Wextra -Werror on an
  emitted program with `offsetof(Pair, b)` fails with -Werror=pointer-to-int-cast at the shim's
  macro; `uintptr_t` is `unsigned long` on every non-_WIN64 target, so those builds are identical
Rejected alternatives: a separate `_WIN64` branch for the macro (duplicates the guard the typedef
  already carries); documenting a limitation (contradicts W13)
Strongest counterargument: one more emitted-C text change on every target
Confidence: High — would change if a target defines `uintptr_t` narrower than a pointer
Hardening: no challenger; covered by the emission-integrity review of this phase
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a target where `uintptr_t` is not `unsigned long` outside _WIN64
```

**AR-57 (runtime, auto-design):**

```text
Authority: AI — delegated by --auto-design
Eligibility: correctness mechanism inside W12/AR-16 (the #line path is a valid C string literal on
  every host); emission detail only
Objective: no source path can change meaning or break the -Werror build through a trigraph
Decision: c_string_body writes every `?` whose previous input character is `?` as `\?`
Evidence: memory-safety W2-M1, correctness PE-1, emission-integrity EI-P01: `tri??=x.clo` and a
  `dq??` directory fail `-Werror=trigraphs` on host cc and m68k GCC, base and head alike
Rejected alternatives: escaping only the second `?` of each pair (a `???=` run still leaves
  `??=`); leaving it (Linux paths may contain `?`, and W12 covers every host)
Strongest counterargument: a pre-existing Linux-only edge case widens this change slightly
Confidence: High — paths without `??` stay byte-identical
Hardening: no challenger; three independent reviewers reproduced the defect
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a toolchain leg that does not accept `\?` in a #line string
```

**AR-58 (runtime, auto-design; reopens the 03-02 read limit and ST-23):**

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation limit (resource bound) inside AR-14; W11 and the PE-version contract
  are unchanged
Objective: the PE version read works on the pinned FS-UAE 3.2.35 executable while memory stays
  bounded against a pathological file
Decision: read at most 256 MiB; a larger file is the same "cannot read the FS-UAE version" error.
  03-02 (select.rs row and error table) and ST-23 change from 64 MiB to 256 MiB
Evidence: on the Windows host, FS-UAE/Windows/x86-64/fs-uae.exe is 69,273,425 bytes; its
  VS_FIXEDFILEINFO is at 0xb1eb08 (4-aligned, structure version 0x00010000, 3.2.35); ST-30 failed
  with "cannot read the FS-UAE version" because of the 64 MiB limit
Rejected alternatives: a streaming scanner (changes the specified pe_file_version(bytes) contract
  for no user-visible gain); reading only the resource section (a PE section-table parser, more
  code for the same result)
Strongest counterargument: every emulator run reads about 70 MB once
Confidence: High — would change if a later FS-UAE executable exceeds 256 MiB
Hardening: no challenger; measured on the real file
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a pinned FS-UAE executable over 256 MiB; a run-time cost users notice
```

**AR-59 (runtime, auto-design):**

```text
Authority: AI — delegated by --auto-design
Eligibility: test seam and internal interface naming inside AR-48's dot-source design
Objective: every PowerShell spec case drives the real installer code with only the host facts it
  names replaced
Decision: install.ps1 defines `Invoke-Installer` (same parameters as the script; returns the exit
  code; the script body is `exit (Invoke-Installer @PSBoundParameters)` unless
  CLORRAINE_INSTALL_LIB=1). Replaceable functions: Get-HostArchitecture (`X64`/`Arm64`/...),
  Get-OsBuild (int), Test-Interactive (bool), Get-ConsoleCapability (VirtualTerminal, CodePage),
  Test-ExtractedVersion (throws on mismatch), Find-HostCompiler (Kind gcc|clang|none, Path),
  Invoke-Download, Invoke-ElevatedSetup, Get-Rc24Registration, Invoke-Clorraine; the PATH key is
  `$script:PathRegistryKey` (default `HKCU:\Environment`). `New-OutputStyle -VirtualTerminal
  -CodePage -NoColor` returns the style table ST-63 checks. Tests run each case in a child
  `powershell -NoProfile` that dot-sources the assembled copy, defines stubs, and calls
  Invoke-Installer.
Evidence: 03-04 §Template and test seam; ST-40/46/47/58/61/63 stub facts outside the four named
  seams; AR-48 requires no Pester
Rejected alternatives: environment-variable overrides for each fact (spreads test hooks through
  the product code); running the shipped script file per case (cannot stub functions)
Strongest counterargument: the installer carries ten small seam functions
Confidence: High
Hardening: no challenger; internal, reversible
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a seam needed by Phase 6 cases is missing
```

**AR-60 (runtime, auto-design; reopens the elevated exit codes in 03-04 and ST-54):**

```text
Authority: AI — delegated by --auto-design
Eligibility: failure-reporting mechanism inside AR-34; the behaviour (a mismatch never runs the
  setup) is unchanged
Objective: the installer tells a tampered or mismatched setup apart from a setup that failed
Decision: the elevated step returns 101 (copy mismatch), 102 (staging name taken), 103 (Program
  Files\Clorraine is a link or not owned by Administrators/SYSTEM/TrustedInstaller), 104 (any
  other error); the setup's own exit code otherwise. ST-54 expects 101 for the mismatch
Evidence: Inno Setup documents exit codes 0-8 (3 = fatal error preparing, 4 = fatal error
  installing); the Phase 6 security review (SA-004)
Rejected alternatives: keeping 3 with a message covering both causes (hides tampering)
Strongest counterargument: a documented spec value changes after the spec tests were written
Confidence: High
Hardening: no challenger; security reviewer recommended the change
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: an upstream setup that uses codes above 100
```

**AR-61 (runtime, auto-design; refines AR-60's 103 within AR-34):**

```text
Authority: AI — delegated by --auto-design
Eligibility: hardening inside the approved AR-34 trust boundary; no user-visible contract
  changes except a clearer 103 message
Objective: the elevated step never runs the setup from, or installs into, a folder another
  account could change
Decision: an existing Program Files\Clorraine must be a real folder owned by Administrators,
  SYSTEM, or TrustedInstaller, with no allow rule granting write data, append, write
  attributes, delete child, delete, change permissions, take ownership, or generic write/all
  to anyone but those three and CREATOR OWNER. The elevated user's own SID is not trusted.
  103 prints the icacls /setowner repair. Tests move the folder with -AdminRoot and name the
  trusted accounts with -TrustedSid (default: the current user); the checks always run
Evidence: Phase 6 security re-review SA-006/SA-007; on the test host a folder under %TEMP%
  inherits Modify for five other accounts, which the new check refuses (ST-54 fixture moved to
  a protected suite root)
Rejected alternatives: trusting the elevated user's SID (under UAC its unelevated twin holds
  the same SID and could rewrite the access list of a folder that SID owns)
Strongest counterargument: a folder created by an admin under the "object creator" owner
  policy, with UAC off, or by the built-in Administrator is refused until repaired
Confidence: Medium
Hardening: no challenger; failure is closed and the message names the repair
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: acceptance shows the default Program Files owner is the admin user's SID
```

**AR-62 (runtime, auto-design; details AR-46 within 03-05):**

```text
Authority: AI — delegated by --auto-design
Eligibility: representation of facts AR-46 already fixes; no new requirement
Objective: a Windows record that acceptance-check can validate exactly and that the producer
  can write honestly
Decision: Windows records hold the shared core plus os_build (integer >= 22000), installer
  {name: install.ps1, sha256}, toolchain {tag, sha256, bytes} with the rc24 setup pin,
  fs_uae {version, source: downloaded, sha256, bytes (zip pin)} or {version, source:
  existing}, host_hello {compiler, result: ok}, existing_registration: reused,
  standard_account: installed, manual {uac_prompt_seen: true, editor_build_run: true}.
  The version is the one archive version the candidate names. acceptance.ps1 installs the
  candidate twice (the second install must reuse rc24 without elevating), reads the rc24 and
  FS-UAE pins from the private install.ps1 text once the HKLM registration shows 16.2-rc24
  and the config names the unpacked FS-UAE, and records failures as values the checker
  rejects (not-reused, administrator, false)
Evidence: 03-05 §Acceptance evidence; the installer verifies size and SHA-256 of both
  downloads before use and deletes them afterwards
Rejected alternatives: re-downloading the 48 MB setup and the 49 MB zip only to hash them
  (proves the release URLs, not what the installer ran)
Strongest counterargument: the toolchain and FS-UAE hashes are pins, not hashes of bytes the
  producer read
Confidence: Medium
Hardening: no challenger; the installer's own verification is covered by ST-54 and ST-56
Policy version: 1
Root invocation ID: exec-windows-cli-20261126-1938
Reopen triggers: a reviewer requires a byte-level hash of the installed setup
```

**AR-63 (reopened by the Phase 8 docs review; user ruling 2026-11-26):**

```text
Authority: user — the URL form depends on how the release is published, which AR-3 keeps
  with the user
Decision: the Windows install and verify commands use
  https://github.com/OpenLorraine/clorraine/releases/download/vX.Y.Z/<asset>, the same form as
  the Linux v1.2.0 commands; until the release the pages write the version as vX.Y.Z and say
  Windows ships with the first release after v1.2.0. At release, vX.Y.Z is replaced with the
  release tag in README.md, docs/windows-cli.md, and site/guide/quick-start.md, and the
  "next release" wording is removed
Evidence: v1.2.0 is a prerelease; releases/latest resolves to v1.1.2 (GitHub's latest skips
  prereleases), so the earlier releases/latest/download/ choice would 404 for a prerelease
Superseded: the AI-delegated choice of releases/latest/download/ URLs
Reopen triggers: the release is published under a scheme without versioned asset URLs
```
