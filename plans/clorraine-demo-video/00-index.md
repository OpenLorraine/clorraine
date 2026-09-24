# Clorraine Demo Video Toolkit Implementation Plan

> **Feature**: A TypeScript toolkit that renders Clorraine demonstration videos from a scene script: HTML/CSS cards, Shiki-highlighted source with a Clorraine grammar, real-terminal command scenes, and FS-UAE emulator captures in a branded frame.
> **Status**: Planning Complete
> **Created**: 2026-09-24 21:35
> **Implements**: — (standalone feature; tracked as roadmap `PLAN-09`)
> **CodeOps Artifact Schema**: 1

## Overview

The Clorraine demo video is produced by `demo/story.sh`, a 281-line bash script in
which every scene, caption, ffmpeg filter, and ordering is hard-coded, plus four static
VHS tapes and a Vim source viewer. The result is hard to change and the look is limited
to ffmpeg `drawtext` cards and a terminal Vim session. This plan replaces that with
`@clorraine/demo`, a zero-build TypeScript tool under `tools/demo/`.

A scene script (a TypeScript module or an equivalent JSON file) describes the video as a
list of scenes. Each scene renders to a clip: `card`/`outro` scenes are HTML/CSS;
`code` scenes are Shiki-highlighted HTML using a new Clorraine TextMate grammar;
`terminal` scenes generate a VHS tape and record a real terminal; `emulator` scenes
build a program and capture FS-UAE inside a branded frame. `compose` normalizes,
captions, crossfades, and concatenates the clips. The Clorraine story becomes one such
script, and `new <file.clo>` scaffolds a script for any source file.

The tool reuses the proven capture tools (VHS, ffmpeg, `x11grab`, the SDK Docker image)
and adds only the Node/Shiki/Chrome surface needed for the better look. The compiler,
the specification, and the standard library are untouched.

## Minimum-Sufficient Baseline

**Original goal:** a utility that turns a Clorraine file into a demonstration video (cards, a syntax-highlighted source viewer, emulator runs), chosen to be written in TypeScript.
**Smallest viable design:** a zero-build TypeScript CLI under `tools/demo/` that reuses VHS, ffmpeg, `x11grab`, and the existing recorder, with a typed scene model and a generated config.
**Excluded machinery:** Remotion/Motion Canvas, a video-editor GUI, audio, non-Amiga emulators, CI execution of the emulator leg, and publishing the package.
**Approved complexity:** the `tools/demo/` npm package with `shiki` + headless Chrome (AR-15), approved after an independent challenge returned `Simplify`.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current demo pipeline |
| 03-01 | [Tool Architecture](03-01-tool-architecture.md) | CLI, config model, module layout, toolchain |
| 03-02 | [Renderers](03-02-renderers.md) | Scene renderers, Chrome capture, composition |
| 03-03 | [Grammar and Theme](03-03-clorraine-grammar.md) | Clorraine TextMate grammar and Shiki theme |
| 03-04 | [Story and Scaffold](03-04-story-and-scaffold.md) | Story migration, `new`, Makefile, README |
| 07  | [Testing Strategy](07-testing-strategy.md) | Spec test cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

```ts
// A minimal config module (the real demo/story.ts has the 15 story chapters)
import { defineDemo } from "../tools/demo/src/index.ts";

export default defineDemo({
  out: "demo/demo-full.mp4",
  poster: "demo/poster.png",
  gif: { out: "demo/demo.gif", sceneIds: ["intro", "hello-src", "hello-emu", "hello-explain"] },
  scenes: [
    { id: "intro",     kind: "card",     title: "Clorraine", subtitle: "A safe, C-like language", duration: 7 },
    { id: "hello-src", kind: "code",     file: "examples/hello/hello.clo", duration: 13, caption: "The smallest program" },
    { id: "hello-emu", kind: "emulator", file: "examples/hello/hello.clo", frame: true },  // default 7s, 50 fps
    { id: "outro",     kind: "outro",    title: "Clorraine · early development", lines: ["github.com/OpenLorraine"], duration: 7 },
  ],
});
```

```shell
make demo                 # render demo/story.ts
make demo-check           # tsc --noEmit + node --test (coverage)
clorraine-demo new examples/hello/hello.clo
clorraine-demo render demo/story.ts --only emulator --out demo/emulator.mp4
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Pipeline | TypeScript orchestrator over VHS, ffmpeg, Chrome, FS-UAE (AR-1) |
| Source viewer | Shiki + a Clorraine TextMate grammar (AR-2) |
| Scene model | card / outro / code / terminal / emulator with captions (AR-6, AR-14) |
| Composition | 0.5s crossfade between adjacent scenes; 1100x700@50 silent (AR-16, AR-18, AR-29) |
| Emulator capture | Real-time window video at 50 fps; per-scene length default 7s (AR-29) |
| Output media | generated + gitignored; poster PNG committed (AR-8, AR-13) |
| Support surface | `tools/demo/` + `shiki` + Chrome, approved (AR-15) |

## Related Files

- `tools/demo/**` (new tool: `src/`, `grammars/`, `assets/`, `test/`, `package.json`)
- `demo/story.ts` (new), `demo/record-emulator.sh` (refactored), `demo/clorraine.vim` (kept)
- `demo/story.sh`, `demo/show-source.sh`, `demo/tape/*.tape` (deleted)
- `demo/demo-full.mp4`, `demo/demo.gif`, `demo/emulator.mp4` (untracked), `demo/poster.png` (new)
- `Makefile` (`demo`, `demo-check`), `.gitignore`, `README.md`, `AGENTS.md`
- `plans/00-roadmap.md` (`PLAN-09`)
