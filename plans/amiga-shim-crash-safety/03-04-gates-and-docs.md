# Gates and docs: Amiga shim crash-safety

> **Document**: 03-04-gates-and-docs.md
> **Parent**: [Index](00-index.md)

## Overview

Target evidence runs on four legs, with no new gate script (AR-13):
- `vamos`, both hosted profiles, with Intuition switched off — the no-Intuition path;
- `sdk-a1200-addons` (A1200/KS3.1, Workbench 3.1 `Libs`/`Devs`/`Fonts`) — the hosted-amiga probes;
- `sdk-a600-addons` (A600/KS2.05, Workbench 2.04 `Devs`) — the IFF clipboard probe on the 2.0
  device;
- `sdk-a500-fault-cleanup` (A500/KS1.3, Workbench 1.3.5 HDF, DF1 capture ADF) — the 1.3 text clip.

## Probes

All probes live in `examples/robustness/crash-safety/` and print one result line. Expected lines
and inputs are owned by [07](07-testing-strategy.md).

| Probe | Leg | Exercises | ST |
|---|---|---|---|
| `alert-first.clo` | vamos, both profiles | `showAlert` with no screen | ST-1 |
| `pubscreen-first.clo` | vamos (hosted-amiga) | `savePublicScreen` with no screen | ST-2 |
| `pubscreen-wb.clo` | A1200 | `savePublicScreen` first, unknown name then `Workbench` | ST-4 |
| `clip-iff.clo` | A1200, A600 | IFF clip writes with a write after each of three reads: part of the clip, exactly the clip, a larger buffer | ST-5 |
| `clip-raw.clo` | A1200 | the verifier's raw write/read/write/read trace | ST-6 |
| `region-free.clo` | A1200 | region freed while installed; layer freed with a clip | ST-7 |
| `guide-late.clo` | A1200 | getter and reply after `closeAmigaGuide` | ST-8 |
| `text-offscreen.clo` | A1200 | the verifier's 200 off-screen `drawText` calls | ST-9 |
| `text-clip.clo` | A500 (`hosted-amiga-13`) | clipped text at each edge vs. a fully inside reference; `saveScreen` to `DF1:` | ST-10 |

## `vamos` leg

`compiler/tests/amiga_crash_safety_spec_test.rs` builds ST-1/ST-2 with the R1 spec test's build
helper pattern (`clc`, then `m68k-amigaos-gcc -Wall -Wextra -Werror` for `nix20` and `nix13`) and
runs `vamos -O intuition.library=mode:off <binary>`. It asserts stdout, exit 0, and no line
matching the R1 leak pattern (`can't expunge|mem_alloc:WARNING:\s+orphan|shutdown: can't expunge`).
It skips without the SDK toolchain, like the R1 suite.

## A1200 leg: `sdk-a1200-addons`

- **Build:** the `sdk-a1200-addons` recipe also builds the six A1200 probes with
  `clorraine build --release --profile hosted-amiga`.
- **Install and run:** the script copies them to `CNEXT:` and appends to the startup-sequence,
  after the three existing legs, one line per probe redirected to its own file
  (`pub.txt`, `clipiff.txt`, `clipraw.txt`, `region.txt`, `guidelate.txt`, `text.txt`), then
  `echo done >CNEXT:done.txt` (AR-14). The clipboard probes run after the region and guide probes,
  `clip-raw` before `clip-iff` so it reads the empty unit (AR-17), and `text-offscreen` runs last,
  because a red-phase hang there stops the rest.
- **Completion:** wait for `done.txt` (bounded at 240 s), then for the HD tree to stay unchanged
  for 3 s (bounded at 30 s), then stop FS-UAE (AR-14). The existing three checks and the
  Workbench-start check stay; each new file is checked against its expected line; a missing file
  fails with its name.
- **Assigns:** the existing `wb_assigns` (`CLIPS:` → `CNEXT:clips`) serves the clipboard probes.

## A600 leg: `sdk-a600-addons`

- **Build:** the `sdk-a600-addons` recipe also builds `clip-iff.clo` with
  `clorraine build --release --profile hosted-amiga`.
- **Install and run:** the script copies it to `CNEXT:` and appends one startup-sequence line after
  the existing legs, `CNEXT:clip-iff >CNEXT:clipiff.txt`. The existing `wb_assigns` provide `CLIPS:`
  and the Workbench 2.04 `Devs` provide `clipboard.device`.
- **Completion and check:** after the existing `asl-done.txt` wait, the gate waits for
  `clipiff.txt` (bounded at 60 s), then for the HD tree to stay unchanged for 3 s (bounded at
  30 s), the AR-14 settle rule. `clipiff.txt` must hold the ST-5 line; a missing file fails with its
  name. The existing checks are unchanged.
- This leg guards the NULL-data reads on the 2.0 device; its red phase is recorded but not required
  (AR-13).

## A500 leg: `sdk-a500-fault-cleanup`

- `text-clip` is added as the last case, built for `hosted-amiga-13` with the other probes, with the
  `chip` column set (the temporary raster must be freed). The case list builds every probe from
  `examples/robustness/fault-cleanup/`; the entry for `text-clip` names its source
  `examples/robustness/crash-safety/text-clip.clo`, and only the build and install steps read that
  path.
- The probe writes its capture with `saveScreen` to `DF1:text.cscr`. After the run the gate unpacks
  it with the existing `xdftool` step, converts it with `docker/scripts/cscr2png.py` (no scale) on
  the host, and, for each clipped instance listed in [07 §ST-10](07-testing-strategy.md#a500-leg--sdk-a500-fault-cleanup),
  compares the window around it with the reference window shifted by the instance's offset, using
  ImageMagick `convert -crop` and `compare -metric AE`; every metric must be 0. The reference window
  must hold at least two colours (text and background), a check that does not depend on how the
  capture decodes the palette. `python3`, `convert`, and `compare` join the gate's host-tool check.
- **Title bar:** I3 and I5 sit in the screen's title-bar rows. `clear` paints over the bar, and the
  probe opens no window and makes no other Intuition call, so Intuition has no reason to redraw it.
  If it does, the comparison fails visibly and names the instance.
- **`chip` column:** this is the first `chip`-checked case that opens an Intuition screen and writes
  a DF1 file. If the green run shows a `chip` or `mem` difference, rerun a local, uncommitted
  contrast of the probe without the clipped and off-screen calls. If the contrast also differs, the
  difference comes from Intuition or DOS: set the column to `-` for `text-clip` and record why in
  the execution plan. Otherwise the temporary raster leaks, which is a defect to fix.

## Make wiring

No new target. `sdk-a1200-addons`, `sdk-a600-addons`, and `sdk-a500-fault-cleanup` are already in
`make verify`; their `AGENTS.md` and Makefile descriptions gain the new probes (AR-13).

## Docs

| File | Content | AR |
|---|---|---|
| `stdlib/std.amiga.text.clo` (`drawText`) | text is clipped to the screen; fully outside or out-of-range draws nothing; a partly visible colour-font string draws nothing | AR-16 |
| `stdlib/std.amiga.intuition.clo` (`showAlert`) | works with no screen; no-op when Intuition cannot be opened | AR-16 |
| `stdlib/std.amiga.workbench.clo` (`savePublicScreen`) | works with no screen; false when Intuition V36 is not available | AR-16 |
| `stdlib/std.amiga.regions.clo` | freeing an installed region unclips its layers first; freeing a layer unclips it first | AR-16 |
| `stdlib/std.amiga.amigaguide.clo` | closing replies held messages; their getters then read 0 and their reply frees only the handle | AR-16 |
| `stdlib/std.amiga.clipboard.clo` (module header, `Clipboard`, `clipboardRead`) | the handle keeps its request and port and opens the unit only during each write or read; a read never goes past the end of the clip and an empty buffer reads nothing, so later writes are not held off; while another program keeps the unit open, a read that returns no data holds off writes until that program closes it (AR-18) | AR-16, AR-18 |
| `docs/known-limitations.md` | §4 Clipboard runtime proof becomes IFF write/read pairs on A1200/KS3.1 and A600/KS2.05; §5 clipboard bullet updated, with the other-opener limit and that each operation may reload `clipboard.device` from `DEVS:` (a volume requester on a floppy system after an expunge); a colour-font clip limit line | AR-16, AR-18 |
| `docs/amiga-sdk/registry.tsv` (SDK-WBK-006) and `coverage.md` | write/update and read proven; evidence `sdk-a600-addons` and `sdk-a1200-addons` with `clip: ok r1 8 r2 24 r3 24 c 109`; regenerate with `make sdk-registry` | AR-18 |
| `AGENTS.md` | the three gate rows name the new probes | AR-13 |

No `spec/` file changes (AR-12).

## Bench evidence

The closing phase runs `make bench-base COMMIT=<plan base>`, `make bench`, `make bench-compare`
(empty `REQUIRE`), and records the release `.text` of `examples/amiga/gui.clo`
(`hosted-amiga-13`) and `examples/amiga/scroller.clo` (`hosted-amiga-13`) at base and head (AR-15).
