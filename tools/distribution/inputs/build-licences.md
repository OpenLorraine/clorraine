# Native build components: terms and delivery obligations

This records inspected source declarations at the revisions and archive hashes
in the input catalog. It is not a legal opinion, a publication approval or a
completed audit of binaries that have not yet been built.

## Arithmetic libraries

| Input | Inspected declaration | Required treatment |
|---|---|---|
| GMP 6.2.1 | `README` and `gmp-h.in` offer LGPL-3.0-or-later or GPL-2.0-or-later, or both in parallel. | Preserve the applicable declarations and licence texts. Record whether the compiler incorporates the library or loads a system copy. Retain matching source and build information for conveyed library code. Do not infer that all documentation/build tools use the library's licence. |
| MPFR 4.1.0 | `README` and `src/mpfr.h` declare LGPL-3.0-or-later. `COPYING` is the accompanying GPL text; `COPYING.LESSER` is LGPL version 3. | Retain both texts, original notices, matching source and applicable build/relink information. The README identifies exceptions for several generated autotools files. |
| MPC 1.2.1 | `src/mpc.h` and `src/add.c` declare LGPL-3.0-or-later. | Use the library declarations, not the permissive notice applying to the README file alone. Preserve `COPYING.LESSER`, the referenced GPL text and original notices with applicable source/build information. |

These inputs build host-side compiler dependencies. They are not m68k target
libraries merely because the compiler targets AmigaOS. Static incorporation
does not make their notice/source obligations disappear. Dynamically linked
Ubuntu libraries must be distinguished from the separately pinned source
builds. Actual loader and link records must settle that distinction.

## Maintainer CPython

CPython 3.12.16 builds the maintainer's `cp312` wheel environment. The user's
interpreter remains an Ubuntu prerequisite; the support payload does not
silently include this interpreter.

`LICENSE` contains PSF version 2 and historical BeOpen, CNRI and CWI terms,
plus the documentation-code dual-licence statement. Preserve the complete
file if conveying the interpreter/source, not only a generic PSF label.
`Doc/license.rst` additionally collects incorporated-code acknowledgements,
including sockets, expat, libffi, zlib, libmpdec and other components.
Linked Ubuntu OpenSSL and other system libraries remain separately identified.

The source-bundled ensurepip wheel has its own contents and notices. Its use
is not evidence about a different pip version. Record the bootstrap actually
used by each maintainer environment; see `python-licences.md`.

## LHA

The selected LHA revision is a new candidate, not recovered historical SDK
provenance. `README.md:407–425` refers to `man/lha.man` for the general terms
and identifies file-specific exceptions. It explicitly says that the proposed
different licence for `src/header.c` is undecided and the general terms apply
for now. Do not treat that proposal as a licence grant.

The Japanese distribution section, `man/lha.man:195–256`, says in substance:

| Part | Observed condition |
|---|---|
| 1 | Do not remove copyright notices. |
| 2(a–c) | Preserve the distributed source, documentation and programmer information; document modifications and added value. Binary-only distribution is not permitted. |
| 3 and note | Endeavour to distribute the latest version, expressly not an obligation. Internet distribution is free; magazine/CD-ROM and similar distribution to people without network access requests email before distribution, or afterwards if advance notice is not possible. |
| 4–5 | No damage warranty or obligation to correct defects. |
| 6 | Incorporation into another program is permitted, but the resulting program must not call itself LHa. |
| 7(a–d) | Additional commercial-use conditions prohibit commercial use centred on this program, restrict distribution to unsuitable recipients, address compulsory installation use and responsibility for resulting damage, and require support when used as added value. |

This is a reading aid, not a replacement for the original Japanese text or a
claim that every commercial distribution is permitted. Preserve the original
manual and source/docs with the helper; a bare executable plus a short licence
label is insufficient. Do not market the aggregate as LHa or represent these
custom terms as unrestricted MIT/GPL terms. Public promotion must account for
the actual distribution/use model and these limits. No email was sent.

File exceptions also need preservation:

| Files | Observed terms |
|---|---|
| `src/fnmatch.c`, `src/fnmatch.h` | Berkeley four-clause wording, including advertising acknowledgement and non-endorsement. Preserve the original text; do not silently delete its third clause. |
| `src/getopt_long.c`, `src/getopt_long.h` | Koji Arai's MIT-style permission and disclaimer. |
| `src/vsnprintf.c` | Berkeley wording followed by an explicit note that advertising paragraph 3 is null and void; retain the note with the text. |

Whether a fallback file enters the binary depends on the actual configure
result. Conveying its source still requires preserving its source notice.

## Libnix and related target headers

Libnix's root README says public domain, but the selected tree has inherited
file-specific terms. A root label is not a complete review:

| Selected paths | Treatment |
|---|---|
| `sources/headers/poll.h`, `sources/socket/extra/poll.c` | Brian Clapper's BSD-style source/binary notice and non-endorsement conditions. |
| `sources/math/common/`, `sources/math/mathfp/` | Preserve Sun/fdlibm permission notices and ARM BSD-style notices where present; do not describe all math code as public domain. |
| `sources/socket/arpa/` | Retain applicable Berkeley notices, including advertising wording where present. |
| `sources/nix/extra/asprintf.c` | Preserve Darren Tucker/Todd Miller permission and disclaimer. |
| `sources/nix/extra/pipeoc.c` | Inherited gnuplot terms permit modifications but require distributing changes as patches to released source, special version identification, a support contact and retained upstream contact information. A tarball containing only the complete modified file is not the stated distribution form. |

The `pipeoc.c` issue applies to source packaging even if the current build
preprocesses its implementation away. Its body is guarded by
`#ifndef __KICK13__`. The existing SDK defines `__KICK13__` for `-mcrt=nix13`;
its ordinary `libnix.a` has `pipeoc.o` but no `popen`/`pclose` symbols.
That observation is not proof about a fresh build or every multilib variant.

A bounded source search found a released gnuplot 3.7.1 `amiga.c` with the same
copyright/permission block and pipe implementation, at repository revision
`938a1c42aca589b9ae7c1a526faacbc42eca67fc`. This is a candidate released-source
baseline, not a proved exact historical import. Before conveying the affected
source or implementation, assemble an original-plus-patch representation that
preserves the required identification and contact information. A scratch proof
generated a unified patch against that released `amiga.c`, applied it with
zero fuzz and reproduced the pinned `pipeoc.c` bytes exactly. That verifies
reconstruction, not the finished source package or its required support
identification. Do not infer a waiver from libnix's public-domain README.
The separate libamiga inclusion direction does not cover this file.

`source-forms/` now retains that exact released-source candidate and a
zero-fuzz reconstruction patch as individually hashed inputs for the final
lock. Its README preserves the distribution-specific identification and points
support to this project's issue tracker. The later source assembler must
convey those files together rather than only the modified source file.
This is the previously reviewed source representation, not an executable
dependency or a substituted build-source revision.

## Preparation tools and networking overlay

| Source | Inspected terms / boundary |
|---|---|
| fd2sfd | `COPYING` supplies GNU terms; `README.md` identifies its fd2inline ancestry. Keep the origin note requested in `fd2inline.c`, source and original notices. Copied `cross/share/` headers remain separate installed files to trace. |
| sfdc | `main.pl` declares GPL-2.0-or-later. Preserve the source, component files, notices and the recipe concatenating them. `Makefile.in` creates a Perl script, not an ELF executable. Verify Perl and module prerequisites rather than assuming an ELF loader audit covers it. |
| fd2pragma | `fd2pragma.readme` declares public domain and disclaims warranty. Preserve it; trace the separately copied `Include/inline/` headers too. |
| AROS pthreads | `pthread.c/.h`, `sched.c/.h`, `semaphore.c/.h` and `debug.h` have origin/alteration/notice restrictions with a grant including commercial use. Only the pthread subtree is selected; other AROS code does not inherit that grant. |
| amiga-netinclude | Roadshow headers say “Freely Distributable”; that wording is in headers, not the two-line root README. Additional Berkeley, DEC and Darren Reed notices apply to particular files. Preserve all applicable notices and do not relicense them under the project's Apache licence. |

In particular, netinclude's `include/netinet/ip_fil.h` and `ip_nat.h` preserve
Darren Reed's credit and unchanged-distribution-terms condition. That text
expressly rejects simply putting the code under another licence, including
the GPL. Keep this component separately identified when assembling the
aggregate; do not label the entire support tree GPL.

## Selected inspected identities

Paths are relative to their source roots. The hashes identify evidence, not
publication approval.

| Source / path | SHA-256 |
|---|---|
| GMP `README` | `5e9f9325fd702bc4bcda27d7a78fea88a2a09fa39b4b15ac7b9b205e0863dc7e` |
| GMP `gmp-h.in` | `8cebcb6af2c79f7b49949b334e4b149a58798ebd7aa3281a652906979b237945` |
| MPFR `src/mpfr.h` | `d800395eec628225e0c5ebd4d4bf21097e67e88e153af4e7a9e89a37c7befc58` |
| MPC `src/mpc.h` | `0fdaa2d9d875193b6797aba9ba2b6acabf24c6321467c624b9ca7468a613ef5a` |
| CPython `LICENSE` | `3b2f81fe21d181c499c59a256c8e1968455d6689d269aa85373bfb6af41da3bf` |
| CPython `Doc/license.rst` | `341832873fd316a37927e79385093fbbfd40a467428480835fe435a80cadf4e5` |
| LHA `README.md` | `6de18f20a0d6b09798b6318be634ba8520643ea6ee35241d4549753df00cfffb` |
| LHA `man/lha.man` | `0fc54b8739b12dc1f9ac95476929ce0c67e372dba9eb4e66b9574fb7ddd254b2` |
| LHA `src/fnmatch.c` | `c41e0013c0dac514c9329557533b31f9514d67af734c4cd03696acd024bf4e4b` |
| LHA `src/vsnprintf.c` | `8b821ff7baad5783f4fe38cf9b79d4e57c39d52240e907b40d7f110da69c64f4` |
| Libnix `sources/nix/extra/pipeoc.c` | `b29a4daf8b5be0090449de5cd5eec1007261dbcf01e0ae0ef138cd43a07de970` |
| Libnix `sources/math/common/sf_log.c` | `67ad7e2ff508d52a20b3270024cb6033c686c6ed1eb6f01eccd65808190d8648` |
| sfdc `Makefile.in` | `6f28947ab4b0311642cb9e323f07677fae049fd32f8506ded21578fb4ca6b6ea` |
| sfdc `main.pl` | `fc9acbd443dc1932b276addd667ec01d44321d75576b979e639b83df80a73eab` |
| Netinclude `include/netinet/ip_fil.h` | `94f29aba2bacccffd4e7a48d06a2c3d9a92141ed01c5ad99ab48719a7afa65cd` |
| gnuplot 3.7.1 `amiga.c` | `5154776f0eb250ed4087ba082f46d9fd2de8074bbf90d04ba8ea066760894c12` |

The released-source candidate was read from
<https://github.com/gnuplot/gnuplot/tree/938a1c42aca589b9ae7c1a526faacbc42eca67fc>.
It is research evidence, not an added executable build dependency.
