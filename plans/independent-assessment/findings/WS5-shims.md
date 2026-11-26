# WS5 — Runtime shims

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS5 (WS5a = core and 2.0 shims; WS5b = optional shims, host, freestanding, `picasso96.h`)

## Scope and limits

Every shim in `compiler/runtime/` except `emulator_runner.c` (driver surface, AR #9).
Method: function-by-function reading against the checklist and the NDK 3.2 autodocs
(`build/distribution/reference-20261117/.../NDK3.2/Autodocs`, read locally), `vamos` probes
for DOS/Exec/timer/utility paths with the `vamos` leak detector, FS-UAE A1200/KS3.1 probes
for the paths `vamos` cannot model (interrupt chains, resources, Intuition), the host shim
under the WS2 ASan/UBSan probes, and patched-C estimates for hot paths.

Limits:
- FS-UAE runs use the A1200/KS3.1 ROM configured in `docker/.env`. No A500/KS1.3 FS-UAE
  probe was run; every A500 statement below is code reading against the V34 surface, and
  the findings that affect both profiles name the shared code.
- Hardware-visible output (sprite images, menus, colours on screen) was not captured;
  those findings rest on code reading plus the autodocs (`probable`), except the colormap
  capture, which is measured from a committed golden.
- The walker's public-port check cannot fire: `FindPort` matches by name and the name
  string lives in the unloaded program, so the seeded leftover port was not found
  (`limit`; removed by counting `SysBase->PortList` nodes instead).

Carried to Phase 9 (recommended in-session 2026-11-26; not yet confirmed by the user):
- Rerun probes 541 and 542 on FS-UAE A500/KS1.3 (ROM decryption as in the `sdk-a500-*`
  gates) to move the A500 lens of ASM-WS5-001 and ASM-WS5-002 from code reading to
  reproduction.
- Replace the walker's `FindPort` check with a `SysBase->PortList` node count and rerun
  541/541c, to remove the public-port `limit` of ASM-WS5-001.
- Optionally retry the dangling-server crash (a filler that also covers the reserved
  16 KB) so the crash itself can move from `probable` to `verified`.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |
| `explore` reconnaissance ×3 (read-only; `shim_hosted_amiga.c` 1–3100; 3100–6030 + `_20`; optional shims, host, freestanding, `picasso96.h`). Candidates only; every candidate below was re-read by the executing session at the cited lines, and the autodoc claims were checked against the local NDK 3.2 autodocs | medium | 2026-11-25 |

## Coverage table

| Group | Files / functions | Status |
|-------|-------------------|--------|
| Probes written (5.1.1, 5.1.2) | `probes/WS5/500`–`520`, `probes/WS4/530` | done |
| Red check (5.1.3) | seeded leak, contrasts, 504 → 507 | done |
| FS-UAE probes (5.2.3) | `probes/WS5/540`–`542`, `54x-*` | done |
| `shim_hosted_amiga.c` 1–3100 | DOS, runtime core (alloc, buffer, string, format, text), IO, screens, windows, gadgets, menus, requesters, pointer, alerts, events, messages, ports, interrupts, fault path — ~140 functions | done; candidates → ASM-WS5-001/002/003/009/010/011/018/019 |
| `shim_hosted_amiga.c` 3100–6030 | tasks, libraries, devices, video (chip, bitmaps, copper, display), sprites, audio, input, timer, text, layers — ~130 functions | done; candidates → ASM-WS5-001/004/005/014/017/019 |
| `shim_hosted_amiga_20.c` | icon, Workbench (AppIcon/AppWindow/AppMenu), expansion, card.resource, keymap, public-screen capture — 31 functions | done; → ASM-WS5-002/007/008/019 |
| `shim_hosted_amiga_optional.c` | semaphores, tags, date stamps, hooks, pools, ReadArgs, EClock, lowlevel, regions, datatypes — 72 functions | done; → ASM-WS5-006/019 |
| `shim_hosted_amiga_optional_services.c` | commodities, locale, ARexx, clipboard, serial, parallel, printer, nonvolatile | done; → ASM-WS5-015/016/019 |
| `shim_hosted_amiga_optional_ui.c` | gadtools, ASL, BOOPSI | done; → ASM-WS5-012/019 |
| `shim_hosted_amiga_optional_rtg.c` | Picasso96 RTG | done; `clean` (bounds and sizes checked) |
| `shim_hosted_amiga_optional_net.c` | bsdsocket, AmigaGuide | done; → ASM-WS5-006/013 |
| `picasso96.h` | LVOs, registers, enums, tags, `RenderInfo` | done; `clean` against P96Develop as recalled; not checked against a local fd file (no P96 fd in the SDK) |
| `shim_host.c` | runtime core mirror | done; → ASM-WS5-021; parity table under WS5-11 |
| `shim_freestanding.c` | console surface, libc subset, `_start` | done; → ASM-WS5-021 |
| Self-refutation (5.3.1) | amiga-platform-reviewer checklist | done |

## Checklist outcomes

Runs 2026-11-25 (vamos logs under `build/tmp/assessment/WS5/`, FS-UAE output under
`build/tmp/assessment/WS5/fsuae/run-<case>/`; committed records `records/WS5-results.json`,
`WS5-contrast.json`, `WS5-runs.log`).

| ID | Outcome | Evidence |
|----|---------|----------|
| WS5-01 | ASM-WS5-019 (V38/V44 tags behind V37 opens) | Every Intuition, graphics, layers, DOS, and Exec call on the KS1.3 path exists in V33/V34 (`clib/*_protos.h` in `ndk13-include`); `LockPubScreen` is reached only after `lib_Version < 37` (`shim_hosted_amiga.c:1611`); the 2.0+ services live in the `_20` and optional suffixes, which the 1.3 profile never links (spec/appendix-amiga.md §1). Exceptions: `ASLFR_DoSaveMode` (V38) with asl opened at V37 (`shim_hosted_amiga_optional_ui.c:46`, `:416`); `WBAPPICONA_PropagatePosition` (V44) with workbench opened at V37 (`shim_hosted_amiga_20.c:88`, `:155`) |
| WS5-02 | ASM-WS5-002, ASM-WS5-007, ASM-WS5-013 | 27 `OpenLibrary` sites (26 shim-internal plus the user wrapper) vs. one `CloseLibrary` (the user wrapper, `shim_hosted_amiga.c:3254`); the four initialized bases are program-owned (`IntuitionBase` sits in the program's `.bss`; `build/tmp/assessment/WS4/size/sieve.clo.map` links only `libstubs.a(dos.o)`). Probes 505, 506, 510 are leak-free under `vamos` (the user `openLibrary`/`closeLibrary` pair, timer, utility) |
| WS5-03 | ASM-WS5-010, ASM-WS5-014 | Every `AllocMem` in the eleven files has a `FreeMem` of the same size on every path (three explorers plus spot checks at `:253`, `:329`, `:1529`, `:1970-1995`); `MEMF_CHIP` is used for bitmaps, sprites, audio samples, and copper lists; exceptions: the pointer image size (ASM-WS5-010) and pre-V36 trackdisk buffers (ASM-WS5-014) |
| WS5-04 | ASM-WS5-002, ASM-WS5-004, ASM-WS5-012, ASM-WS5-019 | Probe 502 (every DOS failure path returns false/null, leak-free) passes; failure-path defects in the cited findings |
| WS5-05 | ASM-WS5-005, ASM-WS5-009 | One `Forbid`/`Permit` pair, balanced; no `Disable`; interrupt code calls only the user hook. Stack: the 4 KB KS1.3 default is not probed (a `limit`: removed by an A500 FS-UAE run with a deep-recursion probe) |
| WS5-06 | ASM-WS5-006, ASM-WS5-015, ASM-WS5-016 | Probe 507 (port, message, reply, drain) leak-free on both profiles; probe 504 was a probe design error (it deleted an undrained port, which spec/appendix-stdlib.md §13.2 makes the caller's duty) |
| WS5-07 | ASM-WS5-020 | No `SIGBREAKF_CTRL_C` token in `compiler/runtime/` or `stdlib/` |
| WS5-08 | ASM-WS5-001, ASM-WS5-003, ASM-WS5-005 | Probes 503 (vamos), 541 (FS-UAE), 501 (vamos) |
| WS5-09 | ASM-WS5-004, ASM-WS5-017, ASM-WS5-019 | `cl_video_display_*` uses `LoadView`/`WaitTOF`/`MrgCop` with a saved `old_view` (`:4493-4625`); blitter paths in the bitmap helpers `WaitBlit` (`:3973`, `:4148`, `:4180`); exceptions in the cited findings (raster DMA forced on and not restored, layer helpers without `WaitBlit`, unclipped screen text, ViewPort offsets doubled) |
| WS5-10 | ASM-WS5-008, ASM-WS5-011, ASM-WS5-019 | BPTR conversions use `MKBADDR`/`BADDR`; IFF parsing is byte-wise and big-endian (`stdlib/std.iff.clo:36-45`); hand prototypes at `:65-82` differ from the NDK in return/`CONST_STRPTR` only (benign through link stubs; Minor in ASM-WS5-019) |
| WS5-11 | ASM-WS5-021 | The WS2 probes (55) run clean under ASan with leak detection and UBSan (`records/WS2-results.json`); the sanitizer self-test detects seeded leak, double free, use-after-free, and overflow. Parity: `cl_trap`/`cl_panic` print to stderr and `abort()` on host, to stdout and `exit(20)` on hosted-amiga — the channel is per spec/17 §12 (debug output: libnix `printf` on hosted-amiga, `fprintf(stderr)` on host); the exit status differs (134 vs 20) and is pinned per profile by `run-fault-amiga`. `cl_text_f64` formats through each libc's `%g` (not differential-tested) |
| WS5-12 | ASM-WS5-021 | Probe 520: the freestanding corpus object has **no** undefined symbol and links with `-nostdlib -lg` (rc 0). Missing `memmove` and the generic `_start` are in ASM-WS5-021 |
| WS5-13 | ASM-WS5-022 | Probe 530 (`probes/WS4/530-format-u32-16bit.diff`): a 16-bit-division digit loop in `cl_format_u32` cuts `strings` by 689,884 instructions (−40.9% of net) on A500 -O2, output identical, `.text` +68 B. Allocation and buffer growth are covered by ASM-WS4-002 |
| WS5-14 | `clean` (no current divergence beyond WS5-11) with the pair list below | Duplicated pairs, all equal in logic today: the runtime core host `shim_host.c:74-671` vs. Amiga `shim_hosted_amiga.c:696-1285` (differs only in `cl_trap`/`cl_panic`/`Open`); `cl_isnan`/`cl_cstring_len`/`cl_write`/`cl_write_line` in all three shims; three strdup helpers (`optional_services.c:24`, `optional_ui.c:19`, `_20.c:95`, flags differ: `MEMF_PUBLIC` not cleared in `_20`); `GfxBase` opened at V33 (`:1307`, `:1604`, `optional_rtg.c:38`) and V37 (`optional.c:911`); `LayersBase` at V33 (`:5859`) and V37 (`optional.c:914`) — whichever runs first wins; `IntuitionBase` opened in two places and missing in a third (ASM-WS5-002); private reply ports (`:5222`, `:2996`); socket connect (`optional_net.c:74`, `:336`); `cl_rexx_delete` and the no-port branch of `cl_rexx_send` (`optional_services.c:417`, `:474`) |

## Findings

### ASM-WS5-001 — A runtime fault exits with OS registrations still installed: a VBlank server is left pointing into the unloaded program

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: A500, A1200 shipped (shared code)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:544-549` (`cl_trap`: `printf`, `fflush`, `exit(20)`), `:562-574` (`cl_panic`), `:640-668` (`AddIntServer` for VERTB/PORTS), at e24a8752
- **Checklist item**: WS5-08 (also WS5-02, WS5-05, WS5-06)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS5/541-fault-handlers.clo` (registers a VBlank handler, creates a public port, then indexes out of range), runner `probes/WS5/54x-fs-uae.sh` (build: `make -C docker exec CMD="bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh build build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/fsuae"`; run on the host: `bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh run build/tmp/assessment/WS5/fsuae 541 541c`). Expected (`541-fault-handlers.expected`: exec `AddIntServer` autodoc — a server must be removed before its code and `Interrupt` structure go away; spec/05 §15 — a fault aborts without the cleanup ladder): the walker's after-line equals its before-line. Observed on A1200/KS3.1: `before vertb 4 …`, `error E11211: index out of range`, `rc 20`, `after vertb 5 …`; the no-fault contrast 541c ends `after vertb 4`. The walker's server count fired on the seeded program (`vertb 4 → 5`). Under `vamos`, probe 503 (`lockPath` then a fault) orphans a 20-byte block, the size of a DOS `FileLock` (`records/WS5-runs.log`), on both profiles and both build lines; its no-fault contrast is clean. Two filler runs (`54x-filler.c`, 541f) did not crash the machine: the freed program memory was not overwritten in those runs, so the crash itself is `probable`, the dangling registration `verified`.
- **Impact**: every program that faults (index out of range, division by zero, out of memory, `assert`, `panic`) after installing an OS registration leaves it behind. For an interrupt server (`addHandler`, `addPortHandler`, and the stdlib users of the dispatcher: input, timer, video) the next program loaded into that memory turns the server into a jump into arbitrary code at 50 Hz — a crash of the whole machine. The same exit skips: worker tasks (left on the ready list in unloaded code), the custom `View` and DMACON (display not restored), windows and screens (IDCMP ports signalling a dead task), the Workbench public-screen lock, public ports (in the system list with a name in freed memory), devices with pending I/O, sprite and audio channels, DOS locks, and every `AllocMem` block (AmigaOS does not reclaim them).
- **Root cause**: the language rule (a fault aborts without drops) is implemented on AmigaOS by libnix `exit()`, which releases only libnix's own stdio and malloc state; the shim keeps no record of the OS resources it acquired, so nothing can release them on the fault path.
- **Recommendation**: a shim-level resource registry: each acquire registers a release record (interrupt server, task, view, window, screen, port, device, lock, sprite, audio, library); `cl_trap`/`cl_panic` (and a libnix `atexit` hook) walk it in reverse before `exit`. The ordinary drop removes the record. Alternatively, as a minimum, remove the two interrupt servers and worker tasks in `cl_trap` — those are the crash class.
- **Effort**: L
- **Related**: ASM-WS5-005 (fault inside a worker task); spec/05 §15 (E4) and spec question SQ-WS5-1; ASM-WS4-003 (the same fault writer)
- **Self-refutation (5.3.1)**: `docs/known-limitations.md` has no entry for OS cleanup after a fault; `run-fault-amiga` checks only the message and exit status. The spec rule that faults skip the ladder is about Clorraine drops; it does not license leaving an interrupt server installed in freed code, which the exec autodoc forbids. Stays Critical.
- **DEF**: DEF-582 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Critical) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the predicted crash is reproduced on both lenses (A1200/KS3.1 Software Failure 8000 0004, A500/KS1.3 Guru 00000004 after the fault; contrasts clean) and the leftover public port shown by a PortList count (0→1; contrast 0→0); 503 also orphans the PathInfo block. Sources `probes/WS5/verify-9.2.4/`.

### ASM-WS5-002 — `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: A1200 shipped (`savePublicScreen`, reproduced); A500 (`showAlert`, reproduced in Phase 9 on A500/KS1.3); A1200 (`showAlert`, code reading)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:1295` (`struct IntuitionBase *IntuitionBase = NULL;`), `:1307-1311` and `:1600-1604` (the only opens: screen and Workbench-window creation), `:2617-2641` (`cl_show_alert` → `DisplayAlert`), `shim_hosted_amiga_20.c:484-496` (`cl_save_public_screen` → `LockPubScreen`), at e24a8752
- **Checklist item**: WS5-02, WS5-04
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS5/542-null-intuition-base.clo`, runner `54x-fs-uae.sh` (commands as in ASM-WS5-001, cases `542 542c`). Expected (`542-null-intuition-base.expected`: `std.amiga.workbench` "Returns false for a null/unknown screen"; intuition `LockPubScreen` returns NULL for an unknown name): `calling`, `saved false`, `rc 0`, `END`. Observed on A1200/KS3.1: 542c (a screen opened first) prints exactly that; 542 (no screen yet) never reaches `END` within 180 s and loses its buffered output — the machine crashed or hung. The link map (`build/tmp/assessment/WS4/size/sieve.clo.map`) shows `IntuitionBase` in the program's own `.bss` and only `libstubs.a(dos.o)` linked, so no libnix auto-open covers it.
- **Impact**: any program that calls `savePublicScreen` (the Workbench capture API) or `showAlert` before opening a screen or window crashes the machine. `showAlert` with no window is the natural "fatal error" path of a CLI tool.
- **Root cause**: the shim defines its own `IntuitionBase` (which suppresses the libnix auto-open object) and opens it lazily only inside the screen/window constructors; other entry points that use Intuition assume an earlier constructor ran.
- **Recommendation**: one `cl_intuition_open()` helper (and `cl_graphics_open()`) called at the top of every entry that uses the base, released at exit (see ASM-WS5-013); a test that calls each Intuition entry first in a fresh program.
- **Effort**: S
- **Related**: ASM-WS5-013; WS5-14 pair list (Intuition opened in two places, missing in a third)
- **Self-refutation (5.3.1)**: neither the stdlib doc comments nor `docs/known-limitations.md` require opening a screen first. The `sdk-a1200-workbench` gate calls `savePublicScreen` after its window is open, which is why it passes. Stays Critical.
- **DEF**: DEF-587 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Critical, verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): 542/542c re-run on A1200; `showAlert` with no screen newly reproduced on A500/KS1.3 ("Software error - task held"; the screen-first contrast shows the alert).

### ASM-WS5-003 — `setCurrentDir` unlocks the launcher's current-directory lock and never restores it

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:448-465` (`cl_dos_set_current_dir`), at e24a8752
- **Checklist item**: WS5-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS5/501-dos-roundtrip.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS5 --clc build/tmp/assessment/baseline/clc --vamos-leaks --only 501"`. Expected (`501-dos-roundtrip.expected`: dos `CurrentDir` returns the old lock, which belongs to the caller's launcher and must be restored before exit; the program unlocks only its own lock): output `true true true true true true`, exit 0. Observed on hosted-amiga and hosted-amiga-13, debug and release: the output matches but the run exits 1; the `vamos` log shows its lock manager failing to release the process's current-directory lock at exit (`KeyError` in `LockManager._unregister_lock`) because the program already unlocked it. The contrast without `setCurrentDir` exits 0.
- **Impact**: from a CLI, the shell's current-directory lock is freed while the shell still holds it (the next command that touches the current directory uses a freed lock); from Workbench, Workbench later unlocks the startup `wa_Lock` a second time. Both are double releases of a filesystem lock — memory corruption in the filesystem handler on a real Kickstart. `examples/amiga/imageviewer.clo` uses `setCurrentDir`.
- **Root cause**: the shim treats the old lock returned by `CurrentDir` as its own and unlocks it, and records nothing to restore at exit.
- **Recommendation**: keep the original lock on the first change; never unlock it; on later changes unlock only locks the program created; at exit (normal and fault, ASM-WS5-001) `CurrentDir(original)` and unlock the program's lock.
- **Effort**: S
- **Related**: ASM-WS5-001
- **Self-refutation (5.3.1)**: the shim comment ("the old lock may be zero at the boot volume root and is not unlocked in that case") shows the release is intentional, not a typo; no limitation covers it. The `vamos` failure is the emulator's own consistency check, not a probe artifact (the contrast passes). Stays Critical (double release of an OS object).
- **Verdict**: downgraded Critical→Minor — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the double release is refuted — from a CLI `UnLock(CurrentDir(new))` is what `C:CD` does, and under Workbench libnix's startup duplicates `wa_Lock` and unlocks the program's lock at exit, so locks balance; what remains is that the launching shell's directory changes and `vamos` exits 1 (A1200 CLI run, no crash).

### ASM-WS5-004 — `drawText` writes unclipped text through a screen `RastPort` that has no layer

- **Severity**: Critical
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:5800-5815` (`cl_text_draw`: `Move` + `Text` on `&screen->RastPort`), `stdlib/std.amiga.text.clo:75` (`drawText`, a safe export), at e24a8752
- **Checklist item**: WS5-09, WS5-04
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. `drawText(screen, font, x, y, pen, text)` passes the caller's `x`, `y`, and the full text length straight to `Text()` on the screen's `RastPort`. A screen `RastPort` has `Layer == NULL`; graphics clips only through layers (graphics autodocs: `ClipBlit` "takes into account the Layers and ClipRects"; `DrawEllipse` "does not clip … to a non-layered rastport"; RKM Graphics: an unlayered RastPort is not clipped). So a position or string extent outside the bitmap makes `Text()` write glyph rows past the bitplanes into adjacent chip memory. The shim's other drawing entries clamp (`cl_video_bitmap_fill`, `:3953-3968`) or draw through a window's layered RastPort (`cl_window_text_draw`).
- **Impact**: a safe program that draws text near the screen edge (a scroller, a status line with a long string, an off-by-one y) overwrites chip memory — other bitplanes, copper lists, or another program's data. Memory unsafety from the safe surface.
- **Root cause**: the wrapper treats `Text()` as bounds-safe; it is not on an unlayered RastPort.
- **Recommendation**: in `cl_text_draw`, compute the extent with `TextExtent`/`TextLength` and the font's baseline/height and reject (or truncate to) text that does not fit the screen bitmap; or draw through a layer.
- **Effort**: S
- **Related**: ASM-WS5-019 (16-bit text length, `RectFill` order); WS6 (safe-surface review)
- **Self-refutation (5.3.1)**: the doc comment bounds only `pen`; the appendix says layer drawing clips, which does not apply to screen drawing. Not reproduced (no off-screen capture taken): `probable`; escalated as a probable Critical per 03-01 §Escalation.
- **DEF**: DEF-585 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Critical; confidence probable→verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): 200 off-screen `drawText` calls hang the A1200; the in-bounds contrast and an out-of-range `fill` complete. Sources `probes/WS5/verify-9.2.4/`.

### ASM-WS5-005 — A runtime fault inside a worker task runs libnix `printf` and `exit()` from a plain Exec task

- **Severity**: Critical
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:3120-3135` (`cl_task_entry` calls the Clorraine entry on a `CreateTask` task), `:544-549` (`cl_trap`), `stdlib/std.amiga.exec.clo:215-222` (the entry "must use only arithmetic … must not call DOS … `std.io`"), at e24a8752
- **Checklist item**: WS5-08, WS5-05
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. The documented contract allows arithmetic in the entry, and the compiler inserts fault checks into arithmetic and indexing (`compiler/src/codegen/expr.rs:985`, `codegen/guards.rs:2118`). A division by zero or an index fault in the entry calls `cl_trap`, which calls libnix `printf` (DOS `Write` from a task without a `Process` structure) and `exit()` (unwinding the main program's startup frame from another task's stack). RKM Libraries: DOS calls require a Process.
- **Impact**: a worker that divides by zero crashes the machine instead of reporting a fault; the user cannot honor the "no DOS" rule because the compiler adds the DOS call.
- **Root cause**: one fault path for every execution context; it assumes it runs in the main process.
- **Recommendation**: in `cl_trap`/`cl_panic`, detect a non-process task (`FindTask(NULL)->tc_Node.ln_Type == NT_TASK`) and record the fault in the task record, signal the parent, and `Wait(0)`; `joinTask` reports it as a fault in the parent.
- **Effort**: M
- **Related**: ASM-WS5-001
- **Self-refutation (5.3.1)**: the doc comment forbids DOS calls by the user, not compiler-inserted ones; no limitation covers faults in workers. Not reproduced (`probable`; removed by an FS-UAE probe dividing by zero in a worker).
- **DEF**: DEF-586 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Critical; confidence probable→verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): a division by zero in a `createTask` worker prints the fault line and hangs the A1200; the contrast joins (`joined 100`). Sources `probes/WS5/verify-9.2.4/`.

### ASM-WS5-006 — Safe handles let a borrowed OS object outlive its owner: clip regions and AmigaGuide messages

- **Severity**: Critical
- **Confidence**: probable
- **Lens**: A1200 shipped (`hosted-amiga` modules)
- **Location**: `compiler/runtime/shim_hosted_amiga_optional.c:938-949` (`cl_free_region` disposes without checking installation), `:1031-1041` (`cl_layer_set_clip`), `shim_hosted_amiga.c:5891-5899` (`cl_layer_free` calls `DeleteLayer` without `InstallClipRegion(l, NULL)`), `stdlib/std.amiga.regions.clo:5-6` and `:85-92` (the rule is a doc comment), `shim_hosted_amiga_optional_net.c:211-220`, `:291-300` (`cl_guide_close`, `cl_guide_reply`), at e24a8752
- **Checklist item**: WS5-06, WS5-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading against layers `InstallClipRegion` ("You MUST remember to call InstallClipRegion(l,NULL) before calling DeleteLayer(l) … if you have installed a non-NULL ClipRegion"). `layerSetClipRegion(borrow layer, borrow region)` ends its borrow at the call, so `freeRegion(region)` or the layer's drop is accepted afterwards; the shim then disposes a region still referenced by `layer->ClipRegion`, or deletes a layer with a user clip installed. `waitAmigaGuideMessage` returns a `GuideMessage` with no link to its `AmigaGuide`; `closeAmigaGuide(g)` followed by `replyGuideMessage(m)` (or the message's drop) calls `ReplyAmigaGuideMsg` after `CloseAmigaGuide` freed the context.
- **Impact**: a program the compiler accepts can make the layers library use a freed region on the next refresh, or reply a message into a closed context — use-after-free from the safe surface. The regions rule is documented, so the risk is a user mistake the type system does not catch.
- **Root cause**: the ownership model sees only call-scoped borrows; the OS keeps the pointer past the call, and the shim does not track the relationship.
- **Recommendation**: track it in the shim: a region records the layer it is installed in, and `cl_free_region` uninstalls it; `cl_layer_free` installs NULL first; a guide context keeps a list of outstanding messages, `cl_guide_close` replies them, and a later reply of an orphaned message is a no-op. Alternatively model the relationship in the stdlib types (the layer takes the region).
- **Effort**: M
- **Related**: WS6-02 (handle lifetimes), ASM-WS5-019 (semaphore freed while owned)
- **Self-refutation (5.3.1)**: the regions doc comment makes the misuse a documented user error, but spec/17 §10 places memory safety "above the hardware boundary" without exceptions for the stdlib's safe exports. Not reproduced (`probable`).
- **DEF**: DEF-593 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Critical, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the code contradicts the layers `InstallClipRegion` rule; one region reproduction attempt showed no failure (memory not reused); the AmigaGuide half was not attempted.

### ASM-WS5-007 — `cardPresent()` and `cardStatus()` remove `card.resource` from the system

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A1200 shipped (A600/A1200 PCMCIA)
- **Location**: `compiler/runtime/shim_hosted_amiga_20.c:393-401` and `:406-416` (`RemResource(card)` after `OpenResource`), at e24a8752
- **Checklist item**: WS5-02, WS5-04
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS5/540-card-resource.clo`, runner `54x-fs-uae.sh` (cases `540 540c`). Expected (`540-card-resource.expected`: exec `RemResource` "removes an existing resource from the system resource list. There must be no outstanding users"; `OpenResource` has no close; `cardPresent` "True when card.resource can be opened"): `card true true`, walker `card 1` before and after. Observed on A1200/KS3.1: `card true false`, walker `after … card 0`; the contrast without the calls keeps `card 1`. The walker's resource check fired on the seeded program.
- **Impact**: after one call, `card.resource` is gone until reboot: the second call reports no PCMCIA, and every later `OpenResource("card.resource")` in the system fails (new users of the PCMCIA slot: carddisk, network and CF drivers being started). No crash was observed; existing users keep their pointer.
- **Root cause**: the code treats `RemResource` as the release that pairs with `OpenResource`.
- **Recommendation**: delete both `RemResource` calls (resources are not closed).
- **Effort**: S
- **Related**: `examples/amiga/services.clo:50` calls it once, which is why the A600 gate passes
- **Self-refutation (5.3.1)**: rated Major, not Critical: wrong result and system damage until reboot, but no crash, hang, or data loss shown.
- **DEF**: DEF-599 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): reproduced exactly (`card true false`, `card 0` after).

### ASM-WS5-008 — The screen capture misreads the 2.0+ colour table, and the A1200 Workbench golden pins the wrong colours

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A1200 shipped (every 2.0+ capture); A500 unaffected (`Type == 0` branch)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:2764-2779` (`cl_screen_save_native`, the `else` branch reads `table[index * 3 + k]` bytes), used by `shim_hosted_amiga_20.c:494`, at e24a8752
- **Checklist item**: WS5-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `convert docs/amiga-sdk/a1200-workbench.png -format %c histogram:info:-` lists the Workbench capture's colours as `(170,170,0)` (617,972 px), `(102,187,0)`, and `(0,255,255)`. The NDK (`graphics/view.h:145-177`) keeps `ColorMap.ColorTable` as 12-bit `UWORD`s in every version (V39 adds `LowColorBits` separately). Decoding the KS3.1 default words `0AAA 0000 0FFF 068B` as byte triples with `(b & 0x0F) * 17` gives exactly `(AA,AA,00)`, `(00,FF,FF)`, `(66,BB,…)`: the expected grey, black, and white are captured as yellow, cyan, and green. The branch also reads `count` bytes past the end of the table.
- **Impact**: every native capture on 2.0+ records wrong colours; the `sdk-a1200-workbench` gate compares against a golden made by the same code, so it pins the defect instead of catching it.
- **Root cause**: the V36+ branch assumes a byte-triple table that no `ColorMap` type has.
- **Recommendation**: read colours with `GetRGB4(colormap, index)` on every version (or `GetRGB32` on V39 for 8-bit precision); regenerate the A1200 Workbench goldens.
- **Effort**: S
- **Related**: WS8 test integrity (a golden that encodes a defect); `docker/scripts/cscr2png.py`
- **Self-refutation (5.3.1)**: the colours could in principle be a deliberate palette, but the Workbench 3.1 default palette is grey/black/white/blue and the observed values are the byte-misread prediction to the last digit. Major (wrong output, out-of-bounds read of OS memory).
- **DEF**: DEF-597 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the histogram matches the byte-triple decode; the A600/KS2.05 gate artifact is also yellow/cyan.

### ASM-WS5-009 — The interrupt servers return with the user handler's condition codes and can end the VERTB chain

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:600-612` (`static void cl_vblank_code(void)`, `cl_ports_code`), `:643` (priority 0), at e24a8752
- **Checklist item**: WS5-05, WS5-09
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -S` of probe 541's emission (`build/tmp/assessment/WS5/fsuae/541.s`): `_cl_vblank_code` is `move.l _cl_vblank_handler,a0; cmp.w #0,a0; jeq L7; jmp (a0)` — a tail call — and the handler `onTick` ends `move.l d0,_cl_Probe_Main_ticks; rts`, which leaves Z clear for a non-zero count. Exec `AddIntServer` autodoc: "Each link in the chain will be called … until … one of the servers returns with the 68000's Z condition code clear … VERTB servers should always return Z set. (Take care with High Level Language servers…)".
- **Impact**: while a Clorraine VBlank handler is installed, servers below priority 0 on the VERTB chain (and on PORTS, CIA-driven and expansion-card servers) are skipped on most frames. What runs below priority 0 depends on the system (third-party drivers, commodities); the A1200 runs in 541c did not show a visible failure.
- **Root cause**: a `void` C function cannot set the condition codes the exec protocol needs, and the tail call hands the decision to user code.
- **Recommendation**: make the servers return `LONG` 0 after the call (GCC then emits `moveq #0,d0; rts`, which sets Z) and stop the tail call; for PORTS, return 0 (not "handled") unless the source is known.
- **Effort**: S
- **Related**: ASM-WS5-001 (the same servers after a fault)
- **Self-refutation (5.3.1)**: the code shape is measured, the protocol is the autodoc's; the visible impact is system-dependent, hence Major.
- **DEF**: DEF-600 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): regenerated assembly shows the tail call and Z clear.

### ASM-WS5-010 — The busy-pointer image has no sprite header or terminator, so sprite DMA reads past the 64-byte allocation

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:1412` (`CL_WINDOW_POINTER_BYTES 64`), `:2648-2680` (template of 32 words, `SetPointer(…, 16, 16, 0, 0)`), at e24a8752
- **Checklist item**: WS5-03, WS5-09
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading against the RKM pointer format and graphics `ChangeSprite` (sprite data = 2 position/control words, then height × 2 words, then 2 zero words): a 16-row pointer needs 36 words (72 bytes). The buffer holds 32 words with neither part, so `ChangeSprite` writes the control words over row 0 and the DMA fetches row 16 and the terminator from 8 bytes past the allocation.
- **Impact**: `setPointer` shows a corrupted pointer (first row lost, trailing garbage lines from whatever follows in chip memory). Not visible to the bitplane-only capture of the GUI gate.
- **Root cause**: the template stores image words only.
- **Recommendation**: allocate 72 bytes, place the image at word 2, zero the first and last two words.
- **Effort**: S
- **Related**: ASM-WS5-019 (sprite channel on screen close)
- **Self-refutation (5.3.1)**: not captured on screen: `probable`. A DMA over-read, not a write: Major.
- **DEF**: DEF-607 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): intuition `SetPointer` takes a full sprite definition; not reproduced visually.

### ASM-WS5-011 — `FileInfoBlock`s on the stack are not guaranteed longword-aligned

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:320`/`:355` (`cl_dos_volumeinfo_new`), `:471`/`:481` (`cl_dos_current_dir_name`), `:497`/`:508` (`cl_dos_parent_dir_name`), at e24a8752
- **Checklist item**: WS5-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. dos `Examine` autodoc: "infoBlock - pointer to a FileInfoBlock (MUST be longword aligned)… AllocDosObject() will allocate them correctly". These three functions declare `struct FileInfoBlock fib;` as a local; m68k-amigaos GCC aligns stack objects to 2 bytes and does not realign the stack, so `&fib` may be 2 mod 4. The filesystem receives `fib >> 2` as a BPTR and writes 2 bytes below the struct. `cl_dos_pathinfo_new` is correct (its FIB is the first member of an `AllocMem` block).
- **Impact**: depending on the stack frame, `volumeInfo`, `currentDirName`, or `parentDirName` corrupts two bytes of the caller's frame and returns a shifted name. Latent: the current frame layout may happen to align.
- **Root cause**: a local FIB.
- **Recommendation**: allocate the FIB with `AllocMem` (1.3) or `AllocDosObject(DOS_FIB)` (2.0+), as `cl_dos_pathinfo_new` does.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: probes 501/502 pass under `vamos`, which does not model BPTR truncation. `probable`.
- **Verdict**: downgraded Major→Minor — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): GCC keeps frames a multiple of 4 and the FIB fills the 260-byte frame, so `&fib` is longword-aligned whenever the process stack is; the autodoc rule stands, the misalignment is a hypothesis.

### ASM-WS5-012 — GadTools: the last gadget is never attached, rendered, or freed

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga_optional_ui.c:236-252` (`cl_gt_build`), `:267` (`cl_gt_free`), `:562` (same `AddGList == 0` test), at e24a8752
- **Checklist item**: WS5-04
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. `CreateContext` puts a context gadget at the head of `glist`, so the chain has `count + 1` gadgets; `AddGList`, `RefreshGList`, and `RemoveGList` are all passed `count`, which covers the context gadget and the first `count − 1` user gadgets. `AddGList` relinks the last added gadget's `NextGadget`, cutting the last user gadget out of the chain that `FreeGadgets` later walks. Intuition `AddGList` "Returns the position of where the first gadget in the list was actually added", so `== 0` is a valid position, not a failure; if it fires, the attached list is freed.
- **Impact**: every GadTools window shows one gadget fewer than the program created, and leaks it. The A600 gate counts capture colours only, so it does not catch this.
- **Root cause**: off-by-one on the context gadget and a wrong return-value test.
- **Recommendation**: pass `-1` (all) or `count + 1`; drop the `== 0` failure test; guard against rebuilding after a partial failure.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: not captured on screen (`probable`).
- **DEF**: DEF-598 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major; evidence now verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the existing `sdk-a600-addons` capture (`build/amiga-sdk/a600-addons/intuition.png`, 2026-11-24, shim unchanged since 2026-11-06) shows three of the four gadgets the program adds while it reports `gad 4`.

### ASM-WS5-013 — The shim's own library bases are never closed

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:1294-1297` (`GfxBase`, `IntuitionBase`, `DiskfontBase`, `LayersBase`, initialized definitions) and their lazy opens (`:1307`, `:1311`, `:1600-1604`, `:3682`, `:5742`, `:5859`); the optional suffixes' bases (`optional.c:848`, `:1062`; `optional_net.c:6`, `:16`; `optional_rtg.c:18`; `optional_services.c:19-22`; `optional_ui.c:16-17`; `_20.c:12`, `:82`, `:311-312`), at e24a8752
- **Checklist item**: WS5-02
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading plus measurement: `grep -n CloseLibrary compiler/runtime` finds only the user wrapper (`shim_hosted_amiga.c:3254`); the link map shows the initialized bases are the program's own symbols (`IntuitionBase` in `.bss`, only `libstubs.a(dos.o)` linked). Exec `OpenLibrary` requires a matching `CloseLibrary`. For the uninitialized (common) bases the picture is mixed: probe 510 uses `UtilityBase` and is leak-free under `vamos`, so libnix manages that one; the others were not probed.
- **Impact**: each run leaves an open count on graphics, intuition, layers, and diskfont (and the 2.0+ libraries a program used): disk-loaded `diskfont.library` and the optional libraries can never be expunged, so memory is lost until reboot; for `bsdsocket.library` the per-task base is never closed, so TCP/IP stacks keep the exited task's sockets and signal bindings.
- **Root cause**: lazy opens with no exit hook.
- **Recommendation**: record each lazy open and close it from the exit path (ASM-WS5-001's registry or a libnix destructor).
- **Effort**: S
- **Related**: ASM-WS5-001, ASM-WS5-002
- **Self-refutation (5.3.1)**: for the ROM-resident graphics and intuition the leak is an open count only; diskfont, bsdsocket, and the optional libraries carry the real cost. Major (shim leak per the rubric).
- **DEF**: DEF-604 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): only the user wrapper calls `CloseLibrary`; an open-count detector could not fire even on a seeded leak of ROM libraries (inconclusive).

### ASM-WS5-014 — Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires

- **Severity**: Minor
- **Confidence**: probable
- **Lens**: A500 with expansion memory (not the stock 512 KB A500 lens, which has only chip RAM)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:3459` (`AllocMem(length, MEMF_PUBLIC | MEMF_CLEAR)`), `:3516-3518` (a caller slice passed straight to `DoIO`), at e24a8752
- **Checklist item**: WS5-03
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. trackdisk `CMD_READ`/`CMD_WRITE`/`TD_FORMAT` notes: "Under versions of Kickstart earlier than V36, the io_Data had to point to a buffer in chip memory"; `TD_RAWREAD`/`TD_RAWWRITE` require chip memory on every version. The generic device opener accepts `trackdisk.device`.
- **Impact**: on an A500 with trapdoor or fast RAM, disk reads land in the wrong memory and writes put garbage on the disk (data loss). The stock A500 lens is unaffected, so the severity comes from the shipped lens (Minor); for real 1.3 machines with memory expansion it is a data-loss defect.
- **Root cause**: `MEMF_PUBLIC` instead of `MEMF_CHIP`, and no bounce for caller slices.
- **Recommendation**: `MEMF_CHIP` for device buffers on the 1.3 profile; bounce non-chip caller buffers when `SysBase->LibNode.lib_Version < 36`, or reject them.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: the lens rule caps it at Minor; Phase 9 may weigh the expansion-RAM case.
- **Verdict**: confirmed (Minor, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4).

### ASM-WS5-015 — Commodity brokers are never activated

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga_optional_services.c:100-112` (`CxBroker` with `nb_Flags = 0`; no `ActivateCxObj` anywhere in `compiler/runtime/`), `:125-127` (`DeleteCxObjAll` without draining the port), at e24a8752
- **Checklist item**: WS5-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. commodities `ActivateCxObj`: "All objects are created in the 'active' state except brokers which are created 'inactive'. Thus, after you create your broker … you must remember to use this function to activate it." `grep ActivateCxObj compiler/runtime` finds nothing. On close, queued `CxMsg`s on the borrowed port are not replied.
- **Impact**: a Clorraine commodity never receives its hotkeys or input events — the module's purpose does not work; closing with messages queued leaves them unreplied.
- **Root cause**: missing call.
- **Recommendation**: `ActivateCxObj(broker, TRUE)` after building the broker; drain and reply the port after `DeleteCxObjAll`.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: the stdlib exports no activate call either; the gate (`sdk-a600-services`) asserts creation only. `probable`.
- **Verdict**: downgraded Major→Minor — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the module does not expose the input-event network, so no hotkey can depend on activation; what remains is an inactive broker in Exchange and undrained CxMsgs at close.

### ASM-WS5-016 — The clipboard read never finishes the read protocol

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga_optional_services.c:687-710` (`cl_clipboard_read`: one `CMD_READ`, never past the end), at e24a8752
- **Checklist item**: WS5-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. clipboard `CMD_READ`: "When io_Offset is beyond the end of the clip, this acts as a signal to the clipboard device that the application is through reading this clip… while an application is in the middle of reading a clip, any attempts to write new data to the clipboard are held off."
- **Impact**: after `clipboardRead`, writes to that unit by any program are held off until the handle is closed; a read followed by a write on the same handle may block indefinitely (hang — not reproduced, hence Major).
- **Root cause**: the read is a single request sized to the caller's buffer.
- **Recommendation**: after the data read, issue a read with `io_Data = NULL` and a large `io_Length` (the autodoc's skip-to-end idiom) to finish the clip.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: if `CloseDevice` ends the read, the cross-program effect lasts only while the handle lives; the same-handle write case remains. `probable`.
- **DEF**: DEF-588 (`plans/00-roadmap.md`)
- **Verdict**: upgraded Major→Critical (confidence probable→verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): on A1200/KS3.1 with the Workbench 3.1 clipboard.device, a `clipboardWrite` after `clipboardRead` on the same handle never returns (step markers stop after the read); the write-write-read contrast completes. Escalated 2026-11-26 02:43. Sources `probes/WS5/verify-9.2.4/`.

### ASM-WS5-017 — Layer drawing returns without `WaitBlit`, so CPU access to the same bitmap races the blitter

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:5974-5975` (`RectFill`), `:5986` (`SetRast`), `:6007` (`Text`) in the layer helpers; CPU readers at `:3919-3926`, `:3986`, `:4198-4205`, `:4227`, at e24a8752
- **Checklist item**: WS5-09
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. The layer helpers draw into a caller-owned `cl_video_bitmap` with blitter-backed graphics calls and return; the same bitmap can then be read or written by the CPU paths (`bitmapPlot`, `bitmapChecksum`, `bitmapClear`). The shim's own blitter paths call `WaitBlit()` for exactly this reason (`:3973`, `:4148`, `:4180`); RKM: wait for the blitter before the CPU touches memory it may still be writing.
- **Impact**: a checksum or pixel read right after a layer fill can see partly drawn data; CPU writes can be overwritten by the finishing blit. Intermittent, timing-dependent wrong output.
- **Root cause**: missing `WaitBlit` in three helpers.
- **Recommendation**: `WaitBlit()` before returning from the layer drawing helpers (or before every CPU access path).
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: graphics drawing calls do not wait for completion before returning (that is why the shim waits elsewhere). `probable`.
- **DEF**: DEF-605 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4).

### ASM-WS5-018 — Classic menus are never laid out: every menu header and item sits at the same position

- **Severity**: Major
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:2372`, `:2397` (`menu->LeftEdge = (WORD)(index * CL_MENU_WIDTH)` while `index` is still 0), `:2403-2413` (index computed afterwards), `:2474-2486` (items never set `TopEdge`), at e24a8752
- **Checklist item**: WS5-04
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. `LeftEdge` is assigned from `index` before the loop that counts the existing menus, so every header gets x = 0; items are allocated `MEMF_CLEAR` and never given a `TopEdge`, so every item overlaps item 0. Classic `SetMenuStrip` does not lay out menus; the application must (RKM Intuition, Menus).
- **Impact**: a program with two or more menus, or two or more items in a menu, shows overlapping entries; only the first is reachable. The GUI gate drives menus with Amiga-key shortcuts, which do not depend on layout.
- **Root cause**: statement order and a missing item layout.
- **Recommendation**: compute the index first; set `TopEdge = index * CL_MENU_HEIGHT` per item.
- **Effort**: S
- **Related**: —
- **Self-refutation (5.3.1)**: not captured on screen (`probable`).
- **DEF**: DEF-606 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4).

### ASM-WS5-019 — Lower-impact shim defects (grouped)

- **Severity**: Minor
- **Confidence**: probable (items marked *h* are `hypothesis`)
- **Lens**: as listed
- **Location**: as listed, at e24a8752
- **Checklist item**: WS5-01, WS5-04, WS5-06, WS5-08, WS5-09, WS5-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading against the cited autodocs.

  | # | Defect | Location | Source |
  |---|--------|----------|--------|
  | a | `layerBeginUpdate` FALSE is not paired with `EndUpdate(l, FALSE)`; the stdlib says to skip the end call, leaving the layer locked | `shim_hosted_amiga.c:6019`, `:6028`; `std.amiga.layers.clo:82` | layers `BeginUpdate` BUGS |
  | b | `openDisplay` adds ViewPort offsets 0x81/0x2C on top of `InitView`'s default position (display pushed off the visible area; the demos use `openDisplayAt(…, 0, 0)`) | `:4515`, `:4493-4499` | graphics `InitView` |
  | c | Raster DMA is forced on and the previous DMACON is not restored; written to native hardware on RTG systems too | `:4586-4587`, `:4622-4625` | RKM Hardware |
  | d | `layerFill` passes unordered coordinates to `RectFill` | `:5974-5975` | graphics `RectFill` (xmax ≥ xmin) |
  | e | Text lengths ≥ 65,536 wrap (V34 vectors take D0.W) | `:5786`, `:5814`, `:5836`, `:6007` | graphics `Text`/`TextLength` |
  | f | A V44 tag (`WBAPPICONA_PropagatePosition`) and a V38 tag (`ASLFR_DoSaveMode`) are passed to libraries opened at V37 (the save requester is a load requester on 2.04/2.05) | `_20.c:88`, `:155`; `optional_ui.c:46`, `:416` | workbench/asl autodocs |
  | g | An AppWindow does not retain its window (closing the window first leaves Workbench holding it) | `_20.c:460-470` | workbench `AddAppWindowA` |
  | h | The sprite channel is not blanked or released when its screen closes; the chip image is later freed while a copper list may still point at it | `:4866-4881`, `:4755-4773` | graphics `FreeSprite` |
  | i | The ASL drawer split: `"DH0:foo"` keeps the whole string as the drawer | `optional_ui.c:366-368` | asl `ASLFR_InitialDrawer` |
  | j | A received ARexx message is replied without setting `rm_Result1`/`rm_Result2` | `optional_services.c:517-526` | rexxsyslib |
  | k | `cl_nv_store` rounds `length` up to tens of bytes as the autodoc prescribes, so `StoreNV` reads up to 9 bytes past the caller's slice into NVRAM | `optional_services.c:1054-1055` | nonvolatile `StoreNV` |
  | l | A semaphore can be freed while owned or waited on; cross-task objects use `MEMF_ANY` instead of `MEMF_PUBLIC` | `optional.c:43-57`; `optional_services.c:86-88`, `:387-388` | exec `InitSemaphore`, RKM memory |
  | m *h* | `serial.device` `OpenDevice` fills the request with Preferences defaults, overwriting the requested baud before `SDCMD_SETPARAMS` | `optional_services.c:760-770` | RKM Devices, Serial |
  | n *h* | `DTM_DRAW` without `DTM_OBTAINDRAWINFO`/`DTM_RELEASEDRAWINFO`; the result is ignored | `optional.c:1206-1214` | datatypes `DrawDTObjectA` |
  | o *h* | `narrator.device` is missing from the extended-request list, so opening it with the generic `IOStdReq` lets the device write defaults over the handle fields | `:3283-3289`, `:3392-3393` | narrator.device (not in the NDK 3.2 autodocs) |
  | p | Hand prototypes for `DateStamp`, `Lock`, … differ from the NDK in return type and `CONST_STRPTR` | `:65-82` | `clib/dos_protos.h` |

- **Impact**: individually limited (wrong layout or colours, a stuck layer on low memory, an ignored tag, a reply without a result code); none is shown to corrupt memory from the safe surface except *h* (DMA reads freed chip memory until the next `MrgCop`) and *o* (hypothesis).
- **Root cause**: per item.
- **Recommendation**: per item, as in the source column; each is a local fix.
- **Effort**: M (all items)
- **Related**: ASM-WS5-004 (screen text), ASM-WS5-010 (pointer sprite)
- **Self-refutation (5.3.1)**: (k) follows the autodoc's own example, so it is an edge on the caller's slice, not a protocol error; (o) cannot be checked against a local autodoc and stays a hypothesis.
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): items a, h, k, l spot-checked against code and autodocs.

### ASM-WS5-020 — No Ctrl-C handling in any blocking shim wait

- **Severity**: Minor
- **Confidence**: probable
- **Lens**: A500, A1200 shipped
- **Location**: `compiler/runtime/shim_hosted_amiga.c:2922` (`WaitPort` on the IDCMP port), `:3148` (task join), `:3518`, `:3572` (`DoIO`/`WaitIO`), `:5637` (timer), `shim_hosted_amiga_20.c:255` (AppMessage wait), at e24a8752
- **Checklist item**: WS5-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading: no `SIGBREAKF_CTRL_C` token in `compiler/runtime/` or `stdlib/`; every blocking wait waits on its own signal only.
- **Impact**: a Clorraine program started from a shell cannot be interrupted with Ctrl-C while it waits for events, a device, a timer, or a worker; the user must close its window or reboot. Owned resources are released only if the program reaches its normal exit.
- **Root cause**: no break-signal surface in the stdlib or the shim.
- **Recommendation**: OR `SIGBREAKF_CTRL_C` into the blocking waits and surface it (a `breakRequested()` query, or an event kind), so a program can exit through its normal drops.
- **Effort**: M
- **Related**: ASM-WS5-001
- **Self-refutation (5.3.1)**: many Amiga programs ignore Ctrl-C; the checklist asks what happens, and the answer is "nothing". Minor.
- **Verdict**: confirmed (Minor, probable) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4).

### ASM-WS5-021 — Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove`

- **Severity**: Minor
- **Confidence**: probable (the NULL-`memcpy` reachability is a *hypothesis*)
- **Lens**: host; freestanding (provisional profile)
- **Location**: `compiler/runtime/shim_host.c:106-114`, `:212`, `:229` (`cl_memcpy`, `cl_buffer_append`, `cl_buffer_write`; the same in `shim_hosted_amiga.c:728-736`, `:834`, `:851`), `shim_host.c:645-654` (`Read`/`Write` cast an `int` length to `size_t`), `shim_freestanding.c:40-81` (no `memmove`), `:171-187`, `:325-328` (generic-fallback `_start`), at e24a8752
- **Checklist item**: WS5-11, WS5-12
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading. C99 7.21.1p2/7.1.4: `memcpy(dst, NULL, 0)` is undefined; `cl_string_append` guards `n != 0`, these do not. No probe produced a NULL slice pointer (empty arrays and buffers allocate a 1-byte block), so reachability is unproven. A length ≥ 2 GiB passed as `i32` wraps to an unbounded `fread` on the host. The GCC manual requires a freestanding environment to provide `memmove` (GCC may emit it); probe 520 shows today's corpus needs none.
- **Impact**: latent; no current program is affected.
- **Root cause**: missing guards.
- **Recommendation**: guard `n != 0` before `memcpy`; reject negative lengths; add `memmove` to the freestanding shim; drop `_start` from the generic fallback.
- **Effort**: S
- **Related**: WS2 sanitizer runs (clean)
- **Self-refutation (5.3.1)**: Minor (latent).
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4).

### ASM-WS5-022 — Integer formatting divides with libgcc's 32-bit routine on the 68000

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500
- **Location**: `cl_format_u32` in `compiler/runtime/shim_hosted_amiga.c` (digit loop with `/ 10` and `% 10` on `ULONG`), at e24a8752
- **Checklist item**: WS5-13
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS4/430-patch-measure.py build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/patch 530"` (A500 -O2, `vamos` 68000 trace): `strings` 1,688,727 → 998,843 instructions (−689,884, −40.9% of net 1,685,808), `.text` 23,920 → 23,988 (+68 B), output identical (`records/WS5-runs.log`). The patch (`probes/WS4/530-format-u32-16bit.diff`) splits the value with one 32-bit division by 10,000 and formats each part with the 68000's 16-bit `divu`.
- **Impact**: every template placeholder and `toString` of an integer on the 68000 pays for `__udivsi3` per digit; this is most of ASM-WS4-001's `strings` gap.
- **Root cause**: generic 32-bit division on a CPU without a 32/32 divide.
- **Recommendation**: the 16-bit split (as patched), or a reciprocal-multiply digit loop.
- **Effort**: S
- **Related**: ASM-WS4-001 (templates cost 33% more than C on the A500)
- **Self-refutation (5.3.1)**: an optimization below the Major threshold on its own; the Major is ASM-WS4-001.
- **Verdict**: confirmed (Opportunity, verified) — verifier ses_dea9a6107ffefWXd6gFWNJQszZ (9.2.4): the 530 measurement reproduces exactly.

### ASM-WS5-023 — `clipboardWrite` reports success for raw data that the 3.1 clipboard device discards, so `clipboardRead` never sees it

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A1200 shipped (`hosted-amiga`, `std.amiga.clipboard`); other 2.0+ versions not tested
- **Location**: `compiler/runtime/shim_hosted_amiga_optional_services.c:658-685` (`cl_clipboard_write`: raw `CMD_WRITE` of the caller's bytes, then `CMD_UPDATE`, returns TRUE), `stdlib/std.amiga.clipboard.clo:34-40` ("Writes the whole clip and marks it current; true on success"), at e24a8752
- **Checklist item**: WS5-06 (also WS5-04)
- **Author session**: ses_dea9a6107ffefWXd6gFWNJQszZ (raised by the 9.2.4 verifier; recorded by the executing session)
- **Evidence**: Reproduction on A1200/KS3.1 with the licensed Workbench 3.1 `Devs/clipboard.device` and `Assign CLIPS: RAM:` (runner `probes/WS5/verify-9.2.4/run-a1200.sh`, sources `clipiff.c`, `clipdump.c`, `clipw.clo`, `clipr.clo`, `clipc.clo` in the same folder). A C control writing with the shim's exact sequence (`io_ClipID = 0`, `CMD_WRITE`, `CMD_UPDATE`): raw 4 bytes → `write 0 update 0 id 2`, then an autodoc-conformant dump reads `actual 0 id -1`; the same bytes wrapped as `FORM…FTXT CHRS` → the dump reads 24 bytes and the shim's `clipboardRead` returns 16. With the shim, `clipw.clo` prints `second program write true`, yet the following dump reads 0 bytes; `clipc.clo` (write, write, read) reads `r2 0`.
- **Impact**: a program that copies text or bytes with `clipboardWrite` gets `true`, but nothing reaches the clipboard, and `clipboardRead` of raw data always returns 0. The `sdk-a600-addons` gate asserts only the write's return value (`clip: ok n 4`).
- **Root cause**: the wrapper treats the clipboard as a raw byte store; the device keeps only IFF clips, and its success codes do not report a discarded clip.
- **Recommendation**: wrap text in an IFF `FTXT`/`CHRS` clip on write and unwrap on read (or expose an IFF-level API); make the gate read the clip back from a second program; or document that raw clips are unsupported and return false.
- **Effort**: S
- **Related**: ASM-WS5-016; ASM-WS8-002
- **Phase 9 correction**: impact — `clipboardWrite` returns true while a raw clip is either dropped (no clip yet) or stored altered (padded to 8 bytes after an IFF clip exists); root cause — the device treats clips as IFF and has no defined behavior for non-IFF data; the known-limitations entry (`docs/known-limitations.md:62-64`) misattributes this symptom to the environment.
- **DEF**: DEF-595 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) with a corrected impact and root cause — verifier ses_dea685c4bffegV12VLXpl04mC6 (9.2.5, a session other than the raiser): on a fresh unit a raw write is discarded; after an IFF clip exists the same 4-byte raw write is stored as an 8-byte clip and read back as 8 bytes; both report success. `docs/known-limitations.md:62-64` attributes the same symptom to the probe environment.

## Spec questions

- **SQ-WS5-1** — spec/05 §15 says runtime faults abort without the cleanup ladder (E4).
  On AmigaOS, aborting without releasing OS registrations crashes the machine
  (ASM-WS5-001). Should the spec require the hosted profiles' fault exit to release
  runtime-owned OS resources (interrupt servers, tasks, display, ports, locks), while
  still skipping user drops? ASM-WS5-001 stands as a platform defect either way.
- **SQ-WS5-2** — `std.amiga.regions` documents a lifetime rule ("clear the clip before
  freeing the region") that the safe surface does not enforce (ASM-WS5-006). Does
  spec/17 §10 ("memory safety is strong above the hardware boundary") allow a safe
  stdlib export to rely on a documented ordering rule?

## Falsifiability log

| Probe | Form | Result |
|-------|------|--------|
| detector (`--vamos-leaks`) | seeded defect `500-seeded-leak.c` (an open library and an orphan block) | fired: "can't expunge … utility.library", "orphan" |
| 500 | contrast (seeded output) | discriminates |
| 501 | contrast (no `setCurrentDir`) | contrast exits 0, main exits 1 |
| 502 | contrast (`deleteFile` result replaced) | discriminates |
| 503 | contrast (in-range index, no fault) | contrast leak-free, main orphans 20 bytes |
| 504 | contrast | probe design error (undrained port); replaced by 507 |
| 505, 506, 507, 510 | contrast (changed payload, library name, tag value) | discriminate |
| 520 | script; the expectation names the undefined-symbol list (empty) | `limit` for seeding: no seeded libc reference was linked; the `nm -u` listing is direct evidence |
| 530 | output comparison of the patched C | identical output; seeded mismatch shown to fail in the WS4 red check |
| walker (`54x-walker.c`) | seeded defect `54x-seeded.c` | server count fired (4 → 5), resource check fired (card 1 → 0); port check did **not** fire (`limit`, see Scope) |
| 540 | contrast 540c (no call) | contrast keeps `card 1`; main `card 0` |
| 541 | contrast 541c (no fault) | contrast `vertb 4`; main `vertb 5` |
| 541f | contrast 541cf, filler `54x-filler.c` | both survived (filler did not reach the freed memory): the crash is not shown |
| 542 | contrast 542c (screen opened first) | contrast `saved false` and END; main no END |
| host shim | the WS2 sanitizer self-test (seeded leak, double free, use-after-free, overflow) | fired |

## Self-refutation (5.3.1)

Checklist: `codeops/specialists/amiga-platform-reviewer.md`.

- **Minimum version at open (inv. 2)**: confirmed the V37 opens behind V38/V44 tags
  (ASM-WS5-019 f) and the missing open (ASM-WS5-002); no 37+ request on a `core` path.
- **Header availability (inv. 3)**: the 1.3 builds of all Phase 0 programs compile
  (`records/WS0-emission-record.tsv`, 31 `hosted-amiga-13` cells); no 2.0 header in the
  shared shim.
- **Command semantics (inv. 4)**: the only 1.3-specific I/O difference found is the
  trackdisk chip-memory rule (ASM-WS5-014), rated by the stock-A500 lens.
- **Release order and handle lifetimes (inv. 6)**: ASM-WS5-006 and ASM-WS5-019 l, g.
- **Evidence honesty (inv. 7)**: every A500 statement is code reading (Scope); the FS-UAE
  runs show `card 1` before every case, which only an A600/A1200-class ROM provides, but
  the emulator log was not kept, so the ROM version is taken from `docker/.env`
  (`amiga-os-310-a1200.rom`) rather than the run log.
- **Gate review**: `sdk-a1200-workbench` compares against a golden produced by the
  defective capture (ASM-WS5-008); `sdk-a600-addons` counts colours only, so GadTools
  defects pass (ASM-WS5-012); `run-fault-amiga` checks message and status but not OS
  cleanup (ASM-WS5-001). These go to WS8 (test integrity).
- **Downgrades applied**: `card.resource` from Critical (explorers) to Major (no crash);
  `StoreNV` from Major to Minor (the autodoc prescribes the rounding); trackdisk from
  Critical to Minor (lens rule); clipboard hang kept at Major (not reproduced); the
  interrupt Z flag raised from `probable` to `verified` by the measured assembly.
- **Refuted candidates**: `UtilityBase` leak (probe 510 leak-free under `vamos`); probe
  504's leak (caller-side, spec/appendix-stdlib.md §13.2).

## Phase 5 gate (5.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | 22 findings, every template field, IDs `ASM-WS5-001..022` |
| ST-2 | verified findings cite committed probes (501, 503, 540–542, 530) with commands, or measurement records (golden histogram, `541.s`) |
| ST-3 | `hypothesis` items appear only inside the Minor findings ASM-WS5-019 and ASM-WS5-021 |
| ST-4 | no Major performance finding in WS5 (ASM-WS5-022 is an Opportunity) |
| ST-5 | WS5 probes committed in `0027bc9d`, 507 in `81bff4eb`, 540–542 and the walker in `73348b3f`, the runner fix and filler revisions before each rerun; falsifiability log complete |
| ST-6 | no spec file changed; two spec questions filed |
| ST-7 | drift check PASS |
| ST-8 | WS5-01..WS5-14 each have one outcome; every coverage-table row complete |
| ST-11 | ASM-WS5-001..006 escalated with timestamp and message |
| ST-12 | measured quantities cite `records/WS5-runs.log` and the commands in the findings |
| ST-14 | patch 530: output identical, committed as a diff, gain against the unpatched A500 -O2 build |
| ST-15 | no 68020 number in WS5 |
| ST-16 | dispatch records `xhigh` and `medium` |

## Escalations

- **2026-11-25 22:50** — "ASM-WS5-001 (Critical, verified): a runtime fault exits with
  OS registrations installed; on A1200/KS3.1 the VBlank server chain keeps a server whose
  code and node are in the unloaded program (vertb 4 → 5; no-fault contrast stays 4). Any
  later program loaded there turns it into a machine crash. Reproduce:
  `make -C docker exec CMD="bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh build build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/fsuae"`
  then `bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh run build/tmp/assessment/WS5/fsuae 541 541c`."
- **2026-11-25 22:50** — "ASM-WS5-002 (Critical, verified): `savePublicScreen` (and
  `showAlert`) before any screen or window calls Intuition through a NULL base; on
  A1200/KS3.1 the machine crashes (no END; with a screen opened first the call returns
  false). Reproduce: the build command above, then `… run build/tmp/assessment/WS5/fsuae 542 542c`."
- **2026-11-25 22:50** — "ASM-WS5-003 (Critical, verified): `setCurrentDir` unlocks the
  launcher's current-directory lock — a double release of a filesystem lock. Reproduce:
  `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS5 --clc build/tmp/assessment/baseline/clc --vamos-leaks --only 501"`."
- **2026-11-25 22:50** — "ASM-WS5-004, -005, -006 (Critical, probable): unclipped
  `drawText` on a screen writes outside the bitmap; a fault in a worker task runs DOS
  and `exit()` from a plain task; a freed clip region or a closed AmigaGuide context can
  be used after free from safe code. Code reading; locations in the findings."
- **2026-11-26 02:43** — Phase 9 verdict ASM-WS5-016 upgraded Major→Critical (verifier ses_dea9a6107ffefWXd6gFWNJQszZ): "On A1200/KS3.1 a `clipboardWrite` after `clipboardRead` on the same handle never returns — the machine's clipboard user hangs. Reproduce: `probes/WS5/verify-9.2.4/clipm.clo` with `probes/WS5/verify-9.2.4/run-a1200.sh` (Workbench 3.1 clipboard.device staged by `stage-wb.sh`)."
- **2026-11-26 02:43** — Phase 9 verdict ASM-WS5-003 downgraded Critical→Minor (verifier ses_dea9a6107ffefWXd6gFWNJQszZ); the 2026-11-25 22:50 escalation is retracted: "the double release is refuted — libnix and the CLI balance the current-directory locks; what remains is that the launching shell's directory changes."
- **2026-11-26 02:43** — Phase 9: ASM-WS5-004 and ASM-WS5-005 confirmed Critical and now `verified` (hangs on A1200/KS3.1); ASM-WS5-001's crash reproduced on A1200 and A500.


## Probe index

Runner: `probes/WS1/probe.py --vamos-leaks` (vamos); `probes/WS5/54x-fs-uae.sh` (FS-UAE).

| Item | Probes |
|------|--------|
| detector | 500 (control), `500-seeded-leak.c`; `54x-walker.c` + `54x-seeded.c` |
| WS5-02/03/08 DOS | 501, 502, 503 |
| WS5-06 Exec ports/messages | 504 (design error), 507 |
| WS5-02/06 timer, libraries | 505, 506 |
| WS5b 2.0 services | 510 |
| WS5-02/04/05/08 FS-UAE A1200/KS3.1 | 540, 541 (+ `54x-filler.c`), 542 |
| WS5-11 host shim | the WS2 probes under ASan/UBSan (`records/WS2-results.json`) |
| WS5-12 freestanding | `520-freestanding-link.sh` |
| WS5-13 hot path | `probes/WS4/530-format-u32-16bit.diff` |
