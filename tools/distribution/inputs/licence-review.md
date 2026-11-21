# Native support source and licence review

**Status: incomplete. The maintainer directed inclusion of the exact ordinary
libamiga archive and retained responsibility for its legal review. No public
release is authorised by that instruction.**

This records evidence for the fixed source cohort in `README.md`. It does not
grant rights, replace upstream terms or establish that every file in a future
payload has been reviewed. The final file-based origin audit must connect each
shipped file to its applicable terms, notices and source obligations.

## Required archive: unresolved provenance and source obligations

The pinned `m68k-amigaos-gcc` build copies a prebuilt archive:

- `Makefile:1555–1568` defines and installs `lib/libamiga.a`.
- The libnix target depends on that archive at `Makefile:1579`.
- The compile matrix links Amiga programs with `-lamiga`; dropping the archive
  is not a compatibility-preserving way to close this review.

| Upstream file | Bytes | SHA-256 |
|---|---:|---|
| `lib/libamiga.a` | 297694 | `10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021` |
| `lib/libb/libamiga.a` | 296032 | `cf2d16f40d850c4d7682c44c9dc4299aa901a3ca89e704f68cd47c8c9c412982` |

The existing SDK's ordinary `m68k-amigaos/lib/libamiga.a` matches the first
hash exactly. This establishes technical identity, not redistribution rights.
The ordinary archive has 1,149 members.

### Bounded primary-evidence search

Both archive histories reachable from build commit
`e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c` contain one introducing commit:
[`9ec400774c517fb67d65ad4ad8bdbdd94e715d8c`](https://github.com/AmigaPorts/m68k-amigaos-gcc/commit/9ec400774c517fb67d65ad4ad8bdbdd94e715d8c).
Its one-line message is “add libamiga, fix sys-includes”. Its changed paths
are `Makefile` and the two binary archives. It contains no source or generator
reference in the commit message.

The build repository's complete recursive tree has 142 entries. The `lib/`
subtree contains the two archives, with no accompanying source or notice.
The pinned libnix tree has 886 entries. Its README says public domain, but that
does not establish ownership or terms for these separately copied binaries.
The inspected `Makefile.gcc6`, `mkstub` and `preplib` contain no `libamiga`
reference. This search did **not** prove that matching underlying source cannot
exist elsewhere.

The build repository carries GPL version 2 text, but its
`COPYING-THIRD-PARTY.md` preserves original component copyrights and terms.
Its aggregate GNU version labels are not reliable substitutes for current
per-component notices. Neither that overview nor libnix's README establishes
the archive-specific source/permission chain.

A commented Makefile command would copy the NDK's `amiga.lib`. That comment
and similar function names are provenance leads, not proof that either archive
was derived from that file. Do not label the archives as NDK-derived without
evidence, and do not assume that lack of a located notice means no licence
exists.

### Maintainer inclusion direction

After reviewing the provenance gap, the maintainer explicitly directed:
“copy the file, include it. I will deal with the legalities myself.”
The exact ordinary archive is therefore included at `inputs/libamiga.a` and
selected for the SDK payload. This lifts the local implementation hold for
that file only; it does not assert a licence grant or matching source that has
not been established. The base-relative archive is not included by this
direction.

The maintainer owns the remaining legal review. Other components retain their
normal source/notice requirements. Public promotion remains a separate action
requiring explicit approval; no upstream message or publication is implied.

### Outstanding source/terms evidence

Connect the exact ordinary archive, and any base-relative archive actually
selected for shipping, to:

1. Its origin and applicable redistribution terms.
2. Required notices and obligations.
3. Corresponding source and build inputs where those terms require them.

An upstream maintainer/rightsholder can supply source, a generator/build route
and its original inputs, or a rights-backed provenance explanation. A similar
source tree, unsupported assurance, root project label or user approval is not
a substitute for that evidence.

Suggested upstream question:

> For the `lib/libamiga.a` archive at commit
> `e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c` (SHA-256
> `10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`),
> what are its original source/provenance, applicable redistribution terms,
> and matching source/build procedure? The reachable introducing commit adds
> the binary without that information. We need to establish the notice and
> source obligations for its inclusion in a reusable compiler support asset.

No upstream issue or message has been sent as part of this review.

## Other inspected declarations

These are preliminary observations, not complete component approvals.

| Component | Inspected source paths | Observation / remaining work |
|---|---|---|
| Build rules | `COPYING`, `COPYING-THIRD-PARTY.md` | GPL-2.0 text and component-specific reservations; preserve build scripts and applicable original terms. Do not extend the overview to unknown binary origins. |
| GCC | `COPYING3`, `COPYING.RUNTIME` | GPL-3.0 text and GCC Runtime Library Exception. Preserve the actual compiler/runtime notices and corresponding source; identify terms for each selected installed file. |
| Binutils | `COPYING*`, `README` | Multiple GNU and runtime-related notices; determine which apply to the selected programs/libraries rather than relying on the build overview's older GPL-2 label. |
| Newlib | `COPYING*`, `README` | Mixed component notices. The exact target include/library subset and per-file obligations still need review. |
| Libnix | `README.md`, build rules and inherited source/header notices | Public-domain README does not override Sun/ARM/Berkeley and other inherited notices. `pipeoc.c` has original-plus-patch distribution conditions; see `build-licences.md`. Installed-member/source-form proof remains pending. |
| fd2pragma | `fd2pragma.readme` | Explicit public-domain declaration and no-warranty text. Preserve the declaration; inspect copied helper headers separately. |
| fd2sfd | `COPYING`, `README.md` | GNU licence text; retain notices and corresponding source, including origins inherited from fd2inline. |
| sfdc | `COPYING`, `sfdc.readme`, `main.pl`, `Makefile.in` | GPL-2.0-or-later source; the installed helper is a concatenated Perl script. Retain source/notices and measure interpreter/module prerequisites. |
| AROS pthreads | `pthreads/pthread.c` | Explicit permissive grant including commercial use, with origin/alteration/notice restrictions. Review all selected pthread source/header files. |
| amiga-netinclude | `README.md`, header tree including `libraries/bsdsocket.h`, `netinet/in.h` and `netinet/ip_fil.h` | Roadshow “Freely Distributable” wording is in headers, not the minimal README. Preserve inherited Berkeley/DEC/Darren Reed terms and unchanged-licence conditions. Do not apply one blanket label. |
| amitools | `README.md`, `pyproject.toml` | Metadata specifies GPL-2.0-or-later. Preserve the applicable text/notices and full matching source; inspect wheel contents. |
| LHA | `README.md:407–425`, `man/lha.man:195–256`, exception files | Custom terms require source/docs, forbid binary-only delivery and constrain commercial use. File-specific notices also apply. Full findings and delivery limits are recorded in `build-licences.md`; no unrestricted-use claim is made. |
| Python runtime/backend archives | Source identities/hashes and inspected notice bodies | Source declarations, vendored notices and packaging obligations are recorded in `python-licences.md`. Actual wheel-content/source assembly must still be audited. |
| Arithmetic libraries and maintainer CPython source | Exact archives, library headers, root notices and CPython incorporated-code acknowledgements | Declarations and notice/source boundaries recorded in `build-licences.md`. Actual linkage and conveyed-file checks remain required. |

The [Python notice review](python-licences.md) records the source declarations
and nested-notice cases separately. It is not a substitute for the real
wheel-content and corresponding-source checks.

The [build-component review](build-licences.md) records LHA's full distribution
section, arithmetic declarations, maintainer CPython notices and inherited
target-library terms. The [origin review](payload-origins.md) traces concrete
install rules and a bounded existing-SDK header comparison. Neither document
claims that a fresh reference inventory or final payload has passed.

### Selected evidence hashes

Hashes identify inspected bytes, not legal approval.

| Source path | SHA-256 |
|---|---|
| Build repository `COPYING-THIRD-PARTY.md` | `62e84db486fbd8c12d3b2e96855dd6bcc5e76a9a657c7980da24bde0c133b083` |
| GCC `COPYING3` | `8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903` |
| GCC `COPYING.RUNTIME` | `9d6b43ce4d8de0c878bf16b54d8e7a10d9bd42b75178153e3af6a815bdc90f74` |
| Libnix `README.md` | `8c32c65b098b8f5efb925b49943325cbc332a5126f813ce97a17cf29ca081900` |
| fd2pragma `fd2pragma.readme` | `0cb6f19392eb196bc6f016185226b9915d66d16001e3fb62c8b100faff2ed789` |
| fd2sfd `COPYING` | `1ca3e8f71b7de618968e7ea9fe47822d9e704dc476aef8544033abedec355225` |
| sfdc `COPYING` | `9e171c0524f22317d9d6dd74c1f41aabf7103fb210d0e5264f028337ca478432` |
| AROS `pthreads/pthread.c` | `5700209576773324c6fadd317c57f5e7a47d31e8c6a2e867323ee2299fa6bedf` |
| amitools `pyproject.toml` | `3be68d2a30f0adc72ea253df69ba6adc646966cc29c49696428832cc8705e525` |

## NDK boundary

The original NDK archive was rechecked against its locked length/hash.
Its `ReadMe-NDK.txt` and `lib/README` describe the development files and link
libraries, but the inspected text does not establish a redistribution grant
for the prebuilt archives above. The original NDK and modified header trees
remain excluded from support distribution.

Only literal listing/notice output was inspected with the existing SDK's LHA.
That is not a build or preparation proof for the newly selected helper.
The future origin audit must still identify NDK-derived bytes outside
specially named directories and retain appropriate notices in local preparation.

## GCC distribution adjustment

The Clorraine build adjustment dated 2026-11-18 quotes the filename returned by
GCC's `find_file_spec_function` before the specs engine reparses it. It keeps
the original GCC revision and staged source bytes unchanged; an exact locked
patch is applied only to the private build copy. This is not an upstream GCC
release, a source-pin substitution or a new licence grant.

The corresponding-source payload must retain the complete original GCC
snapshot, `compiler-patches/gcc-find-file-specs.diff`, its modification notice
and rebuild instructions, the actual build-control inputs and original GNU
licences/notices. Original source without the patch does not reconstruct the
modified driver. The source assembler explicitly conveys these new patch
inputs instead of relying on the pre-existing fixed build-input list.

The original and patched file hashes and zero-fuzz/repeat/mismatch controls
are recorded with the patch. Rebuilt native compatibility and actual relocated
corpus results remain mandatory and are not inferred from successful patch
application. Other component terms and the user-owned ordinary-libamiga
legal review are unchanged.
