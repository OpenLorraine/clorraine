# WS3 — IR and C codegen correctness

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS3 (WS3-01..WS3-11)

## Scope and limits

Components: `ir/`, `codegen/`, the emitted C. Inputs: the Phase 0 emission (93 accepted
cells, `findings/WS0-emission-record.tsv`) and the WS3 probes. Lens: valid C99 on every
compile-matrix leg, no undefined behavior, 68000 correctness, single-width volatile access.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (3.1.1) | `probes/WS3/300`–`380` | done |
| Red check (3.1.2) | seeded files, detector, comparators, contrasts | done |
| Strict and vbcc builds (3.2.1) | 93 + 77 Phase 0 cells | done |
| UB (3.2.2) | 310, 311, WS1 039–043 under UBSan | done |
| 68000, MMIO, names (3.2.3) | 320, 331, 340–344; shim and stdlib code reading | done |
| Guards, determinism, ABI, shapes (3.2.4) | 350–358, 360, 370–372, 380 | done |
| Self-refutation (3.3.1) | against `docs/known-limitations.md` §7, the DEF ledger, spec/17 | done |

## Checklist outcomes

Main runs 2026-11-25 (logs under `build/tmp/assessment/WS3/`; committed records
`records/WS3-results.json`, `WS3-contrast.json`, `WS3-strict-summary.tsv`, `WS3-scripts.log`,
`WS3-mmio.log`).

| ID | Outcome | Evidence |
|----|---------|----------|
| WS3-01 | ASM-WS3-005 | `300-strict-build.sh` over all 93 accepted Phase 0 cells: **0 compile failures**; the 14 host cells are warning-free under `-pedantic -Wall -Wextra -Wconversion -Wsign-conversion -Wcast-align -Wshadow -Wstrict-prototypes`. The Amiga cells carry the same 5 shim warnings each (461 total): `-Wstrict-prototypes` for the NDK's `VOID (*)()` `is_Code` cast, `-Wpedantic` for `(APTR)cl_task_entry` (both forced by the AmigaOS API — noise), and `-Wsign-conversion` in the appended optional shims (`cl_lowlevel_type`, `cl_asl_path`, `cl_socket_connect_raw`; defined conversions — noise). No warning comes from code emitted for a Clorraine statement. The 230 shim warnings are reported at `stdlib/std.io.clo:1457+` — ASM-WS3-005 |
| WS3-02 | `clean` | `301-vbcc-scan.sh` over the 77 Phase 0 vbcc logs: 0 warnings or errors in the emitted unit or a `.clo` source; the seeded file is reported |
| WS3-03 | ASM-WS1-002, ASM-WS3-003 | Under the host UBSan line, the WS1 differential probes 039–043 (wrap at every width, MIN×−1, −MIN, shift masks, MIN/−1 and zero-divisor faults) and 311 (signed left shift: emitted through unsigned forms) are clean; probe 310 shows signed overflow in a value-position `++` (ASM-WS3-003). Call-argument order: ASM-WS1-002; binary operands with two calls are emitted unsequenced (C99 6.5p3), part of ASM-WS1-002 |
| WS3-04 | `clean` | site list: no type-punning dereference in emitted code (aggregate copies use `memcpy`; `Array` element pointers come from `realloc`ed storage, which has no declared type); shims: `shim_hosted_amiga_optional.c:720` reads the `LONG *` that `ReadArgs` returns (API-mandated), the unions at `shim_host.c:354` and `shim_hosted_amiga.c:972` are tagged and read by the member written, colour tables are combined byte-wise (`shim_hosted_amiga.c:2767`, `:4362`) |
| WS3-05 | `limit` | the detector cannot fire: `vamos` (68000) reads a word at an odd address without an address error (`320-odd-address.sh`, same output as `-C 68020`), so runtime detection needs FS-UAE on the A500. Code reading found no odd-address source: struct fields keep natural alignment (WS1-06 probe 048 matches m68k GCC), heap storage comes from `malloc`/`realloc`/`AllocMem`, IFF fields are read byte-wise (`stdlib/std.iff.clo:36-45`), and `str`/`u8[]` views are byte-accessed. Removed by an FS-UAE A500 run of a probe that indexes `u16`/`u32` data at computed offsets |
| WS3-06 | `clean` | `331-mmio-width.sh` (-O2, hosted-amiga and hosted-amiga-13, `records/WS3-mmio.log` — the first record was empty because `poke()` is inlined into `main`; the script now prints `main` and the record was regenerated in Phase 9; verifier ses_dea9a6112ffeMR984F2rpPTplg confirmed the outcome): one `move.w #-32256,14676118` ($DFF096), one `move.l #74560,14676096` ($DFF080, not split), the $BFE001 read and write as one `or.b #2,12574721` (one read cycle, then one write cycle, in order), and two separate `move.w (a0)` loads of $DFF006 (not merged). Probe 330 was a design error (uncalled function, removed by -O2), replaced by 331 |
| WS3-07 | ASM-WS3-001, ASM-WS3-002 | probe 344 (300- and 1000-character identifiers) clean on three profiles; probe 343 was a design error (two user `ffi strdup` declarations, E11101) |
| WS3-08 | `clean` | probes 350–358: inner extent of a 2-D array, `str` index, `Array.get`, `u8` division by zero, `i16` modulo by zero, `assert`, fixed-array field index, negative index, `slice` beyond the view — every fault code as specified on host, hosted-amiga, hosted-amiga-13 (debug and release, host ASan/UBSan) |
| WS3-09 | ASM-WS3-006 | Phase 0: 93/93 cells byte-identical across two emissions, and debug `clc` = release `clc`; `360-determinism.sh`: repeat and debug identical, no date/time strings |
| WS3-10 | ASM-WS3-004 | probes 370 (64-byte struct by value, returned and passed), 372 (function pointers, capture-less lambdas, a capturing closure through a callable bound) clean on three profiles; varargs: none exist (spec/06 FN-3, F3) |
| WS3-11 | ASM-WS3-007 | `380-shapes.py`: 3,000-deep parentheses, a 1,000-term sum, 20,000 statements, 8,000 locals, and a 3,000-case `switch` compile with host cc and m68k GCC within limits (for example `switch-3000` 3.6 s on m68k GCC, limit 46.8 s); vbcc exceeds its limit on the `switch` (ASM-WS3-007) and marginally on the 50,743-line `statements-20000` unit (14.7 s vs 12.2 s from a 14,114-line baseline — proportional to size, not reported). `functions-3000` was a generator error (`f32` is a keyword) |

## Findings

### ASM-WS3-001 — Locals named like C keywords or C/NDK macros make the emitted C invalid

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: all
- **Location**: local naming in `compiler/src/codegen/function.rs` (locals keep their source spelling; spec/17 §7) at e24a8752
- **Checklist item**: WS3-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probes `probes/WS3/340-c-keywords-as-names.clo` and `341-header-macro-names.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 340 341"`. Expected (spec/01 §2-§3: `int`, `char`, `long`, `register`, `TRUE`, `NULL`, `EOF`, `stdout`, `errno` are legal Clorraine identifiers; spec/17 §7: "Local: source name if it is a valid, unique C identifier; otherwise a stable suffix"; §1: one valid C99 unit): both programs build and print 121 and 120. Observed on host, hosted-amiga, and hosted-amiga-13, every build line: the C compiler rejects the unit. **Corrected in task 8.2.1** (re-reading the emitted C of 340): the C89 keywords and `inline` are escaped (`i32 cl_int = 0;`, `cl_char`, `cl_long`, `cl_short`, … `cl_goto`, `cl_inline`, `cl_union`); the unescaped names are the C99 keywords `restrict` and `_Bool` (`i32 restrict = 0;`, `i32 _Bool = 0;` → `expected identifier or '(' before '=' token`, `expected expression before 'restrict'`) and, in 341, the header and NDK macros `NULL`, `EOF`, `errno`, `stdout`, `TRUE`, `FALSE`, `INT_MAX`, `CHAR_BIT`, `EXIT_SUCCESS`, `MEMF_CHIP`, `BYTE`, `APTR`, `ULONG` (`lvalue required as left operand of assignment` and related errors). The diagnostics are reported at `stdlib/std.io.clo` lines (ASM-WS3-005).
- **Impact**: an accepted program emits C that no compile-matrix leg accepts. Natural C89-keyword names (`char`, `int`, `long`, `short`) are already escaped (a test pins `int`: `codegen_ownership_impl_test.rs::a_borrow_element_may_spell_a_c_keyword`); the failing names are `restrict`, `_Bool`, and macro-like names such as `EOF`, `TRUE`, `NULL`, `errno`, `stdout` — plausible in a parser or a port of C code. The failure is closed (no wrong binary runs) but the program cannot be built. (Impact narrowed in task 8.2.1.)
- **Root cause**: the emitter's escape list covers the C89 keywords (and `inline`) but not the C99 keywords `restrict` and `_Bool`, nor the macro and global names the included headers and the shim define.
- **Recommendation**: escape locals (and parameters) whose spelling is a C99 keyword, a reserved name, or a name defined by the included headers or the shim (a fixed deny-list per profile), using the existing stable-suffix scheme; add run goldens for `char`/`int`/`long` locals and for `NULL`/`EOF`/`errno`.
- **Effort**: S
- **Related**: ASM-WS3-002; spec/17 §7; DEF-112 and DEF-138 (collisions with compiler-generated locals, fixed)
- **Self-refutation (3.3.1)**: `docs/known-limitations.md` §7 covers function and global names against the shim and generated helpers, not locals against C keywords or header macros; no DEF row covers it. Stays Critical (accepted program, invalid C on every leg; plausible names).
- **Verdict**: confirmed (Critical, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): `restrict`, `_Bool`, and header/NDK macros reach C unescaped; `is_c_keyword` (`compiler/src/codegen.rs:1204-1213`) lists the C89 keywords plus `inline` and `bool` only.

### ASM-WS3-002 — A module function can collide with a generic instance symbol

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: all
- **Location**: symbol mangling in `compiler/src/codegen/program.rs` and the generic key scheme of spec/17 §7, at e24a8752
- **Checklist item**: WS3-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe folder `probes/WS3/342-generic-key-collision/` (module `get` exports `_Point`; module `Probe.Main` instantiates `get<Point>`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 342"`. Expected (spec/17 §7: exported function `cl_<module>_<name>`, generic instance `cl_<name>__<typekey>`; "a user identifier cannot collide with a generated key"): builds and prints `1 102`. Observed on all three profiles: `error: conflicting types for 'cl_get__Point'; have 'ClPoint(ClPoint)'`.
- **Impact**: an accepted program emits invalid C when a module's lowercase path plus an underscore-led function name spells another generic's instance symbol. The trigger needs an unusual spelling (a module named like a generic function and a function name starting with `_`), so it is rare in practice.
- **Root cause**: the two mangling schemes share the `cl_` prefix and the separator, so `cl_` + `get` + `_` + `_Point` equals `cl_` + `get` + `__` + `Point`; the key escaping protects the type key, not the module/name boundary.
- **Recommendation**: give generic instances a distinct prefix (for example `clg_`) or escape a function name that begins with `_`; add a collision golden.
- **Effort**: S
- **Related**: ASM-WS3-001; `docs/known-limitations.md` §7 (the user-symbol collision check renames functions that collide with shim names and with `cl_init_*`, `cl_zero_ClSlice_*`, `cl_lambda_*` helpers, but generic instance symbols `cl_<name>__<key>` are not in that list); DEF-140 (a different generic-key collision, fixed)
- **Self-refutation (3.3.1)**: not documented and not covered by the existing rename mechanism. Rated Critical by the rubric (invalid C), with the rarity stated in Impact; Phase 9 may weigh likelihood.
- **Verdict**: confirmed (Critical, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): probe 342 fails on all three profiles; the type-side analogue (`struct _i32` next to `Pair<i32>`) is escaped, so only function names collide.

### ASM-WS3-003 — A value-position `++`/`--` emits signed C increment, undefined on the host and vbcc legs at the limit

- **Severity**: Major
- **Confidence**: verified
- **Lens**: host; vbcc leg (both build without `-fwrapv`); the m68k GCC legs are defined by `-fwrapv`
- **Location**: value-position increment lowering (emitted as `++(y)` / `(i)++`) in `compiler/src/codegen/expr.rs`, at e24a8752
- **Checklist item**: WS3-03
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS3/310-increment-wrap.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 310"`. Expected (spec/02 §10 and spec/04 §5: every operator is defined and integer arithmetic wraps; spec/17 §14: "Signed overflow of +, -, *, unary - — wrapped through unsigned-based forms or -fwrapv"): prints the wrapped values, no UBSan report. Observed: the debug and release lines print the expected line on all profiles, but the host UBSan line reports `runtime error: signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'` at `return ++y;`. The emitted C for that line is `return ++(y);`; WS1 probe 021 shows `__cl_tmp0 = (i)++;` for `xs[i++]`. Statement-position increments and compound assignments are lowered safely (`y = (i32)((u32)y + (u32)1);`), as is unary minus (`(i32)(-(u32)x)`).
- **Impact**: a program that wraps an `i32` (or `isize`) through a value-position `++`/`--` relies on undefined behavior when built on the host profile or with vbcc; an optimizing compiler may assume the wrap never happens (loop bounds, comparisons).
- **Root cause**: the value-position increment is emitted as the C operator on the signed type instead of the unsigned-based form used everywhere else.
- **Recommendation**: emit value-position increments through the same unsigned form (`(tmp = y, y = (i32)((u32)y + 1u), tmp)` or a statement temporary); add a UBSan run golden.
- **Effort**: S
- **Related**: ASM-WS1-002 (the same expressions are where evaluation order is pinned)
- **Self-refutation (3.3.1)**: the m68k GCC legs pass `-fwrapv`, so shipped GCC binaries are defined; the defect is real for the host profile and the vbcc leg, which spec/17 §14 covers ("or -fwrapv" applies only where the flag is passed). Stays Major (latent UB in emitted C).
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): UBSan reports the overflow at `return ++y;`; the host line has no `-fwrapv`.

### ASM-WS3-004 — On the host profile `usize` is 32 bits, so a pointer round trip through `usize` crashes

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host only (the Amiga profiles are 32-bit and pass)
- **Location**: the emitted `typedef unsigned int usize;` / `typedef int isize;` and `ULONG` for the host profile (`compiler/src/codegen/program.rs`, host prelude), at e24a8752
- **Checklist item**: WS3-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS3/371-usize-pointer-roundtrip.clo` (`let a: usize = usize(&x); *raw i32(a) = 5;`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 371"`. Expected (spec/02 §2 "usize/isize: pointer-sized integer"; §7 and spec/04 §14 the raw-to-integer and integer-to-raw casts; spec/15 §2 host pointer size "host"): prints 5. Observed: hosted-amiga and hosted-amiga-13 print 5; on host the release and UBSan lines crash with SIGSEGV, ASan reports `DEADLYSIGNAL`, and the debug line fails to build (`cast to pointer from integer of different size [-Werror=int-to-pointer-cast]`).
- **Impact**: host-profile tests of `raw`/`usize` code (allocators, address arithmetic) crash or fail to build although the same code is correct on the target; the host profile exists for fast tests, so its divergence weakens them. No shipped lens is affected.
- **Root cause**: the host prelude maps `usize`/`isize` to the 32-bit `ULONG`/`LONG` model of the Amiga ABI (spec/17 §3 maps `usize` to `ULONG`) although spec/02 makes them pointer-sized and the host's pointers are 64 bits.
- **Recommendation**: on the host profile map `usize`/`isize` to `size_t`/`ptrdiff_t` (or `uintptr_t`/`intptr_t`), or record a spec ruling that the host profile models the 32-bit target and reject `usize(p)` there.
- **Effort**: M
- **Related**: spec/17 §3 (mapping table); WS1 probe 048 (layout) passed because no field is `usize`
- **Self-refutation (3.3.1)**: spec/17 §3 itself maps `usize` to `ULONG`, which reads as an Amiga-ABI statement; together with spec/02 §2 the host case is ambiguous enough that a spec ruling may settle it. Minor (host only).
- **Verdict**: confirmed (Minor, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3).

### ASM-WS3-005 — Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all (visible on the Amiga profiles, whose shim suffix follows the program)
- **Location**: `compiler/src/codegen.rs:815` and `compiler/src/codegen/cleanup.rs:330` (the only `#line` writers; no reset before the appended shim suffix or before hoisted local declarations), at e24a8752
- **Checklist item**: WS3-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — `300-strict-build.sh` (command in its header) reports the shim functions `cl_lowlevel_type`, `cl_asl_path`, `cl_socket_connect_raw` at `stdlib/std.io.clo:1457`, `:2268`, `:3539` (the file has 80 lines); probe 341's errors on the user's local declarations are reported at `stdlib/std.io.clo:34-42`. Expected (spec/17 §11: `#line` maps each emitted statement to its Clorraine source line; inserted code is attributed to the scope's closing line): runtime code is either unmapped (`#line` reset to the C file) or mapped to a real line.
- **Impact**: GCC and vbcc diagnostics in runtime code, and `m68k-amigaos-gdb` stepping through it, point at nonexistent `.clo` lines, which misleads a user investigating a build failure (as in ASM-WS3-001) or a crash in the runtime.
- **Root cause**: the emitter writes `#line` before Clorraine statements but never restores the C file's own line mapping before emitting the appended shim suffix or the hoisted local declarations.
- **Recommendation**: emit `#line <n> "<unit>.c"` (or the shim's own name) before each appended shim and before the declaration block of each function.
- **Effort**: S
- **Related**: ASM-WS3-001
- **Self-refutation (3.3.1)**: no limitation or DEF covers it; affects tooling, not program behavior. Minor.
- **Verdict**: confirmed (Minor, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3); correction: `stdlib/std.io.clo` has 75 lines, not 80.

### ASM-WS3-006 — The emitted C depends on how the source path is spelled and embeds absolute paths

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `#line` path spelling in `compiler/src/codegen.rs:806-815` (the logical path is the command-line spelling), at e24a8752
- **Checklist item**: WS3-09
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — `probes/WS3/360-determinism.sh`, command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS3/360-determinism.sh build/tmp/assessment/baseline/clc build/tmp/assessment/baseline/clc-debug build/tmp/assessment/WS3/determinism"`. Expected (spec/17 §11: "Same source, same profile, same flags produce byte-identical C … no absolute paths (only the logical source path, in #line)"): identical output however the same source is named. Observed: the repeat and debug-`clc` emissions are identical, but compiling the same file from its own directory changes 58, 27, and 224 lines (string-parser, container, boing), and passing an absolute path embeds it in every `#line` (58, 27, 224 occurrences of the repository root).
- **Impact**: builds driven with absolute paths or from different working directories are not reproducible across machines, and leak host directory names into the C and its debug info.
- **Root cause**: the logical path is the path exactly as given on the command line rather than a path relative to the root module's directory.
- **Recommendation**: normalize `#line` paths to the root module's directory (the project root used for `embed`); or obtain a spec ruling defining the logical path as the invocation spelling.
- **Effort**: S
- **Related**: spec/13 §3 (determinism)
- **Self-refutation (3.3.1)**: with one fixed invocation the output is byte-identical (Phase 0, 93/93), which is what the gates rely on; "logical source path" is undefined in the spec, so the finding is Minor and may become a spec question.
- **Verdict**: confirmed (Minor, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): an absolute invocation embeds the repository root 58/27/224 times.

### ASM-WS3-007 — vbcc compile time grows about cubically with `switch` size

- **Severity**: Major
- **Confidence**: verified
- **Lens**: vbcc leg of hosted-amiga (build-only validity leg)
- **Location**: emitted `switch` shape (one `case N: { … break; }` per arm) compiled by `vc +aos68k -O2`, at e24a8752
- **Checklist item**: WS3-11
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `380-shapes.py` (`switch-3000`: vbcc did not finish within 1,200 s; limit 12.2 s = 10 × the 14,114-line Phase 0 vbcc time, `findings/WS0-emission-record.tsv`) and the scaling series `build/tmp/assessment/WS3/main2.sh` (records `records/WS3-scripts.log`): vbcc compile-only of the hosted-amiga emission with a 250-, 500-, 1,000-case `switch`: 1,831 ms, 7,995 ms, 57,489 ms. m68k GCC compiles the 3,000-case unit in 3.6 s and host cc in 0.28 s. Lens: hosted-amiga, `vc +aos68k -c99 -O2 -cpu=68000`, container host, baseline e24a8752.
- **Impact**: a program with a large dispatch `switch` (a bytecode interpreter, a key or event table) cannot be built with the vbcc leg in reasonable time; the GCC legs are unaffected.
- **Root cause**: vbcc's optimizer is super-linear in the number of `switch` arms; the emitter gives it one large function.
- **Recommendation**: document the vbcc limit, or have the emitter split very large `switch` statements (for example a two-level dispatch), or compile very large functions with `-O1` on the vbcc leg.
- **Effort**: M
- **Related**: WS7-06 (compile-time scaling); `docs/known-limitations.md` lists vbcc limits for bsdsocket headers only
- **Self-refutation (3.3.1)**: the root cause is in vbcc, not in an invalid emission; but the compile-matrix bound of 03-01 §Severity is exceeded for a plausible program shape (1,000-case `switch` 57 s, limit 12 s). Stays Major on the vbcc leg; no shipped GCC binary is affected.
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): 1,000 cases take 57–158 s against the 12.2 s limit; `-O1` alone does not bring it under (57.4 s at 1,000), so the `-O1` recommendation is insufficient by itself.

### ASM-WS3-008 — An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C

- **Severity**: Major
- **Confidence**: verified
- **Lens**: vbcc leg of hosted-amiga (`vc +aos68k -c99 -O2 -cpu=68000`); the m68k GCC and host legs accept the same unit
- **Location**: `compiler/src/codegen/control.rs:421-442` (`emit_conditional` emits each false branch inside a new `} else {` block), at e24a8752
- **Checklist item**: WS3-11
- **Author session**: ses_dea9a6112ffeMR984F2rpPTplg (raised by the 9.2.3 verifier; recorded by the executing session)
- **Evidence**: Reproduction — `probes/WS3/verify-9.2.3/vbcc-nest.sh` (committed after the verifier's run), command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS3/verify-9.2.3/vbcc-nest.sh"`: clc accepts every N; vbcc compiles 100, 124–126 arms; at 127 arms vbcc ends with `Segmentation fault (core dumped)`; at 128, 500, and 1,000 it fails with `error 9 … too many (128) nested blocks`; m68k GCC compiles every N. The emitted C nests one block per arm. A control with 127 nested plain source blocks compiles (plain blocks are flattened). C99 §5.2.4.1 guarantees only 127 nesting levels of blocks; 127 arms reach depth 128.
- **Impact**: a program with a long `else if` dispatch (a command parser, an opcode or key table) cannot be built on the vbcc leg; it also rules out the `if`-chain workaround for ASM-WS3-007. Build-time failure, no wrong binary.
- **Root cause**: each `else` arm opens a new C block instead of `} else if (…) {` at the same depth.
- **Recommendation**: emit `} else if (cond) {` when the false branch is a single conditional with no hoisted declarations or cleanup; otherwise fall back to the existing `goto` form past a fixed depth (`control.rs:415`); add a vbcc build golden with a 200-arm chain.
- **Effort**: S
- **Related**: ASM-WS3-007; ASM-WS1-004
- **Verdict**: _(Phase 9: needs a verification session other than ses_dea9a6112ffeMR984F2rpPTplg)_

## Spec questions

None (added in Phase 9: the section was missing from the Phase 3 file).

## Falsifiability log

Red check 2026-11-25 after `06edeb98` (log `build/tmp/assessment/WS3/red.log`,
`contrast.log`; results `build/tmp/assessment/WS3/probes/contrast.json`).

| Probe | Form | Red-check result |
|-------|------|------------------|
| `300-strict-build.sh` | seeded defect `300-seeded.c` (old-style declaration, shadowing, narrowing, sign change) | 5 warnings recorded (`-Wstrict-prototypes` ×2, `-Wshadow`, `-Wconversion`, `-Wsign-conversion`) |
| `301-vbcc-scan.sh` | seeded defect `301-seeded.c` | vbcc warnings 85 ("assignment of different pointers") and 113 ("only 0 should be assigned to pointer") |
| `320-odd-address.sh` | seeded violation: a word read at an odd address on the 68000 | **detector cannot fire**: `vamos` (68000) printed `odd read 515` and exited 0, exactly like `-C 68020`; WS3-05's runtime part is a `limit` and rests on code reading (FS-UAE would be the remaining detector) |
| `380-shapes.py` | comparator with a seeded record (`380-seeded-record.tsv`, every baseline 1 ms) | every compile reported `FAIL` against the 10 ms limit |
| `360-determinism.sh` | comparator (mutated copy) | `cmp` reports a copy with one appended line as different |
| 310, 311, 340, 341, 342, 344, 370, 371, 372 | contrasting input: a changed operand or literal whose expected output differs | 311, 344, 370, 372 matched on host, hosted-amiga, hosted-amiga-13 (debug, release; host also ASan and UBSan). **310**: the host UBSan line reported `signed integer overflow: 2147483647 + 1` at `++y`; **340, 341, 342**: the emitted C does not compile on any profile; **371**: the host builds crash (SIGSEGV) and the host debug build fails (`int-to-pointer-cast`) while the Amiga profiles print the expected value. Each contrast still discriminates (its id or operand change is visible where it runs); the failures are evidence for WS3-03, WS3-07, WS3-10 confirmed by the main run |
| 350–358 | contrasting input: an in-range index or non-zero divisor; the fault probe's contrast must print | all printed the stated value on the three profiles |
| 330 | contrasting input: the second volatile read replaced by a copy (`w = v`); accepted, and `330-mmio-width.sh` must then show one 16-bit load instead of two | accepted on both Amiga profiles; the load count is read in task 3.2.3 |
| 343 | — | **probe design error**: both modules declare `ffi strdup`, which `clc` rejects as one C symbol declared twice (E11101) before the handle typedefs are reached; the handle-typedef question is answered by code reading in task 3.2.3 |

## Measurement and run records

Committed copies (ST-12): `records/WS3-results.json` (WS3 probes and the WS1 rerun under
UBSan), `records/WS3-contrast.json`, `records/WS3-strict-summary.tsv` (warning counts per
profile, origin, flag), `records/WS3-scripts.log` (determinism, shapes, vbcc `switch`
series, vbcc scan), `records/WS3-mmio.log` (the `main` assembly). All: pinned `clc`
sha256 `0c2fd66e…`, baseline `e24a8752`, SDK container (host GCC 12.2.0, m68k-amigaos-gcc
16.2.0b, vbcc from toolchain `e7ed7a73`).

## Phase 3 gate (3.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | seven findings, every template field, IDs `ASM-WS3-001..007` |
| ST-2 | each `verified` finding cites committed probes or scripts and the command |
| ST-3 | no `probable`/`hypothesis` finding above Minor |
| ST-4 | ASM-WS3-007 is a compile-time finding, not an overhead finding; it states the measurement, lens, compiler, and baseline — n/a for the ≥10% instruction rule |
| ST-5 | probes committed in `06edeb98`; corrections (331, the 380 timeout fix) committed before running; falsifiability log complete; the odd-address detector that cannot fire is a `limit` |
| ST-6 | findings cite spec sections; no `spec/` change |
| ST-7 | drift check PASS |
| ST-8 | WS3-01..WS3-11 each have one outcome; coverage table complete |
| ST-11 | ASM-WS3-001 and -002 escalated at 2026-11-25 21:35 with the message text |
| ST-12 | records above |
| ST-14, ST-15 | n/a |
| ST-16 | dispatch record `xhigh` |

## Escalations

| Time | ID | Message sent to the user |
|------|----|--------------------------|
| 2026-11-25 21:35 | ASM-WS3-001 | "Critical (verified): a local named like a C keyword or C/NDK macro (`char`, `int`, `long`, `NULL`, `EOF`, `errno`, `stdout`, …) makes the emitted C invalid on every profile. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --only 340 341\"`." |
| 2026-11-26 02:03 | ASM-WS3-001 (correction) | "Correction to the 21:35 message: `char`, `int`, and `long` are escaped (`cl_char`, `cl_int`, `cl_long`); the names that make the C invalid are the C99 keywords `restrict` and `_Bool` and header/NDK macros (`NULL`, `EOF`, `errno`, `stdout`, `TRUE`, …). Severity unchanged (Critical, verified)." |
| 2026-11-25 21:35 | ASM-WS3-002 | "Critical (verified): module `get`'s function `_Point` and the instance `get<Point>` both mangle to `cl_get__Point`, so the emitted C fails with conflicting types. Reproduce: the same command with `--only 342`." |

No fix is made (AR #12).

## Probe index

| Item | Probes |
|------|--------|
| WS3-01 strict builds | `300-strict-build.sh`, `300-seeded.c` |
| WS3-02 vbcc | `301-vbcc-scan.sh`, `301-seeded.c` |
| WS3-03 UB | 310, 311; WS1 probes 039–043 rerun under UBSan; ASM-WS1-002 |
| WS3-04 aliasing | code reading |
| WS3-05 68000 odd address | `320-odd-address.{c,sh}` |
| WS3-06 volatile/MMIO | 330 + `330-mmio-width.sh` |
| WS3-07 names | 340–344 |
| WS3-08 guards | 350–358 |
| WS3-09 determinism | `360-determinism.sh`; Phase 0 record |
| WS3-10 ABI | 370–372 |
| WS3-11 shapes | `380-shapes.py` (+ `380-seeded-record.tsv`) |
