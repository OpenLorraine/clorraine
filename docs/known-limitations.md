# Known Limitations (v1.0)

This document records the accepted limitations of Clorraine v1.0. It is deliberately short: a new
accepted limitation must be added here and to the production-readiness ambiguity register before
release.

## 1. AddressSanitizer child programs need a non-PIE link in the SDK image

The SDK image's GCC 12 AddressSanitizer runtime intermittently fails at startup for PIE
executables. `make asan` handles this automatically: it installs a temporary `cc` wrapper that
appends `-no-pie` to every AddressSanitizer child link, so the suites run without manual flags.
A direct AddressSanitizer build of a compiler-generated program outside `make asan` must pass
`-no-pie` if it hits the startup failure. This is a build-environment requirement, not a language
or code-generation defect.

## 2. The GCC 15 loop-condition split covers only top-level `while` `&&`

Host GCC 15.2 can report a false `-Werror=sequence-point` diagnostic for a loop condition whose
place-borrowing receiver is a call or a conditional. The compiler splits a top-level `while`
condition joined by `&&`: the right side becomes two tests, a boolean local freezes the arm
condition, and the arm fill runs before the callable designator. The split does not cover `for`,
`do-while`, or conditions whose operands are calls or re-associated; those can still show the
base-identical GCC 15 warning. GCC 12.2, the version in the SDK image, accepts the same C. This is
a build-quality limitation, not a semantic difference: the emitted C remains valid C99 and the
program's behavior is unchanged.

## 3. CI is host-only

The GitHub Actions workflow runs `./spec/check.sh` and `cargo test --workspace` on an Ubuntu
runner. Tests that need `m68k-amigaos-gcc`, `vamos`, FS-UAE, an X display, or Kickstart ROMs skip
themselves when the toolchain is absent. `make verify` remains the local acceptance gate that
exercises the Amiga and freestanding legs.
