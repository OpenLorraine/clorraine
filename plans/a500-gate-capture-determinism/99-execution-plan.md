# Task: Pin the remaining A500 FS-UAE gates to a deterministic window

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 5/7 tasks (71%)
> **Reasoning**: low — gate harness only. Four shell scripts gain a pinned
> FS-UAE window, a capture-size assertion, and regenerated goldens; no compiler,
> runtime, ABI, stdlib, or product change.

## Objective

Deliver issue #34: remove the host-display-scale dependence from the four
sibling A500 screenshot gates. `sdk-a500-smoke` already pins its window to
640x256, asserts the capture size, and crops at a fixed window-relative offset.
`a500-boing`, `a500-sprites`, `a500-paddle`, and `a500-text` still capture from
FS-UAE's default 960x540 window and crop at fixed `+160+88` offsets, so a
different desktop scale or FS-UAE default moves the captured content and
produces an unexplained AE mismatch.

**Smallest viable design:** pin the window every gate already relies on
(`--window-width=960 --window-height=540 --window-border=0`) in the four
scripts, assert the captured size with a clear error before any crop, keep the
existing window-relative crop rectangles, regenerate the committed goldens from
the pinned captures, and add one static specification test that requires the
hello gate and the four demo gates to pin a window, assert the capture size, and
crop inside the pinned window.

The two remaining A500 screenshot gates, `a500-assets` and `a500-exec`, are
outside this issue's named scope; they keep the same host-scale dependence and
are filed as follow-up #35 rather than receiving product work here.

The pinned geometry is the geometry the committed goldens were captured from:
FS-UAE's default is 960x540, the 320x256 demo view renders at 2x in a 640x512
area, and the 640x256 AmigaDOS shell fits at the same scale. Pinning it makes
the capture independent of the host without changing what the goldens show.

## Tasks

- [x] T-01 Add the gate-geometry specification test to
  `compiler/tests/amiga_a500_spec_test.rs`: a table of the five A500 screenshot
  gates with each gate's pinned window, asserting that the script passes the
  pinned width, height, and zero border, checks the captured size before
  cropping, and that every `convert ... -crop` rectangle fits inside the pinned
  window. Record the red run: the four sibling gates fail before the change.
- [x] T-02 Pin the window and zero border in `a500-boing-smoke.sh`,
  `a500-sprite-smoke.sh`, `a500-paddle-smoke.sh`, and `a500-text-smoke.sh`, and
  assert the captured size before the first crop. The error names the observed
  size and the pinned size so a future geometry change fails clearly.
- [x] T-03 Re-derive the crop offsets from the pinned capture, regenerate the
  four line goldens and the four frame/display goldens, and record each golden
  size and whether it changed. A changed golden is reviewed as a capture
  difference, not accepted silently.
- [x] T-04 Run the four gates on the desktop and on a private Xvfb display
  started inside the SDK image (the `sdk-a500-gui` pattern), and fix any
  difference between the two runs.
- [x] T-05 Run the independent correctness and Amiga platform reviews; fix
  every critical/major finding. Both reviews are clean; the two minor
  correctness findings and the four minor platform corrections are recorded
  in the Findings section.
- [ ] T-06 Run `make check` and the full `make verify` gate; record the results.
- [ ] T-07 Update the roadmap and close #34.

**Verify**: `make check`; `make sdk-a500-boing`; `make sdk-a500-sprites`;
`make sdk-a500-paddle`; `make sdk-a500-text`; the same four gates on a private
Xvfb display; `make verify`.

## Design constraints

| Constraint | Reason |
|---|---|
| Pinned window 960x540, border 0 | It is the observed FS-UAE default and the proven content geometry; the 320x256 demo view renders at 2x in a 640x512 area and the 640x256 shell fits, so pinning it preserves the golden content |
| Capture-size assertion before cropping | A window-size or default change then fails with the observed size instead of a large AE mismatch with no cause |
| Crops stay window-relative | No crop may depend on the host display or the WM decorations |
| Goldens regenerated from the pinned capture | The gate oracle must describe the pinned geometry, not an inferred one |
| `--window-border=0` | Removes FS-UAE's own themed border so the captured client area is exactly the pinned size; `import -window <id>` captures the client window, so WM decorations are outside the capture either way |
| No product, ABI, or stdlib change | The defect is in the gate harness, not the compiler or runtime |

## Known risks

| Risk | Mitigation |
|---|---|
| FS-UAE resizes its window when the Amiga screen mode changes | The size assertion catches it; the gate fails with the observed size and the geometry is revisited |
| WM decorations change the captured rectangle | `import -window <id>` captures the client window, so the WM frame is not part of the image; `--window-border=0` also removes FS-UAE's own themed border |
| The 960x540 window does not fit a small desktop | It fits the documented 1280x720 private Xvfb; the desktop run uses the current display as before |
| A pinned capture differs from the default capture | T-03 records every golden size and diff; a changed golden needs a capture-difference review |
| The issue text lists paddle's line crop as `540x22`; the script uses `430x22` and the golden is 430x22 | The script and golden are authoritative; the plan records the corrected value |

## Key decisions

| Decision | Outcome |
|---|---|
| Window geometry | 960x540 with no border, the standard the committed goldens were captured from |
| Line and frame crops | Keep the existing window-relative offsets; re-derive and verify them against the pinned capture |
| Golden scope | Compare all eight A500 line/frame goldens to the pinned capture and record the result; retain a committed file when only the PNG capture timestamp differs |
| Guard | One static specification test over the five named screenshot gates, so a removed pin, a size check moved after a crop, or a crop that leaves the window fails `make check`. A new gate must be added to the table; the two remaining capture gates are follow-up #35 |
| Xvfb proof | Start Xvfb inside the SDK image, as `sdk-a500-gui` does, and run the same four scripts against it |
| Specialist reviews | Correctness plus Amiga platform, because the change is an FS-UAE gate |

## Findings

### Red baseline

`cargo test --test amiga_a500_spec_test
the_a500_screenshot_gates_pin_their_capture_geometry` failed before the change.
The first red run used a combined three-flag assertion; the committed test
asserts each flag separately, so the failure was re-probed with the final test
and the HEAD scripts restored temporarily:

```
docker/scripts/a500-boing-smoke.sh must pin the FS-UAE window with "--window-width=960"
```

The test then passed with the four scripts pinned.

### Pinned geometry and crops

| Gate | Pinned window | Crops (all window-relative) |
|---|---|---|
| `a500-smoke` | 640x256 | line `215x8+169+44`, upscaled 2x |
| `a500-boing` | 960x540 | frame `408x438+418+102`, line `430x22+160+88` |
| `a500-sprites` | 960x540 | frame `640x512+160+14`, line `430x22+160+88` |
| `a500-paddle` | 960x540 | frame `640x512+160+14`, line `430x22+160+88` |
| `a500-text` | 960x540 | frame `408x438+418+102`, line `540x22+160+88` |

The test also rejects any crop rectangle whose right or bottom edge leaves the
pinned window.

### Golden verification

The pinned captures were compared pixel-exact against the committed goldens.
The frame evidence is the saved full-window `live.png` from the iteration that
matched, cropped with the gate's geometry; the loop overwrites `display.png`
after a match, so that file is not the matched frame.

| Golden | Size | AE vs pinned capture | Committed file |
|---|---:|---:|---|
| `a500-boing-frame.png` | 408x438 | 0 | retained |
| `a500-boing-line.png` | 430x22 | 0 | retained |
| `a500-sprites.png` | 640x512 | 0 | retained |
| `a500-sprites-line.png` | 430x22 | 0 | retained |
| `a500-paddle.png` | 640x512 | 0 | retained |
| `a500-paddle-line.png` | 430x22 | 0 | retained |
| `a500-text.png` | 408x438 | 0 | retained |
| `a500-text-line.png` | 540x22 | 0 | retained |

Copying the pinned crops over the committed files changed only the PNG
`tIME`/`tEXt` timestamp metadata. The `IHDR`, `cHRM`, `PLTE`, `bKGD`, and
`IDAT` chunks are byte-identical, so the committed files are retained:
regenerating them would record a new capture timestamp and nothing else. The
gates compare pixels, and every pixel is already the pinned capture's pixel.

### Desktop and Xvfb

All four gates passed on the desktop. All four also passed on a private
`:99` Xvfb started inside the SDK image at 1280x720x24, with `docker/.env`
temporarily pointed at that display and restored afterwards. The log ends with
`xvfb: all four gates passed`.

One paddle run that overlapped the benchmark trace missed the sixteen-second
held frame; the same gate passed immediately when run alone. That sensitivity
is pre-existing (the poll cadence and hold time are unchanged); the gate must
not share the host with a CPU-heavy trace.

### Review findings

The Amiga platform review reported no critical or major findings and four
minor corrections:

| ID | Finding | Resolution |
|---|---|---|
| AP-001 | The test and plan said "every A500 screenshot gate", but `a500-assets` and `a500-exec` also capture the unpinned window | Scoped the wording to the five covered gates and filed follow-up #35 for the two remaining gates |
| AP-002 | `a500-paddle` and `a500-text` did not list `identify` as a required host tool | Added `identify` to both prerequisite loops |
| AP-003 | The size failure named only the observed size, not the pinned size | Both sizes are now in the message: `(pinned 960x540)` |
| AP-004 | The recorded red message came from the earlier combined-flag assertion | Re-probed with the committed per-flag test; the plan records the actual message and the refactor |

The correctness review reported no critical or major findings and two minor
gaps:

| ID | Finding | Resolution |
|---|---|---|
| RV-001 | The test asserted the size check existed but not that it ran before the first crop | Added poll-loop and pre-crop position assertions; a probe that inserts a crop before the check now fails with `the capture size check must precede the first crop` |
| RV-002 | The Guard decision claimed a new gate would fail `make check`, but the test table is fixed | Reworded to the five named gates and the "add a row for a new gate" rule; the two remaining capture gates are #35 |

The reviewer also independently reproduced the red failure against the HEAD
scripts, probed both right- and bottom-edge crop overflow, confirmed the
eight goldens are AE=0 (frames from `live.png`, lines from `crop.png`), and
confirmed the PNG differences are metadata-only.

Reviewer confirmations: FS-UAE 3.1.66 in the pinned image exposes
`window_width`, `window_height`, and `window_border` and logs no unknown-option
warning; `import -window` captures the client area, so the saved captures are
960x540 and independent of the window position; an unmapped window makes
`import` fail and the loop continues, so the assertion does not false-fail
during initialization; all eight goldens are AE=0 against the pinned captures.

