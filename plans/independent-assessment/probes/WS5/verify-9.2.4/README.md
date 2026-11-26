# Verifier reproductions for packet 9.2.4 (WS5)

Copied verbatim from `build/tmp/assessment/verify/` of verifier session
`ses_dea9a6107ffefWXd6gFWNJQszZ` (Phase 9, 2026-11-26) after its runs, so that the
findings whose confidence the verifier raised to `verified` cite committed sources.
The emitted `.c` files and binaries are not committed; rebuild them with the pinned `clc`.
Runners: `run-a1200.sh` (A1200/KS3.1 directory drive, as `../54x-fs-uae.sh`),
`run-a500.sh` / `a500-batch.sh` (A500/KS1.3: the licensed ROM is decrypted under
`build/tmp` and removed on exit; a copy of the Workbench 1.3.5 HDF from `~/amiga`), and
`stage-wb.sh` (stages licensed Workbench 3.1 files for the clipboard runs). They write
only under `build/tmp/assessment/verify/` and use a private Xvfb display.

| Finding | Sources | Observation (verifier report `../../../verification/verdicts-9.2.4.md`) |
|---------|---------|------------------------------------------------------------------------|
| ASM-WS5-001 | `v541.clo`, `v541c.clo`, `v541p.clo`, `v541pc.clo`, `walker2.c`, `trap.c` | A1200: Software Failure 8000 0004 after the fault; A500: Guru 00000004; PortList 0→1 after a fault, contrast 0→0 |
| ASM-WS5-002 | `alert13.clo`, `alert13c.clo` | A500: `showAlert` with no screen → "Software error - task held" |
| ASM-WS5-003 | `cd.clo` | CLI run: no crash; the shell's directory changes |
| ASM-WS5-004 | `ttext.clo`, `ttextc.clo`, `tfill.clo`, `tfillc.clo` | off-screen `drawText` hangs the A1200; the contrasts complete |
| ASM-WS5-005 | `taskdiv.clo`, `taskdivc.clo` | division by zero in a worker hangs the A1200; the contrast joins |
| ASM-WS5-006 | `region.clo`, `regionc.clo` | no failure observed |
| ASM-WS5-011 | `align.c` | frames stay multiples of 4 |
| ASM-WS5-013 | `walker3.c`, `leaklib.c` | open counts unchanged even for the seeded leak (detector cannot fire) |
| ASM-WS5-016 | `clip.clo`, `clipm.clo`, `clipc.clo` | a write after `clipboardRead` never returns; the contrast completes |
| ASM-WS5-023 | `clipw.clo`, `clipr.clo`, `clipiff.c`, `clipdump.c` | raw clips are discarded; an IFF clip is kept |
