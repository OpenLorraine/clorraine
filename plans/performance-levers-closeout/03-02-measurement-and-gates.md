# Measurement and Gates: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 03-02-measurement-and-gates.md
> **Parent**: [Index](00-index.md)

## Overview

The scored benchmark comparison must describe the shipped build, and the
size win must have a repeatable same-compiler attribution. This document
specifies the harness alignment, the new `flag_audit.py` gate, the shape-gate
flags, the release Workbench build, the 1.3 corpus run, and the documentation
and issue closeout.

## Architecture

### Harness (`tools/perf/bench.py`)

Keep the debug flags and the compare invariants. Change the release vectors
to the shipped set and add the m68k release file size:

```python
HOST_RELEASE = [
    "-std=c99", "-O2", "-ffunction-sections", "-fdata-sections",
    "-Wl,--gc-sections", "-s",
]
M68K_RELEASE = M68K_BASE + [
    "-O2", "-ffunction-sections", "-fdata-sections",
    "-Wl,--gc-sections", "-s",
]
```

Add a small helper and use it for both release artifacts:

```python
def binary_bytes(path: Path) -> int:
    """Return a linked artifact's file size in bytes."""
    try:
        return path.stat().st_size
    except OSError as error:
        raise HarnessError(f"missing artifact: {path}") from error
```

`measure()` records `m68k_release_bytes` next to `m68k_release`:
`"m68k_release_bytes": binary_bytes(amiga_release_bin)`. `measure()` returns
the full record for calibration too, but the document's `calibration` summary
keeps only `instructions` and `host_release_ns`; the new field is written for
the scored entries. `command_compare` still requires
equal `flags`, `toolchain`, `runs`, and `profiles`; no new comparison mode is
added.

`tools/perf/tests/test_bench.py` gains:
- a literal pin of `HOST_RELEASE` and `M68K_RELEASE`;
- a `binary_bytes` test with a temporary file;
- a cross-check that parses the Rust constants
  (`HOST_RELEASE_GNU`, `HOST_RELEASE_FALLBACK`, `M68K_RELEASE_FLAGS`,
  `FREESTANDING_RELEASE_FLAGS`) and compares them, composed with their base
  flags, to the harness/audit arrays, including the `-mcrt=nix13` form.

### Audit (`tools/perf/flag_audit.py` and `make bench-flags`)

The audit is a stdlib Python script with a `run` command:
`python3 tools/perf/flag_audit.py run --check --out build/perf/flags.json`.
It imports `bench` for `run_command`, `size_sections`, `trace_vamos`,
`normalize_output`, `expected_output`, and the release constants.

Data model (keyed by `"<profile>/<program>"` so calibration/sieve appear once
per profile):

| Field | Meaning |
|---|---|
| `key` | `hosted-amiga/lookup`, `hosted-amiga-13/file-copier`, ... |
| `program` / `profile` | the split key |
| `old_flags` / `gc_flags` / `new_flags` | complete compiler argument arrays before `-o` (base + mode), no libraries; the old row never uses `--release` |
| `libs` | the link libraries appended after the source |
| `c_bytes` | emitted C size |
| `old` / `gc` / `new` | `.text`, `.data`, `.bss`, file bytes, raw/net instructions, output |
| `delta` | exact and percentage for each floor metric |

Set matrix:

| Profile | Programs | Old flags | GC flags | New flags |
|---|---|---|---|---|
| `hosted-amiga` | calibration + five scored | `-O2` | GC without `-s` | GC + `-s` |
| `host` | calibration + five scored | `-std=c99 -O2` | GC | GC + `-s` |
| `hosted-amiga-13` | calibration, sieve, four named corpus | `-O2` | GC | GC + `-s` with `-mcrt=nix13` |
| `freestanding` | corpus | `-O2` | `-O2` + GC | `-O2` (unchanged; assert equality; record the GC probe growth) |

The arrays are the complete compiler arguments before `-o`: for m68k that
includes `-std=c99 -mcrt=... -m68000 -fwrapv`; for host it includes
`-std=c99`. Libraries are a separate `libs` field. The old row never uses the
driver's `--release` line.

The rejected candidates (`-O3`, `-Os`, `-fomit-frame-pointer`, LTO, LTO+GC,
host unwind flags) are measured once in the Phase 1 probe matrix and recorded
in the findings; the committed repeat gate covers old/GC/new. This keeps the
repeat bounded while R1.1's candidate audit has a durable record.

For each program, emit C once with `clc`, build old/GC/new to one fixed
output path (rebuild in place), record sizes, run the binary, and compare
output. Host and `vamos` runs use a private per-program working directory
under `build/perf/flags/.../run`, because `file-copier` writes
`corpus-in.txt`/`corpus-out.txt` and must not dirty the repository. Trace the
m68k raw instructions for old and new on calibration and the scored programs;
compute `instructions_net` by subtracting each run's calibration. The 1.3
corpus uses
`spec/tests/corpus/<name>/expected-output.txt`, not the benchmark helper.

The repeat set is old/GC/new on the five scored programs plus calibration,
old/GC/new on the four named 1.3 corpus programs, and the freestanding pair.
The script runs it twice and compares every record except timing fields.

`--check` asserts R3 with exact integer arithmetic
(`new * 100 <= old * floor_percent`):
- each scored m68k `.text` ≤ 80% of old;
- each scored m68k stripped file ≤ 80% of old;
- each scored host file ≤ 85% of old;
- each named 13 corpus `.text` ≤ 60% of old;
- calibration raw instructions ≤ 90% of old;
- each scored net instruction count ≤ 101% of old;
- every output equal;
- freestanding shipped `.text` and file bytes **equal** the old pair; the
  separate freestanding GC probe records the 208 → 232-byte growth;
- each scored host release binary has no `.symtab` section (`readelf -S`);
  a missing `readelf` is a failure, not a skip.

It checks every group, prints all misses, writes the JSON first, and exits 1
if any check fails. `--repeat 1` exists for a quick local run; the gate uses
the default 2. A full gate run is roughly the cost of two `make bench` traces
plus the extra builds; development iterations use `--repeat 1`.

Make target:

```make
bench-flags:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/perf/flag_audit.py run --check --out build/perf/flags.json"
```

Add `bench-flags` to `.PHONY` and one row to the `AGENTS.md` command table.

### Shape Gate

`docker/scripts/bench-shape.sh` keeps its warning flags:

- nix20: add `-ffunction-sections -fdata-sections -Wl,--gc-sections -s` to
  the existing `-O2 -Wall -Wextra -Werror` line (links and proves the GC
  link);
- nix13: add `-ffunction-sections -fdata-sections` to the compile-only line;
  it proves compilation, not linking;
- freestanding and vbcc: unchanged.

### Release Workbench Build

`sdk-advanced-build` becomes:

1. `mkdir -p build/amiga-sdk` for the debug copies;
2. build `services.clo` and `workbench.clo` in debug (warning-bearing);
3. copy those binaries to `build/amiga-sdk/services-debug` and
   `build/amiga-sdk/workbench-debug`;
4. build both again with `--release` to the default paths;
5. the existing `sdk-a600-services` and `sdk-a1200-workbench` gates run the
   release binaries at the default paths and compare the same report lines
   and goldens.

The debug copies are developer artifacts with no gate consumer; the
warning-bearing compile is the point. If the debug copies are dropped, the
warning gate still needs a debug compile before the release build.

### 1.3 Release Corpus

`flag_audit.py` builds the four named 1.3 programs, runs them under `vamos`,
and compares each output with `spec/tests/corpus/<name>/expected-output.txt`.
`interrupt-counter` is not in this set; it is FS-UAE-only (see
`docker/scripts/corpus-amiga.sh:36,167-169`).

### Constructor Proof

The audit records:
- `nm`/`objdump` showing the remaining `___INIT_LIST__` entries and the
  `_start` loop that walks them;
- a `vamos --instr-trace` of the GC calibration that contains the `_start`
  init-loop range and at least one remaining constructor body (mapped through
  the HUNK load base);
- the removed `__amigapath.o`/`environ.o` entries from
  `-Wl,--print-gc-sections`.

The trace addresses are relocated by the HUNK load base; the audit derives
the base from the first `_start` PC and maps the `nm` symbol ranges before
asserting. If the trace cannot show a remaining constructor, the audit stops
and the reviewer decides whether the `KEEP` path is needed.

### Documentation and Issues

- `benchmarks/README.md`: shipped flags, attribution, `m68k_release_bytes`.
- `docs/known-limitations.md`: the stripped-release limitation scoped to
  Linux host/Amiga; freestanding and non-Linux keep symbols; debug builds
  have symbols.
- `plans/v1-production-readiness/00-ambiguity-register.md`: the matching
  accepted-limitation entry.
- Four issues: LTO/LTO+GC, opt-in `-Os`/`--size` mode, macOS `-dead_strip`,
  freestanding strip/GC. Labels: `performance`, `deferred`, `enhancement`.

## Implementation Details

### Audit Output Shape

```json
{
  "schema": 1,
  "clc": {"path": "...", "sha256": "...", "git": "..."},
  "toolchain": {"host_cc": "...", "m68k_gcc": "...", "vamos": "...", "python": "..."},
  "runs": 2,
  "programs": {
    "hosted-amiga/lookup": {
      "profile": "hosted-amiga",
      "program": "lookup",
      "libs": ["-lamiga", "-lm"],
      "old_flags": ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2"],
      "gc_flags": ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2", "..."],
      "new_flags": ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2", "...", "-s"],
      "old": {"text": 92976, "bin": 116584, "instructions_raw": 12842},
      "gc": {"text": 39904, "bin": 45008, "instructions_raw": 12398},
      "new": {"text": 39904, "bin": 41740, "instructions_raw": 12398},
      "delta": {"text_percent": -57.08, "bin_percent": -64.20}
    }
  },
  "checks": [{"name": "m68k_text", "passed": true}]
}
```

The findings table is generated from this JSON; the JSON itself stays under
`build/` (gitignored) and the committed evidence is the findings table plus
the script.

### Error Handling

| Error case | Handling | AR Ref |
|---|---|---|
| A build fails | Report the exact command and stop | AR-23 |
| A floor is missed | `--check` exits 1 with every miss; the landing stops | AR-10/AR-21 |
| A repeat differs | Report the differing field; stop | AR-09 |
| `clc` or a tool is missing | Report the missing tool; no partial pass | AR-23 |
| A required constructor is collected | Stop; add `KEEP`/`-u`; re-audit | AR-13 |
| A gate flakes | One audited rerun; a second failure stops | AR-21 |

## Testing Requirements

- Harness literal and `binary_bytes` tests.
- Driver/harness/audit cross-check test.
- Audit floor logic on synthetic pass/fail matrices.
- Audit repeatability on the real matrix.
- Release Workbench gates and the 1.3 corpus outputs.
- The full gate in R4.7.
