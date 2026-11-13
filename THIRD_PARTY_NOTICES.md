# Third-Party Notices

Clorraine's first-party code is licensed under the Apache License 2.0, with
the Clorraine Output Exception (see `LICENSE` and `LICENSE-EXCEPTION.md`). The
material below remains under its own license, and those terms take precedence
for the named files.

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
