# Testing Strategy: Clorraine Amiga Graphics

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Standard-library modules (surface + contract) | 90% |
| Runtime shim functions | 80% |
| Build/verify scripts and video wiring | covered by `make verify` |

- Test names state behavior: `should [expected behavior] when [condition]`.
- E2E is the FS-UAE corpus leg (the one way to observe a real screen); it is
  feasible here because the sentinel-and-diff pattern already exists for the
  interrupt-counter.
- Adjust targets per project in `01-requirements.md` (an AR-referenced decision)
  — never silently.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from `01-requirements.md`, the
> component specs (`03-XX-*.md`), and the Ambiguity Register. They define
> expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the
> implementation. If the implementation does not match a spec test case, the
> implementation is wrong — not the test.
>
> The `Source` column lives in this plan document, not in code. When the
> executor writes the test, the in-code comment quotes the behavior in plain
> language, never the `ST-`/`Req`/`AR` id.

### Screen lifecycle and drawing

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | Compile `spec/tests/corpus/graphics/program.clo` with `--profile host` | exit 1 and first stderr line `error E11200:` naming `host` | Req R6; AR-20, AR-25 |
| ST-2 | Compile a program importing `Screen, openScreen` from `std.amiga.intuition` and `setColor, fill, draw, clear` from `std.amiga.graphics`, with `--profile hosted-amiga` | zero diagnostics, exit 0 | Req R1, R3; AR-5, AR-35 |
| ST-3 | The `stdlib` shim-symbol unit test over the graphics modules | the hosted-amiga shim defines `cl_open_screen(`, `cl_close_screen(`, `cl_set_color(`, `cl_fill(`, `cl_draw(`, `cl_clear(` | Req R5; AR-9, AR-22, AR-24 |
| ST-4 | Run the graphics corpus program under FS-UAE with `RUN_CAPTURE=1` | captured stdout equals `spec/tests/corpus/graphics/expected-output.txt` (`graphics: ok` + newline) and exit 0 | Req R6, R7; AR-19, AR-21 |
| ST-5 | Compile a program that uses a `Screen?` value before a `!= null` check, with `--profile hosted-amiga` | rejected with `E10010` | Req R1; AR-16 |
| ST-6 | The `stdlib` export-table unit tests over both new modules | `std.amiga.intuition` exports `Screen`, `openScreen`; `std.amiga.graphics` exports `setColor`, `fill`, `draw`, `clear` | Req R1, R3; AR-5, AR-22 |

> **ST-5 note (PF-005):** ST-5 guards an existing compiler rule, so it has no
> meaningful red phase; it is a green-only regression. Only ST-2 has a red phase
> (the modules are unresolved before implementation).

> **⚠️ AUTHORING RULE:** Derive expectations from the specification documents
> above. Do NOT imagine or infer what the implementation will produce. If the
> expected output cannot be determined from the spec, that is an ambiguity — add
> it to the register and resolve it with the user before defining the test case.

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as `graphics_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/graphics_spec_test.rs` | ST-1, ST-2, ST-5 | Host rejection, hosted-amiga compile, nullable use |
| existing `compiler/src/stdlib.rs` unit tests | ST-3, ST-6 | Shim symbols and export tables |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as `graphics_impl_test.rs`.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/graphics_impl_test.rs` | The generated C for the corpus program contains exactly the six shim calls in the cleanup path and no application-level `ffi`/`unsafe`; the drop calls `cl_close_screen` once | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Hosted-amiga build | Modules, shim, codegen | The corpus program emits and builds with `m68k-amigaos-gcc -Wall -Wextra -Werror` |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Graphics corpus under FS-UAE | `make corpus-amiga` then `make corpus-fs-uae` | Captured stdout matches the golden; the screen renders |

## Test Data

### Fixtures Needed

- `spec/tests/corpus/graphics/program.clo` and `expected-output.txt` (the corpus golden).
- A small unchecked-nullable program for ST-5 (inlined in the test, like the existing regression tests).

### Mock Requirements

None — the tests use the real compiler, the real toolchain, and the real emulator.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Test coverage meets goals
