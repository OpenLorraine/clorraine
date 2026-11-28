# Driver paths and docs: C Name Integrity

> **Document**: 03-04-driver-paths-and-docs.md
> **Parent**: [Index](00-index.md)

## Overview

The `clorraine` driver keeps absolute paths out of the C it builds (ASM-WS3-006, the user's AR-10
choice), and the docs record the new behaviour and its limits. Decisions AR-10, AR-14, AR-22..AR-24.

## Driver path rewrite (AR-24)

- Where: in the driver before it reads the sources for `build`, `run`, and `emit`
  (`compiler/src/clorraine/`), so the source map, `#line`, and the driver's diagnostics see one
  spelling. `clc` (`compiler/src/driver.rs`) is unchanged.
- Rule: canonicalize the working folder and the source (strip the Windows verbatim prefix with the
  existing `platform.rs` helper). If the source lies inside the folder, pass the relative path
  with `/` separators; otherwise pass the original spelling. A canonicalization failure passes the
  original spelling, and the existing read error reports it.
- Separators: the relative path comes from a pure function in `platform.rs` that takes the
  `HostOs` and the two canonical spellings, like the other helpers there. On Windows `\` and `/`
  are separators and the comparison follows the host's path rules; on Unix only `/` is, and a `\`
  inside a file name stays a name byte (escaped in `#line` by `c_string_body`). Both families are
  tested on Linux through that function.
- The artifact directory still takes the first source's stem (`artifacts.rs:149-153`).

## Docs (AR-14, AR-22, AR-23; AGENTS.md prime directive)

Each phase re-reads `docs/known-limitations.md` and `site/guide/known-limitations.md` before its
commit, edits what its change makes stale, and otherwise records "known limitations: unchanged"
in the commit message. The expected edits, by phase:

- Phase 1: `docs/known-limitations.md` §7 — escaped locals and parameters (the `_0` suffix, the
  classes, the retired `cl_` keyword prefix).
- Phase 2: §7 — fields and captures escape the same way; enum constants are emitted as values.
- Phase 3: §7 — the instance and helper rows (`get._Point`, `box.alloc__i32`, `zero.ClRec`,
  `string.view`); §11 "Hook symbol names" and "Box helper names" (`:217-218`) narrow to names
  the program does not emit, since an emitted one is now E11101.
- Phase 4: §7 — the `#line` mapping for runtime files, and the residual that
  `compiler/runtime/` files are not on a user's disk, so a debugger shows no source for runtime
  frames (AR-21).
- Phase 5: the driver rule, and the residual list in §7: C libraries other than glibc and
  MinGW-w64 (AR-11); toolchains other than the pinned SDK and rc24; vbcc predefines (AR-19);
  `clc` with an absolute path, and a driver source outside the working folder, keep their
  absolute spelling (AR-10, AR-24); `-g` builds carry the C compiler's working folder in debug
  info (AR-22). `THIRD_PARTY_NOTICES.md`: the AR-23 provenance note.
- `site/guide/known-limitations.md` has no symbol-naming rows today (AR-14); each phase still
  checks it and edits it if a summary statement becomes stale.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Canonicalization fails | pass the original spelling | AR-24 |
| Source outside the working folder | pass the original spelling, documented | AR-24 |

## Testing Requirements
- ST-21..ST-24 (07).
