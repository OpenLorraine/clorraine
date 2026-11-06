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

## 3. Exec worker tasks have no DOS process context

A task created by `std.amiga.exec.createTask` is a plain Kickstart 1.3 Exec
task, not a DOS process. Its entry must use only arithmetic, module-level
scalars, and `reportTaskResult`; it must not call DOS, Intuition, `std.io`,
allocate, drop an owning value, or block. Use a message port plus a DOS process
for work that needs those services; the language has no process wrapper in v1.
The compiler cannot enforce this convention, so it is audited and documented in
`spec/appendix-stdlib.md` §13.1.

## 4. Advanced Workbench services need Workbench media

The advanced-tier acceptance baseline has licensed Kickstart ROMs but no
Workbench ADF/HDF. `commodities.library`, `locale.library`,
`rexxsyslib.library`, `clipboard.device`, `serial.device`, `parallel.device`,
`printer.device`, and `nonvolatile.library` are on Workbench media, not in the
ROMs, so Bundle 8 defers those capabilities. The ROM-resident Workbench, icon,
expansion, card, and keymap services are implemented and gated on A1200/KS3.1
and A600/KS2.05. The ROM default keymap maps the raw keys the services demo
checks; a Workbench installation with additional keymaps can map more.

## 5. CI is host-only

The GitHub Actions workflow runs `./spec/check.sh` and `cargo test --workspace` on an Ubuntu
runner. Tests that need `m68k-amigaos-gcc`, `vamos`, FS-UAE, an X display, or Kickstart ROMs skip
themselves when the toolchain is absent. `make verify` remains the local acceptance gate that
exercises the Amiga and freestanding legs.
