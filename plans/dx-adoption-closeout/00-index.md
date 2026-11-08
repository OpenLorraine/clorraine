# DX and Adoption Closeout (PLAN-21)

> **Feature**: Install the SDK from one entry point, edit Clorraine with a
> language server and VS Code extension, generate Markdown reference from
> `///` comments, and translate common Amiga C patterns with a Rosetta guide.
> **Status**: Executing — 52/56 tasks verified; full gates in progress
> **Created**: 2026-11-08
> **Last Updated**: 2026-11-08
> **Implements**: R1-R14
> **CodeOps Artifact Schema**: 1

**Original goal:** close the developer-experience and adoption gap without
changing the language, specification, ABI, standard library, or compiler
performance.

## Overview

The repository has a complete compiler, driver, and frozen specification but a
weak adoption path: setting up the SDK is manual, editing has no semantic
feedback, documentation lives only in source comments, and a C programmer has
no translation guide.

This plan delivers five independently shippable slices:

- a fetch/detect SDK installer that never redistributes licensed media;
- a std-only LSP and a packaged VS Code extension;
- a deterministic `///`-comment Markdown generator;
- a C-to-Clorraine Rosetta guide and wrapper cookbook;
- the tests, documentation, and gates that keep them honest.

## Minimum-Sufficient Baseline

**Original goal:** make the SDK installable, make editing semantic, make
documentation generated, and make C-to-Clorraine translation teachable.

**Smallest viable design:** reuse what exists. The installer orchestrates the
pinned Docker build instead of adding a downloader. The LSP uses the compiler
library through one new shared entry point and a hand-written JSON layer.
The extension is a thin TypeScript adapter with no runtime dependency.
`--doc` parses with the existing parser and slices source text for exact
signatures. The guide is Markdown.

**Excluded machinery:** a Rust LSP framework, `serde_json`, `lsp-types`,
`vscode-languageclient`, a host toolchain installer, a documentation site
generator, a package manager, and a debugger. All were considered and rejected
as larger than the requested outcome (AR-3, AR-23, AR-28).

**Approved complexity:** none. No new crate, service, daemon, or build system.

## Document Index

| # | Document | Description |
| - | -------- | ----------- |
| AR | [Ambiguity Register](00-ambiguity-register.md) | 41 decisions; all resolved |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | R1-R14 and acceptance criteria |
| 02 | [Current State](02-current-state.md) | What exists and what is missing |
| 03-01 | [SDK Installer](03-01-installer.md) | Detection, report, fetch, integrity |
| 03-02 | [Shared Analysis](03-02-shared-analysis.md) | Reusable front-end entry point |
| 03-03 | [Language Server](03-03-language-server.md) | JSON, framing, workspace, features |
| 03-04 | [Doc Generator](03-04-doc-generator.md) | Extraction, rendering, safety |
| 03-05 | [VS Code Extension](03-05-vscode-extension.md) | Client, providers, packaging |
| 03-06 | [Rosetta Guide](03-06-rosetta-guide.md) | Content specification |
| 07 | [Testing Strategy](07-testing-strategy.md) | 57 ST cases and gates |
| 99 | [Execution Plan](99-execution-plan.md) | 8 phases, 56 tasks |
| PR | [Requirements Preflight](00-preflight-report-requirements.md) | 15 findings; 12 resolved, 2 accepted, 1 out of scope |
| PP | [Plan Preflight](00-preflight-report.md) | Findings and verdict |

## Quick Reference

### Usage Examples

```text
$ make setup
$ make setup DRY=1
$ make vscode-package
$ clorraine --doc -o build/api stdlib/std.mem.clo
$ target/debug/clorraine-lsp
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Rust dependencies | None; JSON, framing, and URIs are in-tree (AR-3) |
| LSP analysis unit | Module index plus the requested document's import closure (AR-7) |
| LSP profile | Configurable; `hosted-amiga` default (AR-8a) |
| Doc surface | `clorraine --doc -o DIR` (AR-17) |
| VS Code client | In-repo TypeScript, no runtime npm dependency (AR-23) |
| Installer fetch | Delegated to the pinned image build; media never fetched (AR-28) |
| Gates | Full set plus `demo-check` and `vscode-check` (AR-35) |

## Specialist Agents

**None** — the capability-gap check found no standing specialist role that a
catalog reviewer plus a dynamic packet would not cover.

_Detection evidence: the phases touch the compiler library orchestration
(`compiler/src/driver.rs`, `load/mod.rs`), a new protocol module, Python
tooling, and Markdown. The repository's four installed reviewers
(`correctness-reviewer`, `semantics-reviewer`, `memory-safety-auditor`,
`emission-integrity-auditor`) already cover these areas; no finding-area
recurrence exists for LSP or packaging, and a new role would restate the
correctness lens (AR-37)._

## Related Files

| Path | Change |
| ---- | ------ |
| `tools/setup/install_sdk.py`, `tools/setup/tests/` | New installer and tests |
| `compiler/src/analysis.rs` | New shared entry point |
| `compiler/src/load/mod.rs` | Library-mode option |
| `compiler/src/driver.rs` | Calls the shared entry point |
| `compiler/src/lsp/`, `compiler/src/lsp_main.rs` | New server and binary |
| `compiler/src/clorraine/cli.rs`, `docgen.rs` | `--doc` action and generator |
| `compiler/tests/lsp_spec_test.rs` | LSP integration suite |
| the inline unit tests in `compiler/src/lsp/*.rs` | LSP unit suite |
| `compiler/tests/docgen_spec_test.rs` | Doc golden suite |
| `compiler/src/clorraine/docgen.rs` tests | Doc unit suite |
| `compiler/tests/analysis_spec_test.rs` | Analysis suite |
| `compiler/tests/docs_guide_spec_test.rs` | Guide and link suite |
| `compiler/tests/fixtures/lsp/`, `fixtures/doc/` | Fixtures and golden |
| `tools/vscode/` | Extension package |
| `editors/clorraine.tmLanguage.json` | Canonical grammar (unchanged) |
| `docs/c-to-clorraine.md`, `docs/README.md` | Guide and index |
| `Makefile`, `.github/workflows/check.yml` | Gates and CI |
| `AGENTS.md`, `README.md` | Documentation |
| `plans/00-roadmap.md` | PLAN-21 |
