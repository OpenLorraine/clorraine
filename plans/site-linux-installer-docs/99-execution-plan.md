# Task T-02: Document the Linux installer on the website and in docs/

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 9/10 tasks (90%)
> **Reasoning**: medium — wide but mechanical documentation change; public
> deployment at the end
> **Implements**: — (user-requested task, follows `plans/public-preview-v1.2.0`)

## Objective

Make the public documentation match what is released: the **v1.2.0 developer
preview** is installed on **Linux x86_64** with one command, and that is the
only installation path the documentation describes. Windows and macOS are on
the way. Every other installation path (build from source, the Docker SDK,
`make dist`, a manual VSIX, a hand-downloaded toolchain) is removed from the
website and from three install-facing `docs/` files, to be rewritten later.

## Rulings (user, 2026-11-25)

| # | Decision |
|---|---|
| R1 | Remove other installation instructions **everywhere** on the site, including Contributing; the one-line Linux installer is the only path |
| R2 | The Verification page keeps its gate table (it documents what the gates prove, not how to install); only setup steps are removed |
| R3 | Example pages: programs that exit get `clorraine run -p … --emulator` commands; the never-ending showcases (First Light, Boing, Prism) point to the in-browser player and drop their local-run section |
| R4 | Home page: kicker `Developer preview · v1.2.0 · Linux`, a primary **Install on Linux** button, and a short install section with the one-line command and "Windows and macOS are on the way" |
| R5 | `docs/`: only `docs/linux-cli.md`, `docs/README.md`, and `docs/known-limitations.md` (its install/release entries) |
| R6 | Publication: `make site-check` passes, the user previews the changed pages locally, approves, then execution pushes and verifies the live site |

## Facts that constrain the text

- The one-line command is
  `curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh`;
  a literal `curl | bash` is not supported.
- The installer's five steps, flags (`--no-hello`, `--starter DIR`,
  `--toolchain`, `--fs-uae`, `--kickstart`, `--vscode`, `--uninstall`, …), and
  requirements are defined by `tools/setup/install.sh` and the v1.2.0 release
  notes; the docs restate them, never invent new ones.
- Requirements: Linux x86_64, glibc 2.38+, a host `cc` for the hello; optional
  FS-UAE 3.2+, a licensed 512 KB Kickstart 3.1 A1200 ROM, and VS Code. The ROM
  is never bundled or mirrored.
- An Amiga run on a user machine needs `--emulator`: plain `-p hosted-amiga`
  uses `vamos`, which the installer does not provide.
- The showcases never exit; `run --emulator` would stop them after 90 seconds
  (`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`). Hence R3.
- The Intuition GUI demo needs a Workbench public screen, which the bare FS-UAE
  runner does not boot: its page shows only the `clorraine build` command and
  says so.
- `clorraine new [DIR]` and `clorraine setup --kickstart/--fs-uae` exist since
  `ede6823a`; the CLI reference page still shows the old help text.
- Release state: `v1.2.0` is a public **prerelease** (developer preview);
  `v1.1.2` remains the latest stable release (MIT).
- `tools/setup/tests/test_retained_inputs.py` requires `site/guide/quick-start.md`,
  `site/reference/cli.md`, `site/status.md`, and `docs/linux-cli.md` to exist and
  to avoid the removed-pipeline names. `make site-check` fails on dead links.
  `compiler/tests/docs_guide_spec_test.rs` pins the C-guide links in
  `README.md` and `docs/README.md`.
- Out of scope: `README.md` (already current), `site/README.md` (site
  maintainer notes), `docs/amiga-sdk/`, findings notes, the C guide,
  `AGENTS.md`, `RELEASE.md`, screenshots and other visual assets.

## Inventory and target content

| File | Today | Change |
|---|---|---|
| `site/guide/quick-start.md` | Four paths: build from source, Docker SDK, local candidate, manual VSIX; "no prebuilt release" | Rewrite: developer-preview notice; **Install on Linux** (one-line command, the five steps, requirements, checksum-verified variant, scripted flags pointer, uninstall); **Your first project** (`clorraine new`, `code .`, Run in FS-UAE, the CLI equivalent); **Your first program** with `clorraine run hello.clo`; **Windows and macOS are on the way**; troubleshooting without Docker |
| `site/index.md` + `.vitepress/theme/components/HomePage.vue` | Kicker `v1.1.2`; note "installers in progress"; no install CTA | R4: kicker, **Install on Linux** primary button → `/guide/quick-start`, install section with the one-liner (copy-friendly code block) and the platform line; keep the First Light showcase |
| `site/.vitepress/config.ts` | Nav "Guide" → quick start; description "Developer preview." | Keep links; sidebar entry renamed **Install on Linux** if the quick-start title changes; no new pages |
| `site/.vitepress/theme/Layout.vue` | Site-wide bar: "v1.1.2 — installation, packaging … still in progress" (found during execution) | "v1.2.0 for Linux x86_64 installs with one command. Windows and macOS are on the way." with an **Install on Linux** link |
| `site/status.md` | Linux CLI "Not published"; VS Code "Not published"; release section says candidate | Linux CLI **Developer preview** badge (`preview`) with the one-line install; VS Code extension "installed by the Linux installer (VSIX on the release)"; Windows/macOS "On the way"; release section: v1.2.0 public prerelease, v1.1.2 latest stable |
| `site/guide/roadmap.md` | "v1.2.0 candidate; publication approval pending" | Installer row: released as the v1.2.0 developer preview; next: Windows and macOS installers, marketplace extension |
| `site/guide/known-limitations.md` | Rows "Linux CLI candidate", "No binary release is published" with build-from-source workarounds | Replace with: developer preview; Linux x86_64 only (Windows/macOS on the way); the emulator recipe is A1200/KS3.1; showcases run in the browser |
| `site/project/brief.md` | "v1.2.0 candidate"; installation "in progress" | v1.2.0 developer preview on Linux; one-line installer |
| `site/project/contributing.md` | `cargo build --workspace` setup; installer/packaging notes | R1/R2: remove the setup steps; say developer setup instructions are being reworked and link the repository; keep the ways to contribute |
| `site/project/verification.md` | Gate table | R2: keep the table; remove any install/setup step outside it |
| `site/reference/cli.md` | Old help text; `cargo build` for the LSP | Current `clorraine --help` command list (adds `new`; `setup` remembers the emulator paths); `--fs-uae`/`--kickstart` precedence incl. the remembered value; `clorraine-lsp` is installed by the installer |
| `site/amiga/emulator.md` | Docker `make run` for vamos/FS-UAE; `make fs-uae-check`; Docker troubleshooting | The verified FS-UAE run with the installed CLI (`setup` remembers the ROM); remove the Docker SDK and `vamos` instructions and the Docker troubleshooting links |
| `site/amiga/index.md`, `site/amiga/profiles.md`, `site/language/profiles.md` | `make run …` blocks | `clorraine run -p hosted-amiga --emulator FILE` (and `-p host` for the host profile) |
| `site/examples/index.md` | "From a checkout with the SDK: make run …" | How to run an example with the installed CLI (`--emulator`), and that the showcases run in the browser |
| `site/examples/hello.md`, `site/examples/string-parser.md` | `make run` on vamos/FS-UAE | R3: `clorraine run -p hosted-amiga --emulator FILE`, with a link to download the source |
| `site/examples/first-light.md`, `graphics.md`, `prism.md` | "Run it on your own Amiga" with `make run` | R3: drop the local-run section; keep the browser player and the source link |
| `site/examples/gui.md` | `make build …` | `clorraine build -p hosted-amiga …`; it needs a Workbench screen, which the FS-UAE runner does not boot |
| `docs/linux-cli.md` | "v1.2.0 candidate … not published"; build from source, `make dist` | R5: rewrite as the Linux CLI guide for the released installer (same facts as the quick start, plus the CLI's config, emulator precedence, and `clorraine new`) |
| `docs/README.md` | Index row "v1.2.0 CLI toolchain, setup…" | Row text: installing and using the Linux CLI (developer preview) |
| `docs/known-limitations.md` | §12 "Installer integrity" (Docker SDK); late "§9. Linux CLI v1.2.0 candidate" | R5: replace the candidate section with the released state; drop the Docker-SDK installer row's install instructions |

## Tasks

- [x] T-02.1 **Baseline audit (red).** Record, under `build/tmp`, every hit of the
  install-path audit below over the in-scope files. It must list the current
  Docker/source/candidate instructions; this is the failing baseline. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.2 **Quick start rewrite** (`site/guide/quick-start.md`) per the
  inventory; facts only from the installer and release notes. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.3 **Home page** (`HomePage.vue`, `site/index.md`, `config.ts` if a title
  changes) per R4. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.4 **Status, roadmap, known limitations, brief** — release state and
  platform wording (developer preview; Windows and macOS on the way). ✅ (completed: 2026-11-25 17:14)
- [x] T-02.5 **Contributing and Verification** per R1/R2. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.6 **CLI reference** — current help text, `new`, remembered emulator
  paths, `clorraine-lsp` from the installer. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.7 **Amiga and language pages** (`amiga/emulator.md`, `amiga/index.md`,
  `amiga/profiles.md`, `language/profiles.md`) — installed-CLI commands only. ✅ (completed: 2026-11-25 17:14)
- [x] T-02.8 **Example pages** per R3 (index, hello, string parser, the three
  showcases, GUI). ✅ (completed: 2026-11-25 17:14)
- [x] T-02.9 **`docs/`** — `linux-cli.md` rewrite, `README.md` row,
  `known-limitations.md` sections, per R5. ✅ (completed: 2026-11-25 17:14)
- [ ] T-02.10 **Verify, preview, publish.** Audit green; `make site-check`;
  `make dist-check` (retained-inputs test) and
  `make -C docker exec CMD="cargo test --test docs_guide_spec_test"`; every
  command shown in the docs checked against `clorraine --help` / `install.sh
  --help`; local preview with `make site-dev` for the user (R6); after approval,
  commit and push with the **git-commit skill in push mode** (staging only this
  task's files), wait for the Pages deployment, and spot-check the live quick
  start, home page, and status page.

## Install-path audit

Over `site/**/*.md`, `site/.vitepress/theme/Layout.vue`,
`site/.vitepress/theme/components/HomePage.vue`, and the
three `docs/` files, excluding `site/README.md`, `site/public/`, and
`site/.vitepress/dist/`, the green state has **no** match for:

`make run`, `make build`, `make setup`, `make dist`, `make vscode-package`,
`make fs-uae-check`, `make toolchain-check`, `cargo build`, `git clone`,
`docker/README`, `Docker SDK`, `No prebuilt release`, `not published`,
`v1.2.0 candidate`, `Linux CLI candidate`, `installers in progress`,
`v1.1.2` in the home-page kicker,

with two documented exceptions: `site/project/verification.md`'s gate table
(R2) may name `make` gates, and a sentence may say developer setup instructions
are being reworked. Positive checks: the one-line command appears on the quick
start, the home page, and `docs/linux-cli.md`; "developer preview" appears on
the quick start, home page, and status page; "Windows and macOS" appears on the
quick start, home page, and status page.

**Verify**: the audit above, then `make site-check`, `make dist-check`, and
`make -C docker exec CMD="cargo test --test docs_guide_spec_test"`.

## Execution evidence

- **Baseline (red):** 88 install-path hits in 21 files.
- **Ruling change at execution:** the user asked to commit and push directly,
  replacing the R6 local-preview stop; a headless desktop and mobile render of
  the home page was checked instead.
- **Found during execution:** the site-wide preview bar in
  `site/.vitepress/theme/Layout.vue` still read "v1.1.2 — installation …
  still in progress"; added to the inventory and the audit, and updated.
- **Green:** the audit reports no hits (one unrelated roadmap sentence,
  "Internal planning records are not published here", is allowed); the one-line
  command, "developer preview", and "Windows and macOS" appear where required;
  `make site-check` passes with no dead links; `make dist-check` passes
  (installer 181 + 28, producer 46, Python 37 including the retained-inputs
  test); `docs_guide_spec_test` 4 passed. Every command shown matches
  `clorraine --help` and `install.sh --help`.

## Rollback

Until the push, nothing is public. After the push, a bad deployment is fixed by
a follow-up commit (the Pages workflow redeploys on push to `main`); the
release itself is not touched by this task.
