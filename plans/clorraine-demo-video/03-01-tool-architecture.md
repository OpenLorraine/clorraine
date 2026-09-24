# Tool Architecture: Clorraine Demo Video Toolkit

> **Document**: 03-01-tool-architecture.md
> **Parent**: [Index](00-index.md)

## Overview

`@clorraine/demo` is a zero-build TypeScript CLI and library under `tools/demo/`. It
loads a scene script, validates it, renders each scene to a clip, and composes the
clips into the output video. It shells out to Chrome, VHS, ffmpeg, Docker, and FS-UAE;
it never reimplements video or terminal capture.

## Architecture

### Current Architecture

There is no architecture: `demo/story.sh` performs load-free, validate-free rendering
inline. The new tool introduces the layering below.

```
tools/demo/
├── package.json            # @clorraine/demo, ESM, bin: clorraine-demo
├── package-lock.json       # committed (AR-5)
├── tsconfig.json           # strict, noEmit, erasableSyntaxOnly
├── README.md
├── grammars/clorraine.tmLanguage.json   # 03-03
├── assets/
│   ├── card.html           # card/outro layout
│   ├── code.html           # Shiki source layout
│   ├── frame.html          # branded emulator frame
│   ├── demo.css            # shared brand styles
│   └── theme.json          # Shiki theme
├── src/
│   ├── cli.ts              # render | new | list
│   ├── index.ts            # public API: defineDemo, types
│   ├── scenes.ts           # scene types + validateConfig
│   ├── config.ts           # load .ts or .json config
│   ├── scaffold.ts         # new <file.clo>
│   ├── renderer/
│   │   ├── card.ts         # card/outro → clip
│   │   ├── code.ts         # code → clip
│   │   ├── terminal.ts     # terminal → tape → clip
│   │   ├── emulator.ts     # emulator → capture → framed clip
│   │   ├── poster.ts       # first frame → PNG
│   │   └── compose.ts      # normalize, caption, crossfade, concat, GIF
│   └── util/
│       ├── chrome.ts       # headless Chrome frame capture (AR-20)
│       ├── ffmpeg.ts       # ffmpeg/ffprobe wrappers
│       ├── shiki.ts        # Shiki + Clorraine grammar
│       ├── vhs.ts          # tape generation + vhs invocation
│       ├── proc.ts         # spawn helper (argument arrays)
│       ├── paths.ts        # repo-root resolution + escape guard
│       └── log.ts
└── test/                   # 07-testing-strategy.md
```

### Proposed Changes

- `demo/story.ts` imports `defineDemo` from `tools/demo/src/index.ts`.
- `demo/record-emulator.sh` is refactored (03-02) and invoked by `renderer/emulator.ts`.
- The `Makefile` gains `demo` and `demo-check` (03-04).
- `demo/story.sh`, `demo/show-source.sh`, and `demo/tape/*.tape` are deleted (03-04).

## Implementation Details

### New Types/Interfaces

```ts
/** A complete demo description. */
export interface DemoConfig {
  out: string;                    // output MP4 path, repo-relative
  scenes: Scene[];                // at least one
  poster?: string;                // optional PNG of the first card (AR-13)
  gif?: { out: string; sceneIds: string[] };  // opening loop (AR-17)
  width?: number;                 // default 1100 (AR-18)
  height?: number;                // default 700 (AR-18)
  fps?: number;                   // default 50, PAL rate (AR-18, AR-29)
  transition?: number;            // crossfade seconds, default 0.5 (AR-16)
}

export interface SceneBase {
  id: string;                     // unique within a config
  caption?: string;               // strip caption text
  duration?: number;              // seconds; per-kind default (all 7, except code 12 / terminal 14)
}

export type Scene =
  | (SceneBase & { kind: "card"; title: string; subtitle?: string; lines?: string[] })
  | (SceneBase & { kind: "outro"; title: string; lines: string[] })
  | (SceneBase & { kind: "code"; file: string; scroll?: boolean; highlight?: number[] })
  | (SceneBase & { kind: "terminal"; commands: string[] })
  | (SceneBase & { kind: "emulator"; file: string; frame?: boolean });  // hosted-amiga only (AR-12)

/** identity helper so a config module type-checks without a build step. */
export function defineDemo(config: DemoConfig): DemoConfig;

/** Throws a ConfigError with every problem found, not just the first. */
export function validateConfig(config: unknown): asserts config is DemoConfig;
```

A scene `duration` is optional and defaults per kind — **7 s for `emulator`** (customizable,
AR-29), 7 s for `card`/`outro`, 12 s for `code`, and 14 s for `terminal`.

`validateConfig` enforces: at least one scene; unique non-empty `id`s; a present
`duration > 0`; the referenced files exist and are readable; every `gif.sceneIds`
entry names a scene (PF-011, PF-012, PF-019).

### CLI Contract

| Command | Behavior |
| ------- | -------- |
| `clorraine-demo render <config>` | Render the config; write `out`, optional `poster`/`gif` |
| `clorraine-demo new <file.clo>` | Write `demo/<stem>.demo.ts` from the scaffold defaults (AR-14) |
| `clorraine-demo list <config>` | Print the scene ids, kinds, and durations |
| `clorraine-demo render <config> --only <id>[,<id>] [--out <path>]` | Render selected scenes; `--out` overrides the config's `out` (PF-002) |

`render` exits non-zero on any validation or tool error and prints one clear line.

### Config Loading

- `.ts` is loaded with `await import(pathToFileURL(abs))` and must default-export the config.
- `.json` is read and `JSON.parse`d; the same `validateConfig` runs for both (AR-21).
- All scene `file`/`out`/`poster` paths are resolved against the repository root and
  rejected with a clear error if they escape it (`util/paths.ts`). The root defaults to
  the repository root and is overridable with `--root`/`CLORRAINE_DEMO_ROOT` so tests can
  run against a temp tree (PF-020). The guard canonicalizes the nearest existing ancestor
  with `realpath`, rejects absolute paths, and applies to write paths (`out`, `poster`)
  as well as reads (PF-016).

### Integration Points

- Chrome/VHS/ffmpeg via `util/*`; Chrome is located through `CHROME_BIN` (default
  `google-chrome`) so the missing-Chrome path is testable (PF-009).
- The emulator renders only the `hosted-amiga` profile via `demo/record-emulator.sh`
  (no `profile` option; PF-021).
- `render` returns an exit code; `make demo` wraps it.

### Toolchain Settings

- `tsconfig.json`: `"module": "nodenext"`, `"target": "es2022"`,
  `"strict": true`, `"noEmit": true`, `"allowImportingTsExtensions": true`,
  `"verbatimModuleSyntax": true`, `"erasableSyntaxOnly": true`.
- `erasableSyntaxOnly` guarantees the source stays runnable by Node's type stripping
  (no enums/namespaces/parameter properties). Type-checking is `tsc --noEmit` (AR-19).

## Code Examples

### Example 1: A minimal config

```ts
import { defineDemo } from "../tools/demo/src/index.ts";

export default defineDemo({
  out: "demo/out/hello.mp4",
  poster: "demo/out/hello.png",
  scenes: [
    { id: "intro", kind: "card", title: "Hello, Clorraine", subtitle: "A first program", duration: 6 },
    { id: "src", kind: "code", file: "examples/hello/hello.clo", duration: 12, caption: "The smallest program" },
    { id: "emu", kind: "emulator", file: "examples/hello/hello.clo", frame: true },
  ],
});
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Config fails validation | Collect all problems; print each with its scene `id`; exit 2 | AR-21 |
| A scene `file` is missing or unreadable | Reject at load with the path; exit 2 | PF-019 |
| A scene `file`/`out`/`poster` escapes the repo root | Canonicalize and reject with the offending path; exit 2 | PF-016 |
| Chrome missing/headless failure | Fail fast: name Chrome and the exact failure; exit 1 | AR-27 |
| The DejaVu fonts are missing | Fail fast, naming the font file | PF-018 |
| VHS missing or writes no output | Fail fast; keep the v0.11.0 warning | AR-3, PF-010 |
| ffmpeg/ffprobe failure | Surface the stderr tail; exit 1 | AR-18 |
| Emulator env missing (`docker/.env`, DISPLAY) | Fail fast, as `demo/record-emulator.sh:33-51` does today | AR-12 |

## Testing Requirements

- Unit tests for `validateConfig`, config loading, path resolution, and CLI arg parsing
  (07-testing-strategy.md ST-1, ST-6, ST-9).
- Integration tests for scene→clip rendering with fakes (ST-2..ST-5).
