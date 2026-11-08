# Requirements Preflight: DX and Adoption Closeout

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 15 findings (2 critical, 10
> major, 2 minor, 1 observation); all critical/major resolved in the revised
> requirements and register; 2 minor accepted as documented limits; the
> observation is out of scope
> **Iteration**: 1 (first scan; requirements and register revised in the same
> pass)
> **Artifact**: requirements set at `plans/dx-adoption-closeout/`
> (`00-ambiguity-register.md`, `01-requirements.md`)
> **Codebase Grounded**: 24 source files examined; every finding cites a
> `compiler/src`, `docker/`, `tools/`, `editors/`, or `Makefile` location
> **Last Updated**: 2026-11-08
> **Same-session review**: ⚠️ the requirements were authored in this session.
> The scan was grounded in the code and re-read the driver, loader, CLI,
> parser spans, Dockerfile, and workflows. A fresh-session review is
> recommended before execution if schedule permits.

## Codebase Context Summary

**Tech Stack:** Rust 2021 workspace (std only, no external crates), C99 runtime
shims, embedded Clorraine stdlib, Python-stdlib tooling, Docker SDK, npm-only
tooling under `tools/demo/`.

**Architecture:** `clc` loads user and embedded stdlib sources, runs parse →
load → resolve → sema → ownership → one C translation unit. `clorraine` fronts
the compiler in process and owns the profile toolchain. `load::load` merges
files that declare the same module and always checks the executable entry
point. The CLI parses `--explain` as an early-exit mode before the command
word. The Docker image builds the m68k toolchain and NDK from a pinned
`amiga-gcc` commit.

**Key Files Examined:**
`compiler/src/driver.rs`, `compiler/src/lib.rs`, `compiler/src/load/mod.rs`,
`compiler/src/resolve/mod.rs`, `compiler/src/span.rs`,
`compiler/src/diag.rs`, `compiler/src/clorraine/cli.rs`,
`compiler/src/clorraine/mod.rs`, `compiler/src/clorraine/build.rs`,
`compiler/src/ast/mod.rs`, `compiler/src/ast/expr.rs`,
`compiler/src/parse/mod.rs`, `compiler/src/lex/mod.rs`,
`compiler/src/profile.rs`, `compiler/src/stdlib.rs`,
`compiler/Cargo.toml`, `Cargo.lock`, `Makefile`, `docker/Makefile`,
`docker/Dockerfile`, `docker/versions.env`,
`docker/scripts/init-env.sh`, `editors/clorraine.tmLanguage.json`,
`tools/demo/package.json`, `.github/workflows/check.yml`,
`examples/amiga/` and `spec/01-lexical.md`.

**Reference Verification:** 22 references mapped; 22 verified against code.
Two were corrected during the scan: the LSP analysis unit (whole-workspace scan
would merge independent programs) and the default profile (host would reject
every Amiga module).

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|---|---|---|
| 1 | Ambiguities | 1 | 🟠 major |
| 2 | Implicit Assumptions | 4 | 🔴 critical |
| 3 | Logical Contradictions | 1 | 🟠 major |
| 4 | Completeness Gaps | 2 | 🟠 major |
| 5 | Dependency Issues | 1 | 🟠 major |
| 6 | Feasibility Concerns | 2 | 🟠 major |
| 7 | Testability | 2 | 🟠 major |
| 8 | Security Blind Spots | 2 | 🟠 major |
| 9 | Edge Cases | 1 | 🟡 minor |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 minor |
| 13 | Codebase Alignment | 2 | 🔴 critical |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 2 | all resolved |
| MAJOR | 10 | all resolved |
| MINOR | 2 | accepted as documented limits |
| OBSERVATION | 1 | out of scope, recorded |

---

## Critical Findings

### PF-001: Whole-workspace analysis merges independent programs 🔴 CRITICAL

**Dimension:** 13 — Codebase Alignment / Architecture Mismatch
**Location:** R7.1 (original), AR-7.
**Codebase Evidence:** `compiler/src/load/mod.rs:150-205` groups files by
declared module name and merges them; `check_entry_point` requires exactly one
`main`. The repository's `examples/amiga/*.clo` are independent programs, each
with its own module name and `main`; `examples/hello/hello.clo` likewise.
Analyzing every `*.clo` under the repository root as one program would merge
files that are not one program and flood the editor with false duplicate and
entry-point errors.
**The Problem:** The requirement's "scan and analyze the workspace" model is
invalid for this project's layout.
**Resolution:** R7.1 now separates the **module index** (a bounded scan used
for import lookup) from the **analysis set** (the requested document plus the
transitive closure of its imports and qualified module references). AR-7 and
R7.2 were updated. A fixture workspace with two independent programs plus one
imported library proves the closure and the absence of cross-program errors.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-002: Default `host` profile rejects every Amiga module 🔴 CRITICAL

**Dimension:** 2 — Implicit Assumptions
**Location:** R6.5 (original), R6.6, AR-8a.
**Codebase Evidence:** `compiler/src/profile.rs` gates module availability;
`compiler/src/driver.rs:260-310` reports `E11200` for an embedded stdlib module
the selected profile does not provide. The project's primary target is
`hosted-amiga`/`hosted-amiga-13`; `examples/amiga/*.clo` import
`std.amiga.*`. A host-default server would report `E11200` on almost every
Amiga source file.
**The Problem:** The server would be unusable for the audience the tool exists
to serve.
**Resolution:** R6.6 and AR-8a add a profile option
(`initializationOptions.profile` / `clorraine.profile`) with a
`hosted-amiga` default; the four existing profile names are accepted; an
unknown name is `-32602`. The LSP fixture sets `host` explicitly to keep
diagnostics stable.
**User Decision:** Resolved — AI delegated (technical mechanism; the product
default follows the project's stated Amiga-first target).

## Major Findings

### PF-003: Installer claims a SHA-256 check it does not perform 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** R2.2 (original), AR-29.
**Codebase Evidence:** `docker/Dockerfile:55-82` delegates toolchain and NDK
download to the pinned `amiga-gcc` build; the upstream build verifies archives
where checksums are published. The installer itself would run
`make -C docker image` and never touch a downloaded byte.
**The Problem:** "Verify downloaded artifacts with SHA-256" read as an
installer-owned check, but the installer owns no download; the requirement
would be unverifiable and the acceptance claim false.
**Resolution:** R2.2 and AR-29 now state the boundary explicitly: the image
build is the only downloader, upstream SHA-256 checks apply where published,
and the installer's own fail-closed gate is provenance (pinned commit), NDK
tree presence, and tool versions. A test covers a provenance mismatch with all
other checks passing.
**User Decision:** Resolved — AI delegated (technical boundary).

### PF-004: `--doc` cannot reuse the existing `emit` parser unchanged 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment / Phantom Reference
**Location:** R11.1, AR-17.
**Codebase Evidence:** `compiler/src/clorraine/cli.rs:112-150` resolves
`--explain`, `--help`, and `--version` before the command word; every other
argument goes through `parse_options`, which requires a command and validates
`-o` only for `emit`.
**The Problem:** "An early-exit mode like `--explain`" needs its own argument
scan; simply adding `--doc` to the command list would require a fake command
and would inherit invalid validation.
**Resolution:** R11.1 and AR-17 specify a dedicated early-exit action that
accepts only `FILE...` and an optional `-o DIR`, rejects `--profile` and
command words, and never enters `parse_options`. The help text lists it.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-005: Library analysis needs an entry-point option 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment / Stale Assumption
**Location:** R8.3, AR-8.
**Codebase Evidence:** `compiler/src/load/mod.rs:150,204,476-510` calls
`check_entry_point` unconditionally and reports `E1…`-class "executable builds
need exactly one `func main(): i32`" when absent.
**The Problem:** A library module opened in the editor would always show a
false entry-point error, and "analyze a workspace module" would be untestable
without a fake `main`.
**Resolution:** R8.3 and AR-8 add a `check_entry_point` parameter to the shared
entry point; the server passes `false`, `clc` keeps `true`. The existing load
function keeps its behavior via a delegating wrapper, so no caller changes.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-006: The shared-pipeline refactor could drift from driver behavior 🟠 MAJOR

**Dimension:** 3 — Logical Contradictions
**Location:** R8.2, R8.4, R14.4.
**Codebase Evidence:** `compiler/src/driver.rs:330-410` owns the exact
sequence: profile availability, unavailable-module check, stdlib injection to
a fixpoint, load, resolve, sema gated on zero errors, ownership gated on zero
errors, emit, diagnostics, output. Moving any step changes diagnostics and
exit codes.
**The Problem:** The requirement says "reuse" but did not name the oracle or
the gates that prove the refactor is behavior-preserving.
**Resolution:** R8.2/R8.4 now require the shared entry point to own exactly
that sequence; `driver::run` calls it; every existing suite, corpus emission,
and differential gate is the oracle; R14.4 runs the full set. The task list
adds a base/head emitted-C diff over the corpus.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-007: LSP tests can hang on a stuck child 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** R7.3, R14.5, AR-40.
**Codebase Evidence:** Existing binary tests use `Command::output()` with no
timeout; an LSP test must stay alive across many frames, so a missing response
would block the test process.
**The Problem:** "Without hanging" is a requirement, but the test harness had
no bounded read.
**Resolution:** R14.5 and AR-40 require a per-request 10-second read timeout,
child kill on failure, and `Drop` cleanup. The integration client wraps the
child in a struct that kills it.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-008: The `.vsix` cannot carry the Rust server 🟠 MAJOR

**Dimension:** 5 — Dependency Issues
**Location:** R9.5, R10.2.
**Codebase Evidence:** The extension package is JavaScript/TypeScript only; the
LSP is a Rust binary built by Cargo. A `.vsix` cannot embed a platform binary
without a per-platform release matrix.
**The Problem:** The packaged client test named "the real `clorraine-lsp`" but
did not say how it finds it.
**Resolution:** R9.5 defines the resolution order (`clorraine.lsp.path`,
`CLORRAINE_LSP`, workspace `target/{debug,release}`, `PATH`); R10.2 uses
`CLORRAINE_LSP`. The test builds the binary first in `make vscode-check`.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-009: Grammar staging could dirty the tree 🟠 MAJOR

**Dimension:** 4 — Completeness Gaps
**Location:** R9.3, R9.4.
**Codebase Evidence:** `editors/clorraine.tmLanguage.json` is the only
committed grammar; `tools/demo` consumes it directly. A copy under
`tools/vscode/syntaxes/` would be untracked unless ignored.
**The Problem:** "Packaged grammar is a byte-identical copy" did not say where
the copy lives or how the tree stays clean.
**Resolution:** R9.3/R9.4 stage the package under `build/vscode/` (gitignored),
copy the canonical grammar there, and run `vsce package` from the staging
directory. `tools/vscode` holds no generated grammar.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-010: File-only URI policy rejects new unsaved files 🟠 MAJOR

**Dimension:** 9 — Edge Cases
**Location:** R7.2, N2, AR-12.
**Codebase Evidence:** VS Code represents a new unsaved buffer as an
`untitled:` URI; the LSP `didOpen` carries its full text. The original policy
accepted only `file:`.
**The Problem:** The requirement "handle unsaved buffers" contradicted the URI
policy.
**Resolution:** R7.2 and N2 accept overlay-only `untitled:` documents;
workspace scans still index only `file:` paths; goto to another file still
works from the index.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-011: Module names could escape the document output root 🟠 MAJOR

**Dimension:** 8 — Security Blind Spots
**Location:** R11.5, AR-21.
**Codebase Evidence:** `QualifiedName.segments` are parsed identifiers, but the
module name is also read from source text; the generator turns it into a file
name. `Example.Module` is safe; a crafted `..` segment is not a valid
identifier but must be rejected defensively.
**The Problem:** Path traversal through an output name is a security defect.
**Resolution:** R11.5 and AR-21 validate the dotted name against
`[A-Za-z_][A-Za-z0-9_.]*`, reject empty segments, `..`, separators, and a
leading dot, and canonicalize the joined output path before writing.
**User Decision:** Resolved — AI delegated (technical mechanism).

### PF-012: Installer test injection could become a production backdoor 🟠 MAJOR

**Dimension:** 8 — Security Blind Spots
**Location:** R3.2, AR-32.
**Codebase Evidence:** The original text allowed "a documented option or
environment variable" to inject a fake build runner. A production environment
variable that redirects an installer's command execution is an attack surface.
**The Problem:** Test convenience would weaken the installer's integrity.
**Resolution:** R3.2 and AR-32 allow only a fake `docker` on the test `PATH`
plus unit-test monkeypatching of the single command-runner function. No
production flag or environment variable exists.
**User Decision:** Resolved — AI delegated (technical mechanism).

## Minor Findings

### PF-013: Installer tests would not run in CI 🟡 MINOR

**Dimension:** 7 — Testability
**Location:** R14.1.
**Codebase Evidence:** `.github/workflows/check.yml` runs `./spec/check.sh`
and `cargo test --workspace` only.
**Resolution:** R14.1 now adds the installer unit tests to the host CI job.
The LSP and `--doc` suites ride the existing workspace tests. VS Code tests
stay in `make vscode-check` because they need Node, which CI does not install;
this is accepted as a documented local gate.
**User Decision:** Resolved — AI delegated.

### PF-014: VS Code checks are not part of `make check` 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** R14.1.
**Codebase Evidence:** `make check` is the Docker-only fast gate; `make
demo-check` is host-only because the demo needs Node. `tools/vscode` has the
same Node constraint.
**Decision:** Accepted as a documented limit: `make vscode-check` is a
host-only gate, named next to `make demo-check` in `AGENTS.md` and the final
report. Adding Node to the Docker image would expand the pinned toolchain for
one test lane.
**User Decision:** Accepted — documented limit.

## Observations

### PF-015: Release artifacts do not list the LSP binary 🔵 OBSERVATION

**Dimension:** 10 — Scope Creep Indicators
**Location:** `RELEASE.md`, future release packaging.
**Evidence:** The v1.1.0 binary tarball contains `clc` and `clorraine` only.
Adding `clorraine-lsp` changes the release contents and the runbook.
**Decision:** Out of scope for this plan; the release runbook is updated when a
future release packages the server. The binary is built and documented by
`make vscode-check`/`make check`. Recorded so it is not forgotten.

---

## Verdict

✅ **PREFLIGHT PASSED WITH NOTES** — all 2 critical and 10 major findings are
resolved in the requirements and register; PF-014 is an accepted minor limit;
PF-015 is an out-of-scope observation. The requirements are ready for
`make-plan`.
