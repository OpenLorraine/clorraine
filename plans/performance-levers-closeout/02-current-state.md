# Current State: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

`clc` emits one C translation unit containing the program, the embedded
standard-library support, and the profile's runtime shim. The driver compiles
that unit with a fixed command matrix:

- `host`: `cc -std=c99 -O2 -o <bin> <c>`
- `hosted-amiga`: `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o <bin> <c> -lamiga -lm`
- `hosted-amiga-13`: the same with `-mcrt=nix13`
- `freestanding`: `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o <bin> <c> -lg`

Debug builds use `-O0 -g -Wall -Wextra -Werror`; release builds use `-O2`
with no warnings. The exact lines are owned by
`compiler/src/clorraine/toolchain.rs:36-78,144-157` and mirrored in the
frozen compile matrix at `spec/17-codegen.md:384-397`.

Because the shim and the program are one object, the linker cannot discard an
unused shim function. A hello-world-sized program carries roughly 77 KB of
m68k `.text`, most of it unreachable runtime code. `-ffunction-sections
-fdata-sections` gives each function/datum its own section; `-Wl,--gc-sections`
then drops the unreachable ones; `-s` removes the release symbol table.

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `compiler/src/clorraine/toolchain.rs` | Exact build commands, `Tool`, `run_tool`, unit vectors | Add per-profile release flags, host platform predicate, freestanding host-run release, tests, docs |
| `compiler/src/clorraine/build.rs` | Calls `toolchain::command`; two test doubles construct `Tool` | Add the `release` field to both test doubles; no production behavior change |
| `compiler/src/clorraine/cli.rs` | `--release` help and `Options.release` doc | Describe GC + strip for Linux/Amiga; note freestanding/non-Linux keep symbols |
| `spec/17-codegen.md` §13 | Frozen compile matrix | Update only the two GCC release rows (user ruling AR-19) |
| `tools/perf/bench.py` | Benchmark harness: flags, measure, compare | Mirror the shipped release flags; add `m68k_release_bytes`; keep debug flags and equality checks |
| `tools/perf/tests/test_bench.py` | Harness unit tests | Pin the shipped flags, the new metric, and the driver cross-check |
| `docker/scripts/bench-shape.sh` | Cross-toolchain shape gate | Add the GC/strip flags to the warning-bearing nix20 line; compile the nix13 fixture with section flags |
| `Makefile` | `sdk-advanced-build`, `bench-*` targets | Add the release Workbench/services build and `bench-flags` |
| `examples/amiga/workbench.clo`, `services.clo` | Workbench/services demos | No source change; build mode changes |
| `docs/known-limitations.md` | Accepted limitations | Record the stripped release and the debug-symbol path |
| `plans/v1-production-readiness/00-ambiguity-register.md` | The register the limitations doc requires | Add the stripped-release entry |
| `benchmarks/README.md` | Suite and harness description | Shipped flags, attribution, `m68k_release_bytes` |
| `tools/perf/flag_audit.py` | New audit gate | Emit/build old/new variants, assert R3 floors, repeat |

### Code Analysis

`toolchain::command` builds `<program> <base> <mode> [warnings] -o <bin> <c>
<libs>`. The mode is a single global `RELEASE_FLAGS = ["-O2"]`
(`toolchain.rs:48`), applied to every profile. A per-profile release slice on
`Tool` is the smallest change that keeps the existing argument order and lets
`freestanding` stay at `-O2`.

`run_tool(Profile::Freestanding, native_syscalls)` builds a **host-run**
artifact with `FREESTANDING_HOST_BASE` or `FREESTANDING_HOST_FALLBACK_BASE`
and the same global release mode (`toolchain.rs:122-136`). A global release
change would silently add GC/strip there, which R2.3/R3.8 forbid; the
freestanding host-run tool needs its own release slice and tests for both
`native_syscalls` values.

The host release command depends on the linker. Linux GNU ld/lld supports
`-Wl,--gc-sections`; macOS `ld64` does not (it uses `-dead_strip`). The driver
runs on the host, so the choice must be a pure predicate that can be tested
for both outcomes on the Linux gate host, not a bare `cfg!` branch whose
non-Linux test never runs.

`tools/perf/bench.py` independently hard-codes `HOST_RELEASE` and
`M68K_RELEASE` (`:39-41`) and records them in the document (`:493-498`). The
compare gate requires the `flags` field to match (`:625-630`) but does **not**
compare `compiler`, so the base identity must be asserted explicitly in the
findings.

The shape gate uses warning-bearing GCC lines
(`docker/scripts/bench-shape.sh:27-28`); a literal replacement with the
shipped release line would drop `-Wall -Wextra -Werror`. The nix13 leg is
compile-only, so it cannot prove a GC link; the A500 release gates are the
link proof.

`sdk-advanced-build` builds the Workbench/services demos in debug mode to the
default paths (`Makefile:294-297`), and the a600/a1200 gates run those paths
(`:299-303`). The Workbench/AppIcon path is therefore not exercised with
GC+strip. The 1.3 corpus scripts hard-code `-O0 -g` and do not build
`hosted-amiga-13`; the release 1.3 corpus run must come from the audit
procedure.

## Gaps Identified

| Gap | Current behavior | Required behavior | AR Ref |
|---|---|---|---|
| Release DCE | Unused shim functions stay in `.text` | Section GC drops them on host Linux and both Amiga profiles | AR-04 |
| Release symbols | Release binaries keep the symbol table | `-s` strips release artifacts; debug keeps symbols | AR-05 |
| Host portability | One global release line | Linux gets GC+strip; other hosts keep `-O2` | AR-06 |
| Freestanding | Global release line would change it | m68k and host-run freestanding release stay `-O2` | AR-07 |
| Spec truth | §13 pins the old release lines | The two GCC release rows match the shipped flags | AR-19 |
| Harness fidelity | Harness builds `-O2` only | Harness builds the shipped flags; records binary bytes | AR-08 |
| Attribution | No old-vs-new same-compiler matrix | `flag_audit.py` asserts the R3 floors | AR-09/AR-23 |
| Release Workbench | Debug-only Workbench gate | Release Workbench/services build runs the same goldens | AR-20/AR-24 |
| Constructor proof | Benchmarks only | Removed init-list sections enumerated and classified | AR-13 |
| Flag drift | Three independent copies | Driver/harness/audit cross-check | AR-22/AR-25 |

## Dependencies

### Internal Dependencies

- `toolchain::command` is the single build-command authority; the harness and
  audit must describe the same flags.
- The spec matrix is the acceptance contract; the user ruling (AR-19) is the
  only authorization to edit it.
- The release Workbench gate depends on the existing a600/a1200 smoke scripts
  and their goldens.
- The audit depends on `clc` emission, `m68k-amigaos-gcc`, `vamos`, and the
  pinned SDK image.

### External Dependencies

None. The feature adds no crate, Python package, npm package, service, or
build script beyond the existing tools.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A required libnix constructor is collected | Medium | High | AR-13 audit + release Workbench gate + full verify; stop or add `KEEP` |
| A release Workbench build changes a golden | Low | High | Run the a600/a1200 gates; a failure stops |
| A scored benchmark regresses | Low | High | `make bench-compare` 1% guard; the emission is unchanged |
| The harness and driver drift | Medium | Medium | Cross-check test (AR-22/AR-25) |
| A gate flakes | Medium | Low | One audited rerun; a reproducible failure stops (AR-21) |
| The base commit moves | Low | Medium | Pin `f9d5be8`; assert the full hash and `dirty == false` |
