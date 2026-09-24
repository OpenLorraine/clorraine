# Ambiguity Register: Freestanding profile

> **Status**: ✅ GATE PASSED — 30 items: 28 resolved, 2 explicitly deferred
> **Last Updated**: 2026-09-23 18:43
> **CodeOps Artifact Schema**: 1

Every row below was raised during the Phase 1 interview and resolved by the
user, or resolved by an agreed improvement, or explicitly deferred. The
freestanding profile is a new, third profile that this project implements with a
provisional, LorraineOS-adoptable ABI; it changes the frozen specification only
where the user authorized it (AR-3, AR-21).

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Planning form and tracking | Full feature plan + roadmap row / lightweight task mini-plan / plan folder only | Full feature plan at `plans/clorraine-freestanding-profile/`, plus the existing `PLAN-03` roadmap row | ✅ Resolved |
| AR-2 | Scope | What PLAN-03 covers | Profile only / profile + staged language features / language features only | The profile (target facts, runtime, entry), driver wiring, and spec; not the staged language features | ✅ Resolved |
| AR-3 | Scope | The spec delegates freestanding's facts to LorraineOS, which does not exist | Wait for LorraineOS / this project defines a provisional ABI | This project defines a provisional, LorraineOS-adoptable ABI now (AR-21 authorizes the spec change) | ✅ Resolved |
| AR-4 | Naming | Profile name and Rust spelling | Keep `freestanding` / rename | Keep the spec name `freestanding`; Rust variant `Profile::Freestanding` | ✅ Resolved |
| AR-5 | Naming | Runtime shim file | New file / reuse host shim | New `compiler/runtime/shim_freestanding.c`, with the platform layers clearly sectioned | ✅ Resolved |
| AR-6 | Naming | Plan, test, and corpus locations | Variants | Plan `plans/clorraine-freestanding-profile/`; run harness `compiler/tests/freestanding_spec_test.rs` (owns the two-leg logic); `.clo` inputs under `spec/tests/corpus/freestanding/` | ✅ Resolved |
| AR-7 | Verification | There is no LorraineOS to run on | m68k build-only + host no-libc run / m68k build-only only / vamos exec traps | m68k build-only (`-ffreestanding -nostdlib … -lg`) and a host no-libc run through an authored `_start` and raw syscalls | ✅ Resolved |
| AR-8 | Technical | Host syscalls need architecture-specific code | x86_64 only / x86_64 + aarch64 with a libc fallback elsewhere / skip the run elsewhere | Arch-guarded x86_64 and aarch64; on any other architecture the host run links the platform C runtime so the leg still executes | ✅ Resolved |
| AR-9 | Technical | GCC injects `memset`/`memcpy`/`strlen` under `-nostdlib` | Provide them / forbid builtins only | The runtime provides `memset`, `memcpy`, `memcmp`, and `strlen`; the freestanding build adds `-fno-builtin` and is verified at `-O2` as well as `-O0` | ✅ Resolved |
| AR-10 | Technical | m68k soft-float helpers are not in `libgcc.a` | Reject floats `E11200` / link an SDK archive | Support floats; the m68k leg appends `-lg` (`libg.a` holds the compiler soft-float helpers; a probe linked cleanly when the runtime referenced no libg libc-ish symbol). The host no-libc leg uses native floats | ✅ Resolved |
| AR-11 | Scope | Freestanding standard-library surface | Console only / console + heap + format | Console only. `std.io` is available; `std.mem`, `std.text`, `std.fmt`, and `std.amiga.*` are rejected with `E11200` | ✅ Resolved |
| AR-12 | Behavioral | File I/O lives inside `std.io` beside `print`/`println` | Compile-time contract gate / new `std.console` module / runtime trap | `std.io`'s `Open`/`Close`/`Read`/`Write` compile but trap at runtime | ✅ Resolved |
| AR-13 | Behavioral | Freestanding program entry | Keep `main` and add `_start` in the shim / emit `_start` from codegen | The emitter still writes `int main(void)`; the shim defines `_start`, which calls `main` and then exits | ✅ Resolved |
| AR-14 | Behavioral | Runtime fault and panic behavior | Match host `abort` / print then `_exit` | Host-run `cl_trap`/`cl_panic` write the whole message to fd 2 in one write, then `_exit(1)`; the m68k runtime prints through its no-op stub, then halts | ✅ Resolved |
| AR-15 | Behavioral | m68k build-only runtime behavior | Documented no-op stubs / placeholder MMIO | Documented no-op stubs: the console hook does nothing and `_start` halts after `main`. Each is marked "LorraineOS will provide" | ✅ Resolved |
| AR-16 | Non-functional | Freestanding `W10004` stack-frame threshold | 1024 / 16384 | 1024 bytes | ✅ Resolved |
| AR-17 | Technical | `usize`/`isize` are hardcoded 32-bit | Route width through the profile now / keep 32-bit | Keep 32-bit; ARM64's 64-bit width is deferred (AR-25) | ✅ Resolved |
| AR-18 | Technical | Null and empty-state spelling | `NULL` / `0` | Mirror hosted-amiga: the integer constant `0`, so the shim needs no `<stddef.h>` | ✅ Resolved |
| AR-19 | Behavioral | `Profile` facts are known in two places (`profile.rs` and `driver.rs`) | Duplicate the match / make `Profile` the single source | Make `Profile` the single source of truth: add a capability query (`implemented`) and delete the string match in `driver.rs`. `clc --profile freestanding` is accepted | ✅ Resolved |
| AR-20 | Technical | Driver behavior for freestanding | Build m68k + host-run / build only / run only | `build --profile freestanding` builds the m68k artifact only; `run` compiles the same translation unit on the host with the no-libc layer and executes it | ✅ Resolved |
| AR-21 | Scope | Which frozen-spec artifacts may change | Authorize chapters / docs only | Authorized: `spec/15-profiles.md` (PR-6 reworded, §2 table, §4, §6), `spec/00-introduction.md` §2/§5, `spec/17-codegen.md` §12/§13, `spec/README.md`, `spec/decisions.md`, `spec/14-diagnostics.md` (add the new runtime code), `spec/check.sh` (add the new runtime code to the runtime-fault list), and the test data | ✅ Resolved |
| AR-22 | Scope | Code modification set | Listed files / more / less | `compiler/src/profile.rs`, `compiler/src/driver.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/program.rs`, `compiler/src/stdlib.rs`, `compiler/src/clorraine/{mod,cli,toolchain,build,run,artifacts}.rs`, new `compiler/runtime/shim_freestanding.c`, `compiler/tests/**`, `Makefile`, `AGENTS.md`, `plans/00-roadmap.md` | ✅ Resolved |
| AR-23 | Non-functional | Testing strategy | Spec-first with concrete cases / implementation-first | Spec tests first (concrete input→output), red phase, implementation, green phase, implementation tests, `make check`, then `make verify` | ✅ Resolved |
| AR-24 | Error handling | Rejected module vs allowed float | Compile-time gate / runtime only | A rejected module is a compile-time `E11200`; float types and operations are allowed | ✅ Resolved |
| AR-25 | Deferred | ARM64 profile and 64-bit `usize`/`isize` | Implement now / defer | Deferred to the future ARM64 port. Not implemented by this plan | ⏸ Deferred |
| AR-26 | Deferred | Freestanding heap allocator and the text/format modules | Implement now / defer | Deferred to a later plan. Not implemented by this plan | ⏸ Deferred |
| AR-27 | Technical | The m68k "build-only" claim needs a checkable form | Compile-only / compile and link | The m68k leg compiles **and links**, and the harness asserts the **link command exits 0** (the linker fails on any unresolved symbol, so a clean link proves self-containment; raw `nm` cannot read the Amiga hunk) | ✅ Resolved |
| AR-28 | Behavioral | Runtime "not supported" needs a code | Reuse compile-time `E11200` / add a runtime code | Add the next free runtime fault code `E11214` ("feature not available on the selected profile"); file I/O traps with it. (`E11213` is already the assertion-failure fault) | ✅ Resolved |
| AR-29 | Behavioral | Host-run output may be split across writes | Accept / single write | The runtime writes each message in one call so captured output is atomic | ✅ Resolved |
| AR-30 | Technical | Unknown profile handling after freestanding is implemented | Keep `E11201` / change | Keep `E11201` for an unknown profile name. The obsolete `E11200_unimplemented_profile.clo` golden is removed and **replaced** by a new `E11200` golden for the still-live profile-surface rule (a `host` program importing `std.amiga.dos`), so `E11200` keeps a compile-fail test; `E11201_unknown_profile.clo` is unchanged. The stage expectation for the rejection is removed | ✅ Resolved |

## Resolution Notes

**AR-3:** The specification assigns freestanding's toolchain, entry, and ABI to
LorraineOS (`spec/15-profiles.md:47-57`, `spec/00-introduction.md:129`). Since
LorraineOS does not exist, this project defines a provisional ABI (m68k, 32-bit,
big-endian, no libc) that LorraineOS can later adopt. The user authorized the
matching specification edits (AR-21).

**AR-8:** The host no-libc leg runs only where a hand-written syscall layer
exists (x86_64, measured; aarch64, guarded). Anywhere else the same translation
unit is linked with the platform C runtime so the leg still executes; the
no-libc guarantee is proven independently by the m68k link (AR-27).

**AR-9:** A live probe showed `m68k-amigaos-gcc -nostdlib` links but reports an
undefined `memcpy`; GCC lowers aggregate copies and the always-emitted
`cl_str` helper to `memcpy`/`strlen`. The runtime supplies those, and the build
adds `-fno-builtin` so a `-O2` build cannot turn the runtime's own `memcpy` into
a self-call.

**AR-10:** A live probe found the soft-float helpers (`__adddf3`, `__muldf3`,
`__divdf3`, `__gtdf2`) defined in the SDK's `libg.a`; linking `-lg` produced a
fully resolved binary. `-lm` on the default path did not resolve them, because
the libnix definitions are selected by `-mcrt`. So supporting floats costs one
archive. `libg.a` also holds objects (`malloc`, `exit`) that reference the
AmigaOS global `SysBase`; the m68k runtime must therefore reference none of
libg's libc-ish symbols (its exit is a no-op halt), so only the float objects are
pulled. A runtime that called `_exit` was probed and did **not** link.

**AR-12 / AR-28:** A compile-time rejection of the file functions would need a
profile-scoped FFI contract (the spec's PR-2), a new cross-cutting mechanism.
The user chose the smaller path: the functions exist and trap at runtime. The
trap uses `E11214`, because `E11200` is a compile-time "construct not lowerable"
diagnostic and `E11213` is already the assertion-failure fault.

**AR-18:** The freestanding shim includes no header, so it declares its own
`BPTR`-style handles. Freestanding mirrors the host ABI and declares `BPTR` as
`void *`; the `0` empty state is kept for consistency with the hosted-amiga
provisional ABI, and both `0` and `NULL` are valid for a pointer handle.

**AR-19:** The profile name list lived in two places: `Profile::parse`
(`compiler/src/profile.rs:30-36`) and the driver's string `match`
(`compiler/src/driver.rs:319-337`). Consolidating on `Profile` removes the
duplication that made `freestanding` a special case.

**AR-21 / AR-30:** PR-6 stays as a general rule ("a declared-but-unimplemented
profile is rejected") but loses its `(v1: freestanding)` clause. The obsolete
`spec/tests/compile-fail/E11200_unimplemented_profile.clo` golden is removed and
replaced by an `E11200` golden for a `host` program importing an AmigaOS module,
so the registry's `E11200` keeps a compile-fail test; the freestanding branch of
`stage_spec_test::profile_flag_errors_report_usage_and_codes` becomes a success
expectation; `E11201_unknown_profile.clo` keeps the unknown-name coverage. The
new runtime code `E11214` is registered in Ch 14 and added to the runtime-fault
list in `spec/check.sh`.

## Gate Record

- Systematic scan across all 12 categories completed.
- 30 rows: 28 resolved by an explicit user decision or an agreed improvement;
  2 explicitly deferred (AR-25, AR-26) and excluded from executable plan content.
- No item introduces a material support surface beyond ordinary profile,
  runtime, and driver code. The one broader mechanism considered (a
  profile-scoped FFI contract table) was rejected in favour of the smaller
  runtime-trap path, so the Complexity Escalation Gate is not triggered.
- Header reads `✅ GATE PASSED`.
