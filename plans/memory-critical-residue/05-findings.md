# Findings

## Phase 1 — Carrier sources and callable expressions

**Rows**: `DEF-508`, `DEF-509`, `DEF-510`, `DEF-511`, `DEF-512`, `DEF-519`.
**Implementation**: `c435466`, `00a52bf`, `dffed6f`, `53c152d`, `7805956`, `b7aa4da`.
**Red cases**: `571660c`; pins `af09e8f`, extended in the correction rounds.
**Lenses**: correctness, semantics, memory-safety, emission-integrity.

### What changed

- `call_carrier_sources` returns every argument a returned carrier may read: each
  view-like lend parameter, each owned aggregate with handle-bearing storage, and each
  `take` argument. A candidate is kept only when the callee body's return flow mentions
  the parameter; the flow summary is backward (`7805956`), so a view carried through a
  local still reaches its parameter while an unused owned argument is not a source.
- A member or element chain rooted at a carrier call is judged through the call's sources.
- A struct or array literal in a frame-receiving position descends its view-carrying
  fields/elements, skipping only a field whose provenance already dies with the frame (so
  a parameter-backed arm does not hide a carrier call in the other arm).
- Returning a `take` parameter no longer treats the parameter's own binding as
  frame-dying; the caller-side store still judges the argument's positions. The exemption
  follows an identity relay and a conditional of take parameters.
- `callable_call_signature` resolves a lambda literal, a conditional, a cast, and an
  indexed callable from the expression's recorded type. `check_call_expr` checks a
  non-name callable callee's arguments and markers against that signature. `lower_call`
  lowers the callee as a value and attaches it as the receiver, so the emitter renders
  `((c ? first : second))(args)`.
- A view-returning lambda resolves every value-position parameter and capture its return
  reads (`call_view_sources`), so a two-parameter, multi-capture, or conditional return
  judges each source instead of trusting the first.
- A capture's views are judged by the captured binding's backing provenance, not its own
  slot: a local copy of a parameter/static view survives the frame, while a view into a
  local dies. A write-borrow capture of a local is a same-frame destination; a later
  escape of the captured local is conservatively frame-backed.
- A generic return that mentions a type parameter admits a `take` argument as its source.
  When the returned value is already a view, the local-owner judgment applies at the
  definition, so a callee-local view reports `E10035` exactly as the concrete twin does.
- The conditional-take temp type resolves a `null` arm and a mixed nullable arm from the
  other arm, and the null-shape check reads the resolved temporary type.

### Red/green evidence

| Row | Red at `6a67880` | Green at `7805956` |
|-----|------------------|--------------------|
| `DEF-508` | accepted; ASan heap-use-after-free | `E10030` once, no TU; static control clean |
| `DEF-509` | accepted; ASan heap-use-after-free | `E10030` once; static pair clean |
| `DEF-510` | inline lambda accepted+UAF; conditional call emitted invalid C | both report `E10030` once; local conditional control strict-builds and runs |
| `DEF-511` | accepted; ASan heap-use-after-free | `E10030` once; parameter-backed and local-destination controls clean |
| `DEF-512` | two-argument helper accepted+UAF; identity return falsely rejected | `E10030` once at each caller store; identity, relay, and selection controls clean |
| `DEF-519` | accepted; ASan heap-use-after-free | `E10030` once; static and parameter-arm controls correct |

Focused legs at `b7aa4da`: eval-emission spec 61/61 and impl 51/51, both green;
borrow-generic spec 41/41 and impl 19/19. The eval-emission spec runs under `RDC_ASAN=1`.

## Phase 2 — Generic conditional wrappers

**Rows**: `DEF-514`, `DEF-515`, `DEF-518`, `DEF-521`.
**Implementation**: `c435466`, `dffed6f`, `53c152d`.

- `conditional_take_needs_carrier` accepts a conditional with at least one owning place
  arm; an arm that owns nothing fills the shared carrier, and the owning arm empties its
  source in its own branch. The bare, `unsafe`, null-first, nested, and mixed-nullability
  spellings emit the same shape with one `String` carrier.
- `type_param_conditional_sources`, `unresolved_conditional_place_arms`, and the binding
  effect strip `unsafe`/cast wrappers before the conditional walk.
- The array-literal element path records the type-parameter conditional duty.

| Row | Red | Green |
|-----|-----|-------|
| `DEF-514` | double free | `ClString` carrier, per-arm `h.name = NULL`, post-call empty; clean under ASan |
| `DEF-515` | double free / invalid C | same emission as the bare twin; clean |
| `DEF-518` | accepted; double free | `E10034` once at the owning instance; duplicable control clean |
| `DEF-521` | accepted; double free | `E10034` once at the literal field, binding, `const`, and whole assignment; control clean |

## Phase 3 — Moved state

**Row**: `DEF-523`.
**Implementation**: `c435466`, `53c152d`.

`write_place` compares a recorded path against the written path, wildcard-aware: a
run-time element write clears every recorded element at that position, a strict prefix
survives a nested write, and a whole-field write still re-initializes. A nested write
below a moved field reports `E10020` at the write and keeps the prefix, so the later read
reports at its own site.

Red: generic and concrete twins accepted and SEGV. Green: two `E10020` reports in both
twins; the whole-field control is accepted, clean, and byte-identical. A run-time-index
write now clears the constant-index null narrowing and reports `E10010`, where the first
fix iteration accepted and SEGVed.

## Review

### Verdicts

| Lens | Verdict at `af09e8f` | Corrections | Bounded confirmation |
|------|----------------------|-------------|----------------------|
| memory-safety | NOT CONFIRMED (unsafe binding/const/assignment double-free; null-first take leak; nested take invalid C) | `dffed6f`, `53c152d` | **CONFIRMED** at `53c152d`: all filed and sibling shapes reject once or run ASan/LSan-clean |
| emission-integrity | no introduced finding | `dffed6f`, `53c152d` | **CONFIRMED** at `53c152d`: 64/64 corpus units byte-identical; 17/17 corrected accepted controls strict-build, run, and re-emit byte-identically; 1224/1224 extracted accepted controls strict-build |
| correctness | NOT CONFIRMED (RV-001 runtime-index moved state; RV-002 literal-arm carrier; RV-003 multi-view and conditional lambdas; RV-004 capture destination writes and returns; RV-005 projected-store flow; RV-007 marker; RV-009 relay) | `53c152d`, `7805956`, `b7aa4da` | All findings fixed and re-probed; the added spec/impl cases pin each one |
| semantics | NOT CONFIRMED (SR-001 generic view instance; SR-002 multi-source returns; SR-005 local-hop; SR-004 destination escape) | `7805956` | SR-003, SR-006, SR-007, SR-008 confirmed; SR-001/002/004/005 corrected in `7805956` and re-probed |

### Correction rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| R1 | `dffed6f` | Unsafe/cast binding effect; null-arm temp type and resolved null-shape; capture backing provenance |
| R2 | `53c152d` | Generic-return source gate; capture/multi-view lambda sources; wildcard element clearing; literal-arm frame skip; capture destination frame; marker checks; backward return-flow filter |
| R3 | `7805956` | Correct backward flow direction; generic view-return local-owner judgment; plural value-position lambda sources; written captured local marked frame-backed for a later escape |
| R4 | `b7aa4da` | A capture-written binding is flagged so a recorded initializer position cannot mask it and a returned take parameter is not exempt; a live projected assignment root makes its value's names live |

### Rulings

| Row | Ruling | Rationale |
|-----|--------|-----------|
| `DEF-523` | A nested write reports at the write and a later read reports at its own site (two `E10020`). | The frozen rule forbids every read of a moved value; the concrete twin reports the same two sites. Q-N1 now reads one diagnostic per use site. |
| Carrier source set | A parameter the callee body's return flow does not mention is not a carrier source. | Keeps an unused owned or lend argument accepted; a view carried through a local still reaches its parameter. |
| Lambda view return | Every value-position parameter and capture is a source; the destination judges the whole set. | A conditional or multi-capture return can select any of them; trusting the first source left a use-after-free. |
| Captured local destination | A write through a write-borrow capture marks the enclosing local as possibly frame-backed; a later escape rejects. | The enclosing checker cannot see the closure's written value. The conservative rejection is sound; the safe parameter-backed escape is filed as `DEF-526`. |
| Generic `T` return | A return whose value is already a view is judged at the definition, so a callee-local view reports `E10035`. | The instance cannot make a frame-local view outlive the frame; the concrete twin reports the same code. |

## Close-out

**Verification**: `make check` and `make verify` are green at `b7aa4da`. The verify gate
passed the frozen `spec/check.sh`, the full workspace suite, the hosted-amiga corpus, the
vbcc build-only leg, both FS-UAE output legs, and the six runtime-fault programs. One
earlier verify run hit a transient FS-UAE "No config file!" harness error; the leg passed
on rerun and the full gate then passed cleanly. Focused legs: eval-emission spec 61/61 (ASan), impl 51/51;
borrow-generic spec 41/41 (ASan), impl 19/19.

**Row accounting**: batch Q closes **11/11** named criticals — `DEF-508`..`DEF-512`,
`DEF-514`, `DEF-515`, `DEF-518`, `DEF-519`, `DEF-521`, `DEF-523`; the plan is 32/32
tasks. The roadmap now holds
**517 tracker rows: 369 Done, 147 open (1 critical, 45 major, 101 minor), 1 Deferred**
(`DEF-425`). Newly filed by the reviews: `DEF-525` (critical generic opaque-member view
store), `DEF-526` (minor capture-destination precision), `DEF-527` (major slice-of-func
typedef), `DEF-528` (minor conditional capturing-closure message).

**Code record**: plan and red cases `571660c`; implementation `c435466`; carrier filter
`00a52bf`; review corrections `dffed6f`, `53c152d`, `7805956`, `b7aa4da`; pins `af09e8f`
and the correction commits. The tree is clean apart from the untracked `.opencode/` tooling
catalog; no `spec/` oracle, registry, or generated C was edited.

**Residuals**: `DEF-525` (critical), `DEF-526`, `DEF-527`, `DEF-528`; the pre-existing
`E10213` staged-out `embed` fixture and the generic cast-in-conditional emission note
remain outside the tracker. None of the eleven rows is silently reopened or left partial.
