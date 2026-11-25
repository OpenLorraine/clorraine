# Task T-01: Publish the v1.2.0 developer preview

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 17/18 tasks (94%)
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

- **Installer tests red:** the Revision 2 cases failed before the change (spec
  and internal suites), proving the first-run behavior was missing.
- **Installer tests green:** 113 public checks and 24 internal checks, including
  `--toolchain`, the interactive toolchain path, the pinned download with
  verification, `--no-toolchain`, and the exact host hello.
- **Candidate:** `make dist` produced the archive (6,530,214 bytes,
  `a23706d2…6485`), VSIX (26,435 bytes, `9fa9e826…6ffa`), and installer
  (28,026 bytes, `8cb95c34…d72a`). `make dist-verify` passes.
- **Acceptance:** `make acceptance` passed the seven-guest matrix on the rebuilt
  candidate; evidence `build/acceptance/20261125T104249Z/evidence.json`;
  `make acceptance-check` reports `ok`. `make check` passes.
- **Release:** the rebuilt public prerelease `v1.2.0` is at
  `https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0`; title
  `v1.2.0 developer preview`; `isPrerelease=true`, `isDraft=false`; `v1.1.2`
  remains Latest. The four public assets returned HTTP 200 and `SHA256SUMS`
  verifies.
- **Full first run:** the one-line command downloaded the installer, offered
  and downloaded the pinned rc24 toolchain from AmigaPorts, configured it,
  ran the host hello, and printed the next steps. The installed CLI reports
  `clorraine 1.2.0`; the toolchain is under
  `~/.local/share/clorraine/toolchains/rc24`; the hello output is exact. The
  run was repeated in a fresh HOME with the same result. Log:
  `build/tmp/first-run-verified.log`.
- **README:** the install section now says the installer sets up the rc24
  toolchain (download or existing copy) and runs the hello test.
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
- [ ] T-01.18 **Recreate the prerelease.** Delete and recreate `v1.2.0` with the
  rebuilt assets, then verify the one-line install with VS Code, FS-UAE, the
  persistent examples, and `v1.1.2` still Latest.

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

## Verify

The installer suites pass in both interactive (pseudo-terminal) and
non-interactive modes; `make dist`, `make acceptance`, and `make
acceptance-check` pass on the rebuilt candidate; `gh release view v1.2.0`
reports `isPrerelease=true`, `isDraft=false`, four assets with
matching downloaded hashes, and public URLs return HTTP 200; the one-line
command installs `clorraine 1.2.0` and prints the expected hello line; `v1.1.2`
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
   archive (default yes; about 70 MB).

It then verifies the archive, installs it under
`$XDG_DATA_HOME/clorraine/toolchains/rc24` (or `~/.local/share`), runs
`clorraine setup --toolchain`, and builds and runs a temporary host `hello.clo`.
It requires the exact output `Hello, Amiga, from Clorraine!`. A host `cc` is
needed for that check; non-interactive runs skip it unless `--toolchain PATH`
is supplied.

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
