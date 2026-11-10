# Findings and Evidence: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Executing — Phase 4 complete; closeout pending

## Purpose

This document owns the measured evidence for PLAN-26: the baseline, the
same-compiler old-vs-new flag audit, the constructor/section audit, the scored
base/head comparison, the exact acceptance values, the gate evidence, the
review rulings, and the accepted limits.

## Baseline

**Base:** the post-PLAN-25 `main` head `f9d5be8` (`clc 1.1.1`); the plan
baseline commit is `bb4645a`. The measured compiler binary SHA-256 is
`08efe2e350bf179f862b0767663796cabf7fdb3aa55e866a171ee40b564fab37`.

**Old release flags:** host `-std=c99 -O2`; hosted-amiga
`-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2`; hosted-amiga-13 the same with
`-mcrt=nix13`; freestanding `-std=c99 -ffreestanding -fno-builtin -nostdlib
-m68000 -fwrapv -O2`. Debug flags are unchanged.

**Preserved evidence:** `build/perf/base-v1.1.0.json` (the previous
cumulative baseline) and `build/perf/head-constant-array.json` (the PLAN-25
head) are retained; `build/perf/plan26-baseline.txt` records the identity.

### Probe Matrix (Phase 1)

The exploratory probe used the same `clc` binary and the pinned toolchain; only
the flags differ. The committed audit re-runs the old/GC/new set in Phase 3.

**m68k old flags (`-O2`, hosted-amiga)**

| Program | `.text` | file bytes | raw instructions |
|---|---:|---:|---:|
| calibration | 76,368 | 99,948 | 3,372 |
| sieve | 76,728 | 100,332 | 1,043,953 |
| matrix | 76,676 | 100,276 | 13,280 |
| strings | 77,604 | 101,500 | 2,967,423 |
| moves | 76,896 | 100,632 | 48,577 |
| lookup | 92,976 | 116,584 | 12,842 |

**host old flags (`-std=c99 -O2`)**

| Program | file bytes |
|---|---:|
| calibration | 18,384 |
| sieve | 22,528 |
| matrix | 22,520 |
| strings | 22,912 |
| moves | 22,648 |
| lookup | 38,944 |

The host byte counts are path-sensitive while symbols are present; the audit
rebuilds each variant in one fixed path and the shipped build is stripped.

**hosted-amiga-13 old flags (`-O2`, nix13)**

| Program | `.text` |
|---|---:|
| calibration | 53,432 |
| sieve | 53,792 |
| buffer-container | 54,540 |
| file-copier | 55,296 |
| string-parser | 55,072 |
| volatile-mmio | 53,512 |

**freestanding**

| Variant | `.text` | file bytes |
|---|---:|---:|
| old `-O2` | 208 | 1,552 |
| GC without strip | 232 | 1,576 |

**Rejected candidates (m68k `.text`, calibration/sieve/matrix/strings/moves/lookup)**

| Candidate | `.text` sequence | Runtime finding |
|---|---|---|
| `-O3` | 87,764 / 88,296 / 89,580 / 89,580 / 88,332 / 104,532 | equal, no win |
| `-Os` | 69,756 / 69,984 / 70,204 / 70,204 / 70,060 / 86,260 | strings raw 2,967,423 → 3,357,540 (+13.1%) |
| `-fomit-frame-pointer` | 76,296 / 76,656 / 76,604 / 77,528 / 76,820 / 92,900 | equal |
| `-flto` | 10,640 / 28,288 / 28,204 / 28,636 / 28,228 / 44,496 | worse than GC |
| `-flto` + GC | 5,284 / 23,208 / — | 2–11% smaller than GC, extra link risk |
| host `-fno-(asynchronous-)unwind-tables` | no host size change | — |

## Red Phase

The specification tests were written before the implementation and failed for
the expected reasons.

| Command | Result | Expected failure |
|---|---|---|
| `cargo test -p clc --lib toolchain::tests` | 8 passed, 4 failed | host, hosted-amiga, 1.3, and `host_release_flags` vectors still have the old `-O2` |
| `cargo test -p clc --test clorraine_driver_spec_test` | 13 passed, 2 failed | the spec release rows and the `--release` help still have the old text |
| `make bench-test` | 56 tests, 2 failures + 14 errors | the harness flags and `binary_bytes` are absent; the audit floors/repeat are skeleton stubs |

The freestanding release vectors and every debug vector passed in the red run:
those lines are intentionally unchanged.

## Release Flag Change

The release command is per-profile after Phase 2:

| Profile / mode | Vector |
|---|---|
| host Linux release | `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` |
| host non-Linux release | `cc -std=c99 -O2 -o prog out.c` |
| host debug | unchanged `-O0 -g -Wall -Wextra -Werror` |
| hosted-amiga release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` |
| hosted-amiga-13 release | the same with `-mcrt=nix13` |
| freestanding m68k release | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o prog out.c -lg` |
| freestanding host-run release | `cc -std=c99 -ffreestanding -fno-builtin [-nostdlib] -O2 -o prog out.c` |

The frozen compile matrix changed exactly two rows (`spec/17-codegen.md:391`
and `:393`); the debug, vbcc, and freestanding rows are byte-identical. A
driver spec test pins all eight rows.

The `--release` help and `Options.release` doc now name the Linux/Amiga
strip+GC set and the freestanding/non-Linux/debug symbol exceptions.

**Pre-existing discrepancy recorded, not edited:** the freestanding release
row says "the same line with `-O2` in place of `-O0 -g`", which would retain
the debug warning flags, while the driver's release line omits warnings. The
user authorized only the two GCC release rows; the freestanding row stays as
it is.

**Green:** `cargo test -p clc --lib toolchain::tests` 12 passed;
`cargo test -p clc --test clorraine_driver_spec_test` 15 passed.

## Audit Matrix

`make bench-flags` ran the old/GC/shipped set twice; every non-timing record
was identical, the repeat check passed, and every floor and constructor check
passed. The matrix is in
`build/perf/flags.json`; the values below are the first run, verified identical to the second.

| Program | old `.text` | new `.text` | Δ | old file | new file | Δ |
|---|---:|---:|---:|---:|---:|---:|
| hosted-amiga/calibration | 76,368 | 5,960 | −92.2% | 99,948 | 7,328 | −92.7% |
| hosted-amiga/sieve | 76,728 | 23,832 | −68.9% | 100,332 | 25,676 | −74.4% |
| hosted-amiga/matrix | 76,676 | 23,608 | −69.2% | 100,276 | 25,444 | −74.6% |
| hosted-amiga/strings | 77,604 | 24,196 | −68.8% | 101,500 | 26,080 | −74.3% |
| hosted-amiga/moves | 76,896 | 23,632 | −69.3% | 100,632 | 25,460 | −74.7% |
| hosted-amiga/lookup | 92,976 | 39,904 | −57.1% | 116,584 | 41,740 | −64.2% |
| hosted-amiga-13/calibration | 53,432 | 5,908 | −88.9% | 67,368 | 7,272 | −89.2% |
| hosted-amiga-13/sieve | 53,792 | 23,780 | −55.8% | 67,748 | 25,620 | −62.2% |
| hosted-amiga-13/buffer-container | 54,540 | 24,820 | −54.5% | 68,776 | 26,848 | −61.0% |
| hosted-amiga-13/file-copier | 55,296 | 28,152 | −49.1% | 69,672 | 30,512 | −56.2% |
| hosted-amiga-13/string-parser | 55,072 | 24,932 | −54.7% | 69,376 | 26,972 | −61.1% |
| hosted-amiga-13/volatile-mmio | 53,512 | 5,944 | −88.9% | 67,532 | 7,312 | −89.2% |
| host/calibration | 7,796 | 1,543 | −80.2% | 18,384 | 14,480 | −21.2% |
| host/sieve | 8,298 | 3,518 | −57.6% | 22,520 | 14,536 | −35.5% |
| host/matrix | 8,168 | 3,385 | −58.6% | 22,512 | 14,536 | −35.4% |
| host/strings | 9,863 | 3,994 | −59.5% | 22,904 | 14,536 | −36.5% |
| host/moves | 8,419 | 3,379 | −59.9% | 22,648 | 14,536 | −35.8% |
| host/lookup | 24,348 | 19,550 | −19.7% | 38,944 | 30,920 | −20.6% |
| freestanding/freestanding | 208 | 208 | 0.0% | 1,552 | 1,552 | 0.0% |

The freestanding GC probe (not the shipped row) is 232 `.text` / 1,576 bytes;
the shipped `-O2` row stays exactly 208 / 1,552.

The scored net instruction count is unchanged: `lookup` is 9,470 on both the
old and shipped variants (12,842/12,398 raw minus 3,372/2,928 calibration).
The host release binaries have no `.symtab` section; debug builds do.

The `clc` identity in `flags.json` is the working-tree build during
development (`dirty: true`). The final scored comparison records the clean
`f9d5be8` base and final head identities separately.

## Constructor and Section Audit

`--gc-sections` removes the `__amigapath.o` and `environ.o` objects and their
`.list___INIT_LIST__` entries. The linker log names:

- `__amigapath.o`: `.text`, `.bss`, `.list___INIT_LIST__`
- `environ.o`: `.text`, `.data`, `.bss`, `.list___INIT_LIST__`

The audit's `constructor_proof()` now asserts the shared init-list walk and
every remaining `___init*` constructor in the GC calibration trace. It maps
the `m68k-amigaos-nm` symbols through the trace's HUNK load base. The proof is
recorded in `flags.json` for both m68k profiles.

**hosted-amiga** (load base 0x260c, 2,928 trace instructions):

| Symbol | Range | Trace hits |
|---|---|---:|
| `__Exit` (shared init-list walk) | 0x7c–0xd4 | 823 |
| `___initstdio` | 0x802–0x950 | 82 |
| `___initcpp` | 0xef8–0xf34 | 7 |
| `___initlibraries` | 0x104e–0x10ac | 40 |
| `___initmalloc` | 0x132e–0x1372 | 14 |

**hosted-amiga-13** (load base 0x260c, the 13 calibration trace):

| Symbol | Range | Trace hits |
|---|---|---:|
| `__Exit` (shared init-list walk) | 0x7c–0xd4 | 823 |
| `___initstdio` | 0x64e–0x7b0 | 80 |
| `___initcpp` | 0xeb0–0xeec | 7 |
| `___initlibraries` | 0x1006–0x1064 | 25 |
| `___initmalloc` | 0x12e6–0x132a | 14 |

The 823 count is the whole `__Exit` range, including its short prologue; the
four constructor hit counts are the entry proof. The 444-instruction 2.0
reduction (401 on 1.3) is the two removed constructor bodies, not a skipped
walk. The remaining constructors execute before `main`.

## Scored Comparison

| Item | Value |
|---|---|
| Base compiler | `f9d5be8d66474f00162ef78fc32ef83bae84aeda` (post-PLAN-25 `clc 1.1.1`), `dirty: false` |
| Head compiler | `e3fdbf33a4ee0a88ec80c8c2227479d01a9c4ef3` (`clc 1.1.1`), `dirty: false` |
| Base binary SHA-256 | `6e4c2298e4ec5c30d07aaa83487a1147e331365942c435e289813d13ca7dedfb` |
| Head binary SHA-256 | `0618657cf8b5e47b8786f8aa2194f0a89217879d8b2cf6946c08866c049c8610` |
| Shipped flags | `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s` on both sides |

`make bench-compare` passed with zero deltas — the expected result because this
feature changes no emitted C.

| Benchmark | Base net ins | Head net ins | Δ | Base `.text` | Head `.text` | Δ |
|---|---:|---:|---:|---:|---:|---:|
| lookup | 9,470 | 9,470 | 0.0% | 39,904 | 39,904 | 0.0% |
| matrix | 9,908 | 9,908 | 0.0% | 23,608 | 23,608 | 0.0% |
| moves | 45,205 | 45,205 | 0.0% | 23,632 | 23,632 | 0.0% |
| sieve | 1,040,581 | 1,040,581 | 0.0% | 23,832 | 23,832 | 0.0% |
| strings | 2,964,051 | 2,964,051 | 0.0% | 24,196 | 24,196 | 0.0% |

The calibration raw count is 2,928 on both sides (down from 3,372 under the
old flags). `lookup`'s `m68k_release_bytes` is 41,740; the head document
records the field for every scored benchmark.

## Gate Evidence

### Phase 3

| Gate | Result |
|---|---|
| `make bench-test` | 77 tests, OK |
| `make bench-flags` | two identical runs; floors, constructor, and repeat checks pass; `runs: 2`; `build/perf/flags.json` |
| `make bench-shape` | nix20 link, nix13 compile, vbcc, freestanding all warning-free |
| `make sdk-advanced-build` | debug copies at `build/amiga-sdk/{workbench,services}-debug`; release at the default paths (27,560 / 34,996 bytes) |
| `make sdk-a600-services` | OK — `wb: ok ...; svc: ok ...` on the release binaries |
| `make sdk-a1200-workbench` | OK — report lines plus the Workbench capture and AppIcon goldens |

### Phase 4

| Gate | Result |
|---|---|
| `make bench-base COMMIT=f9d5be8` | base compiler `f9d5be8…`, `dirty: false`, shipped flags |
| `make bench` | head compiler `e3fdbf3…`, `dirty: false`, `m68k_release_bytes` recorded |
| `make bench-compare` | passed, zero output/instruction/`.text` deltas |
| `make check` | passed (spec check, registry, workspace tests, bench-test, fuzz-test, fuzz-smoke `cases=28 failures=0`) |
| `make asan` | 30 suites passed |
| `make ubsan` | 4 suites passed |
| `make corpus-amiga` | all portable programs match on hosted-amiga; build-only legs built |
| `make differential` | 3/3 output matches |
| `make verify` | `verify: all legs passed` |

## Review Rulings

### Phase 2

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-201 | 2 | MINOR | toolchain doc comment | fixed — stale "filled in" clause removed |
| RV-202 | 2 | MINOR | host command test | fixed — platform-conditional host vector plus a composed non-Linux fallback test |
| RV-203 | 2 | MINOR | plan progress | fixed — Phase 2 tasks and progress updated |
| RV-204 | 2 | MINOR | driver spec test doc/name | fixed — scoped to the two GCC release rows |
| EM-2 | 2 | — | base/head emission diff | no findings — 235 sources and 248 profile runs had zero C/status/output differences |

The two reviewers found no critical or major issue. The emission auditor
confirmed the driver change produces byte-identical C and that the two spec
rows match the actual release vector.

### Phase 3

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-301 / DC-304 | 3 | MAJOR | cross-language flag check | fixed — the audit derives host/2.0/1.3 from the harness vectors; VectorTests compare all four profiles to the Rust base/release constants |
| RV-302 / DC-301 | 3 | MAJOR | expected-output oracle | fixed — every run is compared to the committed expected output; a both-variants-wrong test fails |
| RV-303 / DC-302 / DC-303 | 3 | MAJOR | constructor proof | fixed — `constructor_proof()` runs `nm` + a `vamos` trace, maps the HUNK base, and asserts the init loop and every remaining `___init*` constructor on both m68k profiles; `check_constructor()` fails the gate |
| RV-304 | 3 | MAJOR | `m68k_release_bytes` test | fixed — a mocked `measure()` test and a calibration-summary test |
| RV-305 | 3 | MAJOR | gate failure contract | fixed — tests cover `--check` exit 1, JSON-before-exit, repeat failure, default repeat 2, and missing `readelf` |
| RV-306 | 3 | MINOR | freestanding binary-only change | fixed — a binary-only synthetic case fails |
| RV-307 | 3 | MINOR | status/artifact | fixed — status/date updated; `flags.json` now records `runs: 2` and a passed repeat check |
| RV-308 | 3 | MINOR | trace-hit table | fixed — exact `nm` ranges and hits for 2.0 and 1.3 |
| DC-305 | 3 | MINOR | compare scope | accepted — the scored guard remains `.text` + net instructions; the README now calls it a regression guard and the independent base/head C diff is the emission proof |
| DC-306 | 3 | MINOR | base identity echo | accepted — the full `f9d5be8` hash is asserted in the findings; `dirty` is documented as harness-tree cleanliness |
| DC-307 | 3 | MINOR | repeat independence | accepted — the repeat proves same-condition determinism; `runs: 2` and the repeat check are recorded |
| DC-308 | 3 | MINOR | host/13 evidence | partly fixed — host `.text/.data/.bss` are now measured; the 13 constructor proof is recorded; the relative floors remain tied to the frozen matrix |
| DC-309 | 3 | MINOR | audit gate layer | accepted — `make bench-flags` is a release-evidence gate, not a host-CI leg; the final gate records it |
| DC-310 | 3 | OBSERVATION | audit error paths | fixed — missing `vamos`/`size` are reported as audit errors; the freestanding row is build-only by design |
| DC-311 | 3 | OBSERVATION | audit data model | fixed — `c_bytes` and host sections are recorded |
| DC-312 | 3 | OBSERVATION | output hash | fixed — `bench.measure` hashes the actual m68k stdout |

The emission re-check found no findings; it confirmed zero emitted-C differences
and flagged only that the cited `flags.json` predated the `runs` field. The
audit was re-run after the fix.

### Final Review

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| FR-001 | 4 | MINOR | `--repeat` >2 ran only twice | fixed — the loop runs every requested repeat and compares each to the first; a `--repeat 3` test covers it |
| FR-002 | 4 | MINOR | missing constructor proof passed | fixed — a missing `constructors` key fails; the synthetic clean matrix carries both profiles and all four names |
| FR-003 | 4 | MINOR | `__Exit` range labelled as the `_start` loop | fixed — renamed `init_list_walk`; the findings state the 823 is the `__Exit` range and the constructor hits are the proof |
| FR-004 | 4 | MINOR | 1.3 old vector not exact | fixed — the exact nix13 old and GC vectors are pinned, including the Rust `AMIGA_13_BASE` composition |
| FR-005 | 4 | OBSERVATION | over-named output test | fixed — renamed to the single-variant check |
| FR-006 | 4 | OBSERVATION | status/roadmap bookkeeping | fixed in the closeout — statuses, roadmap, and the final gate table updated |
| EM-F | 4 | — | final base/head emission diff | no findings — 668 profile runs, 155 accepted units byte-identical; strict builds clean |

## Accepted Limits

| Limit | Evidence / workaround |
|---|---|
| Freestanding release keeps `-O2` and symbols | GC grew the 208-byte image to 232; strip-only is 244; provisional ABI. Reopen with #23. |
| Non-Linux host keeps `-O2` | `-Wl,--gc-sections` is GNU; the fallback is unit-tested, not booted. Reopen with #22. |
| Release programs are stripped | Linux/Amiga release builds lose function names; a debug build is the symbol-bearing artifact (`docs/known-limitations.md` §14). |
| Constructor collection | `__amigapath.o`/`environ.o` and their `___initamigapath`/`___fillenviron` entries are removed; the four remaining constructors are proven on both m68k profiles. A future API that needs environment/path state needs a targeted `KEEP` and a fresh audit. |
| Audit is a release gate | `make bench-flags` is not in the host-only CI or `make verify`; run it at release/closeout. The vectors and failure paths are unit-tested. |
| Host attribution is strip-dominated | Host `.text` is measured but only the file-size floor is gated; the host win includes symbol removal. |
| 1.3 native-video/audio/input/layers release paths | Those demos remain debug-gated; the A500 release gates cover assets/gui/exec/diskfont, and the 1.3 constructor proof covers the shared init path. |
| Non-GNU Linux host | The predicate checks Linux, not the linker family; a non-GNU Linux linker would need the fallback. |
| Pre-existing freestanding float link | A freestanding floating-point program needs target math bases; base and head fail identically. The shape gate is compile-only. |
| Freestanding spec-row wording | The release row says "same line with `-O2` in place of `-O0 -g`"; the driver omits debug warnings. Only the two GCC rows were authorized. |

## Closeout

- Deferred issues: [#20](https://github.com/OpenLorraine/clorraine/issues/20)
  LTO/LTO+GC, [#21](https://github.com/OpenLorraine/clorraine/issues/21) opt-in
  size mode, [#22](https://github.com/OpenLorraine/clorraine/issues/22) macOS
  dead-strip, [#23](https://github.com/OpenLorraine/clorraine/issues/23)
  freestanding strip/GC.
- The audit artifact `build/perf/flags.json` records `clc` commit `e3fdbf3`
  and SHA-256 `0618657c…`; the harness tree had uncommitted plan notes at run
  time (`dirty: true`), but the compiler binary is the final head. The scored
  base/head documents are clean (`dirty: false`).
- Final feature code: `29b85fb`; the closeout commit is the final branch
  state, and `main` equals `origin/main` after the final push.

## Final Numbers

| Metric | Result |
|---|---|
| Scored m68k `.text` | 57.1–69.3% smaller (lookup 92,976 → 39,904; calibration 76,368 → 5,960) |
| Scored m68k stripped binary | 64.2–74.7% smaller (lookup 116,584 → 41,740) |
| Host release binary | 20.6–36.5% smaller (lookup 38,944 → 30,920) |
| 1.3 corpus `.text` | 49.1–88.9% smaller |
| Calibration raw instructions | 3,372 → 2,928 (−13.2%) |
| Scored net instructions | unchanged (lookup 9,470; sieve 1,040,581) |
| Emitted C | byte-identical base/head |
| Freestanding image | unchanged (208 `.text` / 1,552 bytes) |
