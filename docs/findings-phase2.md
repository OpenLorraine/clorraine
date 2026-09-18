# Phase 2 spike findings - codegen strategy

Status: in progress. S1 recorded; S2-S4 pending. Consolidated at the Phase 1
checkpoint.

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
