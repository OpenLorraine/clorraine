# Requirements: Amiga RTG — Picasso96 on UAEGFX

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

This feature adds the last optional Amiga graphics capability: a small,
documented Picasso96 surface for opening an RTG screen, discovering modes,
allocating an offscreen bitmap, drawing and reading pixels, and blitting a
bitmap to the screen. It is gated headlessly on an A1200/KS3.1 with FS-UAE's
`uaegfx` card and the Aminet Picasso96 2.0 runtime.

The design is deliberately narrower than the Picasso96 API. It exposes the
operations the gate can prove and leaves PIP/overlay, CLUT palette editing,
CyberGraphX, real-card drivers, and text/font rendering out of scope. This is a
capability row, not a header dump.

## Functional Requirements

### Must Have

- **R1 — Module and profile.** Add `std.amiga.picasso96` with opaque
  `RtgScreen` and `RtgBitmap` handles. The module is available only on
  `hosted-amiga`; `host`, `hosted-amiga-13`, and `freestanding` reject it with
  `E11200` and emit no C. [AR-1, AR-4, AR-14]
- **R2 — Library and board discovery.** `rtgBoards()` opens
  `Picasso96API.library` version 2 and returns the P96 board count, or 0 when
  the library or board is absent. The shim must not trap on a missing library.
  [AR-1, AR-2, AR-10]
- **R3 — Mode discovery.** `rtgMode(width, height, depth)` returns a P96
  display ID for the best matching 32-bit truecolor mode, or 0 when no mode
  matches; `depth` must be 32. `rtgModeWidth`, `rtgModeHeight`,
  `rtgModeDepth`, and `rtgModeFormat` query the selected mode through P96
  attributes. [AR-6, AR-7]
- **R4 — Screen lifecycle.** `openRtgScreen(width, height, depth, title)`
  opens a P96 screen with `P96SA_Quiet` and `P96SA_ShowTitle = FALSE`, stores
  the screen and selected mode in an opaque handle, and returns `null` on
  failure. `closeRtgScreen(take)`
  closes exactly once and is also the generated drop;
  `rtgScreenWidth/Height/Depth/Format/Mode` query a live screen. [AR-10,
  AR-14]
- **R5 — Offscreen bitmap lifecycle.** `allocRtgBitmap(width, height, depth)`
  allocates a P96 user-private 32-bit bitmap with `BMF_USERPRIVATE`, owns its
  `RastPort`, and returns `null` on failure. `freeRtgBitmap(take)` frees
  exactly once and is also the generated drop;
  width/height/depth/format/bytes-per-row query the bitmap through
  `p96GetBitMapAttr`. [AR-6, AR-14]
- **R6 — Pixel operations.** `rtgScreenWritePixel`, `rtgScreenReadPixel`,
  `rtgScreenFillRect`, and their bitmap counterparts validate the handle,
  coordinates, and rectangle. They use ARGB colors; a failure returns `false`
  or 0. Screen fills use `p96RectFill`; bitmap fills use a bounded
  `p96WritePixel` loop, the proven path for a user-private bitmap. The demo
  writes and fills the top-left at (20,20)-(30,30), checks (25,25), and also
  fills (300,300)-(310,310) outside the capture. [AR-7, AR-8, AR-10]
- **R7 — Blit and readback.** `rtgScreenBlitBitmap` copies an offscreen bitmap
  to the screen with `BltBitMapRastPort`. `rtgScreenReadPixels` and
  `rtgBitmapReadPixels` fill a caller-provided `u8[]` with RGB bytes through
  `p96ReadPixelArray`; the buffer length must be at least
  `width * height * 3`. The demo writes the probe pixels at (200,200), blits
  to (210,210) for the capture and to (10,10) to prove the full display clip,
  and writes a P6 PPM from a 64x64 region at (200,200), so the capture contains
  the screen pixels and the blitted bitmap. The demo also reads a 32x32 RGB region from the offscreen
  bitmap and checks a pixel, so the bitmap readback path is exercised. [AR-9]
- **R8 — Headless gate.** Add `make sdk-a1200-rtg`: stage a clean Workbench
  3.1 directory HD, install the P96 runtime and the `uaegfx` monitor, boot
  A1200/KS3.1 under FS-UAE with `--graphics-card=uaegfx`, run the demo, and
  assert both the report line and the PPM pixels. The report includes the
  screen, bitmap, mode, fill, and bitmap-readback accessors; the impl suite
  pins the missing-library guard. The gate must not open a host window.
  [AR-2, AR-5, AR-9]
- **R9 — Registry truth.** Replace the old `SDK-OPT-004` placeholder with a
  new optional bundle `10-rtg` and this full row:
  `SDK-RTG-001`, bundle `10-rtg`, area `rtg`, capability
  `Picasso96 RTG (UAEGFX)`, priority `should`, tier `optional`, min OS `3.0`,
  min hardware `AGA/68020/2M`, status `planned` before the gate and `done`
  after it, depends `-`, acceptance `a1200-rtg`, evidence
  `stdlib/std.amiga.picasso96.clo; amiga_rtg_spec_test;
  amiga_rtg_impl_test; A1200/KS3.1 rtg: ok boards 1 mode 640x480x32 ...`,
  spec `spec/appendix-stdlib.md`, notes
  `Bundle 10; Picasso96 2.0 Aminet; UAEGFX; PPM capture`.
  Regenerate coverage and replace the issue #11 placeholder body. [AR-3]
- **R10 — Documentation.** Add appendix sections to
  `spec/appendix-stdlib.md` and `spec/appendix-amiga.md`, a
  `docs/known-limitations.md` RTG subsection, the `AGENTS.md` command and
  layout entries, the programme index, the roadmap row, and a
  `THIRD_PARTY_NOTICES.md` entry naming the P96 authors, Aminet package
  version, CC-BY 4.0 source, and the vendored-header modification. [AR-15]

### Should Have

- **R11 — Toolchain parity.** The RTG shim must compile warning-free with both
  m68k GCC and vbcc. GCC uses the fixed-argument `LP*` call macros; the full
  P96 varargs macros are omitted because vbcc rejects them. vbcc uses the P96
  prototypes and links through the SDK `amiga.lib` P96 stubs (build-only; the
  runtime gate is GCC). `corpus-amiga` gets an explicit RTG build-only leg for
  both toolchains. [AR-12]
- **R12 — Deterministic failures.** Every shim entry checks null handles,
  dimensions (1..4096), depth (32), buffer sizes, and rectangle ordering
  before touching P96. Width 4096 x 3 bytes stays within the signed `WORD`
  `RenderInfo.BytesPerRow`; no wider row is accepted. No unbounded loops or
  allocations are driven by caller input. [AR-10]
- **R13 — Test integrity.** Every public export appears in the appendix check;
  the profile-rejection test fails without the profile gate; the shim pins
  fail without the named P96 calls; the runtime gate fails if the report or
  any checked PPM pixel is wrong. [AR-13]

## Acceptance Criteria and Review

- **AC-1:** `std.amiga.picasso96` is registered and rejected with `E11200` on
  host, `hosted-amiga-13`, and freestanding.
- **AC-2:** The hosted-amiga source emits the `cl_rtg_*` calls and builds
  warning-free with m68k GCC and vbcc.
- **AC-3:** `make sdk-a1200-rtg` reports
  `rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1` and the PPM
  contains the red/green/blue pixels and the yellow/blue blit.
- **AC-4:** `SDK-RTG-001` is `done`; Bundle 9 remains 20/20; Bundle 10 is 1/1;
  coverage and issue #11 are regenerated.
- **AC-5:** `make check`, `make asan` (host suites), `make corpus-amiga`, and
  full `make verify` pass.
- **AC-6:** Independent correctness, memory-safety, and emission-integrity
  reviews are run; every critical/major finding is fixed and the affected
  gate re-run.

### Won't Have (Out of Scope)

- CyberGraphX/CGX4 or a dual-stack abstraction (explicitly dropped 2026-11-06).
- Real Picasso II/IV, CyberVision, or other physical card drivers.
- PIP/video overlay, hardware sprites, CLUT palette editing, or screen-mode
  preference editing.
- Font/text rendering, windowed RTG UI, or a Workbench replacement screen.
- Direct VRAM locking or a public `RenderInfo`/`BitMap` struct.
- Any language, grammar, diagnostic, golden, or ABI change.

## Technical Requirements

### Performance

- The gate reads a 64x64 RGB region (12,288 bytes), not a full 640x480
  framebuffer, so the PPM check is fast and deterministic.
- Bitmap fills are bounded to 1,048,576 pixels; larger rectangles return
  `false` instead of running an unbounded per-pixel loop.
- The shim opens the P96 library once per process and reuses the base.
- No new runtime work occurs when the module is not used beyond the code size
  of the suffix; the P96 library is opened lazily.

### Compatibility

- Baseline: A1200/KS3.1, 68020, 2 MB chip + 0 fast memory, FS-UAE `uaegfx`.
- P96 requires Kickstart 3.0+ and 68020+; `hosted-amiga-13` is rejected by the
  module profile gate even though the C shim is never appended there.
- The vendored header defines only the P96 constants, structs, and library
  call macros the shim uses. It has no link-time dependency on a P96 library
  outside the Amiga guest.
- The suffix is appended after the generated program, so P96 headers cannot
  rewrite user identifiers.

### Security

- The gate uses only user-provided/local media; no archive or decrypted ROM is
  committed.
- Every buffer copy is bounded by the caller's length and the requested
  rectangle.
- The host PPM parser reads only the 64x64 artifact and validates the header,
  maximum value, and pixel count before indexing.
- No secrets, credentials, or user data are written to the report.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Stack | P96 / CyberGraphX / both | P96 | user-approved Aminet package; one honest stack | AR-1 |
| Card | UAEGFX / real card | UAEGFX | reproducible headless gate; spike proven | AR-2 |
| Registry | keep in Bundle 9 / new bundle | new `10-rtg` | Bundle 9 stays 20/20; RTG gets its own plan and evidence | AR-3 |
| Module name | `rtg` / `picasso96` | `picasso96` | library-specific contract | AR-4 |
| Depths | 32-bit / multi-depth / CLUT | 32-bit truecolor | gate-proven and no unproven mode claims | AR-7 |
| Bitmap fill | p96RectFill / pixel loop | pixel loop | spike showed a pure-blue fill mismatch on user bitmaps | AR-8 |
| Capture | CSCR / PPM | PPM | chunky RTG screen; RGB readback is the honest artifact | AR-9 |
| Header | external SDK / vendored | vendored | deterministic build; attributed CC-BY definitions | AR-11 |
