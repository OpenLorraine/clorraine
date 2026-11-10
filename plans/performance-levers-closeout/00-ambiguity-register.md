# Ambiguity Register: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design active — eligible technical decisions delegated under
> policy version 1; product scope, the acceptance direction, safety constraints,
> and public surfaces remain user-owned
> **Root invocation ID**: `build-flags-closeout-20261110`
> **Policy version**: 1

Auto-design active — eligible technical decisions are delegated and recorded.
This register separates the user-owned boundaries from the delegated build and
measurement mechanisms.

## Authority Boundary

| Area | Owner | Evidence |
|------|-------|----------|
| Deliver a measured binary-size win without a runtime or safety regression | User | Goal: "performance-levers audit plus binary-size/DCE/flags closeout"; no scored regression above 1%; no safety trade-offs |
| Keep measurement deterministic and attribution honest; do not tune benchmarks | User | Goal; benchmark fixture policy in `benchmarks/README.md` |
| Add no new target profiles or public build modes | User | Goal scope; the 68020/FPU option is already issue #15 |
| Choose the flag set, measurement method, file layout, and test shape | AI — delegated | `--auto-design`, policy version 1, eligible performance-engineering and compiler-mechanism classes |
| Approve a larger framework, service, dependency, or product surface | User — reserved | Complexity Escalation Gate; none proposed here |

## Decision Records

### AR-01 — Scope of the closeout

- **Status**: ✅ Resolved
- **Authority**: User — the goal authorizes a performance-levers audit and a
  binary-size/DCE/flags closeout; the prior session registered the remaining
  optimization options as separate issues.
- **Objective**: ship the strongest measured build-flag win without changing
  language, ABI, runtime semantics, or safety behavior.
- **Decision**: audit the candidate build/link flags, ship section garbage
  collection plus release stripping on `host` (Linux), `hosted-amiga`, and
  `hosted-amiga-13`, update the two GCC release rows in the frozen compile
  matrix (user ruling 2026-11-10, AR-19), align the benchmark harness with the
  shipped release build, close the release-path gate gaps (AR-20), record the
  attribution, and register the rejected or deferred levers as issues.
  `freestanding` and non-Linux hosts keep their current release flags.
- **Evidence**: `compiler/src/clorraine/toolchain.rs:39-54` holds the release
  flags; `tools/perf/bench.py:36-41` mirrors them; `make bench` builds the
  measured artifacts; the exploratory matrix in AR-04.
- **Rejected alternatives**: an emission-only change (the constant-array win is
  already landed and the next emitted shapes are registered in #14/#18); a
  single unbounded "optimize everything" goal (the prior session deliberately
  split it); a new user-facing `--size` mode (a public surface that needs a
  user decision); a dependency or build-system change (not needed).
- **Strongest counterargument**: build flags are a small code change compared
  with the size of the gate; the full `make verify` is still required because
  link-time elimination can remove constructors, optional shims, or tables.
  The risk is real but the measured win is large and the gate is the point.
- **Confidence**: High — the probe reproduces on every profile and every
  scored benchmark.
- **Hardening**: requirements preflight, the design challenger, and the
  correctness/emission reviews challenge the attribution and the
  constructor-removal risk.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a full gate fails because a needed section is collected;
  a scored benchmark regresses; the user opens a public size mode or a new
  target profile.

### AR-02 — Workflow and authority

- **Status**: ✅ Resolved
- **Authority**: User — "move next with --auto-commit and --auto-design".
- **Decision**: run `make-requirements` → requirements preflight → `make-plan`
  → plan preflight → `exec-plan --auto-commit --auto-design`. The delegated
  authority covers eligible technical mechanisms in the requirements and plan
  chain; reserved product surface stays exactly the goal plus the user's prior
  answers.
- **Evidence**: the user's invocation; the same chain is recorded in the
  prior DX closeout register (`plans/dx-adoption-closeout/00-ambiguity-register.md`).
- **Rejected alternatives**: skipping requirements (the flag set changes the
  shipped build and needs a durable decision record); implementing first and
  documenting later (violates the repository's specification-first policy).
- **Strongest counterargument**: the change is small enough to be a task-lane
  item; the cross-profile gate and the constructor-removal risk make it a
  feature with real acceptance criteria.
- **Confidence**: High.
- **Hardening**: requirements and plan preflights.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the user changes the authority or the workflow.

### AR-03 — Benchmark base

- **Status**: ✅ Resolved
- **Authority**: AI — delegated technical measurement decision.
- **Objective**: attribute the new flag win to this feature, not to the
  already-landed constant-array win.
- **Decision**: base the scored comparison on the current `main` head
  `f9d5be8` (`f9d5be8d66474f00162ef78fc32ef83bae84aeda`), the post-PLAN-25
  closeout. The compiler version remains `clc 1.1.1`. Keep the previous
  `v1.1.0` and `v1.1.1` measurements as historical evidence only.
- **Evidence**: `v1.1.1` (`2d1eb30`, 2026-11-09 15:01) predates the
  constant-array feature commits (`cef0b91` at 23:10 and later), so it does
  **not** include PLAN-25. `build/perf/head.json` already measures the
  post-PLAN-25 compiler at `clc 1.1.1`. The current head `f9d5be8` contains
  the whole previous closeout, including the ffi-shadowing codegen fix
  `fd7fe30`, the exact-cast test pins, the corpus-retry script fix, and the
  plan/documentation commits. It is the branch head, not a release tag.
- **Rejected alternatives**: `v1.1.0` or `v1.1.1` (both double-count or omit
  the previous win and muddy attribution); `82877ab` (same compiler but not
  the branch head); no base (cannot prove no regression).
- **Strongest counterargument**: a `v1.1.0` base tells a longer cumulative
  story. The findings can still mention it, but the gated comparison must
  isolate this feature and start from the code that actually ships today.
- **Confidence**: High.
- **Hardening**: `make bench-base COMMIT=f9d5be8` regenerates the document;
  the findings assert `base.json.compiler.git` is the full `f9d5be8` hash and
  the harness tree is clean (`dirty` is a tree-cleanliness check, not
  base-commit provenance). The harness records the compiler SHA-256.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a later feature needs a different attribution boundary;
  the integration branch head moves before the audit starts.

### AR-04 — Shipped release flag set

- **Status**: ✅ Resolved
- **Authority**: AI — delegated performance-engineering decision.
- **Objective**: the smallest release build that removes unreachable code and
  symbols without changing runtime behavior.
- **Decision**: for `host` on Linux, `hosted-amiga`, and `hosted-amiga-13`
  release builds:
  `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`.
  Keep `-O2`; keep every existing base flag and library.
- **Evidence** (same `clc`, same toolchain, only flags differ):

  | Program | m68k `.text` old → GC | Δ | stripped binary old → GC+s | Δ |
  |---|---:|---:|---:|---:|
  | calibration | 76,368 → 5,960 | −92.2% | 99,948 → 7,328 | −92.7% |
  | sieve | 76,728 → 23,832 | −68.9% | 100,332 → 25,676 | −74.4% |
  | matrix | 76,676 → 23,608 | −69.2% | 100,276 → 25,444 | −74.6% |
  | strings | 77,604 → 24,196 | −68.8% | 101,500 → 26,080 | −74.3% |
  | moves | 76,896 → 23,632 | −69.3% | 100,632 → 25,460 | −74.7% |
  | lookup | 92,976 → 39,904 | −57.1% | 116,584 → 41,740 | −64.2% |

  `hosted-amiga-13` probe: calibration 53,432 → 5,908 `.text` (−88.9%);
  sieve 53,792 → 23,780 (−55.8%); the four portable corpus programs
  (`buffer-container`, `file-copier`, `string-parser`, `volatile-mmio`)
  49.1–88.9% reductions. Host release binary bytes: 20.6–36.6% reductions
  with GC+strip.
- **Rejected alternatives** (measured):
  - `-O3`: `.text` +14.9% on calibration; runtime equal; no win.
  - `-Os` as the default: `.text` −8.7% on calibration and −8.8% on sieve,
    but strings runtime +13.1%; violates the 1% guard.
  - `-fomit-frame-pointer`: `.text` −0.1%; runtime equal; not worth the
    debugging trade.
  - `-flto` alone: worse than GC (calibration 10,640 vs 5,960); `-flto`+GC
    saves only 2–11% more than GC and adds link-time/toolchain risk.
  - `-fno-asynchronous-unwind-tables`/`-fno-unwind-tables`: no host size
    change.
  - `-Wl,--gc-keep-exported`: keeps the global shim symbols, defeating the
    DCE win.
- **Strongest counterargument**: `-O2` is a compromise; `-Os` would shrink
  more but makes `strings` 13% slower. The user's no-regression rule makes
  `-O2` the correct default; an opt-in size mode is registered as an issue.
- **Confidence**: High — the five scored programs, calibration, and the 1.3
  corpus reproduce.
- **Hardening**: the design challenger attacks the attribution; the
  constructor audit (AR-13) attacks the DCE safety; the full verify and the
  release-path gate additions (AR-20) are the final proof.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a scored benchmark regresses; a needed constructor is
  collected; the pinned toolchain changes its linker behavior.

### AR-05 — Release stripping policy

- **Status**: ✅ Resolved
- **Authority**: AI — delegated release-packaging mechanism.
- **Decision**: add `-s` to release builds only. Debug builds keep `-O0 -g`,
  warnings, and symbols. A developer who needs symbols builds without
  `--release`.
- **Evidence**: after GC, `-s` removes 7–27% of the m68k release bytes
  (lookup 45,008 → 41,740 = 7.3%; sieve 28,960 → 25,676 = 11.3%;
  calibration 10,052 → 7,328 = 27.1%) and 6–12% of host release bytes
  (lookup 33,008 → 30,920 = 6.3%; sieve 16,584 → 14,536 = 12.4%;
  calibration 16,000 → 14,480 = 9.5%). It does not change `.text`,
  instructions, or output.
- **Rejected alternatives**: strip everything including debug (breaks the
  debugging workflow); no strip (leaves a measurable win on the table);
  `--strip-debug` only (release builds have no debug sections, so it is a
  no-op).
- **Strongest counterargument**: release crash reports lose function names.
  The repository's debugging path is the debug build, and the release artifact
  is the product; this is normal release packaging. The limitation is recorded
  in `docs/known-limitations.md`.
- **Confidence**: High.
- **Hardening**: the correctness review checks that debug tests and ASan/UBSan
  runs never use the stripped path.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a required production diagnosis needs symbols in the
  shipped artifact; a platform's strip option changes.

### AR-06 — Non-Linux host portability

- **Status**: ✅ Resolved
- **Authority**: AI — delegated implementation mechanism within the approved
  host profile.
- **Decision**: apply the GNU section-GC and strip flags only when the host
  target uses a GNU-compatible linker (Linux in this repository's pinned
  environment). Other host targets keep the current `-std=c99 -O2` release
  command, so the host profile never fails because of a GNU-only linker
  option. The platform choice is a pure predicate parameterised by a linker
  or platform value, so both branches run on the Linux test host.
- **Evidence**: `-Wl,--gc-sections` is a GNU ld/lld option; macOS `ld64` uses
  `-dead_strip` instead. The repository's CI and SDK are Linux; macOS host
  flags are untested. The existing `native_syscalls()`/`run_tool(...,
  native_syscalls)` seam shows the testable shape.
- **Rejected alternatives**: apply GNU flags everywhere (breaks macOS);
  add `-dead_strip` now (untested); remove macOS host support (a product
  regression); use a bare `cfg!` branch and a `#[cfg(not(target_os = "linux"))]`
  test (the test would never run in the gate).
- **Strongest counterargument**: a macOS host would miss the win. That is
  acceptable until a macOS toolchain leg exists; the option is registered as
  an issue.
- **Confidence**: High for Linux; macOS behavior is an explicit fallback.
- **Hardening**: a unit test pins both predicate outcomes on the Linux host;
  the findings record the unsupported platform.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: macOS becomes a supported host or CI leg.

### AR-07 — Freestanding profile

- **Status**: ✅ Resolved
- **Authority**: AI — delegated technical decision within the confirmed scope.
- **Decision**: keep the `freestanding` release command at `-O2`; do not add
  section GC or stripping in this closeout. The freestanding host-run release
  command (`run_tool(Profile::Freestanding, ..)`) also stays at `-O2`.
- **Evidence**: the freestanding corpus image is 208 bytes `.text` with
  `-O2` and 232 bytes with GC (+11.5%); the link is already minimal under
  `-nostdlib -lg`. Strip-only cuts the 1,552-byte file to 244 bytes; GC+strip
  gives 268 bytes. The freestanding image is a provisional LorraineOS ABI
  where symbols and bring-up diagnostics matter more than bytes.
- **Rejected alternatives**: GC (measured growth); strip now (a debuggability
  trade on a provisional target); split the freestanding shim (not needed at
  208 bytes).
- **Strongest counterargument**: stripping saves 84% of the freestanding
  file. The image is tiny and provisional; a future LorraineOS bring-up gate
  is the right place to decide. Registered as an issue.
- **Confidence**: High — measured.
- **Hardening**: the audit records the freestanding row and the unchanged
  command; the full freestanding legs stay green.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: PLAN-06 (LorraineOS bring-up) reopens, or a larger
  freestanding workload shows a GC win.

### AR-08 — Harness alignment and attribution

- **Status**: ✅ Resolved
- **Authority**: AI — delegated measurement mechanism.
- **Decision**: update `tools/perf/bench.py` release flags to the shipped set
  (`host` and `hosted-amiga`), add a `m68k_release_bytes` metric, keep debug
  flags unchanged, and keep the existing `flags` equality check. Regenerate
  `build/perf/base.json` from `f9d5be8` under the new harness and assert the
  recorded `compiler.git` is that full hash and the harness tree is clean
  (`dirty == false`). The scored
  `base.json`/`head.json` comparison is an **emission-regression guard**; the
  **flag win** is the separate same-compiler old-vs-new audit.
- **Evidence**: the harness records `flags` and refuses to compare documents
  whose flags differ (`tools/perf/bench.py:493-498,625-630`). The current
  harness hard-codes `-O2` (`:39-41`), so leaving it unchanged would make
  `head.json` describe a build the driver no longer ships. `bench-compare`
  does not compare `compiler`, so the base identity must be asserted
  explicitly.
- **Rejected alternatives**: keep the harness canonical and measure the
  product elsewhere (head.json would not reflect the shipped artifact); add a
  second benchmark mode (more surface); compare across different flags
  (violates the harness invariant and mixes attributions); rely on the
  version string (both `v1.1.1` and `f9d5be8` report `clc 1.1.1`).
- **Strongest counterargument**: changing the harness makes the base and head
  both use the new flags, so `bench-compare` shows no size win. That is
  intentional: the audit carries the win, and the comparison carries the
  no-regression guard. The findings state both.
- **Confidence**: High.
- **Hardening**: the design challenger checks the two-comparison model; the
  harness unit tests pin the flag strings and the new metric; the findings
  assert the base identity.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the harness needs to measure several flag sets
  routinely; a future feature changes the release line again.

### AR-09 — Audit method and reproducibility

- **Status**: ✅ Resolved
- **Authority**: AI — delegated measurement mechanism.
- **Decision**: the flag audit uses one `clc` binary and one toolchain per
  comparison, varies only the candidate flags, and records emitted-C bytes,
  `.text`/`.data`/`.bss`, release binary bytes, raw instruction count (for the
  base and GC variants), output, compiler SHA-256, and toolchain identity.
  The procedure emits C with `clc` and compiles it with explicit old/new flag
  arrays; it never uses the driver's `--release` line for the old row. It uses
  one fixed output path per program, rebuilt in place for each variant, so
  symbol-table path length cannot move the byte counts. The repeat set is
  base/GC/GC+strip on the five scored programs plus calibration,
  base/GC/GC+strip on the four named 1.3 corpus programs, and the freestanding
  pair; the rejected candidates are measured once in Phase 1 and recorded. Two repeat runs must agree exactly. The
  commands and raw results live in `05-findings.md`; a small committed audit
  procedure reproduces them and asserts every floor.
- **Evidence**: the exploratory probes were deterministic; the executed-code
  difference is the two removed constructor bodies (`___initamigapath` and
  `___fillenviron`, 444 instructions before `main`), not a skipped list walk.
- **Rejected alternatives**: compare two benchmark documents with different
  flags (the harness forbids it); use only `.text` (binary size is the product
  metric); single runs (no repeatability proof); run the full candidate matrix
  twice (unbounded cost for no extra confidence); add a new benchmark tuned
  for flags (benchmark tuning).
- **Strongest counterargument**: an audit procedure outside the scored harness
  can drift. A committed procedure and the findings' raw numbers keep it
  honest; the procedure is a small script or exact command block, not a new
  framework.
- **Confidence**: High.
- **Hardening**: the design challenger reviews the procedure and the
  attribution; the findings include the exact commands; the audit exits
  nonzero on a missed floor.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the audit cannot reproduce on the pinned image.

### AR-10 — Acceptance floors

- **Status**: ✅ Resolved
- **Authority**: User owns the direction (measured win, no scored regression
  above 1%, full gates); AI delegates the materiality floors below as the
  technical operationalization of that direction.
- **Decision**:
  - m68k `.text`: at least 20% smaller for every scored benchmark (measured
    57.1–69.3%; calibration is 92.2% but is not scored).
  - m68k stripped release binary: at least 20% smaller for every scored
    benchmark (measured 64.2–74.7%; calibration is 92.7%).
  - host release binary: at least 15% smaller for every scored benchmark
    (measured 20.6–36.6%).
  - `hosted-amiga-13` corpus `.text`: at least 40% smaller for each of the
    four named corpus programs (measured 49.1–88.9%).
  - calibration raw instructions: at least 10% lower (measured 13.2%).
  - scored net instructions: no benchmark more than 1% worse; expected equal.
  - outputs byte-identical; freestanding image unchanged; full gate green.
  - a missed floor stops the landing; it is escalated, never lowered.
- **Evidence**: the probe table; the floors sit well below the measured
  values so ordinary toolchain variance cannot flip a result.
- **Rejected alternatives**: no floor (a token 0.1% win would pass); a 50%
  uniform floor (host and 13 corpus would fail despite strong wins); a
  per-benchmark custom target (complexity without value).
- **Strongest counterargument**: the floors are conservative and do not
  describe the headline win. The findings record the exact values; the floors
  are regression guards, not marketing claims.
- **Confidence**: High.
- **Hardening**: the plan's Phase 1 re-runs the audit and records exact values;
  the final gate compares them to the floors.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a measured value falls below a floor; the user changes
  the acceptance direction.

### AR-11 — Safety constraints

- **Status**: ✅ Resolved
- **Authority**: User — no safety trade-offs.
- **Decision**: keep every runtime check, bounds check, diagnostic, and
  sanitizer leg. Do not add `-ffast-math`, `-fno-stack-protector`,
  `-fno-delete-null-pointer-checks`, or any flag that weakens defined
  behavior. The release strip removes symbols only, not checks.
- **Evidence**: the candidate flags are code-layout and link-time options;
  none changes arithmetic or bounds semantics. ASan/UBSan runs use debug
  builds, which keep the current flags.
- **Rejected alternatives**: `-Ofast` (unsafe math); disabling stack
  protection or fortify (safety trade); `-fno-wrapv` (changes defined
  overflow semantics used by the code generator).
- **Strongest counterargument**: link-time GC could remove a needed safety
  helper if it is only referenced dynamically. The constructor/section audit
  (AR-13) and the full verify cover this; a missing helper is a stop.
- **Confidence**: High.
- **Hardening**: ASan/UBSan, fuzzing, fault probes, and the full verify.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a gate shows a removed helper is needed.

### AR-12 — Debug builds stay unchanged

- **Status**: ✅ Resolved
- **Authority**: AI — delegated implementation mechanism.
- **Decision**: `-O0 -g -Wall -Wextra -Werror` stays the debug command for
  every profile; no section GC and no strip in debug.
- **Evidence**: `toolchain.rs:44-54`; the test/debug matrix; the ASan/UBSan
  and fuzz legs use debug builds.
- **Rejected alternatives**: add GC to debug (slower links and harder
  debugging for no product win); strip debug (destroys the debugging
  workflow).
- **Strongest counterargument**: debug binaries stay large. They are developer
  artifacts, not shipped products.
- **Confidence**: High.
- **Hardening**: unit tests pin both debug and release vectors.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a debug workflow needs section GC.

### AR-13 — Link-time constructor and section audit

- **Status**: ✅ Resolved
- **Authority**: AI — delegated correctness verification.
- **Decision**: the implementation audit enumerates the sections removed by
  `--gc-sections` that are not ordinary user/runtime functions, in particular
  `.list___INIT_LIST__`, and classifies each as unused or required. It records
  the remaining init-list entries and proves they still run. The full
  `make verify` plus the release-path additions (AR-20: release Workbench/
  services and a 1.3 release corpus run) are the acceptance proof. If a
  required constructor or table is removed, the flag set does not land
  unchanged; the plan must add a targeted `KEEP`/`-u` or stop, and then re-run
  the audit.
- **Evidence**: `--gc-sections` removes the `__amigapath.o` and `environ.o`
  objects and their init-list entries (`___initamigapath` and
  `___fillenviron`); the remaining `___INIT_LIST__` entries still run.
  The 444-instruction reduction is those two constructor bodies, which run
  before `main`. No current `stdlib/` or `compiler/runtime/` code references
  `environ`, `__amigapath`, `getenv`, or `__progname`. All five scored outputs
  and calibration are identical, but the optional/args/RTG/Workbench paths are
  the real risk surface.
- **Rejected alternatives**: trust the benchmark outputs (too narrow); add
  `--gc-keep-exported` (defeats the DCE win); skip the full verify (not
  acceptable); rely on debug-only Workbench gates (they do not exercise GC).
- **Strongest counterargument**: the removed constructors are a real semantic
  change for programs that use libnix environment/path state; the current
  stdlib does not, but a future API could. The audit makes the risk visible
  and a `KEEP` is the fallback.
- **Confidence**: Medium-High — the benchmark and corpus evidence is strong;
  the release Workbench and 1.3 corpus legs are the remaining proof.
- **Hardening**: correctness review + emission-integrity auditor + full
  verify + AR-20; a failure stops the landing.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: any verify leg shows a missing constructor/table, or a
  future stdlib API uses libnix environment/path state.

### AR-14 — Documentation, issues, and roadmap

- **Status**: ✅ Resolved
- **Authority**: User authorized registering remaining options as issues; AI
  delegates the documentation mechanism.
- **Decision**: update `benchmarks/README.md` (shipped flags, attribution, and
  the `m68k_release_bytes` metric), `docs/known-limitations.md` (programs
  built by `clorraine build --release` on Linux host, hosted-amiga, or
  hosted-amiga-13 are stripped; freestanding and non-Linux host keep symbols;
  use a debug build for symbols), the production-readiness ambiguity register
  entry required by that document, the `--release` CLI help, and the plan
  findings. Register four deferred issues: LTO/LTO+GC (one issue), an opt-in
  `-Os` size mode, macOS `-dead_strip`, and freestanding strip/GC. Do not
  duplicate the existing 68020/FPU issue #15. Update the roadmap through the
  CodeOps skills.
- **Evidence**: `benchmarks/README.md` describes the harness;
  `docs/known-limitations.md:3-5` requires a matching production-readiness
  register entry; `compiler/src/clorraine/cli.rs` describes `--release`;
  the prior session registered #13–#19.
- **Rejected alternatives**: no docs (the shipped flags and the stripped
  release are user-visible); duplicate issues (noise); a new perf document
  (the benchmark README and findings are enough); leave the CLI help stale
  (it would describe only `-O2`).
- **Strongest counterargument**: four new issues may be more than needed. Each
  is a distinct measured lever with a reopen trigger; combining them would
  recreate the unbounded goal the prior session split. LTO and LTO+GC are one
  evaluation, so they share one issue.
- **Confidence**: High.
- **Hardening**: the roadmap review at closeout checks links and status.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a deferred lever becomes the next selected goal.

### AR-15 — Versioning and publication

- **Status**: ✅ Resolved
- **Authority**: User — push only when green (standing goal); no version bump
  or release publication is authorized by this feature.
- **Decision**: commit the feature on `main` when the full gate is green and
  push the branch. Do not bump the version, tag, publish crates/npm, or
  rewrite `RELEASE.md`.
- **Evidence**: the repository's release process is separate; the prior
  closeout pushed code without tagging.
- **Rejected alternatives**: tag a release (not asked); publish (external
  action); leave the branch local (the standing goal says push when green).
- **Strongest counterargument**: the feature changes shipped release flags;
  a release note would help users. The roadmap/findings carry the record; a
  release is a separate user decision.
- **Confidence**: High.
- **Hardening**: `exec-plan --auto-commit` plus the final push check.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the user asks for a release.

### AR-16 — Review dispatch

- **Status**: ✅ Resolved
- **Authority**: AI — delegated dispatch per `AGENTS.md` and the quality
  profile.
- **Decision**: dispatch the correctness reviewer and the read-only
  emission-integrity auditor for every phase that changes the build command or
  measurement, plus a design challenge on the audit/attribution design. Do not
  add a memory-safety specialist (no ownership, drop, or emitted-code change),
  but keep the ASan/UBSan gates. No new specialist agent: the existing roles
  cover the work; the specialist check records `None` with evidence.
- **Evidence**: `AGENTS.md` phase-review dispatch; the change surface is
  `toolchain.rs`, `bench.py`, `bench-shape.sh`, tests, and docs, not codegen.
- **Rejected alternatives**: add a build-toolchain specialist (no recurring
  unique evidence; a dynamic packet suffices); skip emission review (the
  feature claims no emitted-C change; the auditor proves it); use the
  hot-path `performance-auditor` (it reviews runtime code, and this feature
  changes no runtime code).
- **Strongest counterargument**: link-time DCE is close to codegen; the
  emission-integrity auditor's base/head emission diff is exactly the right
  proof.
- **Confidence**: High.
- **Hardening**: the findings ledger records every review ruling.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a reviewer finds a codegen or ownership change.

### AR-17 — Benchmark workloads stay frozen

- **Status**: ✅ Resolved
- **Authority**: User — no benchmark tuning.
- **Decision**: keep every `benchmarks/*/program.clo` and
  `expected-output.txt` byte-identical. The flag audit uses the existing
  scored suite plus the 1.3 corpus. Do not add a benchmark that only
  demonstrates the flag win.
- **Evidence**: `benchmarks/README.md`; the prior feature's baseline rule.
- **Rejected alternatives**: add a huge-table benchmark (tuning); change
  expected outputs (invalidates history); measure only a hello world (not
  representative).
- **Strongest counterargument**: the existing suite's startup share is small,
  so the GC win is mostly binary size, not runtime. That is the product goal;
  the 444-instruction startup win is recorded separately.
- **Confidence**: High.
- **Hardening**: the findings compare source hashes; the harness output
  checksums stay.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the user approves a new workload.

### AR-18 — Out-of-scope and deferred candidates

- **Status**: ✅ Resolved
- **Authority**: User — the prior session registered the remaining options;
  this feature does not expand scope.
- **Decision**: out of scope: `-O3`, `-Os` as the default,
  `-fomit-frame-pointer`, LTO/LTO+GC (one deferred issue), 68020/68040/FPU
  profiles (#15), a public size mode, macOS host flags, freestanding
  GC/strip, shim splitting, `-ffast-math`/safety-flag removal, and benchmark
  changes. Deferred items get issues or are already covered by #13–#19.
- **Evidence**: the measured table in AR-04; the open issue list.
- **Rejected alternatives**: bundle every lever into this feature (the reason
  the prior session split them); silently drop a candidate (no record).
- **Strongest counterargument**: LTO+GC gives a small extra win and could be
  included; it adds link-time cost and toolchain risk for 2–11% beyond GC,
  so it is a separate decision.
- **Confidence**: High.
- **Hardening**: the roadmap/findings link each deferred item.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a deferred item is selected as the next goal.

### AR-19 — Frozen compile-matrix spec update

- **Status**: ✅ Resolved
- **Authority**: **User** — direct ruling on 2026-11-10: "Update the two
  release rows." Auto-design cannot approve a frozen-spec change.
- **Objective**: keep `spec/17-codegen.md` §13 truthful as the exact compile
  matrix after the shipped release flags change.
- **Decision**: update the two GCC release rows:
  - `hosted-amiga`: add `-ffunction-sections -fdata-sections
    -Wl,--gc-sections -s`;
  - `hosted-amiga-13`: the same addition.
  Keep the debug rows, the vbcc row, and the freestanding rows unchanged.
  The compile matrix remains the acceptance test; the `toolchain.rs` module
  doc is updated to match.
- **Evidence**: `spec/17-codegen.md:384-397` pins the exact commands and says
  "the compile matrix (section 13) is the acceptance test"; the current
  release rows are `-O2` only. The user chose the row update over a separate
  shipped-release section or keeping the spec and dropping the m68k win.
- **Rejected alternatives**: add a separate shipped-release section (two
  release definitions can drift); keep the spec and drop the m68k change
  (loses the 57–92% m68k win); change the spec silently (violates the frozen
  oracle rule).
- **Strongest counterargument**: editing a frozen spec is exactly what the
  repository normally forbids. The user explicitly ruled for this specific
  row update because the release command is an implementation detail that the
  spec already treats as the acceptance test; no language or ABI behavior
  changes.
- **Confidence**: High — explicit user ruling.
- **Hardening**: the spec change is reviewed with the build-command phase; the
  command-vector tests and `spec/check.sh` keep the chapter links valid.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the user reverses the ruling, or a future flag change
  needs another spec edit.

### AR-20 — Release-path gate coverage

- **Status**: ✅ Resolved
- **Authority**: AI — delegated tooling/gate decision within the user's
  authorized toolchain changes.
- **Objective**: prove the flags on the paths that actually ship release
  binaries, not only debug builds.
- **Decision**: keep the existing warning-bearing debug build in
  `sdk-advanced-build`, `mkdir -p build/amiga-sdk`, copy the debug binaries
  there, and add a release build/leg for the Workbench and services demos, so
  the existing `sdk-a600-services` and `sdk-a1200-workbench` gates run
  GC+strip binaries against their report lines and goldens without losing the
  strict-warning compile. Run the four named
  1.3 corpus programs as release+GC binaries under `vamos` in the audit and
  record their outputs. Do not add a new broad harness.
- **Evidence**: `Makefile` `sdk-advanced-build` builds without `--release`;
  `docker/scripts/corpus-amiga.sh` and `differential-seed.sh` hard-code
  `-O0 -g`; the Workbench/AppIcon path is the largest optional surface and is
  currently debug-only. The probe builds the four 1.3 binaries and runs
  `volatile-mmio`; the audit is the first full run of all four.
- **Rejected alternatives**: rely on `make verify` as-is (it does not exercise
  release+GC for Workbench or the 1.3 corpus); add a new release matrix target
  (overkill); skip Workbench (the riskiest surface).
- **Strongest counterargument**: changing `sdk-advanced-build` to release
  could alter the debug workflow. The gates compare behavior, not debug
  convenience; a release build is the product artifact.
- **Confidence**: High.
- **Hardening**: correctness review; the existing a600/a1200 goldens are the
  proof; a failure stops the landing.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a gate cannot build Workbench in release mode, or the
  release binary changes a golden.

### AR-21 — Failure handling and floor governance

- **Status**: ✅ Resolved
- **Authority**: User owns "no regression / measured win"; AI delegates the
  procedural handling below.
- **Decision**: a missed size floor stops the landing and is escalated; the
  floor is never lowered to pass. A flaky gate may be rerun once with the
  failing leg and its output recorded; a reproducible failure stops. A
  `KEEP`/`-u` added for a constructor requires a fresh audit and re-check of
  the floors. The pinned base hash is authoritative even if the branch head
  moves; `bench-base` fails loudly if the commit is unreachable.
- **Evidence**: the prior closeout hit two environmental FS-UAE flakes and
  reran the legs; the pinned base is a full hash.
- **Rejected alternatives**: silently rerun until green (hides defects);
  lower a floor under time pressure (violates the user's goal); follow the
  moving branch head (destroys attribution).
- **Strongest counterargument**: a single rerun can still hide a flake. The
  recorded failing leg and output make the rerun auditable; a second failure
  stops.
- **Confidence**: High.
- **Hardening**: the findings record every rerun and every floor result.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a gate flakes more than once, or a floor is missed.

### AR-22 — Flag-list consistency

- **Status**: ✅ Resolved
- **Authority**: AI — delegated test/measurement mechanism.
- **Objective**: prevent the driver and the benchmark harness from describing
  different release builds.
- **Decision**: add a harness test that compares `bench.py`'s
  `HOST_RELEASE`/`M68K_RELEASE` vectors with the driver's release vectors (by
  reading the Rust constants or a small shared manifest). The audit
  procedure's new-flag array comes from those same harness constants (or the
  shared manifest), so there is no fourth independent copy. Keep the shape
  gate's warning-bearing copy as a gate-only variant and state that it is not
  the product build. The Rust and Python unit tests continue to pin the exact
  strings.
- **Evidence**: the driver (`toolchain.rs:36-48`), the harness
  (`bench.py:37-41`), and the shape gate (`bench-shape.sh:27-28`) each hold a
  copy; no test compares them today (PF-020).
- **Rejected alternatives**: accept the duplication (drift makes `head.json`
  describe a non-product build); generate one file for both (overkill for two
  vectors); remove the harness copy and import Rust (not possible in Python).
- **Strongest counterargument**: a test that parses Rust source is brittle.
  The parse is a narrow literal extraction, and the alternative is silent
  drift; the test fails loudly if the format changes.
- **Confidence**: High.
- **Hardening**: `make bench-test` runs the comparison; the Rust test pins
  the driver side.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the flag lists become generated from one source.

### AR-23 — Audit script shape

- **Status**: ✅ Resolved
- **Authority**: AI — delegated measurement mechanism.
- **Decision**: add `tools/perf/flag_audit.py` (Python standard library),
  invoked by `make bench-flags`. It imports the `bench` helpers and release
  constants, emits C with `clc`, builds old/GC/new variants with complete
  compiler argument arrays before `-o` and a separate `libs` field (the
  rejected candidates are measured once in Phase 1), records the
  matrix keyed by `<profile>/<program>` as JSON/TSV under `build/perf/`, and
  has a `--check` mode that checks every floor group, prints every miss, and
  exits nonzero. It runs the repeat set twice in one invocation.
- **Evidence**: `tools/perf/bench.py` already has `run_command`,
  `size_sections`, `trace_vamos`, and `normalize_output`; the audit needs
  custom flags, which the current `measure` does not expose. A shell script
  would make the floor assertions and repeatability harder to test.
- **Rejected alternatives**: extend `bench.py` with a second flag-set mode
  (more surface in the scored harness); a copy-paste command block (not a
  gate); a bash script (weaker floor assertions); a new framework (overkill).
- **Strongest counterargument**: a second perf script is surface. It is a
  small, single-purpose gate with unit tests, and the requirements make it the
  attribution record; without it the win is a manual table.
- **Confidence**: High.
- **Hardening**: unit tests on synthetic matrices; the design challenge
  reviews the method; the findings record the two identical runs.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the audit needs to run several flag sets routinely.

### AR-24 — Release Workbench build placement

- **Status**: ✅ Resolved
- **Authority**: AI — delegated tooling mechanism within AR-20.
- **Decision**: `sdk-advanced-build` builds the Workbench and services demos
  in debug mode to the existing default paths (the warning-bearing compile),
  copies those binaries to `build/amiga-sdk/workbench-debug` and
  `build/amiga-sdk/services-debug`, then rebuilds them with `--release` to the
  default paths. The `sdk-a600-services` and `sdk-a1200-workbench` gates run
  the release binaries; the debug copies remain for developers.
- **Evidence**: `Makefile:294-303` builds debug and the gates default to
  `build/clorraine/<stem>/hosted-amiga/<stem>`; `--release` overwrites the
  same path (`compiler/src/clorraine/artifacts.rs:32-34,55-62`).
- **Rejected alternatives**: replace debug with release (loses the
  `-Wall -Wextra -Werror` compile); add a second target with duplicated build
  commands (more surface); switch the gates to a copied release path (the
  scripts already take arguments, but the default path is the smallest
  change).
- **Strongest counterargument**: the build now compiles the demos twice. The
  demos are small and the release gate is the product proof; the debug copy
  keeps the warning gate.
- **Confidence**: High.
- **Hardening**: the two existing gates are the proof; a failure stops.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: a gate needs the debug binary or the build time becomes
  a problem.

### AR-25 — Cross-language flag check

- **Status**: ✅ Resolved
- **Authority**: AI — delegated test mechanism.
- **Decision**: the Python harness test parses the named Rust release
  constants (`HOST_RELEASE_GNU`, `HOST_RELEASE_FALLBACK`,
  `M68K_RELEASE_FLAGS`, `FREESTANDING_RELEASE_FLAGS`) with a narrow literal
  regex and compares them, composed with their base flags, to the `bench`
  vectors and the complete audit arrays, including the `-mcrt=nix13` form.
- **Evidence**: `tools/perf/tests/test_bench.py` already imports `bench`;
  the driver constants are string-literal slices in `toolchain.rs`.
- **Rejected alternatives**: a generated shared manifest (overkill for four
  constants); a Rust test invoking Python (cross-language complexity);
  accepting duplication (silent drift).
- **Strongest counterargument**: parsing source is brittle. The parse is
  narrow and fails loudly if the format changes; the alternative is a harness
  that measures a build the product no longer ships.
- **Confidence**: High.
- **Hardening**: `make bench-test` runs the check; the Rust tests pin the
  same literals.
- **Policy version**: 1
- **Root invocation ID**: `build-flags-closeout-20261110`
- **Reopen triggers**: the constants move or become generated.

## Gate

**✅ GATE PASSED** — every material decision is resolved. The reserved
frozen-spec change has the user's explicit 2026-11-10 ruling (AR-19); every
other entry is an eligible delegated technical decision recorded under policy
version 1. No material item is open or deferred into the executable plan.
