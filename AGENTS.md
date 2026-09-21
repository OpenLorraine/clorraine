# AGENTS.md

Guidance for coding agents working in this repository. Keep it small; operational
routing belongs in configuration, not prose.

<!-- CODEOPS-PROJECT:START -->
## Project

The Clorraine compiler MVP: a Rust workspace whose binary `clc` compiles the
corpus subset of the frozen specification under `spec/` to one C99 translation
unit for the `host` and `hosted-amiga` profiles. The workspace is **std-only**:
no crates, no build scripts. The primary languages are Rust (the compiler), C99
(the emitted translation unit and the runtime shims), and Clorraine itself
(`.clo` sources, including the embedded standard library).

## Commands

Run from the repository root. The Docker SDK container is the pinned toolchain;
`make -C docker exec CMD="..."` runs one command inside it (do not nest quotes
in `CMD`).

| Command | Purpose |
|---------|---------|
| `make check` | Fast verification: `./spec/check.sh`, then `cargo test --workspace` in the container |
| `make verify` | Full acceptance: `check` plus every hosted-amiga cross-toolchain leg |
| `make corpus-amiga` | Build and run the portable corpus on hosted-amiga (`m68k-amigaos-gcc` + `vamos`), the vbcc build-only leg, and build the interrupt-counter |
| `make corpus-fs-uae` | Boot the interrupt-counter in FS-UAE and diff its captured output (needs the SDK X display) |
| `make run-fault-amiga` | Run the six runtime-fault programs under `vamos` |
| `cargo test --workspace` | Rust tests; run in the container via `make check` |
| `make -C docker exec CMD="cargo build --workspace"` | Build `clc` at `target/debug/clc` |

## Layout

| Path | Contents |
|------|----------|
| `spec/` | The frozen specification: chapters, the grammar, the diagnostics registry, and the golden test data (`spec/tests/{compile-fail,run,staged,corpus}`) |
| `stdlib/` | The embedded standard-library sources (`std.*.clo`) |
| `compiler/src/` | The compiler crate: stage modules (`lex`, `parse`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`) plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token` |
| `compiler/runtime/` | The host and hosted-amiga C runtime shims (`shim_host.c`, `shim_hosted_amiga.c`) |
| `compiler/tests/` | The Rust integration test drivers |
| `docker/` | The pinned Amiga SDK: toolchain config, scripts, and the emulator wrappers |
| `plans/` | CodeOps artifacts: the feature plan (`plans/clorraine-compiler-mvp/`) and `plans/00-roadmap.md` (flat layout, no `codeops/` directory) |
| `spikes/`, `docs/` | Phase 1 toolchain spikes and findings records |

## Test naming map

- `compiler/tests/*_spec_test.rs` are **specification tests** derived from
  `spec/`: `stage_spec_test.rs`, `compile_fail_spec_test.rs`,
  `codegen_run_spec_test.rs`, `run_fault_spec_test.rs`,
  `determinism_spec_test.rs`, `corpus_spec_test.rs`. They are an immutable
  oracle: a failing spec test means the implementation is wrong, never the
  test.
- `compiler/tests/stdlib_impl_test.rs` holds **implementation tests** for the
  standard-library slice.
- `#[cfg(test)] mod tests` blocks inside a `compiler/src/` module hold focused
  **unit tests** for that module's internals.

## Generated files and the frozen spec

- `build/` and `target/` are gitignored build output; the emitted `.c` from
  `clc` is generated, so do not hand-edit it.
- `spec/` is the immutable oracle. Do not change a golden or chapter to match
  the implementation; a specification change needs an explicit user ruling.
- Never reference `plans/`, `requirements/`, or a plan/requirement/ambiguity
  identifier in source code or doc comments; restate durable rationale in plain
  language instead.

## CodeOps

- Layout: **flat** (no `codeops/` directory). The plan is at
  `plans/clorraine-compiler-mvp/99-execution-plan.md`; the roadmap is
  `plans/00-roadmap.md`.
- Keep `plans/99-...`/roadmap state in sync through the CodeOps skills, and
  follow the repository's quality profile (strict: independent review, stop on
  critical/major findings).
<!-- CODEOPS-PROJECT:END -->
