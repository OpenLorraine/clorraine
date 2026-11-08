# Plan Preflight: DX and Adoption Closeout

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 16 findings (2 critical, 9
> major, 4 minor, 1 observation); all critical/major resolved in the plan
> documents; 4 minor fixed in place; the observation is out of scope
> **Iteration**: 1 (first scan; plan revised in the same pass)
> **Artifact**: implementation plan at `plans/dx-adoption-closeout/`
> (`00-index.md`, `01-requirements.md`, `00-ambiguity-register.md`,
> `02-current-state.md`, `03-01` to `03-06`, `07-testing-strategy.md`,
> `99-execution-plan.md`)
> **Codebase Grounded**: 27 source files examined; every finding cites a
> `compiler/src`, `docker/`, `tools/`, `Makefile`, or `.github/` location
> **Last Updated**: 2026-11-08
> **Same-session review**: ⚠️ the plan was authored in this session. The scan
> re-read the driver, loader, CLI, AST spans, Dockerfile, Makefile, host
> toolchain availability, and the npm tooling. A fresh-session review is
> recommended before execution if schedule permits.

## Codebase Context Summary

**Tech Stack:** Rust 2021 workspace (std only), C99 runtime shims, embedded
Clorraine stdlib, Python-stdlib tooling, Docker SDK, npm-only tooling.

**Architecture:** `clc` = library plus two binaries. `driver::run` owns the
front-end sequence. `clorraine` fronts the compiler and the C toolchain.
`load::load` merges same-module files and checks the executable entry point.
The Docker image is built from a pinned `amiga-gcc` commit. Node exists on the
host only through nvm; the host has no Rust toolchain.

**Key Files Examined:**
`compiler/src/driver.rs`, `compiler/src/lib.rs`, `compiler/src/load/mod.rs`,
`compiler/src/sema/mod.rs`, `compiler/src/own.rs`,
`compiler/src/codegen.rs`, `compiler/src/span.rs`,
`compiler/src/clorraine/{cli,mod,build}.rs`, `compiler/src/ast/mod.rs`,
`compiler/src/parse/mod.rs`, `compiler/src/profile.rs`,
`compiler/src/stdlib.rs`, `compiler/Cargo.toml`, `Cargo.lock`, `Makefile`,
`docker/{Makefile,Dockerfile,versions.env}`, `docker/scripts/init-env.sh`,
`editors/clorraine.tmLanguage.json`, `tools/demo/{package.json,tsconfig.json}`,
`.github/workflows/check.yml`, `spec/01-lexical.md`, `examples/amiga/`.

**Reference verification:** 31 references mapped; 31 verified. Corrections
made during the scan: the analysis result shape, the parse result shape, the
malformed-frame path, and the host Rust toolchain assumption.

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|---|---|---|
| 1 | Ambiguities | 1 | 🟡 minor |
| 2 | Implicit Assumptions | 3 | 🔴 critical |
| 3 | Logical Contradictions | 2 | 🟠 major |
| 4 | Completeness Gaps | 2 | 🟠 major |
| 5 | Dependency Issues | 2 | 🟠 major |
| 6 | Feasibility Concerns | 2 | 🟠 major |
| 7 | Testability | 2 | 🟠 major |
| 8 | Security Blind Spots | 1 | 🟡 minor |
| 9 | Edge Cases | 1 | 🟠 major |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 | 🟡 minor |
| 12 | Consistency | 1 | 🟡 minor |
| 13 | Codebase Alignment | 2 | 🔴 critical |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 2 | all resolved |
| MAJOR | 9 | all resolved |
| MINOR | 4 | all fixed |
| OBSERVATION | 1 | out of scope, recorded |

---

## Critical Findings

### PF-001: `Analysis` dropped the stage outputs the driver needs 🔴 CRITICAL

**Dimension:** 13 — Codebase Alignment / Impact Blindness
**Location:** [03-02](03-02-shared-analysis.md) `Analysis`, §Behavior preservation.
**Codebase Evidence:** `compiler/src/driver.rs:380-395` stores
`Option<(SemaOutput, OwnershipOutput)>` and passes both to
`codegen::emit(&loaded, sema, ownership, kind, &sources, &mut diagnostics)`
(`compiler/src/codegen.rs:360-367`). The first draft exposed only a `typed`
boolean.
**The Problem:** The driver could not emit C through the new API; the refactor
was not feasible as specified.
**Resolution:** `Analysis.typed` is now
`Option<(SemaOutput, OwnershipOutput)>`; the driver passes it to the emitter.
Both stage outputs own their data (no lifetime added).
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-002: `analysis::parse` returned no file map 🔴 CRITICAL

**Dimension:** 13 — Codebase Alignment / Completeness
**Location:** [03-02](03-02-shared-analysis.md) `parse`.
**Codebase Evidence:** `SourceMap::add_file` returns a `FileId`; a `Program`
alone does not name its file. `compiler/src/span.rs:69-128` provides no reverse
lookup. The doc generator must read the source text for a declaration's span
and write module pages keyed by path.
**The Problem:** The documented tuple `(SourceMap, Vec<(FileId, Program)>,
Diagnostics)` forced the caller to guess which path belongs to which id.
**Resolution:** `parse` returns a `Parsed` struct with `sources`, `files`
(path -> id), `programs`, and `diagnostics`.
**User Decision:** Resolved — AI delegated (technical mechanism).

## Major Findings

### PF-003: A malformed frame had no path to an error response 🟠 MAJOR

**Dimension:** 3 — Logical Contradiction
**Location:** [03-03](03-03-language-server.md) §Framing, §Server lifecycle.
**Codebase Evidence:** The design gives stdin to a reader thread and stdout to
the handler. The reader detects a bad frame and cannot write a response; the
first draft told "the server" to write one without a channel member.
**The Problem:** The error path was unimplementable as written.
**Resolution:** The reader sends `Incoming::Invalid(reason)`; the handler
writes one `-32700`/`id:null` response when possible and exits 1.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-004: The host has no Rust toolchain 🟠 MAJOR

**Dimension:** 5 — Dependency Issues
**Location:** [03-05](03-05-vscode-extension.md) §Headless tests; R10.3.
**Codebase Evidence:** `cargo` and `rustc` are not on the host `PATH`; every
Rust build in this repository runs through `make -C docker exec`. Node is
available only through nvm, outside the container.
**The Problem:** `make vscode-check` cannot build `clorraine-lsp` on the host
as originally written.
**Resolution:** R10.3 and 03-05 now build the workspace in the pinned container
(bind-mounted `target/`), then run the Node checks on the host with
`CLORRAINE_LSP` pointing at `target/debug/clorraine-lsp`.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-005: Installer tests need a CI step, not only a Make target 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** [99](99-execution-plan.md) Phase 2; R14.1.
**Codebase Evidence:** `.github/workflows/check.yml` runs only `spec/check.sh`
and `cargo test --workspace`. A host-side Python suite added to `make check`
would not run in CI.
**The Problem:** The installer tests would rot outside the maintainer's
machine.
**Resolution:** Task 2.3.2 and R14.1 add a `python3 -m unittest` step to the
host CI job. The suite needs no Docker daemon.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-006: Completion and symbols could be disabled by a parse error 🟠 MAJOR

**Dimension:** 9 — Edge Cases
**Location:** [03-03](03-03-language-server.md) §Features.
**Codebase Evidence:** `driver.rs:340-350` drops a program that added a parse
error, because later stages need trustworthy names. An editor still needs
symbols and keywords while the user types an incomplete declaration.
**The Problem:** The first draft tied all features to a clean analysis.
**Resolution:** The feature section now separates the partial parse (symbols,
completion) from the clean analysis (hover, definition); the integration case
ST-32 covers an `untitled:` error document.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-007: The `--doc` path-safety case cannot be built from a `.clo` fixture 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** [07](07-testing-strategy.md) ST-43; [03-04](03-04-doc-generator.md) §Output safety.
**Codebase Evidence:** The parser accepts only identifier segments in a module
name; `..` or `a/b` produces a syntax error before the generator sees it.
**The Problem:** ST-43 as written would test the parser, not the generator's
path guard.
**Resolution:** ST-43 is tested at the generator's page-name function, with a
crafted invalid name, while ST-44 covers the parse-error path through the CLI.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-008: Installer probe timeouts would kill the image build 🟠 MAJOR

**Dimension:** 3 — Logical Contradiction
**Location:** [03-01](03-01-installer.md) §Probe, §Fetch plan.
**Codebase Evidence:** `docker/Dockerfile` builds GCC and the toolchain; a
cold build is far longer than 10 seconds. The first draft applied one timeout
to "every command".
**The Problem:** The fetch could never succeed on a cold machine.
**Resolution:** Only probe commands have a 10-second timeout; the delegated
build and `make init` have no timeout. A test asserts the probe timeout value
and the fetch path's absence of one.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-009: `.env` parsing could become shell evaluation 🟠 MAJOR

**Dimension:** 8 — Security Blind Spots
**Location:** [03-01](03-01-installer.md) §Probe.
**Codebase Evidence:** `docker/.env` is generated by `init-env.sh` with plain
`KEY=value` lines, but a user can edit it. Sourcing it would execute arbitrary
shell.
**The Problem:** The first draft said "reads `.env`" without a rule.
**Resolution:** A literal `KEY=value` parser, comment lines, one pair of
matching quotes stripped, no evaluation; a test covers a value containing
`$(...)`, which must stay text.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-010: The packaged client test cannot `import` a CommonJS module 🟠 MAJOR

**Dimension:** 5 — Dependency Issues
**Location:** [03-05](03-05-vscode-extension.md) §Headless tests.
**Codebase Evidence:** VS Code loads extensions as CommonJS. Node's dynamic
`import()` of a `.js` CommonJS file exposes only statically detected named
exports; a class exported through `exports.LspClient = ...` is not reliably
one of them.
**The Problem:** The packaged test could fail for a module-format reason, not
a behavior reason.
**Resolution:** The test uses `createRequire` to load the extracted
`extension/out/client.js`; the fake-server tests import the TypeScript source
directly.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-011: The LSP profile must be validated before indexing 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** [03-03](03-03-language-server.md) §Server lifecycle.
**Codebase Evidence:** `Profile::parse` returns `None` for an unknown name;
`driver::run` turns that into `E11201`. The server accepts the value from
client-controlled JSON.
**The Problem:** An unknown profile could silently fall back or panic.
**Resolution:** `initialize` validates the profile before indexing and
returns `-32602` with the accepted names. ST-31 covers `host` versus
`hosted-amiga`.
**User Decision:** Resolved — AI delegated (technical mechanism).

## Minor Findings

### PF-012: Stale fixture paths in the index 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** [00-index](00-index.md) Related Files.
**Evidence:** The index first listed `lsp_impl_test.rs` and
`docgen_impl_test.rs`, but the testing strategy puts those cases in
the inline unit tests in `compiler/src/lsp/*.rs` and inline `docgen` unit tests.
**Resolution:** The rows now name the real files. Fixed.

### PF-013: Wrong traceability code in ST-43 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** [07](07-testing-strategy.md) ST-43.
**Evidence:** The source cell read `R21`, which does not exist.
**Resolution:** Corrected to `R11.5, N2`. Fixed.

### PF-014: The doc example linked a built-in type 🟡 MINOR

**Dimension:** 1 — Ambiguity
**Location:** [03-04](03-04-doc-generator.md) example.
**Evidence:** `usize` is a built-in scalar, not a declared type, so it can
never appear in `Related:`.
**Resolution:** The example now links `Buffer`, which the fixture declares.
Fixed.

### PF-015: Task and ST counts drifted during authoring 🟡 MINOR

**Dimension:** 11 — Ordering & Sequencing
**Location:** [99](99-execution-plan.md) header and phase table.
**Evidence:** The header said 52 tasks; the task lines total 56. The ST table
total is 57.
**Resolution:** Header and table now say 56. Fixed.

## Observations

### PF-016: The release tarball does not include the LSP binary 🔵 OBSERVATION

**Dimension:** 10 — Scope Creep Indicators
**Location:** `RELEASE.md`, `build/release/v1.1.0/`.
**Evidence:** The published v1.1.0 binary archive contains `clc` and
`clorraine`.
**Decision:** Out of scope. A future release packages the server; this plan
builds and tests it but does not change the release contents.

---

## Verdict

✅ **PREFLIGHT PASSED WITH NOTES** — all 2 critical and 9 major findings are
resolved in the plan; all 4 minor findings are fixed; PF-016 is an
out-of-scope observation. The plan is ready for `exec-plan`.
