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

## Demo media

`site/public/demos/` contains bootable ADFs and Copperline save states for the
three live demos:

| Demo | Source | ADF | State |
|---|---|---|---|
| First Light | `examples/amiga/first-light.clo` | `first-light.adf` | `first-light.clstate` |
| Boing: Neon Horizon | `examples/amiga/boing-showcase.clo` | `boing.adf` | `boing.clstate` |
| Prism | `examples/amiga/prism.clo` | `prism.adf` | `prism.clstate` |

Each ADF is a `hosted-amiga-13` release build with the demo and a one-line
`startup-sequence` (`DF0:<program>`). Each state was captured from that exact
ADF on a Copperline A500, so the live player skips the AROS boot. Prism's state
is saved from a running game; the First Light and Boing states are ambient.

The states are tied to the Copperline build above. If the bundle is updated,
regenerate the states with the same build; do not update one without the other.
The ADFs and states are Clorraine output, covered by the Clorraine Output
Exception. The AROS ROM inside each state remains under the AROS Public
License. No Kickstart ROM or Workbench media is included.

## Regenerating

1. Build the demo with
   `clorraine build --release --profile hosted-amiga-13`.
2. Write the binary and a one-line `startup-sequence` (`DF0:<program>`) into a
   fresh ADF with `xdftool`, and install the `boot1x` boot block.
3. Boot the ADF on a Copperline A500 with the pinned bundle, wait for the demo
   (Prism: start a game), and save the machine state.
4. Copy the ADF and state into `site/public/demos/` and rebuild the site.

The browser state is captured after the demo opens its display. The player
trims the small AROS screen-border corner and restores the state on every
**Restart**.
