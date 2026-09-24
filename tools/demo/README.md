# @clorraine/demo

Render a Clorraine demonstration video from a scene script.

A scene script is a TypeScript module (or a JSON file with the same shape) that lists
scenes. Each scene becomes a clip: HTML cards and Shiki-highlighted source are rendered by
headless Chrome, command scenes are recorded in a real terminal with VHS, and emulator
scenes are recorded from FS-UAE in a branded frame. ffmpeg composes the clips into the
final video.

## Requirements

- Node.js >= 22.18 (runs TypeScript directly; no build step)
- Google Chrome (headless), ffmpeg, VHS v0.11.0, ttyd, an X display, and the DejaVu fonts
- For emulator scenes: the Docker SDK image, FS-UAE, and a real Kickstart

## Usage

```sh
npm install
node src/cli.ts render ../../demo/story.ts
node src/cli.ts new ../../examples/hello/hello.clo
node src/cli.ts list ../../demo/story.ts
```

The Clorraine story lives in `demo/story.ts`; `make demo` renders it, and `make demo-check`
runs the type check and the tests.

## Layout

| Path | Contents |
| ---- | -------- |
| `src/scenes.ts` | Scene types and `validateConfig` |
| `src/config.ts` | Loads a `.ts` or `.json` scene script |
| `src/cli.ts` | `render` / `new` / `list` commands |
| `src/renderer/` | One module per scene kind, plus composition |
| `src/util/` | Chrome, ffmpeg, VHS, Shiki, path, and process helpers |
| `grammars/` | The Clorraine TextMate grammar |
| `assets/` | HTML pages, CSS, and the Shiki theme |
| `test/` | `*.spec.test.ts` (from the spec) and `*.impl.test.ts` (internals) |
