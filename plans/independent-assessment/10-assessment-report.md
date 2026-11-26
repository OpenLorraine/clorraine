# Independent Assessment of Clorraine — Report

> **Document**: 10-assessment-report.md
> **Parent**: [Index](00-index.md)
> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629
> **Ledger**: [05-findings.md](05-findings.md)

## 1. Executive summary

**Verdict.** Clorraine's design holds where it matters most for an Amiga compiler: its
scalar code is as good as hand-written C on the 68000 (six of eight benchmarks within the
10% line or faster, `.text` within +1–8% of C), the ownership model keeps every host probe
clean under AddressSanitizer (UBSan reports only the value-position `++` overflow,
ASM-WS3-003), and the standard library's pure
modules survive malformed input. The baseline is **not release-grade for safety**: 58
findings survive independent verification — **12 Critical, 19 Major, 21 Minor, 6
Opportunity** — including two silent miscompiles of valid programs (call arguments run
right to left; temporaries release at function exit) and six AmigaOS runtime defects that
crash, hang, or corrupt a real machine (all reproduced on FS-UAE, the fault-exit crash on
both the A500/KS1.3 and the A1200/KS3.1).

**Top ten surviving findings** (§4 has all 58):

| # | ID | Severity | Finding |
|---|----|----------|---------|
| 1 | [ASM-WS5-001](findings/WS5-shims.md) | Critical | A runtime fault exits with OS registrations still installed: a VBlank server is left pointing into the unloaded program |
| 2 | [ASM-WS1-002](findings/WS1-semantic-core.md) | Critical | Call arguments with side effects are evaluated right to left |
| 3 | [ASM-WS2-001](findings/WS2-ownership.md) | Critical | Owned temporaries drop at the end of the enclosing scope, not at the end of their statement |
| 4 | [ASM-WS5-004](findings/WS5-shims.md) | Critical | `drawText` writes unclipped text through a screen `RastPort` that has no layer |
| 5 | [ASM-WS5-005](findings/WS5-shims.md) | Critical | A runtime fault inside a worker task runs libnix `printf` and `exit()` from a plain Exec task |
| 6 | [ASM-WS5-002](findings/WS5-shims.md) | Critical | `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine |
| 7 | [ASM-WS5-016](findings/WS5-shims.md) | Critical | The clipboard read never finishes the read protocol |
| 8 | [ASM-WS3-001](findings/WS3-codegen.md) | Critical | Locals named like C keywords or C/NDK macros make the emitted C invalid |
| 9 | [ASM-WS1-004](findings/WS1-semantic-core.md) | Critical | `clc` aborts with a stack overflow on moderately deep nesting |
| 10 | [ASM-WS1-005](findings/WS1-semantic-core.md) | Critical | Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305 |

**The optimizer question (WS4-10): targeted passes, not an optimizer stage.** GCC already
optimizes the emitted scalar code as well as the C references; the two measured gaps are
lowering shapes of runtime-library calls — the generic template builder (`strings` +32.6%
net instructions on the A500) and the `Array` API through runtime calls and one-element views
(`container` +38.5%). One targeted lowering closes most of each (−23.8% and about −30%,
patched-C probes 413, 411+412); an IR optimizer would duplicate GCC.

**Headline overhead against equivalent C** (net 68000 instructions; record
`findings/records/WS0-matrix.json`):

| Lens | container | local-table | lookup | matrix | moves | sieve | strings | text-scan |
|------|-----------|-------------|--------|--------|-------|-------|---------|-----------|
| A500 shipped -O2 | +38.5% | +4.0% | +1.5% | −8.9% | −16.5% | +0.0% | +32.6% | +7.1% |
| A1200 shipped | +38.8% | +4.5% | +1.5% | −9.1% | −16.5% | +0.0% | +51.3% | +8.5% |
| A1200 68020 probe build | +64.9% | +5.8% | +2.0% | −9.0% | +0.4% | +0.0% | +71.5% | +8.2% |

The fixed cost of a Clorraine program over C is small at start-up (2,919 vs. 2,918
instructions on the A500 calibration program) but large in size when a program can fault:
the runtime fault writer links `printf` and soft-float code, 16.6 KB of `.text`
(ASM-WS4-003).

## 2. Scope, method, and limits

**Baseline.** Commit `e24a8752b08827c3a5d7c77b6e2827dac708b629` (2026-11-25), pinned by the
drift check at every phase gate; the release `clc` (sha256 `0c2fd66e…`) built from it.
Tools (record `findings/WS0-baseline.md`): clc 1.2.0, rustc 1.99.0, host cc (Debian GCC
12.2.0), m68k-amigaos-gcc 16.2.0b, vbcc (SDK image), amitools/`vamos` 0.8.1.dev274, FS-UAE
3.1.66, the SDK image `sha256:3835c03b…`.

**Workstreams.** WS1 semantic core, WS2 ownership and drops, WS3 IR and C codegen, WS4 code
quality and the optimizer question, WS5 runtime shims, WS6 standard library, WS7 runtime and
compile-time performance, WS8 test integrity and defect history; WS0 is the baseline.
Every finding carries a reproduction (a committed probe whose expectation predates its
run) or a measurement record; code-reading findings are `probable` at most.

**Lenses.** Host (output check, ASan, UBSan); A500 shipped (`hosted-amiga-13`, 68000, -O2
and -Os); A1200 shipped (`hosted-amiga`, 68000); A1200 68020 probe build (`-m68020`,
`vamos -C 68020`, Opportunity-only by rule); FS-UAE A1200/KS3.1 and A500/KS1.3 for the
paths `vamos` cannot model.

**Effort per phase** (wall clock between phase gates; Phase 9 ran four verifiers in
parallel): Phase 0 0:42, Phase 1 0:35, Phase 2 0:13, Phase 3 0:59, Phase 4 0:47, Phase 5
0:33, Phase 6 1:36, Phase 7 1:27, Phase 8 0:07, Phase 9 1:15 (from the task completion marks in
`99-execution-plan.md`).

**Limits** (every `limit` outcome):
- WS3-05 — `vamos` reads odd addresses on the 68000 without an address error, so
  odd-address detection rests on code reading (removed by an A500 FS-UAE probe).
- WS5-05 — the 4 KB Kickstart 1.3 default stack was not probed (removed by an A500 deep-
  recursion run).
- WS7-01 — host wall time cannot rank the code: every benchmark runs in about 1–1.6 ms,
  below its own run-to-run spread (removed by host workloads of ≥100 ms).
- WS1-08 — the `embed` path-validation detectors cannot run because every `embed` form is
  rejected (ASM-WS1-003).
- WS5-13 — the verifier's open-count detector could not fire even on a seeded leak of ROM
  libraries, so ASM-WS5-013 stays `probable`.
- Probe 520 — the freestanding link check was not seeded with a libc reference.
- Removed during Phase 9: the A500/KS1.3 lens of WS5 (the verifier ran 541/542 and an
  alert probe on the A500), the public-port detector (a `SysBase->PortList` count), and the
  unshown machine crash of ASM-WS5-001 (reproduced on both machines).

**Phase 9.** Five fresh read-only verification sessions, one per workstream group,
re-ran every reproduction, tried to refute each finding against `spec/`, the known
limitations, and earlier plans, and sampled 15 `clean` outcomes (all confirmed; one
evidence record, the WS3-06 MMIO listing, was empty and was regenerated). Verdicts: 53
confirmed, 2 upgraded (ASM-WS2-001 and ASM-WS5-016 to Critical), 3 downgraded (ASM-WS5-003
Critical→Minor; ASM-WS5-011 and ASM-WS5-015 Major→Minor), none refuted; two findings were
raised by verifiers (ASM-WS3-008, ASM-WS5-023) and verified by a different session.

**Retracted escalations.** ASM-WS5-003 (escalated Critical on 2026-11-25) was downgraded to
Minor; the user was told. ASM-WS3-001's escalation text was corrected (the failing names
are `restrict`, `_Bool`, and header macros, not `char`/`int`/`long`).

**Independence.** The author session wrote every probe and finding; the verifiers were
separate sessions of the same model, given blind packets without the author's
self-refutation notes, and wrote only scratch output. They are independent of the author's
reasoning, not of the model; no human review has taken place yet.

## 3. Scorecard

Grades per 03-04: A — no Major or Critical; B — Majors only, each S or M; C — a Major with
effort L/XL or in a recurring WS8-07 class; D — exactly one Critical; E — two or more
Criticals or a systemic class.

| Area | Grade | Critical | Major | Minor | Opportunity | Justification |
|------|-------|----------|-------|-------|-------------|---------------|
| Semantic core (WS1) | **E** | 3 | 1 | 1 | 0 | argument order runs right to left; `clc` overflows its stack on moderate nesting and hangs on explicit polymorphic recursion |
| Ownership (WS2) | **D** | 1 | 0 | 2 | 0 | temporaries release at function exit; sanitizer-clean everywhere else |
| Codegen correctness (WS3) | **E** | 2 | 3 | 3 | 0 | unescaped C99 keywords and macros, a generic/function symbol collision; vbcc limits |
| Code quality and optimizer (WS4) | **B** | 0 | 2 | 0 | 4 | scalar code at C parity; two runtime-call lowering gaps, each M |
| Runtime shims (WS5) | **E** | 6 | 9 | 7 | 1 | six Criticals that crash, hang, or corrupt a real Amiga; a systemic absence of OS-resource tracking |
| Standard library (WS6) | **C** | 0 | 2 | 3 | 0 | a read error looks like end of file; `slice` of `str` rejected (false reject — a recurring class) |
| Runtime performance (WS7-01..05) | **B** | 0 | 1 | 0 | 1 | module-level fills unroll per element; footprint and chip placement otherwise clean |
| Compile performance (WS7-06..07) | **B** | 0 | 1 | 0 | 0 | superlinear compile time for call-heavy and generic code; RSS linear |
| Test integrity (WS8) | **A** | 0 | 0 | 4 | 0 | by the rubric (Minors only); the Minors are structural — the fuzzer misses the defect-heavy constructs and several gates assert weak properties |

ASM-WS0-001 (Minor; the flag-audit floor fails on `lookup`) is a baseline observation
outside the area grades. ASM-WS7-001 is counted under runtime performance (it also touches
WS7-06).

## 4. Findings by severity

Ranked Critical → Opportunity; within Critical and Major by impact (reach, confirmed
effect on a shipped lens, confidence), then Minor and Opportunity by ID. Every row is a
surviving ledger row of `05-findings.md`.

### Critical

**1. [ASM-WS5-001](findings/WS5-shims.md) — A runtime fault exits with OS registrations still installed: a VBlank server is left pointing into the unloaded program**  
Severity Critical · lens A500, A1200 shipped (shared code) · effort L · verified · verdict confirmed (Critical)

every program that faults (index out of range, division by zero, out of memory, `assert`, `panic`) after installing an OS registration leaves it behind. For an interrupt server (`addHandler`, `addPortHandler`, and the stdlib users of the dispatcher: input, timer, video) the next program loaded into that memory turns the server into a jump into arbitrary code at 50 Hz — a crash of the whole machine. The same exit skips: worker tasks (left on the ready list in unloaded code), the custom `View` and DMACON (display not restored), windows and screens (IDCMP ports signalling a dead task), the Workbench public-screen lock, public ports (in the system list with a name in freed memory), devices with pending I/O, sprite and audio channels, DOS locks, and every `AllocMem` block (AmigaOS does not reclaim them).

**2. [ASM-WS1-002](findings/WS1-semantic-core.md) — Call arguments with side effects are evaluated right to left**  
Severity Critical · lens all · effort M · verified · verdict confirmed (Critical)

any call whose arguments include two or more calls with side effects (I/O reads, allocators, counters, iterators, hardware reads) runs them in the opposite order to the source on every shipped profile; the result is a silent miscompile. Example: `copy(readByte(f), readByte(f))` swaps the two bytes.

**3. [ASM-WS2-001](findings/WS2-ownership.md) — Owned temporaries drop at the end of the enclosing scope, not at the end of their statement**  
Severity Critical (Phase 9: was Major) · lens all (host verified; the lowering is profile-independent) · effort L · verified · verdict upgraded Major→Critical

Owned temporaries are released at function exit (or when the same statement runs again), not at the end of their statement as spec/03 and spec/17 require. A `File` written through a temporary is still open and unflushed when the same function reopens it, so the read returns 0 bytes instead of 5 on host and hosted-amiga (probe 260). Any program that relies on a temporary's deleter running at statement end — closing, flushing, releasing a lock — behaves wrongly.

**4. [ASM-WS5-004](findings/WS5-shims.md) — `drawText` writes unclipped text through a screen `RastPort` that has no layer**  
Severity Critical · lens A500, A1200 shipped · effort S · probable · verdict confirmed (Critical; confidence probable→verified)

a safe program that draws text near the screen edge (a scroller, a status line with a long string, an off-by-one y) overwrites chip memory — other bitplanes, copper lists, or another program's data. Memory unsafety from the safe surface.

**5. [ASM-WS5-005](findings/WS5-shims.md) — A runtime fault inside a worker task runs libnix `printf` and `exit()` from a plain Exec task**  
Severity Critical · lens A500, A1200 shipped · effort M · probable · verdict confirmed (Critical; confidence probable→verified)

a worker that divides by zero crashes the machine instead of reporting a fault; the user cannot honor the "no DOS" rule because the compiler adds the DOS call.

**6. [ASM-WS5-002](findings/WS5-shims.md) — `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine**  
Severity Critical · lens A1200 shipped (`savePublicScreen`); A500 and A1200 (`showAlert`, code reading) · effort S · verified · verdict confirmed (Critical, verified)

any program that calls `savePublicScreen` (the Workbench capture API) or `showAlert` before opening a screen or window crashes the machine. `showAlert` with no window is the natural "fatal error" path of a CLI tool.

**7. [ASM-WS5-016](findings/WS5-shims.md) — The clipboard read never finishes the read protocol**  
Severity Critical (Phase 9: was Major) · lens A1200 shipped · effort S · probable · verdict upgraded Major→Critical (confidence probable→verified)

After `clipboardRead`, a `clipboardWrite` on the same handle never returns on an A1200 with Kickstart 3.1 — the read never finishes the clipboard read protocol, so the device holds writes off. A program that reads, then writes the clipboard hangs.

**8. [ASM-WS3-001](findings/WS3-codegen.md) — Locals named like C keywords or C/NDK macros make the emitted C invalid**  
Severity Critical · lens all · effort S · verified · verdict confirmed (Critical, verified)

an accepted program emits C that no compile-matrix leg accepts. Natural C89-keyword names (`char`, `int`, `long`, `short`) are already escaped (a test pins `int`: `codegen_ownership_impl_test.rs::a_borrow_element_may_spell_a_c_keyword`); the failing names are `restrict`, `_Bool`, and macro-like names such as `EOF`, `TRUE`, `NULL`, `errno`, `stdout` — plausible in a parser or a port of C code. The failure is closed (no wrong binary runs) but the program cannot be built. (Impact narrowed in task 8.2.1.)

**9. [ASM-WS1-004](findings/WS1-semantic-core.md) — `clc` aborts with a stack overflow on moderately deep nesting**  
Severity Critical · lens host (the compiler process; every profile is affected) · effort M · verified · verdict confirmed (Critical)

generated or table-like sources (long sums, deep `else if` dispatch, nested helper calls) crash the compiler with no diagnostic; the crash also takes down `clorraine-lsp`, which runs the same analysis on every keystroke.

**10. [ASM-WS1-005](findings/WS1-semantic-core.md) — Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305**  
Severity Critical · lens host (the compiler process; every profile is affected) · effort S · verified · verdict confirmed (Critical)

a small, plausible generic program (a recursive container walker that wraps its element type) hangs the compiler and the language server.

**11. [ASM-WS3-002](findings/WS3-codegen.md) — A module function can collide with a generic instance symbol**  
Severity Critical · lens all · effort S · verified · verdict confirmed (Critical, verified)

an accepted program emits invalid C when a module's lowercase path plus an underscore-led function name spells another generic's instance symbol. The trigger needs an unusual spelling (a module named like a generic function and a function name starting with `_`), so it is rare in practice.

**12. [ASM-WS5-006](findings/WS5-shims.md) — Safe handles let a borrowed OS object outlive its owner: clip regions and AmigaGuide messages**  
Severity Critical · lens A1200 shipped (`hosted-amiga` modules) · effort M · probable · verdict confirmed (Critical, probable)

a program the compiler accepts can make the layers library use a freed region on the next refresh, or reply a message into a closed context — use-after-free from the safe surface. The regions rule is documented, so the risk is a user mistake the type system does not catch.

### Major

**13. [ASM-WS6-001](findings/WS6-stdlib.md) — `readFile` reports a read error exactly like end of file**  
Severity Major · lens host (reproduced); A500, A1200 shipped (the same `fread` code) · effort S · verified · verdict confirmed (Major)

a program cannot tell a failed read from the end of a file: a copier or loader stops early and reports success with truncated data (a bad floppy sector on the Amiga, a removed volume, a directory given as a file). The `std.io` doc comment groups the two ("zero or less means end of file or an error"), but no value is ever negative, so even that reading cannot be acted on. The `file-copier` corpus program follows exactly this loop.

**14. [ASM-WS5-023](findings/WS5-shims.md) — `clipboardWrite` reports success for raw data that the 3.1 clipboard device discards, so `clipboardRead` never sees it**  
Severity Major · lens A1200 shipped (`hosted-amiga`, `std.amiga.clipboard`); other 2.0+ versions not tested · effort S · verified · verdict confirmed (Major, verified) with a corrected impact and root cause

`clipboardWrite` returns true while a raw (non-IFF) clip is either discarded (no clip yet) or stored padded to 8 bytes (after an IFF clip exists); `clipboardRead` of raw data returns 0 or the padded clip. The clipboard device keeps only IFF clips; the known-limitations entry attributes the empty read to the test environment.

**15. [ASM-WS7-001](findings/WS7-performance.md) — A module-level fill emits one assignment per element: a 64 KB table needs 548 KB of code and 163 s of GCC**  
Severity Major · lens A500 (measured); A1200 shipped and host (the same emission) · effort S · verified · verdict confirmed (Major, verified)

a demo or game that declares a module-level buffer with a fill (a 16 KB sample buffer, a 10 KB bitplane, a lookup table of a constant) gets about 8 bytes of 68000 code per element and runs that code at startup; at 64 KB the image no longer fits the A500, and the C build takes minutes. A zero fill pays the same cost although the static is already zero.

**16. [ASM-WS5-008](findings/WS5-shims.md) — The screen capture misreads the 2.0+ colour table, and the A1200 Workbench golden pins the wrong colours**  
Severity Major · lens A1200 shipped (every 2.0+ capture); A500 unaffected (`Type == 0` branch) · effort S · verified · verdict confirmed (Major, verified)

every native capture on 2.0+ records wrong colours; the `sdk-a1200-workbench` gate compares against a golden made by the same code, so it pins the defect instead of catching it.

**17. [ASM-WS5-012](findings/WS5-shims.md) — GadTools: the last gadget is never attached, rendered, or freed**  
Severity Major · lens A1200 shipped · effort S · probable · verdict confirmed (Major; evidence now verified)

every GadTools window shows one gadget fewer than the program created, and leaks it. The A600 gate counts capture colours only, so it does not catch this.

**18. [ASM-WS5-007](findings/WS5-shims.md) — `cardPresent()` and `cardStatus()` remove `card.resource` from the system**  
Severity Major · lens A1200 shipped (A600/A1200 PCMCIA) · effort S · verified · verdict confirmed (Major, verified)

after one call, `card.resource` is gone until reboot: the second call reports no PCMCIA, and every later `OpenResource("card.resource")` in the system fails (new users of the PCMCIA slot: carddisk, network and CF drivers being started). No crash was observed; existing users keep their pointer.

**19. [ASM-WS5-009](findings/WS5-shims.md) — The interrupt servers return with the user handler's condition codes and can end the VERTB chain**  
Severity Major · lens A500, A1200 shipped · effort S · verified · verdict confirmed (Major, verified)

while a Clorraine VBlank handler is installed, servers below priority 0 on the VERTB chain (and on PORTS, CIA-driven and expansion-card servers) are skipped on most frames. What runs below priority 0 depends on the system (third-party drivers, commodities); the A1200 runs in 541c did not show a visible failure.

**20. [ASM-WS1-001](findings/WS1-semantic-core.md) — An inferred-type decimal literal above `i32` range is accepted and silently wraps**  
Severity Major · lens all · effort S · verified · verdict confirmed (Major)

a program with an un-annotated large decimal constant (for example a 32-bit mask or address written in decimal) compiles and runs with a different value, or emits C that the strict build rejects; the spec requires a compile error.

**21. [ASM-WS3-003](findings/WS3-codegen.md) — A value-position `++`/`--` emits signed C increment, undefined on the host and vbcc legs at the limit**  
Severity Major · lens host; vbcc leg (both build without `-fwrapv`); the m68k GCC legs are defined by `-fwrapv` · effort S · verified · verdict confirmed (Major, verified)

a program that wraps an `i32` (or `isize`) through a value-position `++`/`--` relies on undefined behavior when built on the host profile or with vbcc; an optimizing compiler may assume the wrap never happens (loop bounds, comparisons).

**22. [ASM-WS6-002](findings/WS6-stdlib.md) — `slice` of a `str` is rejected, although the appendix gives it the same view type**  
Severity Major · lens all · effort M · verified · verdict confirmed (Major)

text code cannot take a substring view without copying (`stringOf` of bytes is not available either: `str` from `u8[]` has no safe conversion), so parsers either copy or work on `u8[]` and lose the `str` type. A valid program per the appendix is rejected.

**23. [ASM-WS5-013](findings/WS5-shims.md) — The shim's own library bases are never closed**  
Severity Major · lens A500, A1200 shipped · effort S · probable · verdict confirmed (Major, probable)

each run leaves an open count on graphics, intuition, layers, and diskfont (and the 2.0+ libraries a program used): disk-loaded `diskfont.library` and the optional libraries can never be expunged, so memory is lost until reboot; for `bsdsocket.library` the per-task base is never closed, so TCP/IP stacks keep the exited task's sockets and signal bindings.

**24. [ASM-WS5-017](findings/WS5-shims.md) — Layer drawing returns without `WaitBlit`, so CPU access to the same bitmap races the blitter**  
Severity Major · lens A500, A1200 shipped · effort S · probable · verdict confirmed (Major, probable)

a checksum or pixel read right after a layer fill can see partly drawn data; CPU writes can be overwritten by the finishing blit. Intermittent, timing-dependent wrong output.

**25. [ASM-WS5-018](findings/WS5-shims.md) — Classic menus are never laid out: every menu header and item sits at the same position**  
Severity Major · lens A500, A1200 shipped · effort S · probable · verdict confirmed (Major, probable)

a program with two or more menus, or two or more items in a menu, shows overlapping entries; only the first is reachable. The GUI gate drives menus with Amiga-key shortcuts, which do not depend on layout.

**26. [ASM-WS5-010](findings/WS5-shims.md) — The busy-pointer image has no sprite header or terminator, so sprite DMA reads past the 64-byte allocation**  
Severity Major · lens A500, A1200 shipped · effort S · probable · verdict confirmed (Major, probable)

`setPointer` shows a corrupted pointer (first row lost, trailing garbage lines from whatever follows in chip memory). Not visible to the bitplane-only capture of the GUI gate.

**27. [ASM-WS3-008](findings/WS3-codegen.md) — An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C**  
Severity Major · lens vbcc leg of hosted-amiga (`vc +aos68k -c99 -O2 -cpu=68000`); the m68k GCC and host legs accept the same unit · effort S · verified · verdict confirmed (Major, verified)

a program with a long `else if` dispatch (a command parser, an opcode or key table) cannot be built on the vbcc leg; it also rules out the `if`-chain workaround for ASM-WS3-007. Build-time failure, no wrong binary.

**28. [ASM-WS3-007](findings/WS3-codegen.md) — vbcc compile time grows about cubically with `switch` size**  
Severity Major · lens vbcc leg of hosted-amiga (build-only validity leg) · effort M · verified · verdict confirmed (Major, verified)

a program with a large dispatch `switch` (a bytecode interpreter, a key or event table) cannot be built with the vbcc leg in reasonable time; the GCC legs are unaffected.

**29. [ASM-WS7-002](findings/WS7-performance.md) — Compile time grows faster than quadratically for call-heavy and generic code**  
Severity Major · lens host (`clc` itself) · effort M · verified · verdict confirmed (Major; measurement verified, attribution probable and reproduced)

Compile time grows about quadratically on average (above quadratic on the top doubling) for stdlib-call-heavy and generic code: 8,000 such functions take 45–93 s against 0.4 s for 8,000 one-line functions. Name resolution in the ownership pass, instance de-duplication, and codegen's `resolve_function` fallback scan linear lists inside per-call and per-instance loops. Today's examples compile in milliseconds; large programs and the language server degrade first.

**30. [ASM-WS4-002](findings/WS4-code-quality.md) — `Array<T>` push and get cost 39% more instructions than equivalent C on the A500**  
Severity Major · lens A500 shipped -O2 (also A500 -Os +57.8%, A1200 shipped +38.8%, 68020 probe +64.9%) · effort M · verified · verdict confirmed (Major, verified)

container-heavy code (lists of game objects, parsed records) pays about 40% more CPU on the 68000 than the equivalent C growable array.

**31. [ASM-WS4-001](findings/WS4-code-quality.md) — Templates cost 33% more instructions than equivalent C on the A500**  
Severity Major · lens A500 shipped -O2 (also A500 -Os +19.4%, A1200 shipped +51.3%, 68020 probe +71.5%) · effort M · verified · verdict confirmed (Major, verified)

every program that formats text in a loop (status lines, logs, UI labels) pays about a third more CPU on the 68000 than hand-written C for the string building.

### Minor

**32. [ASM-WS0-001](findings/WS0-baseline.md) — The flag audit's host binary floor fails on `lookup` at the baseline**  
Severity Minor · lens host · effort S · verified · verdict confirmed (Minor)

`make bench-flags` exits non-zero on a clean tree, so the size/runtime floor gate is red for every run and cannot signal a real regression on the other rows; no shipped program is affected.

**33. [ASM-WS1-003](findings/WS1-semantic-core.md) — Data inclusion (`embed`) is staged out, but chapter 13 reads stable and the limitation is undocumented**  
Severity Minor · lens all · effort M · verified · verdict confirmed (Minor)

every program that bakes an asset with `embed` (the spec's own sprite, palette, and level examples) is rejected on every profile; the spec suite hides the gap by excluding its golden.

**34. [ASM-WS2-002](findings/WS2-ownership.md) — A field move out of a call result is accepted**  
Severity Minor · lens all · effort S · verified · verdict confirmed (Minor, verified)

a program the spec rejects compiles and runs memory-safely; portability of sources to a conforming compiler is the only cost.

**35. [ASM-WS2-003](findings/WS2-ownership.md) — Move, copy, and drop decisions are re-derived at about 240 sites in two stages with different predicates**  
Severity Minor · lens all · effort XL · probable · verdict confirmed (Minor, probable)

Move, copy, and drop decisions are made in the ownership pass and re-derived in codegen with a different predicate (`codegen/drop.rs:283` against `own.rs:675`), at about 230 sites in two stages. The split has already produced recorded defects (DEF-56/57/59/61); every new expression shape must be taught to both stages.

**36. [ASM-WS3-004](findings/WS3-codegen.md) — On the host profile `usize` is 32 bits, so a pointer round trip through `usize` crashes**  
Severity Minor · lens host only (the Amiga profiles are 32-bit and pass) · effort M · verified · verdict confirmed (Minor, verified)

host-profile tests of `raw`/`usize` code (allocators, address arithmetic) crash or fail to build although the same code is correct on the target; the host profile exists for fast tests, so its divergence weakens them. No shipped lens is affected.

**37. [ASM-WS3-005](findings/WS3-codegen.md) — Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files**  
Severity Minor · lens all (visible on the Amiga profiles, whose shim suffix follows the program) · effort S · verified · verdict confirmed (Minor, verified)

GCC and vbcc diagnostics in runtime code, and `m68k-amigaos-gdb` stepping through it, point at nonexistent `.clo` lines, which misleads a user investigating a build failure (as in ASM-WS3-001) or a crash in the runtime.

**38. [ASM-WS3-006](findings/WS3-codegen.md) — The emitted C depends on how the source path is spelled and embeds absolute paths**  
Severity Minor · lens all · effort S · verified · verdict confirmed (Minor, verified)

builds driven with absolute paths or from different working directories are not reproducible across machines, and leak host directory names into the C and its debug info.

**39. [ASM-WS5-003](findings/WS5-shims.md) — `setCurrentDir` unlocks the launcher's current-directory lock and never restores it**  
Severity Minor (Phase 9: was Critical) · lens A500, A1200 shipped · effort S · verified · verdict downgraded Critical→Minor

`setCurrentDir` unlocks the old current-directory lock. Phase 9 showed the locks stay balanced (the CLI's `CD` does the same, and libnix's startup duplicates the Workbench lock), so there is no double release; what remains is that the launching shell's directory changes after the program exits, and `vamos` reports an error at exit.

**40. [ASM-WS5-011](findings/WS5-shims.md) — `FileInfoBlock`s on the stack are not guaranteed longword-aligned**  
Severity Minor (Phase 9: was Major) · lens A500, A1200 shipped · effort S · probable · verdict downgraded Major→Minor

Three DOS helpers keep a `FileInfoBlock` on the stack, which the `Examine` autodoc requires to be longword-aligned. GCC keeps the frames at multiples of four and the block fills the frame, so the misalignment is not shown; the rule is followed by accident, not by construction.

**41. [ASM-WS5-014](findings/WS5-shims.md) — Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires**  
Severity Minor · lens A500 with expansion memory (not the stock 512 KB A500 lens, which has only chip RAM) · effort S · probable · verdict confirmed (Minor, probable)

on an A500 with trapdoor or fast RAM, disk reads land in the wrong memory and writes put garbage on the disk (data loss). The stock A500 lens is unaffected, so the severity comes from the shipped lens (Minor); for real 1.3 machines with memory expansion it is a data-loss defect.

**42. [ASM-WS5-015](findings/WS5-shims.md) — Commodity brokers are never activated**  
Severity Minor (Phase 9: was Major) · lens A1200 shipped · effort S · probable · verdict downgraded Major→Minor

Commodity brokers are created inactive and never activated, and queued messages are not drained at close. The module does not expose the input-event network, so no hotkey depends on it; the broker shows as inactive in Exchange.

**43. [ASM-WS5-019](findings/WS5-shims.md) — Lower-impact shim defects (grouped)**  
Severity Minor · lens as listed · effort M · probable · verdict confirmed (Minor)

individually limited (wrong layout or colours, a stuck layer on low memory, an ignored tag, a reply without a result code); none is shown to corrupt memory from the safe surface except *h* (DMA reads freed chip memory until the next `MrgCop`) and *o* (hypothesis).

**44. [ASM-WS5-020](findings/WS5-shims.md) — No Ctrl-C handling in any blocking shim wait**  
Severity Minor · lens A500, A1200 shipped · effort M · probable · verdict confirmed (Minor, probable)

a Clorraine program started from a shell cannot be interrupted with Ctrl-C while it waits for events, a device, a timer, or a worker; the user must close its window or reboot. Owned resources are released only if the program reaches its normal exit.

**45. [ASM-WS5-021](findings/WS5-shims.md) — Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove`**  
Severity Minor · lens host; freestanding (provisional profile) · effort S · probable · verdict confirmed (Minor)

latent; no current program is affected.

**46. [ASM-WS6-003](findings/WS6-stdlib.md) — NaN prints as `-nan` on the host and `NaN` on the Amiga profiles**  
Severity Minor · lens host vs. A500/A1200 shipped · effort S · verified · verdict confirmed (Minor)

a host test golden for a NaN result does not match the target; a NaN's sign leaks into text on the host only.

**47. [ASM-WS6-004](findings/WS6-stdlib.md) — 25 public functions, including `std.io`'s writers, have no specification entry**  
Severity Minor · lens all · effort M · verified · verdict confirmed (Minor)

the appendix is the frozen API oracle; functions outside it have no stated ownership, nullability, or fault behavior to conform to (the `readFile` defect, ASM-WS6-001, sits in such a gap), and changes to them are not spec changes.

**48. [ASM-WS6-005](findings/WS6-stdlib.md) — The appendix's Kickstart minimums and Box signatures drift from the code**  
Severity Minor · lens A1200 shipped (the 2.0 modules); all (spelling) · effort S · verified · verdict confirmed (Minor)

on a V36 Kickstart 2.0 (early A3000 ROMs) the five modules return null while the appendix says they work; the Box rows teach a parameter spelling the grammar rejects.

**49. [ASM-WS8-001](findings/WS8-test-integrity.md) — The fuzzer cannot generate the constructs where the defects are**  
Severity Minor · lens all · effort M · verified · verdict confirmed (Minor, verified)

the differential fuzzer guards arithmetic and layout, where few defects were found; the classes that recur (memory safety, invalid C, miscompile) are outside its reach.

**50. [ASM-WS8-002](findings/WS8-test-integrity.md) — Tests and gates assert weaker properties than the behavior they cover**  
Severity Minor · lens all; A500/A1200 gates · effort M · verified · verdict confirmed (Minor, verified) with a census correction

Several tests and gates assert a weaker property than the behavior they cover: drop-timing tests check leak-freedom, the fault gate checks the message but not the system state, the A1200 Workbench golden was captured by the defective capture code, and the A600 gate counts colours instead of gadgets. (About 32 spec tests check only a clean sanitizer run; most "sanitizer-only" tests also assert a computed exit code.)

**51. [ASM-WS8-003](findings/WS8-test-integrity.md) — Memory-safety and invalid-C defects recur because drop and emission decisions are made per expression shape, in two stages**  
Severity Minor · lens all · effort XL · verified · verdict confirmed (Minor; counts verified, structure probable), narrowed to causes (2)–(4)

Invalid C, route-specific false accepts, and unsequenced-effect miscompiles recur because emission is composed from strings per shape, semantic checks are attached to particular syntactic routes, and effects are left inside C expressions whose order C leaves unspecified (118, 100, and 51 DEF rows). The fourth cause, the second drop decision in codegen, is ASM-WS2-003.

**52. [ASM-WS8-004](findings/WS8-test-integrity.md) — The Amiga shims never run under a sanitizer, and UBSan covers a third of the ASan suites**  
Severity Minor · lens host (sanitizer legs); A500/A1200 (the unsanitized shims) · effort M · verified · verdict confirmed (Minor, verified)

the 22 WS5 findings were invisible to the existing checks; the host-leg UB of ASM-WS3-003 and ASM-WS5-021 sits in the UBSan gap.

### Opportunity

**53. [ASM-WS4-003](findings/WS4-code-quality.md) — The runtime fault writer pulls 16.6 KB of `printf` and soft-float code into every checked program**  
Severity Opportunity · lens A500 shipped -O2 (every Amiga lens links the same code) · effort S · verified · verdict confirmed (Opportunity, verified)

on a 512 KB A500 every program that indexes, divides, or converts a float (almost all) carries about 16 KB of formatted-output and long-double code it never uses outside a fault message; the same applies to the equivalent C written with `printf`.

**54. [ASM-WS4-004](findings/WS4-code-quality.md) — A module-level table with a constant initializer occupies RAM twice**  
Severity Opportunity · lens A500 shipped -O2 · effort S · verified · verdict confirmed (Opportunity, verified)

each mutable module table with a constant initializer costs its size twice (the template in the code hunk and the zeroed copy in RAM), which matters for sine tables, palettes, and level data on a 512 KB A500.

**55. [ASM-WS4-005](findings/WS4-code-quality.md) — Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use**  
Severity Opportunity · lens A500 shipped -O2 · effort M · verified · verdict confirmed (Opportunity, verified), magnitude corrected

a fixed ~0.8 KB per program; small, but it is the whole fixed overhead over C.

**56. [ASM-WS4-006](findings/WS4-code-quality.md) — `-O3` and `-flto` beat the shipped release flags on the A500**  
Severity Opportunity · lens A500 (hosted-amiga-13, nix13, -m68000) and the 68020 probe build (labelled) · effort S · verified · verdict confirmed (Opportunity, verified)

users get up to a quarter fewer instructions on string- and container-heavy code with `-O3`, or about 0.5 KB smaller binaries with LTO, without source changes.

**57. [ASM-WS5-022](findings/WS5-shims.md) — Integer formatting divides with libgcc's 32-bit routine on the 68000**  
Severity Opportunity · lens A500 · effort S · verified · verdict confirmed (Opportunity, verified)

every template placeholder and `toString` of an integer on the 68000 pays for `__udivsi3` per digit; this is most of ASM-WS4-001's `strings` gap.

**58. [ASM-WS7-003](findings/WS7-performance.md) — The 68020 probe build of `container` runs 21% more instructions than the 68000 build**  
Severity Opportunity · lens A1200 68020 probe build (`-m68020`, `vamos -C 68020`) — a 68020 probe build number, not a shipped-profile number · effort S · verified · verdict confirmed (Opportunity, verified)

none on a shipped profile; it would matter if a 68020 variant were added.

## 5. Root-cause themes

| Theme | Mechanism | Surviving findings it explains |
|-------|-----------|-------------------------------|
| T1 — No record of OS resources | The shims acquire interrupt servers, tasks, ports, screens, locks, and library bases without a registry, so a fault exit (`exit(20)`) and some normal exits release nothing; one fault path serves every execution context | ASM-WS5-001, ASM-WS5-005, ASM-WS5-013, ASM-WS5-020, ASM-WS5-006 |
| T2 — OS protocols assumed, not followed | Wrappers call AmigaOS APIs without the protocol their autodocs require (opening the library first, clipping, finishing a read, IFF clips, sprite data format, condition codes, layout, `WaitBlit`, resource ownership) | ASM-WS5-002, ASM-WS5-004, ASM-WS5-007, ASM-WS5-008, ASM-WS5-009, ASM-WS5-010, ASM-WS5-012, ASM-WS5-016, ASM-WS5-017, ASM-WS5-018, ASM-WS5-023, ASM-WS5-019 |
| T3 — Effects and lifetimes left to C | Effectful subexpressions stay inside C expressions whose order C leaves unspecified, and owned temporaries are bound to function-level slots instead of statement-scoped ones (WS8-08 cause 4) | ASM-WS1-002, ASM-WS2-001, ASM-WS3-003, ASM-WS8-003 |
| T4 — C composed from strings | The emitter builds C text per shape (712 `format!` sites); names, nesting depth, and line mapping are not guarded by a typed C representation (WS8-08 cause 2) | ASM-WS3-001, ASM-WS3-002, ASM-WS3-005, ASM-WS3-006, ASM-WS3-008, ASM-WS7-001, ASM-WS8-003 |
| T5 — Checks bound to one route | A rule is enforced on the route that was tested (inferred type arguments, an annotated literal, a local's field) and missed on the others (WS8-08 cause 3) | ASM-WS1-001, ASM-WS1-005, ASM-WS2-002, ASM-WS6-002, ASM-WS8-003 |
| T6 — Two drop authorities | Ownership decisions are re-derived in codegen with a different predicate (WS8-08 cause 1) | ASM-WS2-003 |
| T7 — Unbounded compiler work | Recursive descent on the main-thread stack and linear lookups inside per-call loops | ASM-WS1-004, ASM-WS7-002 |
| T8 — Tests that cannot see the defect | The fuzzer never generates calls, owned values, generics, or module data; gates assert weak properties; Amiga shims never run under a sanitizer | ASM-WS8-001, ASM-WS8-002, ASM-WS8-004, ASM-WS0-001 |

T1 and T2 together explain all six WS5 Criticals; T3–T5 are the three recurring DEF classes
(miscompile 51, invalid C 118, false accept 100 rows; `findings/records/WS8-defs.tsv`) and
are still producing Criticals at the baseline.

## 6. Performance

**Matrix** (§1 table; record `findings/records/WS0-matrix.json`, `measure.py`, every cell's
output checked). `.text` net of calibration is within +1.2% to +8.4% of C on every lens.
-Os on the A500 makes `container` worse (+57.8%) and `lookup` better (−30.3%).

**Attribution (WS4)** — `vamos -B` traces mapped through the link map
(`findings/records/WS4-attribution.log`): `strings`' extra cost is `cl_string_concat`'s
part loop and `__udivsi3` in `cl_format_u32` (patch 530: −40.9% of net, ASM-WS5-022);
`container`'s is `cl_buffer_reserve` per `push` and the one-element `get` view.

**Patched-C estimates** (`findings/records/WS4-patches.log`): inline template builder
−23.8%; direct `Array` push/get −23.0% and −7.1%; fault writer without `printf` −16,652 B
`.text`; constant tables as initialized statics −16,384 B RAM; range-check removal +13.0%
(slower), dead-release removal ±0.

**Flags (WS4-08; `findings/records/WS4-flags.log`)**: `-fwrapv` and
`-fomit-frame-pointer` change nothing; `-flto` saves 280–512 B; `-O3` cuts `strings` by
25.1% and `container` by 18.3% for +270–2,216 B (ASM-WS4-006).

**68020 probe build** (Opportunity only): `strings` −16.3% (hardware divide), most others
±5%, `container` +21.3% (ASM-WS7-003). A 68020 profile variant is not justified by these
numbers (§8, unranked).

**Footprint** (`findings/records/WS7-runs.log`): the 31 A500 programs use 5.9–56.6 KB
static, at most 10% of 512 KB — except a module-level fill, which costs about 8 bytes of
code per element (a 64 KB table needs 548 KB, ASM-WS7-001).

**Compile time** (`findings/records/WS7-runs.log`, probe 710; Phase 0
`findings/records/assessment-baseline-compile.json`): one-line functions scale near
linearly (exponent 0.9–1.2); locals 1.76–1.86, closures 1.66–1.91, stdlib-call-heavy
functions 2.2–2.9, generic instantiations 1.9–2.5; 8,000 call-heavy functions take 45–93 s
under load against 0.3–0.4 s for one-liners (ASM-WS7-002). Peak RSS grows linearly. vbcc's
own compile time grows about cubically with `switch` size (ASM-WS3-007).

## 7. Quick wins

Surviving findings with effort S and severity Minor or above:

| ID | Severity | Finding |
|----|----------|---------|
| [ASM-WS5-004](findings/WS5-shims.md) | Critical | `drawText` writes unclipped text through a screen `RastPort` that has no layer |
| [ASM-WS5-002](findings/WS5-shims.md) | Critical | `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine |
| [ASM-WS5-016](findings/WS5-shims.md) | Critical | The clipboard read never finishes the read protocol |
| [ASM-WS3-001](findings/WS3-codegen.md) | Critical | Locals named like C keywords or C/NDK macros make the emitted C invalid |
| [ASM-WS1-005](findings/WS1-semantic-core.md) | Critical | Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305 |
| [ASM-WS3-002](findings/WS3-codegen.md) | Critical | A module function can collide with a generic instance symbol |
| [ASM-WS6-001](findings/WS6-stdlib.md) | Major | `readFile` reports a read error exactly like end of file |
| [ASM-WS5-023](findings/WS5-shims.md) | Major | `clipboardWrite` reports success for raw data that the 3.1 clipboard device discards, so `clipboardRead` never sees it |
| [ASM-WS7-001](findings/WS7-performance.md) | Major | A module-level fill emits one assignment per element: a 64 KB table needs 548 KB of code and 163 s of GCC |
| [ASM-WS5-008](findings/WS5-shims.md) | Major | The screen capture misreads the 2.0+ colour table, and the A1200 Workbench golden pins the wrong colours |
| [ASM-WS5-012](findings/WS5-shims.md) | Major | GadTools: the last gadget is never attached, rendered, or freed |
| [ASM-WS5-007](findings/WS5-shims.md) | Major | `cardPresent()` and `cardStatus()` remove `card.resource` from the system |
| [ASM-WS5-009](findings/WS5-shims.md) | Major | The interrupt servers return with the user handler's condition codes and can end the VERTB chain |
| [ASM-WS1-001](findings/WS1-semantic-core.md) | Major | An inferred-type decimal literal above `i32` range is accepted and silently wraps |
| [ASM-WS3-003](findings/WS3-codegen.md) | Major | A value-position `++`/`--` emits signed C increment, undefined on the host and vbcc legs at the limit |
| [ASM-WS5-013](findings/WS5-shims.md) | Major | The shim's own library bases are never closed |
| [ASM-WS5-017](findings/WS5-shims.md) | Major | Layer drawing returns without `WaitBlit`, so CPU access to the same bitmap races the blitter |
| [ASM-WS5-018](findings/WS5-shims.md) | Major | Classic menus are never laid out: every menu header and item sits at the same position |
| [ASM-WS5-010](findings/WS5-shims.md) | Major | The busy-pointer image has no sprite header or terminator, so sprite DMA reads past the 64-byte allocation |
| [ASM-WS3-008](findings/WS3-codegen.md) | Major | An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C |
| [ASM-WS0-001](findings/WS0-baseline.md) | Minor | The flag audit's host binary floor fails on `lookup` at the baseline |
| [ASM-WS2-002](findings/WS2-ownership.md) | Minor | A field move out of a call result is accepted |
| [ASM-WS3-005](findings/WS3-codegen.md) | Minor | Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files |
| [ASM-WS3-006](findings/WS3-codegen.md) | Minor | The emitted C depends on how the source path is spelled and embeds absolute paths |
| [ASM-WS5-003](findings/WS5-shims.md) | Minor | `setCurrentDir` unlocks the launcher's current-directory lock and never restores it |
| [ASM-WS5-011](findings/WS5-shims.md) | Minor | `FileInfoBlock`s on the stack are not guaranteed longword-aligned |
| [ASM-WS5-014](findings/WS5-shims.md) | Minor | Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires |
| [ASM-WS5-015](findings/WS5-shims.md) | Minor | Commodity brokers are never activated |
| [ASM-WS5-021](findings/WS5-shims.md) | Minor | Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove` |
| [ASM-WS6-003](findings/WS6-stdlib.md) | Minor | NaN prints as `-nan` on the host and `NaN` on the Amiga profiles |
| [ASM-WS6-005](findings/WS6-stdlib.md) | Minor | The appendix's Kickstart minimums and Box signatures drift from the code |

Opportunities with at least 5% measured gain in net instructions or `.text`:

| ID | Gain | Finding |
|----|------|---------|
| [ASM-WS5-022](findings/WS5-shims.md) | `strings` −40.9% net instructions, A500 (patch 530) | integer formatting with a 16-bit divide |
| [ASM-WS4-003](findings/WS4-code-quality.md) | −16,652 B `.text` on `sieve` (patch 417) | a fault writer without `printf` |
| [ASM-WS4-006](findings/WS4-code-quality.md) | `-O3`: `strings` −25.1%, `container` −18.3% (record `WS4-flags.log`) | release flags |
| [ASM-WS4-005](findings/WS4-code-quality.md) | 440 B of the calibration program's 5,900 B `.text` (7.5%) | unused libamiga stubs and `CreatePort` |

## 8. Remediation roadmap

Order: plans that close Criticals first (R1–R5), then root-cause themes before their
symptoms (R6–R8), then the remaining defects (R9–R11), then performance (R12–R14). Each
entry is a `make-plan` input; its theme parent issue is linked in the appendix once filed.

| # | Plan slug | Goal | Closes | Depends on | Effort |
|---|-----------|------|--------|------------|--------|
| R1 | `amiga-fault-exit-cleanup` | Release every OS registration on every exit: a shim resource registry walked by the fault path and an exit hook; worker-task faults handled in the task | ASM-WS5-001, ASM-WS5-005, ASM-WS5-013, ASM-WS5-020 | — | L |
| R2 | `evaluation-order-and-temporaries` | Sequence effects in source order and release temporaries at statement end: lower effectful subexpressions into ordered temporaries (A-normal form) and give owned temporaries statement scope | ASM-WS1-002, ASM-WS2-001, ASM-WS3-003 | — | L |
| R3 | `c-name-integrity` | Escape every C99 keyword and header/NDK macro name and separate generic-instance symbols from module functions | ASM-WS3-001, ASM-WS3-002, ASM-WS3-005, ASM-WS3-006 | — | S |
| R4 | `front-end-limits-and-checks` | Bound `clc`'s recursion, report E10305 on every instantiation route, and range-check inferred literals | ASM-WS1-004, ASM-WS1-005, ASM-WS1-001, ASM-WS2-002, ASM-WS1-003 | — | M |
| R5 | `amiga-shim-crash-safety` | Fix the shim paths that crash or hang a real machine: open Intuition on demand, clip screen text, finish the clipboard read protocol, track borrowed clip regions and guide messages | ASM-WS5-002, ASM-WS5-004, ASM-WS5-016, ASM-WS5-006 | R1 (shares the registry) | M |
| R6 | `typed-c-emission` | Emit through a typed C representation that escapes names, bounds nesting, maps lines, and makes invalid shapes unrepresentable | ASM-WS8-003, ASM-WS3-008 | R3 | XL |
| R7 | `single-drop-authority` | Make the ownership pass the only place that decides moves and drops; codegen emits recorded drops only | ASM-WS2-003 | R2 | XL |
| R8 | `test-and-gate-integrity` | Extend the fuzzer to calls, owned values, generics, module data, and identifiers; make gates assert behavior (walker system state, independent goldens); keep the `vamos` leak detector and FS-UAE walker as standing gates | ASM-WS8-001, ASM-WS8-002, ASM-WS8-004, ASM-WS0-001 | — | M |
| R9 | `amiga-shim-protocol-fixes` | Follow the AmigaOS protocols the remaining shim wrappers skip | ASM-WS5-007, ASM-WS5-008, ASM-WS5-009, ASM-WS5-010, ASM-WS5-012, ASM-WS5-017, ASM-WS5-018, ASM-WS5-023, ASM-WS5-003, ASM-WS5-011, ASM-WS5-014, ASM-WS5-015, ASM-WS5-019, ASM-WS5-021 | R5 | M |
| R10 | `stdlib-conformance` | Report read errors, give `str` its `slice`, unify NaN text, and bring the appendix in line with the exports | ASM-WS6-001, ASM-WS6-002, ASM-WS6-003, ASM-WS6-004, ASM-WS6-005, ASM-WS3-004 | — | M |
| R11 | `vbcc-leg-limits` | Keep large dispatch code within vbcc's limits: flat `else if` emission and a bounded `switch` shape | ASM-WS3-008, ASM-WS3-007 | R6 (or standalone) | S |
| R12 | `initializer-and-runtime-size` | Lower module fills like local fills and constant tables as initialized statics; a fault writer without `printf`; drop unused libamiga stubs | ASM-WS7-001, ASM-WS4-004, ASM-WS4-003, ASM-WS4-005 | — | S |
| R13 | `runtime-call-lowering` | Close the two measured gaps: inline template builder, `Array` push/get lowering, 16-bit integer formatting; revisit release flags | ASM-WS4-001, ASM-WS4-002, ASM-WS5-022, ASM-WS4-006, ASM-WS7-003 | — | M |
| R14 | `compile-time-scaling` | Index names and instances once per module/program so compile time stays near linear | ASM-WS7-002 | — | M |

Every surviving finding is closed by exactly one entry except ASM-WS3-008, which R6
removes structurally and R11 can fix directly first.

### Unranked recommendations

- **Status carrier (WS6-06, SQ-WS6-2).** No library wrapper returns the spec's `Status`;
  settle whether §4 or the appendix tables are normative before R10 shapes `readFile`'s fix.
- **Dead runtime code (WS6-06).** `cl_memcpy` and `cl_memcmp` are defined in both runtime
  shims and called nowhere.
- **68020 profile variant (WS7-03).** Not justified by the probe-build gains; revisit only
  after R13 and with a cycle-level measurement.
- **Standing assessment gates.** The `vamos --vamos-leaks` detector and the FS-UAE
  system-state walker built for this assessment (`probes/WS5/54x-*`,
  `probes/WS5/verify-9.2.4/`) catch WS5's defect classes; make them part of `make verify`.
- **Host-time benchmarks.** The benchmark suite has no host workload long enough to rank
  code by wall time (WS7-01 limit).
- **A500 lens coverage.** An A500 FS-UAE odd-address probe (WS3-05) and a deep-recursion
  stack probe on the 4 KB Kickstart 1.3 default (WS5-05) remain unrun.

## 9. Spec questions

For the user's ruling; copied from `05-findings.md` (a withdrawn question is kept with its note).

From `findings/WS1-semantic-core.md`:

- **SQ-WS1-1 — withdrawn in task 1.3.1.** The module-level `let embed` contradiction between
  spec/13 §1 and spec/04 §16 was already ruled by the user on 2026-10-24
  (`plans/def-critical-major-closeout/00-ambiguity-register.md` AR-23: follow spec/04 §16)
  and recorded there as a frozen inconsistency; it is not re-raised.
- **SQ-WS1-2 — May a hex or binary literal use the full unsigned width in a signed context?**
  spec/01 §7 and spec/02 §9 say a hex or binary literal "may use the full unsigned width of
  the target" (`0xFFFFFFFF` is a valid `u32`). `let a: i32 = 0x80000000;` is rejected with
  E10202 (`build/tmp/assessment/WS1/diag/v.clo`). The text does not say whether the
  full-width allowance is a bit-pattern rule for signed contexts too.
- **SQ-WS1-3 — Is the opt-in `-m68020` / `-m68881` leg a profile requirement?**
  spec/15-profiles.md §2 lists `hosted-amiga` "Default CPU `-m68000` (opt-in `-m68020`)" and
  an "FPU build leg: opt-in `-m68881`". No compiler or driver option offers either (`grep
  -rn "68020\|68881" compiler/src` finds nothing at e24a8752; the flag tables in
  `compiler/src/clorraine/toolchain.rs:58-63` fix `-m68000`). Either the opt-in is a
  missing feature or the table describes flags a user adds by hand. Input to WS7-03.

From `findings/WS2-ownership.md`:

- **SQ-WS2-1 — When does a temporary created in an `if`/`while` condition drop?** spec/04 §3
  says temporaries drop "at the end of that statement", but an `if` or `while` statement
  contains its body; the text does not say whether a condition temporary drops after the
  condition or after the whole statement (and, for `while`, per iteration). Probes 219 and
  228 avoid depending on the answer.
- **SQ-WS2-2 — Is returning a capturing closure E10404 or E10400?** spec/06 §6.1 says
  "Storing, returning, or capturing a closure is E10404 (L-6)" and "a capturing lambda used
  where a plain `func(...)` value is required is E10400" (L-7). A return of type
  `func(): i32` matches both; `clc` reports E10400 (probe 257).
- **SQ-WS2-3 — Are two-phase borrows part of the model?** The WS2-06 checklist names them,
  but spec/16 does not define them (for example `xs.push(xs.len())`, an exclusive receiver
  lend with a read in the argument). Not probed.

From `findings/WS5-shims.md`:

- **SQ-WS5-1** — spec/05 §15 says runtime faults abort without the cleanup ladder (E4).
  On AmigaOS, aborting without releasing OS registrations crashes the machine
  (ASM-WS5-001). Should the spec require the hosted profiles' fault exit to release
  runtime-owned OS resources (interrupt servers, tasks, display, ports, locks), while
  still skipping user drops? ASM-WS5-001 stands as a platform defect either way.
- **SQ-WS5-2** — `std.amiga.regions` documents a lifetime rule ("clear the clip before
  freeing the region") that the safe surface does not enforce (ASM-WS5-006). Does
  spec/17 §10 ("memory safety is strong above the hardware boundary") allow a safe
  stdlib export to rely on a documented ordering rule?

From `findings/WS6-stdlib.md`:

- **SQ-WS6-1** — The text of NaN and infinity in templates and `text()` is unspecified
  (S2a fixes only the precision). Which spelling, and is a NaN's sign shown? (ASM-WS6-003)
- **SQ-WS6-2** — spec/appendix-stdlib.md §4 says library wrappers return `Status` and
  partial IO returns a count plus a `Status`, but no wrapper does: the appendix's own
  tables give `bool`, `T?`, and −1 counts. Which is normative — the §4 convention or the
  tables? (ASM-WS6-001 depends on the answer for its fix shape.)
- **SQ-WS6-3** — If `slice` of a `str` becomes legal (ASM-WS6-002), `toCString(s)` of a
  sub-view must not return the view's pointer unterminated. Should `toCString` require a
  NUL-terminated origin (and reject sub-views), or copy?

## 10. Appendix

**Measurement and run records** (`findings/records/`): `WS0-matrix.json` (Phase 0 matrix),
`assessment-baseline-bench.json`, `assessment-baseline-compile.json`,
`assessment-baseline-flags.json`, `WS1-results.json`, `WS1-contrast.json`,
`WS1-gating.log`, `WS1-robustness.log`, `WS2-results.json`, `WS3-*`, `WS4-attribution.log`,
`WS4-flags.log`, `WS4-patches.log`, `WS4-size.log`, `WS5-results.json`,
`WS5-contrast.json`, `WS5-runs.log`, `WS6-results.json`, `WS6-contrast.json`,
`WS6-runs.log`, `WS7-runs.log`, `WS8-assertions.tsv`, `WS8-defs.tsv`; the Phase 0 emission
record `findings/WS0-emission-record.tsv`; verifier reports `verification/verdicts-9.2.*.md`.

**Probe index.** Each workstream file ends with its probe index; probes live under
`probes/WS<n>/`, verifier reproductions under `probes/WS2/260-*`, `probes/WS3/verify-9.2.3/`,
and `probes/WS5/verify-9.2.4/`.

**DEF and GitHub issue mapping** (filled in tasks 10.3.2 and 10.3.4):

| ID | Severity | DEF | Issue |
|----|----------|-----|-------|
| ASM-WS5-001 | Critical | — | — |
| ASM-WS1-002 | Critical | — | — |
| ASM-WS2-001 | Critical | — | — |
| ASM-WS5-004 | Critical | — | — |
| ASM-WS5-005 | Critical | — | — |
| ASM-WS5-002 | Critical | — | — |
| ASM-WS5-016 | Critical | — | — |
| ASM-WS3-001 | Critical | — | — |
| ASM-WS1-004 | Critical | — | — |
| ASM-WS1-005 | Critical | — | — |
| ASM-WS3-002 | Critical | — | — |
| ASM-WS5-006 | Critical | — | — |
| ASM-WS6-001 | Major | — | — |
| ASM-WS5-023 | Major | — | — |
| ASM-WS7-001 | Major | — | — |
| ASM-WS5-008 | Major | — | — |
| ASM-WS5-012 | Major | — | — |
| ASM-WS5-007 | Major | — | — |
| ASM-WS5-009 | Major | — | — |
| ASM-WS1-001 | Major | — | — |
| ASM-WS3-003 | Major | — | — |
| ASM-WS6-002 | Major | — | — |
| ASM-WS5-013 | Major | — | — |
| ASM-WS5-017 | Major | — | — |
| ASM-WS5-018 | Major | — | — |
| ASM-WS5-010 | Major | — | — |
| ASM-WS3-008 | Major | — | — |
| ASM-WS3-007 | Major | — | — |
| ASM-WS7-002 | Major | — | — |
| ASM-WS4-002 | Major | — | — |
| ASM-WS4-001 | Major | — | — |
| ASM-WS0-001 | Minor | n/a | — |
| ASM-WS1-003 | Minor | n/a | — |
| ASM-WS2-002 | Minor | n/a | — |
| ASM-WS2-003 | Minor | n/a | — |
| ASM-WS3-004 | Minor | n/a | — |
| ASM-WS3-005 | Minor | n/a | — |
| ASM-WS3-006 | Minor | n/a | — |
| ASM-WS5-003 | Minor | n/a | — |
| ASM-WS5-011 | Minor | n/a | — |
| ASM-WS5-014 | Minor | n/a | — |
| ASM-WS5-015 | Minor | n/a | — |
| ASM-WS5-019 | Minor | n/a | — |
| ASM-WS5-020 | Minor | n/a | — |
| ASM-WS5-021 | Minor | n/a | — |
| ASM-WS6-003 | Minor | n/a | — |
| ASM-WS6-004 | Minor | n/a | — |
| ASM-WS6-005 | Minor | n/a | — |
| ASM-WS8-001 | Minor | n/a | — |
| ASM-WS8-002 | Minor | n/a | — |
| ASM-WS8-003 | Minor | n/a | — |
| ASM-WS8-004 | Minor | n/a | — |
| ASM-WS4-003 | Opportunity | n/a | — |
| ASM-WS4-004 | Opportunity | n/a | — |
| ASM-WS4-005 | Opportunity | n/a | — |
| ASM-WS4-006 | Opportunity | n/a | — |
| ASM-WS5-022 | Opportunity | n/a | — |
| ASM-WS7-003 | Opportunity | n/a | — |

## ST-10 checklist

- [ ] The ten sections of 03-04 are present
- [ ] The ranked findings in §4 are exactly the surviving ledger rows of `05-findings.md`
- [ ] No finding appears in §1, §4, §5, or §7 unless it is surviving
- [ ] Every surviving Critical/Major has a DEF row in `plans/00-roadmap.md`, and every finding-derived DEF row added by this plan maps to a surviving finding
- [ ] Every measured quantity cites its measurement record (ST-12)
