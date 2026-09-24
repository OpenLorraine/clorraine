# Requirements: Clorraine Demo Video Toolkit

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Clorraine's demo video is built by `demo/story.sh`, a 281-line bash script in which
every scene, caption, ffmpeg filter, and ordering is hard-coded, plus four static
VHS tapes and a Vim source viewer. This feature replaces that with a TypeScript
toolkit, `clorraine-demo`, that renders a video from a scene script. A scene script
can define title/subtitle cards, show a source file with Shiki syntax highlighting
over a custom Clorraine grammar, run a command in a real terminal (VHS), and run a
program under FS-UAE inside a branded frame. The Clorraine story becomes one such
script, and `new <file.clo>` scaffolds a script for any source file.

The toolkit is a developer tool. It is not shipped to users and it does not change
the compiler, the specification, or the standard library.

## Functional Requirements

### Must Have

- A CLI `clorraine-demo` with `render`, `new`, and `list` subcommands, plus the
  `--only <id>[,<id>]` and `--out <path>` options used to export a single chapter
  (AR-6, PF-002).
- A scene script as a TypeScript module exporting a `DemoConfig` via `defineDemo`, or
  an equivalent JSON file with the same schema (AR-21).
- Scene kinds: `card`, `code`, `terminal`, `emulator`, `outro`; every scene accepts an
  optional `caption` and a required unique `id` and positive `duration` (03-01).
- `card` scenes accept a title, an optional subtitle, and optional lines; `outro`
  scenes accept a title and lines (AR-14, PF-014).
- `code` scenes accept a file, optional auto-scroll, and optional highlighted line
  numbers; rendering uses Shiki and the Clorraine grammar (AR-2, 03-03).
- `terminal` scenes accept a list of commands; the tool generates a VHS tape and
  records a real terminal (AR-3, 03-02).
- `emulator` scenes accept a file and an optional duration (**default 7 s**, customizable,
  AR-29); the tool builds and runs the program under FS-UAE (`hosted-amiga`), records the
  window as **real-time video at 50 fps** (so animations stay smooth, AR-29), and composites
  the capture into a branded frame (AR-7, 03-02, PF-014).
- Composition: normalize every clip to the common canvas, trim each clip to its
  declared `duration`, apply a 0.5s crossfade between every adjacent scene, concat, and
  write the output MP4 (AR-16, AR-18, PF-001, PF-003).
- Optional PNG poster (from the first card scene; the first scene if none is a card)
  and GIF outputs (AR-13, AR-17, PF-012).
- A single chapter can be re-rendered to its own file with `--only <id> --out <path>`;
  `make demo` uses this to write `demo/emulator.mp4` (PF-002).
- `new <file.clo>` writes a scene script with default scenes and a title derived from a
  leading `//` comment, else the filename (AR-14).
- Result media (`*.mp4`, `*.gif`) is generated and gitignored; the poster PNG is
  committed (AR-8, AR-13).
- `make demo` renders the Clorraine story; `make demo-check` runs the tool's tests
  (AR-24, AR-25).
- The `demo/clorraine.vim` file stays for editor use (AR-11).

### Should Have

- Clear, actionable errors when Node, Chrome, ffmpeg, VHS, the fonts, or the emulator
  environment is missing; Chrome and the fonts are required, with no silent fallback
  (AR-27, PF-018).
- A deterministic frame-capture path so repeated renders are reproducible (03-02).
- `demo-check` reports test coverage (PF-020).

### Won't Have (Out of Scope)

- Audio or narration.
- Windows or macOS support; the tool assumes Linux with an X display for the emulator.
- Running the emulator leg in CI.
- Publishing the npm package.
- Emulator targets other than `hosted-amiga`/FS-UAE (so the tool takes no `profile`
  option; PF-021).
- Rendering `code` scenes with anything other than Shiki (AR-2).

## Technical Requirements

### Performance

- Config load and validation complete in well under a second.
- Rendering is bounded per scene; the tool caches per-scene clips and reuses them when
  the scene and its inputs are unchanged (03-02).

### Compatibility

- Node.js >= 22.18 (native TypeScript type stripping; AR-19).
- Google Chrome (headless), ffmpeg, VHS v0.11.0, ttyd, X display with `xwininfo`, and
  the DejaVu fonts (03-02).
- VHS lives under `$HOME/go/bin` on this host, so the tool prepends `$HOME/go/bin` and
  `$HOME/.local/bin` to `PATH`, as `demo/story.sh:90` does (PF-010).
- The tool must not modify the Rust workspace, its `Cargo.toml`, or the compiler.

### Security

- A scene script is trusted developer input, like the existing `Makefile`.
- File paths in scenes are resolved against the repository root (overridable for tests)
  and rejected if they escape it; the guard canonicalizes the nearest existing ancestor,
  rejects absolute paths, and applies to write paths (`out`, `poster`) as well as reads
  (PF-016).
- Spawned processes use argument arrays, not shell string interpolation, except where
  the existing `Makefile`/emulator wrapper already does so.
- `terminal` scene text is escaped (or rejected) so it cannot inject VHS directives or
  break the generated tape (PF-017).
- No secrets are read or logged; the `docker/.env` values are never echoed (PF-011 note).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Pipeline | TS orchestrator / Remotion / Motion Canvas | TS orchestrator | Reuses proven VHS/ffmpeg/x11grab; smallest new surface | AR-1 |
| Viewer | Shiki / Pygments / terminal page | Shiki + Clorraine grammar | Best highlighting; explicit user choice | AR-2 |
| Terminal scenes | VHS / HTML card / drop | VHS | Keeps an authentic terminal look | AR-3 |
| Scope | story + scaffold / story only / scaffold only | story + scaffold | Delivers the reusable utility the user asked for | AR-6 |
| Output media | generate + gitignore / commit | generate + gitignore | Avoids large churny binaries | AR-8 |
| Emulator look | branded frame / raw + caption | branded frame | Answers the "not good anymore" complaint | AR-7 |
| Crossfade model | scene ≡ chapter / add grouping | scene ≡ chapter | Flat model; matches the duration formula | PF-001 |
| Single-chapter export | `--only --out` / `extraOutputs` / drop | `--only --out` | Reuses one CLI flag; no schema growth | PF-002 |
| Support surface | zero-dep TS / shiki + Chrome | shiki + Chrome | Approved after independent challenge | AR-15 |

## Acceptance Criteria

1. [ ] `make demo` regenerates `demo/demo-full.mp4`, `demo/demo.gif`, `demo/emulator.mp4`,
   and `demo/poster.png` from `demo/story.ts`.
2. [ ] `make demo-check` passes (`tsc --noEmit` + `node --test` with coverage).
3. [ ] `demo/story.sh`, `demo/show-source.sh`, and `demo/tape/*.tape` are deleted; the
   tracked `demo/*.mp4` and `demo/demo.gif` are untracked.
4. [ ] The README embeds `demo/poster.png` and links to the generated video.
5. [ ] `demo/clorraine.vim` and a refactored `demo/record-emulator.sh` remain.
6. [ ] No change to the compiler, `Cargo.toml`, `spec/`, or `stdlib/`.
7. [ ] `new examples/hello/hello.clo` writes a renderable scaffold.
8. [ ] The strict-profile phase reviews cover input validation and the edge cases in
   this document (PF-022).
