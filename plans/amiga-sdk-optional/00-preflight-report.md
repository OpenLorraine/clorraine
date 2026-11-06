# Preflight Report: Amiga SDK Bundle 9

> **Audit target**: `plans/amiga-sdk-optional/` (all documents)
> **Context**: `AGENTS.md`, `spec/`, existing stdlib/shim/tests, Amiga Forever media, FS-UAE 3.1.66, NDK headers
> **Date**: 2026-11-06 02:09
> **Same-session review**: ⚠️ the plan was authored in this session; the scan was grounded in code, NDK headers, and the actual media. A fresh-session review is recommended before the final closeout.

## Codebase Context Summary

The plan extends the existing stdlib/shim/gate pattern. It adds a new optional C suffix, 19 thin
modules, and five headless gates. The key code facts verified: profile gating is module-level
(`compiler/src/profile.rs:85-118`); the shared shim is 1.3-safe and the 2.0 suffix is appended after
the generated program (`codegen.rs:497-513`); handles use `freedBy`/`take`/`borrow`; the existing
gate scripts use A500 ADFs and A600/A1200 directory HDs; `xdftool` and the NDK headers are in the
SDK image; the licensed media contains the required Workbench files.

## Findings and Resolutions

All critical/major findings were resolved in the plan; minor items were fixed or explicitly
accepted. Iteration 2 re-checked the corrected documents against the code and media.

| PF | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🔴 | `u64` is not a v1 type; EClock could not compile. | Dedicated `EClock` handle, `EClockTicks { hi, lo }`, frequency. |
| PF-002 | 🔴 | `CreateGadgets` is not a gadtools function; event path used classic `GetMsg`. | `CreateContext` + `CreateGadgetA`; `GT_GetIMsg`/`GT_ReplyIMsg`/refresh path. |
| PF-003 | 🔴 | `gradientslider` tags were wrong; `colorwheel` lacked `WHEEL_Screen`. | BOOPSI scoped to a native Intuition class; runtime probes later selected `strgclass` because `colorwheel` color readback did not round-trip. |
| PF-004 | 🟠 | `GT_GetGadgetAttrsA` is V39, not 2.0. | Removed gadget readback; capture + count. |
| PF-005 | 🟠 | ASL path/save-mode/input handshake were underspecified. | Drawer/file split, V38 save mode, `asl-ready`/`asl-done`, retries. |
| PF-006 | 🟠 | Hooks exposed unsafe `raw` data and public `h_SubEntry` hack. | `func(i32): i32`, scalar `h_Data`, documented trampoline. |
| PF-007 | 🔴 | `UtilityBlock` was undefined; pool could free live blocks. | Separate `std.amiga.pool`, declared handles, refcounted pool. |
| PF-008 | 🟠 | `DateStamp` static snapshot was not reentrant. | `DateStamp` handle with one snapshot and getters. |
| PF-009 | 🔴 | ReadArgs had no length parameter; slots could overflow; `/N`/`/S` misread as strings. | Template parsing, count match, per-slot kinds, `readArgsFrom`, bounded copy. |
| PF-010 | 🔴 | `DTA_Depth`/`DTA_NumColors` do not exist; draw shape was wrong. | `PDTA_BitMapHeader`/`PDTA_NumColors`, `dtDraw`, 3.1 gate. |
| PF-011 | 🔴 | Region ownership contradicted itself; `AndRectRegion` is void. | Layer borrows region, clear-before-free, void AND, `bounds`. |
| PF-012 | 🟠 | EXE-007 pools are V39; AUD-006/WBK-010 are 3.1; DOS-006 is 3.1. | Baselines corrected; 3.0/3.1 gates added. |
| PF-013 | 🔴 | `10.0.2.2` is not the FS-UAE bsdsocket host alias. | `127.0.0.1` + `--network host` + host echo server. |
| PF-014 | 🔴 | Media helper copied `S/`, used lowercase paths, omitted `Classes`/`AddDataTypes`. | Stage, named-tree copy, exact case, `LIBS ... ADD Classes`, `AddDataTypes`. |
| PF-015 | 🔴 | A500 diskfont gate lacked `C/Assign`, writable ADF, and safe read timing. | `DF1:C/Assign`, `--writable-floppy-images=1`, stop before read, size 12. |
| PF-016 | 🟠 | Gate handshakes/lines were inconsistent (`asl-ready`, `services.txt`, booleans). | One handshake table; full lines with measured values. |
| PF-017 | 🔴 | ARexx message ownership was undefined. | Created/received/reclaim ownership; runtime probes later showed `ClearRexxMsg` after the reply crashes, so `DeleteRexxMsg` alone releases the packet. |
| PF-018 | 🟠 | Clipboard write omitted `CMD_UPDATE`; `CLIPS:` was unnecessary. | Fresh clip ID (`io_ClipID=0`), write, update, read ID; no `CLIPS:`. |
| PF-019 | 🔴 | `IOPrtCmdReq` has no write fields; printer guard was over-broad. | `union printerIO` + `PRD_RAWWRITE`; guard only clipboard. |
| PF-020 | 🟠 | Serial/parallel parameters were inert; parallel is output-only. | SETPARAMS; serial loopback; parallel host sink. |
| PF-021 | 🔴 | `StoreNV` length is in tens of bytes. | Round up; 23-byte round-trip. |
| PF-022 | 🟠 | Plan edited frozen `spec/15-profiles.md`. | Chapter 15 untouched; appendices only. |
| PF-023 | 🟠 | New m68k shim cannot use host ASan. | Static review + host models + m68k runtime gates; no false ASan claim. |
| PF-024 | 🟡 | Coverage counts RTG, so 20/20 was wrong. | 20 in-scope done / 21 total. |
| PF-025 | 🟠 | Appendix stale “deferred/not part” statements were not mapped. | Per-module section anchors + superseded text checks. |
| PF-026 | 🟡 | 18 deferred + 2 missing vs 17 outside arithmetic. | Corrected to 18 outside + 2 already in `9-optional`. |
| PF-027 | 🟡 | Gate/report/pin names diverged. | Row-to-evidence map and exact lines. |
| PF-028 | 🟠 | Existing 1.3 forbidden-call test scans the whole emit and will fail when the optional suffix appears. | Phase 0 scopes it to the shared shim. |
| PF-029 | 🟡 | Five gates vs four scripts; intuition mode had no path. | Intuition mode in `a600-addons`; five targets specified. |
| PF-030 | 🟡 | Example imports omitted `Semaphore`/`Args`; unused `String`. | Imports fixed. |
| PF-031 | 🟡 | `regionBounds` value-struct return was unspecified. | Scalar accessors fill a value struct. |
| PF-032 | 🟡 | `clipboardRead` larger-than-buffer contract was missing. | Reads at most buffer length; returns bytes. |

## Verdict

**✅ PREFLIGHT PASSED — all critical/major findings resolved; minor findings resolved or accepted.**

The plan is executable. The remaining caveats are named in the register: printer output is not
claimed, commodities is broker-lifecycle only, BOOPSI uses one native class, and the m68k shim is
not host-ASan-instrumented. These are explicit scope boundaries, not hidden gaps.
