# Task: Make the A500 screenshot captures select their own window

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 5/5 tasks (100%)
> **Reasoning**: medium — the capture oracle stays exact, but window selection,
> the boot budget, and the failure diagnostics change in five shell gates; no
> compiler, runtime, ABI, stdlib, golden, crop, or geometry change.

## Objective

Deliver issue #37. `make verify` for #36 failed once at `sdk-a500-smoke` with
`AE=4` against the hello golden, and the rerun failed at `sdk-a500-paddle` with
`never saw the paddle screen`. Both gates pass alone and on a private Xvfb.

The issue names the host pointer as the suspect. The probes below show the
pointer is **not** part of an `import -window` capture. The old oracle also
accepted only `AE=0`, so a single bad frame could not have been accepted; the
hello failure means every poll captured a screen that differed.

Two defects are addressed:

1. **Window selection.** `xdotool search` returns the stacking order (bottom to
   top). The hello gate took `head -1` (bottommost), while the other four gates
   already took `tail -1` (topmost). A leftover FS-UAE window below the gate's
   own can therefore be captured by the hello gate. This is a latent
   wrong-window bug; a different screen produces a large AE (the reproduction
   below fails with `AE=1612`), not the one-pixel `AE=4`, so it does not by
   itself explain #36.
2. **One-pixel capture variation.** The `AE=4`/`AE=1` failures are a single
   blended pixel inside an otherwise correct hello line. The exact cause was
   not reproduced. The gate now logs the selected window and geometry each
   poll, keeps the last three captures, and archives the closest non-matching
   capture, so the next occurrence is decidable. The hello budget is raised to
   the demo shape (120 x 2 s after a 10 s start) to cover a slow boot.

The paddle failure is the budget shape: its poll was 60 x 2 s (120 s), while
the hello gate already allowed 30 x 5 s (180 s). The four demo gates now allow
90 x 2 s, and paddle/text report a held-frame mismatch instead of claiming the
demo never appeared.

**Smallest viable design:** take the topmost window in the hello gate, keep the
byte-exact single-match oracle, raise the hello budget to 120 x 2 s and the
four demo budgets to 90 x 2 s, add the poll log and near-miss archive, and
remove the redundant paddle/text liveness branch. Add specification tests for
the topmost search, the client-window capture, the budgets, and the
held-display message.

## Tasks

- [x] T-01 Add `the_a500_screenshot_gates_select_their_own_window` and
  `the_a500_paddle_and_text_failures_name_the_held_display` to
  `compiler/tests/amiga_a500_spec_test.rs`: require `tail -1` in all five
  gates, reject `head -1` and a root capture, require the FS-UAE client-window
  `import`, pin the 120/90 budgets, and pin the held-display failure message.
  Record the red run and a mutation probe.
- [x] T-02 Change the hello search to the topmost window; raise the hello poll
  to 120 x 2 s with a 10 s start and the four demo polls to 90 x 2 s; add the
  per-poll window/geometry/AE log, the last-three-captures archive, and the
  closest-capture archive; remove the redundant paddle/text `saw_live` branch.
- [x] T-03 Prove the behavior: reproduce the stale-window selection (old
  `head -1` fails, new `tail -1` passes); capture the next one-pixel failure
  with the poll log and near-miss archive; run the hello gate five times on
  fresh `:99` displays; run the four demo gates on `:99`; keep the
  pointer-over-line desktop observation.
- [x] T-04 Run the independent correctness and Amiga platform reviews on a
  stable final tree; fix every critical/major finding. The first Amiga review
  rejected the two-consecutive-match design as critical; the second added
  AP-007 (the search fix is latent, not the `AE=4` cause) and AP-008 (stacking
  order, not creation order). Both are addressed above.
- [x] T-05 Run `make check` and the full `make verify`; update the roadmap and
  close #37. The correctness review ran `make check` exit 0 on the final tree;
  the full verify (which includes `make check`) reported `verify: all legs
  passed`, with all five A500 gates OK on `:99`; `docker/.env` was restored to
  `DISPLAY=:0`.

**Verify**: `make check`; the stale-window probe; the diagnostic failure
archive; five `:99` hello runs; the four demo gates; `make verify`.

## Design constraints

| Constraint | Reason |
|---|---|
| The capture stays `import -window "${window}"` | The client-window drawable excludes the host pointer and other windows |
| The search takes `tail -1` in all five gates | `xdotool search` returns stacking order; the gate's newly mapped window is on top |
| Single exact `AE=0` acceptance | The oracle stays byte-exact; a one-pixel regression still fails |
| Goldens and crops are unchanged | The defect is window selection, boot time, and capture variation, not content or geometry |
| Hello budget 120 x 2 s after a 10 s start | The observed `AE=1` failure ended the old 150 s poll; a four-minute boot must still be covered |
| Four demo budgets 90 x 2 s after a 10 s start | The hello gate already allows three minutes; the paddle boot can exceed two minutes on a loaded host |
| Per-poll log, last three captures, closest capture | The next one-pixel failure must be inspectable instead of guessed at |
| No pointer parking or cursor hiding | The pointer is not in the capture, and moving the user's pointer would be an unnecessary side effect |
| Paddle/text report the held-frame failure | `never saw the paddle screen` misled triage when the demo had run but the held frame never matched |
| No product, ABI, runtime, stdlib, or Makefile change | Test-harness only |

## Known risks

| Risk | Mitigation |
|---|---|
| A foreign window is raised above the gate's on a desktop | Use a private display; the gate logs the selected window and geometry every poll, so a wrong selection is visible |
| A persistent one-pixel blending artifact is not fixed by more polls | The near-miss archive records it; if it is a host scaler or GL path, the deterministic path is a private display |
| The longer hello budget slows a failing run | Only failures pay up to four minutes; passing runs break on the first exact match |
| A leftover window remains on a reused private display | Start a fresh display per run; recorded as a limit in #36 |
| The exact `AE=4`/`AE=1` cause remains undiagnosed | The diagnostics now preserve the window, geometry, poll AE sequence, last three captures, and closest capture |
| The paddle failure could be a demo crash, not a slow boot | The gate now reports `held display never matched` and keeps the display and screen |

## Key decisions

| Decision | Outcome |
|---|---|
| Window search | `tail -1` (topmost) in all five gates |
| Acceptance rule | Single exact `AE=0` match, unchanged from the frozen oracle |
| Hello budget | 120 x 2 s after a 10 s start (about 250 s) |
| Demo budgets | 90 x 2 s after a 10 s start (about 190 s) |
| Diagnostics | `poll.log`, `capture-<step>.png` (last three), `near-miss.png` |
| Paddle/text diagnostics | The redundant `saw_live` branch is removed; `frame_matched` reports the held display |
| Root-cause claim | The topmost search fixes a latent wrong-window bug; the one-pixel variation remains undiagnosed |
| Specialist reviews | Correctness plus Amiga platform, because the change is FS-UAE gate mechanics |

## Findings

### The host pointer is not in the capture

| Probe | Result |
|---|---|
| Desktop `:0`, pointer held at window-relative `(175,44)` (the first hello glyph) for the whole run | Gate exit 0, `AE=0`; the pointer was at the hello line at every capture |
| Private `:99`, window captured before and after moving the pointer across the line | Byte-identical captures (`AE=0`); no pointer pixels |
| Private `:99`, pointer moved to the hello line after focusing the window and clicking | Window capture unchanged (`AE=0`); no Amiga pointer appeared |
| `import -window "${window}"` | Reads the client-window drawable; the X pointer is a server sprite, not window pixels |

The `AE=4` mask was one native pixel at upscaled `(12,0)-(13,1)`, i.e. window
`(175,44)`, color `srgb(0,31,62)`. That color is a scaled blend of the blue
background and the glyph, not an X cursor color and not an Amiga palette color.
It is consistent with a host or GL presentation difference inside the correct
line, not with a pointer or a different Amiga screen.

### The window search returns stacking order

Two FS-UAE containers were started on a private `:99`, six seconds apart.

| Observation | Result |
|---|---|
| `xdotool search --name 'FS-UAE'` order | `2097158` (container A), then `8388614` (container B) |
| `head -1` | `2097158`, the **bottommost** window |
| `tail -1` | `8388614`, the **topmost** window |
| Window PIDs | `1` for both (container namespace PID), so the PID cannot identify the container |
| Geometry | both at `320,232`, `640x256`; the stacking order, not position, distinguishes them |

The Amiga platform review measured the same property with two `xmessage`
windows: raising the first window changed the search order from `A B` to
`B A`, and `tail -1` then selected the raised window. The search is stacking
order, not creation order. On a WM-less private display a newly mapped window
is on top, so the gate's own window wins, including the FS-UAE recreation
path. A desktop window raised above the gate's can still be selected; the
per-poll geometry log makes that visible.

### Stale-window reproduction

A paddle FS-UAE window was started first on `:99` and left running. The old
search (`head -1`, five polls) and the new search (`tail -1`, the normal gate)
then ran against the same display.

| Search | Result |
|---|---|
| Old `head -1` | exit 1, `captured line differs ... (AE=1612)` on the same display as the passing `tail -1` run |
| New `tail -1` | exit 0, `a500-smoke: OK`; the gate's own topmost window was captured |

This is the wrong-window failure shape: a different Amiga screen produces a
large AE. The failing screen was not preserved, so the plan does not claim the
head run captured the stale window specifically; the stacking-order fact and
the same-display failure/pass show the selection bug and the fix.

### The one-pixel failure recurred

The final single-match code with `tail -1` first ran on fresh `:99` displays:

| Run | Result | Time |
|---|---|---|
| 1 | OK | 51 s |
| 2 | FAIL, `AE=1` | 186 s |

The run-2 log is kept at `build/tmp/hello-fail-run2/hello-xvfb-2.log`. The next
run deleted the failing `screen.png` before it could be copied; that is why
T-02 added the poll log, the last-three-captures archive, and the near-miss
archive.

The diagnostic loop then ran the final code eight more times on fresh `:99`
displays: **8/8 OK**, 50-51 s each. The one-pixel failure did not recur, and
the loop had no failure to archive. The cause of the single `AE=1` remains
undiagnosed; the longer 2 s poll and the archive are in place for the next
occurrence. The 8/8 sequence is the final-code determinism evidence.

### The first design was rejected by review

The first implementation required two consecutive exact matches. The Amiga
platform review rejected it as critical (AP-001): the old oracle accepted only
`AE=0`, so a transient could not have been accepted, and the failure means
every poll differed. The stricter rule made each gate harder to pass and could
fail a valid run when a short hold yields only a few captures. It was reverted;
the single-match oracle is unchanged.

### Fault injection

A wrapper `import` calls the real tool, then corrupts the first capture that
would otherwise match the golden exactly: one dark pixel in the hello line, one
magenta pixel in the held paddle court.

| Gate | Marker | Result |
|---|---|---|
| hello | injected | gate exit 0; the corrupted capture was skipped and a later exact match passed |
| paddle | injected | gate exit 0; the corrupted held frame was skipped and a later exact match passed |

These probes pass on the old gate too, so they are regression evidence, not the
differentiator for #37.

### Five-run determinism

The rejected stricter design produced 5/5 on fresh `:99` displays
(`build/tmp/repeat-hello-summary-strict.log`). The final single-match code
produced 8/8 on fresh `:99` displays after one earlier `AE=1` one-off:

| Run | Result | Time |
|---|---|---|
| 1 | OK | 50 s |
| 2 | OK | 51 s |
| 3 | OK | 51 s |
| 4 | OK | 51 s |
| 5 | OK | 51 s |
| 6 | OK | 51 s |
| 7 | OK | 51 s |
| 8 | OK | 50 s |

The diary is `build/tmp/repeat-hello-until-fail.log`; the earlier one-off is
recorded under "The one-pixel failure recurred".

Desktop `:0` evidence: the pointer-over-line run passed before the change
(`AE=0`), and one normal post-change run passed in 54 s. Further desktop runs
were kept to a minimum because each opens an FS-UAE window in the user's
session.

### Four demo gates

Each gate ran once on a fresh private `:99` Xvfb with the final single-match
code.

| Gate | Result | Time |
|---|---|---|
| Boing | OK | 83 s |
| sprites | OK | 82 s |
| paddle | OK | 84 s |
| text | OK | 87 s |

The full verify runs them again.

### Full verify

`make verify` ran with `docker/.env` temporarily pointing at a private `:99`
Xvfb (restored to `:0` afterwards), so no gate opened a window on the desktop.
The run exits 0 and reports `verify: all legs passed`. The five A500 gates all
report OK on `:99`: hello, Boing, sprites, paddle, and text. The correctness
review independently ran `make check` exit 0 on the same tree. Passing hello
runs leave `poll.log` and the capture archive in the gate work directory; the
near-miss path is only used when a capture differs.

### Review findings

| ID | Finding | Resolution |
|---|---|---|
| AP-001 | Critical: the two-consecutive rule inverted the failure mode; the old oracle already accepted only `AE=0` | Reverted to single-match acceptance; the search and budget changes are the revised fix |
| AP-002 | Major: the hello gate gained a stricter rule with no added budget, and its message could say `AE=0` | Moot after the revert; the hello budget is now 120 x 2 s |
| AP-003 | Major: paddle/text `saw_live` no longer meant "saw the screen", and the accurate held-frame branch was unreachable | Removed the redundant `saw_live` branch; `frame_matched` reports `held display never matched` with the display and screen paths; pinned by a spec test |
| AP-004 | Minor: the streak survived skipped polls | Moot after the revert; no streak remains |
| AP-005 | Minor: the static test could not tell "two consecutive" from "two ever" | Moot after the revert; the new test pins the topmost search and budgets, and a mutation probe fails when the hello search returns to `head -1` |
| AP-006 | Minor: the root-capture rationale stated a false X11 fact | Reworded: a root capture is rejected because the pinned-size check and window-relative crops assume the client window |
| AP-007 | Major: the search fix does not explain the `AE=4`; it is a latent wrong-window fix, and the hello flake stays unaddressed | Plan corrected; hello budget raised; poll log and near-miss archive added; the exact cause is recorded as undiagnosed |
| RV-001 | Minor: the four demo-gate comments still said "newest/created last" and compared their budget to a three-minute hello gate | Comments now say "stacking order/topmost" and state the demo gates' own three-minute budget; plan AP-008 row corrected |
| AP-008 | Minor: `xdotool search` returns stacking order, not creation order | Comments, test doc, plan, and roadmap reworded to "topmost"; the risk row now covers a foreign window raised above the gate's; the four demo-gate comment blocks were corrected after RV-001 |

The first correctness review stopped because the tree changed mid-review. The
refreshed correctness review on the stable final tree reported no critical or
major findings, one minor wording finding (RV-001), and `make check` exit 0;
RV-001 is fixed. The Amiga platform review's AP-001..AP-008 are resolved.
