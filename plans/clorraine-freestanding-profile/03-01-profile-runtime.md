# Profile and Runtime: Freestanding profile

> **Document**: 03-01-profile-runtime.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes `freestanding` a real profile: the compiler selects it,
embeds a no-libc runtime shim, and gates the standard-library surface. It owns
the `Profile` changes, the new `compiler/runtime/shim_freestanding.c`, the entry
point, the float policy, and the profile-aware emission details. The driver
toolchain is a separate component ([03-02](03-02-driver.md)); the specification
edits are a third ([03-03](03-03-spec-goldens.md)).

## Architecture

### Current Architecture

`Profile` (`compiler/src/profile.rs`) is a two-variant enum. `shim()` returns an
embedded C string; `supports_module` decides which standard-library modules a
profile provides; `stack_frame_limit` supplies the `W10004` threshold. The
driver repeats the profile names in `profile_available`
(`compiler/src/driver.rs:319-337`). Code generation is profile-aware in only two
places beyond the shim: `empty_state` and `null_literal`
(`compiler/src/codegen.rs:311-331`) replace `NULL` with `0` for `hosted-amiga`,
and `emit_typedefs` (`compiler/src/codegen/program.rs:83`) declares the AmigaOS
ABI typedefs only for `host`.

### Proposed Changes

Add `Profile::Freestanding` with its facts, embed the new shim, and make the
name list single-sourced on `Profile` (AR-19). Treat `freestanding` like `host`
for the ABI typedefs and the `0` null spelling (AR-18). Add the module policy
(AR-11) and the threshold (AR-16).

## Implementation Details

### New Types and Functions

`compiler/src/profile.rs`:

```rust
/// The C runtime shim source of the freestanding profile.
const FREESTANDING_SHIM: &str = include_str!("../runtime/shim_freestanding.c");

pub enum Profile {
    Host,
    HostedAmiga,
    /// A platform with no operating system: no libc, its own start-up.
    Freestanding,
}

impl Profile {
    /// Resolves a validated profile name. `None` for an unknown name.
    pub fn parse(name: &str) -> Option<Self> { /* adds "freestanding" */ }

    /// True when the profile is implemented and may be selected.
    ///
    /// A profile can be named by the specification but not yet implemented;
    /// selecting it is an error, not an unknown name. This is the single place
    /// that answers the question, so the driver never repeats the name list.
    pub fn implemented(self) -> bool { true /* all three are implemented */ }

    pub fn supports_module(self, module: &str) -> bool { /* see below */ }
    pub fn shim(self) -> &'static str { /* adds FREESTANDING_SHIM */ }
    pub fn stack_frame_limit(self) -> usize { /* 1024 for freestanding */ }
}
```

`compiler/src/driver.rs`:

The string `match` in `profile_available` is deleted. The check becomes:
`Profile::parse(name)` yields `None` for an unknown name (`E11201`); a parsed
profile that is not `implemented()` is `E11200`. Today every parsed profile is
implemented, so only `E11201` can fire, but the general rule is preserved for a
future declared-but-unimplemented profile (AR-30).

`compiler/src/codegen.rs`:

`empty_state` and `null_literal` gain `Profile::Freestanding` in the `0`
branch. The freestanding ABI mirrors the host and declares `BPTR` as `void *`, so
`NULL` would also be valid; the `0` spelling is chosen for consistency with the
hosted-amiga provisional ABI, whose `BPTR` is an integer (AR-18).

`compiler/src/codegen/program.rs`:

The ABI typedef block (`BPTR`, `APTR`, `ULONG`, and the rest) is emitted for
`host` **and** `freestanding`; the hosted-amiga shim still supplies them through
`exec/types.h` (AR-18).

### Module policy

`supports_module` for `Freestanding` returns true only for `std.io`; `std.mem`,
`std.text`, `std.fmt`, and `std.amiga.*` return false, so importing them under
freestanding is a compile-time `E11200` through the existing gate in
`driver.rs:report_unavailable_profile_imports` (AR-11).

### The runtime shim

`compiler/runtime/shim_freestanding.c` has three layers:

**1. Portable runtime** (same on every target):

| Symbol | Purpose |
| ------ | ------- |
| `memcpy`, `memset`, `memcmp`, `strlen` | GCC lowers aggregate copies and the always-emitted `cl_str` to these; `-nostdlib` has no libc to supply them (AR-9) |
| `#define cl_zero(pointer)` | `memset((void *)(pointer), 0, sizeof(*(pointer)))` |
| `static inline int cl_isnan(double)` | the pure bit-pattern NaN test, copied from the host shim |
| `cl_trap`, `cl_panic` | format `error CODE: message` / `panic: message`, write it in one call to fd 2, then exit (AR-14, AR-29) |
| `cl_write`, `cl_write_line` | the console helpers `std.io` binds; write to fd 1 |
| `Open`, `Close`, `Read`, `Write` | the `std.io` file contract; each calls `cl_trap("E11214", "file I/O is not available on the freestanding profile")` (AR-12, AR-28) |

**2. Platform layer**, selected at compile time:

| Target | Console write | Exit |
| ------ | ------------- | ---- |
| `__m68k__` | documented no-op stub, marked "LorraineOS will provide" | documented infinite halt (AR-15) |
| `__x86_64__`, `__aarch64__` | `write` syscall | `exit` syscall (AR-8) |
| any other host | `extern long write(int, const void *, unsigned long)` from the linked C runtime | `extern void _exit(int)` (AR-8) |

The syscall layer uses inline `__asm__`; it lives in the shim, not in emitted
Clorraine code, and is guarded so the emitted C stays portable C99. The fallback
keeps the run leg working on untested architectures; the no-libc guarantee is
proven separately by the m68k link (AR-27).

**3. Entry**:

```c
int main(void);          /* the generated stub, defined later in the file */

void _start(void)
{
    cl_exit(main());
}
```

On the host, `cl_exit` raises the `exit` syscall with `main`'s return; on m68k it
halts. The generated `int main(void)` stub and its module-initializer calls are
unchanged (AR-13). Because the shim is pushed before the emitter's typedefs, the
shim uses C built-in types only (`void *`, `unsigned int`), exactly as the host
shim does.

### Build flags

| Leg | Command shape |
| --- | ------------- |
| m68k build+link | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o prog prog.c -lg` |
| host no-libc run | `cc -std=c99 -ffreestanding -fno-builtin -nostdlib -O0 -g -Wall -Wextra -Werror -o prog prog.c` (add `-lgcc` only if a probe shows it is needed) |
| host fallback run | the host line without `-nostdlib` |

`-fno-builtin` stops the compiler from turning the runtime's own `memcpy`/
`memset` into calls to themselves at `-O2` (AR-9, AR-16). `-lg` supplies the
soft-float helpers on m68k (AR-10). `libg.a` also contains objects (`malloc`,
`exit`) that reference the AmigaOS global `SysBase`, so the m68k runtime must
reference none of libg's libc-ish symbols: its exit is a no-op halt, and it must
not call `exit`/`abort`/`malloc`. Only then are the float objects alone pulled
in and the link self-contained.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unknown profile name | `E11201`, as today | AR-30 |
| Declared-but-unimplemented profile | `E11200`; no such profile exists today, so the branch is dormant | AR-30 |
| Import of a rejected module under freestanding | `E11200` at the import site | AR-11 |
| Template placeholder under freestanding | compile-time `E11200`; the heap formatter is not in the runtime | AR-11, RV-02 |
| Use of a `std.io` file function | runtime `E11214` trap, non-zero exit | AR-12, AR-28 |
| Runtime fault (bounds, division, conversion) | existing `E11210`-`E11212`, printed by `cl_trap` | AR-14 |
| Float arithmetic on m68k | resolves through `libg.a` from `-lg` | AR-10 |
| `memcpy`/`strlen` unresolved under `-nostdlib` | supplied by the shim | AR-9 |

## Testing Requirements

- Unit tests in `profile.rs` for the new variant, name round-trip, module policy,
  shim contents, and stack threshold (ST-12, ST-14).
- Unit tests in `driver.rs` for the unknown-name and unimplemented branches.
- `stdlib.rs`'s per-profile FFI-binding test is extended to `freestanding`, so
  the shim is asserted to define `Open`, `Close`, `Read`, `Write`, `cl_write`,
  and `cl_write_line` (ST-8).
- An implementation test asserts the m68k runtime references no libg libc-ish
  symbol (`exit`, `abort`, `malloc`), which keeps `-lg` self-contained (AR-10).
- The shim is exercised end to end by the freestanding run harness
  ([07-testing-strategy.md](07-testing-strategy.md)).
