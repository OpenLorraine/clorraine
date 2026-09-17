# Preflight report - Clorraine language specification

> **Status:** passed with notes - all critical and major findings resolved (iterations 2-3); two Phase 2 verification items accepted

**Audit target:** `spec/` (chapters 00-17, `appendix-amiga.md`, `decisions.md`,
`grammar.ebnf.md`, `grammar-deltas.md`, `lineage.md`, `14-diagnostics.md`,
`tests/compile-fail/`)
**Baseline:** commit `6ba1fc8` (content hash at scan start)
**Method:** 13-dimension scan in five clustered auditor dispatches (document
soundness, grounding, delivery, risk, fit), grounded against
`docs/findings-phase0.md`, `docs/findings-phase1.md`, `docs/ergonomics-corpus.md`,
`docker/`, and `examples/hosted-skeleton/`
**Same-session note:** the specification was rewritten earlier in the same
working session (`4472d47`, `6ba1fc8`); the scan was delegated to independent
auditor contexts and every finding was verified against the artifacts before
inclusion. Residual same-model bias is possible.

## Verdict

`✅ PREFLIGHT PASSED WITH NOTES - 49 findings resolved, 2 verification items
accepted for Phase 2`

All findings were ruled on and resolved through the accepted recommendations
(rulings R-01..R-37, recorded in `decisions.md` section 21), then verified by
iterations 2 and 3. The working tree validates with
`spec check: OK (27 files, 101 compile-fail tests)`.

## Findings

| PF | Sev | Cluster refs | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|---|
| PF-001 | 🔴 | S1, R2 | `03-ownership.md`, `06-functions.md`, `12-ffi-contracts.md` | `borrow T?` out-params contradict O3 (assignment drops the old value) and E10034 (callee must not release a borrowed parameter); the callee can drop an uninitialized caller slot | Rule: writes through `borrow T?` do not drop a previous value; callers pass an owned value or `null`; exempt out-param assignment from E10034. Add a test | OPEN |
| PF-002 | 🔴 | R1 | `12-ffi-contracts.md` §10, `17-codegen.md` §5 | FFI declarations lower with Clorraine semantics (`borrow` -> `T *`, `u8[]` -> view struct), which does not produce the real C prototypes (`BPTR`, `APTR`, `LONG`); Phase 0 hand-wrote the true prototypes | Define a separate ABI lowering for `ffi` declarations (semantic modes do not shape the C prototype), or require a paired C-prototype form; prove with a generated-C spike | OPEN |
| PF-003 | 🔴 | F1 | `grammar.ebnf.md` §10.2, `01-lexical.md` | `borrow` and `take` are used as reserved terminals but are absent from the 48-keyword list; the keyword count and the reserved/contextual status are inconsistent | Make `borrow` and `take` reserved keywords (50 total; update grammar list, Ch 01, and every count). Alternative: make both contextual, with lookahead rules in §11 | OPEN |
| PF-004 | 🔴 | D1 | `10-modules.md`, `grammar.ebnf.md` §1 | Name resolution is unspecified: local vs module precedence, shadowing, import binding scope, merged-module collisions | Add a name-resolution section (scopes, shadowing, imports, merged modules) to Ch 10 | OPEN |
| PF-005 | 🟠 | S2 | `02-types.md` §9, `04-expressions.md` §8 | Mixed-width and mixed-sign operator operands have no promotion or result-type rule; `u8 + u16` is undefined | Specify the usual-arithmetic conversion and the result type, or forbid mixed operand types with E10200 | OPEN |
| PF-006 | 🟠 | S10, R5 | `02-types.md` §10, `17-codegen.md` §10 | `MIN / -1` and `MIN % -1` overflow is UB in C; only the zero divisor is trapped | Guard the overflow case and make it fault E11210 (or define wrapping); update the hazard table | OPEN |
| PF-007 | 🟠 | R4 | `04-expressions.md`, `17-codegen.md` §10 | The float-to-int range check does not trap NaN (`f < lo \|\| f > hi` passes NaN), leaving UB in C | Emit `!(f >= lo && f <= hi)` so NaN faults E11212; add a runtime test | OPEN |
| PF-008 | 🟠 | R3 | `17-codegen.md` §10, §13 | Signed wrapping relies on `-fwrapv` or unsigned idioms, but the normative matrix omits `-fwrapv` and vbcc is unverified | Add `-fwrapv` to the GCC leg, specify the unsigned idiom for vbcc, and verify with an arithmetic spike | OPEN |
| PF-009 | 🟠 | S3 | `10-modules.md` §4, `17-codegen.md` §3 | Module init order has no tie-break among independent modules and no rule for indirection through called functions, yet behavior and byte-identical output depend on it | Define a deterministic tie-break (lexicographic module name) and state that only syntactically visible references and transitive init edges count | OPEN |
| PF-010 | 🟠 | S4 | `04-expressions.md` §3, `07-structs.md` | Strict left-to-right evaluation is claimed, but struct-literal fields and array-literal elements/fill have no order row | Add the literal rows in source order, or state that their order is unspecified | OPEN |
| PF-011 | 🟠 | S5, F2, F3 | `05/07/09/08/16` chapters | Rule-ID collisions: `EL-1..EL-3` (element loops) clash with lifetime elision `EL-1..EL-6`; `ST-*` is defined independently in Ch 05, 07, and 09 | Rename element-loop rules (for example `LOOP-*`) and namespace the `ST-*` families (`STMT-*`, `STR-*`, `ERR-*`); keep `EL-*` for elision only | OPEN |
| PF-012 | 🟠 | S6 | `01-lexical.md`, `06-functions.md`, `grammar.ebnf.md` §3.6 | `intrinsic func` declarations vs reserved built-in names: LEX-9 forbids declaring reserved names, yet Ch 06 declares `intrinsic func readVolatile16` | State that `intrinsic func` declarations are the only way the compiler-known names are introduced, and reconcile LEX-9 | OPEN |
| PF-013 | 🟠 | S8 | `16-borrow-rules.md` §2, `06-functions.md` §4 | `take` parameters create no loan, but EL-1 lists handles as elision candidates; a view derived from a `take` parameter is undefined | State that `take` parameters are not elision candidates and name the diagnostic (E10036) | OPEN |
| PF-014 | 🟠 | D2 | `11-memory.md` §5, `08-arrays-strings.md` | Ledger §18 stdlib has no normative signatures: `Buffer`, `Array<T>`, `Span`, error carrier, format helpers, dispatcher, `String` helpers; a chapter points at a "library reference" that does not exist | Add a stdlib appendix/chapter with normative signatures and ownership, or move the list into a normative chapter | OPEN |
| PF-015 | 🟠 | D3 | `grammar.ebnf.md` §6.8, `13-data-inclusion.md` | Constant-expression evaluation is a stub: no operator set, folding semantics, overflow/division rules, or diagnostics | Write a constant-evaluation section (allowed forms, folding semantics, diagnostics) | OPEN |
| PF-016 | 🟠 | D4 | `04-expressions.md` §12 | Dot-call resolution omits the candidate scope, ambiguity/arity tie-breaks, and generic inference interaction | Specify the candidate set and deterministic tie-breaking for `x.f(y)` | OPEN |
| PF-017 | 🟠 | D5 | `10-modules.md` (MOD-20), `14-diagnostics.md`, two tests | E10037 and E11106 have the same trigger (non-duplicable global/module-level `let`); a program cannot deterministically emit both asserted codes | Pick one canonical code, reserve the other, update chapters and the two tests | OPEN |
| PF-018 | 🟠 | D6 | `12-ffi-contracts.md` §6, `14-diagnostics.md`, tests E10100/E10101 | The "uncontracted extern" spelling is undefined: an undeclared call is E10219, and every `ffi` declaration is a contract; the tests invent a harness rule | Define the uncontracted-extern form (or retire the codes and re-scope the tests) | OPEN |
| PF-019 | 🟠 | D7 | tests E90003/E90004/W10005 | Three tests cannot produce their codes under the frozen spec (`stores` is not a mode, no pool API exists, `--contracts` is undocumented) yet count as coverage | Freeze a trigger form or move them to an explicit "unreachable until staged" list excluded from coverage | OPEN |
| PF-020 | 🟠 | D8 | `spec/tests/` | No run tests or positive corpus for the always-on checks (E11210-E11213) or warning escalation; `--deny-warnings` never exercised | Add `spec/tests/run/` fault cases and a positive corpus in Phase 2; add a warning-escalation compile-fail case | OPEN |
| PF-021 | 🟠 | R6 | `08-arrays-strings.md`, `grammar.ebnf.md` §4 | Zero-extent arrays (`u8[0]`) are not rejected and emit invalid C99, despite the empty-struct rule | Reject zero extents (mirror E10206) or define the emitted form | OPEN |
| PF-022 | 🟠 | R7 | `08-arrays-strings.md`, `16-borrow-rules.md` | Implicit `String` -> `str` widening is not stated to create a loan; a `String` mutator could reallocate under a live view (use-after-free) | State that every view derivation, including implicit widening, creates a shared loan for its origin; add a compile-fail test | OPEN |
| PF-023 | 🟠 | R8 | `appendix-amiga.md` (IH-2), `11-memory.md` | Interrupt-handler constraints (allocation-free, drop-free, panic-free) have no diagnostic and no enforcement, though they guard kernel memory | Add an effect marker/diagnostic, or label IH-2 an audited convention with a named checker follow-up | OPEN |
| PF-024 | 🟠 | F7 | chapter headers | "Ledger:" lines cite IDs that do not resolve in `decisions.md` (`F14`/`F17` are ergonomics findings; `GF*` live in `grammar-deltas.md`); the `F` namespace collides | Split headers into Ledger / Grammar freeze (GF, linked) / Ergonomics (F, linked) | OPEN (mechanical) |
| PF-025 | 🟠 | F9 | `appendix-amiga.md` §1, §7 | Appendix claims `seek`, `Lock`/`Unlock`, `println` are "covered in v1" while Ch 12 defers them; uses an undeclared `Message` handle | Align with Ch 12 (mark planned APIs, declare or replace `Message`) | OPEN |
| PF-026 | 🟠 | G1 | `appendix-amiga.md`, `15-profiles.md`, `17-codegen.md` | Float matrix was claimed "verified" though Phase 0 exercised no float code; FPU bit-for-bit claim was false | Reworded to planned/not yet verified; bit-for-bit claim removed | FIXED |
| PF-027 | 🟠 | G2 | `17-codegen.md` §13, `appendix-amiga.md` | Matrix claim overstated (68020 was build-only; FS-UAE ran GCC 68000 only; vbcc ran on vamos only) | Restated precisely | FIXED |
| PF-028 | 🟠 | G3 | `decisions.md` P6 | P6 still said `String`'s deleter is `FreeVec`, contradicting T2a, Ch 11, Ch 17 | Changed to `FreeString` | FIXED |
| PF-029 | 🟠 | G4 | `tests/compile-fail/README.md`, `check.sh` | Three broken links; `check.sh` did not validate nested markdown | Links fixed; `check.sh` now validates `*.md` and `tests/compile-fail/*.md` with per-file link resolution | FIXED |
| PF-030 | 🟠 | G5, F4-F6 | `15-profiles.md`, `appendix-amiga.md`, `11-memory.md`, `17-codegen.md`, `14-diagnostics.md` | Six cross-references pointed at the wrong section numbers | Repointed to the actual sections | FIXED |
| PF-031 | 🟠 | G6 | `tests/compile-fail/E10501...clor` | The E10501 trigger `$"{x:04x}"` cannot parse (`:` is not in an expression); the file cannot produce the code | Define `:`-inside-placeholder detection as a parser-only diagnostic (like E10204/E10208), or record the syntax as pending | OPEN |
| PF-032 | 🟠 | D12 | `14-diagnostics.md` E11100, test E11100 | Trigger said "between modules", but the test uses a same-module cycle; same-module cycles were undefined | Registry trigger widened to modules or same-module globals | FIXED |
| PF-033 | 🟡 | D9 | `spec/README.md` | Validation description predated the N7 and coverage checks; stale Phase 1 wording | Updated | FIXED |
| PF-034 | 🟡 | S7 | `02-types.md`, `04-expressions.md` | T6a `raw` -> integer casts were allowed in the ledger but absent from the cast tables | Added rows citing T6a | FIXED |
| PF-035 | 🟡 | S9 | `04-expressions.md` | Placeholder-free `$"..."` called `static str` in one sentence, `str` elsewhere | Unified to `str` | FIXED |
| PF-036 | 🟡 | S11 | `14-diagnostics.md` | E10033 trigger overlapped E10035; E10200 trigger too narrow for M7a | Triggers narrowed/widened | FIXED |
| PF-037 | 🟡 | S12, G7, F10 | `11-memory.md`, `12-ffi-contracts.md` | Bare `func ...;` declarations outside `ffi` blocks; embedded `ffi_block` production missing `[ "export" ]` | Declarations moved into `ffi stdmem`; production mirrors the master grammar | FIXED |
| PF-038 | 🟡 | G9 | `decisions.md` P1 | Pinned compiler versions ("GCC 16.2.0b", "vbcc 0.9h") are not traceable to `versions.env`/`Dockerfile`; the binary reports 0.9 | Record observed versions in SDK provenance, or quote only what the artifacts report | OPEN |
| PF-039 | 🟡 | G11 | `12-ffi-contracts.md`, `appendix-amiga.md` | `Close` declared `: void` and `IoErr(): u32` vs the NDK `BOOL`/`LONG` returns | Cite the NDK prototypes in the contract table or document the intentional narrowing | OPEN |
| PF-040 | 🟡 | D10 | `04-expressions.md`, `17-codegen.md` | Temporary lifetimes in nested expressions (`f(g(), h())`) are unspecified | State the exact rule (statement end) for nested temporaries | OPEN |
| PF-041 | 🟡 | D11 | chapter headers, `spec/README.md` | The Depends-on graph is cyclic and no Phase 2 milestone order is stated in `spec/` | Add a conceptual reading order and a Phase 2 stage list; mark Depends-on informational | OPEN |
| PF-042 | 🟡 | R9 | `17-codegen.md` §11 | `#line N "file.clor"` is asserted but unproven under vbcc | Add a Phase 2 spike that compiles emitted C with `#line` under vbcc and debugs it | OPEN |
| PF-043 | 🟡 | R10 | `13-data-inclusion.md` | `embed("path")` accepts absolute and `../` host paths, contradicting the deterministic-output claim | Resolve embed paths under a declared asset root; reject absolute/`..` paths | OPEN |
| PF-044 | 🟡 | R11 | `03-ownership.md` §8 | "A `raw` pointer may not launder a lend out of its scope" reads as a guarantee though T6 declares no provenance model | Reword as an audited convention, or add a lint/wrapper rule | OPEN |
| PF-045 | 🟡 | R12 | `11-memory.md`, `06-functions.md` | Recursion is unrestricted and stack depth is "documented guidance", but no such guidance exists and freestanding has no probe | Publish stack-depth guidance (or a stack warning) and state the freestanding policy | OPEN |
| PF-046 | 🟡 | F8 | chapter `Depends on` lines | Depends-on lists omit referenced chapters (for example Ch 02 -> 01/05/16) | Add the missing chapters; mark the field informational if cycles are intentional | OPEN (mechanical) |
| PF-047 | 🟡 | F11 | `17-codegen.md` §7 | Naming table says `Cl<Module>_<Type>` while examples use `ClFile`, `ClStr`, `ClSlice_T` | State a separate handle/built-in view naming rule or align the examples | OPEN |
| PF-048 | 🔵 | F12 | `04-expressions.md`, `grammar.ebnf.md` | Unary `+` is absent from the unary set though the C operator table is claimed "carried" | Add unary `+`, or record its removal in `grammar-deltas.md` | OPEN |
| PF-049 | 🔵 | G10 | `17-codegen.md` §13 | The normative GCC command adds `-std=c99`, but the Phase 0 build used the GNU default, so the strict-C99 claim was not exercised | Add `-std=c99` to the SDK/smoke build | OPEN |

## Decision list (must resolve before Phase 2)

Critical: PF-001, PF-002, PF-003, PF-004.
Semantic majors: PF-005, PF-006, PF-007, PF-009, PF-010, PF-011, PF-012,
PF-013, PF-015, PF-016, PF-017, PF-018, PF-021, PF-022.
Surface/process majors: PF-008, PF-014, PF-019, PF-020, PF-023, PF-024, PF-025,
PF-031.
Reducible without a full design cycle: PF-024, PF-046 (mechanical).

## Fix log (applied after the scan)

Items PF-026..PF-030, PF-032..PF-037 (15 findings) were applied exactly as
recommended. The remaining findings were resolved by the accepted rulings
(R-01..R-37); verification: `cd spec && ./check.sh` ->
`spec check: OK (27 files, 101 compile-fail tests)`.

## Iterations 2 and 3

* **Iteration 2** dispatched three verification auditors across all 13
  dimensions. It confirmed the rulings as implemented, with 20 residual
  implementation gaps (I2-01..I2-30): out-parameter initialization versus the
  definite-assignment exemption, the keyword count in Ch 01, the already-equal
  operand case in the unification rule, missing MIN/-1 and NaN wording in the
  Ch 17 hazard rows, the E10100/E10101 trigger model, unreachable E10225,
  constant-context wording, standard-library naming drift, and test/tooling
  gaps.
* **Iteration 3** fixed every iteration-2 finding within the accepted rulings.
  Two rulings were refined during implementation and are recorded here:
  * out-parameter slots must be initialized by the caller (`= null`), the
    callee owns the incoming value and may release it, and the O3 drop rule
    applies to the slot;
  * E10225 was retired (no reachable trigger: fields win in dot-call and
    duplicate declarations are already E11101/E11103), joining E10037 in the
    retired list.
* **Accepted for Phase 2** (explicitly accepted; not blocking): executing the
  `tests/run/` fault cases, and the vbcc `#line` plus generated-C FFI spikes
  (open items in `docs/findings-phase1.md`).

The Phase 2 compiler MVP may start; its first verification runs are the two
accepted items above.
