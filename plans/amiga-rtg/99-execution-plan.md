# Execution Plan: Amiga RTG — Picasso96 on UAEGFX

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Progress**: 26/26 tasks (100%) — Done
> **Created**: 2026-11-06 13:30
> **Last Updated**: 2026-11-06 22:35
> **CodeOps Artifact Schema**: 1

## How to Execute

Work in the order below: specification tests first, then implementation, then
implementation pins, the runtime gate, registry/docs, and the full acceptance
run. Every task updates this checklist immediately. A failing spec test means
the implementation is wrong, never the test. The runtime gate must run
headlessly on a private Xvfb display and must not commit P96 or ROM media.

## Phase 0: Requirements and plan gates

- [x] 0.1.1 Write the RTG requirements set, ambiguity register, current state, API/runtime/gate docs, and testing strategy; run the independent requirements preflight and resolve every critical/major finding ✅ (completed: 2026-11-06 14:00; 11 PF-REQ findings resolved, second-pass hash recorded)
- [x] 0.1.2 Write this execution plan; run the independent plan preflight; resolve every critical/major finding and record the verdicts ✅ (completed: 2026-11-06 14:00; 11 PF-PLAN findings resolved, verdict PASSED)

## Phase 1: Specification surface (red → green)

- [x] 1.1.1 Add `compiler/tests/amiga_rtg_spec_test.rs`: module lookup/exports, host/1.3/freestanding `E11200`, hosted-amiga emission, and drop/ownership shapes; run red ✅ (completed: 2026-11-06 14:40; 7 spec tests)
- [x] 1.1.2 Add `stdlib/std.amiga.picasso96.clo` and register it in both `SOURCES` and `MODULES` in `compiler/src/stdlib.rs`; run ST-1/2/3/7 green. `hosted-amiga` already accepts every stdlib module; no `Profile` change is needed for the gate, and the suffix belongs to 2.1.3. ✅ (completed: 2026-11-06 14:40)

## Phase 2: Runtime shim

- [x] 2.1.1 Add `compiler/runtime/picasso96.h` with the minimal P96 constants, structs, and fixed-argument call macros; record the CC-BY attribution ✅ (completed: 2026-11-06 14:40; GCC macros + vbcc pragmas + THIRD_PARTY_NOTICES)
- [x] 2.1.2 Add `compiler/runtime/shim_hosted_amiga_optional_rtg.c`: library/board discovery, 32-bit mode query, screen open/close, bitmap alloc/free, pixels, bounded bitmap fill, blit, and RGB readback ✅ (completed: 2026-11-06 14:40)
- [x] 2.1.3 Embed the header before the RTG shim in `HOSTED_AMIGA_SUFFIX` (both `include_str!` fragments), append it for `hosted-amiga` only, and run the strict m68k cross-build ST-4; prove the A500/1.3 unit is unchanged and the host/1.3/freestanding rejection still holds ✅ (completed: 2026-11-06 14:40; profile suffix + nix20/vbcc builds)
- [x] 2.1.4 Add `compiler/tests/amiga_rtg_impl_test.rs` pins for every P96 call, ownership/drop, bounds, the bitmap-loop fill, all accessors, and the missing-library guard; run the suite green ✅ (completed: 2026-11-06 14:40; 6 impl tests)

## Phase 3: Demo, capture, and gate

- [x] 3.1.1 Add `examples/amiga/optional-rtg.clo`: 640x480x32 screen, red/green/blue pixels, screen fill away from the capture, bitmap fill/write, blit at (210,210), every public accessor, bitmap readback, 64x64 screen RGB readback, `rtg.ppm`, and `rtg.txt` ✅ (completed: 2026-11-06 14:40; capture at 200,200)
- [x] 3.1.2 Add `docker/scripts/a1200-rtg-smoke.sh`: P96 SHA-256/extraction, WB3.1 staging, `uaegfx` monitor, ROM decrypt/check, private Xvfb, FS-UAE UAEGFX, report and PPM checks, PNG conversion, and cleanup ✅ (completed: 2026-11-06 14:40)
- [x] 3.1.3 Add `make sdk-a1200-rtg`, include it in `make verify`, and run the gate green ✅ (completed: 2026-11-06 14:40; `rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1`)
- [x] 3.1.4 Add the explicit RTG build-only leg to `docker/scripts/corpus-amiga.sh` for GCC and vbcc; run `make corpus-amiga` green ✅ (completed: 2026-11-06 14:40; both warning-free)

## Phase 4: Registry, docs, and evidence

- [x] 4.1.1 Add `10-rtg` to `bundles.tsv`; replace `SDK-OPT-004` with the full `SDK-RTG-001` row at status `done` (the gate is already green); run `make sdk-registry`; update coverage and replace the issue #11 body ✅ (completed: 2026-11-06 15:00; 80 capabilities, 11 bundles)
- [x] 4.1.2 Add appendix §14.23, the appendix-amiga module row, the RTG known-limitations section, the `AGENTS.md` command/layout entries, the programme index, the roadmap PLAN-17 row, and the `THIRD_PARTY_NOTICES.md` P96 attribution ✅ (completed: 2026-11-06 15:00)
- [x] 4.1.3 Add the durable spike record under `spikes/` for the P96/UAEGFX evidence and the bitmap-fill workaround ✅ (completed: 2026-11-06 15:00; `spikes/06-rtg/README.md`)

## Phase 5: Acceptance and review

- [x] 5.1.1 Run `make check` and `make asan`; fix every failure ✅ (completed: 2026-11-06 15:45; 113 suites, 25 ASan suites)
- [x] 5.1.2 Run `make sdk-a1200-rtg`, `make corpus-amiga`, and full `make verify`; capture the gate report and PPM evidence ✅ (completed: 2026-11-06 15:45; all legs passed)
- [x] 5.1.3 Run the independent correctness, memory-safety (static + host models + m68k gates), and emission-integrity reviews; fix every critical/major finding and re-run the affected gates ✅ (completed: 2026-11-06 15:45; findings fixed in `05-findings.md`)
- [x] 5.1.4 Clean temporary artifacts and archives, confirm the working tree and `docker/.env`, update the plan/roadmap to Done, commit locally, and close issue #11 ✅ (completed: 2026-11-06 15:45; commit `7ce2478`; issue #11 closed)

## Phase 6: Emulated Picasso IV follow-up

- [x] 6.1.1 Add the 24-bit follow-up requirements, the Picasso IV gate design,
  and update the API/runtime/testing docs for depth 24/32 and the second gate
  ✅ (completed: 2026-11-06 21:50; R14–R18, `04-picasso-iv-gate.md`)
- [x] 6.1.2 Extend the shim and module for 24-bit truecolor: allow depth 24/32,
  select `RGBFF_TRUECOLOR`/`B8G8R8` or `RGBFF_TRUEALPHA`/`A8R8G8B8`, add the
  24-bit format constants, and make the demo fall back from 32 to 24 bits
  ✅ (completed: 2026-11-06 21:40; A4000/Picasso IV `rtg: ok boards 1 mode
  640x480x24 ...`)
- [x] 6.1.3 Add the 24-bit spec cross-build and the 24-bit impl pins
  ✅ (completed: 2026-11-06 21:50; `amiga_rtg_spec_test`,
  `amiga_rtg_impl_test`)
- [x] 6.1.4 Refactor the gate into `docker/scripts/rtg-smoke.sh` with two thin
  card entry points; add `make sdk-a4000-rtg-piv` and include it in
  `make verify` ✅ (completed: 2026-11-06 21:44; both gates green)
- [x] 6.1.5 Add `SDK-RTG-002`, regenerate coverage, and update Bundle 10 to
  2/2 ✅ (completed: 2026-11-06 21:50; 81 capabilities, 11 bundles)
- [x] 6.1.6 Update the appendix (§14.23), known limitations, AGENTS command
  table, RTG index, and roadmap PLAN-17 ✅ (completed: 2026-11-06 21:50)
- [x] 6.1.7 Run `make check`, both RTG gates, `make asan`, `make corpus-amiga`,
  and the review pass; fix every critical/major finding and commit locally
  ✅ (completed: 2026-11-06 22:35; correctness/memory/emission reviews,
  113 check suites, 25 ASan suites, corpus GCC+vbcc, `make verify`;
  commit `7501fca`)

## Acceptance

- `make check`: all suites green, including `amiga_rtg_spec_test` and
  `amiga_rtg_impl_test`.
- `make asan`: host ASan suites green; the m68k P96 shim is static-reviewed and
  gate-proven, not host-ASan-instrumented.
- `make sdk-a1200-rtg`: `rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1
  blit 1` and a valid 64x64 PPM.
- `make sdk-a4000-rtg-piv`: `rtg: ok boards 1 mode 640x480x24 red 1 green 1
  blue 1 blit 1` and the same PPM checks on the emulated Picasso IV.
- `make corpus-amiga`: portable programs and the explicit RTG GCC/vbcc
  build-only leg.
- `make verify`: all legs pass.
- `SDK-RTG-001` and `SDK-RTG-002` done; Bundle 9 20/20; Bundle 10 2/2;
  issue #11 closed.
