# Preflight Report: Critical/Major DEF Closeout

> **Status**: ❌ BLOCKED — iteration 2 resolved 19 findings; one reserved specification decision (R-6) remains
> **Iteration**: 2 (re-scan after fixes; fresh session)
> **Artifact**: Implementation plan at `plans/def-critical-major-closeout/` (12 documents; 13 with this report)
> **Artifact content hash (iteration 1 scan start)**: `7647b8253a413c8308eb7e66c7d48e88d8c5f2279f8001a1d2629da210633f37`
> **Artifact content hash (iteration 2 post-fix)**: `2980654bbb04f2cca27bbf804d4b64d38c6a97c6fd468c80951f20c718fa3b44`
> **Repository HEAD**: `ef6fd9524b07a6bec3ec089818e821c5c24f0f90`
> **Codebase Grounded**: 42 source/test/spec files examined; 30 compiler probes in iteration 1; the iteration 2 probes re-ran the DEF-167, DEF-411, DEF-415, DEF-456, DEF-460, DEF-464, and DEF-479 shapes
> **Last Updated**: 2026-10-24
> **Mode**: `--auto-design` active; iteration 1 preflight-only; iteration 2 applied the delegated plan corrections and committed them. No compiler, spec, or golden file changed.

## Codebase Context Summary

**Tech Stack:** Rust 2021 workspace, std-only (no crates, no build scripts); emits one C99 translation unit for the `host`, `hosted-amiga`, and `freestanding` profiles. Frozen specification in `spec/`; tests in `compiler/tests/`; Docker-pinned toolchain (`make check`, `make verify`).

**Architecture:** `compiler/src/` stages: `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `types`, `own`, `ir`, `codegen`, `consts`, plus the `clorraine` driver. The ownership walk (`own/flow.rs`, `place.rs`, `expr.rs`, `statements.rs`, `instance_check.rs`, `call_check.rs`) lowers bodies into typed IR and records move/view/drop duties. Codegen (`codegen/function.rs`, `guards.rs`, `program.rs`, `expr.rs`, `calls.rs`) emits the C99 unit. The type table is program-wide and keyed by simple names (`types/table.rs`); C symbol keys come from `types/keys.rs` and `codegen/types.rs`.

**Key Files Examined:**

- Plan: all 12 documents in `plans/def-critical-major-closeout/`; `plans/00-roadmap.md` at HEAD and in the working tree.
- Ownership: `compiler/src/own/flow.rs`, `place.rs`, `expr.rs`, `statements.rs`, `call_check.rs`, `instance_check.rs`, `loops.rs`, `lower.rs`, `own.rs`.
- Codegen: `compiler/src/codegen/function.rs`, `guards.rs`, `program.rs`, `expr.rs`, `calls.rs`, `types.rs`.
- Types/sema: `compiler/src/types/mod.rs`, `types/table.rs`, `types/keys.rs`, `sema/call.rs`, `sema/ops.rs`, `sema/expr.rs`, `sema/items.rs`, `consts/mod.rs`.
- Tests/spec: `compiler/tests/stage_spec_test.rs`, `compile_fail_spec_test.rs`, `borrow_store_integrity_spec_test.rs`, `borrow_generic_integrity_spec_test.rs`, `checker_conformance_spec_test.rs`, `store_emission_integrity_impl_test.rs`, `v1_language_impl_test.rs`, `spec/13-data-inclusion.md`, `spec/14-diagnostics.md`, `spec/16-borrow-rules.md`, `spec/17-codegen.md`, `spec/decisions.md`, `Makefile`, `AGENTS.md`.
- Probes: `target/debug/clc --profile host -o out.c file.clo` on the exact DEF-470, 543, 473, 472, 415, 411, 505, 167, 171, 458, 506, 507 shapes and their concrete twins; strict-C and ASAN/LSan builds where applicable.

**Reference Verification:** 61 references mapped — 57 verified, 4 unverifiable or contradicted: the `y439nestedarg` fixture (ST-5), the DEF-507 filed red shape (not reproduced for the documented shapes), the plan's "33 active" count (contradicted by DEF-458), and the "constant-context `embed` control builds" claim (contradicted by E10224 and the staged-out data inclusion).

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|---|---|---|
| 1 | Ambiguities | 2 | 🟡 MINOR |
| 2 | Implicit Assumptions | 2 | 🔴 CRITICAL |
| 3 | Logical Contradictions | 4 | 🔴 CRITICAL |
| 4 | Completeness Gaps | 5 | 🔴 CRITICAL |
| 5 | Dependency Issues | 1 | 🟠 MAJOR |
| 6 | Feasibility Concerns | 1 | 🟠 MAJOR |
| 7 | Testability | 3 | 🟠 MAJOR |
| 8 | Security Blind Spots | 3 | 🔴 CRITICAL |
| 9 | Edge Cases | 1 | 🟡 MINOR |
| 10 | Scope Creep Indicators | 1 | 🟠 MAJOR |
| 11 | Ordering & Sequencing | 1 | 🟡 MINOR |
| 12 | Consistency | 2 | 🟠 MAJOR |
| 13 | Codebase Alignment | 2 | 🟠 MAJOR |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| 🔴 CRITICAL | 6 | Iteration-1 counts; 5 verified, 1 reopened by iteration 2 (PF-003) |
| 🟠 MAJOR | 16 | Iteration-1 counts; 10 reopened/verified, 4 new roots (14 in iteration 2) |
| 🟡 MINOR | 6 | Iteration-1 counts; 4 reopened/verified, 1 new root (5 in iteration 2) |
| 🔵 OBSERVATION | 0 | — |

> This table is the iteration-1 summary. The current 20-finding status is in the **Iteration 2** section.

## Critical Findings

### PF-001: The plan's active set omits the open major DEF-458 [🔴 CRITICAL]

**Dimension:** Completeness Gaps
**Location:** `00-index.md:6,11-15`; `01-requirements.md:10-11,97-98`; `00-ambiguity-register.md:8,13,28`; `02-current-state.md:51-65`; `07-testing-strategy.md:39-50`; `99-execution-plan.md:11-14,26,47-53,100-134,295-297`
**Codebase Evidence:** `plans/00-roadmap.md:476` still shows DEF-458 as `Backlog` / `🟠 major` / open. The exact reproducer `v = (g || eat(take v) == 2) ? x : y` with `g = true` compiles at HEAD (exit 0) and LSan reports an 11-byte leak at `cl_string_new`; the plan's Fix is in the same take-capture area (`codegen/function.rs`).

**The Problem:** HEAD has **35 open critical/major rows** (1 critical + 34 major), not 34: DEF-458 is an open major and appears in no batch, ST row, or task. After closing DEF-470 as stale, the active set is **34** (1 critical + 33 major), not 33. Every derived count is short by one: 34 ST rows instead of 35, 73 tasks instead of 74, Batch B ten DEFs instead of eleven, 16 Phase 2 tasks instead of 17, and acceptance criteria 1/5 and success criteria 1/2 cannot be met because DEF-458 remains open. The Batch B label `DEF-454…466` visually includes 458, but every explicit list omits it.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add DEF-458 to Batch B (fix, ST, ASAN, one task; update all counts) | Meets the stated goal; same capture area as DEF-455/456/457; local fix | Adds one DEF, one ST row, one task; count updates across seven docs |
| B | Defer DEF-458 with an explicit user ruling and change the goal/acceptance criteria to the named rows | Keeps the 33-row scope; honest if the user approves | Leaves an open major and an 11-byte leak; contradicts "no open critical/major" |
| C | Argue DEF-456's runtime capture closes DEF-458 too | No new row | The roadmap records a distinct `\|\|`-skip/place-move path; DEF-456's range test is not the condition capture; no evidence |

**Recommendation:** Option A. It is the smallest change that preserves the original goal and the fix sits in the same `codegen/function.rs` take-capture area. This is a **reserved scope decision**: the user must approve adding a 34th active DEF or explicitly defer it (R-1).

**User Decision:** Resolved — User accepted recommendation (R-1): add `DEF-458` to Batch B. Applied 2026-10-24 (AR-18).

### PF-002: DEF-171's positive control cannot build and the E10213 golden stays excluded [🔴 CRITICAL]

**Dimension:** Logical Contradictions
**Location:** `07-testing-strategy.md:87` (ST-33); `03-E-strict-build.md:26,46`; `99-execution-plan.md:222-230`; `01-requirements.md:31-32`; `00-ambiguity-register.md:22` (AR-15)
**Codebase Evidence:** At HEAD `const LEVEL: u8[16] = embed("assets/logo.bin")` reports `E10224: expression is not constant`; a local `let data: u8[16] = embed(...)` is accepted and emits `memcpy(&(data), &(0), sizeof(data))`, which fails strict C with `lvalue required as unary '&' operand`. `compiler/tests/compile_fail_spec_test.rs:47-51` explicitly excludes `E10213`/`E10214` because "data inclusion is not part of the implemented language subset". `compiler/src/sema/expr.rs:1294-1360` has no `embed` arm; `compiler/src/consts/mod.rs:515-563` folds only `sizeof`/`offsetof`/`len`.

**The Problem:** ST-33 requires both "one `E10213`; no C" and "the constant-context control builds". The constant control cannot build without implementing the staged data-inclusion feature (file read, E10214, determinism), which is explicitly out of the 33-row scope. The plan also says the "E10213 golden is enforced", but the golden stays skipped until `compiler/tests/compile_fail_spec_test.rs` moves `E10213` from `EXCLUDED_CODES` to `ASSERTED_CODES`; no task names that harness change. The actual `embed` entry point is the intrinsic check in `sema/expr.rs`, not only `consts/mod.rs`.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Drop the positive `embed` control; assert one `E10213` + no C; re-enable the E10213 golden (keep E10214 excluded); record const-context `embed` as the staged E10224 | Enforces DI-4; no feature work; testable now | Leaves the positive `embed` path staged; E10224 remains the const behavior |
| B | Implement constant-context data inclusion | Makes the control build | New feature, out of scope; needs a separate plan/approval |
| C | Keep the golden excluded and rely only on ST-33 | No harness change | The frozen golden is not asserted; "golden is the oracle" is false |

**Recommendation:** Option A, delegated under `--auto-design` (testing strategy within AR-15). The harness edit is a test-harness change, not a spec/golden change; the golden text stays untouched. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-003: DEF-167's fix breaks the frozen stage-spec acceptance test [🔴 CRITICAL]

**Dimension:** Logical Contradictions
**Location:** `03-E-strict-build.md:44-49`; `99-execution-plan.md:227-234`; `07-testing-strategy.md:86` (ST-32); `00-ambiguity-register.md:21` (AR-14)
**Codebase Evidence:** `compiler/tests/stage_spec_test.rs:24-47` includes `E10300` in `UNACCEPTABLE_CODES`; `:533-555` runs `let call_shaped = a < b > (c);` with `a: i32` through `assert_front_end_accepts`, which requires exit 0 and no unacceptable code. The full compiler is invoked. The actual `E10300` owners are `compiler/src/sema/call.rs:257,299,462,861`, not `types/mod.rs`.

**The Problem:** The plan says the stage parse stays accepted and "existing stage/compile-fail suites stay green", while the fix reports one `E10300` on the same `a < b > (c)` shape. Both cannot hold. The stage test uses a **local i32 `a`**, not a non-generic function; the plan's ST-32 "non-generic `a`" does not distinguish the two, and a broad rule either breaks the frozen stage test or leaves the filed function form (`cl_a__`) accepted. Changing a frozen specification test needs AR-11 approval; the plan names no resolution.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add a parse-only/stage-select mode to `clc` and make the stage test use it for the `call_shaped` snippet; keep the full-compile E10300 in `def_strict_build_integrity_spec_test.rs` | Preserves the parse oracle and the rejection | Small driver surface addition; must be authorized |
| B | Move the `call_shaped` snippet to a compile-fail E10300 golden and amend the stage test, with AR-11 approval | No driver change; clear oracle split | Edits a frozen stage test; needs explicit approval |
| C | Scope the fix to named non-generic functions only; leave the local-value form accepted | Stage test stays green | Does not close DEF-167's filed source; leaves an accepted-invalid program |

**Recommendation:** Option A if a small driver addition is acceptable; otherwise Option B with an AR-11-approved amendment recorded in the register. This is a **reserved decision** (spec-test oracle): the user must approve the stage-test change or the driver surface (R-2).

**User Decision:** Resolved — User accepted recommendation (R-2): add the parse-only/stage-select mode. Applied 2026-10-24 (AR-19).

### PF-004: Batch A's `source_union` is not flow-sensitive and can both miss and invent escapes [🔴 CRITICAL]

**Dimension:** Implicit Assumptions
**Location:** `03-A-critical-flow.md:27-45,56-68`; `99-execution-plan.md:79-87`
**Codebase Evidence:** `compiler/src/own/flow.rs:32-82` puts `provenance`, `copy_source`, and `rebound` on `Binding`; `own/statements.rs:973-995` sets `rebound`/`copy_source` on assignment; `own/place.rs:2594-2608` returns `Local(id)` for a rebound local. The flow-sensitive joined state is `FlowState` (`own/flow.rs:190-226`), cloned/merged at branches and loops. `own/place.rs:870-883` produces `Provenance::Composite` for conditionals. Probe: `let s: str = p; s = "AA"; slot = s;` is accepted at HEAD (safe); the plan's may-union would retain `Param(p)` and reject it.

**The Problem:** The plan seeds `source_union` at declaration and **unions** every assignment, with no strong update, branch join, or speculation rollback. Three concrete failures:
1. A definite overwrite of a parameter view (`let s = p; s = "AA"; slot = s;`) leaves `Param(p)` in the union and falsely rejects a safe store (R4/R6 regression).
2. A conditional assignment (`s = c ? p : "AA"`) stores a `Provenance::Composite`; if the union adds it as one atom and the store judge matches only top-level `Param`/`Local`, the parameter is hidden and the critical DEF-543 class is still accepted.
3. A `take` parameter root and a union containing both a parameter and a local root have no rule, so the diagnostic count/order is undefined; the `Binding` field is also not rolled back around the speculative loop/defer passes, so stale `Local`/`Temp` ids can name the wrong binding.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Move the union into `FlowState` as a joined map; strong-update on a definite whole assignment; snapshot/restore at speculation; flatten `storage_atoms`; define parameter-first precedence; add take/mixed controls | Uses the existing flow-sensitive machinery; no parallel state; preserves precision | Touches the flow-state merge and loop/defer passes; needs careful tests |
| B | Keep it on `Binding` but add a definite-assignment flag and snapshot/restore around loops/defers | Smaller edit | Still not branch-sensitive; false rejects/accepts remain; stale ids |
| C | Keep the may-union and accept the false rejects/accepts | No design change | Violates R4 (safe programs accepted) and the critical goal |

**Recommendation:** Option A, delegated under `--auto-design` (compiler mechanism, within the confirmed goal). It reuses `FlowState` rather than adding a parallel framework; the plan's "local join, not a dataflow framework" intent is preserved. Confidence: High; the only counterargument is the flow-state merge blast radius, which a focused spike can bound.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-005: DEF-456's containment test covers only one direction and uses undefined pointer comparisons [🔴 CRITICAL]

**Dimension:** Security Blind Spots
**Location:** `03-B-raw-store.md:33,42-46,58-62`; `99-execution-plan.md:114`
**Codebase Evidence:** `compiler/src/codegen/function.rs:3178-3196` (`emit_take_alias`) compares `((void *)address) == ((void *)(&target))`; the existing save is a `memcpy` of the destination. The roadmap's DEF-456 records two shapes: (1) a take of a super-object into a **member destination at a nonzero offset** (source contains destination), and (2) a first-member take into a **whole-struct destination** (destination contains source). `03-B:39` tests only "source lies inside the saved destination range", which covers (2), not (1). `compiler/src/codegen/program.rs:89` defines `usize` as `unsigned int` (32-bit on LP64); `compiler/runtime/shim_freestanding.c` has no header; C99 6.5.8p5 makes relational comparison of unrelated pointers undefined.

**The Problem:** As written, the first documented case still double-frees. The range test also needs the address type, the taken subobject's offset/length, and a clamp to the save; casting to `usize` truncates on the 64-bit host and the freestanding unit has no `<stdint.h>`. A bare pointer range comparison is UB for non-aliasing sources, which run through this path.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Test both directions (source⊆destination and destination⊆source), compare integer addresses via `unsigned long` or a compiler-internal `ClAddr`, carry the taken offset/length, clamp to the save; add a nonzero-offset case and a different-size alias | Covers both documented shapes; no C UB; host/64-bit safe | Needs a small address helper and both cases |
| B | Bind the super-object source to a temporary and move the member out per projection | Avoids range math | Larger codegen change; evaluation-order risk |
| C | Conservatively reject a super-object take into a member destination | No runtime test | New rejection behavior; precision loss |

**Recommendation:** Option A, delegated under `--auto-design` (memory-safety mechanism). Use the save (not the destination) as the range base, and pin a no-header freestanding address type. Confidence: High for the logic; Med for the freestanding type spelling (spike it).

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-015: DEF-457 needs a stack of take captures, not one `Option` [🔴 CRITICAL]

**Dimension:** Security Blind Spots
**Location:** `03-B-raw-store.md:34,58-66`; `99-execution-plan.md:115`
**Codebase Evidence:** `compiler/src/codegen.rs:258` holds one `Option<TakeCapture>`; `compiler/src/codegen/function.rs:2827` installs a nested capture with `std::mem::replace` and restores the outer only after the nested store. The DEF-457 roadmap note records that the outer save is not flagged. A nested conditional/materialization with its own save replaces the outer capture for its duration.

**The Problem:** A `take` inside the inner construct that aliases the **outer** destination sets only the inner flag; the outer flag stays clear, the outer save is released, and the value is double-freed. Saving/restoring one `Option` cannot represent two simultaneously live destinations, so "thread the capture through" is insufficient. ST-5 only names the `y439nestedarg` case and gives no concrete two-destination source.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Make `take_capture` a `Vec<TakeCapture>` stack; both alias emitters set every matching capture | Handles arbitrary nesting; small change | Must audit every push/pop and the save release |
| B | Carry a small set of `(flag, target)` pairs | Same coverage | Similar audit; less natural than a stack |
| C | Split the nested store into a separate statement before the outer save | Avoids nesting | Only where evaluation order permits; not general |

**Recommendation:** Option A, delegated under `--auto-design`. Add a two-destination nested test (outer and inner store) under ASAN. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

## Major Findings

### PF-006: DEF-416's module-qualified identity is a cross-cutting type-model change, not a two-file fix [🟠 MAJOR]

**Dimension:** Scope Creep Indicators
**Location:** `03-E-strict-build.md:19,27,31-32,47-48`; `02-current-state.md:33,102`; `99-execution-plan.md:231-232`; `00-index.md:24-34`; `07-testing-strategy.md:88`
**Codebase Evidence:** `compiler/src/types/mod.rs:57` is `Struct(String, Vec<Ty>)`; `types/keys.rs:49-70` derives C/monomorphization keys from that simple name; `codegen/types.rs:91-97` emits `Cl{name}` or `Cl{type_key}`; `types/table.rs:682-705` resolves a qualified path through its **last segment**. There are 75 `TyKind::Struct` references across the compiler. `spec/17-codegen.md:249` requires the user aggregate C name `Cl<Module>_<Type>` (PF-047 applied), while `03-E:31` says "preserve the simple-name spelling for a same-module reference". The existing `checker_conformance_spec_test.rs:11110-11134` (`qinitstructname`) stops at the front end because of the collision.

**The Problem:** The two-file task (`types/table.rs` + `types/mod.rs`) cannot distinguish `a.Thing` from `b.Thing` in `Ty` equality, `type_key`, or the C symbol. A same-module bare `Thing` also needs a stable identity, so "preserve the simple name" and "distinguish modules" cannot both hold in one global C namespace without a qualified key. The change reaches `types/keys.rs`, `codegen/types.rs`, `codegen/program.rs`, the resolver's missing module context, and the existing C pins (`ClS`, `ClPoint`). This is a material new type-identity surface; the plan's "Approved complexity: none" and "no new architecture" baseline are contradicted. ST-34 also omits the same-module bare-name control.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Carry a module-qualified identity through `TyKind::Struct`/a `TypeId`, keys, C naming, and the resolver; update the C pins; spec-conformant `Cl<Module>_<Type>` | Matches the frozen spec; both modules work | Large blast radius (75 refs, many pins); needs a Complexity Escalation Gate packet |
| B | Report the cross-module same-name collision with an existing diagnostic and keep the simple table | Smallest; no C change; matches the roadmap's "or report the collision" | Changes acceptance; a same-name pair is rejected rather than supported; needs an AR-5/AR-11 ruling |
| C | Defer DEF-416 to a separate plan and keep the row open | Honest scope | The closeout cannot claim all critical/major rows |

**Recommendation:** Option B if the user does not want the full identity layer; otherwise Option A with an explicit Complexity Escalation Gate packet and the full file/C-pin inventory. This is a **reserved product/spec decision** (R-3). Do not execute the two-file task as written.

**User Decision:** Resolved — User accepted recommendation (R-3): conservative collision report (E11101), no type-identity layer. Applied 2026-10-24 (AR-20).

### PF-007: ST-27 / DEF-505 is internally contradictory and not the concrete twin [🟠 MAJOR]

**Dimension:** Logical Contradictions
**Location:** `07-testing-strategy.md:76` (ST-27); `02-current-state.md:90`; `03-D-semantic-parity.md:33`; `99-execution-plan.md:196`
**Codebase Evidence:** The exact ST-27 source reports two `E10034` + `E10200` at HEAD. With `take h`/`take d`, the nested `h.inner.n` move reports `E10026` (partial moves are limited to top-level fields). The accepted concrete shape is a top-level field (`h.n` with `take h`), not the nested `h.inner.n`. A duplicable `n: i32` instance is accepted and has no "moved member" duty.

**The Problem:** "Accepted like the concrete twin; no `E10200`; one duty for a moved member" cannot describe one program. The row does not state `n`'s type, the modes of `h`/`d`, or `U`; "duty" is never defined as acceptance or a diagnostic. The owning nested member is rejected (E10026/E10034), while the accepted duplicable shape has no moved member. The row also omits the `h.inner.name` field form named in the roadmap.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Split: (i) duplicable `n: i32` nested projection accepted, no E10200, no duty; (ii) owning `n: String` nested projection rejected once with the concrete twin's E10026/E10034; add the `h.inner.name` duplicate case | Covers both the precision and safety sides; red for the right code | Two cases instead of one |
| B | Keep only the owning case; expect one E10026/E10034, no E10200 (no acceptance) | Simple | Drops the accepted precision control |
| C | Use a top-level `h.n` for acceptance | Concrete twin accepts | No longer tests the nested-projection defect |

**Recommendation:** Option A, delegated under `--auto-design` (test oracle within AR-5). Define "duty" once in 03-D. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-008: ST-23 / DEF-411 mixes five contexts and the wrong diagnostics [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:72` (ST-23); `02-current-state.md:86`; `03-D-semantic-parity.md:29`; `99-execution-plan.md:189-190`
**Codebase Evidence:** At HEAD `if (c ? x : null)` with `x: i32` reports `E10218` ("condition must be 'bool'"), not E10012; `i32?`/`bool?` are themselves invalid (`E10011`, "`?` applies to handles, raw, cstring, and views"); `i32(c ? x : null)`, `(c ? x : null) + 1`, and unannotated `let y = c ? x : null;` are accepted; a direct nullable slice loop is accepted. An annotated `let y: i32 = c ? x : null;` reports E10012.

**The Problem:** No single `x` type makes all five rows produce E10012. The i32 `if` is owned by E10218; an unannotated let has no non-nullable destination (and a nullable `i32?` is not a legal type); the loop-head rule is not grounded in the frozen spec and a direct nullable slice loop is accepted today. The row is a scenario list, not an oracle, and its expected code is wrong for at least the i32 guard.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Split per context: bool `if` → E10012; i32 `if` → E10218 (separate control); `i32(c ? x : null)` → E10012; `(c ? x : null) + 1` → E10012; annotated `let y: i32` → E10012; unannotated `let` → accepted nullable or a separate rule; loop head → only if the spec requires (else remove/file) | Each oracle is grounded; no false E10012 | More cases; the loop-head rule needs a spec ruling |
| B | Use bool for the `if` and assert E10012; keep the i32 guard as a separate E10218 control | Minimal | Other contexts still need types/spec grounding |
| C | Keep one E10012 expectation | No split | Wrong for the i32 guard and the unannotated let |

**Recommendation:** Option A, delegated under `--auto-design` (test oracle within AR-5). If the loop-head rejection is not in the frozen spec, remove it from the non-nullable list and file it separately. Confidence: Med (loop-head spec question).

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-009: DEF-415's expected report count is undefined (1 vs 2) [🟠 MAJOR]

**Dimension:** Logical Contradictions
**Location:** `02-current-state.md:89`; `03-D-semantic-parity.md:32,68`; `07-testing-strategy.md:75` (ST-26); `01-requirements.md:29-30`
**Codebase Evidence:** Two separate concrete arms (`let a: str = xs[0].name; let b: str = xs[0].name;`) each report one `E10010` — two total. `compiler/src/sema/ops.rs:458-475` (`report_null_conditional_arm`) is explicitly per-arm. The same-arm conditional at HEAD is accepted (the defect).

**The Problem:** 02/03-D say "once" and 03-D's testing rule says "exactly one diagnostic", while ST-26 says "once per arm". The central count assertion is undefined; a test can be written to pass with either `assert_rejected_once` or a count of two.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | One E10010 per nullable arm (two for ST-26); update 02/03-D/01 to "one per arm, never duplicated" | Matches the per-arm helper and the concrete twins | Changes the "once" wording in three docs |
| B | Deduplicate to one at the conditional; update ST-26 | One report | Contradicts the existing per-arm helper; needs a rule why the conditional differs |
| C | One per distinct nullable source (same binding → one) | Precision | More complex; not grounded |

**Recommendation:** Option A, delegated under `--auto-design`. Use an explicit count and name fragment in the test. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-010: The ST table is not an executable oracle [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:35-66,74,77-80,88,142-149`
**Codebase Evidence:** ST-1 lacks `: void` (probe: `E20000`); ST-15's `let s: String;` is uninitialized (probe: two `E10021`); ST-5 cites `y439nestedarg` with no fixture in the repo; ST-3/6/7/8/11 omit `unsafe` for raw derefs; ST-16/17/18/21 contain `tag: ...`/`Inner { ... }` ellipses; ST-19/20 give "expected trace" without a trace; ST-25/28/29/31/34 give "expected run"/"runs" without an exit value; ST-2/3/5/7 only assert ASAN.

**The Problem:** The plan calls the ST table an immutable oracle and the 07 checklist requires "concrete input/output pairs", but several rows are scenarios or fragments. The red phase can fail for an unrelated syntax/unsafe/uninitialized reason, and the executor must invent the expected value. The `source_union` shape pin in 99 1.3.2 is also not observable from an integration test (the field is private to the compiler crate); it belongs in `own/tests.rs`.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Complete every ST source (module/imports/`: void`/`unsafe`/`main`) and pin an exact exit code, diagnostic code/count/span, emitted-C fragment, or trace; move shape-only pins to impl tests; add a red-phase evidence table | Makes the oracle executable and auditable | Larger plan edit |
| B | Move all vague rows to implementation tests | Shorter ST table | Loses the specification oracle for those rows |
| C | Leave the derivation to the executor | No edit | The ST is no longer an oracle; test-fitting risk |

**Recommendation:** Option A, delegated under `--auto-design` (testing strategy). Add the red-phase table to `05-findings.md` (command, base commit, observed failure). Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-011: DEF-522's read/return form and arm backing are not pinned [🟠 MAJOR]

**Dimension:** Completeness Gaps
**Location:** `07-testing-strategy.md:80` (ST-31); `02-current-state.md:94`; `03-D-semantic-parity.md:37,71`; `99-execution-plan.md:202-203`
**Codebase Evidence:** The pre-edit DEF-522 roadmap note says "The read/return form of the same view-element projection is still accepted and emits invalid C ... pin both the move and read forms". A read form (`return (d ? h1 : h2).tag.ptr;`) is accepted at HEAD and the strict C build fails; the move form is rejected 2×E10034. `own/place.rs:842-883` follows conditional/arm view storage; a local arm dies with the frame.

**The Problem:** ST-31 only lists the move/store form `S { p: (d ? h1 : h2).tag.ptr }`; the read/return form can remain accepted-invalid after the row is marked Done. The row also never states whether `h1`/`h2` are borrows, statics, or locals, so a type-only acceptance could accept a local-backed view (UAF) with no test to stop it.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add the read/return forms (`.ptr`, `.len`, `tag[0]`) with a strict-C build; pin the arm backing (borrow/static); add a local-arm unsafe twin with the exact concrete diagnostic | Closes both forms; guards against a UAF | More cases |
| B | Fix only the move form and record the read form as a residual | Smaller | DEF-522 cannot be Done; contradicts R5 |
| C | Accept the projection on type substitution alone | Simple | Can accept a local-backed UAF |

**Recommendation:** Option A, delegated under `--auto-design` (safety/test coverage within AR-5). Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-012: Five documented forms are missing from the ST coverage [🟠 MAJOR]

**Dimension:** Completeness Gaps
**Location:** `07-testing-strategy.md:41,46,48,76,88`; `99-execution-plan.md:112,119,123,196,231-237`; `02-current-state.md:55,60,62,90,94`
**Codebase Evidence:** The pre-edit roadmap notes contain forms the ST rows drop: DEF-454's receiver spelling (the row says "argument/receiver" but only asserts the call argument); DEF-461's address-of fixed-array spelling (`cl_zero(&*(&items))` strict-C failure); DEF-464's accepted destination read ("the destination read stays accepted"); DEF-505's `h.inner.name` field form; DEF-416's same-module bare `Thing`. `07-testing-strategy.md:90-92` explicitly allows a row to split into several source cases.

**The Problem:** Each DEF has one ST row, but the row exercises only one spelling. A fix can pass the row while the receiver, address-of, destination-read, field, or same-module form remains broken. This is a completeness gap, not a new feature.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add the missing source cases under the existing ST numbers, with exact expected results | No count churn; closes the documented forms | Larger rows |
| B | Add separate ST numbers for the extra forms | Clear traceability | Changes the 34/35-row count again |
| C | Leave them to implementation tests | Shorter spec table | The forms are not in the immutable oracle |

**Recommendation:** Option A, delegated under `--auto-design` (testing strategy). Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-013: The cross-frame scan cannot see the rebound union and can withhold a needed guard [🟠 MAJOR]

**Dimension:** Dependency Issues
**Location:** `03-A-critical-flow.md:47-50,65-68`; `99-execution-plan.md:86-87`
**Codebase Evidence:** `compiler/src/own/call_check.rs:2189-2215` (`callee_has_whole_out_slot_store`) is a free syntactic scan over the callee AST; `:2385-2390` returns false for any written local; it has no access to the checker's `Binding.source_union` (the checker is for the current function only). A whole-body syntactic union would see a foreign source assigned **after** the store; the callee's flow-sensitive union at the earlier store would not, so both sides can stay silent.

**The Problem:** `1.2.3` says the scan "must also see the rebound union", but the scan cannot. A conservative scan that returns false yields two reports (caller E10030 + callee E10034), contradicting the plan's one-report rule; an over-eager whole-body union can withhold the caller guard for an earlier unsafe store (zero diagnostics). The plan gives no ordered-prefix or duty-recording rule.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Extend the scan to recompute each written local's assignment sources as an ordered prefix at the store and use that union; add a later-assignment test | No new summary abstraction; one report | A second syntactic source-state implementation; must match the ownership walk |
| B | Record the callee's whole-out-slot duty in the IR/sema and query it from the caller | Single source of truth | Larger cross-stage surface |
| C | Keep the caller guard and accept a duplicate | Simple | Violates the one-report rule and AR-8 |

**Recommendation:** Option A if the ordered-prefix scan is small; otherwise B. Delegated under `--auto-design` (implementation mechanism). Add a test where a later assignment adds a foreign source and the earlier store must still reject. Confidence: Med.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-014: DEF-460's "skip the copy-and-empty" leaves the save owning the value [🟠 MAJOR]

**Dimension:** Security Blind Spots
**Location:** `03-B-raw-store.md:35`; `99-execution-plan.md:117`
**Codebase Evidence:** `compiler/src/codegen/function.rs:2914-2919` handles the exact self-copy by restoring `target_c = save` and then `emit_move_zero(save)`; the default arm at `:2977-2979` copies and calls `emit_arm_source_moves`, which empties the source; `emit_conditional_move_store` always releases `save` after the branch (`:2859-2871`). A HEAD probe of `v = g ? (*p) : y` with `p = &v` emits a save copy and a post-branch `FreeString(__cl_save.name)`.

**The Problem:** "Skip the copy-and-empty" alone leaves `v` holding the handle and `__cl_save` holding a copy; the post-branch release frees the value `v` still points at. ST-6's "value preserved" becomes a use-after-free. The fix must restore the save into the destination and zero the save (or suppress that release). The DEF-466 element-store half does not have a save in the emitted shape, so scope this finding to DEF-460.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | On the runtime self-alias path, restore `target_c = save` and zero `save`; add an ASAN read-after-store | Matches the exact-self-copy branch; no double-free | One more branch |
| B | Suppress the save release on the alias path | Smaller | Fragile; must track the flag to the release |
| C | Do not create the save when the arm is provably the target | No runtime test | Only for provable spellings |

**Recommendation:** Option A, delegated under `--auto-design`. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-016: DEF-461's fixed-array guard must use the adjusted array address, in `function.rs` [🟠 MAJOR]

**Dimension:** Codebase Alignment
**Location:** `03-B-raw-store.md:36`; `99-execution-plan.md:119-120`
**Codebase Evidence:** `compiler/src/codegen/function.rs:3649-3660` (`is_array_param_place`) documents that `&(name)` addresses the pointer slot, not the array; `:3684` (`array_source_address`) exists for the adjusted address; `emit_move_place`/`emit_fixed_array_assign_to` are in `function.rs`. The task names `own/place.rs` and `codegen/calls.rs`.

**The Problem:** If the new guard uses the address-of the parameter, it compares the pointer slot with the target and misses the array alias; the source is copied and emptied while the target still owns the same storage — a double free. A guard that re-evaluates a side-effecting array source after the call is unstable. The plan says "keep a stable alias guard" but not the adjusted address or the insertion point.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Use `is_array_param_place`/`array_source_address` for the guard address; pin the adjusted pointer in a temp before the call; apply in `function.rs` | Correct address; stable | Small codegen change |
| B | Apply the guard only in `own/place.rs` | Matches the task text | The actual move/alias path is in codegen; the bug remains |
| C | Conservatively reject the aliasing parameter | No runtime test | Precision loss |

**Recommendation:** Option A, delegated under `--auto-design`. Add ST-7 with a parameter source, a distinct-address control, and ASAN. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-017: DEF-472/473 cannot be fixed in `instance_check.rs` alone [🟠 MAJOR]

**Dimension:** Feasibility Concerns
**Location:** `03-C-conditional-materialization.md:32-33`; `99-execution-plan.md:152-153`
**Codebase Evidence:** `compiler/src/own/expr.rs:107-130` records an arm only if `maybe_owns(&ty)` is true (true only for a type mentioning a parameter); a concrete `s: String` arm is dropped. `own/instance_check.rs` audits recorded duties only. HEAD probe: `pick<T>(c, take h: T, s: String) { let r: T = c ? h : s; }` at `T = String` is accepted; the concrete twin reports E10034 for `s`.

**The Problem:** A naive fix that marks concrete arms as maybe-moved would record a source-type-based duty and report at the safe `T = str` instance too (a view copy there is legal). The judgment needs both the concrete arm and the substituted destination; the current duty shapes do not carry that pair. `instance_check.rs` alone cannot see the dropped arm.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Record a destination-aware duty at the definition (concrete arm place/type + generic destination type); judge per instance; add a `T = str` safe control | Correct precision/safety; reuses the instance audit | New duty payload |
| B | Extend `RequiresNotLend`/`RequiresDuplicable` with the destination type and a definite-source flag | Smaller | Must preserve the existing duty semantics |
| C | Reject at the definition for a maybe-owning destination | Simple | Over-rejects the safe `T = str` view control |

**Recommendation:** Option A, delegated under `--auto-design` (instance-audit mechanism). Add the `T = str` safe control so a destination-blind fix fails. Confidence: Med.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-018: DEF-506/507's precision fixes lack a defined no-escape proof; DEF-507 appears stale [🟠 MAJOR]

**Dimension:** Implicit Assumptions
**Location:** `03-D-semantic-parity.md:34-35,41-50`; `99-execution-plan.md:197-199`; `07-testing-strategy.md:77-78`
**Codebase Evidence:** `call_check.rs` has only the boolean `callee_has_whole_out_slot_store`; it collects no viewed positions or return derivation. A HEAD probe of `H { name: String; tag: static str }` passed to a callee that views only `tag` is **accepted** (not the E10030 the plan claims); `always(xs)` returning `"ZZ"` is **accepted** in both the slot-store and return forms (not the E10035 the plan claims). A plausible DEF-506 unsafe twin (`storeName`, viewing `name`) reports one E10034.

**The Problem:** "The same direct scan" is overstated: DEF-506 needs a viewed-position proof (indirect calls, lambdas, returns, multiple stores) and DEF-507 needs a return-source proof; both are new bounded scans, not the existing whole-slot scan. An uncertain scan can suppress the caller guard and accept a UAF. DEF-507's documented red shape is not reproducible, so the row may be stale (AR-16). The plan's "if the direct scan cannot prove the safe case, stop under AR-12" conflicts with the hard closeout unless a fallback is chosen.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Phase 0 revalidates 506/507; close stale rows with HEAD evidence. For 506, define a conservative no-escape proof (every direct store/return of the slot is provably static and no indirect call/lambda/return can pass it); keep the rejection when uncertain; add unsafe twins | Honest; no UAF; no new summary | May leave 506 open if the proof is too weak |
| B | Approve a bounded per-callee viewed-position summary | Closes 506 | New support surface; Complexity Escalation Gate |
| C | Accept the type-level rule and record a limitation | No work | Row not closed; possible UAF |

**Recommendation:** Option A, delegated for the revalidation/conservative proof; if the proof fails and Option B is needed, stop at the Complexity Escalation Gate for a user ruling (R-5). Confidence: Med.

**User Decision:** Resolved — User accepted recommendation (R-5): Phase 0 revalidates 506/507 and uses the conservative proof; a summary surface needs a new ruling. Applied 2026-10-24.

### PF-019: No existing-test impact inventory; several frozen pins will change or stay stale [🟠 MAJOR]

**Dimension:** Completeness Gaps
**Location:** `07-testing-strategy.md:96-133`; `99-execution-plan.md` phase closes; `03-E-strict-build.md:49`
**Codebase Evidence:** Existing pins that will change or need review: `stage_spec_test.rs:533` (DEF-167, see PF-003); `compile_fail_spec_test.rs:50` (E10213/E10214 exclusion, see PF-002); `borrow_generic_integrity_spec_test.rs:1455-1481` (`generic-mixed` expects two E10034; ST-14 expects one concrete-twin rejection); `checker_conformance_spec_test.rs:11110-11134` (`qinitstructname` documents the DEF-416 collision); `borrow_store_integrity_spec_test.rs:2211-2345` (accepted out-slot controls); `store_emission_integrity_impl_test.rs:824` (`ClS`), `v1_language_impl_test.rs:631` (`ClPoint`) (C naming pins).

**The Problem:** The plan claims "existing stage/compile-fail suites stay green" but names no updates. At least the stage test, the generic-mixed count, and `qinitstructname` cannot stay green unchanged; a module-qualified C naming change also breaks the `ClS`/`ClPoint` pins. Without an inventory, the executor will either edit tests ad hoc or leave stale pins that no longer assert the intended rule.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add a Phase 0 inventory of every existing test touching each DEF and a retarget line per phase close | Auditable; avoids stale pins | One more Phase 0 task |
| B | Require the executor to update any failing test under the same DEF row | Less planning | Ad hoc; easy to miss |
| C | Leave it to the phase review | No planning | The failures appear late |

**Recommendation:** Option A, delegated under `--auto-design` (testing strategy). Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-020: The task-to-owner map points at the wrong stage for many tasks [🟠 MAJOR]

**Dimension:** Codebase Alignment
**Location:** `99-execution-plan.md:112,157-158,189-190,227-232`; `03-B-raw-store.md:19,31`; `03-C-conditional-materialization.md:20,31,56`; `03-E-strict-build.md:16-19`
**Codebase Evidence:** `struct_literal_needs_stores` is `codegen/function.rs:2580`; `conditional_struct_needs_stores` `:2448`; `emit_struct_value_store_arm` `:2492`; the plain literal is `codegen/expr.rs:1476`; the guarded literal path is `guards.rs:193/1340`; `program.rs` owns module-initializer literals. `E10300` is `sema/call.rs`; E10012 is `sema/ops.rs:466`, `own/expr.rs:1090`, `own/loops.rs:501`; DEF-171's `embed` is `sema/expr.rs:1294` plus the compile-fail harness; DEF-412's E10036 is `own/place.rs`/`own/statements.rs`; DEF-513's return walk is `own/expr.rs`; DEF-507's E10035 is `own/statements.rs`.

**The Problem:** An executor following the owner column edits the wrong stage for DEF-454, DEF-477, DEF-167, DEF-411, DEF-415, DEF-171, DEF-416, and the DEF-412/507/513 tasks. This is not merely imprecision: for DEF-167 and DEF-171 the named stage has no relevant code. The `ValueKind::Deref` variant in 03-C is phantom; the real shape is `ValueKind::Unary { op: UnaryOp::Deref }`.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Correct the task map per stage owner; add a "stage owner" column or a file-map table; fix the `Deref` spelling | Prevents wasted implementation | A documentation edit |
| B | Remove file hints from the 99 and rely on 03 integration points | Less to maintain | 03 also names some wrong modules |
| C | Add a Phase 0 file-target audit | Catches errors before coding | One more task |

**Recommendation:** Option A plus a Phase 0 spot-check, delegated under `--auto-design`. Confidence: High.

**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-021: The plan-creation roadmap edit erased the 33 rows' severity/evidence; the header is stale [🟠 MAJOR]

**Dimension:** Consistency
**Location:** `00-ambiguity-register.md:24` (AR-17); `99-execution-plan.md:47-53,93-94,130-131,169-170,208-209,238-239,258-259`; `05-findings.md:6-14`; working-tree `git diff plans/00-roadmap.md`
**Codebase Evidence:** The diff is 34 insertions/34 deletions. For each plan row the last column (severity + reproduction + evidence + fix direction) was replaced by an empty note/`Plan Created`, e.g. DEF-416 at `plans/00-roadmap.md:434` and DEF-454 at `:472` now have no severity or evidence. The header says `395/530 rows done; 135 open (1 critical, 34 major, 100 minor)`, while the actual tracker at HEAD has 396 Done and 136 open, and the working tree has 397 Done/33 Plan Created/102 open. The 33 Plan Created rows are unclassified; only DEF-458 retains its major marker.

**The Problem:** The roadmap lost the durable severity/evidence for the 33 active rows; only git history retains it. The header contradicts the rows, and Phase 0 0.2.3/acceptance criterion 5 ("no open critical/major row") cannot be checked from the roadmap labels alone. AR-17 says the roadmap is synced through the roadmap skill, but every phase close edits the table directly. The plan's `05-findings.md` has no per-row schema, so the R8 evidence has nowhere structured to go.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Restore the original severity/evidence notes from `HEAD:plans/00-roadmap.md`, append the plan link, fix the header counts; route phase closes through the roadmap skill; add a per-row `05-findings.md` table | Restores traceability; R8 has an evidence home | Edits a context document (`plans/00-roadmap.md`); needs the user's modification-set confirmation |
| B | Keep the empty notes and move all evidence to `05-findings.md` | No roadmap edit | The roadmap stays uninformative; the severity labels are lost from the cross-session ledger |
| C | Leave it and rely on git history | No work | The next session cannot see the severity/evidence |

**Recommendation:** Option A. This is a **reserved modification-set decision**: the audit target is the plan directory, and the roadmap is a context document; the user must confirm the expanded modification set (R-4). Confidence: High on the finding; the fix is a documentation restore.

**User Decision:** Resolved — User accepted recommendation (R-4): restore the roadmap evidence/header and route phase closes through the roadmap skill. Applied 2026-10-24 (AR-21).

### PF-023: DEF-479's ST-20 does not pin the scalar nested-literal order [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:64` (ST-20); `03-C-conditional-materialization.md:38`; `99-execution-plan.md:160`
**Codebase Evidence:** `compiler/src/codegen/expr.rs:1484-1502` renders fields in **declared** order; a scalar nested literal stays on the compound path unless `struct_literal_needs_guard_stores` detects a fixed-array field or an owned move. ST-20's `Outer { tag: tagOf(), inner: Inner { names: n1() } }` uses a fixed-array field, which may already take the per-field path; a scalar `Inner { n: n2() }` with a source order different from the declaration order still reorders at HEAD.

**The Problem:** The test can pass without exercising the scalar nested-literal bug. The plan's 03-C row says "Hoist/emit nested literal fields in source order", but ST-20 does not state the struct's declaration order or a reversed source order. A green ST-20 is not evidence that the scalar case is fixed.

**Options:**

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add a scalar nested-literal source whose source order differs from the declaration order; pin a trace; state the struct field order | Exercises the bug; red before the fix | One more case |
| B | Keep ST-20 and add the trace assertion only | Smaller | May still not exercise the scalar path |
| C | Leave it to the impl test | Shorter spec table | The spec oracle does not cover the scalar case |

**Recommendation:** Option A, delegated under `--auto-design` (testing strategy). Confidence: Med (the exact codegen path depends on the struct field order).

**User Decision:** AI — delegated by `--auto-design`: Option A.

## Minor Findings

### PF-022: Test-file/helper plan is ambiguous and partly impossible [🟡 MINOR]

**Dimension:** Ambiguities
**Location:** `99-execution-plan.md:57-58`; `07-testing-strategy.md:19-23,98-114`; `00-index.md:31-32`; `01-requirements.md:107`; AR-13
**Codebase Evidence:** No `compiler/tests/common` module exists; 62 files define `ScratchDir`, 54 define `run_clc`; every integration test duplicates its helpers. `07` lists four new spec files + four impl files (eight), while 99 0.2.2 says "four new spec/impl test files". `Binding` is private to the compiler crate, so the `source_union` shape pin cannot be observed from an integration test.
**The Problem:** "Four files" vs "four pairs" can leave the impl files missing; "shared helpers" either introduces a new common module (a new test-support surface, contrary to "no new harness" and AR-13) or is misnamed duplication. The internal shape pin belongs in `own/tests.rs`.
**Options:** (A) State "four spec/impl pairs (eight files), per-file helper copies; internal pins in `own/tests.rs`". (B) Authorize a `tests/common` module (new surface, needs a ruling). (C) Keep the wording and let the executor guess.
**Recommendation:** Option A, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-024: DEF-465's `__cl_rhs` path is inside the `needs_drop` gate; the fill twin's value is unpinned [🟡 MINOR]

**Dimension:** Edge Cases
**Location:** `03-B-raw-store.md:39`; `99-execution-plan.md:124`; `07-testing-strategy.md:49`
**Codebase Evidence:** `compiler/src/codegen/function.rs:1824-1834` puts the `__cl_rhs` path inside `if self.needs_drop(ty)`; a duplicable array goes to `emit_array_assign_to` and `a[0] = [a[0][1], a[0][0]]` yields `[2,2]` at HEAD. The plan already names the duplicable `__cl_rhs` case, but the fill twin's expected value is not pinned.
**The Problem:** An executor adding the `__cl_rhs` handling inside the existing gate leaves ST-10 red; the fill twin can also read the destination after the first element write. This is a small implementation detail, not a design gap.
**Options:** (A) Move the RHS evaluation outside the drop gate; pin the fill twin's value. (B) Add a separate duplicable path. (C) Leave it to the executor.
**Recommendation:** Option A, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-025: The stated B→A and C→B dependencies are false [🟡 MINOR]

**Dimension:** Ordering & Sequencing
**Location:** `02-current-state.md:104-112`; `99-execution-plan.md:267-287`
**Codebase Evidence:** DEF-464's post-store invalidation uses `conditional_move_sources`/moved-state paths (`own/expr.rs:242-276`, `own/place.rs:523-540`), not `Binding.source_union`. Batch C's ST cases do not call `emit_fixed_array_conditional_assign` or B's new capture helper; its helpers are the pre-existing `conditional_struct_needs_stores`/`emit_struct_value_store_arm` in `function.rs`. The strict chain is risk order; Batch E is independent but output-coupled to the C pins.
**The Problem:** The docs claim a technical dependency that does not exist; the executor may wait for or reuse the wrong helper. The order A→B→C→D→E is still safe, but the rationale should be risk order plus the actual shared files (`function.rs`, `guards.rs`, `instance_check.rs`).
**Options:** (A) Correct the dependency text. (B) Keep the chain and say it is risk order. (C) Reorder the phases (not recommended).
**Recommendation:** Option A/B, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-026: Phase count and small documentation inconsistencies [🟡 MINOR]

**Dimension:** Ambiguities
**Location:** `00-ambiguity-register.md:10`; `00-index.md:58`; `01-requirements.md:86`; `99-execution-plan.md:22-32`; `03-A-critical-flow.md:57`; `07-testing-strategy.md:35`
**Codebase Evidence:** The plan says "five phases" in AR-3/index/01 and "7 phases" in 99; the five batches are Phases 1-5 plus Phase 0/6. `03-A` writes `Fn store_source_union`; `07` ST-1 lacks `: void`; the range titles `DEF-454…466`/`DEF-470…481` include excluded IDs (458/459/463/474).
**The Problem:** A reader tracking "phase count" gets two answers; the `Fn` typo and the missing `: void` are small but make a copy-paste test fail.
**Options:** (A) Say "five batches (A–E) across seven execution phases (0–6)"; fix `Fn`/`: void`/titles. (B) Leave the wording. 
**Recommendation:** Option A, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option A.

### PF-027: DEF-412 has no unsafe twin or named safety pin [🟡 MINOR]

**Dimension:** Completeness Gaps
**Location:** `07-testing-strategy.md:73,130-131,144`; `03-D-semantic-parity.md:69-71`
**Codebase Evidence:** The fixture list names unsafe twins for DEF-506/507/522 but omits DEF-412. The existing E10035/E10036 suites (`compiler/tests/callable_release_integrity_*`, `conditional_projection_residue_*`) cover the view-return rule generally; the plan does not cite them as the safety pin.
**The Problem:** A precision fix could accept a substituted view-array return that the concrete rules reject; the 07 checklist says every accepted control needs an unsafe twin or a safety pin, but ST-24 has neither specified.
**Options:** (A) Add a local-backed E10035 twin and a two-candidate E10036 twin. (B) Cite the existing E10035/E10036 suites as the safety pin. (C) Rely on the generic safety tests.
**Recommendation:** Option A or B, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option B (cite the existing suite); add a twin if the suite does not cover the substituted form.

### PF-028: Phase C/D review lenses disagree between the index and the 99 [🟡 MINOR]

**Dimension:** Consistency
**Location:** `00-index.md:71-74`; `99-execution-plan.md:141,179`; `03-D-semantic-parity.md:37,71`; `AGENTS.md` phase-review dispatch
**Codebase Evidence:** The index assigns `semantics-reviewer` to Phases A, D, E, but 99 Phase 3 (C) lists semantics (DEF-478/479 are evaluation-order changes). 99 Phase 4 (D) omits emission-integrity, yet DEF-522 is a `codegen/expr.rs` change and compares emitted C. AGENTS.md requires emission-integrity for codegen changes.
**The Problem:** The phase review may skip the semantics lens for C and the emission lens for D, leaving a codegen regression until Phase 6.
**Options:** (A) Align the table and 99 lenses (semantics for C, emission for D). (B) Move DEF-522 to Phase C. (C) Rely on Phase 6.
**Recommendation:** Option A, delegated under `--auto-design`. Confidence: High.
**User Decision:** AI — delegated by `--auto-design`: Option A.

## Delegated Decisions (auto-design)

> **Authority:** AI — delegated by `--auto-design`
> **Eligibility:** Technical mechanisms, sequencing, and testing strategy within the confirmed goal (AR-5, AR-8, AR-15); no product behavior, scope, or acceptance-criteria change.
> **Objective:** Close every active critical/major DEF with a red regression and a sound direct fix.
> **Evidence:** The Codebase Evidence in each finding; HEAD probes; the independent cluster audit and the challenger.
> **Rejected alternatives:** See each finding's Options; the rejected choices are recorded above.
> **Strongest counterargument:** The plan's author chose a smaller `Binding` field/one-file fixes; the delegated choices sometimes grow the touched surface (`FlowState`, both-direction ranges, a capture stack). The counterargument is bounded by reusing existing machinery and by the requirement that a fix must not leave a double-free, false reject, or accepted-invalid program.
> **Hardening:** Independent challenger: 21 confirmed, 4 partially confirmed, 0 rejected. Adjustments: PF-014 scoped to DEF-460; PF-021 evidence (not the criterion) is the erasure; PF-006 drops the AR-12 label but keeps the surface; PF-023 keeps the scalar case; PF-024 downgraded.
> **Confidence:** High for PF-002, 004, 005, 007, 009, 010, 011, 012, 014, 015, 016, 019, 020, 022, 025, 026, 028; Med for PF-008, 013, 017, 018, 023.
> **Policy version:** 1
> **Root invocation ID:** `preflight-def-critical-major-closeout-2026-10-24`
> **Reopen triggers:** a reserved decision changes the goal/spec/scope; a spike invalidates a chosen mechanism; a Phase 0 stale row changes the active set; a new test/scan surface is needed.

| PF | Delegated decision | Confidence |
|---|---|---|
| PF-002 | Drop the positive `embed` control; assert one E10213 + no C; re-enable the E10213 golden; keep E10214 excluded; record const-context `embed` as the staged E10224 | High |
| PF-004 | Move the source union into `FlowState` with join/strong update/rollback; flatten `storage_atoms`; add take/mixed-root controls | High |
| PF-005 | Both-direction containment with an integer address type, taken offset/length, clamp; add nonzero-offset and different-size cases | High/Med |
| PF-007 | Split DEF-505 into accepted duplicable + rejected owning cases; add the `h.inner.name` form | High |
| PF-008 | Split DEF-411 per context; keep E10218 for the i32 `if`; add a positive nullable-let control; ground or drop the loop-head rule | Med |
| PF-009 | One E10010 per arm (two for ST-26); update the "once" wording | High |
| PF-010 | Complete every ST source; pin exact values/diagnostics/traces; add a red-phase table; internal pins in `own/tests.rs` | High |
| PF-011 | Add DEF-522's read/return forms and a local-arm unsafe twin; pin the arm backing | High |
| PF-012 | Add the five missing forms under the existing ST numbers | High |
| PF-013 | Ordered-prefix callee scan (or IR duty) + later-assignment test | Med |
| PF-014 | Restore/zero the save on the DEF-460 self-alias path; ASAN read-after-store | High |
| PF-015 | `Vec<TakeCapture>` stack + two-destination nested test | High |
| PF-016 | Adjusted array address + stable temp; owner `function.rs`; add ST-7 parameter-source control | High |
| PF-017 | Destination-aware recorded duty + `T = str` safe control | Med |
| PF-018 | Phase 0 revalidate 506/507; close stale; conservative no-escape proof for 506; stop at the Complexity Gate if a summary is needed | Med |
| PF-019 | Phase 0 existing-test inventory + per-phase retarget line | High |
| PF-020 | Correct the task-to-owner map; fix the `Deref` spelling | High |
| PF-022 | Four spec/impl pairs (eight files), per-file helpers; internal pins in `own/tests.rs` | High |
| PF-023 | Add a reversed scalar nested-literal source + trace; state the struct field order | Med |
| PF-024 | Move the `__cl_rhs` evaluation outside the `needs_drop` gate; pin the fill twin | High |
| PF-025 | Correct the dependency text to risk order + real shared files | High |
| PF-026 | "Five batches across seven phases"; fix `Fn`/`: void`/titles | High |
| PF-027 | Cite the existing E10035/E10036 suites as DEF-412's safety pin, or add the twin | High |
| PF-028 | Align the review lenses (semantics for C, emission for D) | High |

## Fix Application Log (2026-10-24)

The user accepted the recommendations for R-1…R-5 and authorized applying the
delegated fixes. No compiler code, spec chapter, or golden file was changed.

| Surface | Applied change |
|---|---|
| `plans/00-roadmap.md` | Restored the 33 plan rows' severity/evidence notes (escaped code pipes), linked `DEF-458` to the plan, and fixed the header to `397/532` done / `135 open (34 Plan Created: 1 critical + 33 major; 101 Backlog minor)`. |
| `00-ambiguity-register.md` | Added AR-18…AR-22 and the resolution notes; status now 22/22. |
| `00-index.md` | Counts are 35 critical/major at HEAD, 34 active (1 critical + 33 major), 35 DEF rows, five batches across seven phases; added the preflight report, the DEF-458/DEF-416/DEF-167 decisions, and corrected the file map/review lenses. |
| `01-requirements.md` | R2 has eleven Batch B DEFs; R5 is the E11101 collision; counts and acceptance criteria use 34 active / 35 DEF rows / 35 ST rows; the 101 minor rows are the excluded set. |
| `02-current-state.md` | Added DEF-458; DEF-416 is the collision report; DEF-506/507 are revalidation rows; corrected the file map and dependency edges. |
| `03-A-critical-flow.md` | The source union moved to `FlowState` with strong update/join/rollback, `storage_atoms` flattening, take-root and mixed-root precedence, and an ordered-prefix callee scan. |
| `03-B-raw-store.md` | Added DEF-458; DEF-456 tests both directions with integer addresses; DEF-460 restores/zeros the save; DEF-457 uses a capture stack; DEF-461 uses the adjusted array address; DEF-465 leaves the `needs_drop` gate; file paths and tests corrected. |
| `03-C-conditional-materialization.md` | DEF-472/473 use a destination-aware recorded duty; DEF-477 is `function.rs`; DEF-479 pins the scalar trace; DEF-522 pins the read/return and arm backing. |
| `03-D-semantic-parity.md` | DEF-411 is split per context; DEF-415 reports one per arm; DEF-505 has accepted/owning cases; DEF-506/507 revalidate with the conservative proof; DEF-522 covers read/return; DEF-412 cites the safety pin. |
| `03-E-strict-build.md` | DEF-167 uses the parse-only mode; DEF-171 re-enables the E10213 golden and drops the positive embed control; DEF-416 is the E11101 collision report. |
| `05-findings.md` | Added the preflight ruling table and the R-1…R-5/AR-22 record. |
| `07-testing-strategy.md` | Rewrote the ST table: 35 rows, concrete expected values/sources, ST-35 for DEF-458, the DEF-411 split, the DEF-415 two-report rule, the DEF-505 split, the DEF-522 read/return, the DEF-416 collision, and the E10213/parse-only changes. |
| `99-execution-plan.md` | 76 tasks across 7 phases; Phase 0 has the test-impact inventory; Phase 2 has DEF-458; Phase 5 has the parse-only mode; file paths, dependencies, and success criteria are corrected. |

**Not applied:** no compiler implementation, no spec chapter, no golden file,
and no commit. Iteration 2 must verify the corrected plan and the new counts.

## Reserved Decisions (user)

> **Status (2026-10-24):** R-1…R-5 were accepted and applied; see the Fix
> Application Log and AR-18…AR-22. The entries below are the historical
> decision packets, kept for the audit trail.

### R-1 — Add DEF-458 or defer it (PF-001)
- **Decision needed:** Add DEF-458 to Batch B (recommended; 34 active, 35 plan rows incl. DEF-470, 76 tasks after the preflight task additions, Batch B 11 DEFs/17 tasks) or explicitly defer it and change the goal/acceptance criteria to the named 33 rows.
- **Evidence:** `plans/00-roadmap.md:476` open major; HEAD reproducer accepted with an 11-byte LSan leak.
- **Why reserved:** Product scope and acceptance criteria are reserved authority.

### R-2 — DEF-167 stage-spec oracle (PF-003)
- **Decision needed:** Add a parse-only/stage-select mode to `clc` (recommended if acceptable; small driver surface) or amend the frozen stage test with AR-11 approval; a narrow rule that leaves the local-value form accepted does not close DEF-167.
- **Evidence:** `stage_spec_test.rs:24-47,533-555`; E10300 in `UNACCEPTABLE_CODES`; the stage test runs the full compiler.
- **Why reserved:** A frozen specification-test change requires explicit AR-11 approval; a new driver surface is a support-surface decision.

### R-3 — DEF-416 behavior and complexity (PF-006)
- **Decision needed:** (A) Implement module-qualified identity end-to-end (`TyKind`/key/C name/resolver, spec-conformant `Cl<Module>_<Type>`, update the C pins) with a Complexity Escalation Gate packet; (B) report the cross-module collision with an existing diagnostic (smallest, no C change, changes acceptance); or (C) defer DEF-416 to a separate plan.
- **Evidence:** `TyKind::Struct(String, ...)` 75 refs; `types/keys.rs`, `codegen/types.rs`; `spec/17-codegen.md:249`; `qinitstructname` fixture.
- **Why reserved:** Product behavior/spec and a material new type-identity surface; the plan's "Approved complexity: none" is false for option A.

### R-4 — Restore the roadmap evidence/header (PF-021)
- **Decision needed:** Authorize the modification set to restore the 33 rows' severity/evidence notes from `HEAD:plans/00-roadmap.md`, append the plan link, and fix the header counts; route phase closes through the roadmap skill.
- **Evidence:** `git diff plans/00-roadmap.md` 34/34; the rows lost their severity/evidence; the header says 135 open/395 done while the tracker has 136 open/396 done.
- **Why reserved:** `plans/00-roadmap.md` is a context document outside the plan audit target; the preflight scope contract requires the user's confirmation to expand the modification set.

### R-5 — DEF-506 summary surface, only if the conservative proof fails (PF-018)
- **Decision needed:** If Phase 0 confirms DEF-506 and the conservative no-escape proof cannot be built with the existing scans, approve a bounded per-callee viewed-position summary or keep the conservative rejection (the row stays open).
- **Evidence:** `call_check.rs` has only the boolean whole-slot scan; the DEF-506 safe shape is accepted at HEAD, but a plausible unsafe twin reports E10034; DEF-507's filed red shape is not reproduced.
- **Why reserved:** A new summary surface is a Complexity Escalation; the plan's AR-12 stop rule.

## Verification of the Audit

- **Independent cluster audit:** 5 clusters (document soundness, grounding, delivery, risk, fit) returned 88 raw findings; the lead merged/deduplicated to 28 PF findings.
- **Hardening challenger:** 21 confirmed, 4 partially confirmed, 0 rejected. The partials were adjusted as noted.
- **Probes:** DEF-470 (one E10020), DEF-543 (accepted), DEF-473 (accepted), DEF-472 (accepted), DEF-415 (accepted; two concrete E10010), DEF-411 (E10218 for i32 `if`; others accepted), DEF-505 (2×E10034 + E10200; E10026 concrete nested; accepted top-level), DEF-167 (accepted; invalid C), DEF-171 (accepted; invalid C; E10224 const), DEF-458 (accepted; 11-byte LSan leak), DEF-506 (accepted; unsafe twin E10034), DEF-507 (accepted).
- **Same-agent bias:** The plan was created in a previous session (2026-10-23), not this one. The independent clusters and challenger provide review independence. A human compiler/semantics expert is still recommended for the DEF-416 type-identity choice and the DEF-167 spec-test oracle.
- **Adversarial questions:** (1) The creation assumed the 33-row list was complete; DEF-458 disproves it. (2) C99 pointer comparison and the frozen `Cl<Module>_<Type>` naming are external constraints the plan does not address. (3) A domain expert would flag the non-flow-sensitive source union and the save-ownership paths; both are confirmed above.

## Verdict

**🟡 FIXES APPLIED — iteration 2 verification pending.**

The 28 findings and the R-1…R-5 rulings were applied to the plan documents and
the roadmap on 2026-10-24. The corrected plan now has:
- 34 active DEFs (1 critical + 33 major) and 35 DEF rows including `DEF-470`.
- 35 ST cases (ST-35 for `DEF-458`); ST-12 is the green stale pin.
- 76 tasks across 7 phases.
- The DEF-167 parse-only mode, the DEF-416 E11101 collision report, and the
  restored roadmap evidence.

**Not yet verified:** no compiler code was built, no tests were run, and no
iteration 2 scan was performed in this session. A fresh-session preflight must
verify the corrected plan, the new counts, and the ST oracles before execution.

**Roadmap:** Not advanced. This report remains the iteration 1 audit and the
durable record of the rulings.

---

# Iteration 2 — Fresh-Session Re-Scan (2026-10-24)

> **Previous iteration:** 28 findings (6 critical, 16 major, 6 minor); all 28 applied on 2026-10-24.
> **This iteration:** 20 findings (1 critical, 14 major, 5 minor). 19 resolved by plan edits; 1 reserved (`R-6`).
> **Carried forward:** PF-033 (`R-6`).
> **Audit target:** unchanged — all 12 plan documents. Strict scope; no `--explore-scope`.
> **Independence:** the plan was created on 2026-10-23 and iteration 1 ran in a previous session. Iteration 2 ran in a fresh session with five independent cluster auditors and lead re-verification against HEAD `ef6fd95`.

## Iteration 2 cluster verification

| Cluster | Dimensions | Raw findings |
|---|---|---|
| Document soundness | 1, 3, 12 | 7 |
| Grounding | 2, 13 | 8 |
| Delivery | 4, 5, 11 | 6 |
| Risk | 8, 9, 6 | 8 |
| Fit | 7, 10 | 8 |

The lead deduplicated the 37 raw findings into 20 `PF` roots. The largest merges were PF-003 (four clusters), PF-004 (two), PF-005 (two), PF-008 (two), PF-010 (three), PF-019 (two), PF-029 (two), and PF-030 (two). No cluster finding was dismissed as a strawman; the merged roots below carry the strongest evidence from each.

## Iteration 1 fix verification

| PF | Status | Evidence |
|---|---|---|
| PF-001 | ✅ Verified | `DEF-458` is in the index, ST-35, task 2.2.11, and the 34/35/76 counts |
| PF-002 | 🟠 Residual | The E10213 constant-context mechanism is still missing; see PF-002 |
| PF-003 | 🔴 Residual | The filed local-i32 shape is still outside the full-compile oracle; see PF-003 |
| PF-004 | 🟠 Residual | The source set's lifecycle and take/destination rules; see PF-004 |
| PF-005 | 🟠 Residual | The containment wording and the subrange carrier; see PF-005 |
| PF-006 | 🟠 Residual | The E11101 owner cannot be `types/table.rs`; see PF-006 |
| PF-007 | ✅ Verified | ST-27 is split into accepted duplicable, owning E10026, and field E10034 |
| PF-008 | 🟠 Residual | The loop head is still optional; see PF-008 |
| PF-009 | 🟡 Residual | `02-current-state` still says "once"; see PF-009 |
| PF-010 | 🟠 Residual | Several ST rows remain scenarios; see PF-010 |
| PF-011 | 🟠 Residual | ST-31 still lacks the backing and exact code; see PF-011 |
| PF-012 | ✅ Verified | The receiver, address-of, destination-read, field, and bare-name forms are present |
| PF-013 | 🟡 Residual | The path rule is still undefined; see PF-013 |
| PF-014 | ✅ Verified | The save is restored and zeroed on the self-alias path |
| PF-015 | ✅ Verified | The capture stack replaces the single `Option` |
| PF-016 | ✅ Verified | The adjusted array address is named in `function.rs` |
| PF-017 | 🟠 Residual | The destination owners are incomplete; see PF-017 |
| PF-018 | ✅ Verified | The conservative proof and the AR-12 stop rule are present |
| PF-019 | 🟠 Residual | The inventory is partial and no phase close retargets it; see PF-019 |
| PF-020 | ✅ Mostly | The cited owners are corrected; the residual owners are PF-006/PF-017 |
| PF-021 | ✅ Verified | The roadmap header is 397/532; 34 Plan Created; evidence restored; `DEF-458` linked |
| PF-022 | ✅ Verified | Four pairs/eight files; the internal pin is in `own/tests.rs` |
| PF-023 | 🟠 Residual | ST-20 does not require a differing declaration order; see PF-023 |
| PF-024 | ✅ Verified | The `__cl_rhs` evaluation is outside the `needs_drop` gate |
| PF-025 | ✅ Verified | The dependency text is risk order, with the real shared files |
| PF-026 | 🟡 Residual | AR-1 counts, ST task ranges, and the `DEF-470` wording; see PF-026 |
| PF-027 | ✅ Verified | ST-24 cites the existing E10035/E10036 suite |
| PF-028 | 🟡 Residual | Phase 2/5 lens lists; see PF-028 |

## Findings

### PF-003 (reopened) — DEF-167's filed local-i32 shape is not in the full-compile fix or oracle [🔴 CRITICAL]

**Dimension:** Completeness Gaps / Codebase Alignment
**Location:** `07-testing-strategy.md:97` (ST-32); `99-execution-plan.md:300-312` (5.2.1/5.2.4); `03-E-strict-build.md:19-22,29,33-36,50-53`; `02-current-state.md:102`
**Codebase Evidence:** `compiler/tests/stage_spec_test.rs:533-555` is the frozen stage shape and uses `a: i32` — a local value, not a free function. `compiler/src/sema/call.rs:133-146` sends a non-generic free function to `check_args_direct`, which ignores `type_args`; `:189-238` falls through for a local non-callable callee and returns `Ty::unknown()` without checking the type arguments. A HEAD probe of the exact stage source exits 0 and emits `call_shaped = a(c);`, which strict C rejects with "called object 'a' is not a function or function pointer".
**The Problem:** ST-32 mixed "a non-generic function `a`" with "the local-i32 stage shape", and the full-compile E10300 expectation was tied to the callable forms. Task 5.2.1 listed free, qualified, dot, and func-typed callees only. The parse-only mode (AR-19) makes the stage test pass, so an executor can implement the four named forms and close DEF-167 while its filed reproducer still emits invalid C. This is the residual of the PF-003 root cause, not a new scope.
**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Make ST-32 require a full compile of the exact local-i32 source to report one `E10300`; add the non-callable local/parameter fallthrough to 5.2.1 | Closes the filed shape; uses AR-14's existing E10300 | One more ST case |
| B | Scope DEF-167 to the callable forms and file the local-value form separately | Smaller | Leaves the roadmap row's own reproducer open; fails AR-8 |
**Recommendation:** A — it is the only option that closes the filed defect with the already-approved diagnostic.
**User Decision:** AI — delegated by `--auto-design`: Option A, within AR-14/R-2. Applied to ST-32, 5.2.1, 03-E, and 02.

### PF-002 (reopened) — `embed` has no constant-context signal [🟠 MAJOR]

**Dimension:** Logical Contradictions / Feasibility
**Location:** `03-E-strict-build.md:30,45,54-57`; `99-execution-plan.md:303-307`; `07-testing-strategy.md:98` (ST-33)
**Codebase Evidence:** `compiler/src/sema/expr.rs:1294-1362` has no `embed` arm and no context flag. `compiler/src/sema/items.rs:112-134` runs `require_constant` (E10224 via `consts/mod.rs:591`) and then `check_into_slot` → `check_value` → `check_intrinsic`. `consts/mod.rs:515-563` folds only `sizeof`, `offsetof`, and `len`; `embed` is `NotConstant`.
**The Problem:** A global `embed` arm fires in both contexts, so a module `const embed` would report E10224 and E10213, contradicting the plan's "const-context stays staged as E10224". The plan named only `sema/expr.rs` and the harness, not the context signal.
**Options:** A — add a constant-context flag around the constant positions and report E10213 only outside; B — report E10213 everywhere and accept the double report.
**Recommendation:** A — it preserves AR-15 and the staged const behavior.
**User Decision:** AI — delegated by `--auto-design`: Option A. Applied to 5.2.2, 03-E, ST-33, and 02.

### PF-033 (new) — `spec/04` §16 and `spec/13` §1 disagree on a module-level `let` initializer [🟠 MAJOR — RESERVED]

**Dimension:** Logical Contradictions / Security Blind Spots
**Location:** `03-E-strict-build.md:33-41,45`; `07-testing-strategy.md:98` (ST-33); `01-requirements.md:33-39`
**Codebase Evidence:** `spec/04-expressions.md:363-367` says "a module-level `let` initializer is runtime too; neither is a constant context". `spec/13-data-inclusion.md:56-66` says "A module-level initializer is also a constant context (E10213), so an embedded asset can back a module-level `let`" and shows `let palette: u8[64] = embed(...)`. DI-4 says `embed` is allowed only in a constant context. At HEAD a module-level `let embed` is accepted and emits invalid C; a module `const embed` reports E10224.
**The Problem:** If the plan follows `spec/04`, a module-level `let embed` must report E10213, rejecting a shape `spec/13` explicitly allows. If it follows `spec/13`, the shape stays accepted and emits invalid C until data inclusion lands (out of scope). The plan's current R5 text claims DEF-171 broadly; it cannot do so without choosing. This is a frozen-spec conflict, so AR-11 applies.
**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Follow `spec/04` §16: module-level `let embed` reports E10213; record the `spec/13` sentence as a frozen limitation; no chapter change | Rejects the invalid C; matches AR-15 and the E10213 golden | Rejects a spec-13 example |
| B | Follow `spec/13` §1: leave module-level `let embed` accepted; record the data-inclusion gap; E10213 only for function-body/local forms | Preserves the spec-13 example | Leaves an accepted-invalid program in an active row |
| C | Implement constant-context data inclusion | Makes the example build | Out of scope; a new feature |
**Recommendation:** A — it preserves the program's "no accepted invalid C" gate and the existing E10213 golden; the user should confirm because it changes behavior for a spec-sanctioned shape.
**User Decision:** **Pending — R-6.** The plan now scopes R5 to the function-body/local-`const` forms and records the conflict; no spec chapter changed. Confidence: High on the conflict; Med on A.

### PF-004 (reopened) — the source set's lifecycle and judgment rules [🟠 MAJOR]

**Dimension:** Codebase Alignment / Security Blind Spots
**Location:** `03-A-critical-flow.md:23-67`; `99-execution-plan.md:99-113`; `02-current-state.md:46-50`
**Codebase Evidence:** `compiler/src/own/flow.rs:191-246` defines `FlowState`/`PlaceState`; `drop_bindings_from` retains only `places`, `loans`, `element_loans`, and `carried_owners`, and `edge_state` (`own.rs:1377-1387`) removes only `places`/`loans`/`element_loans`. `merge` (`own.rs:1409-1481`) is a hand-written `PlaceState` construction. `whole_slot_store_root` (`own/expr.rs:2505-2507`) skips `owner == destination`; `store_source_provenance` (`own/place.rs:2594-2608`) returns `Local(id)` for a rebound local; a take view dies with the frame (`own/place.rs:1017-1026`).
**The Problem:** The applied iteration-1 text moved the source union to `FlowState` but did not add it to the pruning hooks; a speculative/defer temporary id could be reused with a stale source. It also omitted the destination-root exclusion (a safe self-copy would false-reject), the take-view discriminator (a take view could be accepted as an owned take), and the type-parameter deferral (a safe `T = str` instance could be rejected). The "maybe assignment joins" wording could also defeat the strong update for a conditional RHS.
**Options:** A — keep a parallel map and add all hooks; B — put the set on `PlaceState`, so `merge` joins it and `drop_bindings_from`/`edge_state` prune it automatically, and state the destination/take/type-parameter rules explicitly.
**Recommendation:** B — fewer hooks, no silent omission; the lifecycle comes from the existing place machinery.
**User Decision:** AI — delegated by `--auto-design`: Option B. Applied to 03-A, 99 1.2.1-1.2.3, 02, and ST-1 (slot-copy, take-view, owned-take, generic, all-static conditional controls).

### PF-005 (reopened) — DEF-456's containment wording and subrange carrier are inconsistent [🟠 MAJOR]

**Dimension:** Codebase Alignment / Feasibility
**Location:** `03-B-raw-store.md:38,48-54,60-64,75-79`; `07-testing-strategy.md:53` (ST-4); `99-execution-plan.md:146-147`
**Codebase Evidence:** The save is a separate copy (`codegen/function.rs:2769-2775`; the fixed-array path uses `memcpy` at `:1975-1985`). The current alias test compares the taken source to `&target` (the destination), not the save (`:3192-3194`). `TakeCapture` is only `{flag, target}` (`codegen.rs:264-269`); the post-branch action is a whole-save zero or drop (`function.rs:2867-2871`, `:2030-2048`).
**The Problem:** The plan said "the save may contain the source ... or the source may contain the save", but the containment is source vs destination; the save copy's address never participates. It also required a subrange clear while claiming the `(flag, target)` capture shape was unchanged, so the executor had no place to carry the offset/length. The probes show the first-member case leaks the sibling and the super-object case double-frees.
**Options:** A — test source vs destination, map the taken subrange onto the save copy, and attach the range to the capture; B — keep the whole-save clear (the sibling leaks).
**Recommendation:** A — it preserves the both-direction rule and the existing save.
**User Decision:** AI — delegated by `--auto-design`: Option A. Applied to 03-B, ST-4, and 2.2.3.

### PF-006 (reopened) — DEF-416's E11101 cannot be reported from `types/table.rs` [🟠 MAJOR]

**Dimension:** Codebase Alignment
**Location:** `03-E-strict-build.md:31,37,46,58-61`; `99-execution-plan.md:307-309`; `02-current-state.md:104`
**Codebase Evidence:** `types/table.rs:118` is `pub fn build(loaded: &Loaded, layout: AbiLayout) -> Self` — no diagnostics sink. The table keys by simple name and overwrites (`:182-190`); the first pass already collapses the two declarations. The existing E11101 check is per module in `load/mod.rs:385-400`; the only cross-module precedent is the generic-symbol check in `codegen/program.rs:37-66`, which has a sink.
**The Problem:** Reporting from the table constructor requires a new signature and storing the first declaration's module/span; the table cannot name the second declaration today. The natural owner is `sema` (it has diagnostics and the table) with a pure table query.
**Options:** A — a `sema` cross-module check with a pure `TypeTable` collision query; B — give `TypeTable::build` a sink and store the first declaration.
**Recommendation:** A — it reuses the existing diagnostic and keeps the table pure.
**User Decision:** AI — delegated by `--auto-design`: Option A. Applied to 03-E, 5.2.3, and 02; `qinitstructname` is retargeted.

### PF-008 (reopened) — DEF-411's loop-head SEGV is still optional [🟠 MAJOR]

**Dimension:** Edge Cases / Security Blind Spots
**Location:** `07-testing-strategy.md:83` (ST-23(g)); `03-D-semantic-parity.md:34`; `99-execution-plan.md:251-252`; `02-current-state.md:88`
**Codebase Evidence:** `compiler/src/own/loops.rs:495-505` reports E10012 only when `is_null_shaped(iterable_ty)` (unknown+nullable); a nullable-array conditional iterable is accepted. A HEAD probe of `for (v in c ? a : null)` exits 0 and emits the conditional dereference; the roadmap row records the SEGV. `spec/02-types.md:126` (TY-6/TY-7) and the existing loop rule ground the rejection.
**The Problem:** ST-23(g) said "only if the frozen spec requires" and allowed removal. Removing it lets DEF-411 close while the filed SEGV remains; keeping it left no expected code. The active row already includes the loop head and its fix direction.
**Options:** A — make the nullable conditional loop head a mandatory E10012 case; B — remove it and file a new DEF (leaves the row's SEGV open).
**Recommendation:** A — the existing null rule and the filed row decide it.
**User Decision:** AI — delegated by `--auto-design`: Option A. Applied to ST-23(g), 03-D, 4.2.1, and 02.

### PF-009 (reopened) — DEF-415's summary still says "once" [🟡 MINOR]

**Dimension:** Consistency
**Location:** `02-current-state.md:91`; `03-D:37,67`; `07:86` (ST-26); `01:29-31`
**Codebase Evidence:** `sema/ops.rs:458-475` reports one E10010 per nullable arm; the iteration-1 probe records two E10010 for the two-arm twin.
**The Problem:** 02 said "Report the concrete null rule once" while 03-D/ST-26 require two. An executor could implement a dedupe.
**Recommendation:** A — update 02 to "one per nullable arm (two for ST-26)". **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-010 (reopened) — the ST table is still not fully executable [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:51-61,73,74,76,77,88,89,97` (ST-2…ST-11, ST-18, ST-21, ST-22, ST-28, ST-29, ST-32)
**Codebase Evidence:** `spec/04-expressions.md:111-112` requires `unsafe` for `&`/`*`; probes show `let p: raw S = &a` is E10601 and `*alt(p,q)` without the block is E10600. `spec/07-structs.md:43` makes a missing field E10203; the ST-21 literal omitted the `row` field it then projects (`sema/expr.rs:1137-1154`). `struct_literal` renders declared order (`codegen/expr.rs:1473-1509`).
**The Problem:** Several rows still fail for a syntax/unsafe/uninitialized/missing-field reason before reaching the defect, or describe a scenario with no source (ST-18, ST-22, ST-28, ST-29). ST-4/ST-5 had no concrete source; ST-7 had no exit value; ST-10's fill value was not exact. A red phase can pass or fail for the wrong reason.
**Options:** A — write the exact `unsafe` contexts, sources, exit values, and expected traces/values; B — move the vague rows to implementation tests (loses the spec oracle).
**Recommendation:** A.
**User Decision:** AI — delegated by `--auto-design`: Option A. Applied to ST-2…ST-11, ST-18, ST-21, ST-22, ST-28, ST-29, ST-32, and the checklist.

### PF-011 (reopened) — ST-31 still leaves the backing, read value, and escape code open [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:91` (ST-31); `03-D:42,88`
**Codebase Evidence:** `own/place.rs:862-882` keeps a conditional local-arm provenance; `spec/14-diagnostics.md:58,61` distinguishes E10030 (scope) from E10033 (field/global escape). The local-backed arm dies with the frame, so E10030 is the concrete code.
**The Problem:** The row instructed the executor to "state whether h1/h2 are borrow/static"; (c) allowed E10030 or E10033; (b) had no exact read value. A type-only acceptance could accept a local-backed UAF.
**Recommendation:** A — pin `h1: borrow H`, `h2: borrow H`, `(c)` E10030, and `(b)` length 2/exit 0. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-013 (reopened) — the cross-frame scan's path rule is undefined [🟡 MINOR]

**Dimension:** Dependency Issues / Feasibility
**Location:** `03-A-critical-flow.md:61-67`; `99:107-113`; ST-1 at `07:43`
**Codebase Evidence:** `call_check.rs:2189-2215` builds `locals`/`written` as whole-body syntactic passes; `value_has_foreign_root:2385-2390` returns false for any written local. The scan has no CFG.
**The Problem:** "Ordered prefix ... on the same path" is not computable syntactically; a linear prefix can over- or under-approximate. Over-approximation withholds the caller guard unsoundly; under-approximation duplicates the report.
**Recommendation:** A — define a must-source walk (intersection at branches, conservative at loops/defers); keep the caller guard when unprovable. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-017 (reopened) — DEF-472/473's destination is known outside the named owners [🟠 MAJOR]

**Dimension:** Codebase Alignment
**Location:** `03-C-conditional-materialization.md:25-31,36-37,63-65`; `99:199-205`
**Codebase Evidence:** `own/statements.rs:1008-1045` has the early `maybe_owns` branch; `own/place.rs:594-663` records the duty; `ir/mono.rs:172-201` `RequiresNotLend` has no destination type; `instance_check.rs:212-286` judges it.
**The Problem:** The task named only `own/expr.rs`/`instance_check.rs`; the destination type is known in `statements.rs`, the duty payload is in `mono.rs`, so the audit is blind or unimplementable.
**Recommendation:** A — thread the destination through `BindingEffect::MoveIfOwning`, `mark_moved_if_owning`, and `RequiresNotLend`; judge it in `check_not_lend_requirements`. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied to 03-C and 3.2.2/3.2.3.

### PF-019 (reopened) — the test-impact inventory is partial and not retargeted [🟠 MAJOR]

**Dimension:** Completeness Gaps
**Location:** `99:65-68` (0.2.4); phase closes at `99:120-121,168-170,226-230,274-277,318-321`; `03-E:58-61`; `checker_conformance_spec_test.rs:11091-11134`; `borrow_generic_integrity_spec_test.rs:1470-1483`
**Codebase Evidence:** `qinitstructname` calls `emit_pair` and asserts success; `generic-mixed` expects two E10034; the conditional-projection/residue, store-emission, call-value, runtime-value, eval-emission, module-initializer, callable-release, and unit families pin changed shapes.
**The Problem:** The six-item inventory is a starter, not the full impact; no phase close owns the retarget, and `qinitstructname`'s "comment is updated" does not change its success assertion.
**Recommendation:** A — extend 0.2.4, add a retarget line to each phase close, and assign `qinitstructname`/`generic-mixed` to their owners. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-023 (reopened) — ST-20 can pass at HEAD without exercising the scalar bug [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07:75` (ST-20); `03-C:42,99`
**Codebase Evidence:** `codegen/expr.rs:1473-1509` renders a compound literal in declared order; the per-field path is chosen only for a fixed-array field or an owned move (`guards.rs:1215-1247`). Probes: declaration `tag, inner` + source `tag, inner` → `tagOf,n2` (passes); declaration `inner, tag` → `n2,tagOf` (red).
**The Problem:** The row said "with the struct declaration order stated" but did not require it to differ from the source order. With the matching order the scalar twin passes at HEAD.
**Recommendation:** A — require `struct Outer { inner: Inner; tag: i32; }` and pin the source-order trace. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-026 (reopened) — document consistency residue [🟡 MINOR]

**Dimension:** Consistency
**Location:** `00-ambiguity-register.md:8`; `99:135-140,191-195,243-248`; `02:14-17`; `99:48-54`
**Codebase Evidence:** The working tree has 135 open (34 Plan Created + 101 minor), `DEF-470` Done; AR-1 still said "100 minor + 2 unclassified" and the task ranges still included DEF-459/463/474; the `DEF-470` wording still said "close".
**Recommendation:** A — update AR-1, replace the ST ranges with explicit lists, and reword `DEF-470` as re-record/verify. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-028 (reopened) — Phase 2/5 review lenses [🟡 MINOR]

**Dimension:** Consistency
**Location:** `99:132,291`; `00-index.md:80-83`; `AGENTS.md` phase-review dispatch
**Codebase Evidence:** Batch B contains DEF-458 (take capture), DEF-462 (pin before the RHS), and DEF-465 (RHS before the clear); AGENTS.md requires semantics for evaluation-order changes. Phase 5 is a correctness-relevant parser/type change but omitted the correctness lens.
**Recommendation:** A — add semantics to Phase 2 and correctness to Phase 5. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-029 (new) — the stale-green pins contradict the acceptance criteria [🟠 MAJOR]

**Dimension:** Logical Contradictions
**Location:** `01:37-45,119-124`; `99:396-402`; `07:28-30,101-102,155-160`; `03-D:54-56,86-87`; `99:260-264`
**Codebase Evidence:** The plan explicitly allows DEF-506/507 to close stale with a green HEAD pin; iteration 1 found DEF-507's documented red shape not reproducible. R6, acceptance 2, and success 3 named only ST-12 as the green pin.
**The Problem:** If either row closes stale, the plan's own completion gate is unsatisfiable without a fake red test, violating AR-16.
**Recommendation:** A — allow AR-16 stale green pins in R6, acceptance 2, success 3, and the 07 wording; add the proof requirement. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-030 (new) — Phase 0 runs the probe before the baseline build and pins a field before it exists [🟠 MAJOR]

**Dimension:** Ordering & Sequencing
**Location:** `99:48-61` (0.1.1, 0.2.1, 0.2.2) vs `99:99-113` (1.2.1, 1.3.2)
**Codebase Evidence:** `target/debug/clc` does not exist on a clean checkout before `make check`/`cargo build`; `FlowState` has no source field at HEAD. 0.2.2 asked for the internal source-set pin before 1.2.1 creates it.
**The Problem:** The probe can use a missing or stale binary, and the pin cannot compile; the Phase 0 `make check` gate fails or the pin becomes a placeholder.
**Recommendation:** A — build the workspace in 0.1.1 before probing; reserve the pin location in 0.2.2 and author it in 1.3.2. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

### PF-031 (new) — ST-9's `idx = 1` shape is a valid self-copy, not the DEF-464 defect [🟠 MAJOR]

**Dimension:** Testability / Feasibility
**Location:** `07:58` (ST-9); `03-B:44,74-75`; `99:157-160`
**Codebase Evidence:** The existing accepted `p423cond` case (`conditional_projection_residue_spec_test.rs:20193-20215`) uses `let idx = 0`, `set_idx()` sets `idx = 1`, destination `a[0]`, and reads `a[0]`. With `idx = 1` from the start, source and destination are both `a[1]`; the self-alias path restores the save, so the later `a[1]` read is valid. The run-time source path is an `ElementTemp`; `note_maybe_moved_use` (`own/place.rs:523-590`) uses structural `starts_with`, so a constant `a[1]` read does not match it.
**The Problem:** The ST-9 oracle expected E10020 for a shape that is a safe self-copy; and even the `p423cond` shape needs a selected-element rule for the run-time source before the later constant read can be rejected while `a[0]` stays accepted.
**Recommendation:** A — pin `idx = 0` then `set_idx` sets 1 (destination `a[0]`, run-time source `a[1]`), and state the selected-element rule in 2.2.8. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied to ST-9, 03-B, and 2.2.8.

### PF-032 (new) — the AGENTS.md test-naming map omits the four new pairs [🟡 MINOR]

**Dimension:** Completeness Gaps
**Location:** `AGENTS.md` test-naming map; `07:19-25`; `99:58-61`
**Codebase Evidence:** The map lists the existing families but not `def_raw_store_integrity_*`, `def_conditional_value_integrity_*`, `def_semantic_parity_integrity_*`, or `def_strict_build_integrity_*`; `store_emission_integrity_*` is also omitted.
**Recommendation:** A — add the four pairs (and `store_emission`) in 0.2.2. **User Decision:** AI — delegated by `--auto-design`: Option A. Applied.

## Iteration 2 fix log

| Surface | Applied change |
|---|---|
| `00-preflight-report.md` | This iteration-2 section, the header, and the evidence |
| `00-ambiguity-register.md` | AR-1 counts; the R-6 note in Confirmation |
| `00-index.md` | Audit-baseline HEAD; `DEF-167`/`DEF-171`/`DEF-416` decisions; AR-16 green pins |
| `01-requirements.md` | R4 per-arm count; R6/acceptance 2 stale pins; R5 scope and the R-6 note |
| `02-current-state.md` | DEF-415 per arm; DEF-505 no duty/owning; DEF-167 local form; DEF-171 flag/R-6; DEF-416 owner; Batch A take/destination rules; risks |
| `03-A-critical-flow.md` | Source set on `PlaceState`; strong update/join; destination/take/type-parameter rules; must-source walk |
| `03-B-raw-store.md` | Source-vs-destination containment; capture range; DEF-456 sources; DEF-464 destination/source pin |
| `03-C-conditional-materialization.md` | DEF-472/473 destination owners; DEF-479 differing declaration order |
| `03-D-semantic-parity.md` | DEF-411 loop-head E10012; stale green pins |
| `03-E-strict-build.md` | DEF-167 local-i32 full compile; DEF-171 context flag/R-6; DEF-416 `sema` owner; `qinitstructname` retarget |
| `05-findings.md` | Iteration-2 rulings; red-phase evidence template |
| `07-testing-strategy.md` | ST-1, ST-2…ST-11, ST-18, ST-20…ST-23, ST-28/29, ST-31…ST-35; green-pin wording; checklist |
| `99-execution-plan.md` | Phase 0 build order and pin timing; explicit ST lists; must-source walk; DEF-456/464/472/473/416/167/171 owners; phase retarget lines; Phase 2/5 lenses; success criterion 3; R-6 stop |

**Not applied:** no compiler implementation, no spec chapter, no golden file, and no roadmap stage advance.

## Reserved decision R-6

- **Decision needed:** For a module-level `let embed`, follow `spec/04` §16 (runtime, so `E10213`) or `spec/13` §1 (constant context, so the shape is allowed but data inclusion is staged and currently emits invalid C). A third option is to implement constant-context data inclusion, which is out of scope.
- **Recommended:** A — follow `spec/04` §16; report E10213; record the `spec/13` sentence as a frozen inconsistency. This preserves the "no accepted invalid C" gate and the AR-15 decision.
- **Why reserved:** It changes behavior for a shape one frozen chapter explicitly allows, and AR-11 reserves spec-conflict rulings to the user.
- **Evidence:** `spec/04-expressions.md:363-367`; `spec/13-data-inclusion.md:56-66`; DI-4; the HEAD probes (module `const` → E10224; module/local `let` → accepted, invalid C).
- **Blocking:** Phase 5.2.2 and DEF-171's module-level `let` control. The other 33 active DEFs and Phases 0–4 are ready.

## Iteration 2 verdict

**❌ PREFLIGHT BLOCKED — 1 reserved major (PF-033 / R-6); 19 findings resolved.**

The plan is corrected and internally consistent for the 33 other active DEFs. The iteration-2 re-scan verified the counts, links, DEF set, ST numbering, task ordering, and the codebase owners; the applied fixes are recorded above. The plan cannot be marked `Plan Preflighted` while R-6 is open.

**Roadmap:** Not advanced — a BLOCKED verdict does not advance the target rows. After the user rules R-6, re-check ST-33 and 5.2.2, then advance the 34 linked rows to `Plan Preflighted` through the roadmap skill.

**Same-agent bias:** iteration 2 is a fresh session relative to the plan creation and iteration 1; the five cluster auditors and the lead's HEAD probes provide independent evidence. A human compiler/semantics reviewer is still recommended for the `spec/04`/`spec/13` ruling.
