# Requirements: Conditional, Projection, and Update Residue II

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Complete the 27 open follow-on rows batch I filed and did not close: one critical, five majors,
and 21 minors, all in the batch-I machinery. The current status of every row whose note names a
follow-up was re-verified by probing the actual compiler (AR-3): the landed parts are green
controls, the residuals are red. Each implemented row is a spelling, position, or judgment the
batch-I fixes did not reach, with a reproduction and base/head evidence in the roadmap. Every
requirement below is conformance to the frozen specification: the fix makes an accepted program
build and run with the language's semantics, stops a false rejection, or makes a report match the
pinned one-report boundary. No new language surface, no new diagnostic code, no specification or
registry edit (AR-2).

## Functional Requirements

### Must Have

- [x] **R1 — View owners, loans, and stores** (`DEF-281`, `DEF-285`, `DEF-286`, `DEF-287`,
  `DEF-288`, `DEF-290`, `DEF-291`): a duplicable projection over a residual-owning fresh root
  binds the root for its residual release in the take, return, and read positions (the selected
  value stays a copy); a runtime-index element store of a view routes through the constant
  index's copy/owner/loan path; a cross-frame out-slot pass-through is judged at the call site
  so a view whose backing dies before the slot is read is rejected (one report); a module-global
  view initialized from a fresh call releases its owner at module initialization; a
  view-carrying container escape consults the binding's element loans exactly as the carrier
  path does; the literal-carrier loan is dropped when the field or the whole carrier is
  overwritten; an all-null composite view store is accepted (an empty storage-atom set names no
  storage).
- [x] **R2 — Guarded, discarded, and carrier roots** (`DEF-284`, `DEF-292`, `DEF-293`, `DEF-297`):
  a guarded argument whose view element wraps a fresh call root binds the call root for its
  residual release and runs no call on the skipped path; a conditional carrier literal's taken
  arm binds its fresh element root for the local's frame; a discarded arm's release need is
  judged by the root's residual ownership, not only the selected type; the bare discarded
  all-fresh literal statement materializes and releases its fields.
- [x] **R3 — Fixed-array and literal emissions** (`DEF-298`, `DEF-299`, `DEF-300`, `DEF-302`,
  `DEF-303`, `DEF-304`, `DEF-305`, `DEF-306`): a fixed-array field initialized from a
  conditional value and a conditional root projected into a literal field route through the
  per-arm destination store; a guarded `len` over a literal view-array field emits the
  array-elements initializer; a conditional of call-filled fixed-array-field literals keeps its
  call arguments; a `String[2]` conditional-root projection returned by value binds each arm's
  root (the critical: no segfault); the guarded/deferred expression-path fallback routes
  constructors through the per-arm destination store; `fixed_array_field_needs_stores` descends
  array-literal elements; the nested conditional-arm projection distributes at the element-read
  base.
- [x] **R4 — Loop conditions, update order, and store lowering** (`DEF-294`, `DEF-295`, `DEF-308`,
  `DEF-309`, `DEF-310`): a loop-condition receiver with mixed arms pins its receiver position
  (the strict build passes on both toolchains, or the row closes as a control with the
  re-measured evidence); a loop-condition member projection of a fresh root binds the root once
  and reads the member from it (no double-free); a guarded `&&`/`||` operand and a `?:` arm pin
  their updates inside the guard path (the per-iteration `Materialize` fill precedent); the
  array fill expression is evaluated once, before the element loop; a bare raw-deref store
  target lowers and the write lands.
- [x] **R5 — Diagnostics and marks** (`DEF-282`, `DEF-283`, `DEF-307`): a nested fresh
  conditional at a mixed view return reports once at its span, and a downstream consequence
  report is withheld where the callee-side judgment already reports (the one-report boundary);
  a deferred mixed view return renders the pinned `a compiler-generated owner` phrase; the
  whole-array conditional iterable marks no arm as moved, so a later use stays accepted.

### Won't Have (Out of Scope)

- `DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — the six older rows parked
  for user rulings (AR-2): a non-callable-callee diagnostic code, an escaping type-key scheme,
  staged `embed`, a nameless nullable source diagnostic, the `unsafe` exemption, and the folded
  fixed-array `.len` receiver evaluation.
- `DEF-289`, `DEF-296`, `DEF-301` (fixed in batch-I follow-ups) and `DEF-278` (completed):
  verified at the planning head (AR-3) and out of scope; their shapes stay as regression
  controls only.
- Any new diagnostic code, registry row, or specification text (reserved; every fix reuses the
  frozen registry).
- A whole-program lifetime analysis for `DEF-286` (the bounded call-site rule, AR-6) and a new
  pinning pass for `DEF-308` (trigger extensions of the existing pin, AR-4/AR-14).
- The batch E/F/G residues (sequenced after this batch; AR-1).

## Technical Requirements

### Compatibility

- The host, hosted-amiga, and freestanding profiles stay conformant; every accepted program keeps
  building under `-std=c99 -Wall -Wextra -Werror` on the pinned container, and the `DEF-294`
  shape also on the host toolchain (AR-10).
- Byte identity wherever a phase does not change the shape (the emission differential classifies
  every delta).

### Security

- Not applicable beyond the compiler's own input validation (unchanged).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | batch-I follow-ons (27) / batch E residue (2 majors) / batch F+G residues | batch-I follow-ons | The only open critical, the most majors, one machinery, fresh evidence with fix directions | AR-1 |
| Ruling-gated rows | include and pause / park | park | Reserved decisions are not touched; none of the 27 needs a spec or registry edit | AR-2 |
| Phase grouping | five phases by machinery / one per row / ownership vs emission | five by machinery | The rows group by the hooks they extend; the dependency order is P1 → P2 → P3 → P4 → P5 | AR-4 |
| `DEF-286` | call-site rule / recorded limit | call-site rule, stop-gated | BR-8 conformance; a recorded limit changes the accepted surface and is not auto-approved | AR-6 |
| One-report boundary | one per span / keep both | one per span | The batch-I AR-26 precedent; the semantics lens reviews the cross-site boundary | AR-7 |
| `DEF-294` | host red + dual pin / control | host red + dual pin, re-measured | The fix direction makes the emission explicit; the phase decides per the dual-toolchain evidence | AR-10 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.
> See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every one of the 27 rows is closed with a reproduction that flips (red before, green
   after) and a suite pin that fails without the fix; the landed partial fixes (`DEF-306`'s
   store/return/iterable distribution, `DEF-308`'s plain loop-condition sibling) stay green as
   pinned controls (AR-3, AR-5).
2. [x] Every ownership fix is ASan-clean on the host (plain and `RDC_ASAN=1`), with loop and
   early-exit variants where the shape is re-executed.
3. [x] The strict build passes for every accepted shape in scope on the pinned container; the
   `DEF-294` shape is measured on both toolchains and the outcome recorded (AR-10).
4. [x] No behavior change outside the in-scope rows; the batch-I residue suites, the batch-H
   suites, and the `callable_release_integrity` oracle stay green.
5. [x] `make check` green on every phase; `make verify` green at close-out.
6. [x] The roadmap's 27 rows are accounted for (Done with per-row evidence); the six older
   parked rows stay open with their rulings listed (AR-2); the batch-I rows already closed stay
   untouched.
7. [x] No spec, registry, or golden changes; the new cases only append to the batch-I suite
   pair (AR-12); the `AGENTS.md` test-map note updates at close-out only if its wording needs
   the batch mention.
