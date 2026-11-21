# Third-Party Notices

Clorraine's first-party code is licensed under the Apache License 2.0, with
the Clorraine Output Exception (see `LICENSE` and `LICENSE-EXCEPTION.md`). The
material below remains under its own license, and those terms take precedence
for the named files.

## Native support libamiga input

`tools/distribution/inputs/libamiga.a` is an unchanged prebuilt target library
from `lib/libamiga.a` in
<https://github.com/AmigaPorts/m68k-amigaos-gcc> at commit
`e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c`.
It is 297,694 bytes, SHA-256
`10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`.

This is not first-party Clorraine code. Clorraine's Apache licence and Output
Exception do not relicense this archive. Its original source/licence chain
has not been established by the current review. The maintainer explicitly
directed its inclusion and retained responsibility for its legalities.
Provenance and the unresolved questions are recorded in
`tools/distribution/inputs/licence-review.md`; this notice is not a permission
or licence-clearance statement. Public release remains separately authorised.

## Native support component review

Future support assets also retain the original terms of GCC, binutils,
libnix and its inherited code, preparation helpers, arithmetic libraries and
the vamos/Python dependency closure. The review records are
`tools/distribution/inputs/licence-review.md`, `build-licences.md` and
`python-licences.md` in that directory. They identify notice/source obligations
and exclusions; they do not replace the original texts in a delivered asset.

LHA has custom source/documentation and commercial-use conditions. Libnix's
public-domain README does not override inherited file-specific notices or
gnuplot-derived source-distribution conditions. Components must remain
separately identified; Clorraine's licence does not relicense them.

## Hamurabi

`examples/amiga/hamurabi.clo` is a Clorraine port of `full/hamurabi.c` from
<https://github.com/michaelcmartin/bumbershoot>. The upstream file is licensed under the MIT
License:

```text
The MIT License (MIT)

Copyright (c) 2015 Michael C. Martin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Picasso96 developer definitions

`compiler/runtime/picasso96.h` is a minimal derivative of the P96 developer
package `P96Develop` 3.6.0 from Aminet (`dev/misc/P96Develop`). The original
P96 API definitions are copyright 1996–1998 Alexander Kneer and Tobias Abt;
the developer package is distributed under the Creative Commons
Attribution 4.0 International license
(<https://creativecommons.org/licenses/by/4.0/>).

The vendored file keeps only the constants, structures, and fixed-argument
library-call macros used by `compiler/runtime/shim_hosted_amiga_optional_rtg.c`.
The full P96 varargs macro forms and the PIP/overlay and palette APIs are
omitted. The vbcc branch uses plain prototypes; the compiler's `amiga.lib` provides the
P96 stubs for the build-only corpus leg.

## Copperline browser build

`site/public/copperline/pkg/` contains an unmodified copy of the Copperline
**v1.1.0** browser build (JavaScript and WebAssembly) used by the live First
Light player. Copperline is copyright its contributors and licensed under the
GNU General Public License, version 3 or later. The full license is in
`site/public/copperline/LICENSE`; the corresponding source is
<https://github.com/CopperlineHQ/Copperline/tree/v1.1.0>.

## AROS ROMs

`site/public/copperline/aros/` contains the AROS m68k ROM images bundled with
that Copperline release. AROS is copyright the AROS Development Team and is
distributed under the AROS Public License. The full text and acknowledgements
are in that directory. The First Light, Boing, and Prism save states under
`site/public/demos/` contain a copy of these ROMs as part of the emulated
machine.
