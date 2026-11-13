# Findings: Opt-in Size Build Mode (PLAN-37)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-13

## 1. Baseline

- Base commit: `e65de1ca47423199f09f1f1098416bfede125741`
  (`main == origin/main`, clean tree).
- Compiler: `clc 1.1.2`.
- Container: the pinned Docker SDK image from `docker/`.

The base driver has no `--size` flag:

```
$ ./target/debug/clorraine build --size examples/hello.clo
usage error: unknown option '--size'
```

The base help line is:

```
      --release                       Optimize (-O2); strip + section-GC on Linux/Amiga; freestanding/non-Linux and debug keep symbols
```

The base toolchain unit suite passed 13 cases, pinning the debug and `-O2`
release vectors.

## 2. Red Phase

The tests were written before the implementation. The focused unit run failed
because the new model did not exist:

```
$ cargo test -p clc --lib clorraine::cli::tests
error[E0432]: unresolved import `super::toolchain::BuildMode`
error[E0609]: no field `mode` on type `Options`
error[E0560]: struct `Tool` has no field named `size`
error[E0425]: cannot find function `host_size_flags` in this scope
error[E0433]: cannot find type `BuildMode` in this scope
```

The base driver's `--size` behaviour was captured directly:

```
$ ./target/debug/clorraine build --size examples/hello.clo
usage error: unknown option '--size'
```

The end-to-end driver cases are in the same crate, so the red run is the
compile failure above; the base binary's observable rejection is the runtime
proof.

## 3. Implementation

- `compiler/src/clorraine/toolchain.rs`: added `BuildMode` (`Debug`,
  `Release`, `Size`) and `Tool.size`; added `HOST_SIZE_GNU`,
  `HOST_SIZE_FALLBACK`, `M68K_SIZE_FLAGS`, and `FREESTANDING_SIZE_FLAGS`;
  `command()` now selects the mode slice and keeps warnings debug-only.
- `compiler/src/clorraine/cli.rs`: replaced `Options.release: bool` with
  `Options.mode: BuildMode`; parsed `--release` and `--size` through
  `select_mode`; rejected the conflict and rejected `--size` with `emit`; added
  the help line.
- `compiler/src/clorraine/build.rs` and `mod.rs`: thread `BuildMode` through
  `build`, `compile`, the normal run path, and the freestanding host-run leg.
- `docs/known-limitations.md` §14 and `AGENTS.md`: documented the opt-in mode
  and the workload-dependent trade.

The default debug and release vectors are unchanged; the unit cases and the
driver spec row that pins the two GCC release lines still pass.

## 4. Measurements

All measurements use the delivered driver and the pinned SDK image. Size and
release builds share the same emitted C; the emitted-C identity check compared
the seven program/profile pairs and reported `identical` for every pair.

### 4.1 Per-profile artifact comparison

`calibration` is the startup/loop probe; `strings` is the allocation-heavy
probe. The freestanding profile cannot build `strings` because `std.text` and
`std.fmt` are not available there, so it uses `calibration` only.

| Program | Profile | Mode | C bytes | Artifact bytes | `.text` | `.data` | `.bss` |
|---|---|---|---:|---:|---:|---:|---:|
| calibration | host | release | 25,102 | 14,480 | 1,543 | — | — |
| calibration | host | size | 25,102 | 14,480 | 1,554 | — | — |
| strings | host | release | 30,015 | 14,528 | 4,007 | — | — |
| strings | host | size | 30,015 | 14,520 | 3,530 | — | — |
| calibration | hosted-amiga | release | 313,314 | 7,320 | 5,952 | 284 | 116 |
| calibration | hosted-amiga | size | 313,314 | 7,380 | 6,004 | 284 | 116 |
| strings | hosted-amiga | release | 318,152 | 25,844 | 23,972 | 292 | 116 |
| strings | hosted-amiga | size | 318,152 | 25,820 | 23,952 | 292 | 116 |
| calibration | hosted-amiga-13 | release | 181,934 | 7,264 | 5,900 | 272 | 120 |
| calibration | hosted-amiga-13 | size | 181,934 | 7,292 | 5,924 | 272 | 120 |
| strings | hosted-amiga-13 | release | 186,772 | 25,788 | 23,920 | 280 | 120 |
| strings | hosted-amiga-13 | size | 186,772 | 25,768 | 23,904 | 280 | 120 |
| calibration | freestanding | release | 14,278 | 1,552 | 208 | 0 | 0 |
| calibration | freestanding | size | 14,278 | 1,504 | 160 | 0 | 0 |

The m68k `.text` is effectively neutral after section GC: +52 bytes on
`calibration` (nix20), −20 bytes on `strings` (nix20), and within 24 bytes on
the 1.3 pair (+24 on `calibration`, −16 on `strings`). The host `.text` falls
on `strings` (−477 bytes, −11.9%) and the freestanding `.text` falls by 48
bytes. Artifact bytes are dominated by headers/alignment, so several pairs
differ by only 8–60 bytes while `.text` moves more.

### 4.2 Scored-suite sweep

The same driver builds each committed benchmark in both modes. Instructions are
raw `vamos` counts; the net change subtracts the matching calibration count.

| Program | m68k `.text` release → size | Host `.text` release → size | Instructions release → size | Net change |
|---|---:|---:|---:|---:|
| calibration | 5,952 → 6,004 | 1,543 → 1,554 | 2,928 → 2,991 | +63 (+2.15%) |
| sieve | 23,816 → 23,816 | 3,443 → 3,045 | 1,043,009 → 1,043,277 | +205 (+0.02%) |
| matrix | 23,592 → 23,756 | 3,297 → 2,965 | 12,289 → 11,609 | −743 (−7.9%) |
| strings | 23,972 → 23,952 | 4,007 → 3,530 | 1,224,160 → 1,228,166 | +3,943 (+0.32%) |
| moves | 23,684 → 23,752 | 3,423 → 3,217 | 47,558 → 61,607 | +13,986 (+31.3%) |
| lookup | 39,884 → 39,912 | 19,459 → 19,260 | 11,707 → 11,701 | −69 (−0.79%) |
| local-table | 24,568 → 24,580 | 4,135 → 3,945 | 5,855 → 5,640 | −278 (−9.5%) |

The host `.text` falls for every benchmark except `calibration` (+11 bytes),
with the largest drop on `strings` (−11.9%). The m68k `.text` is neutral to
slightly larger. Runtime is workload-dependent: `matrix`, `lookup`, and
`local-table` improve, while `moves` regresses by 31.3% net and `strings` by
0.3%.

The `hosted-amiga-13` runtime probe confirms the same picture: calibration
3,043 → 3,039 instructions (−0.13%) and `strings` 1,688,835 → 1,692,829
(+0.24%). The freestanding artifact cannot run on the m68k target here; its
`.text` and file bytes are the measured comparison.

Host wall time over 31 runs is advisory only (the host is shared and the
programs are short): `calibration` median 1.08 ms → 0.65 ms and `strings`
0.85 ms → 1.06 ms. These numbers are too noisy to attribute and are not used
for a decision.

### 4.3 Relationship to the issue evidence

The issue cited calibration `.text` 76,368 → 69,756 (−8.7%) and `strings` raw
instructions 2,967,423 → 3,357,540 (+13.1%). Both were measured before the
current release line had section GC and before the `strings` formatting
rewrites:

- With `-ffunction-sections -fdata-sections -Wl,--gc-sections`, the retained
  m68k `.text` is about 6 KB for calibration, not 76 KB. The `-Os` delta on the
  retained code is therefore near zero, not −8.7%.
- The current `strings` baseline is 1,224,160 instructions, not 2,967,423. The
  `-Os` cost is +0.32%, not +13.1%; the formatting rewrite removed the code
  that the older `-Os` build penalized. `moves` is the current worst case at
  +31.3%.

The delivered mode is therefore a real, opt-in C-compiler mode, but it is not
a general m68k size win after GC. The default stays `-O2`, and the limitation
is recorded for users instead of being hidden.

## 5. Test Counts

| Suite | Result |
|---|---|
| `cargo test -p clc --lib` | 460 passed, 0 failed |
| `cargo test -p clc --test clorraine_driver_spec_test` | 22 passed, 0 failed (7 new size cases) |
| Toolchain unit cases | 13 base + 4 size cases, green |

The new CLI cases cover the mode selection, the conflict, the repeated flag,
and the `emit` rejection. The new toolchain cases pin the host Linux, host
fallback, both Amiga profiles, and both freestanding host-run branches. The
new driver cases cover `build --size`, `run --size`, the conflict, the `emit`
rejection, the help text, and the Amiga size build.

## 6. Reviews

An independent correctness review covered the phase diff against `e65de1c`,
the blast radius of every `command()`/`Tool`/`BuildMode`/CLI path, the test
integrity of each new assertion, the documentation, and the repository
standards.

**Verdict: PASS with five minor findings; no critical or major findings.**
All five were fixed before closeout:

| # | Finding | Fix |
|---|---|---|
| 1 | The requirements and design repeated the pre-section-GC issue numbers as if they were current | Relabelled them as historical and added the current measured trade (`moves` +31.3% net) |
| 2 | The findings said five new driver cases; the suite has six, and the promised Amiga run case was only a build case | Added `hosted_amiga_size_run_prints_program_output`; the suite is now 22 cases (7 new) |
| 3 | The known-limitations row mixed raw and net instruction deltas and implied release is the default build | Switched to net deltas and stated that `--release` is the `-O2` line while the default build is debug |
| 4 | The roadmap and execution-plan task counts were stale | Updated both and checked the completed Phase 4 tasks |
| 5 | The `BuildMode::Size` doc promised a smaller code size, which is not guaranteed | Reworded to "optimized for code size" and made the 1.3 deltas exact |

The reviewer also observed the real compiler argument vectors with temporary
logging wrappers: host debug/release/size and both freestanding legs matched
the pinned lines. No `spec/`, `Cargo`, `tools/`, or benchmark changes were
present.

## 7. Acceptance

| Gate | Result |
|---|---|
| `make check` | exit 0; 148 workspace suites; `cargo test -p clc --lib` 460 passed; driver spec 22 passed; spec check, setup check, bench-test, fuzz-test, and fuzz-smoke green |
| `make asan` | 38 suites, green |
| `make ubsan` | 12 suites, green |
| `make bench-shape` | host GCC, nix13 GCC, vbcc, and freestanding builds warning-free |
| `make bench-base COMMIT=e65de1c` + `make bench` + `make bench-compare` | every benchmark row `+0.0%`; outputs equal; `compare: passed` |
| `make corpus-amiga` | all portable programs match on hosted-amiga; vbcc build-only legs warning-free |
| `make differential` | 3/3 outputs match |
| `make verify` | `verify: all legs passed` |
| Emitted-C identity | `identical` for all seven program/profile pairs |
| Default vectors | debug and release command vectors byte-identical to the base; the pinning driver spec row still passes |

The first full `make verify` run on the host `:0` display failed at
`sdk-a500-smoke` because FS-UAE produced no capturable window (its log showed
Mesa/GLX errors); the program's C and the default build line were unchanged.
The complete gate was then run on a private Xvfb `:99` display and every leg
passed. `docker/.env` was restored to `DISPLAY=:0` afterwards and no Xvfb
process remains.

Delivered in commit `480f743` (`feat(driver): add opt-in --size build mode`);
issue #21 is closed with the measured trade recorded above.
