# Findings: Amiga SDK Bundle 1 — Native Video

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Pre-implementation spike

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| F-1 | `LoadView` alone does not change the display; the view needs `MakeVPort` and `MrgCop` first. | The first C spike showed only the shell; after adding `MakeVPort`/`MrgCop`, the same red plane displayed. | The design uses the full pipeline and saves `GfxBase->ActiView` for restore. |
| F-2 | `LoadView(NULL)` is not the restore path; it disables the view. | The working spike saves `GfxBase->ActiView` before loading and restores that pointer. | `cl_video_display_free` restores the saved view. |
| F-3 | The ADF reaches the startup sequence in about 18–20 seconds; the full Boing run reaches its display around 50 seconds and prints the checksum around 65 seconds. | Hello/graphics captures at 8–18 seconds showed the shell; the Boing poll saw a live frame at 50 and 60 seconds and the checksum line at 65 seconds. | The Boing smoke polls from 30 to 100 seconds in five-second steps. |
| F-4 | The existing Intuition graphics path is healthy on 1.3. | `spec/tests/corpus/graphics/program.clo` built with `nix13` showed its color bars at 30 seconds. | Bundle 1 leaves the Intuition surface unchanged. |
| F-5 | The masked blit needs the documented cookie-cut minterm `0xE0` and a single-plane mask; `BltBitMap`'s `mask` argument is a plane-enable mask, not an image mask. | `BltMaskBitMapRastPort` with `0xE0` produced the ball in the real run; the plain `BltBitMap` copy used `0xC0`/`0xFF`. | `bitmapBlitMasked` uses `BltMaskBitMapRastPort` with a temporary `RastPort`. |
| F-6 | The final frame checksum is stable across runs. | Two independent FS-UAE runs produced `boing: checksum 0x2CF3DCFD` and the same line crop. | The smoke compares the crop byte-exact. |
| F-7 | The A500 demo sustains the 50 Hz frame budget. | The final shell line reports `boing: frames 400 ticks 400`; at 50 ticks per second that is 400 frames in 8 seconds. | The late screenshot timeline is the ADF boot/load, not the animation. SDK-BASE-010 is evidenced. |
| F-8 | The user copper list must be attached after `InitVPort`; `InitVPort` clears `UCopIns`. | The first implementation attached it in `displaySetCopper` before the first flip; the held frame had 15 colors and no gradient. After storing the lists and attaching them after `InitVPort`, the held frame has 54 colors and the smoke compares it byte-exact. | The display stores `user_copper[2]`; the first flip attaches them before `MakeVPort`. |
| F-9 | The plain blit helpers may return before the blitter has finished. | The Autodoc for `BltBitMap` does not guarantee completion; the shim now calls `WaitBlit()` after both blit forms. | Source and destination planes are safe to free or inspect on return. |

## Red phase

The specification tests were written before the modules existed:

```text
error E11102: 'ChipBuffer' is not exported from module 'std.amiga.video'
error E11102: 'Bitmap' is not exported from module 'std.amiga.video'
error E11102: 'newBitmap' is not exported from module 'std.amiga.video'
...
test result: FAILED. 0 passed; 6 failed
```

The impl test failed for the same unresolved-module reason. The host math
case failed because `std.math` was absent. The red log is in the run output,
not committed.

## Green phase

| Gate | Result |
|---|---|
| `cargo test --test amiga_video_spec_test` | 7 passed (host/3.1/A500 compile, host rejection, `nix13`/`nix20` cross-builds, host math values, Exec still gated) |
| `cargo test --test amiga_video_impl_test` | 3 passed (wrapper routing, null-guarded drops, no Intuition injection) |
| `make check` | exit 0; 93 test groups before the bundle; new suites green |
| `make sdk-a500-boing` | OK; held display crop and checksum line matched; `boing: frames 400 ticks 400` |
| `make sdk-registry` | OK (79 capabilities, 10 bundles; Bundle 1 11/11) |

## Runtime evidence

| Evidence | Value |
|---|---|
| Live display | `docs/amiga-sdk/a500-boing.png` |
| Golden checksum line | `docs/amiga-sdk/a500-boing-line.png` |
| Checksum | `boing: checksum 0xF58C7129` |
| Frame budget | `boing: frames 400 ticks 400` (50 Hz at 50 ticks/s) |
| Held display crop | `docs/amiga-sdk/a500-boing-frame.png` |
| Machine | FS-UAE A500, real Kickstart 1.3 rev 34.5, decrypted licensed ROM |
| ROM SHA-1 | `891e9a547772fe0c6c19b610baf8bc4ea7fcb785` |

## Review ledger

The independent review found two critical and six major findings. All critical
and major findings were fixed and re-tested; the two remaining minor items are
accepted residuals.

| ID | Severity | Finding | Fix / disposition |
|---|---|---|---|
| C1 | Critical | 16/32-bit chip access used `offset + N`, which wraps near `0xFFFFFFFF`. | Bounds now use `offset > size || size - offset < N`; the Boing self-test exercises `0xFFFFFFFC` and `0xFFFFFFFE`. |
| C2 | Critical | `displaySetCopper` set `UCopIns` before `InitVPort`, which clears it, so the copper was not merged and leaked. | The display stores `user_copper[2]`; the first flip attaches them after `InitVPort` and before `MakeVPort`. The held frame and smoke now compare the display crop. |
| M1 | Major | Blits returned before the blitter finished. | `WaitBlit()` added after both blit forms; the design and requirements say so. |
| M2 | Major | Rectangle bounds could overflow, and the OCS blitter width limit was not enforced. | Subtraction bounds and a 992x1024 cap; the mask must be depth 1 and match the source. |
| M3 | Major | Replacing a queued copper leaked the previous lists. | The display frees the previous `user_copper` before storing the new pair. |
| M4 | Major | The API accepted AGA-only depth and oversized displays. | `openDisplay` is capped at 320x256x5; docs updated. `newBitmap` still allows offscreen bitmaps up to 1024x1024. |
| M5 | Major | The drop test did not pin the cleanup order or the chip/copper drops. | The impl test now checks copper, chip, display, back, front in the generated ladder. |
| M6 | Major | The smoke only counted colors and did not check the display. | The held frame crop is compared byte-exact to `a500-boing-frame.png`; the checksum line remains the content oracle. |
| m1 | Minor | `sqrt(INT_MAX)` and `cos(INT_MAX)` overflowed. | Initial estimate uses `value / 2 + value % 2`; `cos` reduces the angle before adding 64. Host test covers both boundaries. |
| m2 | Minor | The checksum seed omitted width/height. | Width and height are folded in before the plane bytes. |
| m3 | Minor | The masked-copy fixture mismatched source and mask sizes. | The full-surface fixture uses a 64x64 sprite and 64x64 mask; the shim requires depth exactly 1. |
| m4 | Minor | Flip failure was silent and `waitVBlank` could no-op. | `displayFailed` added; `waitVBlank` opens graphics first. |
| m5 | Minor | `copperColor` said "waits and writes". | Comment corrected. |
| m6 | Minor | `std.math` claimed every profile. | Comment says host and both hosted-Amiga profiles. |
| m7 | Minor | Plan/registry counts disagreed during the review. | Reconcile in the close-out task. |
| m8 | Minor | `displaySetColor` claimed a future display effect. | Comment says a post-flip call has no visible effect. |

Accepted residuals:

- `Display` borrows its bitmaps; the language cannot enforce that across a
  returned handle. The module and appendix document the drop order and the
  return hazard.
- The palette is static after the first flip. Per-line animation uses the
  copper; runtime palette rebuilding is out of scope.
