# Story, Scaffold, and Integration: Clorraine Demo Video Toolkit

> **Document**: 03-04-story-and-scaffold.md
> **Parent**: [Index](00-index.md)

## Overview

This component migrates the Clorraine story from `demo/story.sh` to `demo/story.ts`,
adds the `new <file.clo>` scaffold, wires the `Makefile` targets, and updates the README,
AGENTS.md, and `.gitignore`. It also lists the files that are removed.

## The Clorraine story (`demo/story.ts`)

`demo/story.ts` is the direct successor of `demo/story.sh`: the same chapters in the same
order, expressed as scenes. Cards use the text from `demo/story.sh:31-78`; code chapters
replace the Vim tapes with `code` scenes; command chapters become `terminal` scenes; the
emulator chapters become `emulator` scenes with `frame: true`.

| Order | id | kind | Source / text |
| ----- | -- | ---- | ------------- |
| 1 | `intro` | card | `CLO_*` (`demo/story.sh:32-35`) |
| 2 | `hello-src` | code | `examples/hello/hello.clo`, caption `CAPTION_HELLO` |
| 3 | `hello-emu` | emulator | `examples/hello/hello.clo`, caption `CAPTION_HELLO_EMU` |
| 4 | `hello-explain` | card | `HELLO_EXPLAIN_*` |
| 5 | `real` | card | `REAL_*` |
| 6 | `source` | code | `spec/tests/corpus/string-parser/program.clo`, caption `CAPTION_SOURCE` |
| 7 | `own-explain` | card | `OWN_*` |
| 8 | `ownership` | terminal | `make -s run FILE=examples/ownership-fail.clo`, caption `CAPTION_OWNERSHIP` |
| 9 | `build-explain` | card | `BUILD_*` |
| 10 | `run` | terminal | `make -s run FILE=…/string-parser/program.clo PROFILE=hosted-amiga`, caption `CAPTION_RUN` |
| 11 | `emulator` | emulator | `spec/tests/corpus/string-parser/program.clo`, caption `CAPTION_EMULATOR` |
| 12 | `draw-explain` | card | `DRAW_*` |
| 13 | `graphics` | emulator | `spec/tests/corpus/graphics/program.clo`, caption `CAPTION_GRAPHICS` |
| 14 | `los` | card | `LOS_*` |
| 15 | `outro` | outro | `OUTRO_*` |

Config outputs:

- `out`: `demo/demo-full.mp4`
- `gif`: `demo/demo.gif` over `[intro, hello-src, hello-emu, hello-explain]` (AR-17)
- `poster`: `demo/poster.png`
- `make demo` then runs a second render, `render demo/story.ts --only emulator --out
  demo/emulator.mp4`, to keep the single-chapter file from today (PF-002).

The chapter captions for the emulator scenes must not say "real Amiga"; they use the
FS-UAE wording already enforced in `demo/story.sh:78`.

## `new <file.clo>` scaffold

`scaffold.ts` writes `demo/<stem>.demo.ts`:

```ts
import { defineDemo } from "../tools/demo/src/index.ts";

export default defineDemo({
  out: "demo/out/<stem>.mp4",
  poster: "demo/out/<stem>.png",
  scenes: [
    { id: "intro",  kind: "card",     title: "<derived>", duration: 6 },
    { id: "src",    kind: "code",     file: "<file>", duration: 12 },
    { id: "run",    kind: "terminal", commands: ["make -s run FILE=<file> PROFILE=hosted-amiga"], duration: 14 },
    { id: "emu",    kind: "emulator", file: "<file>", frame: true },
    { id: "outro",  kind: "outro",    title: "Clorraine", lines: ["github.com/OpenLorraine"], duration: 6 },
  ],
});
```

Title derivation (AR-14): the first leading `//` comment line of the source, trimmed of
the leading `//` and any trailing period; if there is none, the file stem. For
`examples/hello/hello.clo:1` (`// The smallest Clorraine program: print one templated
line.`) this yields `The smallest Clorraine program: print one templated line` (PF-015).

`new` only writes the config; it does not render. It refuses to overwrite an existing
file unless `--force` is passed.

## Integration

### Makefile

| Target | Behavior |
| ------ | -------- |
| `demo` | `npm ci` in `tools/demo/` when `node_modules` is absent (AR-25); then `node tools/demo/src/cli.ts render demo/story.ts`; then `render demo/story.ts --only emulator --out demo/emulator.mp4` (PF-002) |
| `demo-check` | `tsc --noEmit` and `node --test --experimental-test-coverage` in `tools/demo/` (AR-24, PF-020) |

`demo` and `demo-check` are separate from `check`/`verify`; `demo` requires the emulator
environment (AR-12), and neither is added to `make check` or `make verify`.

### Files removed

- `demo/story.sh`
- `demo/show-source.sh`
- `demo/tape/hello-source.tape`, `demo/tape/ownership.tape`, `demo/tape/run.tape`,
  `demo/tape/source.tape`
- tracked result media: `demo/demo-full.mp4`, `demo/demo.gif`, `demo/emulator.mp4`

### Files kept

- `demo/clorraine.vim` (AR-11)
- `demo/record-emulator.sh`, refactored into a callable recorder (03-02)

### `.gitignore`

Add:

```
# Generated demo media (do not commit)
/demo/*.mp4
/demo/*.gif
/demo/out/
/build/demo/
```

`demo/poster.png` is intentionally **not** ignored (AR-13).

### README and AGENTS.md

- `README.md`: replace the `demo/demo.gif` embed with `demo/poster.png`; state that
  `make demo` regenerates the video and that the featured files are not committed.
- `AGENTS.md`: add `tools/demo/` to the layout, `make demo`/`make demo-check` to the
  commands, and a line on the generated-media policy.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `new` target exists | Refuse unless `--force`; print the path; exit 2 | AR-14 |
| Scaffold source has no leading comment | Use the file stem as the title | AR-14 |
| `node_modules` install fails in `make demo` | Surface the npm error; exit 1 | AR-25 |
| `demo` run without an X display / Kickstart | Fail fast in the recorder, as today | AR-12 |

## Testing Requirements

- A test that `story.ts` validates and contains the 15 chapters with the expected kinds.
- A scaffold test for title derivation from a comment vs the filename (ST-9).
- A Makefile test that `demo` installs deps when `node_modules` is absent (ST-12).
