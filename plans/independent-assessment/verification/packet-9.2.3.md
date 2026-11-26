# Verification packet 9.2.3 — WS3, WS4

## Scope

- Whole components at the baseline `e24a8752b08827c3a5d7c77b6e2827dac708b629`; pre-existing defects are in scope.
- Finding IDs use the `ASM-WS<n>-<NNN>` scheme.
- Read-only: do not edit any tracked file; write scratch output only under `build/tmp/assessment/verify/`.
- Before each reproduction run the drift check: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment e24a8752b08827c3a5d7c77b6e2827dac708b629` (it must pass). Builds and runs go through the SDK container: `make -C docker exec CMD="…"` (no nested quotes in CMD). The pinned compiler is `build/tmp/assessment/baseline/clc` (release; `clc-debug` alongside).
- Judge each finding from its text, the committed probes, the code, and `spec/`. Do not rely on the workstream files' other sections; they are the author's reasoning.

## Checklist briefs

- `.opencode/agents/emission-integrity-auditor.md`
- `plans/independent-assessment/03-02-compiler-workstreams.md (§WS3, §WS4 checklists)`

## Protocol

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources with the drift check
   passing; for every `probable` Major or Critical it attempts a reproduction.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.
6. Escalates any verdict that results in a Critical, per §Escalation.

The verifier also re-checks at least two `clean` outcomes per workstream, preferring items on
the A500 lens and memory-safety checklist items (WS2, WS5-03, WS6-05), and records a verdict
for each. Findings raised by a verifier are
added to the workstream file and receive their verdict from a different verification session.
Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Severity

| Severity | Definition | Ref |
|----------|------------|-----|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` against the equivalent C reference on a shipped 68000 build (§Measurement rules) | AR #15, PF-009 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that has already caused a recorded DEF | AR #15, PF-020 |
| 🟢 Opportunity | A measured improvement with no defect behind it below the Major threshold, or any gap measured only on a 68020 probe build | AR #15, PF-009 |

A `hypothesis` is never above Minor. A `clc` crash or hang on any input is Critical; a compile
time or peak RSS above 10× the Phase 0 `bench-compile` value for the same input size is Major
(PF-020). For emitted C (WS3-11), a C-compiler crash on valid emitted C, or a C compile time
above 10× the Phase 0 C compile time of the nearest-size emitted program, is Major (PB-005).

### Confidence

`verified` is only this confidence label; it never means "passed Phase 9" (PF-002).

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline (drift check passing), or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; never ranked above Minor (AR #15) |

A finding is **surviving** when its Phase 9 verdict is `confirmed`, `downgraded`, or
`upgraded`. Only surviving findings reach the report and the DEF ledger (AR #13 as clarified
by PF-002).



## Output

Write `build/tmp/assessment/verify/9.2.3.md` and return its content: one row per finding `| ID | verdict | reason (one line) | what you re-ran and observed |`, one row per sampled clean outcome with a verdict (`confirmed` or `refuted` + reason), any duplicate you would merge, and any new finding you raise in the finding template (it will be verified by a different session).

## Findings to verify

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

### ASM-WS4-001 — Templates cost 33% more instructions than equivalent C on the A500

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (also A500 -Os +19.4%, A1200 shipped +51.3%, 68020 probe +71.5%)
- **Location**: template lowering (`cl_string_concat((ClStrPart[]){…}, n)` per template) in `compiler/src/codegen/`; `cl_string_concat` in `compiler/runtime/shim_hosted_amiga.c:1046-1094`, at e24a8752
- **Checklist item**: WS4-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — Phase 0 matrix (`records/WS0-matrix.json`, A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `strings` Clorraine 1,685,808 vs C reference 1,271,509 net instructions (+32.6%; raw 1,688,727 / 1,274,427; calibrations 2,919 / 2,918). Attribution (`400-attribute.py`, `records/WS4-attribution.log`): `cl_string_concat` 326,086, `cl_string_new` 32,032, and 36,045 extra `memcpy` instructions; formatting is identical on both sides. Patch 413 (`probes/WS4/413-strings-template-inline.diff`, `430-patch-measure.py`, `records/WS4-patches.log`) builds the same string inline: 1,688,727 → 1,286,737 (−401,990, −23.8% of net), output identical.
- **Impact**: every program that formats text in a loop (status lines, logs, UI labels) pays about a third more CPU on the 68000 than hand-written C for the string building.
- **Root cause**: each template fills a stack array of tagged `ClStrPart` structs and calls a generic builder that walks the parts twice (sizing, then writing) and copies literal pieces with `memcpy`, instead of emitting the fixed sequence of copies and formats the template's shape already determines.
- **Recommendation**: specialize templates with a fixed shape at lowering time (reserve the sum of the part maxima, copy literals with known lengths, format scalars in place), keeping `cl_string_concat` for the general case.
- **Effort**: M
- **Related**: WS4-09 template specialization; WS6-03 (hidden costs)

### ASM-WS4-002 — `Array<T>` push and get cost 39% more instructions than equivalent C on the A500

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (also A500 -Os +57.8%, A1200 shipped +38.8%, 68020 probe +64.9%)
- **Location**: `stdlib/std.mem.clo` `push`/`get` (lines 110-132 per the emission `#line`s) lowered to `cl_buffer_reserve` per push and a one-element view per get; `compiler/runtime/shim_hosted_amiga.c:803-818` (`cl_buffer_reserve`), at e24a8752
- **Checklist item**: WS4-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — Phase 0 matrix (A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `container` Clorraine 57,198 vs C reference 41,312 net instructions (+38.5%; raw 60,117 / 44,230). Attribution: `cl_buffer_reserve` 8,465 instructions (called on every one of 1,024 pushes), `memset` 1,028 (zeroing each grown region), `main` +5,987 (the view and its second check per element). Patches: 412 (`push` with an inline capacity test and a direct store) 60,117 → 46,960 (−23.0% of net); 411 (`get(i)[0]` as one check and an indexed read) → 56,045 (−7.1%); outputs identical.
- **Impact**: container-heavy code (lists of game objects, parsed records) pays about 40% more CPU on the 68000 than the equivalent C growable array.
- **Root cause**: `push` always calls the runtime reserve (which returns at once when the capacity suffices) and then builds a one-element view checked twice; `get` returns a view that the caller indexes with a second check.
- **Recommendation**: make the `Array` element access compiler-known: inline the capacity test with the reserve call on the slow path only, store directly, and lower `get(i)[0]` to one bounds check and an indexed read.
- **Effort**: M
- **Related**: WS4-09; DEF history of `Array` ownership (WS8); `plans/real-workload-audit/99-execution-plan.md` (concluded the `get`/`push` instances are inlined into `main` and left them unchanged; amortized reserve growth followed in `growable-array-growth`)

### ASM-WS4-003 — The runtime fault writer pulls 16.6 KB of `printf` and soft-float code into every checked program

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (every Amiga lens links the same code)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:544-549` (`cl_trap` uses `printf("error %s: %s\n", …)`), at e24a8752
- **Checklist item**: WS4-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `425-size.sh` (`records/WS4-size.log`, A500 -O2 without `-s`): the `sieve` binary retains `___vfprintf_total_size` 10,582 B, `___addxf3` 2,452 B, `___mulxf3` 1,124 B, `___cmpxf2` 776 B, `___xf_pack` 388 B, `___extenddfxf2` 288 B; `calibration` (no runtime check) retains none of them. Patch 417 (`probes/WS4/417-trap-no-printf.diff`) writes the same line with `fputs`/`fputc`: `.text` 23,772 → 7,120 B (−16,652 B, −70%), output identical, instructions +12.
- **Impact**: on a 512 KB A500 every program that indexes, divides, or converts a float (almost all) carries about 16 KB of formatted-output and long-double code it never uses outside a fault message; the same applies to the equivalent C written with `printf`.
- **Root cause**: the fault path formats with `printf`, which links libnix's full `vfprintf` including floating-point conversion.
- **Recommendation**: write the fault line with `fputs`/`fputc` (or `Write` to `Output()`), exactly as patch 417 does; check the other shim `printf`/`snprintf` users the same way.
- **Effort**: S
- **Related**: WS4-07; WS7-02 (A500 footprint)

### ASM-WS4-004 — A module-level table with a constant initializer occupies RAM twice

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2
- **Location**: module-value initializer lowering (zeroed `static` plus `static const __cl_ctmpl_<n>` and a start-up `memcpy` in `cl_init_<module>`), `compiler/src/codegen/program.rs`, at e24a8752
- **Checklist item**: WS4-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — patch 416 (`probes/WS4/416-lookup-static-init.diff`; `records/WS4-patches.log`, A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): emitting `lookup`'s 4,096-entry table as an initialized static: `.bss` 16,504 → 120 B (−16,384), `.text` −24 B, instructions −7, output identical.
- **Impact**: each mutable module table with a constant initializer costs its size twice (the template in the code hunk and the zeroed copy in RAM), which matters for sine tables, palettes, and level data on a 512 KB A500.
- **Root cause**: module `let` initializers are lowered as run-time stores from a template even when every element is constant.
- **Recommendation**: emit a constant-initialized module `let` as an initialized static and drop the template and the start-up copy.
- **Effort**: S
- **Related**: WS7-02; `plans/constant-array-initializer` (introduced the template for module values)

### ASM-WS4-005 — Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2
- **Location**: references from the embedded shim (`shim_hosted_amiga.c`) to `CreateDir`, `CurrentDir`, `DateStamp`, `Delay`, `DeleteFile`, `Examine`, `Info`, `Lock`, `ParentDir`, `Rename`, `UnLock`, `CreatePort`, at e24a8752
- **Checklist item**: WS4-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `425-size.sh` with a link map (`build/tmp/assessment/WS4/size/calibration.clo.map`): the `calibration` link includes `libamiga.a(createdir.o … unlock.o)` stubs (20–28 B each) and `CreatePort.o` (188 B) although the program's own retained `.text` is 412 B; Clorraine `calibration` `.text` 5,900 B vs C 5,124 B (+776 B, +15%).
- **Impact**: a fixed ~0.8 KB per program; small, but it is the whole fixed overhead over C.
- **Root cause**: the archive members are extracted for references in shim functions that `--gc-sections` later removes, and the stub objects stay linked.
- **Recommendation**: move rarely used shim functions to the optional suffixes (as already done for 2.0+ services), or reference the DOS calls through the profile-specific suffix only when the module is imported.
- **Effort**: M
- **Related**: ASM-WS4-003

### ASM-WS4-006 — `-O3` and `-flto` beat the shipped release flags on the A500

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 (hosted-amiga-13, nix13, -m68000) and the 68020 probe build (labelled)
- **Location**: `compiler/src/clorraine/toolchain.rs:94-109` (shipped `-O2`/`-Os` lines), at e24a8752
- **Checklist item**: WS4-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `420-flags.py` (`records/WS4-flags.log`). Flag set A (shipped): `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`. Flag set B: the same with `-O3`: net instructions container −18.3%, strings −25.1%, moves −98.5%, local-table −5.1%, others within ±1.4%, at +272 to +2,216 B `.text`. Flag set C: `-O2 -flto`: −280 to −512 B `.text` on every benchmark, container −7.3% instructions. `-fwrapv` removed: identical code (it costs nothing); `-fomit-frame-pointer`: identical (already implied).
- **Impact**: users get up to a quarter fewer instructions on string- and container-heavy code with `-O3`, or about 0.5 KB smaller binaries with LTO, without source changes.
- **Root cause**: the release line is fixed to `-O2`/`-Os` and does not offer `-O3` or LTO.
- **Recommendation**: evaluate an `-O3` release option (speed) and `-flto` for the size line; keep `-fwrapv` (free).
- **Effort**: S
- **Related**: WS7-03; `plans/performance-levers-closeout/00-ambiguity-register.md:146-156` (rejected `-O3` as "`.text` +14.9% on calibration; runtime equal; no win" and LTO+GC as "saves only 2–11% more than GC" with link-time risk)

## Clean outcomes to sample

From `findings/WS3-codegen.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS3-06 | `clean` | `331-mmio-width.sh` (-O2, hosted-amiga and hosted-amiga-13, `records/WS3-mmio.log`): one `move.w #-32256,14676118` ($DFF096), one `move.l #74560,14676096` ($DFF080, not split), the $BFE001 read and write as one `or.b #2,12574721` (one read cycle, then one write cycle, in order), and two separate `move.w (a0)` loads of $DFF006 (not merged). Probe 330 was a design error (uncalled function, removed by -O2), replaced by 331 |
| WS3-08 | `clean` | probes 350–358: inner extent of a 2-D array, `str` index, `Array.get`, `u8` division by zero, `i16` modulo by zero, `assert`, fixed-array field index, negative index, `slice` beyond the view — every fault code as specified on host, hosted-amiga, hosted-amiga-13 (debug and release, host ASan/UBSan) |

From `findings/WS4-code-quality.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS4-03 | `clean` | patch 410 removes every range-provable `E11211` check in `sieve`: output identical, instructions **+13.0%** (1,043,170 → 1,178,532) and `.text` −36 B — GCC schedules the guarded loop better; a range-fact pass gives no gain on this shape. The C references carry the same checks, so guards are not part of any gap above |
| WS4-04 | `clean` | patch 415 removes the dead null-checked release at the `strings` loop head: +12 instructions (noise); drop flags and cleanup ladders that survive `-O2` cost nothing measurable here |

