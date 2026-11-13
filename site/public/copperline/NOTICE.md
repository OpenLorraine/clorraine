# Copperline browser bundle and demo media

This directory contains a pinned copy of the Copperline browser build used by
the live First Light demo. It is not built by the site; it is copied from the
published Copperline release so the saved machine state and the emulator stay
in step.

| Item | Provenance |
|---|---|
| `pkg/copperline_web.js`, `pkg/copperline_web_bg.wasm` | Copperline **v1.1.0** (`dfb12c45c`) browser build |
| `aros/aros-amiga-m68k-rom.bin`, `aros/aros-amiga-m68k-ext.bin` | AROS m68k ROMs bundled with that Copperline release |
| `aros/LICENSE`, `aros/ACKNOWLEDGEMENTS` | AROS Public License and acknowledgement texts |
| `LICENSE` | Copperline's GNU General Public License v3.0 or later |

- Copperline source: <https://github.com/CopperlineHQ/Copperline>
- Copperline v1.1.0 source tree:
  <https://github.com/CopperlineHQ/Copperline/tree/v1.1.0>
- AROS project: <https://github.com/aros-development-team/AROS>

The browser build is single-threaded and needs no `SharedArrayBuffer` or
cross-origin isolation headers, which is why it works on GitHub Pages.

## First Light demo media

`site/public/demos/first-light.adf` is a bootable ADF containing the release
build of `examples/amiga/first-light.clo` for the `hosted-amiga-13` profile.
`site/public/demos/first-light.clstate` is a Copperline save state captured from
that exact ADF on the A500 machine. The live player loads the state instead of
booting the floppy, so visitors do not wait for AROS startup.

The state is tied to the Copperline build above. If the bundle is updated,
regenerate the state with the same build; do not update one without the other.
The ADF and state are Clorraine output, covered by the Clorraine Output
Exception. The AROS ROM inside the state remains under the AROS Public License.
No Kickstart ROM or Workbench media is included.

## Regenerating

1. Build `examples/amiga/first-light.clo` with
   `clorraine build --release --profile hosted-amiga-13`.
2. Write the binary and a one-line `startup-sequence` (`DF0:first-light`) into
   a fresh ADF with `xdftool`, and install the `boot1x` boot block.
3. Boot the ADF on a Copperline A500 with the pinned bundle, wait for the demo,
   and save the machine state.
4. Copy the ADF and state into `site/public/demos/` and rebuild the site.

The browser state is captured after the demo opens its display. The player
crops the first rows of the AROS screen border and restores the state on every
**Restart**.
