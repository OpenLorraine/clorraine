# Support payload origin review

This is a review of the fixed upstream install rules and selected source
files, with a bounded comparison against the existing SDK. It is not the
complete measured inventory of a newly built reference SDK.
Never use a directory-name allowlist as a substitute for file origins.

## Install-rule trace

Line numbers refer to `m68k-amigaos-gcc/Makefile` at the catalog's exact commit.
Paths below are relative to the SDK prefix.

| Installed paths | Observed producer | Distribution treatment |
|---|---|---|
| `m68k-amigaos/ndk-include/**` | Lines 1435–1473 copy NDK C/assembler includes and generate wrappers from SFD files; helper headers are also copied into this tree. | Exclude NDK-origin contents from shipped support. Retain independent helper origins separately; the directory itself is not one licence. |
| `m68k-amigaos/ndk13-include/**` | Lines 1513–1534 copy/truncate newer NDK headers and generate wrappers from derived FD/SFD data. | Exclude these derived headers too. Regenerate locally and verify complete content hashes against the private reference. |
| `m68k-amigaos/ndk/lib/{fd,sfd,libs,fd13,sfd13}/**` | Lines 1443–1448 and 1525–1528 copy NDK inputs or derive 1.3 inputs. | Exclude from ordinary support and corresponding-source archives; this includes link libraries, not only files ending in `.h`. |
| `m68k-amigaos/ndk-include/inline/{alib,stubs,macros}.h` | Lines 1452–1455 copy fd2sfd/fd2pragma source headers. | Trace to the helper checkouts, not automatically to NDK or generated output. Preserve their notices if conveying them independently. |
| `m68k-amigaos/ndk-include/**` overlay | Lines 1545–1547 copy `amiga-netinclude/include/**` over the prepared NDK tree. | Resolve overwritten files to the overlay's actual bytes and file-specific terms. A location under `ndk-include` does not prove NDK authorship. |
| `m68k-amigaos/sys-include/**` | Lines 1711–1715 copy newlib generic headers followed by Amiga system headers, then run newlib install. | Review both source trees and install overwrites. Do not claim this directory is NDK-free solely from its name. |
| `m68k-amigaos/libnix/include/**` | Line 1586 copies `libnix/sources/headers/**`. | Preserve inherited header notices, including `poll.h`; root public-domain wording is insufficient. |
| `m68k-amigaos/include/*.h` from pthreads | Lines 1665–1675 copy pthread headers, excluding `debug.h`, and install target archives. | Trace the actual selected headers/archives to the pthread subtree and its notices; do not include the entire AROS repository as runtime code. |
| `m68k-amigaos/libnix/lib/**` | Libnix `Makefile.gcc6:140–173` installs archives and startup objects; the outer rules also build target runtime variants. | Retain each required runtime's actual closure and notices. Review generated members, inherited code and source distribution form. |
| `lib/gcc/m68k-amigaos/*/include/**`, libgcc | GCC installation and target-library build. | Preserve GCC runtime/header notices and applicable Runtime Library Exception, plus matching source/build information. Do not extend that exception to the compiler itself or unrelated libraries. |
| `m68k-amigaos/lib/libamiga.a` | Outer lines 1556–1565 copy the ordinary prebuilt archive. The private reference recipe explicitly installs the hash-verified copy first. | Include only the exact user-directed ordinary file, retaining its unresolved historical terms and provenance. |
| `m68k-amigaos/lib/libb/libamiga.a` | Separate outer rule at lines 1567–1569. | Not covered by the ordinary-file inclusion direction. Do not pick it up through a broad library-directory copy. |
| `bin/sfdc` | Concatenated Perl source per sfdc `Makefile.in`. | Script/interpreter/module audit, source and notices; not an ELF helper. |
| Host compiler, binutils and native helpers | Selected build/install outputs. | Inspect actual binaries, scripts, links, native libraries and notices. Exclude GDB and other unneeded SDK programs even if upstream dependencies built them. |

The private reference intentionally contains excluded material. It is not a
redistributable support tree. Copying the entire reference `sdk/` directory
would violate this boundary.

## Existing-SDK cross-check

The bounded comparison used the immutable SDK image
`sha256:3835c03b273e3d71b5ae3790c6c5706a511722690c9e67a6316420746880a782`,
without networking or emulator launch. It compared regular-file SHA-256
values in `include`, `sys-include` and `libnix/include` against the existing
`ndk-include`, `ndk13-include` and `ndk` trees, without treating links as files.

One identical-file candidate was found outside those NDK-named directories:

| Existing SDK path | Matching prepared-tree path | Source check |
|---|---|---|
| `m68k-amigaos/sys-include/resolv.h` | `m68k-amigaos/ndk-include/resolv.h` | Both match the pinned netinclude `include/resolv.h`: SHA-256 `3b3e143c36e4eee892399b660418d1d7b93f6dd288912c5fd4100a8865636462`. |

This is an independently identified networking header, not proof of NDK
authorship. Its inline notices must be retained. The queried generic newlib
`newlib/libc/include/resolv.h` path is absent at the pinned revision; no source
origin is invented for it.

No other byte-identical candidate was found in that limited comparison.
This does not rule out transformed NDK contents, unexamined directories,
archive members or embedded data. Equality is a lead; installed-path and
producer tracing remain necessary.

## Required measured follow-through

Before accepting the final lock or support package:

1. Inventory the new private reference's regular files and links, including
   the complete locally prepared include trees. Record relative paths, hashes
   and exact link targets, not just counts.
2. Trace every selected installed file through its copy/generation/link rule.
   Resolve overwritten sources. Record all contributing origins where needed.
3. Build the NDK-derived exclusion set from actual inputs and transformed
   outputs, including bytes outside specially named directories.
4. Keep unknown-origin files out of the candidate unless an explicit bounded
   direction covers that exact file. The ordinary libamiga direction must not
   spread to another archive or source.
5. Preserve per-file terms and required matching source/build information.
   Resolve libnix's `pipeoc.c` source-form conditions before conveying it.
6. Audit native ELF dependencies, script interpreters/modules, relative links
   and builder-prefix leaks. Verify Ubuntu 22.04/24.04 prerequisites and both
   target runtime profiles after relocation.
7. Compare the actual payload with the complete audit. Missing, extra,
   modified, forbidden or untraced files fail packaging.

The existing SDK comparison cannot replace this measured proof. The source
review exposes the producers and exclusions; reference inventory and final
payload assembly must provide the actual records.

## New private reference measurements

The corrected offline reference build has now passed. Generated records under
`build/distribution/reference-20261117` include `reference-tree.json`,
`reference-prepared-files.json`, `reference-elf-inspection.json`,
`reference-target-archives.json` and `reference-compile-smoke.json`.
These are private evidence, not a shipped payload audit or final lock.

| Measurement | Result |
|---|---|
| SDK files / links | 3,793 regular files; 11 internal relative links; no unsafe link; six empty directories |
| Prepared-tree locations | 1,012 files in `ndk-include`, 166 in `ndk13-include`, 194 under `ndk/` |
| Native host ELF | 52 x86_64 files; GLIBC requirement at most 2.35; no RPATH/RUNPATH |
| Target archives | 166 verified through the built cross-objdump; every member is m68k; 165 are HUNK archives |
| Startup/other target objects | 26 standalone `.o` files; `ncrt0.o` and `libinit.o` independently confirmed as non-archive Amiga/m68k objects |
| Compile/link probe | Strict C99 hello links for `nix13` and `nix20` with `/sdk` mounted read-only and original builder prefix unavailable |

The complete reference still includes excluded build-time programs and other
unselected files. The full-tree ELF dependency list is not a shipped-runtime
prerequisite set; in particular GDB's dependencies must not leak into it.
Location-based prepared-tree counts also include independent overlays/helper
headers and must not become a blanket NDK-origin classification.

The user-approved audit contract now distinguishes `amiga-object` from `ar`
and `elf`, with narrow specification coverage authored before implementation.
Do not omit the native record or mislabel an object as an archive to get a
candidate through validation. Actual packaging/acceptance is still pending.
All 26 reference objects satisfy the approved format/architecture constraints.
