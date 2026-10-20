# Findings

## Phase 1 — Opaque member store

**Rows**: `DEF-525` (critical).
**Implementation**: `cca403d`, `37621a6`, `5fc503d`, `4581083`, `3e336e2`.
**Red cases**: `f251cdb`, extended in each correction round.
**Lenses**: correctness, semantics, memory-safety, emission-integrity.

### What changed

- A store into a projection the declaration sees through a type parameter records
  `ir::RequiresOpaqueStore`: the root binding's declared type, the projection path,
  the source type, every frame-dying backing, the owned source (when the instance
  decides between a move and a widening), the store and return sites, and the
  caller-slot flag.
- The instance audit substitutes the root, projects the path, and applies the
  concrete conversion. A view member accepts a widened or view source; a returned
  frame-backed view, a read after the backing's scope closes, a backing rewrite, a
  caller-slot escape, and an owned member's move/lend/partial-move rule all report
  the concrete twin's code at the concrete site.
- The escape duty follows direct, conditional, copied (`let`/`const`/assignment),
  call-relayed, dotted, constant-index, concrete-typed, and aggregate
  (struct/array literal) returns. An overwritten path supersedes only the view it
  replaced; a constant element keeps its sibling's view; two run-time stores do not
  replace each other.
- The emitter chooses a view store when the recorded target was not concrete and
  the instance target is view-like, widens an owned `String` through
  `cl_string_view`, leaves the source readable at a view instance, keeps the move
  and zero at an owned instance, and resolves a whole fixed-array move through the
  instance (`memcpy`). Opaque `.len` and index reads resolve through the instance
  (`(h.names[0]).ptr[0]`), and the unused-local analysis resolves the same shapes.
- Null at a non-nullable instance member reports `E10012`; a null at a nullable
  one stays accepted.

### Red/green evidence

| Row | Red (baseline `766e9b9`) | Green (head `3e336e2`) |
|-----|--------------------------|------------------------|
| `DEF-525` | accepted; ASan heap-use-after-free; the owned-source form emitted invalid C | `E10030` once at the return; caller-backed twin accepted and ASan/LSan-clean; owned member `E10200`; widened store `cl_string_view` and no move |
| Return variants | conditional/copy/relay/dotted/aggregate forms accepted and UAF | each `E10030` once |
| Backing liveness | nested-block, loop-break, source rewrite, read-then-reinit accepted and UAF/SEGV | `E10030`/`E10031`/`E10020` at the concrete site |
| Owned/widened stores | view member emitted `ClStr = ClString`; owned instance kept the move | view member widens; owned instance moves; `Wrap<T>` at `T=str`/`String` split correctly |
| Null and partial | null at a fixed array ran an OOB memcpy; `a[0].v` accepted | `E10012` / `E10026` |

### Review

| Lens | Verdict at `cca403d` | Corrections | Bounded confirmation |
|------|----------------------|-------------|----------------------|
| memory-safety | NOT CONFIRMED (non-place returns, backing liveness) | `37621a6`, `5fc503d`, `3e336e2` | **CONFIRMED** at `3e336e2`: no new leak, double free, use-after-free, or release-count defect on the reviewed diff |
| emission-integrity | NOT CONFIRMED (EI-001/EI-002 conditional/call returns and early backing death) | `37621a6`, `5fc503d`, `3e336e2` | **CONFIRMED** at `3e336e2`: corpus byte-identical; whole-array `memcpy`, opaque index `.ptr[0]`, and accepted controls strict-build/run |
| correctness | NOT CONFIRMED (copy routes, element identity, lambda binding indices, partial moves) | `5fc503d`, `4581083`, `3e336e2` | all reported criticals closed; the two-return-site count is filed as `DEF-530` |
| semantics | NOT CONFIRMED (return propagation, backing rewrite, source move/widening) | `37621a6`, `5fc503d`, `3e336e2` | all reported criticals closed; span/subject precision filed with the remaining minor rows |

### Rulings

| Rule | Ruling | Rationale |
|------|--------|-----------|
| Opaque view store | The instance decides the member kind: a view member accepts a widened or view source, an owned member keeps the concrete move/conversion rule. | The declaration only sees `T`; the concrete twin judges the substituted member. |
| Frame escape | A returned carrier, a read after the backing's scope, a caller-slot store, and a backing rewrite report the concrete escape/conflict code at the concrete site. | BR-1 and the last-use rule apply to the stored view exactly as they do to a concrete view store. |
| Owned source | An owned source waits for the instance: a view member widens it and leaves it readable; an owned member moves it (or reports the lend/partial-move rule). | A move is only correct when the destination owns; the instance decides. |
| Overwrite | A store at or above an earlier path replaces only the view that path definitely covers; a constant element and a run-time element are distinct. | A run-time element store may write another position, so it cannot erase a constant element's view. |

## Phase 2 — Type and closure surface

**Rows**: `DEF-527`, `DEF-528`.
**Implementation**: `ee35e0e` (red `fa79fda`).
**Lenses**: correctness, emission-integrity, semantics.

- `declarator`'s `Raw` arm now spells a pointer to a function pointer as
  `ret (**name)(params)`, built through `func_pointer_declarator`; the slice typedef
  uses the shared declarator instead of appending `*ptr` to `c_type`.
  `should_emit_a_slice_of_func_typedef`,
  `should_emit_a_pointer_to_function_pointer_slice`: the typedef is
  `i32 (**ptr)(i32)`, the slice strict-builds and calls its first element.
- A conditional of two distinct capturing closures reports `E10404` once at the
  conditional and returns `unknown`; a capture-less pair keeps the ordinary
  `func` conversion. `should_reject_a_conditional_of_capturing_closures_with_the_ephemeral_rule`
  checks one `E10404`, no `E10200`, and the accepted capture-less control.

## Phase 3 — Capture write summary

**Rows**: `DEF-526`.
**Implementation**: `1535df6`, `8155bbb` (red test in `1535df6`).
**Lenses**: correctness, memory-safety.

`mark_written_captures` now scans the lambda body for whole-name writes to the
write-borrow capture. A write is surviving when its value is static, a module
path, a capture backed by caller storage, or a conditional/cast wrapper of those;
a shadowing binding, a nested lambda that names the capture, a projection write,
or an unclassified value keeps the conservative frame-backed mark. A body with no
write leaves the local unchanged. `should_accept_a_parameter_backed_capture_write`
accepts the caller-backed case and runs ASan/LSan-clean; the batch-Q
`capturedestination` local-owner twin still reports `E10030`.

## Phase 4 — Generic dot call

**Rows**: `DEF-395`, `DEF-401`, `DEF-406`.
**Implementation**: `1eb54b2` (red `1ff66e1`).

`check_generic_dot_call` returns the substituted result type, so a `T`-returning
dot call emits the instance type (`ClString`, not `ClT`) and a generic caller
whose own `T` shares the callee's spelling keeps the callee's binding. Explicit
type arguments are resolved, counted (`E10300`), and seeded before the receiver
and argument checks. `should_bind_a_generic_dot_calls_instance`,
`should_emit_the_generic_dot_calls_substituted_result`.

## Phase 5 — Qualified call route

**Rows**: `DEF-396`, `DEF-407`.
**Implementation**: `fd5b1bc` (red `d7096f0`).

A qualified module function whose head is not a lexical local routes through
`check_generic_call` or `check_args_direct` with its written type arguments,
so arity (`E10308`), type-argument count (`E10300`), conversions (`E10200`),
and the substituted return type match the direct spelling. A fixed-array result
used as a loop iterable names the concrete element (`i32`, not
`#element of T`). `should_check_a_self_qualified_generic_call`,
`should_emit_the_concrete_element_of_a_qualified_generic_iterable`.

## Phase 6 — Initializer verification

**Rows**: `DEF-402`.
**Implementation**: `a782866` (verification only).

The self-qualified generic initializer emits `cl_id__i32(20)` and runs clean at
the batch head; no dedicated change was required.
`should_reach_a_self_qualified_generic_in_an_initializer`.

## New rows filed by the phase review

| Row | Severity | Finding |
|-----|----------|---------|
| `DEF-529` | major, pre-existing | A run-time-index view store consumes the owned source into a transfer temporary that is never released (a leak; the constant-index store is clean). |
| `DEF-530` | minor, pre-existing | Two definite carrier return sites report one `E10030` because the duty keeps one return span; the concrete twin reports each site. |


## Close-out

`make check` and `make verify` pass at the batch head. The roadmap records
`DEF-525`, `DEF-526`, `DEF-527`, `DEF-528`, `DEF-395`, `DEF-396`, `DEF-401`,
`DEF-406`, `DEF-407`, and `DEF-402` as Done; `DEF-529` (run-time-index view-store
leak) and `DEF-530` (one report for two return sites) are filed as follow-up rows.
No `spec/` file, diagnostic registry entry, or generated C was edited.
