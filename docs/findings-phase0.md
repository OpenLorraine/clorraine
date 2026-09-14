# Phase 0 findings - hosted walking skeleton

Status: complete. Feed the Clorraine Phase 1 specification and the SDK
follow-up list.

## Scope

Validate the hosted AmigaOS target model before any compiler exists:

* exec/dos ownership contracts (`returns-owns`, `consumes`, `borrows`)
* releases on every exit path, null-safe, exactly once
* a strict common C subset accepted by GCC and vbcc
* the run matrix: `vamos`, FS-UAE directory drive, FS-UAE bootable ADF

Skeleton: [`examples/hosted-skeleton/`](../examples/hosted-skeleton/).

## Results

| Leg | Build | Runner | Result |
|---|---|---|---|
| 1 | GCC 16.2, `-mcrt=nix20 -m68000`, `-Wall -Wextra -Werror` | `vamos` | all scenarios ok |
| 2 | GCC 16.2, `-m68020` | build only | compiles clean |
| 3 | vbcc `+aos68k -cpu=68000` | `vamos` | all scenarios ok |
| 4 | GCC 68000 | FS-UAE, directory drive | output matches `vamos` |
| 5 | GCC 68000 | FS-UAE, bootable ADF | output matches `vamos` |

Release proof: `acquisitions=12 releases=12` across 6 scenarios (happy,
alloc-fail, open-fail for each cleanup shape). Failure scenarios confirm
null-safe drops: a failed `Open` still releases the owned library, a failed
`AllocVec` still releases file and library.

## Toolchain findings

| Finding | Detail |
|---|---|
| Auto-open library is `-lamiga`, not `-ldos` | `-ldos` fails with "have you installed the static version of the dos library?". `libamiga.a` is the stub archive; archive semantics pull in only the libraries actually referenced, so no unwanted libraries are opened |
| vbcc needs the NDK include tree | `vc +aos68k` does not search it by default; add `-I/opt/amiga/m68k-amigaos/ndk-include`. Follow-up: bake this into the vbcc target config in the SDK image |
| vbcc warning 53 is benign | `devices/timer.h` (system header) triggers "struct/union member needs identifier"; not our code, accepted and documented |
| GCC NDK 3.2 headers are warning-clean | `-Wall -Wextra -Werror` passes unchanged |
| CPU baselines | `-m68000` and `-m68020` both build; `-m68000` stays the default for maximum compatibility |
| Runtime | libnix `-mcrt=nix20` works under `vamos` and FS-UAE/AROS with no extra setup |

## Cleanup shape comparison

| Property | Shape A: single exit | Shape B: scoped |
|---|---|---|
| Control flow | one `goto cleanup`; reverse-order drops | nested blocks; drops at scope ends |
| Early exits (`return`, `break`) | natural: jump to the shared label | must duplicate drops at every exit point |
| Code growth | bounded: one cleanup ladder per function | grows with nesting and exit count |
| Readability | flat but label-jumping | mirrors source scopes but nests deeply |
| Compiler complexity | low: one label, fixed drop order | higher: drop placement per exit edge |

**Decision:** the canonical emitted pattern is **Shape A extended with one label
per scope** (a single-exit ladder that falls through scope-by-scope). It keeps
early-exit correctness and bounded code size while honoring scope-accurate
drop order. Shape B remains valid for very short scopes. This will be
formalized in the spec's codegen contract.

## Contract validation (v0)

Implemented and exercised in `ffi_contracts.h`:

| Function | Contract | Validated |
|---|---|---|
| `OpenLibrary(name, ver)` | returns-owns(`CloseLibrary`), nullable | yes |
| `CloseLibrary(base)` | consumes, null-safe | yes |
| `AllocVec(size, flags)` | returns-owns(`FreeVec`), nullable | yes |
| `FreeVec(mem)` | consumes, null-safe | yes |
| `Open(name, mode)` | returns-owns(`Close`), nullable | yes |
| `Close(file)` | consumes, null-safe (0 invalid) | yes |
| `Write(file, buf, len)` | borrows(`buf`, len bytes), value result | yes |

Deferred to a later phase: `CreateMsgPort`/`CreateIORequest`/pools,
`GetMsg`/`PutMsg`/`ReplyMsg` ownership transfers, and any callback or
`stores`-style contracts.

## Runtime shim seed

`ffi_contracts.h` establishes the pattern the runtime will grow from:

* typed acquire wrappers (`cl_acquire_*`) return a nullable owned handle
* typed drop helpers (`cl_drop_*`) are null-safe and release exactly once
* release counters turn silent leaks into test failures - the property the
  borrow checker will eventually prove statically

## Runner findings

* Directory hard drive remains the default: rebuilds need no image step.
* The bootable ADF path requires three things that are now encoded in
  `docker/scripts/run.sh`: executable protection (`xdftool protect +e`),
  deleting any previous image (`xdftool create` refuses to overwrite), and
  `--writable-floppy-images=1` so writes land in the ADF instead of FS-UAE
  overlay files.
* `RUN_CAPTURE=1` output extracted from the ADF matches the `vamos` output,
  so the ADF path is CI-ready for later phases.

## Recommendations for Phase 1

1. Profile order: implement `hosted-amiga` first; keep `host` (native) as the
   fast test target; defer `freestanding` until LorraineOS boots.
2. Contract vocabulary: adopt `returns-owns(deleter)`, `consumes`,
   `borrows`/`borrows-mut`, `value` as the annotation set; extend with
   `stores` only when callbacks arrive.
3. Runtime interface: mirror the acquire/drop wrapper pattern; keep it
   libc-free where practical so the eventual freestanding profile can reuse it.
4. Codegen contract: null-safe drops, single-exit ladder with per-scope
   labels, stable names, `#line` before each emitted statement.
5. SDK follow-up list: wire the NDK include path into the vbcc target config;
   decide a warning policy for vbcc system headers; keep `make image` as the
   only toolchain entry point.

## Follow-ups

* Full SDK image rebuild from the renamed Dockerfile (pending; the current
  image is the fast derived equivalent).
* Add vbcc include-path wiring to the image so `-I` is not needed by users.
* Phase 1 spec skeleton, using these findings as input.
