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

## 4. Advanced Workbench services use licensed media

The advanced and optional-tier gates install the licensed Amiga Forever
Workbench media from the local `~/amiga` tree. No media or decrypted ROM is
committed. The gates are headless and use private Xvfb displays; they still
need the media present on the machine running `make verify`.

The Bundle 9 services are runtime-proven at these baselines:

| Service | Baseline | Runtime proof |
|---|---|---|
| Commodities | A600/KS2.05 | Broker open, duplicate rejection, close, reopen |
| Locale | A600/KS2.05 | Default locale language, country, GMT, format |
| ARexx | A600/KS2.05 | Typed message create/send/receive/reply/reclaim |
| Clipboard | A600/KS2.05 | Whole-clip write and `CMD_UPDATE` |
| Serial/parallel/printer | A600/KS2.05 | Typed open and close; data paths are impl-tested |
| Nonvolatile | A600/KS3.1 | 23-byte store/get/delete round-trip |

## 5. Optional-service limits

These are deliberate boundaries, not hidden gaps:

- **BOOPSI** uses the native Intuition `strgclass` (2.0+). The licensed 3.0/3.1
  `colorwheel.gadget` loads but its color attributes did not round-trip in
  probes, so the module does not claim them. The runtime gate runs on A1200/KS3.0.
- **Clipboard read** is implemented and pinned, but the gate proves only the
  write/update path; the 2.05 clipboard device returned no data on read in the
  probe environment.
- **Serial and parallel I/O** are typed and bounded, but the runtime gate proves
  open/close only; their data paths are implementation-tested. **Printer**
  proves open/close only and does not claim output or a `PRD_RAWWRITE` result
  because no driver/prefs sink is configured.
- **Datatypes** proves the V40 object opens and parses the 320x200x4 ILBM
  (width, height, depth, colors). Its `DTM_DRAW` did not render visibly on the
  custom screen, so no visual capture is claimed. The licensed 3.1
  `ilbm.datatype` leaves `PDTA_NumColors` at zero, so the shim falls back to
  `1 << depth`; the gate cannot distinguish a 16-color palette from the
  depth-derived fallback. `GetDTAttrsA` writes through a `ULONG` slot even
  though the attribute is documented as a `UWORD` value; a `UWORD` destination
  corrupts the adjacent stack on this class.
- **Network** proves a TCP echo through `tcpEcho`. The typed split
  `openSocket`/`connectSocket`/`sendSocket` path is compile- and impl-tested and
  forwards caller data, but the FS-UAE emulated library's split descriptor path
  did not reach the host peer in probes. Real hardware needs a third-party
  TCP/IP stack. `std.amiga.bsdsocket` is GCC-only: vbcc cannot parse the NDK
  socket headers, so the socket half of the shim is omitted from vbcc builds.
- **AmigaGuide** is a 3.0+ disk library; the runtime gate runs on A1200/KS3.1.
  It compiles under both m68k GCC and vbcc.
- **Nonvolatile** uses an `nv_location` file pointing at `CNEXT:nonvolatile`.
  `StoreNV` length is in ten-byte units, so a 23-byte payload reads back as a
  padded 30-byte record.
- **ARexx** copies the extension and host strings for the packet lifetime and
  deletes each owned argument string directly; it does not call
  `ClearRexxMsg`, which crashed after the reply on the licensed 2.05 library.
  The reply port must stay open until the reply is reclaimed.
- **m68k shim memory safety** cannot use host AddressSanitizer. The optional
  shim is reviewed statically, with host record models and m68k runtime gates;
  `make asan` does not instrument the new m68k-only code.

## 6. RTG is Picasso96 on UAEGFX and emulated Picasso IV

`std.amiga.picasso96` is a 24- or 32-bit truecolor PC-RTG surface for
Picasso96 2.0. Two gates prove it: A1200/KS3.1 with FS-UAE’s `uaegfx` card
(32-bit true alpha) and A4000/KS3.1 with the emulated Picasso IV Zorro III
card (24-bit truecolor, the format the real card exposes). It is not a generic
RTG abstraction: CyberGraphX/CGX4, physical Picasso II/IV or CyberVision
drivers, PIP/video overlay, CLUT palette editing, hardware sprites, and RTG
text/font rendering are out of scope. CGX4 is explicitly dropped as a product
row because it is commercial, end-of-life, and requires media that is not part
of this project. Both gates check a PPM capture rather than a native CSCR,
because an RTG screen is chunky/truecolor. The P96 archive is user-provided
and verified by SHA-256; it is not committed. The emulated Picasso IV gate
also needs the user-provided 7.4 flash ROM (SHA-1 verified, not committed) and
attaches the supplied monitor settings to the board before the monitor starts.
The bitmap fill uses a bounded pixel loop because `p96RectFill` did not
round-trip pure blue on a user-private bitmap in the spike; screen fills still
use `p96RectFill`.

## 7. Runtime and generated C symbol collisions

The compiler mangles a user definition to `cl_<module>_<name>`. The runtime
shim and some generated helpers use the same `cl_` namespace, so a legal source
name could emit two conflicting C definitions. The emitter now checks every
user function and global against the file-scope names the selected profile
shim defines and against the generated helpers (`cl_init_*`,
`cl_zero_ClSlice_*`, and `cl_lambda_*`). A colliding declaration gets a
deterministic `__user` suffix, and the suffix grows until it misses every other
user spelling. Non-colliding declarations keep their ordinary symbol, so the
change only affects names that previously failed the strict C build.

| Source shape | Ordinary symbol | Emitted symbol |
|---|---|---|
| `module socket; func open` | `cl_socket_open` | `cl_socket_open__user` |
| `module zero; func ClSlice_u8` | `cl_zero_ClSlice_u8` | `cl_zero_ClSlice_u8__user` |
| `module init; func foo` beside module `foo` | `cl_init_foo` | `cl_init_foo__user` |

An explicit `ffi` name is a literal C symbol and is never renamed. Re-declaring
a public standard-library prototype with the same shape is allowed; a different
shape, or a binding to an internal shim helper, is rejected with `E11101` at
the declaration. The regression tests emit and cross-build the `socket.open`
and `zero.ClSlice_u8` shapes, and a unit test pins the `init.foo` rename.

## 8. CI is host-only

The GitHub Actions workflow runs `./spec/check.sh` and `cargo test --workspace` on an Ubuntu
runner. Tests that need `m68k-amigaos-gcc`, `vamos`, FS-UAE, an X display, or Kickstart ROMs skip
themselves when the toolchain is absent. `make verify` remains the local acceptance gate that
exercises the Amiga and freestanding legs.
