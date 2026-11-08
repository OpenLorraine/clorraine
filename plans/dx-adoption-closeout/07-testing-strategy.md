# Testing Strategy: DX and Adoption Closeout

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Compiler analysis entry point | Existing suite parity plus direct API cases |
| LSP protocol and JSON | Every error path and every frame boundary |
| LSP features | One integration case per feature plus cross-file and overlay cases |
| Doc generator | Byte-exact golden plus attachment and safety cases |
| Installer | Every component state, dry run, confirmation, and integrity failure |
| VS Code client | Framing, crash, packaged handshake, manifest, grammar bytes |
| Guide | Heading, diagnostic, and link completeness |

Tests use real compiler values and real child processes. The installer uses a
fake `docker` and monkeypatched runner because a Docker daemon is not a test
oracle. No test writes outside its temporary root.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from the requirements and the component specifications. They define
> expected behavior before implementation and are the immutable oracle.

### Installer

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | Temp root with `.env`, a fake image, all tools, NDK, one ROM, one ADF | Every component row `ok`; exit 0; `ready: true` in JSON | R1.2-R1.3 |
| ST-2 | Docker CLI missing | Docker row `missing`; exit 1; hint names Docker | R1.2, R2.5 |
| ST-3 | Image missing, no `--fetch` | Image row `missing`; plan row names `make -C docker image`; exit 1 | R2.1, R2.5 |
| ST-4 | Same as ST-3 with `--fetch --yes` | Runs image build then init; re-probes; exit 0 when the fake build succeeds | R2.1, R2.3 |
| ST-5 | `--fetch` without `--yes` and no TTY | Exit 2; stderr names `--yes` and `--dry-run` | R2.3 |
| ST-6 | `--dry-run` with everything missing | Full report and plan, no state-changing command, exit 0 | R2.3 |
| ST-7 | Provenance commit differs from `versions.env` | Exit 1; message names both commits; no retry | R2.2 |
| ST-8 | Fake image build exits 7 | Exit 1; message includes the command and a recovery step | R2.5 |
| ST-9 | No media; default | ROM and Workbench rows `optional`; exit 0 | R1.4 |
| ST-10 | No media; `--require-media` | ROM and Workbench rows `missing`; exit 1 | R1.4 |
| ST-11 | Media files mode 000 | Detection still reports the counts; no file content is read | R1.3, R3.2 |
| ST-12 | `--json` | Stable keys `ready`, `image`, `media_root`, `components`, `actions`, `plan`; no media bytes | R1.3 |

### Shared analysis

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-13 | `Mode::ParseOnly` with a syntactically valid file | `loaded` is `None`; no type or ownership diagnostics | R8.2 |
| ST-14 | Full analysis of a library module without `main`, `check_entry_point=false` | No entry-point diagnostic; `typed` true | R8.3 |
| ST-15 | Same module with `check_entry_point=true` | The existing entry-point error is reported | R8.3 |
| ST-16 | Corpus emission through `clc` before and after the refactor | Byte-identical C for every corpus program | R8.4 |

### LSP lifecycle and protocol

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-17 | `initialize` with `rootUri` and `host` profile | Result has the five capabilities, `positionEncoding: "utf-16"`, `serverInfo.name: "clorraine-lsp"` | R4.3-R4.4 |
| ST-18 | `shutdown` then `exit` | `shutdown` result `null`; process exit 0 | R4.3 |
| ST-19 | `exit` without `shutdown` | Process exit 1 | R4.3 |
| ST-20 | Request before `initialize` | Error `-32002`; process stays alive | R4.5 |
| ST-21 | Malformed JSON body, then a valid request | Error `-32700` with `id: null`; the valid request then succeeds | R4.5 |
| ST-22 | Unknown request method | Error `-32601`; process stays alive | R4.3 |
| ST-23 | Unknown notification | No response; process stays alive | R4.3 |
| ST-24 | `$/cancelRequest` for an unknown id, then a request | No response to the notification; the request succeeds | R7.4 |
| ST-25 | Cancelled request sent before its handler starts | Response error `-32800` | R7.4 |
| ST-26 | `didOpen` a file with a type error | One `publishDiagnostics` with that URI, one error with a code and range | R5.1-R5.2 |
| ST-27 | `didChange` fixes the error | A second publish with an empty array | R5.2, R7.2 |
| ST-28 | `didClose` | A publish with an empty array | R5.2 |
| ST-29 | Request for a missing document | Empty diagnostics; hover/definition/symbols return `null`; no crash | R5.4, R7.3 |
| ST-30 | Workspace has two independent programs with `main` | Neither program produces cross-program duplicate or entry-point diagnostics | R7.1 |
| ST-31 | Profile `host` with an `std.amiga` import | `E11200`; profile `hosted-amiga` accepts the same file | R6.6 |
| ST-32 | `untitled:` document with an error via `didOpen` | Diagnostics publish for the `untitled:` URI; symbols work | R7.2, N2 |

### LSP features

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-33 | Hover on a documented function name | Markdown containing the exact signature and the module | R6.1 |
| ST-34 | Definition on a local `let` use | Same-file identifier range of the `let` name | R6.2 |
| ST-35 | Definition on an imported function use | Location in the other module file | R6.2 |
| ST-36 | Completion at a blank line | Contains `func`, `let`, `i32`, the current module's declarations, and imported names; sorted and unique | R6.3 |
| ST-37 | Document symbols | Top-level functions/types/constants in source order with correct LSP kinds | R6.4 |
| ST-38 | Hover/definition on `std.amiga.dos` segment | Definition resolves to the embedded stdlib buffer or returns `null`; never a crash | R6.2, R5.3 |

### Doc generator

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-39 | Two-module fixture with generics, struct, enum, handle, const, global, multi-line function | Every generated file equals the committed golden byte-for-byte | R11.3, R12.2-R12.3 |
| ST-40 | `///` run followed by a blank line | The comment is not attached | R11.2 |
| ST-41 | `///` run with no following declaration | Ignored; no output mention | R11.2 |
| ST-42 | Same input twice | Byte-identical output; no timestamps or absolute paths | R12.1 |
| ST-43 | Module name `..` or `a/b` | Exit 2; no file written | R11.5, N2 |
| ST-44 | Source with a syntax error | Exit 1; no output directory content | R11.5 |
| ST-45 | Type declared in another input module used in a signature | `Related:` link points to that module page and anchor | R11.4 |
| ST-46 | Signature with `export`, generics, and multiple lines | The signature block preserves the exact source text | R11.3 |

### VS Code extension

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-47 | Manifest and grammar | Grammar path exists; bytes equal `editors/clorraine.tmLanguage.json` | R9.2-R9.3 |
| ST-48 | `make vscode-check` with the real server | Packaged client initializes, gets one diagnostic, and gets a hover | R10.2 |
| ST-49 | `.vsix` archive audit | Contains manifest, `extension.js`, `client.js`, grammar, README; excludes `src`, `test`, `node_modules`, `*.ts` | R9.4 |
| ST-50 | Staged package version | Equals `compiler/Cargo.toml` version | R9.4 |
| ST-51 | Client against a response split across two chunks | Request resolves | R10.1 |
| ST-52 | Client against a server that exits mid-request | Pending request rejects; `onExit` fires; no unhandled rejection | R10.1 |

### Guide and integration

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-53 | `docs/c-to-clorraine.md` | Contains every required section heading and all 12 diagnostic codes | R13.2-R13.3 |
| ST-54 | Relative links from `docs/*.md` and `README.md` | Every target exists | R13.1, R14.2 |
| ST-55 | `make check` | Green | R14.4 |
| ST-56 | `make demo-check` and `make vscode-check` | Green | R14.4 |
| ST-57 | `make verify` | Prints `verify: all legs passed` | R14.4 |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `tools/setup/tests/test_detect.py`, `test_fetch.py`, `test_cli.py` | ST-1 to ST-12 | Installer |
| `compiler/tests/analysis_spec_test.rs` | ST-13 to ST-15 | Shared analysis |
| `compiler/tests/lsp_spec_test.rs` | ST-17 to ST-38 | LSP |
| `compiler/tests/docgen_spec_test.rs` | ST-39 to ST-46 | Doc generator |
| `tools/vscode/test/manifest.test.ts` | ST-47, ST-50 | Extension manifest |
| `tools/vscode/test/packaged.test.ts` | ST-48, ST-49 | Packaged client |
| `compiler/tests/docs_guide_spec_test.rs` | ST-53, ST-54 | Guide and docs |
| Existing corpus and differential gates | ST-16, ST-55 to ST-57 | Integration |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/lsp/tests.rs` | JSON escapes/surrogates/depth, split frames, header case, oversized frame, UTF-16, URI, workspace closure | High |
| `compiler/src/clorraine/docgen.rs` unit tests | Attachment edge cases, anchor collisions, path validation | High |
| `tools/vscode/test/client.test.ts` | Split response, crash, pending rejection | High |
| `tools/setup/tests/test_detect.py` | Probe helpers and report ordering | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | All | Spec check, registry, Rust suites, benchmarks, fuzzer |
| `make vscode-check` | LSP + extension | Build server, type-check, package, extract, handshake |
| `make demo-check` | Demo + grammar | Existing demo suite remains green |

## Test Data

### Fixtures Needed

| Fixture | Purpose |
| ------- | ------- |
| `compiler/tests/fixtures/lsp/workspace/` | Two modules, one independent program, one library |
| `compiler/tests/fixtures/lsp/errors/` | Type error at a known line |
| `compiler/tests/fixtures/doc/` | Two modules with doc comments and generics |
| `compiler/tests/fixtures/doc/golden/` | Expected Markdown |
| `tools/setup/tests/fixtures/` | Fake docker, provenance, media tree |
| `tools/vscode/test/fixtures/` | One-file workspace for the packaged handshake |

### Mock Requirements

- Installer: fake `docker` executable and monkeypatched `run_command`.
- VS Code: a real `clorraine-lsp` process; no mocked protocol.
- Everything else: real compiler values and files.

## Verification Checklist

- [ ] All ST cases defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement or component decision
- [ ] Specification tests written before implementation
- [ ] Red phase recorded for every phase
- [ ] All specification tests pass after implementation
- [ ] Implementation tests cover edge cases and internals
- [ ] No regressions in existing suites
- [ ] `make check`, `make demo-check`, `make vscode-check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, and `make verify` green
