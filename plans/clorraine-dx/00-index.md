# Clorraine DX Tooling Implementation Plan

> **Feature**: The DX slice of PLAN-05 — `--explain` on both CLIs, `--diagnostics=json` machine-readable diagnostics, and a canonical, complete TextMate grammar under `editors/`.
> **Status**: Done (28/28)
> **Created**: 2026-10-10 19:09
> **Implements**: PLAN-05 (DX tooling slice)
> **CodeOps Artifact Schema**: 1

## Overview

The batch delivers the three developer-experience quick wins of PLAN-05, without touching the
language, the checker, or the emitted ABI. `--explain <CODE>` on both `clc` and `clorraine` prints
the normative entry for a diagnostic code — severity, section, trigger, message template, and fix
hint — parsed from the registry in `spec/14-diagnostics.md` embedded into the binary, so the
explanation can never drift from the registry. `--diagnostics=json` turns every diagnostic (and
every driver-level error) into one newline-delimited JSON object on stderr, giving editors and CI
a stable machine-readable stream while the human output and exit codes stay byte-identical. The
TextMate grammar moves to `editors/clorraine.tmLanguage.json`, gains every keyword and reserved
built-in name the frozen EBNF declares, and is enforced by a workspace test; the demo toolkit
consumes the same single file instead of owning a copy.

The batch also authors the twelve registry rows that carry no fix hint today (the staged `E90xxx`
codes and the `W100xx` warnings) — the only specification edit this plan authorizes. Everything
else is tooling: two leaf modules (a registry parser and a JSON renderer), flag plumbing in the
two CLIs, and grammar data.

`02-current-state.md` carries the `file:line` mapping per gap; `03-01` – `03-03` own the designs;
`07-testing-strategy.md` owns the 21 ST-cases; the `00-ambiguity-register.md` entries own the
decisions cited as `AR-##`.

## Minimum-Sufficient Baseline

**Original goal:** implement the DX slice — `--explain`, machine-readable diagnostics, and a
canonical TextMate grammar.
**Smallest viable design:** render both new outputs from data that already exists — the registry
file is embedded with `include_str!` and parsed, the JSON objects are rendered from the existing
`Diagnostic` values by a hand-rolled escaped writer (std-only), and the grammar is completed in
place and moved, not regenerated. No new dependency, module boundary, or harness.
**Excluded machinery:** None considered — LSP, VSCode extension packaging, the SDK installer,
watch mode, alternate output formats, and grammar generation from the EBNF were user-declined in
AR-1 and stay backlog.
**Approved complexity:** None (AR complexity check).

## Document Index

| #   | Document                                                       | Description                                 |
| --- | -------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                 | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                           | Analysis of current implementation          |
| 03-01 | [Explain Mode](03-01-explain-mode.md)                        | Registry parse, render, and flag plumbing   |
| 03-02 | [Machine-Readable Diagnostics](03-02-machine-readable-diagnostics.md) | JSON stream, schema, and error paths |
| 03-03 | [TextMate Grammar](03-03-textmate-grammar.md)                | Canonical home, completion, and consumers   |
| 05  | [Findings](05-findings.md)                                     | Plan refresh, red phases, and phase reviews |
| 07  | [Testing Strategy](07-testing-strategy.md)                     | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```text
$ clorraine --explain E10034
error E10034: cannot release or store borrowed parameter '{name}'
  trigger: Borrowed parameter released or stored by the callee
  section: Ownership, moves, initialization, nullability, borrows (E100xx)
  help: declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt

$ clc --diagnostics=json --profile host -o out.c main.clo
{"code":"E10034","severity":"error","message":"cannot release or store borrowed parameter 'x'","help":"declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt","file":"main.clo","line":10,"column":25,"endLine":10,"endColumn":26}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Explain text source | Embedded registry (`include_str!`), parsed at run time — one source of truth (AR-4) |
| Explain precedence | Early-exit mode like `--version`; unknown code exits 2 (AR-3) |
| JSON transport | NDJSON on stderr; stdout and exit codes untouched (AR-5) |
| JSON schema | Fixed key order with `null` for absent fields; end position included (AR-6) |
| Driver errors in JSON | One object with `code: null` (AR-7) |
| Grammar home | `editors/clorraine.tmLanguage.json`; demo consumes it (AR-8) |

## Specialist Agents

**None** — the batch's change surfaces (CLI flag plumbing, diagnostic rendering, registry
wording, grammar data) are covered by the four installed project reviewers
(`.opencode/agents/correctness-reviewer.md`, `emission-integrity-auditor.md`,
`memory-safety-auditor.md`, `semantics-reviewer.md`); no candidate clears the disqualifier
"would only restate AGENTS.md or an existing lens", and no ownership/IR/codegen surface is
touched, so the specialist lanes beyond correctness are not triggered.

_Detection evidence: `codeops/specialist-check.json` (refreshed 2026-10-10, 28 plans) records the
installed specialists as covering the repository's recurring finding areas; this plan's areas are
the CLI (`compiler/src/driver.rs`, `compiler/src/clorraine/`) and data files, none of which match
a new specialist lane._

## Related Files

| Path | Change |
| ---- | ------ |
| `compiler/src/diag/registry.rs` | New — registry parse and entry lookup |
| `compiler/src/diag.rs` | JSON renderer, format enum, escaping, driver-error line |
| `compiler/src/driver.rs` | `clc --explain` and `--diagnostics=json` |
| `compiler/src/clorraine/cli.rs` | Flag parse, help text |
| `compiler/src/clorraine/mod.rs`, `build.rs` | Explain action, JSON error printer, flag forwarding |
| `spec/14-diagnostics.md` | The 12 help texts (AR-10) |
| `editors/clorraine.tmLanguage.json` | New canonical grammar (moved + completed) |
| `tools/demo/grammars/clorraine.tmLanguage.json` | Removed (moved) |
| `tools/demo/src/util/shiki.ts` | Grammar path |
| `tools/demo/test/grammar.spec.test.ts` | Fixture assertions extended |
| `compiler/tests/dx_tooling_spec_test.rs` | New — specification tests |
| `compiler/tests/dx_tooling_impl_test.rs` | New — implementation tests |
