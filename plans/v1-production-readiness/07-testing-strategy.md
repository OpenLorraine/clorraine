# Testing Strategy: Clorraine v1.0 Production Readiness

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Compiler residual fixes | Every new diagnostic and release path covered by a spec test and an ASan probe |
| Verification scripts | Positive run plus one negative control per script |
| Specification status | Every v1.0 chapter header checked; anchors explicitly classified; `./spec/check.sh` green |
| Release artifacts | Licence and metadata audited; x86_64 binary smoke test in the conditional publish sub-plan |

- Test names state behavior: `should [expected behavior] when [condition]`.
- The compiler suites use the existing emit → strict C build → run pattern.
- ASan cases run only under `RDC_ASAN=1`; `make asan` supplies it, serialises the tests inside each
  suite, and installs a `-no-pie` `cc` wrapper for the ASan child builds so the container's PIE
  startup failure cannot abort the batch.
- The DX and PLAN-13 plans own their own ST cases; this plan gates on their completion.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These cases derive from the requirements (`01-requirements.md`), the component specs
> (`03-01` – `03-05`), the frozen specification, and the Ambiguity Register. They are the
> immutable oracle for the new behaviour.

### Residual hardening

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | Three **independent** compilations: (a) `let b: bool = null;`; (b) `let b: bool = bool(null);`; (c) `let b: bool = !null;` | (a) reports `E10012` (already correct); (b) reports `E10201` at base and must report `E10012` after the fix; (c) reports `E10218` at base and must report `E10012` after the fix. Do not combine them in one program: the first semantic error suppresses later nullability reports. `spec/04-expressions.md` records the null-cast exception. | AR-4; `spec/02-types.md:127` (TY-7); `spec/04-expressions.md:307-330` (TY-14) |
| ST-2 | Module-level nested fill: `struct Grid { rows: str[2][2]; }` and `let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];` | Builds and runs; the selected element is valid; ASan reports no leak, double free, or use-after-free. At base the unselected row's fill root leaks 11 B. | AR-4; `03-01` §Nested fill |
| ST-3 | Module-level multi-element view array: `let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];` with `mk(): H { v: String; w: String; }` and a `tag: String` field | Builds and runs; both element roots are retained and the unselected root is released once; ASan clean. At base the unselected element leaks 11 B. The sibling owning `tag` field is required to reach the store path. | AR-4; `plans/def-zero-closeout/05-findings.md:294-298` |
| ST-4 | Local initializer field call root: `let b: Box = Box { n: sink(mk().xs[0]) };` with `mk(): H { xs: String[2]; }`; plus a func-typed callee twin | Builds and runs; the whole `mk()` root is retained, the sibling `xs[1]` is released once, and the callable receives the view; ASan clean. At base each twin leaks 11 B. | AR-4; `03-01` §Initializer field call root |
| ST-5 | Module-level indexed view global: `let selected: str = mk()[pick()];` where `mk(): String[2]` and `pick(): usize` selects a run-time slot | Builds and runs; the root and index are evaluated once; the unselected slot is released once; ASan clean. At base the unselected slot leaks 11 B. The local owning form `let s: String = mk()[pick()]` is `E10026` and is not part of this case. | AR-4; `03-01` §Indexed call-root projection |
| ST-6 | Accepted controls: (a) `let s: String? = null;` with an `if (s == null)` early return; (b) `let xs: u8[2] = [1, 2]; let ys: u8[] = slice(xs, 0, 2);`; (c) simple fill `let names: str[2] = [; stringOf("x")];`; (d) clean nested literal `let rows: str[2][2] = [["aa", "bb"], ["cc", "dd"]];` | Zero diagnostics and the documented exit codes; no new diagnostic; ASan clean. These are the controls that prove the fixes did not broaden or narrow acceptance. | AR-4; `spec/02-types.md:127` |

### Verification infrastructure

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-7 | `make asan` on a clean tree | Discovers every `compiler/tests/*.rs` file that references `RDC_ASAN`, including the new `v1_production_readiness_spec_test.rs`; runs each suite in its own bounded process, with `--test-threads=1`, and with the `-no-pie` `cc` wrapper for ASan child builds; the reported count equals the number of matching files (21 at base; 22+ after Phase 1); exits 0 (the 50-test backlog suite passes 50/50) | AR-11; 03-03 |
| ST-8 | `make differential` on a clean tree | Host and hosted-amiga stdout match for `buffer-container`, `file-copier`, and `string-parser`; exits 0. This is a convenience gate over the same recorded outputs, not an independent signal. | AR-10; 03-03 |
| ST-9 | `DIFF_ONLY=1 make differential` with a deliberately changed throwaway host output | The `make` recipe forwards `DIFF_ONLY` and `OUTDIR` into the container; the diff reports the difference and exits nonzero; no tracked golden is modified. The script must not regenerate the output when `DIFF_ONLY=1`. | AR-10; 03-03 |
| ST-10 | `./spec/check.sh` and `cargo test --workspace` on the host, matching the CI workflow | Both exit 0; m68k-only tests skip with their existing note. The CI workflow is separately recorded by its run URL when an external push is authorized. | AR-9; 03-03 |

### Specification, documentation, and release

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-11 | Read every v1.0 chapter and appendix header, the anchor statuses, and run `./spec/check.sh` | Chapters `00`–`17` and appendices A/B read `stable (v1.0)`; the staged `E90xxx`/`W10005` markers remain; `grammar.ebnf.md`, `grammar-deltas.md`, `lineage.md`, `decisions.md`, and `preflight-report.md` keep their anchor/report statuses and are explicitly excluded from the chapter freeze; `spec/README.md` reflects the same table; the check is OK | AR-6; 03-02 |
| ST-12 | Read the root `README.md` status table | NLL is described as implemented; there is no "next step" claim; the production-readiness status is current | AR-6; 03-02 |
| ST-13 | Open `docs/known-limitations.md` and the links from `README.md` and `spec/README.md` | It lists exactly: the SDK image's ASan `-no-pie` build requirement (applied automatically by `make asan`; the GCC 12 runtime otherwise intermittently fails at startup for PIE executables), the top-level-`while`-only loop split, and the host-only CI limitation. There is no open-ended catch-all; a new accepted residual must be added to the list and AR-14 before release. The multi-element view-array leak is in the AR-4 fix list, not the accepted list. Both links resolve. | AR-14; 03-05 |
| ST-14 | Inspect `LICENSE`, `compiler/Cargo.toml`, `Cargo.lock`, `tools/demo/package.json`, `tools/demo/package-lock.json`, and `README.md` | The MIT text is present; the compiler package metadata says `license = "MIT"` and `version = "1.0.0"`; `Cargo.lock` records the matching version; `tools/demo/package.json` says `license = "MIT"`; the demo lockfile root records MIT; the README licence section names MIT | AR-5; 03-05 |
| ST-15 | Execute `plans/clorraine-dx/` to completion | Its progress reads 28/28; `make check`, `make verify`, and `make demo-check` are green; `spec/14-diagnostics.md` is re-reviewed after its help-cell edit | AR-1; 03-04 |
| ST-16 | Execute the PLAN-13 slice | `std.amiga.exec` exists; both allowlisted console programs build and run on hosted-amiga against an independent, hand-checked expected-output file; `spec/appendix-amiga.md` and `spec/appendix-stdlib.md` are re-reviewed against the delivered API and re-recorded `stable (v1.0)`; `make verify` is green | AR-8, AR-13; 03-04 |

> **⚠️ AUTHORING RULE:** Derive expectations from the frozen specification and the requirements
> above. Do not infer them from the implementation. If an expected output cannot be determined,
> add it to the Ambiguity Register and resolve it before defining the test.

## Specification Test Fixtures

The following programs are the exact fixtures for ST-1 – ST-6. ST-1 is three separate
compilations; each must be emitted and checked on its own.

### ST-1 — three independent null compilations

```clorraine
module spec.v1pr.nullcontrol;

func main(): i32 {
    let b: bool = null;
    return 0;
}
```

```clorraine
module spec.v1pr.nullcast;

func main(): i32 {
    let b: bool = bool(null);
    return 0;
}
```

```clorraine
module spec.v1pr.nullnot;

func main(): i32 {
    let b: bool = !null;
    return 0;
}
```

### ST-2 — nested fill at module scope

```clorraine
module spec.v1pr.nestedfill;

import { stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(len(first)) - 2;
}
```

### ST-3 — multi-element view array at module scope

```clorraine
module spec.v1pr.multielement;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { names: str[2]; tag: String; }

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(picked)) - 2;
}
```

### ST-4 — local initializer field call root

```clorraine
module spec.v1pr.fieldcall;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let b: Box = Box { n: sink(mk().xs[0]) };
    return b.n - 2;
}
```

### ST-4b — func-typed callee twin

```clorraine
module spec.v1pr.fieldcallfunc;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let f: func(str): i32 = sink;
    let b: Box = Box { n: f(mk().xs[0]) };
    return b.n - 2;
}
```

### ST-5 — indexed module view global

```clorraine
module spec.v1pr.indexedview;

import { String, stringOf } from std.text;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { return 1; }

let selected: str = mk()[pick()];

func main(): i32 {
    return i32(len(selected)) - 2;
}
```

### ST-6 — accepted controls

```clorraine
module spec.v1pr.controls;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let s: String? = null;
    if (s == null) {
        let xs: u8[2] = [1, 2];
        let ys: u8[] = slice(xs, 0, 2);
        let names: str[2] = [; stringOf("x")];
        let rows: str[2][2] = [["aa", "bb"], ["cc", "dd"]];
        return i32(xs[0]) + i32(ys[1]) + i32(len(names[0])) + i32(len(rows[0][0])) - 6;
    }
    return 1;
}
```

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/v1_production_readiness_spec_test.rs` | ST-1 – ST-6 | Residual hardening |
| `compiler/tests/dx_tooling_spec_test.rs` | Its own ST-1 – ST-19 | DX plan (created by that plan; not present yet) |
| PLAN-13 plan suites | Its own ST cases | PLAN-13 (new) |

### Implementation Tests and Script Controls

| Artifact | Description | Priority |
| -------- | ----------- | -------- |
| `compiler/tests/v1_production_readiness_impl_test.rs` | Root binding, release counts, index carry-through, nested-array recursion, and byte-identical re-emission where pinned | High |
| `docker/scripts/asan-suite.sh` | Suite discovery, per-suite timeout, serial test execution; exercised by a failing-suite negative control | Medium |
| `docker/scripts/differential-seed.sh` | Three-program host/m68k diff, `DIFF_ONLY` negative control, build/guard/clean behaviour | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | Compiler, spec, tests | Fast gate |
| `make asan` | Compiler, ASan suites | Sanitizer gate |
| `make differential` | Compiler, host C, m68k GCC, `vamos` | Convenience differential gate |
| `make verify` | All of the above plus FS-UAE and faults | Full acceptance |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Host CI | Push a branch or open a PR when authorized | The `check` workflow runs `spec/check.sh` and `cargo test --workspace`; host tests pass; m68k tests skip with the note; the run URL and conclusion are recorded. If no push is authorized, record the local command evidence and mark acceptance 9 as locally validated. |
| Release smoke (conditional publish sub-plan) | Build the Linux x86_64 binary; run `--version` and compile a hello-world program | The binary reports the exact string `clc 1.0.0` and produces a runnable host program; the exact `--version` string is asserted. aarch64 release artifacts are deferred to PLAN-07. |
| DX usage | `clorraine --explain E10034`; `--diagnostics=json` on a compile error | The DX plan's ST cases pass. |

## Test Data

### Fixtures Needed

- The exact ST-2 – ST-6 programs above, embedded in the spec suite.
- The existing corpus programs for the differential seed.
- A throwaway copy of one host output for the `DIFF_ONLY` negative control.
- The existing spec goldens and compile-fail cases.

### Mock Requirements

None. The tests use the real compiler, the real host C compiler, the real m68k toolchain, and
the real emulator. No network or external service is involved.

## Verification Checklist

- [x] All specification test cases (ST-1 – ST-16) defined with concrete input/output pairs
- [x] ST-1 pins E10012 for the explicit cast/condition spellings and keeps the annotated-let control
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written before implementation
- [x] Specification tests verified to fail before implementation, with the base leak or diagnostic
      recorded for ST-2 – ST-5
- [x] All specification tests pass after implementation
- [x] Implementation tests written for the new release paths
- [x] `make check`, `make asan`, `make differential`, and `make verify` pass
- [x] `make asan` runs each suite serially and reports the discovered count
- [x] The DX plan is 28/28, its verification is green, and `spec/14-diagnostics.md` is re-reviewed
- [x] The PLAN-13 slice is Done and both programs match an independent expected-output oracle
- [x] No regressions in the existing suites
