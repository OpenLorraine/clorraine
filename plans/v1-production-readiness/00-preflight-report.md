# Preflight Report: Clorraine v1.0 Production Readiness

> **Status**: ✅ PREFLIGHT PASSED — PF-004 resolved; 4 new findings resolved; no open critical/major
> **Iteration**: fresh-session audit + fix application + deeper execution probe + PF-004 resolution (iteration 4; the prior report is preserved as `00-preflight-report-iterations-1-3.md`)
> **Artifact**: Implementation plan directory `plans/v1-production-readiness/` (11 plan documents)
> **Artifact manifest**: `8888356a151129ab48331d0fa0bdb45e77e5493b283994f19b446d517ef8bd74` (SHA-256 of the `sha256sum` manifest over the 11 plan files after the PF-004 fix; the prior post-fix manifest was `40552bbf5c59846cd6474dfa16fb12a1d375a6db37f22b3d0bb42e5c33f34a64`)
> **Repository revision at scan start**: `334fc2c` (`main`)
> **Audit target**: the 11 plan documents in `plans/v1-production-readiness/` (AR, index, requirements, current state, 03-01 – 03-05, testing strategy, execution plan)
> **Context documents** (read for evidence; not passed or modified): `plans/00-roadmap.md`, `plans/def-zero-closeout/05-findings.md`, `plans/clorraine-dx/99-execution-plan.md`, `spec/`, `Makefile`, `docker/Makefile`, `docker/compose.yaml`, `docker/scripts/`, `compiler/src/`, `compiler/tests/`, `Cargo.toml`, `compiler/Cargo.toml`, `Cargo.lock`, `tools/demo/package.json`, `tools/demo/package-lock.json`, `README.md`, `AGENTS.md`
> **Modification set**: the 11 plan documents, edited with the user's authorization to apply the best options. No compiler, runtime, specification, or test source was changed.
> **Last Updated**: 2026-11-01

**Same-session note:** the plan was not created in this session; it was authored earlier. Same-model bias cannot be excluded. This fresh session used direct compiler probes, a line-by-line codebase check, and one independent challenger; every finding below was re-verified against the artifact or the code.

**Scope mode:** strict. Optional additions were not reported. The audit target is the plan directory only; context documents are not passed or modified.

**Mode:** normal (no `--auto-design`). The user chose the "best possible options"; the four findings were resolved with the recommended Option A. The user then authorized the deeper probe; PF-004 was root-caused and resolved with the `-no-pie` ASan wrapper.

---

## Codebase Context Summary

**Tech stack:** a std-only Rust 2021 workspace with one member, `compiler/`; `clc` emits one C99 translation unit and `clorraine` builds/runs programs. The SDK container supplies `m68k-amigaos-gcc`, `vamos`, and FS-UAE. The demo tooling is a separate npm package under `tools/demo/`.

**Architecture:** `compiler/src/` has `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`, plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, `init_graph`, and the `clorraine` driver. The runtime shims are `compiler/runtime/shim_{host,hosted_amiga,freestanding}.c`.

**Key files examined:** `Makefile`; `spec/check.sh`; `spec/README.md`; `spec/00-introduction.md`; `spec/02-types.md`; `spec/04-expressions.md`; `spec/14-diagnostics.md`; `spec/appendix-amiga.md`; `spec/appendix-stdlib.md`; `compiler/src/driver.rs`; `compiler/src/clorraine/cli.rs`; `compiler/src/sema/expr.rs`; `compiler/src/codegen/program.rs`; `compiler/src/own/lower.rs`; `compiler/Cargo.toml`; `Cargo.lock`; `compiler/tests/*.rs`; `docker/Makefile`; `docker/compose.yaml`; `docker/scripts/corpus-amiga.sh`; `plans/def-zero-closeout/05-findings.md`; `plans/clorraine-dx/99-execution-plan.md`; `plans/00-roadmap.md`.

**Observed facts used below:**

| Fact | Evidence |
|---|---|
| `make check` = `./spec/check.sh` + `cargo test --workspace` in the SDK container | `Makefile:43-45` |
| `make verify` = `check` + `corpus-fs-uae` + `run-fault-amiga`; `corpus-fs-uae` depends on `corpus-amiga`; FS-UAE needs X and local ROMs | `Makefile:80-91`, `Makefile:19-21` |
| `make asan` and `make differential` do not exist yet | `Makefile` target list |
| 81 integration test files; 21 reference `RDC_ASAN` at base | `ls compiler/tests/*.rs`; `grep -l RDC_ASAN compiler/tests/*.rs` |
| `./spec/check.sh` is green: 27 files, 111 compile-fail tests | `./spec/check.sh` |
| The spec is a working draft (v0.4); every chapter and appendix is draft | `spec/README.md:3`; `spec/*.md` status lines |
| The DX plan is 0/28; `plans/amiga-library-surface/` does not exist | `plans/clorraine-dx/99-execution-plan.md:6`; `ls plans` |
| `std.amiga.exec` is absent; `std.amiga.{custom,dos,graphics,interrupt,intuition}` exist | `ls stdlib` |
| `spec/appendix-amiga.md:247` already imports `std.amiga.exec` | `spec/appendix-amiga.md:247` |
| `compiler/Cargo.toml` has no `license`, `description`, or `repository`; version is `0.1.0` | `compiler/Cargo.toml:2-6` |
| `tools/demo/package.json` declares `"license": "UNLICENSED"`; the lockfile root is also `UNLICENSED`, version `0.0.0` | `tools/demo/package.json:24`; `tools/demo/package-lock.json:8-11` |
| `Cargo.lock` records `clc` version `0.1.0` | `Cargo.lock:6-8` |
| The binary reports `clc 0.1.0` and `clorraine 0.1.0` | `./target/debug/clc --version`, `./target/debug/clorraine --version` |
| There is no `.github/workflows/`, no `LICENSE`, no `RELEASE.md`, no `docs/known-limitations.md`, no `CHANGELOG.md`, and no git tag | `ls`, `git tag` |
| The roadmap PLAN-15 row is `Plan Preflighted` (🔬) | `plans/00-roadmap.md:578` |
| `docker/Makefile exec` does not forward host environment variables | `docker/Makefile:69-71`; `docker/compose.yaml:5-12` |

---

## Verification Probes

These probes were run against `target/debug/clc` (built 2026-10-30) with `--profile host`; ASan probes used `cc -fsanitize=address` and `ASAN_OPTIONS=detect_leaks=1`.

| Probe | Result |
|---|---|
| `let b: bool = null;` | `error E10012: 'null' is not allowed here` |
| `bool(null)` | `error E10201: cannot cast 'null' to 'bool'` |
| `!null` | `error E10218: condition must be 'bool'` |
| ST-2 nested fill fixture | emits, strict-builds, runs; **11-byte LeakSanitizer report** at base |
| ST-3 multi-element fixture (with `tag`) | emits, strict-builds, runs; **11-byte LeakSanitizer report** at base |
| ST-3 without `tag` | emits, strict-builds, runs clean (confirms the `tag` requirement) |
| ST-4 direct field-call-root fixture | emits, strict-builds, runs; **11-byte LeakSanitizer report** at base |
| ST-4b func-typed field-call-root fixture | emits, strict-builds, runs; **11-byte LeakSanitizer report** at base |
| ST-5 indexed module-view fixture | emits, strict-builds, runs; **11-byte LeakSanitizer report** at base |
| ST-6 accepted controls | emits, strict-builds, runs exit 0; no sanitizer report |
| `let s: String = mk()[pick()];` | `error E10026: partial moves are limited to top-level fields in v1` |
| Host corpus `buffer-container`, `file-copier`, `string-parser` | each matches `expected-output.txt` |
| `freestanding` under `--profile host` | matches `expected-output.txt` |
| `grep -l RDC_ASAN compiler/tests/*.rs \| wc -l` | 21 (base) |
| `./spec/check.sh` | OK — 27 files, 111 compile-fail tests |

The host has no `cargo`; the full `make check`, `make verify`, `make asan`, and `make differential` targets were not run here. The plan's execution-time gates run them inside the SDK container. The probe results above use the already-built `clc` and the host C compiler; the container probes below use the pinned SDK image and its `cargo`/`cc`.

---

## Deeper Execution Probe (2026-11-01)

The fresh audit ran the exact ASan command pattern the plan's `make asan` target will use, in the SDK container. The failure was reproducible but nondeterministic; a deeper probe isolated it.

| Probe | Result |
|---|---|
| `cargo test --test corpus_spec_test` | 6 passed; `cargo test --test <suite>` from the workspace root works |
| `RDC_ASAN=1 cargo test --test array_value_integrity_spec_test -- --test-threads=1` | 42 passed in 12.55 s |
| `RDC_ASAN=1 cargo test --test def_backlog_closeout_spec_test -- --test-threads=1` | **failed once**: 3 test failures and one test still running at the 600 s probe timeout |
| `should_move_the_indexed_element_when_an_aggregate_field_reads_a_call_result` alone, `RDC_ASAN=1 --test-threads=1` | **failed after 120 s** on one run (`the program did not finish within 120s`); **passed in 0.35 s** on a repeat run |
| The same test without `RDC_ASAN=1` | passed in 0.29 s |
| The generated ASan child while hung | process state `R` with no syscall; instrumented emitted C printed no `TRACE` line before repeated `AddressSanitizer:DEADLYSIGNAL` lines, so the fault is before `main`, inside ASan startup |
| Trivial ASan program, 100 runs | **32 hangs with PIE**; **0 hangs with `-no-pie`** |
| Real emitted program, 200 runs | **57 hangs with PIE**; **0 hangs with `-no-pie`** |
| Backlog suite through a `cc` wrapper that adds `-no-pie` | **50/50 passed in 6.91 s** |
| Full base set through the wrapper | **21/21 suites passed**, status 0; the 50-test backlog suite passed 50/50 |

The failure is an ASan startup failure for PIE executables under the SDK container's ASLR, not a generated-program defect and not a test-harness pipe or lock issue. `--test-threads=1` does not change it. Disabling ASLR with `setarch -R` is blocked by the container's seccomp policy; `-no-pie` is the usable fix. The plan now applies it in `make asan` and in the new Phase 1 ASan helper.

---

## Prior-Finding Verification

The prior report's 40 findings (PF-001 – PF-040) were checked against the current plan text and code. PF-004 is resolved by the deeper probe and the `-no-pie` wrapper; the other prior critical/major findings remain resolved.

| Prior finding group | Verification | Status |
|---|---|---|
| PF-001 null ruling | `clc` probes: control E10012, cast E10201, `!null` E10218 | Resolved; ST-1 matches |
| PF-002/PF-003 residual cases | ST-2 – ST-5 each 11 B LSan at base; ST-6 clean; ST-3 without `tag` clean | Resolved |
| PF-004/PF-009 ASan serial | The `-no-pie` wrapper removes the PIE/ASan startup failure; backlog 50/50; full base set 21/21 | ✅ Resolved |
| PF-005/PF-006/PF-007 scope/gates | 45 + 2 tasks; AR-13 gate before 5.1.1; Phase 5 bounded | Resolved |
| PF-008/PF-010 review lenses | Phase 1 has four lenses; Phase 4 re-reviews `spec/14-diagnostics.md` | Resolved |
| PF-011/PF-037 known-limitations link | Deferred to Phase 6; the target is now named `spec/README.md` | Resolved |
| PF-012 CI hardening | `permissions: contents: read`, SHA pin, `persist-credentials: false`, timeout, concurrency | Resolved |
| PF-013/PF-014 version/arch | Version task added; x86_64 only; PF-042 corrected the expected `--version` string | Resolved |
| PF-015/PF-016 differential | Convenience-gate rationale; `DIFF_ONLY` seam; PF-043 added container forwarding | Resolved |
| PF-017/PF-018 known limitations/statuses | 03-05 sole owner; target status table | Resolved |
| PF-019/PF-020 CI/FS-UAE evidence | Tasks 3.1.7 and 7.1.6; environment preflight | Resolved |
| PF-021/PF-022 demo MIT/release notes | Task 6.1.2; conditional task C.1.2 | Resolved |
| PF-023/PF-024 references/falsifiable STs | Roadmap references checked; exact fixtures present | Resolved |
| PF-025 45-task count | Phases 4/5 are plan-gated and report their own progress | Resolved |
| PF-026 – PF-040 iteration 2/3 fixes | Spot-checked the current text; no regression | Resolved |

---

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 (resolved) | 🟠 MAJOR |
| 4 | Completeness Gaps | 0 | — |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 2 (resolved) | 🟠 MAJOR |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 (resolved) | 🟠 MAJOR |
| 12 | Consistency | 0 | — |
| 13 | Codebase Alignment | 1 (resolved) | 🟠 MAJOR |

Each finding is counted once by its primary dimension; the "also" dimensions in the finding headers are cross-references, not additional counts.

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 0 | — |
| MAJOR | 5 | 5 resolved; 0 open |
| MINOR | 0 counted | 4 non-blocking notes addressed |
| OBSERVATION | 0 | — |

---

## Findings

### PF-004: The sanitizer gate's serial mitigation missed the PIE/ASan startup failure 🟠 MAJOR — ✅ RESOLVED

**Dimension:** 7 Testability (also 6 Feasibility)
**Location:** `03-03-verification-infrastructure.md` §Current Architecture and §`docker/scripts/asan-suite.sh`; `07-testing-strategy.md` ST-7 and ST-13; `99-execution-plan.md` tasks 1.1.1, 1.1.2, 1.2.5, 1.3.2, 3.1.1, 3.1.2, and 7.1.3; `03-01-residual-hardening.md:92-94,218-221`; `03-05-release-engineering.md` known limitations; `00-ambiguity-register.md` AR-4, AR-11, and AR-14; `01-requirements.md` R3; `02-current-state.md` Gap 3 and the risk row.
**Codebase evidence:** `compiler/tests/def_backlog_closeout_spec_test.rs:153-210` is the `run_with_timeout` helper, `:219-260` is `run_asan`, and `:529` is the indexed-box test. The earlier probe, `RDC_ASAN=1 cargo test --test def_backlog_closeout_spec_test -- --test-threads=1`, failed/hung in the SDK container. The deeper probe found the child in state `R` with no syscall; instrumented emitted C printed no `TRACE` line before repeated `AddressSanitizer:DEADLYSIGNAL` lines. The fault is before `main`, inside ASan startup, not in the generated program's logic. The SDK image uses GCC 12.2. A trivial ASan program hung 32/100 with PIE and 0/100 with `-no-pie`; the real emitted program hung 57/200 with PIE and 0/200 with `-no-pie`. The test helper's `cc` build is PIE by default; `--test-threads=1` does not change the startup.
**The problem:** The plan's claim that `--test-threads=1` removes the flake was false, and the plan listed the flake as an accepted residual instead of fixing the gate. `make asan` could fail at ASan startup, so the R10 final gate was unreliable.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Fix the ASan child build: link the test children with `-no-pie` through a `cc` wrapper, and keep the serial per-suite gate | Preserves AR-11's full set and the sanitizer signal; fixes the actual startup failure | Adds a small temporary PATH wrapper to the target |
| B | Exclude the known-flaky suite from the gate | The gate can pass with a documented residual | Weakens AR-11; needs a user ruling |
| C | Add retry/quarantine machinery | Keeps the full set | New support surface; does not fix the startup failure |
| D | Run the ASan target on the host | The manual program passes on the host | The host has no `cargo` here; changes the pinned-toolchain assumption |

**Resolution:** Option A applied. `03-03` now installs a temporary `cc` wrapper first on `PATH` that appends `-no-pie` only to `-fsanitize=address` commands; the `make asan` script runs every suite through it. The new Phase 1 suite's ASan helper links with `-no-pie` directly. The plan's error handling, ST-7, ST-13, AR-4/AR-11/AR-14, R3, the current-state risk row, and the known-limitations list now record the PIE/ASan requirement and the wrapper. Evidence: the 50-test backlog suite passed 50/50 in 6.91 s under the wrapper; the full base set passed 21/21 suites with status 0; the trivial program was 0/100 and the real emitted program was 0/200 with `-no-pie`.
**User Decision:** Resolved — the user authorized the recommended Option A.
**Confidence:** High. **Hardening:** direct container probes; no additional challenger is needed because the fix is measured end to end and does not disable LSan or exclude a suite.

---

### PF-041: The sanitizer gate's acceptance count is stale after Phase 1 adds the new ASan suite 🟠 MAJOR — ✅ RESOLVED

**Dimension:** 3 Logical Contradictions (also 7 Testability, 12 Consistency)
**Location:** `07-testing-strategy.md:44` (ST-7); `99-execution-plan.md:149` (task 3.1.2); related snapshots `00-index.md:102`, `00-ambiguity-register.md:18,90`, `03-03-verification-infrastructure.md:24`
**Codebase evidence:** `03-03-verification-infrastructure.md:68` discovers every `compiler/tests/*.rs` file containing `RDC_ASAN`; the base count is 21. Phase 1 task 1.1.1 (`99-execution-plan.md:73`) creates `compiler/tests/v1_production_readiness_spec_test.rs`. `03-01-residual-hardening.md:92` says the new spec suite uses the existing test helpers for `RDC_ASAN` runs, and `:216-217` says every new memory case runs with `RDC_ASAN=1`. Phase 3 runs after Phase 1, so the new suite is discovered and the count is at least 22 (23 if the new implementation suite also references `RDC_ASAN`).
**The problem:** ST-7 expected "21" and task 3.1.2 said "confirm the discovered count is 21". After Phase 1, the target correctly discovers 22 or more. The task would either fail or tempt the executor to exclude the new suite to hit 21, which violates AR-11 ("every test file that references `RDC_ASAN`") and removes the new sanitizer cases from the gate.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Make the count dynamic: ST-7 and task 3.1.2 assert the discovery rule and require the new suite; report the count without a hardcoded 21; update the snapshots to "21 at base, 22+ after Phase 1" | Robust; matches AR-11; no future drift | Slightly weaker exact-count assertion |
| B | Change the expected count to 22 (or "22+") and keep the hard assertion | Minimal text change; exact | Fragile if the implementation suite also uses `RDC_ASAN` or future suites are added |
| C | Exclude the new suite from `make asan` to keep 21 | Keeps the number | Violates AR-11; misses the new ASan cases; not viable |

**Recommendation:** Option A — the discovery rule is the real oracle; the count is informational. Option B is acceptable only as "22+" with the rule spelled out.

**Resolution:** Option A applied. `07:44` now requires the count to equal the number of matching files and names the new suite; `99:149` now says "confirm the reported count equals the number of files matching `RDC_ASAN` and includes the new production-readiness suite (21 at base; 22+ after Phase 1)"; `00-index.md:102`, `00-ambiguity-register.md:18,90`, and `03-03:24` now use the base-to-current wording.
**User Decision:** Resolved — user chose the best options; Option A applied.
**Confidence:** High. **Hardening:** independent challenger confirmed the contradiction; no divergence.

---

### PF-042: The release smoke test expects `v1.0.0`, but the binary prints `clc 1.0.0` 🟠 MAJOR — ✅ RESOLVED

**Dimension:** 13 Codebase Alignment (also 7 Testability)
**Location:** `01-requirements.md:65-67`; `03-05-release-engineering.md:120,139-140`; `07-testing-strategy.md:242`; `99-execution-plan.md:261,268`
**Codebase evidence:** `compiler/src/driver.rs:253-254` prints `clc {CARGO_PKG_VERSION}`. `compiler/Cargo.toml:5` is currently `version = "0.1.0"`; `./target/debug/clc --version` prints `clc 0.1.0`. After task 6.1.2 sets `version = "1.0.0"`, the output is `clc 1.0.0`, not `v1.0.0`. The `clorraine` driver uses the same no-`v` format (`compiler/src/clorraine/cli.rs:94-96`).
**The problem:** The plan said the binary must report `v1.0.0` and that "the exact `--version` string is asserted". A literal `v1.0.0` assertion fails against `clc 1.0.0`; the error-table remedy ("set the Cargo version before building") cannot add the missing `v`. The conditional publish smoke test would be red even after the correct version bump.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Assert `clc 1.0.0` exactly; keep the tag `v1.0.0`; optionally also assert `clorraine 1.0.0` | Matches the CLI; no code change; precise | Updates five plan lines |
| B | Change `driver.rs`/`cli.rs` to print `v{version}` | Matches the plan's literal | User-facing CLI change; needs authorization and tests; not needed |
| C | Assert a `1.0.0` substring | Simple | Contradicts "exact string"; weaker |

**Recommendation:** Option A — the release tag is `v1.0.0`; the binary output is `clc 1.0.0`. Update all five plan locations and the error-table remedy.

**Resolution:** Option A applied. `01:65-67`, `03-05:119,137-138`, `07:242`, `99:261,268` now assert the exact output `clc 1.0.0`; the release tag remains `v1.0.0`.
**User Decision:** Resolved — user chose the best options; Option A applied.
**Confidence:** High. **Hardening:** independent challenger confirmed; no divergence.

---

### PF-043: `DIFF_ONLY=1 make differential` does not pass `DIFF_ONLY` into the SDK container, so the negative control can false-pass 🟠 MAJOR — ✅ RESOLVED

**Dimension:** 7 Testability (also 4 Completeness, 13 Codebase Alignment)
**Location:** `03-03-verification-infrastructure.md:200` (the pre-fix code example); `03-03:105-118` (the script branches on `DIFF_ONLY`); `99-execution-plan.md:150-151` (tasks 3.1.3 and 3.1.4); `07-testing-strategy.md:46` (ST-9)
**Codebase evidence:** `docker/Makefile:69-71` runs `$(COMPOSE_RUN) -T dev bash -lc '$(CMD)'` with no `-e`; `docker/compose.yaml:5-12` defines `x-common`/`dev` without an `environment` entry for `DIFF_ONLY`. Docker Compose does not forward arbitrary host environment variables. Unless the Makefile recipe interpolates `DIFF_ONLY=$(DIFF_ONLY)` into `CMD`, the script sees `DIFF_ONLY` unset, takes the normal branch, regenerates both `out.txt` files, and overwrites the corrupted output; the diff passes and ST-9's required failure never happens.
**The problem:** The negative control is a required test (ST-9; 03-03 Testing). A false pass defeats the test-integrity rule ("an assertion must fail without the fix"). The script itself is correct; the container boundary is the gap.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Specify the Makefile recipe to forward `DIFF_ONLY` (and `OUTDIR`) into the container, for example `CMD="DIFF_ONLY=$(DIFF_ONLY) OUTDIR=$(OUTDIR) docker/scripts/differential-seed.sh"`; keep the documented command | Smallest; keeps `make differential` as the test entry | One recipe detail to maintain |
| B | Run the negative control directly in the container (`make -C docker exec CMD="DIFF_ONLY=1 docker/scripts/differential-seed.sh"`) and change the code example/ST-9 to that command | No Makefile forwarding | ST-9 no longer exercises the target's environment handling |
| C | Add `-e DIFF_ONLY` to the compose/exec path | General environment passing | Changes the shared exec surface; more than needed |

**Recommendation:** Option A — the plan's own code example is the intended UX; forward the two variables explicitly.

**Resolution:** Option A applied. `03-03` now specifies the `asan` and `differential` Makefile recipes, with the `differential` recipe forwarding `DIFF_ONLY=$(DIFF_ONLY)` and `OUTDIR=$(OUTDIR)` into the container; the script's `CLC` guard is skipped in `DIFF_ONLY=1` mode; `99:150-151` and `07:46` now require and test the forwarding.
**User Decision:** Resolved — user chose the best options; Option A applied.
**Confidence:** High. **Hardening:** independent challenger identified; verified against `docker/Makefile` and `docker/compose.yaml`.

---

### PF-044: Phase 2 marks `spec/appendix-amiga.md` stable while its `std.amiga.exec` example is delivered only in Phase 5, and no Phase 5 task re-reviews the appendix 🟠 MAJOR — ✅ RESOLVED

**Dimension:** 11 Ordering & Sequencing (also 4 Completeness, 12 Consistency)
**Location:** `99-execution-plan.md:120` (task 2.1.5 marks appendices A/B stable); `03-02-spec-stabilization.md:63` (appendices `stable (v1.0)`); `99-execution-plan.md:208-211` (Phase 5 delivers `std.amiga.exec`); `99-execution-plan.md:288` (the final gate); `03-04-dx-and-acceptance.md:64` (the new plan owns the wrapper surface)
**Codebase evidence:** `spec/appendix-amiga.md:247` imports `{ MsgPort, signals, waitSignals, getMsg, replyMsg } from std.amiga.exec`. `stdlib/` has no `std.amiga.exec.clo`, and `spec/appendix-stdlib.md` has no `std.amiga.exec` section. The module is absent in Phase 2 and delivered in Phase 5. The Phase 5 tasks 5.1.1 – 5.1.4 did not update or re-review the appendix. The final gate checked that the spec is stable, not that the appendix matches the delivered module. The Phase 2 review can either remove the example as "stale text" (losing the spec-level API example) or leave it (a stable appendix documenting a module that does not exist yet and whose API the new plan may change).
**The problem:** The stable spec and the delivered module can diverge. The appendix's example is the only spec-level definition of `std.amiga.exec`; the new PLAN-13 plan owns the wrapper API but was not required to match the appendix. No task re-reviewed the appendix after Phase 5, so a mismatch could survive the final gate.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Extend Phase 5 (task 5.1.3 or 5.1.4) to update/re-review `spec/appendix-amiga.md` and `spec/appendix-stdlib.md` after `std.amiga.exec` lands; rerun `./spec/check.sh`; re-record `stable (v1.0)` | Closes the ordering gap; keeps the 45-task count by folding it into an existing task | One review addition |
| B | Mark the appendix example staged in Phase 2 and restore/re-review it in Phase 5 | Honest staged status | More edits; still needs the Phase 5 re-review |
| C | Remove the example from the appendix and document the module only in the PLAN-13 plan | No dangling import | Loses the spec-level example; the appendix no longer defines the API |

**Recommendation:** Option A — extend the existing Phase 5 review (5.1.4) or the verification task (5.1.3) to include the two appendices; keep the 45-task count by folding it into an existing task, and add the appendix check to 7.1.6.

**Resolution:** Option A applied. `03-02:63` now notes the Phase 5 re-review; `03-04` adds the re-review to Proposed Changes and Integration Points; `99:211` (task 5.1.3) now re-reviews both appendices, reruns `./spec/check.sh`, and re-records their `stable (v1.0)` status; `99:288` (task 7.1.6) verifies the Phase 5 appendix re-review; `07:58` (ST-16) requires it. The 45-task count is unchanged.
**User Decision:** Resolved — user chose the best options; Option A applied.
**Confidence:** High for the ordering gap; Medium for the API mismatch (it depends on the future PLAN-13 plan).
**Hardening:** independent challenger identified; verified the appendix line and the Phase 2/5 task lists.

---

## Non-Blocking Notes (addressed, not counted)

These were status, metadata, or ambiguity issues. They are recorded here because they were corrected in the same fix application; they were not part of the critical/major count.

| Note | Pre-fix evidence | Resolution |
|---|---|---|
| N-1 Roadmap status text was stale | `99-execution-plan.md:206,329` said "the roadmap stays at `Plan Created`"; `plans/00-roadmap.md:578` is `Plan Preflighted` (🔬) | Both lines now say the roadmap moves to `⛔ Blocked` with the AR-13 reason; AR-13 now says Phase 5 remains Blocked until the names are recorded |
| N-2 "the spec's limitations section" was not named | `03-02:37-38`, `03-05:76`, `99:239`; `spec/README.md` has no limitations section; `spec/00-introduction.md:94` section 3 "What Clorraine deliberately is not" is the closest | The plan now names `spec/README.md` and its "Known limitations" section; ST-13 and task 7.1.6 check both links |
| N-3 Lockfiles were outside the manifest audit | `Cargo.lock` tracked `clc 0.1.0`; `tools/demo/package-lock.json` root `0.0.0`, `UNLICENSED`; tasks 6.1.2/6.1.5 and ST-14 named only the two manifests | Task 6.1.2, task 6.1.4, task 7.1.6, ST-14, and 03-05 now include `Cargo.lock` and `tools/demo/package-lock.json` in the metadata update and audit |
| N-4 `DIFF_ONLY` guard checked `CLC` before the diff-only branch | `03-03:109-112` checked `CLC` before the `DIFF_ONLY` branch; the negative control does not need the compiler | The script guard is now `if [ "${DIFF_ONLY:-0}" != "1" ] && [ ! -x "${CLC}" ]; then` |

---

## Fix Application (2026-11-01)

| Finding | Change made | Verification |
|---|---|---|
| PF-004 | `03-03` adds the `-no-pie` `cc` wrapper; `03-01`, `07`, `99`, `03-05`, the Ambiguity Register, R3, and the current-state risk row record the PIE/ASan requirement | Backlog 50/50; full base set 21/21; real program 0/200 with `-no-pie` |
| PF-041 | ST-7, task 3.1.2, the index, AR-11, and 03-03 now use "21 at base; 22+ after Phase 1" and require the discovery rule, including the new suite | No stale `21 today` / `count is 21` text remains; the script still discovers dynamically |
| PF-042 | Five plan locations now assert the exact output `clc 1.0.0`; the release tag remains `v1.0.0` | `compiler/src/driver.rs:254` prints `clc {version}`; after `version = "1.0.0"` the output matches |
| PF-043 | 03-03 now specifies the `asan` and `differential` Makefile recipes; `DIFF_ONLY`/`OUTDIR` are forwarded; the `CLC` guard is skipped in diff-only mode; ST-9 and tasks 3.1.3/3.1.4 test the forwarding | The recipe matches the `docker/Makefile exec` boundary; the negative control can now reach the script |
| PF-044 | 03-02, 03-04, 07 (ST-16), and 99 (5.1.3 and 7.1.6) now re-review the Amiga and stdlib appendices after `std.amiga.exec` lands | The appendix re-review is folded into task 5.1.3; the 45-task count is unchanged |
| N-1 – N-4 | The roadmap status text, the `spec/README.md` limits section, the lockfile audit, and the `CLC` guard are corrected | See the notes table above |

**Changed plan files:** `00-ambiguity-register.md`, `00-index.md`, `01-requirements.md`, `02-current-state.md`, `03-01-residual-hardening.md`, `03-03-verification-infrastructure.md`, `03-05-release-engineering.md`, `07-testing-strategy.md`, `99-execution-plan.md`. `03-02-spec-stabilization.md` and `03-04-dx-and-acceptance.md` are unchanged.

**Post-fix checks:**

| Check | Result |
|---|---|
| `./spec/check.sh` | OK — 27 files, 111 compile-fail tests |
| Main-plan task count | 45 (Phase 1 10, Phase 2 9, Phase 3 7, Phase 4 3, Phase 5 4, Phase 6 5, Phase 7 7) |
| Conditional publish task count | 2 |
| Duplicate task IDs | None |
| Stale phrases (`21 today`, `count is 21`, `Plan Created`, `spec's limitations`, `--version` `v1.0.0`) | None |
| Relative links in the plan documents | All resolve |
| Backlog ASan suite through the `-no-pie` wrapper | **PASSED** — 50/50 in 6.91 s |
| Full base ASan set through the `-no-pie` wrapper | **PASSED** — 21/21 suites, status 0 |
| New manifest | `8888356a151129ab48331d0fa0bdb45e77e5493b283994f19b446d517ef8bd74` |

---

## Pass/Fail Determination

| Criterion | Result |
|---|---|
| All 13 dimensions scanned | Yes |
| Codebase grounding | Yes — compiler probes, test counts, Makefile/Docker checks, spec and release-file checks |
| All critical/major resolved | **Yes** — 4 new major findings resolved; PF-004 resolved; 0 critical, 0 major pending |
| All minor accepted | No minors counted; 4 non-blocking notes addressed |
| Prior 40 findings | Verified resolved; PF-004 resolved by the `-no-pie` wrapper |
| Optional additions | None (strict scope) |

**Verdict:** `✅ PREFLIGHT PASSED — PF-004 resolved; 4 new findings resolved; no open critical/major.`

The plan's architecture, scope, phase order, and the four new fixes are sound. The deeper execution probe root-caused PF-004 to a PIE/ASan startup failure; the plan now applies a `-no-pie` `cc` wrapper in `make asan` and in the new Phase 1 ASan helper. The backlog suite passes 50/50 and the full base set passes 21/21. No compiler, runtime, specification, or test source was changed.

**Residual plan state (not a finding):** AR-13 remains an explicit user gate before Phase 5; if the two programme names and licence audit are not recorded, Phase 5 is Blocked and the plan is not Done. The conditional publish sub-plan remains outside the 45-task count; publication can stay blocked without preventing the main plan from closing production-ready.

---

## Next Steps

Roadmap: 546/546 defect rows done; 0 open. Remaining: **PLAN-15 — v1.0 production readiness** (`Plan Preflighted`, 0/45 tasks + 0/2 conditional publish tasks).

| Action | Owner |
|---|---|
| Review the four applied corrections, the PF-004 `-no-pie` fix, and the post-fix report | User |
| Record the AR-13 programme names and licence audit before Phase 5 | User |
| Start PLAN-15 Phase 1 through `exec-plan` | Executor |
| Refresh the roadmap note after execution starts | Executor / roadmap skill |

The four text fixes and the PF-004 wrapper are applied; the plan is **ready for execution**. Phase 1 starts with the three independent ST-1 compilations and the four 11-byte leak fixtures, then the null rule and initializer retention fixes. The DX and PLAN-13 phases remain plan-gated; Phase 5 stays Blocked until AR-13 is recorded.
