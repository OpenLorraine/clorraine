# Current State: Clorraine Demo Video Toolkit

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The demo video is built entirely by bash. `demo/story.sh` (281 lines) hard-codes the
story text, the ffmpeg `drawtext` card layout, the caption filters, the chapter order,
and the final `concat`. It writes `demo/demo-full.mp4`, `demo/demo.gif`, and
`demo/emulator.mp4`. Terminal and Vim chapters are pre-written VHS tapes under
`demo/tape/`. `demo/show-source.sh` opens a file read-only in Vim, using the Clorraine
syntax rules in `demo/clorraine.vim`. `demo/record-emulator.sh` (136 lines) builds a
`hosted-amiga` program, boots FS-UAE, waits for the window, and captures it with
`x11grab`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `demo/story.sh` | Hard-coded story builder | Replace with `demo/story.ts`; delete the script |
| `demo/record-emulator.sh` | FS-UAE capture | Refactor: fix the source-stem output collision; callable by the tool |
| `demo/show-source.sh` | Vim viewer launcher | Delete; superseded by the Shiki viewer |
| `demo/tape/*.tape` | Static VHS tapes (4) | Delete; generate tapes from scenes |
| `demo/clorraine.vim` | Clorraine syntax for Vim | Keep unchanged (AR-11) |
| `demo/demo-full.mp4`, `demo/demo.gif`, `demo/emulator.mp4` | Committed result media | `git rm --cached`; gitignore (AR-8) |
| `README.md` | Embeds `demo/demo.gif` and describes the story | Embed `demo/poster.png`; link the generated video |
| `Makefile` | Entry points | Add `demo` and `demo-check` targets |
| `.gitignore` | Build output ignores | Ignore `*.mp4`/`*.gif` under `demo/` |
| `AGENTS.md` | Project guidance | Document the tool, commands, and layout |

### Code Analysis

`demo/story.sh:120-145` defines `normalize()` and `card()`: all visuals are ffmpeg
filters, so typography and layout are limited to `drawtext`. `demo/show-source.sh:13`
runs `vim -R` with a colorscheme, so the source look is a terminal Vim session.
`demo/record-emulator.sh:29-30` derives both the build output directory and the program
name from the source file stem, so two different `program.clo` files share the artifact
path `build/clorraine/program/hosted-amiga/program`. `demo/story.sh:101-114` has a
`record_tape` cache (`reuse when the output is newer than the tape`), but the emulator
chapters have no cache.

## Gaps Identified

### Gap 1: No data-driven scene model

**Current Behavior:** Every scene, caption, and ordering is a bash literal.
**Required Behavior:** A typed scene list (card/code/terminal/emulator/outro) that the
tool renders.
**Fix Required:** `tools/demo/src/scenes.ts` plus a config loader (03-01).

### Gap 2: Weak visual output

**Current Behavior:** Cards are ffmpeg `drawtext`; source is a terminal Vim session.
**Required Behavior:** HTML/CSS cards and Shiki-rendered, grammar-driven source.
**Fix Required:** Chrome rendering and the Clorraine grammar (03-02, 03-03).

### Gap 3: Stale, duplicated emulator handling

**Current Behavior:** `record-emulator.sh` is invoked directly and the stem collision
at `demo/record-emulator.sh:29-30` is latent.
**Required Behavior:** A reusable recorder that cannot collide and that the tool drives.
**Fix Required:** Refactor the recorder (03-02).

### Gap 4: No per-file generation

**Current Behavior:** Only the single Clorraine story exists.
**Required Behavior:** `new <file.clo>` scaffolds a renderable scene script.
**Fix Required:** `tools/demo/src/scaffold.ts` (03-04).

## Dependencies

### Internal Dependencies

- `demo/record-emulator.sh`, `docker/scripts/run.sh`, `docker/.env` (emulator capture).
- The `make build FILE=… PROFILE=hosted-amiga` passthrough (program build).
- `examples/` and `spec/tests/corpus/` sources shown in the video.

### External Dependencies

- Node.js >= 22.18, `shiki` (npm), dev `typescript`.
- Google Chrome (headless), ffmpeg, VHS v0.11.0, ttyd, `xwininfo`, Docker SDK, FS-UAE.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Headless Chrome frame capture is host-sensitive | Med | Med | Deterministic `t`-driven pages, fixed canvas, local fonts, `--force-device-scale-factor=1` (03-02) |
| npm/browser surface drifts from the dependency-free ethos | Med | Low | Isolated in `tools/demo/`; committed lockfile; Rust workspace untouched (AR-15) |
| VHS v0.12 is broken (v0.11 pinned) | Low | Med | Keep the version check from `demo/story.sh:110-113` |
| Emulator scenes need an X display + Kickstart | High | Low | `make demo` is local-only and out of `make verify` (AR-12) |
| Generated media churn in git | Low | Med | `git rm --cached` + gitignore; commit only the poster (AR-8) |
