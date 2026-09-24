# Current State: Freestanding profile

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler has two working profiles, `host` and `hosted-amiga`. A `Profile`
value is chosen by name in the driver, maps to an embedded C runtime shim and a
few numeric facts, and is passed to code generation. `freestanding` is known
only as a rejected name.

The `clorraine` driver (added by PLAN-02) owns the profile-to-toolchain table
and the build/run pipelines; the compiler (`clc`) stays emit-only and never
spawns a process.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/profile.rs` | `Profile` enum, `parse`, `shim`, `supports_module`, `stack_frame_limit` | Add `Freestanding`; add an `implemented` query; add the freestanding shim, module policy, and threshold |
| `compiler/src/driver.rs` | `clc` entry; `profile_available` and `report_unavailable_profile_imports` | Delete the duplicate name `match`; accept `freestanding` via `Profile`; keep the module `E11200` gate |
| `compiler/src/codegen.rs` | Driver of emission; `empty_state` and `null_literal` are profile-aware | Add the freestanding arm for the `0` null spelling |
| `compiler/src/codegen/program.rs` | Fixed-width typedefs (always `usize = unsigned int`), the AmigaOS ABI typedefs for `host`, and the `int main(void)` entry stub | Emit the ABI typedefs for `freestanding` too, because the emitted `APTR`/`ULONG` prototypes need them; the entry stub is unchanged |
| `compiler/src/stdlib.rs` | Embedded modules and the per-profile shim/symbol test | Extend the FFI-binding test to cover `freestanding` |
| `compiler/src/clorraine/cli.rs` | `--profile` parsing; help text; unknown-profile message | Accept `freestanding`; update the help and error text |
| `compiler/src/clorraine/toolchain.rs` | Profile-to-C-compiler table and runner | Add the freestanding m68k tool and the host no-libc run tool |
| `compiler/src/clorraine/{build,run}.rs` | Emit, compile, run, capture | Accommodate the two-artifact freestanding flow |
| `compiler/src/clorraine/mod.rs` | Command dispatch; `run_command` and `required_tools` | Route the freestanding run to the host artifact and require `cc` |
| `compiler/src/clorraine/artifacts.rs` | Artifact path policy | Own the `<stem>.host` path |
| `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c` | Existing profile shims | Reference only; a new `shim_freestanding.c` is added |
| `spec/tests/compile-fail/E11200_unimplemented_profile.clo` | Golden asserting freestanding is rejected | Removed; `E11201_unknown_profile.clo` keeps the unknown-name coverage |
| `compiler/tests/stage_spec_test.rs:1945-2007` | Asserts the freestanding rejection | Rewritten to assert acceptance |
| `spec/15-profiles.md`, `spec/00-introduction.md`, `spec/17-codegen.md`, `spec/README.md`, `spec/decisions.md`, `spec/14-diagnostics.md` | The frozen specification | Authorized edits (AR-21) |

### Code Analysis

The profile name is known in two places, which is why `freestanding` became a
special case:

- `Profile::parse` (`compiler/src/profile.rs:30-36`) maps `host` and
  `hosted-amiga`, and returns `None` for `freestanding`.
- `profile_available` (`compiler/src/driver.rs:319-337`) repeats the same names
  and gives `freestanding` its own `E11200` branch.

Consolidating on `Profile` (adding an `implemented` query) removes the
duplication and makes the new profile an ordinary case (AR-19).

The emitter always writes fixed-width typedefs including
`typedef unsigned int usize;` (`compiler/src/codegen/program.rs:73-80`), so
`usize` is 32-bit on every profile; the language models it as 32-bit too
(`compiler/src/types/mod.rs:309-323`). The freestanding ABI keeps that width
(AR-17).

The emitter always emits a `cl_str` helper that calls `strlen`
(`compiler/src/codegen/program.rs:99-105`), and GCC lowers aggregate copies to
`memcpy`. A live probe confirmed that `m68k-amigaos-gcc -nostdlib` therefore
reports an undefined `memcpy`; the freestanding runtime must provide `memcpy`,
`memset`, `memcmp`, and `strlen` (AR-9).

The `std.io` module mixes console printing with OS file handles:
`print`/`println` call `cl_write`/`cl_write_line`, while `Open`/`Close`/`Read`/
`Write` are OS contracts (`stdlib/std.io.clo:27-34`). Under freestanding the
console helpers work and the file helpers trap (AR-11, AR-12).

The generated entry is a single `int main(void)` stub that runs module
initializers and then the user `main`
(`compiler/src/codegen/program.rs:600-631`). Freestanding keeps this stub and
adds `_start` in the shim (AR-13).

## Gaps Identified

### Gap 1: The profile does not exist

**Current Behavior:** `Profile::parse("freestanding")` returns `None`, and the
driver prints `error E11200: 'freestanding' is not available on profile
'freestanding'`.

**Required Behavior:** `freestanding` is an ordinary profile with facts and a
shim; unknown names remain `E11201`.

**Fix Required:** Add the variant and facts; consolidate the name list on
`Profile` (AR-4, AR-19).

### Gap 2: There is no no-libc runtime

**Current Behavior:** Both existing shims use libc (`malloc`, `snprintf`,
`fwrite`, `abort`) and rely on the platform's C startup to call `main`.

**Required Behavior:** A freestanding shim provides `_start`, an allocation-free
runtime, and the injected `mem*`/`strlen` helpers, with a host syscall layer and
m68k no-op stubs.

**Fix Required:** Add `compiler/runtime/shim_freestanding.c` (AR-5, AR-7, AR-9,
AR-13, AR-14).

### Gap 3: No verification path without an OS

**Current Behavior:** Freestanding is not run or built. The "run" leg of a
profile is a single C compiler plus a runner.

**Required Behavior:** The m68k artifact builds and links with zero undefined
symbols; the host no-libc artifact compiles and runs.

**Fix Required:** Add the two tool rows and the harness that asserts both
(AR-7, AR-20, AR-27).

### Gap 4: The specification says freestanding is rejected

**Current Behavior:** PR-6 rejects "a profile that is designed but not
implemented (v1: `freestanding`)", and a golden test pins it.

**Required Behavior:** The profile is implemented with a provisional ABI; PR-6
remains a general rule with no current instance; the runtime code `E11214` is
registered and replaces the obsolete rejection golden.

**Fix Required:** The authorized specification and golden edits (AR-21, AR-30).

## Dependencies

### Internal Dependencies

- `Profile` and the codegen shim mechanism (`profile.rs`, `codegen.rs`).
- The module-availability gate (`driver.rs:report_unavailable_profile_imports`).
- The driver toolchain table and pipelines (`clorraine/**`).
- The existing `main` entry stub (`codegen/program.rs`).

### External Dependencies

- `m68k-amigaos-gcc` and its `libg.a` soft-float runtime (in the SDK container).
- The host `cc` and, on x86_64/aarch64, the raw syscall layer.
- `vamos` / FS-UAE are not used by this plan.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| `-ffreestanding -nostdlib` on m68k pulls in more than `memcpy`/`strlen` | Medium | Medium | Compile and link a representative corpus and assert the link exits 0 (AR-27) |
| `-fno-builtin` does not stop the backend's `memcpy` lowering | Low | Medium | Provide `memcpy`/`memset` and verify at `-O2`; a `-O2` link failure is caught by the test (AR-9, AR-16) |
| The aarch64 syscall layer cannot be tested on this machine | High | Low | Guard it, fall back to the libc-linked host run where untested (AR-8) |
| The provisional ABI drifts from what LorraineOS later needs | Medium | Medium | Keep the ABI thin (entry, exit, console hook) and record it in the spec for adoption (AR-3) |
| `cl_str`'s `strlen` is emitted even for programs that never use it | Low | Low | It is `static inline`; providing `strlen` removes any reference concern (AR-9) |
