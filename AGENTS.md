# AGENTS.md

Guidance for coding agents working in this repository. Keep it small; operational
routing belongs in configuration, not prose.

<!-- CODEOPS-PROJECT:START -->
## Project

The Clorraine compiler: a Rust workspace with two binaries. `clc` compiles the
corpus subset of the frozen specification under `spec/`, extended with the v1
language completion surfaces (generics, lambdas, and volatile access with the
per-chip MMIO wrappers), to one C99 translation unit for the `host`,
`hosted-amiga`, and `freestanding` profiles; `clorraine` is the build/run driver
around it. The
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
| `make demo` | Render the demo video from `demo/story.ts` plus the emulator chapter (host-only: needs Chrome, VHS, ffmpeg, and the FS-UAE SDK) |
| `make demo-youtube` | Render the story and the emulator chapter at 1920×1080 for upload (pillarboxed 1100×700 design) |
| `make channel-assets` | Render the YouTube banner, avatar, and thumbnail into `build/channel/` |
| `make demo-check` | Type-check and unit-test the `tools/demo/` toolkit (`tsc --noEmit` + `node --test`) |

## Layout

| Path | Contents |
|------|----------|
| `spec/` | The frozen specification: chapters, the grammar, the diagnostics registry, and the golden test data (`spec/tests/{compile-fail,run,staged,corpus}`) |
| `stdlib/` | The embedded standard-library sources (`std.*.clo`), including the Amiga modules (`std.amiga.dos`, `std.amiga.interrupt`, `std.amiga.intuition`, `std.amiga.graphics`, `std.amiga.custom`) |
| `compiler/src/` | The compiler crate: stage modules (`lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`) plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, `init_graph` (module-initializer dependencies shared by `sema` and `codegen`), and the `clorraine` build/run driver |
| `compiler/runtime/` | The host, hosted-amiga, and freestanding C runtime shims (`shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c`); the hosted-amiga shim carries the screen and drawing calls |
| `compiler/tests/` | The Rust integration test drivers |
| `docker/` | The pinned Amiga SDK: toolchain config, scripts, and the emulator wrappers |
| `demo/` | The demo story (`story.ts`), a per-file scaffold (`cube.demo.ts`), the kept `clorraine.vim`, the recorder shim, and the committed poster; rendered media is gitignored |
| `tools/demo/` | `@clorraine/demo`, the zero-build TypeScript toolkit that renders the demo (headless Chrome, Shiki, VHS, ffmpeg) |
| `examples/` | Small example programs (`hello`, `graphics`, `hosted-skeleton`) and a compile-fail ownership sample |
| `plans/` | CodeOps artifacts: the feature plans (one directory per plan) and `plans/00-roadmap.md` (flat layout: plans live under `plans/`, not `codeops/features/`) |
| `spikes/`, `docs/` | Toolchain spike records and phase findings/ergonomics documents |

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
- `compiler/tests/callable_release_integrity_spec_test.rs` holds the
  **specification tests** for callable release integrity (the generated
  callable identities, the per-callee-kind effect modes, the owner-release
  completeness cases, the view-return and array-elision parity, and the
  possibly-null view widening), and
  `compiler/tests/callable_release_integrity_impl_test.rs` holds its
  **implementation tests** (emitted callable and owner shapes, the mode
  vectors per callee kind, the report counts, and the byte-identity pins).
- `compiler/tests/clorraine_driver_spec_test.rs` holds the **specification
  tests** for the `clorraine` build/run driver.
- `compiler/tests/non_lexical_lifetimes_spec_test.rs` holds the specification
  tests for the v2 last-use borrow model, and
  `compiler/tests/non_lexical_lifetimes_impl_test.rs` holds its implementation
  tests (accepted programs build and run; conflicts report once; diagnostics
  are deterministic).
- `compiler/tests/codegen_ownership_spec_test.rs` holds the specification
  tests for borrow-element aliasing and local shadowing (the element view is a
  true alias on every exit path; every shadowing local gets its own identity),
  and `compiler/tests/codegen_ownership_impl_test.rs` holds its implementation
  tests (emitted-C shape, replacement read-back, slice iterables, sanitized
  name collisions, a build-only freestanding compile, and a run timeout).
- `compiler/tests/element_loop_integrity_spec_test.rs` holds the specification
  tests for owned-element lends and nested-loop steps (an element read passed
  to a parameter never double-frees; every nested `for` runs its own step; the
  run cases share a bounded timeout), and
  `compiler/tests/element_loop_integrity_impl_test.rs` holds its implementation
  tests (emitted-C shape for element arguments, ownerless temporaries, and
  template placeholders; nested loop headers; byte-identical re-emission).
- `compiler/tests/array_value_integrity_spec_test.rs` holds the specification
  tests for fixed-array values and `len` (copy, move, reassignment,
  self-assignment, multi-extent row element loops, row-unit slices, call-result
  projections, and the element-move limit), and
  `compiler/tests/array_value_integrity_impl_test.rs` holds its implementation
  tests (byte-copy and move shapes, the call temporary, the pointer-to-array
  row, row-unit slice offsets and bounds, guard traps, and release counts);
  `iterated_element` and the element-move predicate have unit tests in
  `compiler/src/types/tests.rs` and `compiler/src/own/tests.rs`.
- `compiler/tests/ownership_drop_spec_test.rs` holds the specification tests
  for array ownership, classification, and drop (an element moved into an array
  literal cannot be used again; a fixed array stays a lend candidate for return
  elision; a fixed array is not nullable), and
  `compiler/tests/ownership_drop_impl_test.rs` holds its implementation tests
  (the emitted C runs against the host C library, so a double free aborts).
- `compiler/tests/partial_move_impl_test.rs` holds **implementation tests** for
  partial-move / drop coherence: it builds the emitted C and runs it, so a
  double free aborts in the C library.
- `compiler/tests/borrow_view_and_shadowing_spec_test.rs` holds the
  specification tests for borrow-element-view moves and shadowed narrowing (an
  ownership transfer through a `for (borrow e in xs)` view reports `E10026` in
  every move form; a `let` or `const` initializer shadowing a narrowed nullable
  binding compiles; the null rules and the borrow-parameter rule are
  unchanged), and `compiler/tests/borrow_view_and_shadowing_impl_test.rs` holds
  its implementation tests (one report per transfer, the shadowing binding
  usable after its declaration, and the `E10010`/`E10012` reporting shape).
- `compiler/tests/runtime_value_integrity_spec_test.rs` holds the
  **specification tests** for released statement temporaries and their view
  lifetime, call-rooted element-loop iterables, conditional fixed-array values,
  callable-field receiver guards, the func-pointer hidden destination, and the
  panic message runtime, and
  `compiler/tests/runtime_value_integrity_impl_test.rs` holds its
  implementation tests (release and transfer-temp shapes, the iterable
  temporary, the conditional branch store, the receiver traps, the func
  declarator and destination, and the two-argument `cl_panic`).
- `compiler/tests/call_value_integrity_spec_test.rs` holds the **specification
  tests** for owned conditional stores, fixed-array value positions, func
  pointers and callable resolution, and guard/cleanup emission, and
  `compiler/tests/call_value_integrity_impl_test.rs` holds its implementation
  tests (branch-store and transfer-temporary shapes, element-store byte copies,
  materialized value positions, declarator and callee resolution, parameter
  copy/zero walks, the flag-first do-while, and the growable drop expression).
- `compiler/tests/freestanding_spec_test.rs` holds the **specification tests**
  for the `freestanding` profile: the m68k link-exit assertion and the host
  no-libc run.
- `compiler/tests/residual_defect_hardening_spec_test.rs` holds the
  **specification tests** for the four residual defects (flat array literal
  types and the deferred row store, the multi-extent return copy, the
  loop-condition owned-call release, and substitution-aware ownership), and
  `compiler/tests/residual_defect_hardening_impl_test.rs` holds their
  **implementation tests** (emitted shapes per instance, the non-owning no-op
  branches, the emptied take-argument place, the single `E10020` report, the
  nested-instance declaration, and byte-identical re-emission).
- `compiler/tests/residual_defect_completion_spec_test.rs` holds the
  **specification tests** for the six filed residual defects (fixed-array C
  symbol keys, the substituted generic-struct borrow argument, lifted-lambda
  instance specialization, ownerless call-result projection binding, the
  ownerless element take under an owning substitution, and the guarded
  drop-needing element hoist), and
  `compiler/tests/residual_defect_completion_impl_test.rs` holds their
  **implementation tests** (key and symbol shapes, specialization and
  environment names, binding and release shapes, guard-drop ordering, the
  opt-in sanitizer runs, and byte-identical re-emission).
- `compiler/tests/def_backlog_completion_spec_test.rs` holds the
  **specification tests** for the seven backlog defects (generic-lend
  rejection under an owning substitution, run-time-index move-out integrity,
  do-while condition release, and the closure callable spelling), and
  `compiler/tests/def_backlog_completion_impl_test.rs` holds their
  **implementation tests** (duty-report dedupe, index-carrying move-out and
  store shapes, condition-release counts and ordering, closure symbol and
  empty-call text, and byte-identical re-emission).
- `compiler/tests/def_backlog_closeout_spec_test.rs` holds the
  **specification tests** for the seven closed backlog defects (the guarded
  member-root take, the indexed aggregate-literal consume, the nullable borrow
  out-slot, the slice element through a statement temporary, the ownerless
  `len` argument, the do-while deferred return, and the unlisted lambda
  capture), and `compiler/tests/def_backlog_closeout_impl_test.rs` holds their
  **implementation tests** (guard zeroing, indexed store and move-out shapes,
  the pointer out-slot declarators, release counts and ordering, the capture
  scope floor and its listed-name exemption, and byte-identical re-emission).
- `compiler/tests/slice_temporary_integrity_spec_test.rs` holds the
  **specification tests** for the slice-and-temporary batch (row-typed slice
  views, the view-element conversion and const-extent agreement, the deferred
  consume fill and source order, the root and receiver binding on
  take/conditional/member shapes, the loop projection adoption, and the
  call-indexed element store), and
  `compiler/tests/slice_temporary_integrity_impl_test.rs` holds their
  **implementation tests** (row view keys, pointer declarators and zero
  helpers, builder and index spellings, deferred fill/move-out and adoption
  shapes, conditional branch stores, read-position receiver binds and their
  exclusions, the drop-suffix grouping, and byte-identical re-emission).
- `compiler/tests/defer_integrity_spec_test.rs` holds the **specification
  tests** for the defer-integrity batch (a `defer` runs on every exit edge; a
  deferred `continue`/`break` reaches the loop's edge; a deferred return
  continues the running cleanup and stores its value before the cleanup
  releases what it reads; a superseded owned result is released), and
  `compiler/tests/defer_integrity_impl_test.rs` holds their **implementation
  tests** (per-site copies with per-copy label suffixes, bounded copies, the
  remaining-actions order, the value-before-drop order, the outer-rung jump,
  the base local-name numbering, the once-per-occurrence diagnostics, and
  byte-identical re-emission).
- `compiler/tests/borrow_generic_integrity_spec_test.rs` holds the
  **specification tests** for the borrow/generic integrity batch (a whole-value
  store into a lend parameter is rejected once; a parameter-typed slot is
  judged per instance and nullable out-slots stay accepted; an out-slot
  self-assignment is rejected; instance keys distinguish nullable type
  arguments; a view compares with `null` and narrows; a func-typed callee named
  like a module lender keeps its own identity), and
  `compiler/tests/borrow_generic_integrity_impl_test.rs` holds their
  **implementation tests** (the emitted out-slot store shape, the loop-site
  diagnostic counts, the instance typedef and symbol spellings, the lowered
  `.ptr` null tests, the masked parameter spellings, and byte-identical
  re-emission).
- `compiler/tests/borrow_store_integrity_spec_test.rs` holds the
  **specification tests** for the borrow-store batch (a whole-slot or element
  self-assignment through a lend is judged per instance — the owning and
  nullable spellings are rejected while the duplicable, view, and non-nullable
  element spellings stay accepted; a whole out-slot return transfers the value
  and empties the caller's slot; a possibly-null view gates element stores,
  reads, and updates with the bounds traps), and
  `compiler/tests/borrow_store_integrity_impl_test.rs` holds their
  **implementation tests** (one report per site naming the first offending
  instance, the duplicable self-copy emission, the caller-slot emptying on a
  direct return, the bounds-trap emission for constant, compound, and update
  stores, the static fixed-array emission, and byte-identical re-emission).
- `compiler/tests/closure_integrity_spec_test.rs` holds the **specification
  tests** for the closure/callable integrity batch (a nested lambda's relayed
  capture is rejected at the entry unless the enclosing list carries the name;
  a capture entry naming a module-level name reports `E10406`; a dotted
  capture write reports `E10402`; a returned by-value owning capture reports
  `E10401` per instance while a duplicable instance copies; a called
  func-typed capture named like a module lender releases once; a call of an
  unbounded type-parameter value or of its field reports `E10307`), and
  `compiler/tests/closure_integrity_impl_test.rs` holds their
  **implementation tests** (the report count, name, and span per shape, the
  deep relay, one report per body form, the duty's instance order, the
  lender-named capture emission, and byte-identical re-emission).
- `compiler/tests/eval_emission_integrity_spec_test.rs` holds the
  **specification tests** for the evaluation/emission integrity batch (the
  module-initializer emission positions, the single-evaluated guarded operand,
  the fixed-array and slice emission shapes, the substituted projection reads,
  the null-inferred local, `str` pair equality, `str` element loops, the
  write-mutability routing for value-position updates and `cstring` writes, and
  the ownership-lifetime cases: the view-bound owner, the preempted ordinary
  return, the func-value temporary argument, the block-bodied and conditional
  capture moves, and the escaped-owner rejection), and
  `compiler/tests/eval_emission_integrity_impl_test.rs` holds their
  **implementation tests** (the emitted destination and guard shapes, the
  `__cl_ownedN` owner binding and release counts, the preemption-edge drop, the
  argument materialization, the duty/report counts and spans, and byte-identical
  re-emission where pinned).
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

- Layout: **flat** — each feature plan lives at
  `plans/<feature>/99-execution-plan.md`; the roadmap is `plans/00-roadmap.md`.
  There is no nested `codeops/features/` tree; `codeops/` holds only the
  project-level specialist check state (`codeops/specialist-check.json`).
- Specialist coverage: up to date (2026-10-10) — see
  `codeops/specialist-check.json`; re-run `analyze-agents` after new plans land.
- Do not run the CodeOps installer from inside this repository: it replaces
  the two customized reviewer briefs (`correctness-reviewer`,
  `semantics-reviewer`) with catalog defaults. Run installs/upgrades from
  outside the repo (for example, `cd ~`), and after any in-repo install
  restore the two briefs (`git checkout --
  .opencode/agents/correctness-reviewer.md
  .opencode/agents/semantics-reviewer.md`). The catalog's other project-scope
  files (agents, skills, references, `_shared/`) are environment tooling and
  stay untracked; only the four project agents under `.opencode/agents/` are
  committed.
- Keep `plans/*/99-...`/roadmap state in sync through the CodeOps skills, and
  follow the repository's quality profile (strict: independent review, stop on
  critical/major findings).
- Phase-review dispatch follows the change surface:
  - ownership, drop, or emission changes: correctness reviewer plus the
    read-only memory-safety auditor
    (`.opencode/agents/memory-safety-auditor.md`), which probes the changed
    shapes under the host AddressSanitizer;
  - IR, codegen, or symbol-name changes: those two plus the read-only
    emission-integrity auditor
    (`.opencode/agents/emission-integrity-auditor.md`), which diffs base and
    head emissions over the corpus and hunts accepted-program C build
    failures;
  - language-rule changes (lends, captures, evaluation order): those two plus
    `semantics-reviewer`, with the `spec/17-codegen` ABI lens where signatures
    or lowering change.
- Reviewer packets always include a blast-radius step (every call site,
  exhaustive match, and parallel path of the changed construct) and a
  test-integrity step (an assertion must fail without the fix).
<!-- CODEOPS-PROJECT:END -->
