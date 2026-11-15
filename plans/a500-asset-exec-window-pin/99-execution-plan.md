# Task: Pin the A500 asset and exec gates to a deterministic window

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 5/5 tasks (100%)
> **Reasoning**: low — two shell gates gain an explicit FS-UAE window pin, a
> size assertion, and the caller-display override; no compiler, runtime, ABI,
> stdlib, crop, or product change.

## Objective

Deliver issue #35. `a500-assets-smoke.sh` and `a500-exec-smoke.sh` captured the
FS-UAE window but never pinned it or asserted its size. They passed because
FS-UAE's default window is 960x540; a display-scale, window-default, or
position change would move the content and produce a large AE mismatch with no
cause. The exec gate also claimed it used the caller's private Xvfb display,
but both scripts overwrote `DISPLAY` from `docker/.env`.

**Smallest viable design:** add `--window-width=960 --window-height=540
--window-border=0` and a pre-crop `960x540` size assertion to both gates, add
the #36 caller-display override, keep the existing window-relative crops
(`414x410+418+98`, `620x22+160+88`), regenerate the three goldens from the
pinned captures, and extend the geometry and caller-display spec tests to the
two gates.

## Tasks

- [x] T-01 Extend `the_a500_screenshot_gates_pin_their_capture_geometry`,
  `the_a500_screenshot_gates_honour_a_caller_display`, and
  `the_a500_screenshot_gates_select_their_own_window` to seven gates. Record
  the red run: the assets gate had no window pin and no caller override.
- [x] T-02 Add the 960x540 pin, the pre-crop size assertion, `identify` as a
  required tool, the caller-display override, and the 90-poll ceiling to both
  gates; document the pin and display option in both headers.
- [x] T-03 Run both gates on private `:99` displays; compare the held assets
  frame and both line crops to the goldens (AE=0); regenerate the three goldens
  from the pinned artifacts.
- [x] T-04 Run the independent correctness and Amiga platform reviews; fix every
  critical/major finding. Both reviews are clean on critical/major; the four
  Amiga minors and the two correctness minors are fixed and recorded below.
- [x] T-05 Run `make check` and the full `make verify`; update the roadmap and
  close #35. The correctness review ran `make check` exit 0; the full verify
  reports `verify: all legs passed` with all seven A500 gates OK on `:99`;
  `docker/.env` was restored to `DISPLAY=:0`.

**Verify**: the two `:99` gate runs; the AE=0 golden comparison; `make check`;
`make verify`.

## Design constraints

| Constraint | Reason |
|---|---|
| Both windows pinned to 960x540 with no border | The committed goldens were captured from that geometry |
| Capture size asserted before the first crop | A window default or display-scale change fails with the observed size |
| Existing crops unchanged | The pinned window is the geometry the goldens already use |
| Caller `DISPLAY`/`XAUTHORITY` wins when set | The private-Xvfb acceptance needs no `.env` edit |
| Both gates poll 90 x 2 s | The #37 demo budget covers a loaded boot; the exec re-import is size-checked too |
| Goldens kept in the existing docs paths | The gate paths and the documentation links stay stable |
| No product, ABI, runtime, stdlib, or Makefile change | Test-harness only |

## Known risks

| Risk | Mitigation |
|---|---|
| The golden bytes change without a content change | The pinned captures compare AE=0 to the previous goldens; the byte change is encoding/metadata only |
| A future crop leaves the 960x540 window | The geometry test parses every crop and asserts it stays inside the pinned window |
| The desktop path changes | Only the window pin and the caller override are added; the `.env` desktop default is unchanged |
| A caller display is unreachable | The existing no-window or size error reports the gate failure; the caller can unset `DISPLAY` |
| An aspect/scaling change inside the pinned window | The size check does not see it; the crop AE comparison fails closed, and the SDK image pin holds FS-UAE 3.1.66 |
| The generated ADFs and work artifacts remain | Deliberate: the last screen and ADF are kept for diagnosis, matching the other A500 gates |

## Key decisions

| Decision | Outcome |
|---|---|
| Window pin | 960x540 with no border in both gates |
| Size assertion | `960x540` before the first crop |
| Crops | Unchanged: assets `414x410+418+98` and `620x22+160+88`; exec `620x22+160+88` |
| Caller display | The #36 `DISPLAY`/`XAUTHORITY` override |
| Poll budget | 90 x 2 s in both gates, matching the #37 demo gates |
| Guards | The geometry and caller-display spec tests now cover seven gates |
| Specialist reviews | Correctness plus Amiga platform, because the change is FS-UAE gate mechanics |

## Findings

### Red baseline

Both extended tests failed before the scripts changed:

```
docker/scripts/a500-assets-smoke.sh must pin the FS-UAE window with "--window-width=960"
docker/scripts/a500-assets-smoke.sh must honour a caller-supplied display with "caller_display=\"${DISPLAY:-}\""
```

The seven-gate `amiga_a500_spec_test` suite passes after the change.

### Pinned gate runs

| Gate | Display | Result | Time |
|---|---|---|---|
| assets | fresh `:99` Xvfb | OK | 68 s |
| exec | fresh `:99` Xvfb | OK | 26 s |

### Regenerated goldens

| Golden | Size | AE against the previous golden |
|---|---|---|
| `docs/amiga-sdk/a500-assets.png` (held frame, cropped from `live.png`) | 414x410 | 0 |
| `docs/amiga-sdk/a500-assets-line.png` | 620x22 | 0 |
| `docs/amiga-sdk/a500-exec.png` | 620x22 | 0 |

The binary delta is encoding/metadata only; the pixels are unchanged.

### Review findings

The Amiga platform review reported four minor findings and no critical or
major finding.

| ID | Finding | Resolution |
|---|---|---|
| AP-001 | The exec gate's second import in the `CAPTURE_GOLDEN=1` path replaced the screenshot without a size check, so the saved golden could come from an unasserted capture | Added the `960x540` assertion after the re-import as well; the cropped image is always size-checked |
| AP-002 | `the_a500_screenshot_gates_select_their_own_window` still covered only five gates, leaving the assets/exec search and budget unguarded | Extended to seven gates with the assets/exec topmost search and 90-poll budget |
| AP-003 | The test module doc still said "the hello and four demo screenshot gates" | Reworded to the A500 screenshot gates without a stale count |
| AP-004 | The geometry doc claimed the size check catches an aspect-correction change; a change inside the pinned window is caught by the crop AE comparison | Reworded in the test doc |

The correctness review reported two more minors.

| ID | Finding | Resolution |
|---|---|---|
| RV-001 | The selection-test doc still said "the four demo gates allow three minutes" | Reworded to the six demo gates |
| RV-002 | The 90-poll ceiling and the seven-gate selection test are beyond the dispatch's original change list | Kept: they match the #37 demo budget, the plan records them in T-01/T-02, and they change only the worst-case timeout and the static guard, not captures, crops, or output |

Open questions resolved: both gates now use the #37 90-poll budget; the aspect
correction is guarded by the AE comparison and the pinned SDK image; the full
verify points `docker/.env` at `:99` for the legacy display gates; the ADFs are
kept as diagnostics; `aqtinstall.log` is untracked and is not staged.

### Full verify

`make verify` ran with `docker/.env` temporarily pointing at a private `:99`
Xvfb (restored to `:0` afterwards), so no gate opened a window on the desktop.
The run exits 0 and reports `verify: all legs passed`. All seven A500 gates
report OK on `:99`: hello, Boing, sprites, paddle, text, assets, and exec. The
correctness review independently ran `make check` exit 0 on the same tree.
