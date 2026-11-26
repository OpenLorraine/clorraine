# WS6 — Standard library

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS6 (WS6-01..WS6-07)

## Scope and limits

The 43 embedded modules under `stdlib/` (about 490 public functions and 64 handle
types), against `spec/appendix-stdlib.md` and `spec/appendix-amiga.md`. Method: scripted
API, gating, handle, and embedding checks (probes 600, 640, 650, 670) reviewed by hand;
core-module boundary probes on host (debug, release, ASan, UBSan) and on both Amiga
profiles under `vamos` with the leak detector; scaling probes on the A500 lens.
Behavior of the C shims behind the wrappers is WS5's; WS6 cites those findings where a
wrapper inherits them.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (6.1.1) | `probes/WS6/600`–`670`, `run.sh` | done |
| Red check (6.1.2) | seeded detectors, contrasts | done |
| Core modules (6.2.1) | `std.fmt`, `std.text`, `std.mem`, `std.math`, `std.iff`, `std.io` — probes 610–617 on host (debug, release, ASan, UBSan) and both Amiga profiles (`vamos`, leak detector) | done |
| API conformance (6.2.1) | all 43 modules — 600 (412 table rows), every report reviewed | done |
| Hidden costs (6.2.1) | 620 (7 workloads, A500) | done |
| Amiga modules: Kickstart (6.2.2) | 37 `std.amiga.*` modules — 640, every report reviewed | done |
| Amiga modules: handles (6.2.2) | 55 handle types in 30 modules — 650; drops of 9 handle kinds — 651 | done |
| Consistency, embedding (6.2.3) | 43 modules — 670, conventions reading | done |
| Self-refutation (6.3.1) | against WS5-02/WS5-03 and the DEF ledger | done |

## Checklist outcomes

Runs 2026-11-26 (`run.sh … main`; logs under `build/tmp/assessment/WS6/`; committed
records `records/WS6-results.json`, `WS6-contrast.json`, `WS6-runs.log`).

| ID | Outcome | Evidence |
|----|---------|----------|
| WS6-01 | ASM-WS6-002, ASM-WS6-004, ASM-WS6-005 | 600 over 412 appendix table rows: 28 reports, each reviewed — 22 are notation (`slice` documented as `u8[]` for `T[]`; `const` is the explicit spelling of the default read-only lend, spec/06 §3; `text(v)` is a compiler builtin, `compiler/src/sema/call.rs:1584`; `newStringGadget(window, id, geometry, initial)` abbreviates x/y/width/height; `func(...)` types and the `appendix-amiga.md` tables are outside the parser), 2 are cross-section table placement (`slice`, `bufferBytes` listed in §11), 4 are the Box rows' mode-after-colon spelling (ASM-WS6-005); no name, arity, or result type differs. 57 public functions unmatched by the parser; 25 of them appear nowhere in `spec/` (ASM-WS6-004). `slice` of a `str` is rejected (612, ASM-WS6-002) |
| WS6-02 | ASM-WS6-001, ASM-WS6-003 | Clean on host debug/release/ASan/UBSan and both Amiga profiles under `vamos` with the leak detector: 610 (`toHex`, `toDecimal` limits, padding below and above the length), 613 (`spanOf` with a wrapping end faults E11211), 614 (`sin`/`cos`/`sqrt` edges, wrap at `i32` limits), 615 (ten malformed and valid ILBM inputs: no trap, every result per §24, FNV-1a hashes match the definition), 616 (empty buffers, no UBSan report). 611: float text matches six significant digits; NaN differs by profile (ASM-WS6-003). 617: a read error is reported like end of file (ASM-WS6-001) |
| WS6-03 | `clean` | 620 on the A500 lens: every workload linear (ratio 1.98–2.03; seeded quadratic 4.00). Marginal costs: `push` 22.7 instructions/element, `padLeft`/`clone`/`stringOf` 10.1/byte, `bufferFrom`+hash 18.1/byte, `toDecimal` 318 per call, a two-placeholder template 711 per call (the formatting cost is ASM-WS5-022 / ASM-WS4-001) |
| WS6-04 | ASM-WS6-005 (and ASM-WS5-019 f) | 640: no module allowed on 1.3 opens a library above V34 (the two V37/V39 hits are `lib_Version` guards, `shim_hosted_amiga.c:1611`, `:4569`); no appendix 2.0+ module is on the 1.3 allowlist; 3.0/3.1 modules on `hosted-amiga` fail cleanly at run time (V39/V40 opens return null; `pool` checks `lib_Version >= 39`). Reviewed non-defects: bsdsocket (V4) and Picasso96 (V2) are stack/API versions, not Kickstart versions. Five "2.0" modules open V37 (2.04) — ASM-WS6-005 |
| WS6-05 | ASM-WS5-001, ASM-WS5-006 (cross-referenced) | Normal paths clean: 650 over 55 handle types — every deleter takes `take`, no wrapper releases a borrowed handle (two `review` lines are not releases: `volumeFreeBytes` reads free bytes, `releaseSemaphore` unlocks); 651 drops File, Buffer, String, Lock, PathInfo, MsgPort, Timer, TagList, Semaphore at scope end and closes Timer and Lock explicitly — "9 2", leak-free and error-free under `vamos`. Fault paths release nothing (ASM-WS5-001); borrowed OS objects outlive their owner (ASM-WS5-006) |
| WS6-06 | ASM-WS6-004 and SQ-WS6-2 | Conventions: camelCase throughout; acquisitions return `T?`, operations `bool`, counts `i32` with −1 on failure (the device, serial, socket, args, DOS modules) — except `std.io`, whose `fread`-based reads cannot report −1 (ASM-WS6-001); no wrapper returns `Status` although spec §4 prescribes it (SQ-WS6-2); release names follow the OS (`free*`, `close*`, `delete*`, `dispose*`, `unlock*`). Unranked: `cl_memcpy`/`cl_memcmp` are defined in both runtime shims and called nowhere |
| WS6-07 | `clean` | 670: 43 files, 43 embedded modules, every embedded name equals the file's `module` declaration, every 1.3-allowlist entry exists, every `ffi` function has a C body in the shims of every profile that allows its module; the seeded missing function is reported |

## Findings

### ASM-WS6-001 — `readFile` reports a read error exactly like end of file

- **Severity**: Major
- **Confidence**: verified
- **Lens**: host (reproduced); A500, A1200 shipped (the same `fread` code)
- **Location**: `stdlib/std.io.clo:46-54` (`readFile` returns `Read`'s count), `compiler/runtime/shim_host.c:645-648` and `compiler/runtime/shim_hosted_amiga.c:1259-1262` (`Read` is `fread`, which returns 0 on both end of file and error; `ferror` is never consulted); `writeBytes` / `Write` likewise report a failed write only as a short count, at e24a8752
- **Checklist item**: WS6-02 (also WS6-06)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/617-read-error.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --sanitize --only 617"`. Expected (`617-read-error.expected`: spec/appendix-stdlib.md §4 — partial IO returns the count moved plus a status; failures are not hidden; C99 7.19.8.1 — end of file and error are told apart only by `feof`/`ferror`): a failing read (a directory opened for reading, `EISDIR`) and a clean end of file report differently ("true true"). Observed on host debug, release, ASan, and UBSan: "true false" — both return 0; the contrast (two clean ends of file) prints "true false" as expected.
- **Impact**: a program cannot tell a failed read from the end of a file: a copier or loader stops early and reports success with truncated data (a bad floppy sector on the Amiga, a removed volume, a directory given as a file). The `std.io` doc comment groups the two ("zero or less means end of file or an error"), but no value is ever negative, so even that reading cannot be acted on. The `file-copier` corpus program follows exactly this loop.
- **Root cause**: the shim maps the AmigaDOS-style `Read` contract (−1 on error) onto `fread`, which has no error return.
- **Recommendation**: return −1 when `ferror` is set (and clear it), as the AmigaDOS `Read` contract does, and document the −1; or return the spec §4 count-plus-`Status` pair. Same for `Write`.
- **Effort**: S
- **Related**: SQ-WS6-2 (the §4 carrier); ASM-WS6-004 (`std.io` has no appendix table)
- **Self-refutation (6.3.1)**: no limitation or DEF row covers it; the doc comment acknowledges ambiguity but promises a "zero or less" signal that the code never produces for errors. Major (missing failure path).
- **Verdict**: confirmed (Major) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): probe 617 reproduces on all four host lines; the Amiga lens is the same `fread` path (code reading).

### ASM-WS6-002 — `slice` of a `str` is rejected, although the appendix gives it the same view type

- **Severity**: Major
- **Confidence**: verified
- **Lens**: all
- **Location**: `stdlib/std.mem.clo:40` (`intrinsic func slice<T>(v: T[], lo: usize, hi: usize): T[]` — `str` does not unify with `T[]`), the intrinsic's checking in `compiler/src/sema/call.rs:114`, `:341`, `:557`, at e24a8752
- **Checklist item**: WS6-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/612-cstring-of-slice.clo` (`let part: str = slice(full, 0, 36);`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --only 612"`. Expected (spec/appendix-stdlib.md §11: `slice(view, lo, hi)` — "view -> same view type", no allocation; spec/08 §3: the stdlib `slice` helper is a source of views; the expectation file pre-registers a rejection as this finding): the program compiles. Observed on all three profiles: `error E10200: cannot convert '?[]' to 'str' implicitly`.
- **Impact**: text code cannot take a substring view without copying (`stringOf` of bytes is not available either: `str` from `u8[]` has no safe conversion), so parsers either copy or work on `u8[]` and lose the `str` type. A valid program per the appendix is rejected.
- **Root cause**: `slice` is declared over `T[]` only; `str` is a distinct view type, and the intrinsic has no `str` overload.
- **Recommendation**: give `slice` a `str` form (or a `substr(s: str, lo, hi): str` helper) with the same bounds check; at the same time decide SQ-WS6-3 — `toCString` returns the view's pointer without a terminator at the view's end, which is sound today only because every `str` ends at a NUL.
- **Effort**: M
- **Related**: SQ-WS6-3 (`toCString` of a sub-view); ASM-WS1-003 (the earlier spec-vs-implementation gap class)
- **Self-refutation (6.3.1)**: not listed in `docs/known-limitations.md`; the appendix row is explicit. Major (a wrongly rejected valid program).
- **Verdict**: confirmed (Major) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): E10200 on all three profiles; no safe route to a `str` sub-view exists.

### ASM-WS6-003 — NaN prints as `-nan` on the host and `NaN` on the Amiga profiles

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host vs. A500/A1200 shipped
- **Location**: `cl_text_f64` / `cl_str_part_f64` through each C library's `%g` (`compiler/runtime/shim_host.c:540-548`, `compiler/runtime/shim_hosted_amiga.c:1158-1166`), at e24a8752
- **Checklist item**: WS6-02 (and WS5-11 parity)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/611-float-format.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --sanitize --only 611"`. Expected (`611-float-format.expected`: S2a six significant digits; one program prints one text on every profile): `nan inf -inf`. Observed: host (all four lines) `-nan inf -inf`; hosted-amiga and hosted-amiga-13 `NaN inf -inf`. The finite values (`0.1 2.5 1.23457e+08 1e+300 -0`) agree everywhere.
- **Impact**: a host test golden for a NaN result does not match the target; a NaN's sign leaks into text on the host only.
- **Root cause**: the template delegates to each libc's `%g`, whose NaN spelling and sign handling differ.
- **Recommendation**: format non-finite values in the shim (`nan`, `inf`, `-inf`) before calling `%g`; record the spelling in the spec (SQ-WS6-1).
- **Effort**: S
- **Related**: SQ-WS6-1
- **Self-refutation (6.3.1)**: the spec does not name a spelling, so the defect is the divergence, not either spelling. Minor.
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): `-nan` on host, `NaN` on both Amiga profiles; finite values agree.

### ASM-WS6-004 — 25 public functions, including `std.io`'s writers, have no specification entry

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `stdlib/` exports listed below, at e24a8752
- **Checklist item**: WS6-01, WS6-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — 600's undocumented list (`records/WS6-runs.log`) filtered by `grep -c "\b<name>\b" spec/*.md` = 0: `std.io.createFile`, `std.io.writeBytes`, `std.amiga.dos.delayTicks`, `std.amiga.timer.freeTimer`, `std.amiga.input.decodeJoystick`, `std.amiga.intuition.openScreenHires`, `std.amiga.intuition.openWorkbenchWindow`, `std.amiga.audio.sampleCycles`/`samplePeriod`/`sampleVolume`, `std.amiga.sprite.freeAnimation`/`spriteChannel`/`spriteHeight`/`spriteVisible`/`spriteX`/`spriteY`, `std.amiga.video.bitmapBytesPerRow`/`bitmapDepth`/`bitmapHeight`/`chipRead16`/`chipRead32`/`chipWrite16`/`chipWrite32`/`freeCopper`/`openDisplayAt`. `std.io` as a whole has no API table (its read side appears only in chapter examples).
- **Impact**: the appendix is the frozen API oracle; functions outside it have no stated ownership, nullability, or fault behavior to conform to (the `readFile` defect, ASM-WS6-001, sits in such a gap), and changes to them are not spec changes.
- **Root cause**: modules grew after the appendix tables were written.
- **Recommendation**: add a `std.io` section and the missing rows (a spec change, so it needs a user ruling); or mark the functions internal.
- **Effort**: M
- **Related**: ASM-WS6-001
- **Self-refutation (6.3.1)**: maintainability and documentation debt, no behavior defect by itself. Minor.
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): all 25 names absent from `spec/*.md`.

### ASM-WS6-005 — The appendix's Kickstart minimums and Box signatures drift from the code

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: A1200 shipped (the 2.0 modules); all (spelling)
- **Location**: `spec/appendix-amiga.md:236-250` ("2.0" for `std.amiga.utility`, `gadtools`, `asl`, `regions`, `commodities`) against their opens at V37 (`shim_hosted_amiga_optional.c`, `_optional_ui.c:38`, `:46`, `_optional_services.c:76`); `spec/appendix-stdlib.md:88-92` (`value: take T`) against `stdlib/std.mem.clo:75-91` (`take value: T`) and grammar `param = [ param_mode ] , identifier , ":" , value_type`, at e24a8752
- **Checklist item**: WS6-04, WS6-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — 640 (`above-min` for the five modules: V37 requested, appendix 2.0 = V36) and 600 (the four Box rows), `records/WS6-runs.log`.
- **Impact**: on a V36 Kickstart 2.0 (early A3000 ROMs) the five modules return null while the appendix says they work; the Box rows teach a parameter spelling the grammar rejects.
- **Root cause**: "2.0" is used for 2.04 (V37) in the table; the Box rows were written in a type-position mode style.
- **Recommendation**: state "2.04 (V37)" in the table; respell the Box rows (both are spec edits needing a ruling).
- **Effort**: S
- **Related**: ASM-WS5-019 f (V38/V44 tags behind V37 opens)
- **Self-refutation (6.3.1)**: V36 machines are rare; documentation precision only. Minor.
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): the appendix's own `asl … 2.0 (save mode 2.04)` row shows "2.0" means V36; the Box rows contradict grammar line 86.

## Spec questions

- **SQ-WS6-1** — The text of NaN and infinity in templates and `text()` is unspecified
  (S2a fixes only the precision). Which spelling, and is a NaN's sign shown? (ASM-WS6-003)
- **SQ-WS6-2** — spec/appendix-stdlib.md §4 says library wrappers return `Status` and
  partial IO returns a count plus a `Status`, but no wrapper does: the appendix's own
  tables give `bool`, `T?`, and −1 counts. Which is normative — the §4 convention or the
  tables? (ASM-WS6-001 depends on the answer for its fix shape.)
- **SQ-WS6-3** — If `slice` of a `str` becomes legal (ASM-WS6-002), `toCString(s)` of a
  sub-view must not return the view's pointer unterminated. Should `toCString` require a
  NUL-terminated origin (and reject sub-views), or copy?

## Falsifiability log

Red check 2026-11-26 (`run.sh … seeded`, `run.sh … contrast`; logs under
`build/tmp/assessment/WS6/`).

| Probe | Form | Result |
|-------|------|--------|
| 600 | seeded defect: an appendix copy with a renamed function, a changed return type, and a flipped mode | all three reported (`missing`, `returns`, `mode`) |
| 620 | seeded defect: an O(N²) workload | first attempt ratio 1.00 (GCC -O2 folded the loop into a closed form); fixed with a `volatile` sink in `f4f3cc5f`; then ratio 4.00, reported |
| 640 | seeded defect: `std.amiga.workbench` and `std.amiga.semaphore` allowed on 1.3 | first attempt fired `13-too-new` only (workbench is not in the appendix table); semaphore added in `f4f3cc5f`; both classes fire |
| 650 | seeded defect: `closeX(borrow x)` calling the deleter | `double-close` reported |
| 670 | seeded defect: an `ffi` function with no C body | `no-shim` reported |
| 610, 614 | contrast | discriminate |
| 611 | contrast (`2.5` → `3.5`) | discriminates on the changed value; the NaN spelling differs from the expectation on every profile (see Step 6.2) |
| 612 | contrast | not reached: `clc` rejects `slice` of a `str` (E10200), the pre-registered conformance outcome |
| 613, 615, 616 | contrast | first run: probe design errors (fixed arrays passed as slices, E10200; a forward variable reference, E10219); fixed in `b2c10b73`; rerun 24/24 discriminate |
| 651 | contrast (count change); release detection rests on the `vamos` detector shown in WS5 | discriminates |

## Self-refutation (6.3.1)

- **Against WS5-02/WS5-03**: 651 shows every normal-path drop releasing (no `vamos`
  orphan or open library across nine handle kinds, including the utility and timer
  bases), which agrees with WS5: the shim leaks are on fault paths (ASM-WS5-001) and in
  the lazily opened bases `vamos` does not model (graphics, intuition, layers, diskfont;
  ASM-WS5-013). No WS6 handle finding duplicates a WS5 finding; WS6-05 cites them.
- **DEF ledger**: no DEF row covers `readFile` error reporting, `str` slicing, or NaN
  text (`grep -n "readFile\|slice.*str\|NaN" plans/00-roadmap.md` finds only DEF-75, DEF-89, and DEF-98, which concern array slices and emission, not these behaviors).
- **Downgrades considered**: ASM-WS6-002 could be read as a missing feature; the
  appendix row is explicit, so it stays a conformance Major. ASM-WS6-001 rests on a host
  reproduction; the Amiga path is the same `fread` code (code reading), stated in Lens.
- **No Critical** in WS6.

## Phase 6 gate (6.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | five findings, every template field, IDs `ASM-WS6-001..005` |
| ST-2 | each verified finding names a committed probe (611, 612, 617) or a measurement record with its command |
| ST-3 | no `hypothesis` |
| ST-4 | no performance Major |
| ST-5 | probes committed in `e97cf9f4`; detector fixes `f4f3cc5f` and probe-design fixes `b2c10b73` before the main run; 617 in `b01d537e` before its run; falsifiability log complete |
| ST-6 | three spec questions; no spec file changed |
| ST-7 | drift check PASS |
| ST-8 | WS6-01..WS6-07 each have one outcome; every coverage-table row complete |
| ST-11 | no Critical |
| ST-12 | the scaling numbers cite 620's command and `records/WS6-runs.log` |
| ST-14 | no patched-C estimate in WS6 |
| ST-15 | no 68020 number |
| ST-16 | dispatch record `high` |

## Escalations

None yet.

## Probe index

Runner: `probes/WS6/run.sh <clc> <out> main|contrast|seeded`.

| Item | Probes |
|------|--------|
| WS6-01 API conformance | 600 (script), 612 |
| WS6-02 core boundaries | 610 (`std.fmt`), 611 (float text), 612 (`toCString`), 613 (`spanOf`), 614 (`std.math`), 615 (`std.iff`), 616 (`std.mem`) |
| WS6-03 hidden costs | 620 (scaling, A500) |
| WS6-04 Kickstart gating | 640 (script) |
| WS6-05 handle ownership | 650 (script), 651 (drops under `vamos`) |
| WS6-07 embedding and skew | 670 (script) |
