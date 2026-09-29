# Requirements: Runtime Value Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Seven filed defects share one theme: a runtime value the compiler already accepts is
materialized, typed, or evaluated incorrectly. A call-rooted element-loop iterable emits
invalid C, an owning compiler temporary is never released, a fixed-array conditional value
byte-copies a decayed pointer, a runtime-index callable-field receiver is never
bounds-guarded, a `func` value with a fixed-array return carries the wrong C pointer type,
a non-literal `panic` message fails the C compile, and a `take` argument inside a
short-circuit operand moves eagerly. This plan fixes all seven against the frozen
specification and keeps every existing accepted program compiling.

The specification is the only oracle: `spec/05` §9 (element loops over an indexable
value), `spec/17-codegen.md:80` (D5/O2: temporary release at statement end),
`spec/03-ownership.md` §3–4 (copies, moves, partial moves), `spec/16-borrow-rules.md`
(BR-1: a loan never outlives its owner; `E10030`), `spec/17-codegen.md:344` (the
panic behavior), `spec/decisions.md` (D4, E4, T5), and the diagnostics registry. No
specification, grammar, or registry change is authorized (AR-12).

## Functional Requirements

### Must Have

- [x] **R1 — A call-rooted element-loop iterable compiles and runs (`DEF-24`).**
  `for (x in mk())` and `for (borrow e in mk())` materialize the call result once before
  the first iteration; reads, writes, and copies through the loop behave as for a place
  iterable, and the materialized value is released after the loop. (`spec/05` §9; AR-6)
  A call-rooted iterable whose type mentions a generic parameter keeps today's behavior
  (the `mentions_type_param` guard; AR-6, Won't Have).
- [x] **R2 — Owning compiler temporaries are released at statement end (`DEF-23`).**
  A materialized temporary that owns a value is dropped at the end of the statement in
  reverse creation order; an unevaluated guard path leaves its temporary empty and its
  drop is a no-op. The release is emitted by a statement-position path (not the
  scope-collected `Drop`): it drains after each statement, and every edge that leaves the
  statement before its end (a `return`, `break`, or `continue`) carries the pending
  releases, so every runtime path releases exactly what it created; the drops are
  null-safe, so an overlapping release is harmless. A temporary that is re-materialized
  inside a loop releases its previous value before the new one is stored. The release
  covers the destination materializations only: the guard-operand temporaries, the
  iterable temporary, and the conditional read-position temporary. Take-call result
  temporaries and aggregate-literal adoption temporaries are transfer temps whose
  destination already owns the value and are excluded. A view derived from such a
  temporary — directly or through a call, conditional, or return — is a loan of that
  temporary; a use with no release-free path reports the existing `E10030`.
  (`spec/17-codegen.md:80`, D5/O2; `spec/16` BR-1; AR-5, AR-18)
- [x] **R3 — Fixed-array conditional values compile in the routed value positions
  (`DEF-25`).**
  `let c: i32[2] = ok ? a : b;`, `c = ok ? a : b;`, and `return ok ? a : b;` (through the
  fixed-array return emitter) store a fixed-array conditional through a branch that
  evaluates only the chosen arm: a byte copy for a duplicable arm, a move for an owned
  arm. A conditional argument whose arms are places stays inline as a forwarding value,
  so a borrow parameter reads the caller's chosen storage and a take parameter consumes
  the chosen arm through the parameter pointer as today; the checker's place-projection
  predicate is extended so no owning copy is bound. Consuming a conditional's owned arm
  — an initializer, an assignment, a return, a take argument, or a materialized read —
  leaves either owned place arm maybe-moved, so a later use of it reports the moved-value
  error `E10020`. `s = ok ? s : t;` keeps `s`'s old value when the condition holds, takes
  `t`'s value otherwise, and releases the old target exactly once; `s` stays initialized
  because it is the assignment target. (`spec/03` §3–4; AR-7, AR-16, AR-19)
- [x] **R4 — A runtime-index callable-field receiver is bounds-guarded (`DEF-26`).**
  `ops[i].run(1)` traps with `E11211` before the call when `i` is out of range, on the
  expression-statement, fixed-array-assign, and fixed-array-return paths, exactly like an
  ordinary runtime index. (AR-8)
- [x] **R5 — A `func` value with a fixed-array return carries the hidden out-param ABI
  (`DEF-27`).** Struct fields, locals, parameters, and discarded calls through those
  render `void (*)(params..., <destination>)`, and a call through a func value passes its
  destination (or a released discard temporary). A discarded call through a local
  func-typed callee resolves the callee's return type from the local's declared type.
  (`spec/17-codegen.md`, the by-value aggregate return rule; AR-9)
- [x] **R6 — A `str`, `String`, or literal panic message compiles (`DEF-28`).**
  `panic(s)` with any of the three forms prints `panic: <message>` and aborts; the
  runtime takes pointer and length, so no NUL terminator is required. (`spec/17` panic
  row, D4/E4; AR-10, AR-17)
- [x] **R7 — A `take` argument in a guarded operand moves only when evaluated (`DEF-29`).**
  With `ok && sink(s)` and `ok == false`, the call and the move do not run and `s` stays
  owned; with `ok == true` the call runs and `s` moves exactly once. The same holds for
  both `?:` arms and for `||`. (AR-11)

### Should Have

- [x] **R8 — Guard behavior is unchanged.** Existing materialization shapes (index,
  division, float-to-int, slice operands), guard traps, laziness of unevaluated
  operands, `E10040` for a non-duplicable element loop without `borrow`, field-over-
  function precedence, scalar func values, and the null/ownership diagnostics keep their
  behavior. (AR-12)

### Won't Have (Out of Scope)

- Any change to `spec/`, the grammar, or the diagnostics registry (AR-12).
- A slice-typed call-rooted iterable: a slice call result is a view over storage the
  callee does not own, so it stays inline as today (only fixed-array and value call
  results are materialized; AR-6).
- A call-rooted iterable whose type mentions a generic parameter: it keeps today's
  behavior under the existing `mentions_type_param` guard (AR-6).
- A fixed-array return through a closure struct or a lifted lambda: those signatures are
  rendered inline, not by the shared func-pointer declarator; the accepted-but-invalid
  form is tracked as `DEF-30` (AR-9).
- An indexed fixed-array element target store (`rows[i] = value;`, including an owned
  literal assembled from variables) and a nested-literal row store: today's `StorePointer`
  path is a separate, broader defect tracked as `DEF-31` (AR-16).
- A conditional fixed-array argument whose arm includes a call result: the decayed call
  names no destination, so the accepted form is tracked as `DEF-32` (AR-16).
- A diagnostic for non-string panic message types such as `panic(42)` (AR-17).
- Any further defect filed after `DEF-32` and the backlog programs `PLAN-05`/`PLAN-13`
  (AR-1).

## Technical Requirements

### Performance

No runtime target: materialization, guards, and type spelling are compile-time decisions.
A call-rooted iterable adds one materialized value per loop; a conditional array adds a
branch instead of a ternary copy.

### Compatibility

All three profiles (`host`, `hosted-amiga`, `freestanding`) and both C toolchains keep
working. The `cl_panic` signature changes in all three shims together with the emitted
call; the panic behavior is unchanged (`spec/17-codegen.md:344`). `make check` and
`make verify` cover the corpus and legs.

### Security

Not applicable: compiler-internal acceptance and emission rules with no user input,
secrets, persistent data, or network surface (AR-13).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defects in this plan | DEF-23 – DEF-29 / include PLAN-13 / include PLAN-05 + PLAN-13 | The seven filed defects | One ownership/codegen program; separate programs stay separate | AR-1 |
| Plan structure | one cumulative plan / program of three plans / one plan per defect | One cumulative plan | Matches the array-value-integrity precedent; one preflight and one execution thread | AR-2 |
| Phase order | cluster order / strict severity / conversions first | Materialization → callable fields → runtime boundary/deferral | Groups shared fix surfaces; the critical DEF-24 leads its cluster | AR-3 |
| Temporary release scope | all owning temps / arrays only | All owning temporaries, statement end, reverse order | The spec rule (D5/O2) is explicit; empty-state temps make an unevaluated drop safe | AR-5 |
| Temporary release mechanism and set | scope-collected `Drop` / dedicated statement-position release | Dedicated statement-position release; destination materializations only | `Statement::Drop` is scope-collected and would fire at scope end; transfer temps are already owned by their destination | AR-18 |
| Temporary view lifetime | release with no view rule / temporary loan provenance + `E10030` | Statement-scoped temporary loan provenance | A derived view would otherwise dangle; the existing loan rule reports the later use without a new diagnostic | AR-18 |
| Call-rooted iterable | accept + materialize / reject | Accept and materialize once | `spec/05` §9 defines the loop over an indexable value | AR-6 |
| Conditional arrays | duplicable + owned / duplicable only | Both, chosen-arm branch with moves for owned arms | The accepted `s = ok ? s : t` must stay sound | AR-7 |
| Conditional coverage | all value positions / stores only | Routed store positions, the return emitter, and a materialized conditional argument | One branch-aware store emitter plus the return emitter covers the accepted forms; the indexed element target is filed separately | AR-16 |
| Receiver guard | guard on all call paths / reject | Guard on all three paths | Bounds traps are the established runtime rule | AR-8 |
| Func-pointer ABI scope | all func spellings / fields only | The filed spellings (fields, locals, parameters, discarded calls) plus call destinations | The helper is shared by those spellings; closure/lambda signatures are separate and filed | AR-9 |
| Panic conversion | runtime ptr + length / NUL-terminate | Runtime takes pointer + length | No NUL assumption and no allocation on the abort path | AR-10 |
| Panic message types | str/String/literal / also restrict others | The three string forms | The registry owns no diagnostic for other types | AR-17 |
| Deferred move-out | extend materialization / branch restructuring | Extend the materialized value | Local to lowering and the guard emitter | AR-11 |
| Oracle | change spec / keep frozen | Keep frozen | The specification is the immutable oracle | AR-12 |
| Verification | `make check` + `make verify` / other | `make check` per task, `make verify` at close | Project standard from `AGENTS.md` | AR-14 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #)
> that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every requirement R1–R7 traces to at least one passing specification test case in
   `runtime_value_integrity_spec_test.rs`; R8 is guarded by the existing suites
   (`array_value_integrity`, `element_loop_integrity`, `codegen_run_spec_test`).
2. [x] `make check` passes with no warnings; `make verify` reports all legs passed.
3. [x] The host probes run under AddressSanitizer with leak detection on for the owning
   temporary cases (ST-3, ST-4, ST-7, ST-24, ST-26, ST-29) and report clean.
4. [x] `DEF-23` – `DEF-29` are marked Done in `plans/00-roadmap.md` with a link to this
   plan; `DEF-30` – `DEF-45` remain filed as backlog defects.
5. [x] Documentation updated: `AGENTS.md` test map, roadmap rows.
