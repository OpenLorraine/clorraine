# Merge Notes: Windows CLI and installer

> **Document**: 08-merge-notes.md
> **Parent**: [Index](00-index.md)
> **Status**: Draft — nothing here is filed, opened, or merged without the user's approval

## Before merging

1. Rebase `windows-cli` on `main` and rerun `make check IMAGE=clorraine-sdk:windows-cli`
   (task 9.2.3).
2. The user approves opening the pull request (task 9.2.4).

## After merging (AR-37)

1. **Rebuild the shared image.** `make -C docker image` rebuilds `clorraine-sdk:dev` with the
   MinGW-w64 layer and the `x86_64-pc-windows-gnu` Rust target. `make check` now runs
   `cargo check --target x86_64-pc-windows-gnu`, so an older `clorraine-sdk:dev` fails that step.
   A from-source rebuild currently fails upstream: amiga-gcc clones vbcc's `master` branch, which
   no longer exists (AR-54). Until that is fixed upstream, build the Windows layer on top of the
   existing `clorraine-sdk:dev`, as the branch image was built.
2. **`AGENTS.md`.** Add the Windows facts:
   - Profiles/Project: the CLI also builds for `x86_64-pc-windows-gnu`; the Windows installer is
     `tools/setup/install.ps1`.
   - Commands: `make check` includes the Windows `cargo check`; `make dist` writes five assets
     (Linux archive, Windows zip, VSIX, `install.sh`, `install.ps1`) under one `SHA256SUMS`;
     `make dist-check` runs the Windows packaging and evidence suites; `make acceptance-check`
     validates Linux and Windows records.
   - Testing: the PowerShell suites `tools/setup/tests/install.{spec,impl}.test.ps1` run on a
     Windows 11 host (not in `make check`); `tools/setup/windows-remote.sh` pushes a folder to
     the Windows test host and runs a command in its desktop session; Windows acceptance is
     `tools/setup/acceptance.ps1`.
   - Layout: `docs/windows-cli.md`; `tools/setup/pe_check.py`.
3. **Roadmap.** Move the `windows-cli` row to its completed stage in `plans/00-roadmap.md`.
4. **Local `docker/.env` (not tracked).** The main checkout's `XAUTHORITY` still names
   `/run/user/1000/.mutter-Xwaylandauth.7G0MY3`, which no longer exists; the current Xwayland
   auth file is `.mutter-Xwaylandauth.89WIZ3`.

## At release (AR-3, AR-63)

1. Choose the release that carries Windows.
2. Replace `vX.Y.Z` with its tag in the Windows commands of `README.md`, `docs/windows-cli.md`,
   and `site/guide/quick-start.md`, and remove the "next release" wording there and in
   `site/status.md`, `site/guide/known-limitations.md`, `site/guide/roadmap.md`,
   `site/project/brief.md`, `site/project/contributing.md`, and the two site theme components.
3. Bump the version, rebuild with `make dist`, and update `CHANGELOG.md`, `RELEASE-NOTES.md`,
   and `RELEASE.md`. Upload all five assets and `SHA256SUMS`.

## Known gate behaviour

- `sdk-a500-sprites` fails on the Wayland desktop display: the Amiga pointer starts a few lines
  lower (429 pixels differ in the top-left corner) while the playfield, the sprites, and the
  collision line match. The `sprite-demo` binary is byte-identical between `main` and this
  branch, and the unchanged gate passes on a private Xvfb display. `make verify` passed with
  `DISPLAY` set to a private Xvfb server.

## Follow-up: the VS Code extension for a local candidate

Both installers download the VSIX from the pinned release URL, also with `--archive`/`-Archive`.
For an unpublished candidate that URL serves the previous release's VSIX, whose size and SHA-256
differ from the pins, so the installer correctly refuses it and the extension is not installed.
The Windows acceptance run installed the candidate VSIX by hand with the installer's own launcher
logic. A follow-up could use a pinned-matching VSIX found next to a local archive, on both
platforms.

## Decision for the user: encrypted ROMs on Linux

On Windows `clorraine` now refuses an encrypted Cloanto ROM (header `AMIROMTYPE1`) that has no
`rom.key` beside it, and the installer's ROM search skips such a ROM; before, FS-UAE showed a
black screen and the run timed out after 90 seconds. Linux behaves as before (AR-29 keeps
Linux unchanged), but FS-UAE on Linux has the same need. Extending the check to Linux is a
one-line change in `kickstart_key_missing_for` plus `install.sh`'s ROM search.

## Follow-up issue (draft): Clang cannot build debug programs from the emitted C

**Title:** Emitted C is not warning-free under Clang's `-Werror` (debug builds)

**Body:**

With Clang as the host C compiler (for example LLVM-MinGW's `cc.exe` on Windows, or a Linux
host whose `cc` is Clang), `clorraine run` and `clorraine build` fail in debug mode, because
the debug flags add `-Wall -Wextra -Werror` and Clang reports warnings GCC does not:

- `-Wunused-function` for the runtime's `static inline` helper `cl_isnan` when a program does
  not use it;
- `-Wparentheses-equality` for comparisons the emitter wraps in an extra pair of parentheses.

Release builds succeed. The failures come from the emitted C and the strict flags, not from
Windows. On Windows the driver tries `gcc`, `cc`, then `clang`, and the documentation tells
users to install GCC (w64devkit or MSYS2 UCRT64).

**Done when:** the corpus and examples build with Clang under the debug flags on Linux and
Windows, a Clang leg is added to the strict-build checks, and the Windows guide drops the Clang
limitation.
