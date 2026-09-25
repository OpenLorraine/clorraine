# AGENTS.md

Guidance for coding agents working in this repository. Keep it small; operational
routing belongs in configuration, not prose.

<!-- CODEOPS-PROJECT:START -->
## Project

The Clorraine compiler: a Rust workspace whose binary `clc` compiles the
corpus subset of the frozen specification under `spec/`, extended with the v1
language completion surfaces (generics, lambdas, and volatile access with the
per-chip MMIO wrappers), to one C99 translation unit for the `host`,
`hosted-amiga`, and `freestanding` profiles. The
`freestanding` profile is a provisional, LorraineOS-adoptable ABI (m68k, 32-bit,
big-endian, no libc) with a console-only standard-library surface. The workspace
is **std-only**: no crates, no build scripts. The primary languages are Rust
(the compiler), C99 (the emitted translation unit and the runtime shims), and
Clorraine itself (`.clo` sources, including the embedded standard library). The
demo-video toolkit under `tools/demo/` is a separate npm package; the Rust
workspace stays std-only.

## Commands

Run from the repository root. The Docker SDK container is the pinned toolchain;
`make -C docker exec CMD="..."` runs one command inside it (do not nest quotes
in `CMD`).

| Command | Purpose |
|---------|---------|
| `make check` | Fast verification: `./spec/check.sh`, then `cargo test --workspace` in the container |
| `make verify` | Full acceptance: `check` plus every hosted-amiga cross-toolchain leg |
| `make corpus-amiga` | Build and run the portable corpus on hosted-amiga (`m68k-amigaos-gcc` + `vamos`), the vbcc build-only leg, and build the FS-UAE-only programs (interrupt-counter and graphics) |
| `make corpus-fs-uae` | Boot the FS-UAE-only programs (interrupt-counter and graphics) in FS-UAE and diff their captured output (needs the SDK X display) |
| `make run-fault-amiga` | Run the six runtime-fault programs under `vamos` |
| `make run FILE=…` / `make build FILE=…` | Build (and run) a Clorraine program with the `clorraine` driver inside the container; add `PROFILE=hosted-amiga` or `PROFILE=freestanding`, `EMULATOR=1` (boot FS-UAE on the host), and `CAPTURE=1` |
| `make -C docker exec CMD="target/debug/clorraine run FILE.clo"` | The driver directly: `run`, `build`, or `emit` (`-p host\|hosted-amiga\|freestanding`, `--release`, `--capture`, `--emulator`, `--emit-c`) |
| `cargo test --workspace` | Rust tests; run in the container via `make check` |
| `make -C docker exec CMD="cargo build --workspace"` | Build `clc` at `target/debug/clc` |
| `make demo` | Render the demo video from `demo/story.ts` (host-only: needs Chrome, VHS, ffmpeg, and the FS-UAE SDK) |
| `make demo-youtube` | Render the story at 1920×1080 for upload (pillarboxed 1100×700 design) |
| `make channel-assets` | Render the YouTube banner, avatar, and thumbnail into `build/channel/` |
| `make demo-check` | Type-check and unit-test the `tools/demo/` toolkit (`tsc --noEmit` + `node --test`) |

## Layout

| Path | Contents |
|------|----------|
| `spec/` | The frozen specification: chapters, the grammar, the diagnostics registry, and the golden test data (`spec/tests/{compile-fail,run,staged,corpus}`) |
| `stdlib/` | The embedded standard-library sources (`std.*.clo`), including the Amiga modules (`std.amiga.dos`, `std.amiga.interrupt`, `std.amiga.intuition`, `std.amiga.graphics`, `std.amiga.custom`) |
| `compiler/src/` | The compiler crate: stage modules (`lex`, `parse`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`) plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, and the `clorraine` build/run driver |
| `compiler/runtime/` | The host, hosted-amiga, and freestanding C runtime shims (`shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c`); the hosted-amiga shim carries the screen and drawing calls |
| `compiler/tests/` | The Rust integration test drivers |
| `docker/` | The pinned Amiga SDK: toolchain config, scripts, and the emulator wrappers |
| `demo/` | The demo story (`story.ts`), a per-file scaffold (`cube.demo.ts`), the kept `clorraine.vim`, the recorder shim, and the committed poster; rendered media is gitignored |
| `tools/demo/` | `@clorraine/demo`, the zero-build TypeScript toolkit that renders the demo (headless Chrome, Shiki, VHS, ffmpeg) |
| `plans/` | CodeOps artifacts: the feature plans (`plans/clorraine-compiler-mvp/`, `plans/clorraine-v1-language-completion/`, and the other `plans/clorraine-*` plans) and `plans/00-roadmap.md` (flat layout, no `codeops/` directory) |
| `spikes/`, `docs/` | Phase 1 toolchain spikes and findings records |

## Test naming map

- `compiler/tests/*_spec_test.rs` are **specification tests** derived from
  `spec/`: `stage_spec_test.rs`, `compile_fail_spec_test.rs`,
  `codegen_run_spec_test.rs`, `run_fault_spec_test.rs`,
  `determinism_spec_test.rs`, `corpus_spec_test.rs`, `graphics_spec_test.rs`.
  They are an immutable oracle: a failing spec test means the implementation is
  wrong, never the test.
- `compiler/tests/stdlib_impl_test.rs` and
  `compiler/tests/graphics_impl_test.rs` hold **implementation tests** for the
  standard-library slice.
- `compiler/tests/v1_language_spec_test.rs` holds **specification tests** for
  the v1 language completion surfaces (generic structs, lambdas, volatile), and
  `compiler/tests/v1_language_impl_test.rs` holds their **implementation
  tests**.
- `compiler/tests/clorraine_driver_spec_test.rs` holds the **specification
  tests** for the `clorraine` build/run driver.
- `compiler/tests/freestanding_spec_test.rs` holds the **specification tests**
  for the `freestanding` profile: the m68k link-exit assertion and the host
  no-libc run.
- `#[cfg(test)] mod tests` blocks inside a `compiler/src/` module hold focused
  **unit tests** for that module's internals.
- `tools/demo/test/*.spec.test.ts` are **specification tests** for the demo
  toolkit and `*.impl.test.ts` are its implementation tests; run both with
  `make demo-check`. `render.smoke.test.ts` is an opt-in end-to-end render (set
  `DEMO_E2E=1`; needs Chrome and ffmpeg).

## Generated files and the frozen spec

- `build/` and `target/` are gitignored build output; the emitted `.c` from
  `clc` is generated, so do not hand-edit it.
- The demo media (`demo/*.mp4`, `demo/*.gif`, and `build/demo/`) is generated by
  `make demo` and gitignored, and the channel images (`build/channel/`) by
  `make channel-assets`; only `demo/poster.png` is committed. `tools/demo/` is
  npm-only and is not part of `make check` or `make verify`.
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
