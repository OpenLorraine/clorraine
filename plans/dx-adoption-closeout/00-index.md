# DX and Adoption Closeout (PLAN-21)

> **Feature**: Install the SDK from one entry point, edit Clorraine with a
> language server and VS Code extension, generate Markdown reference from
> `///` comments, and translate common Amiga C patterns with a Rosetta guide.
> **Status**: Requirements preflighted — plan in preparation
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

This plan delivers five independent slices:

- a fetch/detect SDK installer that never redistributes licensed media;
- a std-only LSP and a packaged VS Code extension;
- a deterministic `///`-comment Markdown generator;
- a C-to-Clorraine Rosetta guide and wrapper cookbook;
- the tests, documentation, and gates that keep them honest.

## Document Index

| # | Document | Description |
| - | -------- | ----------- |
| AR | [Ambiguity Register](00-ambiguity-register.md) | Decisions and delegated technical mechanisms |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | R1-R14 and acceptance criteria |
| PR | [Requirements Preflight](00-preflight-report-requirements.md) | 15 findings; 12 resolved, 2 accepted, 1 out of scope |

## Related Files

| Path | Change |
| ---- | ------ |
| `tools/setup/install_sdk.py` | New installer |
| `tools/setup/tests/` | New installer unit tests |
| `compiler/src/analysis.rs` | New shared front-end entry point |
| `compiler/src/lsp/` | New std-only JSON-RPC and LSP server |
| `compiler/src/clorraine/docgen.rs` | New `--doc` generator |
| `compiler/src/clorraine/cli.rs` | `--doc` action |
| `compiler/tests/lsp_spec_test.rs`, `lsp_impl_test.rs` | LSP suites |
| `compiler/tests/docgen_spec_test.rs` | Doc generator suites |
| `tools/vscode/` | Extension package |
| `editors/clorraine.tmLanguage.json` | Canonical grammar (unchanged) |
| `docs/c-to-clorraine.md`, `docs/README.md` | Rosetta guide and index |
| `Makefile`, `.github/workflows/check.yml` | Commands and CI |
| `AGENTS.md`, `README.md` | Documentation |
| `plans/00-roadmap.md` | PLAN-21 |
