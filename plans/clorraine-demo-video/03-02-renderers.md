# Renderers and Composition: Clorraine Demo Video Toolkit

> **Document**: 03-02-renderers.md
> **Parent**: [Index](00-index.md)

## Overview

Each scene kind renders to a short MP4 on the common canvas; `compose` then normalizes,
captions, crossfades, and concatenates them. HTML-rendered scenes use one headless
Chrome session; terminal scenes use a generated VHS tape; emulator scenes use FS-UAE
plus `x11grab` and a branded frame.

## Common canvas

All clips normalize to `width` x `height` (default 1100x700), 50 fps (PAL rate, AR-18 /
AR-29), `yuv420p`, H.264 CRF 20, audio-free, reusing the filter shape from
`demo/story.sh:120-125`. An 80 px bottom strip holds the caption, so captions never
cover content, as today.

## Renderers

### card / outro (HTML)

`assets/card.html` renders the title, subtitle, and lines, centered. The renderer
captures `round(duration * fps)` frames and encodes an MP4. This replaces the ffmpeg
`drawtext` `card()` at `demo/story.sh:133-145`.

### code (Shiki)

`assets/code.html` shows the file with line numbers, the filename in a header, and the
rendered Shiki HTML. Options:

| Option | Effect |
| ------ | ------ |
| `scroll: true` | Smoothly scroll from the top to the end across `duration` |
| `highlight: [n, …]` | Dim all lines except the listed ones |
| `caption` | Bottom strip text |

Highlighting uses the Clorraine grammar (03-03) and the brand Shiki theme. When
`scroll` is false and the file is taller than the viewport, the tool prints a warning
and clips the overflow (PF-019).

### terminal (VHS)

`util/vhs.ts` prepends `$HOME/go/bin` and `$HOME/.local/bin` to `PATH` (VHS is not on
the default PATH here; PF-010), then writes a tape to `build/demo/tape/<id>.tape`:

```
Output <build/demo/raw/<id>.mp4>
Set Shell "bash"
Set FontSize 16
Set Width <width>
Set Height <height - 80>
Set Padding 20
Set Theme "Dracula"
Set TypingSpeed 80ms

Type "<command 1>"
Enter
Sleep <duration>s
```

One `Sleep <duration>s` follows each command (PF-020). Command text is escaped so a `"`
or newline cannot break the tape or inject VHS directives (PF-017); the trust boundary
(developer-authored text) is documented at the writer.

The renderer runs `vhs <tape>` and keeps the v0.11.0 version check from
`demo/story.sh:110-113`. The canvas/tab settings mirror `demo/tape/*.tape`.

### emulator (FS-UAE + branded frame)

1. Build the program: `make -s build FILE=<file> PROFILE=hosted-amiga`.
2. Copy the built binary to `build/demo/bin/<id>` before launching. The build directory
   is still keyed by the source stem (`demo/record-emulator.sh:29-30`), so the copy makes
   the launched program unambiguous; scenes render sequentially and each build precedes
   its own run (PF-023 / cluster-2 PA-004).
3. Boot FS-UAE through `docker/scripts/run.sh` and **record the emulator window as
   real-time video** with ffmpeg `x11grab` at **50 fps** (the PAL rate, so animations and
   moving graphics stay smooth; AR-29), for the scene's `duration` (default 7 s) — this is
   a true video capture, not frame screenshots — reusing the window-discovery and geometry
   logic in `demo/record-emulator.sh:73-132`. FS-UAE 3.2.35 exposes no native video
   recorder (checked `fs-uae --help`), so window capture is the recording path.
4. Render `assets/frame.html` once to a PNG (a static branded bezel whose title is the
   source file stem, with the caption; PF-014) and overlay the capture inside it with
   ffmpeg (AR-28).

The recorder logic is refactored out of `demo/record-emulator.sh` into a callable that
takes `(source, output, duration, title, caption)`. The collision fix in step 2 is the
behavioral change; everything else is preserved.

## Chrome frame capture (`util/chrome.ts`)

- Launch once per scene:
  `google-chrome --headless=new --disable-gpu --hide-scrollbars
  --force-device-scale-factor=1 --no-first-run --no-default-browser-check
  --user-data-dir=<tmp> --remote-debugging-port=0 --window-size=<w>,<h>`.
- Connect with the Node built-in `WebSocket` to the DevTools endpoint (AR-20).
- Navigate to `file://<assets>/<page>.html?id=<sceneId>` and wait for `window.__ready`.
- For each frame: `Runtime.evaluate('window.render(<t>)')`, then
  `Page.captureScreenshot`; write the PNG to `build/demo/frames/<id>/<n>.png`.
- Encode with `ffmpeg -framerate <fps> -i <frames>/%05d.png …`.

Determinism: the page's appearance is a pure function of `t`; CSS `@keyframes` are
disabled; fonts are local DejaVu (AR-10); there is no network access. Each captured PNG
is asserted to be exactly `width`×`height`; a mismatch is scaled/padded before encoding
(PF-018), and a missing font file fails fast.

## Composition (`renderer/compose.ts`)

1. Normalize and caption every clip to the common canvas, then **trim each clip to its
   declared `duration`** (`ffmpeg -t`), because a terminal clip normally runs longer than
   its `duration` (typing + `Sleep`; PF-003).
2. Crossfade every adjacent scene with ffmpeg `xfade` for `transition` seconds
   (default 0.5; AR-16, PF-001 — a scene is a chapter). `xfade` takes exactly two inputs,
   so the filtergraph chains one `xfade` per boundary, each with an offset equal to the
   running output length minus `transition`; a config with one scene skips the chain
   (PF-004). Expected total duration = `sum(durations) − (n−1)·transition` for `n ≥ 2`.
3. Concat and write `out`, or `--out` when given (PF-002).
4. `gif`: encode the scenes named by `gif.sceneIds` (validated at load; PF-012) with the
   palettegen/paletteuse pipeline from `demo/story.sh:277-279` (AR-17).
5. `poster`: copy frame 0 of the first card scene, or of the first scene when no card
   exists (PF-012), to `poster` (AR-13).

## Caching

Per-scene clips are cached in `build/demo/raw/` and reused when the clip is newer than
its scene inputs, generalizing `record_tape` from `demo/story.sh:101-114`. The emulator
clip gets the same cache (today it has none).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Chrome fails to start | Kill the process; report the stderr tail; exit 1 | AR-27 |
| Page never signals `__ready` | Time out after 10s with the page URL; exit 1 | AR-20 |
| A captured frame is not `width`×`height` | Scale/pad to the canvas and warn | PF-018 |
| VHS writes no output | Report the v0.11.0 pin hint (VHS v0.12.0 is broken); exit 1 | AR-3 |
| FS-UAE window never appears | Report as `demo/record-emulator.sh:84-87` does today | AR-12 |
| An `xfade` total-duration check fails | Report the expected vs actual length; exit 1 | AR-16 |

## Testing Requirements

- Unit tests for tape generation, ffmpeg `xfade` argument construction, and the
  total-duration formula (07 ST-4, ST-10).
- A smoke render (opt-in) validated with `ffprobe` (07 ST-13).
