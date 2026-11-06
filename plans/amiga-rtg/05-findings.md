# Findings Ledger: Amiga RTG

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Scope**: independent correctness, memory-safety, and emission-integrity reviews of the Picasso96 RTG feature
> **Date**: 2026-11-06

## Correctness Review

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| CRIT-001 | MAJOR | `cl_rtg_read_pixels` did not cap the requested width/height or the actual P96 screen size; a screen wider than 10,922 could make the `WORD` row stride wrap. | `cl_rtg_read_pixels` now rejects width/height > 4096 and target width/height > 4096 before the `WORD` cast; the impl pin covers `target_width > 4096` and `target_height > 4096`. Re-ran the RTG gate and spec/impl suites. |
| CRIT-002 | MAJOR | `p96ReadPixel` returns 0 inside the screen's top-left clip even when a blit is present; the public read-pixel contract and the gate did not cover that region. | The shim reads one pixel through a 1x1 `p96ReadPixelArray` and converts RGB back to ARGB (`cl_rtg_read_pixel_rp`). The demo now blits at both (10,10) and (210,210) and checks both through the fixed read path. |
| CRIT-003 | MINOR | The API doc said the title could be null, but the public `cstring` parameter cannot be null. | The title is copied into the handle for the screen lifetime; the API says use an empty string for none. The copy also closes the retained-pointer hole found by the memory review. |
| CRIT-004 | MINOR | The demo printed `rtg: err 16` but returned 0. | The error branch now frees the handles and returns 1. |
| CRIT-005 | MINOR | Gate/demo docs still described the earlier (0,0)/red/fast-memory behavior. | Docs now describe the (200,200) capture, (210,210) and (10,10) blits, top-left write/fill at (20,20)-(30,30), blue fill at (300,300), 0 fast memory, and retained P96 extraction. |
| CRIT-006 | MINOR | The readback pin was satisfied by the blit guard, the drop pin only checked presence, and `P96SA_Title` was not pinned. | Added readback-specific `target_width/height` pins, cleanup nulling checks, and `P96SA_Title`/`P96SA_DClip`/`P96SA_ShowTitle` pins. |
| CRIT-007 | MINOR | Roadmap/programme status counts were stale while the registry and plan had advanced. | Updated at closeout: Bundle 9 20/20, Bundle 10 1/1, PLAN-17 19/19. |
| CRIT-008 | MINOR | The appendix-amiga sentence attributed all optional services to the licensed Workbench media. | It now names the user-provided Aminet P96 archive for Picasso96. |
| CRIT-009 | OBSERVATION | The specialist-check state did not include the new plan folders. | `codeops/specialist-check.json` now lists all 65 plan folders; `AGENTS.md` records the new check date/count. |

### Second Correctness Pass

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| CRIT2-001 | MAJOR | Screen rectangle fills still used the screen RastPort, so a top-left fill could return true while the layer clip rejected it; the gate only filled at (300,300). | Screen fills now use a bare RastPort over the screen bitmap, like the blit path. The demo fills at (20,20)-(30,30), reads (25,25), and the gate passes. |
| CRIT2-002 | MINOR | Status copies were mid-flight while the plan was still executing. | Fixed at closeout: plan 19/19, roadmap PLAN-17 Done, Bundle 9 20/20, Bundle 10 1/1. |
| CRIT2-003 | MINOR | DClip used the requested size rather than the actual P96 mode, so a non-exact mode could leave a visible-clip mismatch; actual screens >4096 were not rejected at open. | `cl_rtg_open_screen` queries the actual mode dimensions, rejects >4096, and builds DClip from them; blit/read/fill validate against the actual screen. |
| CRIT2-004 | MINOR | Runtime docs still named the old P96 symbols and the screen RastPort blit. | Updated to `CL_P96_*`, the bare screen-bitmap RastPort, actual-mode DClip, and the target-size readback cap. |
| CRIT2-005 | OBSERVATION | The impl suite pinned `p96ReadPixel` although the shim now reads through `p96ReadPixelArray`. | Removed the stale pin; `p96ReadPixelArray` and `cl_rtg_read_pixel_rp` remain pinned. |
| CRIT2-006 | OBSERVATION | A failed `p96CloseScreen` would free the title/handle while the live screen might reference them. | `cl_rtg_close_screen` now leaks the record on a failed close instead of freeing live memory; success frees exactly once. |

## Memory-Safety Review

Host AddressSanitizer is not applicable to the m68k-only shim. The reviewer used
a host model of the shim under ASan/LSan, emitted-C ownership probes, the real
A1200/UAEGFX gate, and the strict m68k/vbcc builds.

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| MS-RTG-001 | MINOR | `P96SA_Title` retained the caller's title pointer; the API did not require it to outlive the screen. | `cl_rtg_open_screen` copies the title into the handle, passes the copy, frees it after `p96CloseScreen`, and the API/appendix document the copy. |
| — | — | Handle alloc/free pairing, failure cleanup, close/free exactly once, DClip lifetime, readback bounds/stride, overflow, and bitmap-fill cap were clean. | Recorded in the review; no code change needed. |

## Emission-Integrity Review

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| EI-RTG-001 | MINOR | New file-scope P96 spellings (`RGBFB_*`, `RGBFF_*`, `BMF_USERPRIVATE`, `P96BMA_*`, `P96IDA_*`, `P96BIDTAG_*`, `P96SA_*`, `P96RD_*`) could collide with legal user enum macros. | Every internal header spelling is now `CL_P96_*`/`CL_P96_BMA_*`/etc.; `struct RenderInfo` is `CL_P96_RenderInfo`. The public module has no such names. A spec test defines `enum RGBFB` and `enum P96BMA` and cross-builds. |
| EI-RTG-002 | MINOR | The vbcc `#pragma libcall` block was inert; vbcc linked through `amiga.lib` P96 stubs, while docs claimed the pragma was the mechanism. | Removed the inert pragmas. GCC uses fixed-argument `LP*` macros; vbcc uses plain prototypes and the SDK `amiga.lib` P96 stubs (build-only; the runtime gate is GCC). Requirements/plan/notices/corpus comments updated. |

### Final Correctness Notes

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| CRIT3-001 | MINOR | Fill docs still described only the (300,300) blue fill after the top-left red fill was added. | Demo header, R6, gate step 3, and this ledger now describe both the (20,20)-(30,30) top-left fill and the (300,300) blue fill. |
| CRIT3-002 | MINOR | Status files were still mid-flight at 15/19. | Final state is plan 19/19 Done, PLAN-17 Done, Bundle 9 20/20, Bundle 10 1/1. |
| CRIT3-003 | OBSERVATION | `03-02-runtime.md` still used some unqualified P96 spellings and omitted the actual-target readback caps. | Updated to `CL_P96_*` names, bare-RastPort fill/blit, actual-mode DClip, and target caps. |
| CRIT3-004 | OBSERVATION | Top-left write/read was not runtime-checked. | The demo writes and reads (20,20) before the fill checks; a failure reports `err 5b`. |
| CRIT4-001 | OBSERVATION | A few legacy `RGB*` spellings remained in the runtime doc. | Replaced with `CL_P96_*` spellings. |

## Final Review Passes

| Review | Final verdict | Notes |
|---|---|---|
| Correctness | PASSED WITH NOTES | CRIT-001/002 and CRIT2-001/003/004/005/006 are fixed; CRIT3-001/002/004 and CRIT4-001 were documentation/status follow-ups and are fixed. Top-left write, fill, and blit are gate-checked. |
| Memory safety | PASSED WITH NOTES | MS-RTG-001 is fixed by copying the title; a failed close leaks rather than freeing live memory; the close-failure guard is now pinned. Host ASan is not applicable to the m68k shim; a host model under ASan/LSan, emitted-C ownership probes, and the real gate cover the shim's side. |
| Emission integrity | PASSED | EI-RTG-001/002 are fixed; 101 strict builds pass, 132 pre-existing units are byte-identical, the 40 hosted-amiga units differ only by the exact 24,574-byte RTG suffix, and the final gate/PPM matches. |

The final `docs/amiga-sdk/a1200-rtg.png` was regenerated from the final shim
with `make sdk-a1200-rtg`.

## Re-Review

The corrected diff was rechecked by the correctness, memory-safety, and
emission-integrity reviewers. The final evidence is:

- `make check`: 113 suites, 0 failures.
- `make asan`: 25 host suites, 0 failures.
- `make corpus-amiga`: portable programs plus RTG GCC and vbcc warning-free.
- `make sdk-a1200-rtg`: report and PPM checks pass; `docs/amiga-sdk/a1200-rtg.png`.
- `make verify`: all legs pass.

## Follow-up Review: Emulated Picasso IV and 24-bit

The 24-bit extension and the second gate were reviewed independently by the
correctness, memory-safety, and emission-integrity reviewers.

### Correctness

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RV-001 | MINOR | `compiler/src/stdlib.rs` export table omitted the two new 24-bit constants, so the appendix export check did not cover them. | Added `RTG_FORMAT_R8G8B8` and `RTG_FORMAT_B8G8R8`. |
| RV-002 | MINOR | The new spec case is compile-only and the impl pins named header constants that already existed, so a wrong 24-bit format mapping could pass `cargo test`. | Added exact pins for the mode-format ternary and both bitmap-format arms. |
| RV-003 | MINOR | The module header and demo header still described only the 32-bit UAEGFX gate. | Updated both to describe 24/32-bit and both gates. |
| RV-004 | MINOR | `SDK-RTG-002` did not state the A4000/Zorro III baseline requirement. | Added `A4000/Zorro III required` to the row notes. |
| RV-005 | MINOR | The plan marked 6.1.7 complete before review and commit; the four new files were untracked. | 6.1.7 is closed only after the review, commit, and final gate re-run; all new files are staged. |
| RV-006 | MINOR | The plan attributed the Z2 failure to an invalid `picasso-iv-z2` spelling that FS-UAE 3.1.66 silently ignores. | Retested with `picasso-iv` (auto/Z2) on A1200: the 24-bit demo passes. The gate keeps A4000/Z3 as the native path; the docs now describe both. |

### Memory safety

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| MS-001 | MINOR | The gate cleanup removed the decrypted Kickstart but left the staged Picasso IV flash ROM in `build/`. | `cleanup` now removes `${work}/Kickstarts` (the only staged copy). |

The independent host model ran 24- and 32-bit screen/bitmap scenarios under
ASan/LSan/UBSan, including exact-size and short-buffer readback, allocation
failures, close failures, and the demo fallback; no critical/major finding.
The m68k shim is still not host-ASan-instrumented; the runtime gate is the
24-bit proof.

### Emission integrity

No finding attributable to the change. Base/head emission across 155 sources
and four profiles showed no diff outside `hosted-amiga`; all 51 hosted-amiga
units share the same intended shim hunk, the header still precedes the shim,
and 124 emitted units plus the demo built with strict host/m68k/vbcc flags.
`EI-PRE-001` (a user module named `std.amiga` can collide with a stdlib C
symbol) reproduces on both refs and is pre-existing; it is not fixed here.

### Follow-up verification

- `make check`: 113 suites, 0 failures.
- `make asan`: 25 host suites, 0 failures.
- `make corpus-amiga`: RTG GCC and vbcc builds warning-free.
- `make sdk-a1200-rtg`: 32-bit report/PPM unchanged.
- `make sdk-a4000-rtg-piv`: 24-bit report/PPM green; staged ROM removed by
  cleanup.
- `make verify`: all legs pass.
