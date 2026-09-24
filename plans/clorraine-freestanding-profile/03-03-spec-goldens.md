# Spec and Goldens: Freestanding profile

> **Document**: 03-03-spec-goldens.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the authorized changes to the frozen specification and the
golden test data. It is where the provisional ABI is written down for LorraineOS
to adopt, and where the obsolete "freestanding is rejected" oracle is replaced
(AR-3, AR-21, AR-30).

## Architecture

### Current Architecture

The specification records freestanding as designed-but-unimplemented: PR-6
rejects it (`spec/15-profiles.md:39`), §2 says its facts are "TBD with
LorraineOS" (`:47-57`), §4 says it is "not run" (`:97`), and the introduction
and README repeat the "designed, later" status. A golden
`spec/tests/compile-fail/E11200_unimplemented_profile.clo` pins the rejection,
and `compiler/tests/stage_spec_test.rs:1990-2006` asserts it.

### Proposed Changes

State the profile as implemented with a **provisional** ABI, keep PR-6 as a
general rule, add the runtime diagnostic, and replace the rejection oracle with
an acceptance test.

## Implementation Details

### `spec/15-profiles.md`

| Location | Change |
| -------- | ------ |
| PR-6 (`:39`) | Reword: "A profile that is declared but not implemented is rejected" — drop the `(v1: freestanding)` clause; the rule stays for future profiles |
| §2 table (`:49-57`) | `freestanding` becomes an implemented row: compiler `m68k-amigaos-gcc -nostdlib`; runtime "self-contained freestanding shim"; CPU `-m68000`; endianness big; pointer 32-bit; entry `_start`; package raw hunk. Add one sentence: the ABI is provisional and LorraineOS-adoptable |
| §2 policy table (`:62-66`) | Float ABI: soft-float through `libg.a` (`-lg`); stack threshold 1024 bytes |
| §4 (`:97`) | `freestanding` "built and linked on m68k; run through the host no-libc layer" |
| §6 (`:113-119`) | Note the profile is implemented; ARM64 remains design-only |

### `spec/00-introduction.md`

- `:68-70` and the §5 table (`:129`): freestanding status becomes "implemented
  (provisional, m68k)".

### `spec/02-types.md`

- The implicit-widening table (section 9, `:266-271`): add the
  `static str -> cstring` row (a literal is a NUL-terminated C string). This is
  the AR-31 runtime decision; it is the rule ST-18 depends on.

### `spec/17-codegen.md`

- §12 (`:339-346`): fill the freestanding column — acquire/drop are the
  console/file trap helpers; panic prints then halts; trap prints then halts;
  NaN test is the same bit test; debug output is the console hook.
- §13 (`:352-387`): add the freestanding command line as a normative row:
  `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000
  -fwrapv -O0 -g -Wall -Wextra -Werror -o demo demo.c -lg`, and note the release
  variant and the host no-libc run line.

### `spec/14-diagnostics.md` and `spec/check.sh`

Add to the runtime-fault table (`§11`): `E11214` — "feature not available on the
selected profile" — used when a `std.io` file function runs under freestanding.
`spec/check.sh`'s `excluded_codes` list already names the runtime faults
(`E11210`–`E11213`); add `E11214` there too, or the per-code coverage loop
demands a compile-fail golden for it.

### `spec/README.md` and `spec/decisions.md`

- README scope line: freestanding moves from "designed-for, implemented later"
  to "implemented provisionally".
- decisions.md: record the provisional-ABI ruling (m68k/32-bit/big-endian, no
  libc, thin entry/exit/console surface, ARM64 width deferred).

### Golden and corpus data

| Artifact | Change |
| -------- | ------ |
| `spec/tests/compile-fail/E11200_unimplemented_profile.clo` | **Removed** (the profile is implemented); **replaced** by `E11200_unavailable_module.clo`, a `host` program importing `std.amiga.dos`, so `E11200` keeps a compile-fail golden |
| `spec/tests/compile-fail/E11201_unknown_profile.clo` | Kept; it still covers the unknown-name rule |
| `spec/tests/corpus/freestanding/program.clo` + `expected-output.txt` | Added; a small console program run by the freestanding harness |
| `compiler/tests/stage_spec_test.rs:1945-2007` | The freestanding branch asserts acceptance and emitted C instead of `E11200` |
| `compiler/tests/compile_fail_spec_test.rs:39` | `E11200` stays in the expectation set via the replacement golden; `E11201` is unchanged |

The corpus harness (`compiler/tests/corpus_spec_test.rs`) names its programs
explicitly, so the new `freestanding/` directory is not auto-run there; the
freestanding harness reads it by name. This plan verifies that assumption before
relying on it ([99-execution-plan.md](99-execution-plan.md) task 1.3.1).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A golden removed but still referenced by a test list | Update `compile_fail_spec_test.rs` and `stage_spec_test.rs` in the same phase | AR-30 |
| `spec/check.sh` per-code coverage | `E11200` is registered and not excluded, so deleting its only golden fails the check; add the replacement golden and, for `E11214`, extend the `excluded_codes` list | AR-21, AR-30 |
| A spec chapter is changed to match the implementation without a ruling | The modification set is exactly AR-21; no other chapter changes | AR-21 |

## Testing Requirements

- The stage spec test asserts `clc --profile freestanding` exits 0 and emits the
  shim and `_start` (ST-1).
- The compile-fail suite still passes with the unknown-profile golden (ST-14).
- `./spec/check.sh` passes, including any golden count it asserts.
