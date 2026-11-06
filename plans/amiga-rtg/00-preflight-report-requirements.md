# Preflight Report: Amiga RTG Requirements

> **Audit target**: `plans/amiga-rtg/` requirements set (`00-index.md`, `00-ambiguity-register.md`, `01-requirements.md`, `02-current-state.md`, `03-01-api.md`, `03-02-runtime.md`, `03-03-gate-media.md`, `07-testing-strategy.md`)
> **Artifact hash**: `3c9547d091453b910613b05f88d7200b59dd38c97c30da75c83840d3e8e5a2dd` (second pass; concatenated per-file SHA-256 list)
> **Date**: 2026-11-06
> **Reviewer**: independent preflight pass (subagent) plus lead reconciliation
> **Verdict**: ✅ PASSED — all findings resolved

⚠️ **Same-session review:** the requirements were authored in this session and
then independently reviewed by a separate agent. The reviewer's findings below
were verified against the repository and the P96 media before resolution.

## Codebase Context

| Area | Evidence checked |
|---|---|
| Profile gate | `compiler/src/profile.rs:91-124` (`supports_module`), `compiler/src/driver.rs:258-272` (`E11200`) |
| Shim placement | `compiler/src/profile.rs:18-24`, `compiler/src/codegen.rs:626-645`; only `hosted-amiga` appends the optional suffix |
| Existing optional modules | `stdlib/std.amiga.amigaguide.clo`, `stdlib/std.amiga.bsdsocket.clo`, `compiler/runtime/shim_hosted_amiga_optional*.c` |
| Handles/drops | `export handle … freedBy …` in the existing optional modules |
| Media/gate patterns | `docker/scripts/workbench-media.sh`, `docker/scripts/a1200-addons-smoke.sh`, `docker/scripts/a300-*` |
| P96 API | `build/rtg/p96dev/Picasso96Develop/Include/libraries/Picasso96.h`, `inline/Picasso96.h`, `autodocs/Picasso96API.doc`, `fd/Picasso96API_lib.fd` |
| P96 runtime | `build/rtg/p96/Picasso96Install/`; `Picasso96API.library` 2.300; archive SHA-256 `0146d4a2…` |
| FS-UAE/P96 spike | `build/rtg/probe/rtgprobe.c` + `build/rtg/probe/spike.sh`; report in `build/rtg/loop-evidence.txt` |
| Registry | `docs/amiga-sdk/registry.tsv:81`, `bundles.tsv`, `tools/amiga-sdk/registry.sh` |
| Attribution | `THIRD_PARTY_NOTICES.md`, `P96Develop.readme` (CC-BY 4.0) |

## Findings and Resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-REQ-001 | MAJOR | The PPM was read from (0,0) while the bitmap blit went to (200,200), so the capture could not contain the blit pixels; the checked (10,10) point was unresolved. | The demo now writes at (200,200) and blits to (210,210) inside the 64x64 capture; the gate checks relative red (0,0), green (1,0), blue (2,0), yellow (10,10), blue (11,11), and blue (20,20). `03-03` and `07` are explicit. |
| PF-REQ-002 | MAJOR | The public format constants used wrong P96 values for 15/16-bit formats. | The surface is now 32-bit only. Constants are the four 32-bit values (6, 7, 8, 9), and the gate accepts the reported screen/bitmap byte order. |
| PF-REQ-003 | MAJOR | The bitmap-fill workaround was justified by a pixel loop absent from the retained spike. | The loop probe was re-run and retained at `build/rtg/loop-evidence.txt`; it reports bitmap `(1,1)=ff`, `(20,20)=ff`, blit `(201,201)=ff`. The negative control at `build/rtg/rect-fill-evidence.txt` shows `p96RectFill` returning red for the same blue fill. IT-8 pins the loop. |
| PF-REQ-004 | MAJOR | The requirements promised 15/16/24/32-bit modes but the gate proved only 32-bit. | The API and gate are narrowed to 32-bit truecolor. `depth != 32` returns 0/null; every public accessor is exercised by the demo or the impl suite. |
| PF-REQ-005 | MAJOR | vbcc parity was conditional and `corpus-amiga` did not explicitly build the RTG unit. | The vendored header omits P96's varargs macros (which vbcc rejects); GCC uses fixed-arg `LP*` calls and vbcc links the SDK `amiga.lib` P96 stubs. R11 and ST-5 require a warning-free build with both toolchains; the corpus script gets an explicit RTG build-only leg. |
| PF-REQ-006 | MINOR | CC-BY attribution was asserted but not specified. | R10 now requires a `THIRD_PARTY_NOTICES.md` entry with authors, archive/version, CC-BY 4.0 source, and the vendored-header modification. |
| PF-REQ-007 | MINOR | The registry replacement did not specify the full row or the issue #11 transition. | R9 gives all 14 columns for `SDK-RTG-001` in `10-rtg`; the plan requires replacing issue #11's placeholder body and superseding the old Bundle 9 RTG text. |
| PF-REQ-008 | MINOR | Several names/counts were stale (`Profile::provides`, nine functions, close/free names, Implements range). | Corrected to `Profile::supports_module`, twelve fixed-arg P96 calls, explicit `closeRtgScreen`/`freeRtgBitmap`, and `Implements: R1–R13`. |
| PF-REQ-009 | MINOR | The readback contract used the wrong `RenderInfo` width bound and implied a P96 lock requirement. | Width is capped at 4096 (stride 12,288, inside signed `WORD`); the output stride is the shim's `width * 3`. `BMF_USERPRIVATE` is documented as lock-free for this path. |
| PF-REQ-010 | MINOR | No acceptance/review section. | Added AC-1–AC-6, including `make check`, `make asan`, `make corpus-amiga`, `make verify`, the three independent reviews, and gate re-runs. |
| PF-REQ-011 | OBSERVATION | The gate used an unproven 8 MB fast-memory setting and only said the P96 hash was "recorded". | The gate uses the proven `--fast-memory=0`; the P96 SHA-256 and ROM SHA-1 are verified. |

## Second Pass

The plan preflight found four requirement-level contradictions that were
introduced or left stale during the first pass:

- PF-PLAN-003: the fixed-arg `LP*` macros were not vbcc-safe; the vendored
  header now uses plain prototypes and the SDK `amiga.lib` P96 stubs for vbcc.
- PF-PLAN-004: `03-03` still described the old capture; it now writes at (200,200) and blits at (210,210).
- PF-PLAN-005: accessor and missing-library coverage was incomplete; the demo
  now exercises every public accessor and bitmap readback, and the impl suite
  pins the library guard.
- PF-PLAN-009: the `p96RectFill` mismatch needed a retained negative control;
  `build/rtg/rect-fill-evidence.txt` now records it.

These are resolved in the current artifact hash. The requirements preflight is
therefore re-run and passing as of the second-pass hash above.

## Pass Tier

**✅ PASSED** — every critical/major finding is resolved; the two remaining
minor findings (attribution and registry transition) are resolved by the
requirements; no unresolved notes remain. The plan preflight consumes this
report and may add plan-specific findings.

## Residual Risks

| Risk | Mitigation |
|---|---|
| P96 is third-party and its API has historical quirks (bitmap fill color, monitor icon tooltypes). | The subset is gate-proven; bitmap fill uses the pixel loop; the monitor is named `uaegfx` so the driver guesses its board type. |
| The P96 archive is not committed. | The gate checks the exact SHA-256 and exits with a clear media instruction. |
| vbcc may reject another P96 header construct later. | The corpus RTG build-only leg is explicit and fails the build, not silently skips. |
