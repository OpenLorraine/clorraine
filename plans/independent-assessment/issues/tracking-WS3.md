<!-- title: [bug] Minor findings from the independent assessment: C code generation -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:codegen,tier:advanced,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 3 surviving Minor finding(s) in the C code generation workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS3-codegen.md`.

- [ ] **ASM-WS3-004** — On the host profile `usize` is 32 bits, so a pointer round trip through `usize` crashes (effort M, verified, verdict confirmed (Minor, verified); theme {THEME:R10})
  - Host-profile tests of `raw`/`usize` code (allocators, address arithmetic) crash or fail to build although the same code is correct on the target; the host profile exists for fast tests, so its divergence weakens them. No shipped lens is affected.
  - Location: the emitted `typedef unsigned int usize;` / `typedef int isize;` and `ULONG` for the host profile (`compiler/src/codegen/program.rs`, host prelude), at e24a8752
  - Fix: on the host profile map `usize`/`isize` to `size_t`/`ptrdiff_t` (or `uintptr_t`/`intptr_t`), or record a spec ruling that the host profile models the 32-bit target and reject `usize(p)` there.
- [ ] **ASM-WS3-005** — Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files (effort S, verified, verdict confirmed (Minor, verified); theme {THEME:R3})
  - GCC and vbcc diagnostics in runtime code, and `m68k-amigaos-gdb` stepping through it, point at nonexistent `.clo` lines, which misleads a user investigating a build failure (as in ASM-WS3-001) or a crash in the runtime.
  - Location: `compiler/src/codegen.rs:815` and `compiler/src/codegen/cleanup.rs:330` (the only `#line` writers; no reset before the appended shim suffix or before hoisted local declarations), at e24a8752
  - Fix: emit `#line <n> "<unit>.c"` (or the shim's own name) before each appended shim and before the declaration block of each function.
- [ ] **ASM-WS3-006** — The emitted C depends on how the source path is spelled and embeds absolute paths (effort S, verified, verdict confirmed (Minor, verified); theme {THEME:R3})
  - Builds driven with absolute paths or from different working directories are not reproducible across machines, and leak host directory names into the C and its debug info.
  - Location: `#line` path spelling in `compiler/src/codegen.rs:806-815` (the logical path is the command-line spelling), at e24a8752
  - Fix: normalize `#line` paths to the root module's directory (the project root used for `embed`); or obtain a spec ruling defining the logical path as the invocation spelling.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
