# Phase 2 Spikes: Clorraine Compiler MVP

> **Document**: 03-02-spikes.md
> **Parent**: [Index](00-index.md)
> **Status**: Phase 1 source — executed before compiler code (AR-8, AR-41)

## Overview

Four spikes are the accepted Phase 2 verification items from the preflight
gate (`preflight-report.md:121-126`, `findings1:87-91`, `decisions.md:330,336,370`).
They validate on the real toolchain the three codegen strategies the MVP
depends on before any Rust code is written, and they turn the six existing
`spec/tests/run/` fault cases into an executable pattern. Each spike is a
small hand-written C fixture in the style of
`examples/hosted-skeleton/skeleton.c`, run through the exact Ch 17 §13
commands; the results go to `docs/findings-phase2.md`.

Ordering: Phase 0 (corpus) first, then these spikes (user instruction,
AR-1); the compiler phases depend on both.

## S1 — VBCC `#line` (PF-042)

**Goal:** prove or refute that `#line N "file.clo"` (Ch 17 §11) survives
vbcc into usable debug/line information.

**Fixture:** `spikes/01-vbcc-line/demo.c` — a small program with `#line`
directives that map successive statements to `demo.clo` lines, including one
deliberately erroneous statement behind a second translation of the file
(`-DERR`), plus a cleanup ladder shape.

**Procedure:**
1. Compile with `vc +aos68k -c99 -g -O0 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -o demo demo.c -lamiga`.
2. Confirm `#line` is accepted (no error, no warning).
3. Inspect the debug/line table: `m68k-amigaos-objdump --dwarf=decodedline demo` (and `m68k-amigaos-readelf --debug-dump=decodedline demo` as fallback); record whether entries name `demo.clo`.
4. Compile the `-DERR` variant and record which file/line the vbcc error names.
5. If gdb can load the hunk executable (`m68k-amigaos-gdb demo`), record `info line` / `list` output.

**Acceptance / decision impact:** the finding records exactly how far vbcc
honors `#line`. If debug line tables do not carry `demo.clo`, the Ch 17 §11
debugging claim is refuted in practice; the user rules on the spec text at the
checkpoint (AR-31). The emitter always writes `#line` (P13) regardless.

## S2 — Generated-C FFI ABI (PF-002)

**Goal:** prove that prototypes produced by the Ch 17 §3 ABI mapping compile
against the NDK and behave correctly on the target.

**Fixture:** `spikes/02-ffi-abi/ffi_abi.c` — hand-written prototypes exactly
as the emitter would write them:

```c
typedef struct ClFileClosure *ClFile;      /* BPTR (32-bit)             */
typedef struct ClLibraryClosure *ClLibrary;/* exec base                 */
typedef struct { const u8 *ptr; u32 len; } ClStr;   /* str view        */

extern ClLibrary OpenLibrary(const char *name, ULONG version);   /* LONG */
extern void      CloseLibrary(ClLibrary base);
extern ClFile    Open(const char *name, LONG mode);              /* BPTR */
extern LONG      Read(BPTR f, APTR buf, LONG len);
extern LONG      Write(BPTR f, APTR buf, LONG len);
extern void      Close(BPTR f);
extern APTR      AllocVec(ULONG size, ULONG flags);
extern void      FreeVec(APTR mem);

/* the emitted view wrapper: pointer + separate length, no struct */
static LONG cl_write_view(BPTR f, const u8 *ptr, u32 len) { return Write(f, (APTR)ptr, (LONG)len); }
```

plus a struct-by-value call and release counters (the Phase 0 pattern).

**Procedure:**
1. Compile with the GCC debug command of Ch 17 §13 and with the vbcc command.
2. Run under `vamos` in the SDK container; copy a file through the mappings.
3. Record the observed ABI facts: handle width, `BPTR`↔pointer equivalence,
   `APTR` view passing, `BOOL`/`LONG` return handling, struct-by-value.
4. Record acquisition/release counts (must balance).

**Acceptance / decision impact:** both compilers accept the prototypes and the
program runs correctly under vamos. A mismatch stops codegen work and is
recorded for a user ruling.

## S3 — Arithmetic Wrapping and Shifts (PF-008)

**Goal:** fix the wrapping strategy for both compilers (P12, Ch 17 §10): GCC
takes `-fwrapv`; vbcc receives unsigned-idiom operations converted back.

**Fixture:** `spikes/03-arithmetic/arith_gcc.c` and
`spikes/03-arithmetic/arith_vbcc.c` (`-DVBCC_IDIOM` selects the unsigned
forms), covering:
- `i32` `+`, `-`, `*`, unary `-` at the extremes (`INT32_MIN`, `INT32_MAX`);
- `MIN / -1` and division by zero behind the emitted guard;
- shift counts 0, 15, 31, 32, 33, `-1` (masked), signed `>>` (arithmetic) and
  unsigned `>>` (logical).

Expected values come from the Ch 17 §10 table (wrapping two's complement,
counts masked to the operand width, sign-replicating signed `>>`).

**Procedure:**
1. Host sanity: `cc -std=c99 -fwrapv` runs the table.
2. GCC leg: `m68k-amigaos-gcc ... -fwrapv`; run under `vamos`; diff the table.
3. vbcc leg: `vc +aos68k ...` with the unsigned idiom; run under `vamos`; diff.
4. Also compile the numeric-operation file *without* `-fwrapv` to record
   whether the default differs (evidence for why the flag is normative).

**Acceptance / decision impact:** both compilers produce the defined results.
The finding records the exact vbcc idiom (or proves it unnecessary), which
[03-06-codegen.md](03-06-codegen.md) then implements; PF-008 closes.

## S4 — Run-Fault Patterns (PF-020)

**Goal:** validate the emitted runtime guards and the fault shim before
codegen, so the six `spec/tests/run/` cases become executable patterns.

**Fixture:** `spikes/04-run-faults/traps.c` — the emitted patterns:
- division guard: `if (d == 0 || (a == INT32_MIN && d == -1)) cl_trap("E11210", "integer division by zero");`
- bounds guard: `if (i >= n) cl_trap("E11211", "index out of range");`
- float guard: `if (cl_isnan(f) || !(f >= lo && f <= hi)) cl_trap("E11212", "float-to-integer conversion out of range");`
- `assert`: `if (!cond) cl_trap("E11213", "assertion failed");`

`cl_trap` prints exactly `error CODE: message` as its first line and aborts
without running a registered cleanup (a counter proves the ladder is skipped).
Two shim variants: host (`fprintf(stderr, ...)` + `abort()`) and Amiga
(`printf(...)` + `exit(20)`).

**Procedure:**
1. Host: compile with `cc -std=c99`, run each guard, capture stderr and exit
   status; assert the first line and that the cleanup counter stays 0.
2. Amiga: GCC and vbcc builds of the Amiga shim; run under `vamos`; capture
   stdout; assert the first line and skipped cleanup.
3. Confirm the six `spec/tests/run/` cases map one-to-one onto the fixture
   patterns (including NaN via `0.0 / 0.0` and `MIN / -1`).

**Acceptance / decision impact:** the six run cases are executable patterns
with a proven first line and abort behavior on both profiles; this fixes the
shim contract implemented in [03-06-codegen.md](03-06-codegen.md) and makes
R13 of [01-requirements.md](01-requirements.md) directly testable.

## Deliverables and Findings

| Item | Location |
|---|---|
| Spike fixtures and Makefiles | `spikes/01-vbcc-line/`, `spikes/02-ffi-abi/`, `spikes/03-arithmetic/`, `spikes/04-run-faults/` |
| Results, commands, observations | `docs/findings-phase2.md` (one section per spike, with the exact commands and raw output) |
| Closed PF items | PF-002, PF-008, PF-042 statuses updated in `spec/preflight-report.md` only after a user ruling (AR-31) |

The findings document must state, per spike: goal, fixture, exact commands,
observed result, the codegen rule it confirms or changes, and any open
question requiring a specification amendment. Codegen work (Phase 6) may not
rely on a strategy that a spike did not confirm.
