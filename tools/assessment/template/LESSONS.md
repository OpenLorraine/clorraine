# Lessons from the first assessment run

Folded into the method for the next run. Only lessons that Phase 9 of the first run
confirmed (a verifier reproduced the problem or its fix) are listed.

## Probe design

- **Fixed arrays do not widen to slices.** Pass `slice<u8>(arr, 0, N)` wherever a `u8[]`
  parameter is expected (spec/08 §3); three probes of the first run were design errors
  for this reason.
- **GCC -O2 removes what a seed does not use.** A seeded quadratic loop was folded to a
  closed form and a never-written static array was dropped. Route seeded work through a
  `volatile` module value, or write to the seed's data from a run-time value (`argc`).
- **Inlining hides functions from disassembly checks.** A probe that reads one function's
  assembly must also print its callers (`main`); the first MMIO record was empty because
  the checked function was inlined.
- **Module-level fills unroll per element.** Do not use a large module-level `[; v]` fill
  in a probe or a seed (a 600 KB fill did not build in 30 minutes).
- **One shell layer in the SDK container.** `make -C docker exec CMD="..."` with a nested
  `bash -c '...'` runs in the wrong directory; run each command separately.

## Emulators

- **`vamos` does not model** 68000 address errors, BPTR alignment truncation, or ROM
  library open counts. Use FS-UAE for those, or record a `limit`.
- **A leftover public port cannot be found by name** after its program is unloaded (the
  name string is freed). Count the nodes of `SysBase->PortList` instead.
- **A dangling interrupt server crashes only when its memory is reused.** To show the
  crash, load a program whose code hunk is filled with `ILLEGAL` after the faulting one
  (the first run's verifier did this on both the A1200/KS3.1 and the A500/KS1.3).
- **A500/KS1.3 under FS-UAE** needs an ADF or a Workbench 1.3 HDF (no directory drive at
  boot); decrypt the licensed ROM under `build/tmp` and remove it on exit, as the
  `sdk-a500-*` gates do.
- **The clipboard device keeps only IFF clips**, and a read that does not run to the end
  blocks later writers; clipboard probes need the licensed Workbench `clipboard.device`.

## Measurement

- **Parallel verification inflates timing.** Compare scaling exponents, not absolute
  times, and run timing-sensitive verification alone.
- **Host wall time cannot rank the benchmarks** (about 1–1.6 ms each, below the spread).
- **The WS8 assertion classifier misses computed exit statuses**: a program that returns a
  value-derived status and is checked with `assert_exit(…, 0)` is a value check; refine
  `800-assertion-scheme.py` before reusing its census.
