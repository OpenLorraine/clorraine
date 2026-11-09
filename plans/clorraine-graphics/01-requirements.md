# Requirements: Clorraine Amiga Graphics

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Add a small, safe graphics surface to the hosted-amiga standard library so a
Clorraine program can open a custom Amiga screen and draw into it. Two modules
split the concerns: `std.amiga.intuition` owns the `Screen` handle lifecycle,
and `std.amiga.graphics` owns drawing. Both reach the profile C shim through
`ffi`, so application code never writes `ffi` and never touches `unsafe`.

The feature is a standard-library and runtime addition. It changes no grammar or
semantics; the frozen language specification stays untouched, and only the
standard-library appendix and the Amiga platform appendix are updated, plus one
new corpus program (AR-3).

## Functional Requirements

### Must Have

- **R1** — `std.amiga.intuition` exports an owned `Screen` handle and
  `openScreen(width: i32, height: i32, depth: u8, title: cstring): Screen?`. A
  failure returns `null`. (AR-6, AR-8, AR-16)
- **R2** — The `Screen` handle is released by the generated drop through a
  null-safe `cl_close_screen` deleter; the screen closes exactly once. (AR-6, AR-17)
- **R3** — `std.amiga.graphics` exports `setColor(pen, r, g, b)`,
  `fill(x1, y1, x2, y2, pen)`, `draw(x1, y1, x2, y2, pen)`, and `clear(pen)`;
  each takes `borrow screen: Screen`. (AR-7, AR-12, AR-15, AR-18)
- **R4** — Coordinates are `i32`, the pen index is `u8`, and `setColor`
  components are `u8` (0-255) mapped to the Amiga 4-bit palette. (AR-12)
- **R5** — The runtime shim opens `graphics.library` and `intuition.library`
  once, on the first screen open, and drives `OpenScreen`/`CloseScreen`,
  `SetRGB4`, `SetAPen`, `RectFill`, `Draw`, and a clear fill. (AR-9, AR-23)
- **R6** — A graphics corpus program (`spec/tests/corpus/graphics/`) draws,
  prints `graphics: ok`, holds the screen briefly, and exits 0; on the host
  profile it is rejected with `E11200`. (AR-10, AR-20, AR-28)
- **R7** — `make verify` runs the graphics program under FS-UAE and diffs its
  captured output against the golden. (AR-19, AR-21)
- **R8** — The surface is documented in `spec/appendix-amiga.md` and
  `spec/appendix-stdlib.md`, and the demo story video gains a graphics chapter. (AR-27, AR-28)

### Should Have

- **R9** — The corpus README records the construct coverage and the hand-check
  record, matching the existing corpus convention. (AR-10, AR-27)

### Won't Have (Out of Scope)

- Intuition windows and the event loop (AR-29).
- Text and fonts (AR-30).
- Bitmaps, off-screen images, and blit (AR-31).
- Sprites, copper, and custom-chip display setup (AR-32).
- `SetRGB32` and palettes beyond 4 bits (AR-33).
- A screenshot-golden diff harness (AR-34).

## Technical Requirements

### Compatibility

- `hosted-amiga` only. The existing `Profile::supports_module` already rejects
  `std.amiga.*` on `host` and `freestanding`, so the host profile emits `E11200`
  for the graphics program. (AR-20, AR-25)
- The emitted C and shim must stay within the strict C99 subset and compile
  clean under the profile's `-Wall -Wextra -Werror`. (AR-24)

### Security

- Application code never writes `ffi` and never uses `unsafe`; the shim holds
  the raw pointers and library bases. (AR-6, AR-23)
- `openScreen` is the only allocation-like entry; a failure returns `null` and
  never a partially initialised handle. (AR-16)
- No new input surface, no file or network access, no user-controlled sizes
  beyond the numeric screen dimensions, which `OpenScreen` validates. (AR-14)
- The `title` is a `cstring` the caller owns and must keep alive for the
  screen's lifetime; the wrapper does not copy it. (AR-8; PF-004)

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Display model | screen only / window only / both | screen only | Smallest visual surface; the spike validated it; windows need an event loop | AR-2, AR-4 |
| Module split | one module / `intuition`+`graphics` | `intuition`+`graphics` | Mirrors AmigaOS and appendix-amiga.md; separates lifecycle from drawing | AR-5 |
| Handle model | owned `Screen` / exposed `RastPort` | owned `Screen`, hidden `RastPort` | Reuses the validated contract; keeps the raw `RastPort` inside the shim | AR-6, AR-18 |
| Drawing set | fill/line/clear / add text / add bitmaps | fill/line/clear | The smallest set that makes a real animated demo | AR-2, AR-4 |
| `openScreen` shape | positional / `ScreenSpec` | positional | Fewer moving parts; matches the spike | AR-8, AR-26 |
| Verification | FS-UAE smoke / screenshot diff | FS-UAE smoke + sentinel | Reuses the interrupt-counter pattern; avoids a new harness | AR-21, AR-26 |

> **Traceability:** Every scope decision cites the Ambiguity Register entry that
> resolved it; see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] `make check` is green, including the new `graphics_spec_test.rs` and `graphics_impl_test.rs`.
2. [x] `make verify` runs the graphics corpus program under FS-UAE and its captured output matches `spec/tests/corpus/graphics/expected-output.txt`.
3. [x] Under `--profile host`, the graphics corpus program is rejected with `E11200`.
4. [x] The emitted C and the hosted-amiga shim compile with zero warnings under the profile's flags.
5. [x] `spec/check.sh` stays green; the two appendices document the surface.
6. [x] `demo/story.sh` produces the story with a graphics chapter.
7. [x] `plans/00-roadmap.md` carries the `PLAN-08` row linking this plan.
