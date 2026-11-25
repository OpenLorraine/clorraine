# Task T-01: Publish the v1.2.0 developer preview

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 26/27 tasks (96%)
> **Reasoning**: high — public release, interactive installer, immutable tag,
> asset verification, rollback
> **Decision**: Public GitHub **prerelease**, tag `v1.2.0`, title
> `v1.2.0 developer preview`, with an interactive installer launched by **one
> simple download-run command**. The first run detects or offers the pinned
> upstream rc24 toolchain, configures it, and runs a host hello smoke test.
> Docs, website, release notes, changelog, and `RELEASE.md` stay unchanged
> until the user's test passes; the README is updated for the new installer.
> GitHub keeps `v1.1.2` as Latest; no stable v1.2.0 release is created.

## Objective

Make the first impression one command, then a friendly installer:

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

The installer detects Linux x86_64 and glibc, asks for the install prefix, asks
whether to add the bin directory to PATH, confirms, verifies the archive
against the pinned size and SHA-256, installs side by side with an atomic
`current` switch, and prints the version and the exact next steps.

The command is deliberately **download → run**, not `curl … | bash`:

- the project's installer rule forbids piping remote text into a shell;
- a piped script consumes stdin, so the installer cannot reliably prompt;
- a saved file can be inspected before it runs.

The release body also offers the longer verified path (download `install.sh`
and `SHA256SUMS`, check the checksum, then run) for cautious users.

**Smallest viable design:** add interactive prompts to the existing installer
only when both stdin and stdout are terminals; preserve every current
non-interactive flag and behavior for CI, tests, and scripts; rebuild and
re-accept the candidate; publish one public prerelease. Do not add a new
bootstrap service, a short-link domain, or a literal pipe.

## Confirmed decisions

| Decision | Value |
|---|---|
| Release state | Public prerelease; not Latest; `v1.1.2` remains Latest |
| Tag and title | `v1.2.0` / `v1.2.0 developer preview` |
| Primary install | One-line `curl … -o clorraine-install.sh && sh clorraine-install.sh` |
| Installer prompts | Prefix (default `~/.local`), PATH (default yes), confirm (default yes) |
| Non-interactive | Current flags/defaults preserved; no prompt when stdin or stdout is not a terminal |
| Release assets | Archive, VSIX, `install.sh`, `SHA256SUMS` |
| Repository docs/site | Frozen: no `docs/`, `site/`, `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, or `AGENTS.md` changes |
| Test | The user downloads the public assets and tests on their own machine |
| Rollback | Only on the user's explicit instruction; never automatic |
| Follow-up | Separate plan updates docs/site and optionally promotes the release |

## Scope

**In scope:** failing installer tests for interactive and non-interactive
modes; installer prompts and next-step output; the one-line install command;
candidate rebuild; fresh acceptance and hashes; a public prerelease; release
verification; the user's test instructions.

**Out of scope:** a literal `curl … | bash`; a new download/bootstrap service or
short-link domain; rebuilding the compiler behavior; changing the candidate
version; docs/website/README/release-note/changelog edits; a stable or Latest
release; deleting `v1.1.2`; public announcements.

## Tasks

- [x] T-01.1 **Preflight.** Confirm the current candidate hashes and
  `06-publication-handoff.md`; confirm `gh` is authenticated as an admin of
  `OpenLorraine/clorraine`; confirm no `v1.2.0` release or tag exists; confirm
  `v1.1.2` is Latest. Record the release target commit. Do not publish yet. ✅ (completed: 2026-11-25 11:14)

- [x] T-01.2 **Installer tests first.** Extend the shell installer suites with
  failing cases for: a pseudo-terminal session that answers prefix/PATH/confirm
  prompts; the default answers; a non-terminal run that never blocks and keeps
  the existing flags/defaults; and the post-install next-step output. Use the
  existing fixture archive; do not require network, Docker, or a real toolchain. ✅ (completed: 2026-11-25 11:14)

- [x] T-01.3 **Implement the interactive installer.** In
  `tools/setup/install.sh`, enable prompts only when stdin and stdout are
  terminals; prompt for the prefix (`~/.local`), the PATH block (yes), and the
  install confirmation (yes). Preserve `--prefix`, `--modify-path`,
  `--archive`, `--checksums`, `--uninstall`, and `--purge` exactly. Print the
  installed version, the toolchain-setup command, a hello example, and the VSIX
  command. Reject nothing new; no literal-pipe mode. Run the installer suites. ✅ (completed: 2026-11-25 11:14)

- [x] T-01.4 **Rebuild and re-accept the candidate.** Run `make dist` and
  `make acceptance` with the pinned rc24 archive and licensed KS3.1 A1200 ROM;
  update the release-body hashes from the rebuilt `build/dist`. Verify
  `make acceptance-check` and the new asset sizes/SHA-256. The candidate bytes
  change; do not reuse the old hashes. ✅ (completed: 2026-11-25 11:14)

- [x] T-01.5 **Write the preview release body** under `build/tmp/` (not a
  repository document). Include the one-line command, the optional verified
  path, the prompt description, the rebuilt hashes, the rc24/FS-UAE/KS3.1
  requirements, the A1200-vs-A500 caveat, and the docs/site-pending notice.
  The first instruction must be the one-line command above. ⏳ (implemented: 2026-11-25 11:14)

- [x] T-01.6 **Create the public prerelease.** `gh release create v1.2.0
  --prerelease --title "v1.2.0 developer preview" --notes-file <body>
  --target main` with the four rebuilt assets. Do not pass `--latest`. Record
  the release URL and database ID. ✅ (completed: 2026-11-25 11:16)

- [x] T-01.7 **Verify the uploaded release.** Require `isPrerelease=true`,
  `isDraft=false`, tag `v1.2.0`, and exactly four assets; `v1.1.2` remains Latest.
  Download the public assets; compare sizes and SHA-256 to the release body and
  `SHA256SUMS`; require public asset URLs to return HTTP 200; confirm `v1.1.2`
  remains Latest. Run the one-line command in a fresh HOME on this host with a
  terminal and confirm the prompts, installed version, and hello output. ✅ (completed: 2026-11-25 11:16)

- [x] T-01.8 **Deliver and close.** Give the user the one-line command and the
  test checklist; record that their test is the next decision point. Confirm
  no docs/site/README/release-note changes; record rollback options; commit the
  plan update with the `git-commit` skill. ✅ (completed: 2026-11-25 11:16)

## Execution evidence

- **Installer tests:** 147 public checks, 28 internal checks, and 41 producer
  checks pass; `make dist-check` passes. The new cases cover VS Code detection,
  the pinned VSIX download, persistent examples, FS-UAE/ROM detection, the
  guest run, and the fallback commands.
- **Candidate:** `make dist` produced the archive (6,530,816 bytes,
  `547d1bdd…fdbb`), VSIX (26,435 bytes, `3918ca3f…e180`), and installer
  (42,958 bytes, `2239e244…3e05`). `make dist-verify` passes.
- **Acceptance:** `make acceptance` passed the seven-guest matrix on the rebuilt
  candidate; evidence `build/acceptance/20261125T112110Z/evidence.json`;
  `make acceptance-check` reports `ok`. `make check` passes.
- **Release:** the rebuilt public prerelease `v1.2.0` is at
  `https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0`; title
  `v1.2.0 developer preview`; `isPrerelease=true`, `isDraft=false`; `v1.1.2`
  remains Latest. The four public assets returned HTTP 200 and `SHA256SUMS`
  verifies.
- **Full first run:** the public one-line command downloaded the installer and
  the pinned rc24 toolchain, installed the CLI, ran the host hello, detected
  FS-UAE and the licensed ROM, ran the five-second guest hello, downloaded and
  installed the VSIX, and printed the complete summary. The installed CLI
  reports `clorraine 1.2.0`; both examples are under
  `~/.local/share/clorraine/examples`; `code --list-extensions` lists
  `openlorraine.clorraine`. Log: `build/tmp/first-run-polish-public.log`.
- **README:** the install section now describes the toolchain, VS Code, FS-UAE,
  and persistent hello behavior.
- **Docs/site:** unchanged; still pending the user's test.

## Revision 2 — full first run

- [x] T-01.9 **Installer first-run tests.** Add failing cases for an existing
  rc24 root (`--toolchain`), an interactive toolchain path, the pinned upstream
  download/extract path, the host hello smoke test, `--no-toolchain`, and the
  updated success summary. The existing CLI-install tests remain green.
- [x] T-01.10 **Implement the first-run steps.** Detect an existing root or
  `CLORRAINE_TOOLCHAIN`; otherwise prompt for an existing root or offer the
  pinned AmigaPorts rc24 download (69,868,548 bytes, SHA-256 `b93b070a…c100`);
  verify before extraction; install under
  `$XDG_DATA_HOME/clorraine/toolchains/rc24` (or `~/.local/share`); run
  `clorraine setup --toolchain`; create a temporary `hello.clo`, run it on the
  host, and require `Hello, Amiga, from Clorraine!`. Add `--toolchain PATH` and
  `--no-toolchain`; skip the toolchain/hello in non-interactive runs unless a
  root is supplied.
- [x] T-01.11 **Rebuild and re-accept.** `make dist`, `make acceptance`, and
  `make acceptance-check` on the rebuilt candidate; update the release body and
  hashes. Update the README toolchain wording (the installer now offers the
  download) while leaving docs/site frozen.
- [x] T-01.12 **Recreate the v1.2.0 prerelease.** After the new candidate
  passes, delete the current `v1.2.0` prerelease and tag (0 downloads), then
  recreate it with the rebuilt assets and body. Verify the one-line command end
  to end: download, toolchain download/setup, host hello, version, and
  `v1.1.2` remaining Latest. Record rollback options. ✅ (completed: 2026-11-25 11:50)

## Revision 3 — first-run polish

- [x] T-01.13 **First-run polish tests.** Add failing cases for VS Code
  detection and extension install, the persistent host and Amiga hello files,
  FS-UAE and Kickstart detection, an automatic guest hello, and the fallback
  next-step commands. Keep the existing installer tests green.
- [x] T-01.14 **Install the VS Code extension.** Detect `code`, `codium`, or
  `code-insiders`; offer the pinned VSIX from the same release; verify its bytes
  and SHA-256 before `--install-extension`. Add `--vscode` and `--no-vscode`;
  non-interactive runs skip it unless `--vscode` is supplied.
- [x] T-01.15 **Leave runnable examples.** Write `hello.clo` and
  `hello-guest.clo` under `$PREFIX/share/clorraine/examples` without overwriting
  an existing file. The guest example prints the hello line and waits five
  seconds so the FS-UAE window is visible.
- [x] T-01.16 **Run the guest hello when possible.** Detect FS-UAE and a
  licensed 512 KB KS3.1 A1200 ROM from `--fs-uae`/`--kickstart`, the
  `CLORRAINE_*` environment, or common paths. If both are present and a display
  is available, run `hello-guest.clo` on FS-UAE and show the output. Otherwise
  keep the host hello and print the exact guest command. Add `--no-emulator`.
- [x] T-01.17 **Rebuild and re-accept.** `make dist`, `make acceptance`, and
  `make acceptance-check`; update the release body, asset hashes, and README.
- [x] T-01.18 **Recreate the prerelease.** Delete and recreate `v1.2.0` with the
  rebuilt assets, then verify the one-line install with VS Code, FS-UAE, the
  persistent examples, and `v1.1.2` still Latest. ✅ (completed: 2026-11-25 12:29)

### Revision 3 evidence

- **Installer suites:** 147 public checks, 28 internal checks, and 41 producer
  checks pass; `make dist-check` passes.
- **Candidate:** archive 6,530,816 bytes `547d1bdd…fdbb`, VSIX 26,435 bytes
  `3918ca3f…e180`, installer 42,958 bytes `2239e244…3e05`; `make dist-verify`
  passes.
- **Acceptance:** `build/acceptance/20261125T112110Z/evidence.json`;
  `make acceptance-check` reports `ok`.
- **Real host checks:** the non-interactive installer ran the guest hello on
  FS-UAE with the licensed ROM; the interactive path auto-detected FS-UAE and
  the ROM and ran the five-second guest hello; the VSIX installed into a
  temporary extensions directory and listed `openlorraine.clorraine`.

## Revision 4 — an exciting first run

User verdict on Revision 3: functionally passed, but the experience is flat.
Rulings (2026-11-25): demo-scene intro for the Amiga hello; a hackable visual
starter app; a permanent `clorraine new` command; a Run-in-FS-UAE button in the
VS Code editor title. Probe finding: every `std.amiga.video` display is black on
the A1200/KS3.1 recipe because a silent startup leaves bitplane DMA off; the
default `openDisplay` offset also pushes the picture right on that machine.

- [x] T-01.19 **Fix the KS2+ custom display.** Enable raster DMA after the first
  `LoadView`; prove it on the A1200 runner and keep the A500 gates green. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.20 **Demo-scene hello intro.** `examples/amiga/hello-intro.clo`:
  copper sky and chrome logo gradients, moving raster bars, a static star field, a
  waving greeting; exits on its own (or on a mouse click) and prints the exact
  hello line. User visual approval before the next demo. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.21 **Hackable starter app.** A short, commented visual program that
  quits on a mouse click; user visual approval. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.22 **`clorraine new` and remembered emulator settings.** `new [DIR]`
  writes the starter, `.vscode/` tasks/settings/extension recommendation, a
  README, and `.gitignore`; refuses a non-empty directory. `setup` also stores
  `--kickstart` and `--fs-uae`; `run --emulator` falls back to them. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.23 **VS Code run button.** Editor-title Run in FS-UAE / Run actions
  for `.clo` files; extension tests updated. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.24 **Installer experience.** Branded, step-numbered output; asks
  before running the hello; runs the intro in FS-UAE when available; offers a
  starter in the current folder and opens it in VS Code; closes with clear next
  steps. Tests first; non-interactive flags preserved. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.25 **Independent review.** Correctness reviewer and
  `amiga-platform-reviewer` (no ownership or drop change, so no memory-safety
  audit) on the shim, demos, CLI, extension, and installer. ✅ (completed: 2026-11-25 15:01)
- [x] T-01.26 **Rebuild and re-accept.** `make check`, the A500 video gates,
  `make dist`, `make acceptance`, `make acceptance-check`, `make vscode-check`. ✅ (completed: 2026-11-25 15:01)
- [ ] T-01.27 **Recreate the prerelease** after the user's approval of the
  visuals, then verify the public one-line install end to end.

### Revision 4 evidence

- **Display fix:** a bare KS3.1 startup left bitplane DMA off; the first flip
  now enables raster DMA after `WaitTOF` (Intuition's `ON_DISPLAY`). Probed on
  the A1200 runner; `amiga_video_impl_test::the_first_flip_enables_bitplane_dma`
  fails without the fix. A500 gates boing, sprites, paddle, text, and assets
  pass unchanged. Follow-up: no gate yet boots a video program on KS2+ from a
  bare startup (AP-001); the intro run in the installer is the current proof.
- **Intro:** `examples/amiga/hello-intro.clo`, 49 fps on a stock A1200 (release),
  39 fps debug; dirty-region updates after a 6 fps first draft. Stars are a
  static field (per-frame plots were the CPU bottleneck without fast RAM).
  User approved the look.
- **Starter:** `compiler/templates/starter/main.clo`, 50 fps debug; per-buffer
  erase before draw (user-reported H trail fixed, verified at 10x speed);
  click = button held two frames then released (a stray reading quit early on
  the user's desktop). User approved.
- **CLI:** `clorraine new [DIR]`; `setup` remembers `--kickstart`/`--fs-uae`;
  `run --emulator` resolves flag, environment, remembered, PATH.
  `first_run_spec_test` 12 cases.
- **Extension:** Run in FS-UAE and Run editor-title buttons; `make vscode-check`.
- **Installer:** banner, five steps, hello prompt, intro in FS-UAE, starter in
  the current folder, open in VS Code, summary with next commands; 181 spec,
  28 impl, 46 producer checks.
- **Reviews:** amiga-platform-reviewer (AP-001..AP-006) and correctness
  (RV-001..RV-008): no critical/major; all minors fixed except AP-001
  (follow-up gate) and AP-005 (nit, left).
- **Gates:** `make check`, `make dist`, `make dist-verify`, `make acceptance`
  (`build/acceptance/20261125T135937Z/evidence.json`), `make acceptance-check`.
  Final interactive installer run: toolchain, Kickstart and FS-UAE remembered,
  both hellos, starter created; the starter runs from its folder with no flags.
- **Candidate:** archive 6,556,265 `af33058f…3794`; VSIX 26,530
  `3901f7d0…ba68`; installer 83,216 `d64f7de9…c3ca`.

## Verify

The installer suites pass in both interactive (pseudo-terminal) and
non-interactive modes; `make dist`, `make acceptance`, and `make
acceptance-check` pass on the rebuilt candidate; `gh release view v1.2.0`
reports `isPrerelease=true`, `isDraft=false`, four assets with
matching downloaded hashes, and public URLs return HTTP 200; the one-line
command installs `clorraine 1.2.0`, downloads the toolchain, runs the host and
FS-UAE hello checks, installs the extension, and leaves both examples; `v1.1.2`
remains Latest; and `git status` shows no docs/website/release-note changes.

## Install and test instructions (release body)

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

The installer asks:

1. Where to install (default `~/.local`);
2. Whether to add the bin directory to `~/.profile` (default yes);
3. Whether to install now (default yes);
4. For an existing rc24 root, or whether to download the pinned AmigaPorts
   archive (default yes; about 70 MB);
5. For a Kickstart ROM when FS-UAE is detected;
6. Whether to install the VS Code extension when an editor is detected.

It then verifies the archive, installs it under
`$XDG_DATA_HOME/clorraine/toolchains/rc24` (or `~/.local/share`), runs
`clorraine setup --toolchain`, and builds and runs the host hello. It requires
the exact output `Hello, Amiga, from Clorraine!`. A host `cc` is needed for that
check. When FS-UAE and a licensed ROM are available it also runs the Amiga hello
and leaves both sources under `$PREFIX/share/clorraine/examples`; otherwise it
prints the exact guest command. Non-interactive runs skip the optional steps
unless the matching flags are supplied.

Confirm the install:

```sh
"$HOME/.local/bin/clorraine" --version
```

For the guest run, create `hello.clo`:

```clorraine
module demo.Hello;
import { println } from std.io;
func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
```

Run it on the host and in FS-UAE:

```sh
"$HOME/.local/bin/clorraine" run hello.clo
"$HOME/.local/bin/clorraine" run -p hosted-amiga --emulator \
    --kickstart "$HOME/amiga/roms/amiga-os-310-a1200.rom" hello.clo
```

Install the VSIX from the same release:

```sh
curl -fsSLO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/clorraine-1.2.0.vsix
code --install-extension clorraine-1.2.0.vsix
```

Expected guest output: `Hello, Amiga, from Clorraine!`. FS-UAE must be 3.2.0 or
newer; the Kickstart must be a licensed 512 KB KS3.1 A1200 image. The A500
profile runs on the A1200 recipe but is not a Kickstart 1.3 proof.

For a verified install, download `install.sh` and `SHA256SUMS`, run
`sha256sum -c SHA256SUMS --ignore-missing`, and then run `sh install.sh`.
The one-line download-run is the primary path; a literal `curl … | bash` is
not supported.

## Rollback and follow-up

The tag and prerelease are public. If the user's test fails, do not delete or
rewrite anything automatically. Present the choice: delete the prerelease and
tag (public history change), or publish a fixed prerelease under a new tag.
A public/stable release, docs/website updates, and any promotion of this
prerelease belong to a separate approved plan after the user's test.
