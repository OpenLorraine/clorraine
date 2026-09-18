# Phase 2 spike findings - codegen strategy

Status: in progress. S1 and S2 recorded; S3-S4 pending. Consolidated at the
Phase 1 checkpoint.

This document records the four accepted Phase 2 spikes. Each section states the
goal, the fixture, the exact commands, the raw output, the codegen rule it
confirms or changes, and any open question that needs a specification ruling.
The raw command transcripts live under each spike's `results/` directory.

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
