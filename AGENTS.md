# AGENTS.md

Guidance for coding agents working in this repository. Keep it small; operational
routing belongs in configuration, not prose.

<!-- CODEOPS-PROJECT:START -->
## Project

The Clorraine compiler: a std-only Rust workspace (one member, `compiler/`; no
external crates, no build scripts) with two binaries. `clc` compiles `.clo`
sources to one C99 translation unit; `clorraine` is the build/run driver.

Profiles:
- `host` — development and host test runs.
- `hosted-amiga` — m68k AmigaOS-compatible output (`m68k-amigaos-gcc`, `vamos`,
  FS-UAE).
- `hosted-amiga-13` — A500/Kickstart 1.3 baseline: 68000, OCS, 512 KB chip,
  `-mcrt=nix13`.
- `freestanding` — provisional m68k LorraineOS ABI: 32-bit, big-endian, no libc,
  console-only standard library.

The language is frozen under `spec/`. The compiler implements the corpus subset
plus the completed language surfaces (generics, lambdas, volatile/MMIO with
per-chip wrappers, and the v2 NLL borrow model). The embedded standard library
is `stdlib/`; the runtime shims are C in `compiler/runtime/`. Emitted C must stay
valid C99 and pass the strict host and Amiga toolchain legs. `tools/demo/` is a
separate npm package and is not part of the Rust workspace.

## Commands

Run from the repository root. The Docker SDK image is the pinned toolchain. Use
`make -C docker exec CMD="..."` for one command inside it; do not nest quotes in
`CMD`. A host-only GitHub Actions workflow (`.github/workflows/check.yml`) runs
the spec check and workspace tests on push and pull request; it is not the
acceptance gate. `make check` and `make verify` remain authoritative.

| Command | Purpose |
|---------|---------|
| `make check` | Fast gate: `./spec/check.sh`, `tools/amiga-sdk/registry_test.sh`, then `cargo test --workspace` in the container |
| `make asan` | Run every suite that references `RDC_ASAN` under AddressSanitizer, one suite per process; the runner links ASan children with `-no-pie` |
| `make differential` | Compare host and hosted-amiga stdout for `buffer-container`, `file-copier`, and `string-parser`; `DIFF_ONLY=1` re-diffs an existing `OUTDIR` |
| `make verify` | Full acceptance: `check` plus `sdk-a500-smoke`, `sdk-a500-boing`, `sdk-a500-sprites`, `sdk-a500-paddle`, `sdk-a500-text`, `sdk-a500-assets`, `sdk-a500-gui`, `differential`, `corpus-fs-uae`, and `run-fault-amiga`; the FS-UAE legs need the SDK X display |
| `make corpus-amiga` | Build/run the portable corpus on hosted-amiga (`m68k-amigaos-gcc` + `vamos`); vbcc build-only; build FS-UAE-only programs |
| `make corpus-fs-uae` | Boot the FS-UAE-only programs and diff captured output |
| `make run-fault-amiga` | Run the six runtime-fault programs under `vamos` |
| `make run FILE=…` / `make build FILE=…` | Build (and run) one program in the container; set `PROFILE=hosted-amiga\|hosted-amiga-13\|freestanding`, `EMULATOR=1`, `CAPTURE=1` |
| `make -C docker exec CMD="cargo build --workspace"` | Build `clc` and `clorraine` at `target/debug/` |
| `make -C docker exec CMD="target/debug/clorraine run FILE.clo"` | Driver directly: `run`, `build`, or `emit` (`-p host\|hosted-amiga\|hosted-amiga-13\|freestanding`, `--release`, `--capture`, `--emulator`, `--emit-c`, `--explain CODE`, `--diagnostics=json`) |
| `make sdk-registry` / `make sdk-issues` | Validate the Amiga SDK registry, regenerate `docs/amiga-sdk/coverage.md`, and write per-bundle issue bodies |
| `make sdk-a500-smoke` | Build hello for `hosted-amiga-13`, decrypt the licensed Kickstart 1.3 ROM locally, boot the ADF in FS-UAE as an A500, and compare the captured screen line |
| `make sdk-a500-boing` | Build the native-video Boing demo for `hosted-amiga-13`, see its live custom display on the A500/KS1.3 run, and compare the final frame-checksum line |
| `make sdk-a500-sprites` | Build the sprite demo for `hosted-amiga-13`, see the live sprite screen on the A500/KS1.3 run, and compare the hardware-collision line |
| `make sdk-a500-paddle` | Build the audio/input paddle game for `hosted-amiga-13`, see the live court on the A500/KS1.3 run, and compare the hit/audio/input line |
| `make sdk-a500-text` | Build the scrolling text/layers demo for `hosted-amiga-13`, see the held layer frame on the A500/KS1.3 run, and compare the measured text line |
| `make sdk-a500-assets` | Build the ILBM viewer for `hosted-amiga-13` in release mode, see the held image frame on the A500/KS1.3 run, and compare the DOS/asset report line |
| `make sdk-a500-gui` | Build the Intuition GUI demo for `hosted-amiga-13`, drive it headlessly on Xvfb with the Amiga-key shortcuts, and compare its native screen capture and report line |
| `cargo test --workspace` | Rust tests; run inside the container via `make check` |
| `make demo-check` | Type-check and test `tools/demo/` (`tsc --noEmit`, `node --test`) |
| `make demo`, `make demo-youtube`, `make channel-assets` | Host-only media renders; `demo`/`demo-youtube` need Chrome, VHS, ffmpeg, and the SDK/emulator; `channel-assets` needs Chrome |

## Layout

| Path | Contents |
|------|----------|
| `spec/` | Frozen specification: chapters, grammar, diagnostics registry, and golden data (`spec/tests/{compile-fail,run,staged,corpus}`) |
| `stdlib/` | Embedded standard library (`std.*.clo`), including `std.iff`, `std.math`, `std.amiga.video`, `std.amiga.ilbm`, `std.amiga.sprite`, `std.amiga.audio`, `std.amiga.input`, `std.amiga.timer`, `std.amiga.text`, `std.amiga.layers`, `std.amiga.exec`, and the other Amiga modules (`std.amiga.*`) |
| `compiler/src/` | Compiler stages (`lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`) plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, `init_graph`, and the `clorraine` driver |
| `compiler/runtime/` | C runtime shims: `shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c` |
| `compiler/tests/` | Rust integration test drivers |
| `docker/` | Pinned Amiga SDK: toolchain config, scripts, and emulator wrappers |
| `.github/workflows/` | Host-only GitHub Actions checks (`check.yml`) |
| `demo/` | Demo story (`story.ts`), per-file scaffold, Vim syntax file, recorder shim, and committed poster; rendered media is ignored |
| `tools/demo/` | `@clorraine/demo`, the zero-build TypeScript render toolkit (headless Chrome, Shiki, VHS, ffmpeg) |
| `editors/` | Canonical editor assets: `clorraine.tmLanguage.json`, consumed by the demo toolkit and enforced against the EBNF by `make check` |
| `examples/` | Small example programs, including the hosted-amiga acceptance programs under `examples/amiga/`, and a compile-fail ownership sample |
| `plans/` | CodeOps plans (one directory per feature) and `plans/00-roadmap.md`; flat layout |
| `docs/`, `spikes/` | Findings/ergonomics notes, `docs/known-limitations.md`, and toolchain spike records; `RELEASE.md` and `THIRD_PARTY_NOTICES.md` sit at the root |

## Testing

- `compiler/tests/*_spec_test.rs` are **specification tests**. The core suites
  (`stage`, `compile_fail`, `codegen_run`, `run_fault`, `determinism`, `corpus`,
  `graphics`) derive from `spec/`; feature suites derive from their approved
  requirements. They are an immutable oracle: a failing spec test means the
  implementation is wrong, never the test.
- `compiler/tests/*_impl_test.rs` are **implementation tests** for internals and
  emitted shapes. Feature suites follow the `<feature>_spec_test.rs` /
  `<feature>_impl_test.rs` naming; list `compiler/tests/` for the current set.
  Some pin emitted C text; update a pin only for the intended emission change.
- `#[cfg(test)] mod tests` inside `compiler/src/` holds focused unit tests.
- `tools/demo/test/*.spec.test.ts` and `*.impl.test.ts` follow the same split;
  `render.smoke.test.ts` is opt-in (`DEMO_E2E=1`, Chrome + ffmpeg).
- Golden inputs live under `spec/tests/{compile-fail,run,staged,corpus}`; run the
  whole suite with `cargo test --workspace` via `make check`.
- `make asan` discovers the suites that reference `RDC_ASAN`, runs each in its
  own bounded process with `--test-threads=1`, and installs a temporary `cc`
  wrapper that appends `-no-pie` to ASan links. The wrapper is required: the
  SDK image's GCC 12 ASan runtime is unstable for PIE executables.
- `make differential` is a convenience gate over the same host and hosted-amiga
  outputs the corpus pins; it is not an independent signal. The CI workflow is
  host-only, so m68k-only tests skip themselves without the SDK toolchain.
- `make sdk-a500-gui` runs FS-UAE on a private Xvfb display and drives the demo
  with Amiga-key shortcuts; it does not open a window on the desktop. The
  artifact is the app's native `CSCR` capture, converted by
  `docker/scripts/cscr2png.py`, not a host-window screenshot.

## Generated files and the frozen spec

- `build/` and `target/` are gitignored; the `.c` emitted by `clc` is generated,
  so do not hand-edit it.
- Demo media (`demo/*.mp4`, `demo/*.gif`, `build/demo/`) is generated by
  `make demo` and ignored; only `demo/poster.png` is committed. `tools/demo/` is
  npm-only and excluded from `make check`/`make verify`.
- `docker/.env` and `docker/compose.override.yaml` are local and gitignored.
- `spec/` is the immutable oracle. Do not change a golden or chapter to match the
  implementation; a specification change needs an explicit user ruling.
- Never reference `plans/`, `requirements/`, or a plan/requirement/ambiguity
  identifier in source code or doc comments; restate durable rationale in plain
  language.

## CodeOps

- Layout: **flat** — feature plans live at `plans/<feature>/99-execution-plan.md`;
  the roadmap is `plans/00-roadmap.md`. There is no nested `codeops/features/`
  tree; `codeops/` holds only `specialist-check.json`.
- Branch: `main` is the integration branch (`origin`:
  `git@github.com:OpenLorraine/clorraine.git`).
- Specialist coverage: **up to date (2026-11-05)** — 61 plan folders recorded in
  `codeops/specialist-check.json`.
- Do not run the CodeOps installer from inside this repository: it replaces the
  two customized reviewer briefs (`correctness-reviewer`, `semantics-reviewer`)
  with catalog defaults. Run installs/upgrades from outside the repo (for
  example, `cd ~`), and after any in-repo install restore those two briefs:
  `git checkout -- .opencode/agents/correctness-reviewer.md
  .opencode/agents/semantics-reviewer.md`. The catalog's other project-scope
  files stay untracked; only the four project agents under `.opencode/agents/`
  are committed.
- Keep `plans/*/99-...` and the roadmap in sync through the CodeOps skills, and
  follow the strict quality profile: independent review, stop on critical/major
  findings.
- Phase-review dispatch follows the change surface:
  - ownership, drop, or emission changes: correctness reviewer + read-only
    memory-safety auditor (host AddressSanitizer probes);
  - IR, codegen, or symbol-name changes: those two + read-only
    emission-integrity auditor (base/head corpus emission diff, strict C build
    hunt);
  - language-rule changes (lends, captures, evaluation order): those two +
    `semantics-reviewer`, with the `spec/17-codegen` ABI lens where signatures
    or lowering change.
- Reviewer packets always include a blast-radius step (every call site,
  exhaustive match, and parallel path of the changed construct) and a
  test-integrity step (an assertion must fail without the fix).
<!-- CODEOPS-PROJECT:END -->
