# Requirements: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Revision**: approved — requirements preflight passed
> **Status**: Approved
> **Base**: current `main` head `f9d5be8` (post-PLAN-25; `clc 1.1.1`)

## Feature Overview

`clc` emits one C translation unit that embeds the runtime shim, the standard
library support, and the program. Release builds compile that unit and link the
Amiga libraries. Because the shim and the program are one object, the linker
cannot discard an unused shim function unless the compiler first places each
function and datum in its own section. The result is that a 300-line program
ships with roughly 77 KB of m68k `.text`, most of it unreachable runtime code.

Two standard GNU toolchain options fix this without touching the emitted C:

- `-ffunction-sections -fdata-sections` give every function and datum its own
  section;
- `-Wl,--gc-sections` removes sections that no reachable symbol references;
- `-s` removes the symbol table from the release artifact.

An exploratory probe on the current tree measures the effect with the same
`clc` binary and the pinned toolchain. Only the flags differ.

### Measured evidence (exploratory, re-run in Phase 1)

| Profile / program | Metric | Current `-O2` | GC + strip | Delta |
|---|---|---:|---:|---:|
| hosted-amiga calibration | `.text` | 76,368 | 5,960 | −92.2% |
| hosted-amiga sieve | `.text` | 76,728 | 23,832 | −68.9% |
| hosted-amiga matrix | `.text` | 76,676 | 23,608 | −69.2% |
| hosted-amiga strings | `.text` | 77,604 | 24,196 | −68.8% |
| hosted-amiga moves | `.text` | 76,896 | 23,632 | −69.3% |
| hosted-amiga lookup | `.text` | 92,976 | 39,904 | −57.1% |
| hosted-amiga calibration | file bytes | 99,948 | 7,328 | −92.7% |
| hosted-amiga sieve | file bytes | 100,332 | 25,676 | −74.4% |
| hosted-amiga matrix | file bytes | 100,276 | 25,444 | −74.6% |
| hosted-amiga strings | file bytes | 101,500 | 26,080 | −74.3% |
| hosted-amiga moves | file bytes | 100,632 | 25,460 | −74.7% |
| hosted-amiga lookup | file bytes | 116,584 | 41,740 | −64.2% |
| hosted-amiga-13 calibration | `.text` | 53,432 | 5,908 | −88.9% |
| hosted-amiga-13 sieve | `.text` | 53,792 | 23,780 | −55.8% |
| hosted-amiga-13 corpus (4 programs) | `.text` | 53,512–55,296 | 5,944–28,152 | −49.1% to −88.9% |
| host calibration | file bytes | 18,384 | 14,480 | −21.2% |
| host sieve | file bytes | 22,528 | 14,536 | −35.5% |
| host matrix | file bytes | 22,520 | 14,536 | −35.5% |
| host strings | file bytes | 22,912 | 14,536 | −36.6% |
| host moves | file bytes | 22,648 | 14,536 | −35.8% |
| host lookup | file bytes | 38,944 | 30,920 | −20.6% |

Runtime is unchanged. The scored net instruction count is identical because
the harness subtracts the calibration run. The raw calibration count falls
from 3,372 to 2,928 instructions (−13.2%): `--gc-sections` removes the
`__amigapath.o` and `environ.o` objects and their init-list entries
(`___initamigapath`, `___fillenviron`), which run before `main`. The
remaining `___INIT_LIST__` entries still run. All benchmark outputs are
byte-identical.

The freestanding corpus image is 208 bytes `.text` at `-O2` and 232 bytes with
GC (+11.5%); the freestanding profile is therefore excluded from the flag
change. Stripping alone would cut its 1,552-byte file to 244 bytes (GC+strip
gives 268), but the profile is a provisional LorraineOS ABI and is deferred.

The rejected candidates are measured too: `-O3` grows `.text` by 14.9% with
no runtime win; `-Os` shrinks `.text` by 8.8% but makes `strings` 13.1%
slower; `-fomit-frame-pointer` changes `.text` by −0.1%; LTO alone is worse
than GC, and LTO+GC saves only 2–11% more than GC while adding link-time and
toolchain risk.

### Constructor-removal risk

`--gc-sections` removes the `__amigapath.o` and `environ.o` objects and their
`.list___INIT_LIST__` entries (`___initamigapath`, `___fillenviron`) when
nothing references them. The remaining constructor list still runs. The
444-instruction reduction is those two constructor bodies. The benchmark and
corpus outputs are identical, and no current `stdlib/` or `compiler/runtime/`
code references `environ`, `__amigapath`, `getenv`, or `__progname`. The
optional shims (network, RTG, Workbench, DOS, args) are the real risk surface.
The feature must audit the removed sections and pass the full `make verify`
gate plus the release-path additions below. A required constructor or table
that disappears stops the landing; a targeted `KEEP`/`-u` is the fallback and
requires a fresh audit.

## Scope Boundary

**In scope:** the repeatable flag audit; the release flag change in
`compiler/src/clorraine/toolchain.rs`; the two GCC release rows in
`spec/17-codegen.md` §13 (user ruling AR-19); the benchmark-harness alignment
in `tools/perf/bench.py`; the shape-gate release leg in
`docker/scripts/bench-shape.sh`; the release Workbench/services build and the
1.3 release corpus run (AR-20); unit tests that pin every command vector and
cross-check the harness flags; the constructor/section audit; the `--release`
CLI help and the documentation/register updates; issue registration; the
roadmap closeout; the required reviews and full gate.

**Out of scope:** any language, grammar, diagnostic, ABI, standard-library, or
runtime-shim behavior change; any emitted-C change; any other spec chapter or
row; new target profiles; 68020/68040/FPU work (#15); a public `--size` or
`-Os` mode; LTO; macOS host flags; freestanding GC/strip; shim splitting;
`-ffast-math` or any safety-flag removal; benchmark workload changes; new
crates, build scripts, or npm dependencies; version bumps or release
publication.

## Scope Decisions

| Decision | AR Ref |
|---|---|
| The feature delivers the flag audit, the shipped GC+strip release line, harness alignment, and the docs/issues closeout | AR-01 |
| The workflow is requirements → preflights → plan → execution under auto-design | AR-02 |
| The benchmark base is the current `main` head `f9d5be8`, not a release tag | AR-03 |
| The shipped release flags are `-O2` plus section GC and `-s`; `-O3`, `-Os`, frame-pointer, and LTO lose | AR-04 |
| Release builds are stripped; debug builds keep symbols | AR-05 |
| Non-Linux hosts keep the `-O2` fallback | AR-06 |
| Freestanding keeps `-O2` and is excluded | AR-07 |
| The harness builds the shipped flags and the audit carries the attribution | AR-08 |
| The audit uses one compiler, varies only flags, and repeats | AR-09 |
| The materiality floors are 20% m68k, 15% host, 40% 1.3 corpus, 10% startup | AR-10 |
| No safety flag is removed | AR-11 |
| Debug commands stay byte-identical | AR-12 |
| Removed constructor/section tables are audited; a required one stops the landing | AR-13 |
| Docs, issues, and roadmap are updated | AR-14 |
| No version bump or publication | AR-15 |
| Reviews are correctness, emission integrity, and a design challenge on attribution | AR-16 |
| Benchmark workloads are frozen | AR-17 |
| The listed levers are out of scope or deferred | AR-18 |
| The user authorizes updating the two GCC release rows in `spec/17-codegen.md` §13 | AR-19 |
| Release Workbench/services and the 1.3 corpus are exercised under GC+strip | AR-20 |
| A missed floor stops; one audited rerun is allowed for a flake | AR-21 |
| The harness and driver flag lists are cross-checked | AR-22 |

## Two Comparisons, One Vocabulary

**Head** means the final feature head. The feature uses two deliberately
separate comparisons.

- **Emission guard (scored):** base `f9d5be8` → head, both built by the
  updated harness with the shipped release flags. `make bench-compare`
  enforces equal output, no net-instruction regression above 1%, and no
  `.text` regression above 1%. Because this feature does not change emitted C,
  the expected deltas are zero; the comparison proves the flag work did not
  disturb emission.
- **Flag audit (attribution):** one `clc` binary, one toolchain, old flags vs
  shipped flags. This is where the size win is measured. It is not a
  `bench.py compare` document because the harness intentionally refuses to
  compare documents with different flag sets; the audit's raw matrix lives in
  `05-findings.md` and is reproducible from the recorded procedure.

**Base:** `f9d5be8`, the current `main` head after the constant-array closeout.
`v1.1.1` predates that feature and is not a valid attribution base; the
`v1.1.0` measurements stay as historical evidence only.

## Functional Requirements

### R1 — Repeatable flag audit

- **R1.1** Measure the candidate set once in the Phase 1 audit: current
  release (`-O2`), section GC
  (`-ffunction-sections -fdata-sections -Wl,--gc-sections`), section GC +
  strip (`-s`), `-O3`, `-Os`, `-fomit-frame-pointer`, LTO, LTO+GC, and the
  host-only unwind-table candidates. The freestanding profile is measured with
  and without GC. The repeat gate then covers base/GC/new only.
- **R1.2** Cover the six `benchmarks/*` programs (calibration plus the five
  scored ones) on `hosted-amiga` and `host`, calibration, sieve, and the four
  portable corpus programs (`buffer-container`, `file-copier`,
  `string-parser`, `volatile-mmio`) on `hosted-amiga-13`, and the
  freestanding corpus program.
- **R1.3** Record for each row: emitted C bytes, `.text`, `.data`, `.bss`,
  release binary bytes, raw instruction count where traced, program output,
  compiler SHA-256, compiler commit, toolchain identity, and the exact flags.
  Use one fixed output path per program and rebuild it in place for each
  variant, so symbol-table path length cannot move the host byte counts.
- **R1.4** Use one `clc` binary and one pinned toolchain within the audit;
  only the candidate flags vary.
- **R1.5** Run the repeat set twice and require identical section sizes,
  binary bytes, instruction counts, and outputs. The repeat set is
  base/GC/GC+strip on the five scored programs plus calibration,
  base/GC/GC+strip on the four named 1.3 corpus programs, and the
  freestanding pair; the rejected candidates are measured once in the Phase 1
  probe and recorded, not repeated by the gate. A difference is a
  nondeterminism finding, not noise to average away.
- **R1.6** Record the exact reproduction commands and the raw results in
  `05-findings.md`. A committed script or make target is the gate: it emits C
  with `clc` and compiles it with explicit old/new flag arrays, never using
  the driver's `--release` line for the old row. The new-flag array comes from
  the harness constants or a shared manifest, not a fresh copy. The procedure
  reproduces the table without editing benchmark sources and exits nonzero if
  any floor in R3 is missed. A copy-paste command block in the findings is
  documentation only, not the gate.
- **R1.7** Enumerate the non-function sections removed by `--gc-sections`
  (at minimum `.list___INIT_LIST__`) and classify each as unused or required.
  Record the remaining init-list entries and prove they still run. The audit
  must name the libnix objects (`__amigapath.o`, `environ.o`) and the removed
  constructors (`___initamigapath`, `___fillenviron`).

### R2 — Shipped release flags

- **R2.1** On a Linux/GNU host, the `host` release command is
  `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`.
- **R2.2** The `hosted-amiga` and `hosted-amiga-13` release commands add
  `-ffunction-sections -fdata-sections -Wl,--gc-sections -s` to their existing
  base flags and keep `-lamiga -lm` last.
- **R2.3** The `freestanding` release command stays exactly at its current
  `-O2` line; no GC and no strip. The freestanding **host-run** release
  command (`run_tool(Profile::Freestanding, ..)` for both
  `native_syscalls` values) also stays at `-O2` with no GC and no strip.
- **R2.4** Every debug command stays exactly at its current
  `-O0 -g -Wall -Wextra -Werror` line; no GC and no strip.
- **R2.5** A non-Linux host keeps the current `-std=c99 -O2` release
  command, so the host profile never fails on a linker without GNU
  `--gc-sections`. The platform/linker choice is a pure predicate so both
  branches are exercised on the Linux test host; a `cfg!`-only branch with a
  test that never runs is not acceptable.
- **R2.6** The release flag list has one source of truth per profile in the
  driver; unit tests pin the exact debug and release argument vectors for
  `host`, `hosted-amiga`, `hosted-amiga-13`, `freestanding`, both freestanding
  host-run branches, and the host fallback. A harness test cross-checks the
  Python release vectors against the driver's vectors.
- **R2.7** Existing base flags (`-std=c99`, `-mcrt=nix20`/`nix13`, `-m68000`,
  `-fwrapv`), debug warnings, and link libraries are preserved byte-for-byte.
- **R2.8** The emitted C is unchanged. No compiler front-end, IR, codegen,
  stdlib, or runtime-shim source changes are part of this feature.
- **R2.9** Update the two GCC release rows in `spec/17-codegen.md` §13 to the
  shipped flags, as ruled by the user (AR-19). The debug, vbcc, and
  freestanding rows stay unchanged. A driver spec test reads the table and
  pins the two new rows plus the six unchanged rows; the `toolchain.rs`
  module doc matches the new matrix.

### R3 — Size and runtime acceptance

- **R3.1** Every scored benchmark's m68k `.text` under the shipped flags is at
  least 20% smaller than the same `clc` emission under the old flags.
- **R3.2** Every scored benchmark's stripped m68k release binary is at least
  20% smaller than the old release binary.
- **R3.3** Every scored benchmark's host release binary is at least 15%
  smaller than the old release binary.
- **R3.4** Each of the four named `hosted-amiga-13` corpus programs
  (`buffer-container`, `file-copier`, `string-parser`, `volatile-mmio`)
  has a `.text` at least 40% smaller under the shipped flags than under the
  old flags.
- **R3.5** The calibration raw instruction count is at least 10% lower than
  the same `clc` emission under the old flags (the flag audit), not the
  base/head emission guard.
- **R3.6** No scored benchmark's net instruction count is more than 1% worse
  than base; the expected delta is zero.
- **R3.7** Every benchmark, corpus program, and acceptance demo produces the
  same output as before; the harness output checksums are equal.
- **R3.8** The freestanding release command and its image stay unchanged.
- **R3.9** `05-findings.md` records the exact measured values, not only the
  floors. The floors are regression guards; the findings carry the headline
  numbers.
- **R3.10** A missed floor stops the landing and is escalated; the floor is
  never lowered to pass. The audit compares with exact integer arithmetic
  (`new * 100 <= old * floor_percent`) and checks every group before exiting.
  A flaky gate may be rerun once with the failing leg and output recorded; a
  reproducible failure stops.
- **R3.11** If a constructor `KEEP`/`-u` is added, the audit and every floor
  are re-run before the landing.

### R4 — Harness and gate integration

- **R4.1** `tools/perf/bench.py` builds its `host` and `hosted-amiga` release
  artifacts with the shipped release flags, so `head.json` describes the
  product build rather than a measurement-only build.
- **R4.2** The harness records `m68k_release_bytes` alongside the existing
  section sizes and `host_release_bytes`; the field is written for every
  scored benchmark. `measure()` returns the full record for calibration too,
  but the document's `calibration` summary keeps only `instructions` and
  `host_release_ns`.
- **R4.3** The harness debug flags stay `-O0 -g -Wall -Wextra -Werror`.
- **R4.4** The harness keeps the `flags`, `toolchain`, `runs`, and `profiles`
  equality checks; a cross-flag comparison still fails loudly.
- **R4.5** Regenerate `build/perf/base.json` with
  `make bench-base COMMIT=f9d5be8` and the updated harness. The findings
  assert `base.json.compiler.git` is the full `f9d5be8` hash; `dirty` is a
  harness-tree cleanliness check, not base-commit provenance. The version
  string alone is not a discriminator.
- **R4.6** `docker/scripts/bench-shape.sh` adds the shipped GC/strip flags to
  the existing warning-bearing nix20 shape line (keeping
  `-Wall -Wextra -Werror`), compiles the nix13 fixture with
  `-ffunction-sections -fdata-sections` (compile-only; it proves compilation,
  not linking), and keeps the freestanding and vbcc legs on their current
  flags. The shape gate stays warning-free. Separately, `sdk-advanced-build`
  keeps its warning-bearing debug build and adds a release build/leg for the
  Workbench and services demos, so the existing a600/a1200 gates run GC+strip
  binaries without losing the strict-warning compile; the 1.3 release corpus
  run is part of the audit evidence.
- **R4.7** The final gate includes `make check`, `make asan`, `make ubsan`,
  `make corpus-amiga`, `make differential`, `make bench-shape`,
  `make bench-base COMMIT=f9d5be8`, `make bench`, `make bench-compare`,
  `make bench-flags`, `make verify`, the release Workbench/services gates,
  and the 1.3 release corpus run.

### R5 — Tests

- **R5.1** A Rust unit test pins the exact release command vector for every
  profile and every mode, including the `-o`, source, and library positions,
  and the freestanding host-run release vectors for both `native_syscalls`
  values.
- **R5.2** A Rust unit test proves debug commands contain no
  `-ffunction-sections`, `-fdata-sections`, `--gc-sections`, or `-s`.
- **R5.3** A Rust unit test proves the non-Linux host fallback keeps exactly
  the current `-std=c99 -O2` release command, with both predicate outcomes
  exercised on the Linux host.
- **R5.4** A Python harness test pins the release flag strings and proves
  `m68k_release_bytes` is measured and written. A second harness test
  cross-checks `HOST_RELEASE`/`M68K_RELEASE` against the driver's release
  vectors composed with their base flags, so the harness cannot describe a
  different build. The audit procedure's new-flag array comes from those
  harness constants (or a shared manifest), and the cross-check covers it,
  including the `-mcrt=nix13` composition.
- **R5.5** A shape-gate test or gate run proves the shipped flags compile the
  shape fixture warning-free with the existing warning flags.
- **R5.6** The audit procedure is exercised twice; both runs agree exactly,
  and the procedure exits nonzero on a missed floor.
- **R5.7** No specification golden, expected output, or benchmark source is
  edited to make a test pass, except the two authorized GCC release rows in
  `spec/17-codegen.md` §13 (AR-19).

### R6 — Documentation, issues, and roadmap

- **R6.1** `benchmarks/README.md` explains that the harness builds the shipped
  release flags, that the flag audit (not `bench-compare`) carries the size
  attribution, and that the harness now records `m68k_release_bytes`.
- **R6.1a** The `--release` help in `compiler/src/clorraine/cli.rs` says
  release builds for Linux host and the Amiga profiles are optimized,
  section-GC'd, and stripped; freestanding and non-Linux host release builds
  keep symbols and skip section GC; debug builds keep symbols. The line and
  the `toolchain.rs` module doc both name the two exclusions explicitly.
- **R6.2** `docs/known-limitations.md` records that programs built by
  `clorraine build --release` on Linux host, hosted-amiga, or hosted-amiga-13
  are stripped; freestanding and non-Linux host keep symbols; a debug build
  is the symbol-bearing artifact. The entry required by that document is added
  to `plans/v1-production-readiness/00-ambiguity-register.md`.
- **R6.3** Register four deferred GitHub issues: LTO/LTO+GC (one issue), an
  opt-in `-Os` size mode, macOS `-dead_strip`, and freestanding strip/GC. Do
  not duplicate the existing 68020/FPU issue #15 or the codegen issues
  #13/#14/#16/#17/#18/#19.
- **R6.4** `05-findings.md` records the audit matrix, the constructor audit,
  the exact acceptance values, the review rulings, and the accepted limits.
- **R6.5** Add the PLAN-26 roadmap row at RD Drafted, update the progress
  counter, then move the row through the stages and close it as Done.
- **R6.6** If the feature adds a make target or a committed audit script,
  `AGENTS.md` gains exactly one command-table row for it; no other guidance is
  restated.

### R7 — Reviews and acceptance gates

- **R7.1** The requirements preflight and the plan preflight pass with no
  open critical or major finding.
- **R7.2** Each phase that changes the build command or measurement gets an
  independent correctness review and an emission-integrity review; the audit
  and attribution design also gets a design challenge. The reviewer packets
  include the blast-radius step (every profile, mode, and call site of the
  command), the test-integrity step (the assertion fails without the change),
  and, for the correctness reviewer, an explicit check that no required
  constructor or runtime safety helper is collected.
- **R7.3** No critical or major review finding remains open at closeout.
- **R7.4** The full gate in R4.7 is green on the final head.
- **R7.4a** A flaky gate may be rerun once with the failing leg and its
  output recorded; a reproducible failure stops the landing. The findings
  record every rerun.
- **R7.5** The final branch is pushed only when R7.4 is green; `main` equals
  `origin/main` afterwards.

## Non-Functional Requirements

- **N1 — Behavior:** every program's observable output and exit status are
  unchanged. No evaluation order, overflow, null, bounds, or ownership
  behavior changes.
- **N2 — Safety:** no runtime check, bounds check, diagnostic, sanitizer leg,
  or fault probe is removed or weakened. `-ffast-math`, `-fno-stack-protector`,
  and similar flags are forbidden.
- **N3 — Determinism:** the audit uses the pinned image and fixed flags; the
  repeat run is byte-identical. Host wall time is advisory, never scored.
- **N4 — Dependencies:** no new Rust crate, Python package, npm package,
  build script, service, or external tool. The audit uses the existing
  toolchain and Python standard library.
- **N5 — No benchmark tuning:** workloads, expected outputs, and the scored
  set are unchanged.
- **N6 — Frozen surfaces:** the language, the ABI, the stdlib semantics, and
  the runtime shim behavior are unchanged. The only `spec/` edit is the two
  GCC release rows in §13 authorized by the user (AR-19); every other
  chapter and row is untouched.
- **N7 — Portability:** the Linux/GNU host and the pinned m68k toolchain get
  the win; other host targets keep a working `-O2` fallback.
- **N8 — Maintainability:** the flag set is declared once per profile in the
  driver, pinned by Rust tests, and cross-checked against the harness by a
  Python test; the shape gate's warning-bearing copy is documented as a gate
  variant. The audit procedure is reproducible; the docs explain the
  attribution.
- **N9 — Performance:** the measured size floors in R3 hold, the scored
  runtime does not regress, and the 444-instruction startup reduction is
  recorded.

## Security Considerations

- **Data sensitivity:** N/A — the feature changes build/link flags and a
  benchmark harness; it handles no user data, credentials, or PII.
- **Input validation:** N/A — no new input surface; the driver still receives
  the same source/profile/release arguments.
- **Authentication & authorization:** N/A — no service, account, or role
  surface.
- **Injection risks:** N/A — no new shell interpolation, query, or path
  construction. The audit procedure uses fixed argument lists.
- **Encryption needs:** N/A — no data at rest or in transit.
- **Rate limiting:** N/A — no endpoint or public service.
- **Infrastructure:** release binaries lose symbols (`-s`); this is a
  debuggability limitation, not a secret exposure. Link-time DCE must not
  remove a runtime safety check or a required constructor; N2 and AR-13 make
  that a stop condition.
- **Security testing:** the full ASan/UBSan, fuzz, fault-probe, differential,
  and acceptance gates stay green; no security test is skipped or weakened.

## Out-of-Scope Candidates

| Candidate | Measured effect | Disposition |
|---|---|---|
| `-O3` | `.text` +14.9% on calibration; runtime equal | Rejected; no win |
| `-Os` as the default | `.text` −8.8%; `strings` +13.1% runtime | Rejected; the opt-in `-Os`/`--size` mode is one deferred issue |
| `-fomit-frame-pointer` | `.text` −0.1%; runtime equal | Rejected; no material win |
| `-flto` / `-flto` + GC | LTO alone is worse than GC; LTO+GC is 2–11% smaller with extra link cost/risk | Deferred as one issue |
| `-fno-asynchronous-unwind-tables` / `-fno-unwind-tables` | No host size change | Rejected |
| 68020/68040/FPU profiles | Not measured here; breaks the A500 baseline if defaulted | Existing issue #15 |
| Public `--size`/`-Os` mode | Same deferred issue as the opt-in `-Os` mode above | Not authorized; one issue, not two |
| macOS `-dead_strip` | Untested | Deferred as an issue |
| Freestanding GC/strip | GC grows the tiny image; strip trades symbols on a provisional ABI | Deferred as an issue |
| Shim splitting / whole-program DCE | GC makes it unnecessary | Rejected |
| `-ffast-math`, `-fno-stack-protector`, `-fno-wrapv` | Safety/behavior trade | Forbidden |
| Benchmark workload changes | Would invalidate attribution | Forbidden |
| Version bump / tag / publish | Not requested | Out of scope |

## Acceptance Summary

| Gate | Pass condition |
|---|---|
| Requirements preflight | No open critical/major finding |
| Plan preflight | No open critical/major finding |
| Spec | Only the two authorized GCC release rows in §13 change; all other rows/chapters unchanged |
| Rust tests | Exact release/debug vectors, both freestanding host-run branches, and the host fallback pinned |
| Harness tests | Shipped flag strings, driver cross-check, and `m68k_release_bytes` pinned |
| Audit repeatability | Two runs identical; the procedure exits nonzero on a missed floor |
| m68k `.text` | ≥20% smaller on every scored benchmark |
| m68k stripped binary | ≥20% smaller on every scored benchmark |
| host release binary | ≥15% smaller on every scored benchmark |
| 1.3 corpus `.text` | ≥40% smaller on each of the four named programs |
| Calibration instructions | ≥10% lower |
| Scored net instructions | No benchmark >1% worse |
| Outputs | Byte-identical everywhere |
| Freestanding | Release commands (m68k and host-run) and image unchanged |
| Release path | Workbench/services gates run GC+strip binaries; 1.3 release corpus outputs checked |
| Full gate | `make check`, `asan`, `ubsan`, `corpus-amiga`, `differential`, `bench-shape`, `bench-base`, `bench`, `bench-compare`, `bench-flags`, `verify` all green |
| Reviews | Correctness, emission integrity, design challenge; no open critical/major |
| Roadmap | PLAN-26 row added and Done; four issues registered |
