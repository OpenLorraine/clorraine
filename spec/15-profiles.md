# Chapter 15 - Platform Profiles

> **Status:** planned
> **Depends on:** Ch 00 (axioms), Ch 12 (contracts), Ch 17 (codegen)

## Scope (planned)

A profile is a machine-readable bundle of target facts; the core language
never contains them (A5).

| Profile fact | `hosted-amiga` (v1) | `host` (tests) | `freestanding` (later) |
|---|---|---|---|
| Compiler | `m68k-amigaos-gcc`, `vc +aos68k` | host `gcc`/`clang` | toolchain TBD with LorraineOS |
| Runtime | libnix `-mcrt=nix20` | host shim | LorraineOS entry points |
| Default CPU | `-m68000` (opt-in `-m68020`) | native | m68k later ARM64 |
| Endianness | big | little | profile-defined |
| Pointer size | 32-bit | host | 64-bit on ARM64 |
| Entry | `main()` exported to C | `main()` | startup defined by LorraineOS |
| Package | hunk executable (dir drive/ADF) | host binary | LorraineOS image |

Rules:

* *PR-1* All target facts live in the profile; source files compile unchanged
  across profiles that support the features they use.
* *PR-2* Contracts are profile-scoped (Ch 12 FFI-5).
* *PR-3* The compiler rejects a construct that the selected profile cannot
  lower (E112xx block).
* *PR-4* Profiles are versioned with the compiler; the SDK image records the
  active profile in its provenance.

## Open questions

| Question | Recommendation |
|---|---|
| Profile file format | Compiled-in tables first; data files later |
| Cross-profile testing | `host` profile runs on every commit; `hosted-amiga` in the emulator CI |
| ARM64 profile | Design-only now; add when LorraineOS needs it |
