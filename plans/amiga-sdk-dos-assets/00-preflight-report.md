# Preflight Report: Amiga SDK Bundle 5 — DOS and Assets

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit Target**: all nine plan documents in `plans/amiga-sdk-dos-assets/`
> **Context**: `docs/amiga-sdk/registry.tsv`, `bundles.tsv`, `README.md`,
> `AGENTS.md`, `Makefile`, `stdlib/`, `compiler/src/{abi,stdlib,profile}.rs`,
> `compiler/runtime/shim_hosted_amiga.c`, `compiler/tests/`,
> `docker/scripts/a500-*-smoke.sh`
> **CodeOps Artifact Schema**: 1
> **Verdict**: ✅ PASSED — all critical/major findings resolved

⚠️ Same-session review: the plan was authored in this session. This report
records the independent subagent audit and the reconciling revisions.

## Codebase context

The audit verified the plan's material claims against the repository and the
pinned SDK image:

| Claim | Evidence |
|---|---|
| `std.iff` can be pure and host-runnable | `compiler/src/profile.rs:82` accepts non-`std.amiga.*`; host shim provides `std.mem` |
| `Lock` maps to `BPTR` | `compiler/src/abi.rs:16` |
| `File` is libc-backed | `compiler/runtime/shim_hosted_amiga.c:563-589` |
| DOS headers exist on 1.3 | `dos/dos.h` has `FileInfoBlock`/`InfoData`; `dos/filehandler.h` has `DeviceNode`/`dn_Name`; the 1.3 FD has `Lock`, `Examine`, `Info`, `CreateDir`, `CurrentDir`, `ParentDir`, `DeleteFile`, `Rename` |
| Display limits | `shim_hosted_amiga.c:993-1000` (bitmap 1024/8) and `1655-1660` (display 320x256/5) |
| Host harness | `compiler/tests/codegen_run_spec_test.rs`; cross-build pattern in `amiga_text_spec_test.rs` |
| Registry shape | Bundle 5 has 7 rows: 5 must/should core + 2 advanced |

## Findings and resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-01 | Critical | `ilbmDecode` ORs into a `Buffer` that is not zeroed; the host test used a zeroed literal, so the A500 could decode into garbage. | `ilbmDecode` clears `pixels[0..width*height]` before decoding; every decode test starts from `0xFF` and asserts that every pixel is overwritten. |
| PF-02 | Major | `volumeInfo` used the lock after `UnLock` for the name fallback and dereferenced a zero `id_VolumeNode`. | Check `Info` and `id_VolumeNode`; run the `Examine` fallback before `UnLock`; only read `dn_Name` from a valid node. |
| PF-03 | Major | Name copies were unbounded. | `pathInfoName`, `volumeName`, `currentDirName`, and `parentDirName` copy `min(name_len, buf.len)` and return the copied count; add short-buffer cases. |
| PF-04 | Major | `fileLength == -1` reached `usize` and a trapping allocator. | Loader rejects `length <= 0` and `length > 128 KB`; loader dimensions are capped at 320x256; allocation limits are documented. |
| PF-05 | Major | FORM/chunk window checks used additions that can wrap. | Use subtraction checks: `length <= data.len - 8`, `size <= end - offset - 8`; add a wrapped-length malformed case. |
| PF-06 | Major | The A500 hash was self-captured, so a deterministic decode bug could become the golden. | Add `ilbmPixelHash` (FNV-1a over chunky pixels); the host test pins the fixture hash from an independent decoder; the A500 line prints the same `hash` value. |
| PF-07 | Minor | `ilbmDecode` did not require `info.ok` or bound the cursor by `body_start + body_len`. | Require `info.ok`, validate geometry/compression, compute `body_end` by subtraction. |
| PF-08 | Minor | Zero-length BODY and multi-BODY files were accepted. | Reject both; add ST-5 cases. |
| PF-09 | Minor | Palette copied a partial triplet. | Copy whole `min(colors, palette.len/3)*3` bytes. |
| PF-10 | Minor | Pseudo-code used an integer as a condition and `u8 << 8`. | Use `bit != 0` and `u16(data[i]) << 8`. |
| PF-11 | Minor | ST-6 cited a nonexistent R15. | Added R13 (profile availability) and cited it. |
| PF-12 | Minor | SDK-DOS-003 says path/assign/volume; assign enumeration was unstated. | Record that DOS resolves assigns through `Lock`/`Open`; 1.3 assign-list enumeration is out of scope. |
| PF-13 | Minor | Lock/current-dir paths were compile-only. | The demo adds lock (64) and cwd (128) bits; `dos` is now an 8-bit mask. |
| PF-14 | Minor | Parser implementation cases were placed in the spec test. | Moved to `amiga_assets_impl_test.rs`. |
| PF-15 | Minor | Fixture reproducibility was not executed. | IT-4 runs the generator into a scratch directory and compares bytes. |
| PF-16 | Minor | Closeout did not name registry values or `bundles.tsv`. | Closeout lists all five done rows, both deferred rows, and the bundle plan/status. |
| PF-17 | Minor | Execution plan schema drifted. | Rewritten in the Bundle 4 shape with `Last Updated`, phases, dependencies, success criteria, and per-phase Verify. |
| PF-18 | Minor | `05-findings.md` was missing from the index. | Added to the Documents table. |
| PF-19 | Minor | The line/crop width was unstated and tight. | Shortened the line to `img: ok 320x200x4 hash … dos 255 vol 1`; crop is 620x22, calibrated on capture. |
| PF-20 | Minor | No chip-RAM budget. | Added a budget table; file <= 128 KB, pixels <= 320x256, front/back 32 KB each. |
| PF-21 | Minor | Caption pens were named by color, not index. | Fixture defines pen 0 dark blue and pen 1 white; the caption uses those indices. |
| PF-22 | Minor | ST-15 had no command. | The smoke compares two consecutive held captures at zero diff, then the golden. |
| PF-23 | Minor | Export-table updates were implicit. | Name `std.amiga.dos`, `std.amiga.video`, and `std.amiga.ilbm` in the MODULES table update. |
| PF-24 | Minor | `pathInfo` pseudo-code omitted the `Examine` check. | Added the failure path. |
| PF-25 | Minor | `03-01` claimed R11. | Now covers R6–R8 only. |
| PF-26 | Minor | Specialist packet did not name the ASan probe. | The parser host suite is `RDC_ASAN`-aware; `make asan` runs it. |
| PF-27 | Minor | Commit/push was baked into a task. | The task follows the active exec-plan commit mode; `analyze-agents` is rerun at closeout. |
| PF-28 | Minor | `make sdk-issues` was missing. | Closeout regenerates the issue body before closing #6. |
| O-01 | Observation | nix13 stdio is first exercised here. | Accepted: the A500 self-test is the gate; the loader has a 128 KB file cap. |
| O-02 | Observation | 1.3 `DeleteFile`-on-directory, `Lock("")`, and `ParentDir` semantics are target-only. | Accepted: the A500 self-test is the oracle. |
| O-03 | Observation | The poll budget may be tight. | The smoke keeps 15x5 s polls but starts the hold at 1200 ticks; if the run needs more, the poll count grows. |
| O-04 | Observation | A post-line volume bug would be invisible. | The demo reads volume info before printing; the report is last. |

## Verdict

All critical and major findings are resolved in the revised plan. The remaining
Minor and Observation items are either fixed in the same revision or explicitly
accepted above.

**Pass tier:** ✅ PASSED WITH NOTES — 0 critical/major open; accepted residuals:
nix13 stdio first use, target-only 1.3 DOS semantics, empirical poll budget,
and the documented ILBM subset.
