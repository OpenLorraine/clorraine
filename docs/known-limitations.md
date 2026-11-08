# Known Limitations (v1.1)

This document records the accepted limitations of Clorraine v1.0 and v1.1. It is deliberately short: a new
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

## 8. Compiler edge cases outside the robustness closeout

The robustness closeout fixed the six recorded latent defects and the enum
conversion path. The following accepted-source edge cases remain. They are
pre-existing, are not regressions from the closeout, and have no shared
workaround beyond the notes here.

| Edge case | Current behavior | Guidance |
|---|---|---|
| An imported enum or handle type name used as a cast (`import { E }`; `E(2)`, `Buffer(0)`) | The resolver treats the import as a value, so the call is emitted as a C function call. It can fail the strict C build with an implicit declaration. | Convert the value explicitly to the backing scalar, or declare the conversion in the same module. |
| A type-name call with zero or more than one argument, explicit type arguments, or a call-site marker (`E()`, `E(1, 2)`, `E<T>(2)`, `E(borrow x)`) | Only the exact `T(expr)` form is a cast. Any other form stays on the call path and fails the strict build with an implicit declaration. | Pass exactly one unmarked expression and no type arguments. |
| A non-integer explicit enum member (`A = true`, `A = 1.5`, `A = "x"`) | The declaration is accepted and the member is emitted as its ordinal; an auto member after it is accepted unchecked. | Use an integer constant expression; cast a bool with `u8(...)` and reject a float or string at the source level. |
| A runtime integer converted to a handle (`Box(n)`) | The host 64-bit strict build reports `-Werror=int-to-pointer-cast`; the m68k build accepts it. | Use the handle's constructor or a constant address; the host strict build is the acceptance signal. |
| A module `const` that does not fold in the body used as a `case` label | The label emits the mangled symbol and the strict build rejects it as a non-constant case label. A const that folds in the body is accepted. | Use a literal, a const that folds in the body, or a chain of `if` tests. |
| A constant `MIN / -1` or `MIN % -1` | The guard traps first, but the C still contains the undefined operation and the strict build may reject it. | Use a non-constant dividend or a guard that the compiler can fold; the runtime trap is the defined behavior. |
| A statement pure conditional fill | The value expression, including its condition, is repeated for each element; a volatile condition is read once per element. | Use a temporary condition or a loop when the condition has a side effect. |
| A module fill that covers no slot with an impure or struct value | The prepared local is not read; the strict build reports an unused variable. | Ensure the prefix covers at least one slot, or use a statement fill. |
| A fixed-array row-place field | The row bounds guard is emitted twice. | No action; the first guard traps before the access. |

The enum member policy uses the target ABI layout for `sizeof`, so an enum
macro agrees with the runtime C `sizeof`. A value-position `const` that reads
`sizeof` still stays a runtime place; only a constant context folds it.

## 9. CI is host-only

The GitHub Actions workflow runs `./spec/check.sh` and `cargo test --workspace` on an Ubuntu
runner. Tests that need `m68k-amigaos-gcc`, `vamos`, FS-UAE, an X display, or Kickstart ROMs skip
themselves when the toolchain is absent. `make verify` remains the local acceptance gate that
exercises the Amiga and freestanding legs.

## 10. Benchmark instruction counts are guest CPU counts, not cycle counts

`make bench` measures the m68k dynamic instruction count with a streamed
`vamos --instr-trace` run and subtracts a calibration program's fixed startup
count. This measures guest CPU instructions, not bus cycles, memory stalls, or
Amiga library internals that `vamos` emulates in Python. FS-UAE 3.1.66 has no
stable headless cycle counter, so a cycle-accurate comparison is not part of
the gate. Host wall time is recorded and displayed but is advisory: its
run-to-run spread is often tens of percent, while the instruction and
section-size metrics are deterministic.

## 11. v1.1 surface limits

These limits belong to the v1.1 `Box<T>`, ordinary `freedBy` deleter, and
C-hook trampoline surfaces.

| Area | Limitation | Workaround |
|---|---|---|
| `Box<T>` allocation | An allocation failure uses the `E1` trap; `Box<T>` itself is not nullable. `Box<T>?` is an optional slot, not an allocation result. | Check an allocation through a nullable API before boxing, or accept the trap. |
| `Box<T>` moves | A boxed value cannot be moved out of its box. | Use `boxSet` to replace it or `boxWith` to lend it. |
| `Box<T>` FFI | A direct Box FFI crossing is `E10105`. | Use `boxWith` or a concrete C-facing struct. |
| `Box<T>` freestanding | The freestanding profile has no heap and rejects Box (`E11200`). | Use `host` or `hosted-amiga`, or a plain value/raw pointer. |
| `Box<T>` name | A user type declaration named `Box` shadows the built-in. | Use a different name. |
| `Box<str>` | Only `static str` elements are accepted; a plain `str` element is `E10227`. | Box an owner or use a `static str` literal. |
| Ordinary deleters | An ordinary `freedBy` deleter is resolved module-scoped; a cross-module candidate must be `export`ed. v1.1 accepts only a **top-level `ffi` `take` release** in the deleter body; ordinary forwarding (`relay(w)`), borrow-only calls, releases nested in `if`/loop/`defer`/block, and dot-call releases (`w.release()`) are `E10102`. | Declare the deleter in the handle's module, export a cross-module one, and call an `ffi` release declared `take` in a direct expression statement. |
| Hook lifetime | A C hook is synchronous. The environment is a caller-stack node, never a heap allocation; a C callee that stores the callback violates the contract. | Keep the callback inside the C call; the no-live-node case faults `E11215`. |
| Hook after-return aliasing | A stored capturing callback invoked during a later invocation of the same lambda site finds the newer environment because one plain C function pointer identifies the site. | Do not store callbacks across calls; a strict per-invocation token is not part of v1.1. |
| Hook generics | A capturing hook inside a generic function body is `E10400`; a per-instance trampoline is not part of v1.1. | Move the hook into a non-generic function. |
| Hook signature set | Views, slices, fixed arrays, borrowed move-only structs, and fixed-array returns are `E10210`; a function-pointer callback return is also rejected because the `ffi` prototype cannot spell it. | Use scalar, `bool`, enum, `raw`, `cstring`, handle, non-generic struct, or function-pointer parameters; return a scalar, enum, `raw`, handle, or non-generic struct. |
| Hook capture/move | A capturing hook that borrows a binding the same call moves is `E10030`. | Copy the value into the callback before the call, or move the callback before the `take`. |
| Hook symbol names | Generated `cl_hooktramp_*`, `cl_hookwrap_*`, and `cl_hook_env_*` names are not reserved against user `ffi` declarations. | Avoid declaring an `ffi` symbol with one of those prefixes. |
| Box helper names | Generated `cl_box_alloc__<key>` and `cl_box_drop__<key>` names are not reserved against user `ffi` declarations. | Avoid declaring an `ffi` symbol with those prefixes. |
| Box in a C-facing struct | `E10105` rejects a Box at the top of an FFI type (and inside raw/array/function types), but a non-generic struct with a Box field crosses by value; the C side then owns a Box it cannot release. | Keep Box fields out of structs that cross the FFI boundary. |
| `boxWith` capture-lend | The callable-bound `boxWith` path shares the pre-existing borrow-checker hole where a callback borrows a box and mutates it through another capture during the call; the read can use freed storage. This is the same pre-existing capture-lend defect recorded in the v1.1 findings ledger, not a new Box rule. | Do not mutate a box through one capture while another capture reads it during the same callback. |

## 12. DX tooling limits

| Area | Limitation | Workaround |
|---|---|---|
| LSP navigation | Rename, formatting, references, and workspace symbols are not implemented. | Use the implemented hover, definition, completion, and document-symbol features. |
| LSP workspace | The index reads at most 2000 `*.clo` files, and one request analyzes at most 256 files. | Open the file directly or keep unrelated programs in separate roots. |
| LSP analysis | A file is parsed and type-checked on each request; there is no incremental cache. | Small workspaces are the intended scope. |
| Installer integrity | The installer downloads nothing itself. The pinned image build verifies upstream archives where upstream publishes SHA-256 values; the installer's own gate is provenance, NDK presence, and tool versions. | Inspect `docker/versions.env` and `/opt/amiga/BUILD-PROVENANCE.txt`; rebuild with `make setup FETCH=1 YES=1`. |
| VS Code tests | The extension tests need Node.js 22+ on the host and are not part of `make check` or the host CI job. | Run `make vscode-check`. |
| `--doc` | It documents top-level declarations only; struct fields, enum members, and nested items have no separate entries. Related links resolve a type through the referring module’s imports, then the first input module that declares it; an aliased or qualified reference to a same-named type in another module can point to the wrong page. | Read the source for member-level detail; use unambiguous type names across input modules. |
| Release packaging | The v1.1.0 binary tarball contains `clc` and `clorraine`, not `clorraine-lsp`. | Build the server from source with `cargo build --workspace`. |
