# Phase 2 spike findings - codegen strategy

Status: complete through S4; consolidated at the Phase 1 checkpoint.

This document records the four accepted Phase 2 spikes. Each section states the
goal, the fixture, the exact commands, the raw output, the codegen rule it
confirms or changes, and any open question that needs a specification ruling.
The raw command transcripts live under each spike's `results/` directory.

## Consolidated result

| Spike | Verdict | Codegen rule it fixes |
|---|---|---|
| S1 `#line` | Diagnostics on both compilers name the mapped `.clor` line; GCC's DWARF is debuggable, vbcc's Hunk Debug is not decodable with the SDK tools | Always emit `#line`; record the vbcc debugging limitation |
| S2 FFI ABI | ABI-typed prototypes build warning-free on GCC and vbcc, run under `vamos`, and balance their release counters | Emit ABI types (`BPTR`, `APTR`, `struct Library *`); include only type headers, not `proto/` headers; discard a `void` deleter's result |
| S3 arithmetic | Host, GCC `-fwrapv`, and vbcc unsigned-idiom legs all match the hand-computed table | `-fwrapv` for GCC, the unsigned idiom for vbcc, masked shift counts, arithmetic vs logical `>>`, and the division guard |
| S4 run faults | The shim contract is confirmed and five of six run cases trap on every leg; the NaN case is blocked by `vamos` | `cl_trap` prints `error CODE: message` first and aborts without the ladder; add `-lm` (GCC) and `-lmieee` (vbcc) for floats |

Preflight items the spikes bear on:

| Item | Severity | Status after the spikes |
|---|---|---|
| PF-002 | 🔴 | Resolved by S2; the checkpoint rules on the handle representation (see S2) |
| PF-006 | 🟠 | Guard confirmed; S4's `min_neg_one` case traps E11210 |
| PF-007 | 🟠 | The guard is confirmed on the host; the Amiga NaN verification is blocked by `vamos` (ruling requested) |
| PF-008 | 🟠 | Resolved by S3 |
| PF-020 | 🟠 | Run-case patterns and the corpus exist; the Amiga NaN leg is blocked (ruling requested) |
| PF-042 | 🟡 | `#line` diagnostics confirmed; the debug-decoding limitation is requested for a ruling (see S1) |
| PF-047 | 🟡 | Tied to the S2 handle-representation ruling |

## Checkpoint decisions requested

| # | From | Decision |
|---|---|---|
| D-1 | S1 | Keep Ch 17 section 11 as-is, or record that GCC is debuggable and vbcc debug output is not decodable with the SDK tools (recommended) |
| D-2 | S2 | Emit an opaque `Cl<Name>` handle plus a boundary cast, or emit the ABI type directly (recommended, with PF-047) |
| D-3 | S4 | Keep the `!(f >= lo && f <= hi)` NaN guard and mark the Amiga NaN leg environment-blocked, or add a shim `cl_isnan` bit test so the guard is platform-independent (recommended) |
| D-4 | S4 | Add `-lm`/`-lmieee` to the Amiga float legs always (recommended) or only when a program uses floats |

---

## S1 - vbcc `#line` (PF-042)

**Goal:** prove or refute that `#line N "file.clor"` survives vbcc into usable
debug and diagnostic information, before the emitter depends on it.

**Fixture:** [`spikes/01-vbcc-line/demo.c`](../spikes/01-vbcc-line/demo.c) - a
hand-written copy routine with an owned file handle, an owned buffer, a
single-exit cleanup ladder with null-safe drops, and a `#line` directive
directly before each statement. The directives map the statements to a
Clorraine source file, `demo.clor`, that is deliberately absent from disk.
Compiling with `-DERR` adds one statement that names an undefined symbol.
Makefile: [`spikes/01-vbcc-line/Makefile`](../spikes/01-vbcc-line/Makefile).

**Commands** (inside the SDK container, from `/work`):

| Purpose | Command |
|---|---|
| vbcc build | `vc +aos68k -c99 -g -O0 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -c -o build/demo.o demo.c` then link with `-lamiga` |
| objdump decode | `m68k-amigaos-objdump --dwarf=decodedline build/demo` |
| readelf decode | `m68k-amigaos-readelf --debug-dump=decodedline build/demo` |
| gdb lookup | `m68k-amigaos-gdb --batch -ex "info line demo.clor:28" build/demo` |
| error location | `vc ... -DERR -o build/demo-err demo.c -lamiga` |
| GCC comparison | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -o build/demo-gcc demo.c -lamiga`, then the same gdb lookup |

Raw output: `spikes/01-vbcc-line/results/{compile,decodedline,err,gcc-debug,format-probe}.txt`.

**Observed:**

| Probe | Result |
|---|---|
| vbcc build | succeeds; the only warnings are warning 53 from the system header `devices/timer.h`, not from `demo.c` |
| vbcc diagnostic through `#line` | `error 82 in line 32 of "demo.clor": unknown identifier <undefined_spike_symbol>` - the mapped file and line are used |
| vbcc debug decode | `objdump --dwarf=decodedline` prints only the file header; `readelf` rejects the file (`Not an ELF file`); `gdb` reports `No symbol table is loaded` |
| vbcc debug payload | the string `demo.clor` is present in both `demo.o` and the linked executable, so vbcc's `-hunkdebug` blob does carry the mapped name, but no SDK tool can decode it |
| GCC diagnostic through `#line` | `demo.clor:32:15: error: 'undefined_spike_symbol' undeclared` - same mapping |
| GCC debug decode | `gdb` resolves `Line 28 of "/work/demo.clor"` and lists `/work/demo.clor` in `info sources`; the executable has a `DEBUGGING` section |

**Decision / rule:**

* The emitter writes `#line` before every emitted statement unconditionally
  (P13). The spike confirms this is both correct and useful: both compilers name
  the `.clor` file and line in diagnostics, at no observed build cost.
* On the GCC legs the mapping reaches DWARF and a debugger can step and list the
  Clorraine source; the Ch 17 section 11 debugging claim holds there.
* On vbcc the mapping reaches only the Hunk Debug blob, which the SDK image's
  `objdump`, `readelf`, and `gdb` cannot consume. Debugger stepping of vbcc
  output is therefore not usable in this environment, although the information
  is emitted.

**Open question (specification ruling requested at the Phase 1 checkpoint):**
Ch 17 section 11 states the source-mapping and determinism guarantees, and
section 12 lists debug output, without naming a toolchain limitation. The
"stepping never enters generated code" wording is verifiable for GCC but not for
vbcc under the SDK image.

| Option | Text change | Trade-off |
|---|---|---|
| A | Keep Ch 17 section 11 unchanged | Claim stays universal but is not true for vbcc tooling; readers may assume vbcc debugging works |
| B | Note the limitation: GCC legs are debuggable and mapped; vbcc emits the mapping but SDK tooling cannot decode it in v1 | Accurate and explicit; no codegen impact |

Recommendation: **B**. It records the measured behavior without changing the
emission strategy, and it keeps PF-042 honest. The codegen rule itself (always
emit `#line`) is unaffected either way.

---

## S2 - generated-C FFI ABI (PF-002)

**Goal:** prove that the prototypes the compiler emits for FFI declarations
compile against the real NDK and behave correctly on the target.

**Fixture:** [`spikes/02-ffi-abi/ffi_abi.c`](../spikes/02-ffi-abi/ffi_abi.c) -
the emitted shape written by hand: ABI-typed prototypes (`BPTR` for a File
handle, `APTR` for a buffer view, `struct Library *` for the exec base,
`const char *` for a `cstring`), opaque `ClFile`/`ClLibrary` Clorraine-side
handles with an explicit boundary cast to and from the ABI type, a `ClStr` view
struct and a native pair passed by value, and release counters. The file
includes only the type headers (`exec/types.h`, `exec/memory.h`, `dos/dos.h`),
not the `proto/` headers, so these are the only prototypes.
Makefile: [`spikes/02-ffi-abi/Makefile`](../spikes/02-ffi-abi/Makefile).

**Commands** (inside the SDK container, from `/work`):

| Purpose | Command |
|---|---|
| GCC build | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -Wall -Wextra -Werror -o build/ffi-abi-gcc ffi_abi.c -lamiga` |
| vbcc build | `vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -o build/ffi-abi-vbcc ffi_abi.c -lamiga` |
| run | `vamos build/ffi-abi-gcc` and `vamos build/ffi-abi-vbcc` |

Raw output: `spikes/02-ffi-abi/results/{build-gcc,build-vbcc,gcc,vbcc,diff}.txt`.

**Observed:**

| Probe | Result |
|---|---|
| GCC build | warning-free under `-Wall -Wextra -Werror` |
| vbcc build | warning-free (no `proto/` header, so the system-header warning 53 does not appear) |
| handle width | `BPTR`, `APTR`, `ClFile`, and `ClLibrary` are all 4 bytes on m68k |
| boundary casts | `(BPTR)(ClFile)file` and `(struct Library *)(ClLibrary)lib` round-trip unchanged |
| view and struct by value | `ClStr` passed by value with a separate length; native `ClPair` by value returns 7 |
| `Close` as `void` deleter | links and runs although the NDK returns `BOOL`; the result is discarded |
| file round trip | write then read agree byte for byte (`round-trip=ok`) |
| release counters | `acquisitions=4 releases=4` on every path |
| compiler agreement | the GCC and vbcc outputs are identical (`diff` empty) |

**Decision / rule:**

* The **ABI-typed lowering is confirmed**: FFI prototypes must be written from
  the declared types through the Ch 17 section 3 ABI mapping, never from the
  parameter modes. The emitted prototypes link and run against the NDK.
* The emitted file must **not** include the `proto/` headers when it declares
  the same functions itself; it includes only the type headers. That is the
  Phase 0 pattern and it compiles cleanly on both compilers.
* Declaring an FFI deleter `void` while the NDK returns `BOOL`/`LONG` is
  ABI-safe; the emitted call discards the result.
* The boundary cast between the opaque `Cl<Name>` handle and the ABI type is a
  no-op on m68k. Keeping an opaque local type is therefore free, not costly.
* No prototype change is required. PF-002 is resolved by this spike.

**Open question (specification ruling requested at the Phase 1 checkpoint):**
Ch 17 section 3 maps a `handle File` binding to the opaque typedef `ClFile`,
while the ABI table (same section) and section 10 say the FFI prototype uses the
contract table's C type (`BPTR`). Both are true at once only if the emitter
converts at the call boundary, which the fixture proves works. PF-047 already
flags the related naming inconsistency.

| Option | Emitted local for an FFI-mapped handle | Trade-off |
|---|---|---|
| A | Opaque `Cl<Name>` handle plus a boundary cast at each call (current Ch 17 section 3 wording) | Keeps a distinct opaque type for C-side type safety; adds a cast per call |
| B | The contract-table ABI type directly (`BPTR file`), no cast; reserve `Cl<Name>` for handles with no fixed C type | Simplest emitted C; changes the Ch 17 section 3 mapping row and touches PF-047 |

Recommendation: **B**. On m68k both are 32-bit and the cast is a no-op, so B
produces less code and removes a conversion the checker would otherwise have to
track. It requires a small Ch 17 section 3/7 wording change, so it waits for the
checkpoint ruling together with PF-047.

---

## S3 - arithmetic wrapping and shifts (PF-008)

**Goal:** fix the wrapping strategy for both compilers (P12, Ch 17 section 10):
GCC takes `-fwrapv`; vbcc receives the unsigned idiom.

**Fixture:** one shared case table,
[`spikes/03-arithmetic/arith_cases.h`](../spikes/03-arithmetic/arith_cases.h),
driven by two entry points: `arith_gcc.c` (host and GCC) and `arith_vbcc.c`
(vbcc, with `-DVBCC_IDIOM` selecting the unsigned +, -, *, and unary - forms).
The table covers the signed extremes, `MIN / -1` and divide-by-zero behind the
emitted guard, and shift counts 0, 15, 31, 32, 33, and -1 for signed and
unsigned right shifts. The hand-computed reference is
[`spikes/03-arithmetic/expected.txt`](../spikes/03-arithmetic/expected.txt).
Makefile: [`spikes/03-arithmetic/Makefile`](../spikes/03-arithmetic/Makefile).

**Commands** (exact legs):

| Leg | Command |
|---|---|
| host | `cc -std=c99 -fwrapv -O2 -Wall -Wextra -Werror arith_gcc.c` |
| GCC | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -fwrapv ... arith_gcc.c -lamiga`, run under `vamos` |
| GCC no-flag | the same without `-fwrapv` (recorded, not normative) |
| vbcc | `vc +aos68k -c99 -O2 -cpu=68000 ... -DVBCC_IDIOM arith_vbcc.c -lamiga`, run under `vamos` |
| vbcc plain | the same without the idiom (recorded, not normative) |

Raw output: `spikes/03-arithmetic/results/{host,gcc,gcc-nowrapv,vbcc,vbcc-plain}.txt`
and the recorded diffs.

**Observed:**

| Leg | Result |
|---|---|
| host `-fwrapv` | matches the expected table exactly |
| GCC `-fwrapv` | matches exactly |
| vbcc with the unsigned idiom | matches exactly |
| GCC without `-fwrapv` | also matches at `-O2`; the recorded diff is empty |
| vbcc without the idiom | also matches at `-O2`; the recorded diff is empty |
| shifts | masked counts and sign-replicating signed `>>` agree on every leg |
| division guard | the `MIN / -1` and divide-by-zero cases return the sentinel; no undefined operation executes |

**Decision / rule:**

* The emission strategy of Ch 17 section 10 is **confirmed**: GCC legs carry
  `-fwrapv`, and vbcc receives `+`, `-`, `*`, and unary `-` on the unsigned
  counterpart, converted back to the signed type. The exact idiom is the
  `WRAP_ADD` / `WRAP_SUB` / `WRAP_MUL` / `WRAP_NEG` macros in
  `spikes/03-arithmetic/arith_cases.h`.
* Shift emission is confirmed: mask the count with `& 31`, shift the unsigned
  value for `<<` and logical `>>`, and keep the signed value for arithmetic
  `>>`.
* `/` and `%` stay behind `d == 0 || (a == MIN && d == -1)`; `/` and `%` then
  execute directly.
* At `-O2` both compilers happened to produce the wrapped results even without
  the flag or idiom. That is a coincidence of this optimizer setting: signed
  overflow is undefined behavior, so `-fwrapv` and the idiom remain normative
  and are what the emitter must produce.

**Open question:** none. The spike confirms the rules already written in
Ch 17 section 10, so no specification ruling is needed and PF-008 can close at
the checkpoint.

---

## S4 - run-fault patterns (PF-020)

**Goal:** validate the emitted runtime guards and the fault shim before codegen,
so the six `spec/tests/run/` cases become executable patterns with a proven
first line and skipped cleanup.

**Fixture:** [`spikes/04-run-faults/traps.c`](../spikes/04-run-faults/traps.c) -
the emitted guards for E11210 (zero divisor and `MIN / -1`), E11211 (bounds),
E11212 (float range and NaN), and E11213 (assert), plus a cleanup stand-in that
prints if it ever runs. Two shim variants are selected with `-DAMIGA_SHIM`:
host (`fprintf(stderr, ...)` then `abort()`) and Amiga (`printf(...)` then
`exit(20)`). The checker
[`spikes/04-run-faults/run-faults.sh`](../spikes/04-run-faults/run-faults.sh)
asserts, per case, the exact first line, that the cleanup ladder did not run,
and that the program did not exit 0.
[`case-map.txt`](../spikes/04-run-faults/case-map.txt) maps the six spec run
cases to the fixture cases.

**Commands** (inside the SDK container, from `/work`):

| Leg | Command |
|---|---|
| host | `cc -std=c99 -O2 -Wall -Wextra -Werror traps.c`, then `run-faults.sh host ...` |
| GCC | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -DAMIGA_SHIM ... -lamiga -lm`, then `run-faults.sh amiga-gcc ... vamos` |
| vbcc | `vc +aos68k -c99 -O2 -cpu=68000 ... -DAMIGA_SHIM ... -lamiga -lmieee`, then `run-faults.sh amiga-vbcc ... vamos` |

Raw output: `spikes/04-run-faults/results/{host,amiga-gcc,amiga-vbcc}-<case>.txt`.

**Observed:**

| Case | Host | Amiga GCC | Amiga vbcc |
|---|---|---|---|
| `div_by_zero` (E11210) | traps, exit 134 | traps, exit 20 | traps, exit 20 |
| `min_neg_one` (E11210) | traps, exit 134 | traps, exit 20 | traps, exit 20 |
| `index_out_of_range` (E11211) | traps, exit 134 | traps, exit 20 | traps, exit 20 |
| `float_to_int_range` (E11212) | traps, exit 134 | traps, exit 20 | traps, exit 20 |
| `nan_float_to_int` (E11212) | traps, exit 134 | **no trap**: prints a result and runs cleanup | **no trap**: `vamos` raises `ValueError: cannot convert float NaN to integer` in `IEEEDPFix` |
| `assertion_failed` (E11213) | traps, exit 134 | traps, exit 20 | traps, exit 20 |

The exact first line is `error CODE: message` using the Ch 14 messages, and no
captured output contains a cleanup marker for the five trapping cases.

**Decision / rule:**

* The **shim contract is confirmed**: `cl_trap` prints exactly
  `error CODE: message` as its first line and aborts without running the
  cleanup ladder. The host shim writes to stderr and calls `abort`; the Amiga
  shim writes to stdout and calls `exit(20)`. Codegen implements this contract.
* The guard shapes are confirmed and executable for five of the six cases:
  `d == 0 || (a == MIN && d == -1)`, `i >= n`, `!(f >= lo && f <= hi)`, and
  `!cond`.
* The GCC Amiga float legs need `-lm`; without it the soft-float double helpers
  (`__divdf3`, `__gedf2`, `__ledf2`) do not link. The vbcc float legs need
  `-lmieee`; without it `_MathIeeeDoubBasBase` is undefined. These are
  additions to the Ch 17 section 13 compile matrix.
* A NaN must be built from two distinct volatile zeros. A single `0.0 / 0.0`
  is folded to `1.0` by vbcc at `-O2`; two volatile zeros survive on both
  compilers and produce a real quiet NaN.

**Open questions (specification rulings requested at the Phase 1 checkpoint):**

1. **The E11212 NaN case cannot be verified under `vamos`.** The emulator's
   `mathieeedoubbas` comparisons are not IEEE: a real NaN (bits
   `7FF8000000000000`) compares as `nan == nan`, `nan >= lo`, and
   `nan <= hi`, so `!(f >= lo && f <= hi)` is false. The vbcc leg is worse:
   `vamos` raises a Python exception in `IEEEDPFix` when asked to convert NaN.
   This is an emulator limitation, not an emitted-C defect; the host leg and
   IEEE hardware behave correctly.

   | Option | Change | Trade-off |
   |---|---|---|
   | A | Keep the Ch 17 section 10 guard; mark the Amiga NaN run leg environment-blocked in the suite (PF-020 note) | No spec or codegen change; the NaN rule stays unverified on the Amiga legs until real hardware or a fixed emulator |
   | B | Add a shim `cl_isnan` (a bit test) and emit `if (cl_isnan(f) || !(f >= lo && f <= hi)) cl_trap(...)` | Makes NaN detection independent of the platform math library, so the NaN run case passes under `vamos`; needs a small Ch 17 section 10/12 amendment and one shim helper |

   Recommendation: **B**. It is a one-helper change that removes a dependency on
   a library the spikes proved unreliable, and it makes the always-on safety
   promise testable on every leg.

2. **Compile matrix update.** Ch 17 section 13 lists GCC as `... -lamiga` and
   vbcc as `... -lamiga`. Float programs need `-lm` (GCC) and `-lmieee`
   (vbcc). Either add the flags only for float-using programs or always.

   | Option | Change | Trade-off |
   |---|---|---|
   | A | Always append the flags to the Amiga leg commands | Deterministic and simple; a tiny unused-library cost when a program has no floats |
   | B | Add the flag only when the translation unit uses a float type | Smaller command line; needs a "uses float" tracking rule |

   Recommendation: **A**. The flags are harmless when unused, and always-on
   keeps the emitted command line deterministic and the matrix simple.
