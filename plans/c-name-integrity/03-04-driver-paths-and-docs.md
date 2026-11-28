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
- The artifact directory still takes the first source's stem (`artifacts.rs:149-153`).

## Docs (AR-14, AR-22, AR-23)

- `docs/known-limitations.md` §7: extend the table and text with escaped source names (the six
  positions, the `_0` suffix, the classes), the instance and helper rows (`get._Point`,
  `box.alloc__i32`, `zero.ClRec`), and the `#line` mapping for runtime files.
- A residual list in the same section: C libraries other than glibc and MinGW-w64 (AR-11);
  toolchains other than the pinned SDK and rc24; vbcc predefines (AR-19); `clc` with an absolute
  path, and a driver source outside the working folder, keep their absolute spelling (AR-10,
  AR-24); `-g` builds carry the C compiler's working folder in debug info (AR-22);
  `compiler/runtime/` files are not on a user's disk, so a debugger shows no source for runtime
  frames (AR-21).
- `THIRD_PARTY_NOTICES.md`: the AR-23 provenance note.
- No site page lists these details (AR-14).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Canonicalization fails | pass the original spelling | AR-24 |
| Source outside the working folder | pass the original spelling, documented | AR-24 |

## Testing Requirements
- ST-21..ST-24 (07).
