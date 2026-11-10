# AGENTS.md

Guidance for coding agents working in this repository. Keep it small; operational
routing belongs in configuration, not prose.

<!-- CODEOPS-PROJECT:START -->
## Project

The Clorraine compiler: a std-only Rust workspace (one member, `compiler/`; no
external crates, no build scripts) with three binaries. `clc` compiles `.clo`
sources to one C99 translation unit; `clorraine` is the build/run driver;
`clorraine-lsp` is the stdio language server.

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
valid C99 and pass the strict host and Amiga toolchain legs. `tools/demo/` and
`tools/vscode/` are separate npm packages and are not part of the Rust
workspace.

## Commands

Run from the repository root. The Docker SDK image is the pinned toolchain. Use
`make -C docker exec CMD="..."` for one command inside it; do not nest quotes in
`CMD`. A host-only GitHub Actions workflow (`.github/workflows/check.yml`) runs
the spec check and workspace tests on push and pull request; it is not the
acceptance gate. `make check` and `make verify` remain authoritative.

| Command | Purpose |
|---------|---------|
| `make check` | Fast gate: `./spec/check.sh`, `tools/amiga-sdk/registry_test.sh`, `cargo test --workspace` in the container, then `bench-test`, `fuzz-test`, and `fuzz-smoke` |
| `make asan` | Run every suite that references `RDC_ASAN` under AddressSanitizer, one suite per process; the runner links ASan children with `-no-pie` |
| `make ubsan` | Run every suite that references `RDC_UBSAN` under UndefinedBehaviorSanitizer; a known-overflow probe proves the leg is active, and the runner links children with `-no-pie`; part of `make verify` |
| `make fuzz-test` | Fuzzer unit tests; part of `make check` |
| `make fuzz-smoke` | Bounded deterministic differential fuzzer matrix (12 seeds + generated 0–15); part of `make check` |
| `make fuzz-smoke-host` | Host-only fuzzer fallback; no m68k leg |
| `make fuzz` | Longer local fuzzer run (generated 0–255, full host/sanitizer matrix) |
| `make differential` | Compare host and hosted-amiga stdout for `buffer-container`, `file-copier`, and `string-parser`; `DIFF_ONLY=1` re-diffs an existing `OUTDIR` |
| `make verify` | Full acceptance: `check`, `bench-shape`, `sdk-a500-smoke`, `sdk-a500-boing`, `sdk-a500-sprites`, `sdk-a500-paddle`, `sdk-a500-text`, `sdk-a500-assets`, `sdk-a500-gui`, `sdk-a500-exec`, `sdk-a600-services`, `sdk-a1200-workbench`, `sdk-a500-diskfont`, `sdk-a600-addons`, `sdk-a600-ks31-addons`, `sdk-a1200-ks30-addons`, `sdk-a1200-addons`, `sdk-a1200-rtg`, `sdk-a4000-rtg-piv`, `differential`, `corpus-fs-uae`, and `run-fault-amiga`; the FS-UAE legs need the SDK X display and the licensed media under `~/amiga` |
| `make corpus-amiga` | Build/run the portable corpus on hosted-amiga (`m68k-amigaos-gcc` + `vamos`); vbcc build-only; build FS-UAE-only programs |
| `make corpus-fs-uae` | Boot the FS-UAE-only programs and diff captured output |
| `make run-fault-amiga` | Run the six runtime-fault programs under `vamos` |
| `make run FILE=…` / `make build FILE=…` | Build (and run) one program in the container; set `PROFILE=hosted-amiga\|hosted-amiga-13\|freestanding`, `EMULATOR=1`, `CAPTURE=1` |
| `make -C docker exec CMD="cargo build --workspace"` | Build `clc`, `clorraine`, and `clorraine-lsp` at `target/debug/` |
| `make -C docker exec CMD="target/debug/clorraine run FILE.clo"` | Driver directly: `run`, `build`, or `emit` (`-p host\|hosted-amiga\|hosted-amiga-13\|freestanding`, `--release`, `--capture`, `--emulator`, `--emit-c`, `--explain CODE`, `--doc [-o DIR] FILE...`, `--diagnostics=json`) |
| `make sdk-registry` / `make sdk-issues` | Validate the Amiga SDK registry, regenerate `docs/amiga-sdk/coverage.md`, and write per-bundle issue bodies |
| `make sdk-a500-smoke` | Build hello for `hosted-amiga-13`, decrypt the licensed Kickstart 1.3 ROM locally, boot the ADF in FS-UAE as an A500, and compare the captured screen line |
| `make sdk-a500-boing` | Build the native-video Boing demo for `hosted-amiga-13`, see its live custom display on the A500/KS1.3 run, and compare the final frame-checksum line |
| `make sdk-a500-sprites` | Build the sprite demo for `hosted-amiga-13`, see the live sprite screen on the A500/KS1.3 run, and compare the hardware-collision line |
| `make sdk-a500-paddle` | Build the audio/input paddle game for `hosted-amiga-13`, see the live court on the A500/KS1.3 run, and compare the hit/audio/input line |
| `make sdk-a500-text` | Build the scrolling text/layers demo for `hosted-amiga-13`, see the held layer frame on the A500/KS1.3 run, and compare the measured text line |
| `make sdk-a500-assets` | Build the ILBM viewer for `hosted-amiga-13` in release mode, see the held image frame on the A500/KS1.3 run, and compare the DOS/asset report line |
| `make sdk-a500-gui` | Build the Intuition GUI demo for `hosted-amiga-13`, drive it headlessly on Xvfb with the Amiga-key shortcuts, and compare its native screen capture and report line |
| `make sdk-a500-exec` | Build the Exec demo for `hosted-amiga-13`, boot it on the A500/KS1.3, and compare the report line and the 620x22 line golden |
| `make sdk-advanced-build` | Build the Workbench and services demos plus the two test-only Workbench fixtures |
| `make sdk-a600-services` | Build the advanced demos/fixtures, boot A600/KS2.05, start the ROM Workbench, and assert the Workbench and services report lines |
| `make sdk-a1200-workbench` | Build the advanced demos/fixtures, boot A1200/KS3.1, assert the report lines, and compare the native Workbench capture and AppIcon crop goldens |
| `make sdk-bundle9-build` | Build the release Bundle 9 hosted-amiga demos |
| `make sdk-a500-diskfont` | Build the diskfont demo for `hosted-amiga-13`, boot A500/KS1.3 with the Workbench 1.3.5 HDF, and assert `diskfont: ok h 12` |
| `make sdk-a600-addons` | Boot A600/KS2.05, assert the Exec/args/services/intuition lines, drive the ASL requester on Xvfb, and compare the intuition capture |
| `make sdk-a600-ks31-addons` | Boot A600/KS3.1 and assert `ll: ok v 40` and `nv: ok n 23 size 30` |
| `make sdk-a1200-ks30-addons` | Boot A1200/KS3.0, assert `pool: ok 1` and `boopsi: ok val 42`, and compare the BOOPSI capture |
| `make sdk-a1200-addons` | Boot A1200/KS3.1 with Workbench and the host echo peer, and assert the datatypes, network, and AmigaGuide lines |
| `make sdk-a1200-rtg` | Boot A1200/KS3.1 with Picasso96 and FS-UAE UAEGFX, assert the 32-bit RTG report and PPM capture, and write `docs/amiga-sdk/a1200-rtg.png` |
| `make sdk-a4000-rtg-piv` | Boot A4000/KS3.1 with Picasso96, the emulated Picasso IV Zorro III card, and the 7.4 flash ROM, assert the 24-bit RTG report and PPM capture, and write `docs/amiga-sdk/a4000-rtg-piv.png` |
| `cargo test --workspace` | Rust tests; run inside the container via `make check` |
| `make bench` | Trace the benchmark suite in the pinned container and write `build/perf/head.json` |
| `make bench-fast` | Same suite without the m68k trace; writes `build/perf/head-fast.json` |
| `make bench-base COMMIT=…` | Build the base `clc` from a commit and write `build/perf/base.json`; retains `build/perf/base-clc` |
| `make bench-compare` | Enforce the comparison invariants and any declared `REQUIRE` expectations between `base.json` and `head.json`; empty `REQUIRE` checks the 1% no-regression guard only |
| `make bench-flags` | Run the same-compiler old-vs-GC-vs-shipped flag audit, assert the size/runtime floors, and write `build/perf/flags.json` |
| `make bench-test` | Harness unit tests; also part of `make check` |
| `make bench-shape` | Build the generated-C shape fixture with m68k GCC, vbcc, and the freestanding profile |
| `make demo-check` | Type-check and test `tools/demo/` (`tsc --noEmit`, `node --test`) |
| `make setup` | Detect the SDK; `FETCH=1` builds missing public components, `DRY=1` prints the plan, `YES=1` skips confirmation; never downloads media |
| `make setup-check` | Installer unit tests; also part of `make check` |
| `make vscode-check` | Build the server in the container, type-check and test `tools/vscode/`, package a `.vsix`, audit it, and run the packaged client |
| `make vscode-package` | Build and package `build/vscode/clorraine-<version>.vsix` |
| `make demo`, `make demo-youtube`, `make channel-assets` | Host-only media renders; `demo`/`demo-youtube` need Chrome, VHS, ffmpeg, and the SDK/emulator; `channel-assets` needs Chrome |

## Layout

| Path | Contents |
|------|----------|
| `spec/` | Frozen specification: chapters, grammar, diagnostics registry, and golden data (`spec/tests/{compile-fail,run,staged,corpus}`) |
| `stdlib/` | Embedded standard library (`std.*.clo`), including `std.iff`, `std.math`, `std.amiga.video`, `std.amiga.ilbm`, `std.amiga.sprite`, `std.amiga.audio`, `std.amiga.input`, `std.amiga.timer`, `std.amiga.text`, `std.amiga.layers`, `std.amiga.exec`, `std.amiga.icon`, `std.amiga.workbench`, `std.amiga.expansion`, `std.amiga.picasso96`, and the other Amiga modules (`std.amiga.*`) |
| `compiler/src/` | Compiler stages (`lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`) plus `driver`, `analysis`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, `init_graph`, the `clorraine` driver, `docgen`, and `lsp/` |
| `compiler/runtime/` | C runtime shims: `shim_host.c`, `shim_hosted_amiga.c` plus the 2.0+ `shim_hosted_amiga_20.c` and optional suffixes, `shim_freestanding.c`, and the vendored `picasso96.h` definitions |
| `compiler/tests/` | Rust integration test drivers |
| `benchmarks/` | Generated-C benchmark fixtures (not spec goldens) |
| `tools/perf/` | Benchmark harness and its unit tests |
| `tools/fuzz/` | Deterministic differential fuzzer, committed seeds and goldens, and its unit tests |
| `docker/` | Pinned Amiga SDK: toolchain config, scripts, and emulator wrappers |
| `.github/workflows/` | Host-only GitHub Actions checks (`check.yml`) |
| `demo/` | Demo story (`story.ts`), per-file scaffold, Vim syntax file, recorder shim, and committed poster; rendered media is ignored |
| `tools/demo/` | `@clorraine/demo`, the zero-build TypeScript render toolkit (headless Chrome, Shiki, VHS, ffmpeg) |
| `tools/setup/` | Fetch/detect SDK installer and its Python unit tests |
| `tools/vscode/` | VS Code extension package, LSP client, and headless tests |
| `editors/` | Canonical editor assets: `clorraine.tmLanguage.json`, consumed by the demo toolkit and enforced against the EBNF by `make check` |
| `examples/` | Small example programs, including the hosted-amiga acceptance programs under `examples/amiga/`, the target-side fault probes under `examples/robustness/`, and a compile-fail ownership sample |
| `plans/` | CodeOps plans (one directory per feature) and `plans/00-roadmap.md`; flat layout |
| `docs/`, `spikes/` | Documentation index, the C-to-Clorraine guide, findings/ergonomics notes, `docs/known-limitations.md`, and toolchain spike records; `RELEASE.md` and `THIRD_PARTY_NOTICES.md` sit at the root |

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
- `tools/setup/tests/` runs with `make setup-check`; `tools/vscode/test/`
  runs with `make vscode-check` and includes a packaged-client smoke test.
- `compiler/tests/lsp_spec_test.rs`, `compiler/tests/docgen_spec_test.rs`, and
  `compiler/tests/docs_guide_spec_test.rs` cover the language server, the
  documentation generator, and the guide links.
- `tools/perf/bench.py` measures generated-C size, m68k section sizes, host time,
  and the streamed `vamos` instruction count; `make bench-test` runs its unit
  tests. Benchmark fixtures live under `benchmarks/` and are not spec goldens.
- Golden inputs live under `spec/tests/{compile-fail,run,staged,corpus}`; run the
  whole suite with `cargo test --workspace` via `make check`.
- `make asan` discovers the suites that reference `RDC_ASAN`, runs each in its
  own bounded process with `--test-threads=1`, and installs a temporary `cc`
  wrapper that appends `-no-pie` to ASan links. The wrapper is required: the
  SDK image's GCC 12 ASan runtime is unstable for PIE executables.
- `make ubsan` discovers the suites that reference `RDC_UBSAN`, runs each in
  its own bounded process with `--test-threads=1`, proves the sanitizer is
  active with a known signed-overflow probe, and appends `-no-pie` to
  sanitizer links. The robustness suite also builds its own ASan/UBSan
  controls, so a silently unsanitized child build is a failure.
- `tools/fuzz/fuzz.py` is a SHA-256-seeded, typed template generator. The
  smoke target runs the committed seeds on host, ASan/UBSan, and m68k, plus a
  bounded generated range; `make fuzz` is the longer local run. Seed goldens
  catch shared host/m68k wrong values; the differential catches divergence.
- `make differential` is a convenience gate over the same host and hosted-amiga
  outputs the corpus pins; it is not an independent signal. The CI workflow is
  host-only, so m68k-only tests skip themselves without the SDK toolchain.
- `make sdk-a500-gui` runs FS-UAE on a private Xvfb display and drives the demo
  with Amiga-key shortcuts; it does not open a window on the desktop. The
  artifact is the app's native `CSCR` capture, converted by
  `docker/scripts/cscr2png.py`, not a host-window screenshot.
- Bundle 9's `sdk-a500-diskfont`, `sdk-a600-addons`, `sdk-a600-ks31-addons`,
  `sdk-a1200-ks30-addons`, and `sdk-a1200-addons` gates install the licensed
  Workbench media into clean directory HDs. The A1200/KS3.1 gate starts
  Workbench, runs the host TCP echo peer with `--network host`, and uses
  FS-UAE's `bsdsocket_library`. The optional C shim is m68k-only, so it is not
  covered by host AddressSanitizer; `make asan` does not instrument it.
- `make sdk-a600-services` and `make sdk-a1200-workbench` boot clean directory
  hard drives and start the ROM-resident Workbench with the test-only
  `docker/scripts/workbench-start.c` fixture (the private vector `C:LoadWB`
  uses). `workbench-post.c` posts the three AppMessages. No host input is
  injected. The A1200 gate compares the native Workbench capture and AppIcon
  crop; the A600 gate asserts the same report lines on the 2.0 minimum.

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
- Specialist coverage: **up to date (2026-11-09)** — 71 plan folders recorded in
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
