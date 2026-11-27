# Testing Strategy: Amiga shim crash-safety

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| The four fixed paths (Intuition open, text classify/clip, clipboard finish, region/guide release) | every branch exercised by an ST case or an impl test |
| Existing behaviour of the same entries | unchanged: existing gates and pins stay green |

- Rust test names state behaviour (`a_region_freed_while_installed_unclips_its_layer`).
- Hardware behaviour runs on FS-UAE; `vamos` covers only the no-Intuition path, because it lacks
  `BltTemplate`, real layers, and `clipboard.device` (AR-13).

## Test Slices & Groups

| Slice | Tier | Test Files | Sources | Depends On | Gate |
| ----- | ---- | ---------- | ------- | ---------- | ---- |
| crash-safety (vamos, pins) | task | `compiler/tests/amiga_crash_safety_{spec,impl}_test.rs` | `compiler/runtime/shim_hosted_amiga*.c`, `examples/robustness/crash-safety/` | — | task |
| A1200 probes | release | `docker/scripts/a1200-addons-smoke.sh` | same | task | release (`make sdk-a1200-addons`) |
| A500 probe | release | `docker/scripts/a500-fault-cleanup-smoke.sh` | same | task | release (`make sdk-a500-fault-cleanup`) |

**Task verify**: `make check`. **Task verify evidence**: 4442 tests observed, 2026-11-27 (the
`make check` run of the previous plan's closeout on this tree).
**Gate**: `make verify` (AGENTS.md).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from [01](01-requirements.md), the 03 documents, the frozen spec rows they cite, and the
> register. **Immutable oracle:** if the implementation disagrees, the implementation is wrong.
> Test code quotes the behaviour in plain language, never an ST/AR identifier or a `plans/` path.

**Leak pattern** (ST-1, ST-2): a `vamos` line matching
`can't expunge|mem_alloc:WARNING:\s+orphan|shutdown: can't expunge`.

### `vamos` leg

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `alert-first.clo`: `println("calling")`, `showAlert("probe alert")`, `println("back")`, return 0; `hosted-amiga` and `hosted-amiga-13`; `vamos -O intuition.library=mode:off` | stdout exactly `calling\nback\n`; exit 0; no leak line | FR-1, AR-2 |
| ST-2 | `pubscreen-first.clo`: `println("calling")`, `savePublicScreen("ClNoSuchScreen", "RAM:probe")`, print `saved {ok}`, return 0; `hosted-amiga`; same option | stdout exactly `calling\nsaved false\n`; exit 0; no leak line | FR-1, AR-2 |
| ST-3 | release builds of `examples/hello.clo` (both profiles) and of `alert-first.clo`; `m68k-amigaos-nm` | hello links neither `cl_res_each` nor `cl_text_draw_clipped`; a `drawText` caller links `cl_text_draw_clipped`; a program with no region or guide links no `cl_res_each` | AR-15, 03-02 §New Functions, 03-03 §Registry walk |

### A1200 leg — `sdk-a1200-addons`

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-4 | `pubscreen-wb.clo`, run after Workbench started: `savePublicScreen("ClNoSuchScreen", "CNEXT:none.cscr")` then `savePublicScreen("Workbench", "CNEXT:pub.cscr")` as the program's first Intuition calls | `pub.txt` = `pub: ok unknown false wb true`; `CNEXT:pub.cscr` starts with `CSCR` | FR-1, AR-2 |
| ST-5 | `clip-iff.clo`: `openClipboard(0)`; write clip A = the 24-byte IFF `FORM` (size 16) `FTXT` `CHRS` (size 4) `abcd`; read into an 8-byte buffer; write clip B = the same form with `wxyz`; read into a 64-byte buffer; `closeClipboard` | `clipiff.txt` = `clip: ok r1 8 r2 24 c 119` (`c` = byte 20 of the second read, `w`); the program finishes | FR-3, AR-10, AR-11 |
| ST-6 | `clip-raw.clo`: the verifier's trace — write raw `abcd`, read into 16 bytes, write raw `efgh`, read into 16 bytes | `clipraw.txt` = `clipraw: ok w2 true` (the read counts are not asserted: raw clips belong to another plan) | FR-3, AR-10 |
| ST-7 | `region-free.clo`: bitmaps A, B, C 64×32×1; layers LA, LB, LC covering each; region R1 = rect (0,0)-(3,3) installed in LA; `freeRegion(R1)`; `layerFill` LA and LB (0,0)-(63,31) pen 1; compare `bitmapChecksum` A vs B. Then region R2 = same rect installed in LA and LC; `freeLayer(LC)`; `freeRegion(R2)`; `layerClear` A and B pen 0; fill both again; compare | `region.txt` = `region: ok same 2` (both comparisons equal) | FR-4, AR-6, AR-7 |
| ST-8 | `guide-late.clo`: `openAmigaGuide("CNEXT:hello.guide", "Workbench")`, `waitAmigaGuideMessage`, `closeAmigaGuide(guide)`, then `guideMessageType(message)`, then `replyGuideMessage(message)` | `guidelate.txt` = `guidelate: ok type 0`; the program finishes | FR-5, AR-9 |
| ST-9 | `text-offscreen.clo` (`hosted-amiga`): `openScreen(320, 200, 2)`, `defaultFont()`, 200 × `drawText(s, f, 0, -4000 + i*40, 1, "WWWWWWWW")`, then `drawText` at (100000, 50), (50, -100000), (-100000, 50), then `println("text: ok after")` | `text.txt` = `text: ok after`; the gate's `done.txt` appears | FR-2, AR-3, AR-5 |

### A500 leg — `sdk-a500-fault-cleanup`

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-10 | `text-clip.clo` (`hosted-amiga-13`): `openScreen(320, 200, 2)`, `clear(s, 2)`, `defaultFont()` (topaz 8); the text `CLIP` in pen 1 (screen draw mode as Intuition leaves it). Reference R at (140, 100). Clipped instances: I1 (-13, 50), I2 (300, 70), I3 (140, 2), I4 (140, 202), I5 (-20, 3), I6 (305, 203). Then 200 × `drawText` at x = 1000 and 200 × at x = -1000 with y = -4000 + i*40; then (100000, 50) and (50, -100000). `saveScreen(s, "DF1:text.cscr")`. Prints `textclip: ok w {textWidth} h {textHeight}` | output line `textclip: ok w 32 h 8`; rc ok; walker equality including `chip`. Capture: for each Ii with offset (dx, dy) = (140 - xi, 100 - yi), the window columns `max(0, xi) .. min(320, xi + w) - 1` and rows `max(0, yi - h) .. min(200, yi + h) - 1` equals the same-size window at (+dx, +dy) — AE metric 0; the reference window holds at least one pen-1 pixel | FR-2, AR-3..AR-5 |

### Regression

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-11 | the existing legs of `sdk-a1200-addons` (dt, net, guide, Workbench start) and `sdk-a500-fault-cleanup` (all ten probes), plus `sdk-a500-gui`, `sdk-a500-text`, `sdk-a500-assets`, `sdk-a1200-workbench`, `sdk-a600-addons` | unchanged expected lines and goldens | AR-13, FR-6 |

**Red-phase expectations** (AR-13):
- Fail before implementation: ST-1, ST-2 (crash through A6=0, verified in planning); ST-4 (crash,
  reproduced by the verifier); ST-5, ST-6 (the second write never returns; `done.txt` missing);
  ST-7 (the dangling clip still clips LA, so the checksums differ, or the layer crashes); ST-8 (a
  stale non-zero type or a crash); ST-9 (hang, reproduced by the verifier); ST-3 (the symbols
  do not exist yet).
- May pass before implementation, justified: ST-10's capture comparison (the base also draws the
  visible pixels; it guards the new clip path), and its walker line (the base's damage outside the
  bitmap is not deterministic). ST-9 carries the required red for the text fix. ST-11 is a guard.

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/amiga_crash_safety_spec_test.rs` | ST-1, ST-2, ST-3 | 03-01, 03-02, 03-03 |
| `docker/scripts/a1200-addons-smoke.sh` | ST-4..ST-9, ST-11 (its legs) | 03-01..03-04 |
| `docker/scripts/a500-fault-cleanup-smoke.sh` | ST-10, ST-11 (its legs) | 03-02, 03-04 |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/amiga_crash_safety_impl_test.rs` | Intuition opened before `DisplayAlert`/`LockPubScreen`; the V36 check | High |
| same | clipboard: saved count, skip read with NULL data, one NULL past-the-end read, no offset reset, retry only on the caller read | High |
| same | text: range checks first; visible rectangle from `min` of screen and bitmap sizes; default-glyph index; `LONG` arithmetic; colour-font rule; `WaitBlit` before the frees; the fully inside path is the old call sequence | High |
| same | layers: `cl_layer_free` unclips before `DeleteLayer`; the update flag set/cleared and ended before `InstallClipRegion`; `cl_free_region` walks before `DisposeRegion` | High |
| same | guide: message → guide link; close replies and orphans before `CloseAmigaGuide` | High |
| `compiler/tests/amiga_{intuition,workbench,optional}_impl_test.rs` | update only the pins whose shim text intentionally changes | Med |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make sdk-a1200-addons` | shims, A1200/KS3.1, Workbench 3.1 | ST-4..ST-9 and the existing legs |
| `make sdk-a500-fault-cleanup` | shims, A500/KS1.3, walker | ST-10 and the existing ten probes |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| full acceptance | `make verify` | `verify: all legs passed` |

## Test Data

### Fixtures Needed
- The probes above; the existing `examples/amiga/hello.guide`; the 24-byte IFF clips built in
  `clip-iff.clo` from byte arrays.

### Mock Requirements
- None; real `vamos`, real Kickstart ROMs under FS-UAE.

## Verification Checklist
- [ ] All ST-cases defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase), with the listed guards justified
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] `make verify` passes; bench evidence recorded
- [ ] No regressions in existing tests
