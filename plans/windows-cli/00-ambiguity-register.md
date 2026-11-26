## Ambiguity Register: Windows CLI and installer

> **Status**: ✅ GATE PASSED — all 53 items resolved
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
