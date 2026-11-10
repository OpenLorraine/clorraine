# Findings and Evidence: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Executing — Phase 3 record and claim review complete; tag pending

## Purpose

This document is the release evidence ledger: repository state, the
release-tree comparison, the same-compiler flag audit, gate results, artifact
hashes, tag identity, publication record, review rulings, and accepted limits.
`RELEASE.md` is the summary and publication record.

## Baseline

| Item | Value |
|---|---|
| Previous release | annotated tag `v1.1.1` (tag object `144e53b`) → commit `2d1eb3026fc6f75885785e6ff9601f99aaba4021` |
| Pre-plan `main` | `3227321572a7d214e7415e2a5f5724a1d8872853` (equal to `origin/main` at fetch) |
| Plan baseline (C0) | `9fb48ac438b96fcc52b348e3017e56e8aa5a262f` |
| Version | `1.1.1` in Cargo, lock, and the VS Code package |
| `clc` in the tree | `clc 1.1.1` |

The pre-Phase-2 evidence was preserved as `build/perf/{base,head,flags}-plan26.json`
and `build/perf/plan27-baseline.txt`; it describes the PLAN-26 runs, not the
release. Phase 2 deletes and regenerates the live JSONs.

**v1.1.1 benchmark check:** a temporary worktree at `v1.1.1` built `clc`, and
that compiler emitted the current `benchmarks/lookup/program.clo` for
`hosted-amiga`. Compiled with the shipped flags, `.text` is 64,328 and the
program prints `lookup: 134217728`. The base can measure the current set.

**Current-version statements:** `README.md`, `RELEASE.md`, `CHANGELOG.md`,
`RELEASE-NOTES.md`, and `docs/known-limitations.md`; no test under
`compiler/tests`, `tools/vscode/test`, or `spec/` pins `1.1.1`.

**Environment:** `node --version` is `v22.23.3` (nvm). The repository is private on GitHub (`gh` authenticated as
`blendsdk`, ADMIN); Node 22.23.3 exists under
`~/.nvm/versions/node/v22.23.3/bin` but is not on the default `PATH`, so
`make vscode-check` needs that directory prepended.

## Release Scope

The 31 pre-plan commits since v1.1.1 classify as:

| Class | Commits | Notes |
|---|---|---|
| Product | `cef0b91`, `fd7fe30`, `b019771` | constant-array emission, collision-free template symbol, release GC+strip |
| Behavior-neutral compiler | `902ef30`, `cda97cc` | dead-code removal; per-profile release seams still on `-O2` |
| Tests and tooling | `a886a34`, `a589353`, `ac92308`, `ca30d36`, `4e6e1d1`, `866104c`, `e3fdbf3`, `29b85fb` | the `lookup` benchmark, vbcc probe retries, test pins, the flag audit |
| Records | the remaining 18 | PLAN-25/PLAN-26 plans, reviews, roadmap, publication notes |

No other user-visible feature is in the range. The closeout adds only the
release records.

## Measurements

The head and flag-audit numbers are from the gated commit `74d4380` on a
clean tree. The release-tree base column is from `2d1eb30` (the `base.json`
compiler), and the isolated PLAN-25 table is historical evidence from
`isolated-head.json` (`e2d3c76`) and `head-constant-array.json` (`82877ab`).

### Identities

| Document | Compiler | SHA-256 | Dirty |
|---|---|---|---|
| `build/perf/base.json` | `2d1eb3026fc6f75885785e6ff9601f99aaba4021` (`clc 1.1.1`) | `d58c693c…` | `false` |
| `build/perf/head.json` | `74d4380a540bf609f46f005c882dd4ee2c06f371` (`clc 1.1.2`) | `7b7ab88b…` | `false` |
| `build/perf/flags.json` | `74d4380…` (`clc 1.1.2`) | `7b7ab88b…` (matches head) | `false` |

### Release-tree comparison (v1.1.1 → v1.1.2, shipped flags)

| Benchmark | Base net ins | Head net ins | Δ | Base `.text` | Head `.text` | Δ |
|---|---:|---:|---:|---:|---:|---:|
| lookup | 13,653 | 9,470 | −30.6% | 64,328 | 39,904 | −38.0% |
| matrix | 9,908 | 9,908 | 0.0% | 23,608 | 23,608 | 0.0% |
| moves | 45,205 | 45,205 | 0.0% | 23,632 | 23,632 | 0.0% |
| sieve | 1,040,581 | 1,040,581 | 0.0% | 23,832 | 23,832 | 0.0% |
| strings | 2,964,051 | 2,964,051 | 0.0% | 24,196 | 24,196 | 0.0% |

Emitted C: `lookup` 474,430 → 340,503 bytes (−28.2%); the other four are
byte-identical. Host timings are advisory and were not compared. The
calibration raw count is 2,928 in both documents.

`make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"` passed:
lookup instructions −30.6% (need ≤ −20%), lookup `.text` −38.0% (need
≤ −20%).

### Isolated PLAN-25 evidence (old flags)

The original constant-array measurement, before the flag change, on the same
`lookup` program:

| Metric | Before | After | Δ |
|---|---:|---:|---:|
| net instructions (minus 3,372) | 13,653 | 9,470 | −30.6% |
| m68k `.text` | 117,440 | 92,976 | −20.8% |
| emitted C bytes | 474,430 | 340,503 | −28.2% |

Source: `build/perf/isolated-head.json` and
`plans/constant-array-initializer/05-findings.md`.

The release-tree row above is the same benchmark measured with the shipped
flags on both trees: `lookup` binary 82,544 → 41,740 (−49.4%), `.text`
64,328 → 39,904 (−38.0%). Do not substitute the flag-audit binary row
(116,584 → 41,740) for this comparison; that row is the same-compiler flag
effect.

**Calibration and net runtime:** calibration falls 3,372 → 2,928 on 2.0 and
3,348 → 2,947 on 1.3. Scored net instructions are unchanged by the flags
(`lookup` 9,470, matrix 9,908, moves 45,205, sieve 1,040,581, strings
2,964,051 in the flag audit); the constant-array change is what improves
`lookup` net by 30.6%.

### Same-compiler flag audit (old → GC → shipped)

| Program | old `.text` | shipped `.text` | Δ | old bin | shipped bin | Δ |
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

The five hosted-amiga scored benchmarks are `.text` 57.1–69.3% and binaries
64.2–74.7% smaller. The 1.3 audit set is `.text` 49.1–88.9% smaller. Host
binaries are 20.6–36.5% smaller. The freestanding image is unchanged. The
host `.symtab` check passed for every scored host binary.

### Constructor proof

| Profile | Load base | Init-list walk | `___initstdio` | `___initcpp` | `___initlibraries` | `___initmalloc` |
|---|---:|---:|---:|---:|---:|---:|
| hosted-amiga | 9,740 | 823 | 82 | 7 | 40 | 14 |
| hosted-amiga-13 | 9,740 | 823 | 80 | 7 | 25 | 14 |

The linker audit identifies the two removed libnix constructor bodies
(`___initamigapath`, `___fillenviron`); they account for the calibration
reduction. The trace proof records only the four remaining constructors, and
all four are hit during the run, before `main`.

## Gate Evidence

All gates ran on `74d4380` (C1) with a clean tree.

| Gate | Result |
|---|---|
| `make check` | passed (spec check, registry, setup-check, workspace, bench, fuzz; `cases=28 failures=0`) |
| `make asan` | 30 suites passed |
| `make ubsan` | 4 suites passed |
| `make corpus-amiga` | all portable programs match; build-only legs built |
| `make differential` | 3/3 match |
| `make bench-shape` | warning-free |
| `make bench-base COMMIT=v1.1.1` | base `2d1eb30…`, clean |
| `make bench` | head `74d4380…`, clean, `clc 1.1.2` |
| `make bench-compare` | passed; zero regressions; both lookup floors pass |
| `make bench-flags` | `runs: 2`; floors, constructors, repeat pass; clean |
| `make verify` | `verify: all legs passed` |
| `make vscode-check` | passed (Node 22.23.3 on `PATH`) |

`make verify` left no tracked changes; the RTG captures matched.

## Tag and Artifacts

_Pending Phase 3._ Gated commit, evidence commit, tag object, artifact names,
sizes, SHA-256 values, and the archive audit.

## Publication

_Pending Phase 3/4._ Draft asset verification, published URL, and final status.

## Review Rulings

### Phase 2

| Finding | Severity | Area | Ruling |
|---|---|---|---|
| RV-401 | MINOR | progress count | fixed — 19/40 at Phase 2 close; 24/40 after 3.1.5 |
| RV-402 | MINOR | batch timestamps | fixed — each gate time is the log completion time; 2.1.1–2.1.4 use the C1 commit minute |
| RV-403 | MAJOR | missing isolated-base evidence | fixed — added the isolated PLAN-25 table, the release-tree binary row, and the calibration/net-unchanged paragraph |
| RV-404 | MINOR | lifecycle status | fixed — plan, findings, and roadmap now say Executing |
| RV-405 | MINOR | constructor proof wording | fixed — the removed names are attributed to the linker audit; the trace proof records the four remaining constructors |

The reviewer found no critical issue. It independently re-emitted the four
unchanged benchmarks and confirmed the C bytes are identical.

### Pre-publication claim review

| Finding | Severity | Area | Ruling |
|---|---|---|---|
| CR-001 / CR2-001 | MAJOR | calibration missing from the public record | fixed — changelog, notes, and runbook gate row record 3,372 → 2,928 (2.0) and 3,348 → 2,947 (1.3) |
| CR-002 | MAJOR | findings provenance overstated | fixed — head/flag audit scoped to `74d4380`, base to `2d1eb30`, isolated table to `e2d3c76`/`82877ab` |
| CR-003 | MAJOR | runbook tag/draft sequence incomplete | fixed — tag step, HEAD assertion, and all three asset paths added |
| CR-004–CR-012 | MINOR | wording, counts, artifact table, v1.1.1 history, §14 caveats, status, Node, roadmap term | fixed |
| CR2-004 | MINOR | hello `clc` command was not explicit | fixed — exact `clc`/`cc`/run and `clorraine` commands |
| CR2-005 | MINOR | host `.text` range mixed calibration | fixed — scored host range, calibration 80.2% called out |

No false public numeric claim or gate claim was found. The review identified
missing or overbroad record text, all corrected before C2.

## Accepted Limits

| Limit | Evidence / workaround |
|---|---|
| Release programs are stripped | Programs built by `clorraine build --release` on Linux/Amiga lose function names; the `clc` and `clorraine` binaries keep symbols. Use a debug build for diagnosis. |
| Freestanding and non-Linux programs keep symbols | Freestanding and non-Linux host builds keep the `-O2` line; section GC grew the 208-byte freestanding image to 232. |
| The LSP is not in the binary tarball | Build `clorraine-lsp` from source; the tarball includes the notices and limitations docs. |
| The source tarball is the tag snapshot | Its runbook records the pre-publication status; the publication record is in the repository and release page. |
| `make bench-flags` is a release gate | It is not in the host CI or `make verify`; run it at release/closeout. |
| Host timing is advisory | `bench-compare` enforces output, net instructions, and `.text`; host wall time is printed but not compared. |
| The repository is private | "Published" means a non-draft GitHub release visible to repository users, not a public internet release. |
| Pre-existing freestanding spec-row wording | The freestanding release row says "same line with `-O2` in place of `-O0 -g`"; the driver omits debug warnings. Only the two GCC rows were authorized to change. |
