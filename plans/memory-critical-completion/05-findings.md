# Findings

## Phase 1 — Loan-carrying copies

**Reviewed implementation**: `e80c2c0` (copy destinations), corrections `91d2de0` (call-site
candidate substitution, wrapper gates, receiver backing, alias copies) and `617ce40` (carrier
return, projected carrier source, alias receiver slots). **Lenses**: correctness, semantics,
memory-safety, emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS for the six rows after corrections; residual precision rows filed | The six filed shapes reject once with no TU; the six controls run exit 0; RV-001/RV-004/RV-006/RV-007/RV-010 fixed; RV-008 and the safe mixed-aggregate twin are recorded as `DEF-506`/`DEF-507`. |
| semantics | PASS for the six rows | The const/global/call/shadow/alias copies share the source positions' loans; the direct twins report the same codes; the accepted controls are byte-identical. The static-return helper precision loss is `DEF-507`. |
| memory-safety | CONFIRMED for the six rows | The filed reproductions no longer UAF; the controls are ASan/LSan clean; the introduced `pick<T>(a: str, b: T)` regression was found and fixed in `91d2de0`; the safe mixed-aggregate twin is a filed conservative rejection (`DEF-506`). |
| emission-integrity | CONFIRMED after corrections | 28 accepted corpus units across both profiles are byte-identical and strict-build; the introduced generic-candidate regression and the qualified-receiver false rejection were corrected; the projected carrier member remains `DEF-508`. |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `25721e3`, `e80c2c0` | Red cases, copy destinations, call-carrier source, mixed aggregate, shadow snapshot, alias return, pins |
| C1 | `91d2de0` | Argument-type candidate substitution, wrapper copy gates, receiver backing, alias copy roots, carrier return supplement, const/let shadow snapshots |
| C2 | `617ce40` | Carrier-source projection through a call, global parameter-backed carrier arms, alias receiver slots, plain const shadow |
| C3 | `324fe76` | Composite-copy snapshots before the target record is cleared (self-conditional assignment, bare call/struct-literal shadow) |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (`e80c2c0`) | **NOT CONFIRMED**: `pick<T>(a: str, b: T)` at `T = i32` became accepted+UAF; the `const` shadow and `unsafe` wrapper lost loans; the safe mixed aggregate was falsely rejected; the global indexed arm and dot carrier receiver were open |
| B2 (`617ce40`, `324fe76`) | **CONFIRMED**: the generic candidate regression, parameter-backed global arm, alias/unsafe/const/let shadow copies, carrier returns, receiver backing, qualified return, func-field/global-callable/alias-slot/dot-carrier cases, and the self-copy snapshot are fixed; `DEF-506`, `DEF-507`, and `DEF-508` remain the recorded conservative/open rows |

### Filed rows

`DEF-506` (safe mixed aggregate false rejection, conservative safety side), `DEF-507` (static-return
helper false rejection), `DEF-508` (projected carrier-returning member), `DEF-509` (two-source
carrier call), `DEF-510` (inline lambda/conditional func callee), `DEF-511` (closure capture
out-slot).

## Phase 2 — Call-mediated view escapes

**Reviewed implementation**: `17025f0` (view-return call source, wrapper descent, dot receiver),
corrections `91d2de0` and `617ce40`. **Lenses**: correctness, semantics, memory-safety,
emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS for the five rows after corrections | The filed shapes reject once; the new controls (static/caller-backed receivers, closure/func-local/generic/returned-callable calls) are green; the red-gate overlap with phase 1 is recorded (488/489 were already fixed by the phase-1 carrier source). |
| semantics | PASS for the five rows; precision rows filed | The receiver is judged at parameter zero; the wrapper and callable forms resolve in the call-check order; the safe static-view receiver is accepted after `91d2de0`; `DEF-507` records the static-return helper precision loss. |
| memory-safety | CONFIRMED for the five rows | The filed UAFs reject; the controls are clean; the introduced receiver false rejection was corrected; `DEF-508`/`DEF-509`/`DEF-511` record the remaining pass-through classes. |
| emission-integrity | CONFIRMED after corrections | The accepted corpus is byte-identical and strict-build; the qualified-receiver false rejection and the func-typed-field source indexing were corrected; no accepted unit emits invalid C. |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `17025f0` | View-return call roots, wrapper descent, dot receiver, red/impl cases |
| C1 | `91d2de0` | Call-site argument types, func-typed-field indexing, receiver backing, module func-typed globals, carrier returns, alias slots |
| C2 | `617ce40` | Receiver roots for qualified returns, carrier-source receiver judgment, projected carrier source |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (`17025f0`) | **NOT CONFIRMED**: safe qualified receivers were falsely rejected; the func-typed field call mapped the receiver to argument zero; the qualified return and carrier receiver paths were missed; `DEF-507` recorded |
| B2 (`617ce40`, `324fe76`) | **CONFIRMED**: the receiver false rejection, qualified return, dot carrier receiver, func-field source index, module func-typed global, carrier return, and alias receiver slot are fixed; `DEF-507` and `DEF-508` remain the recorded rows |

## Phase 3 — Generic per-instance conditional sources

**Implementation**: `91d2de0` (take path duty, raw field strip, struct-literal/assignment duties,
nested projection), red tests `f060470`, corrections `617ce40`, `4110acc` (bare projections),
`5180cf0` (moved-path later-use judgment). **Lenses**: correctness, semantics, memory-safety,
emission-integrity.

The first review at `617ce40` was NOT CONFIRMED: the bare projection-over-conditional spelling was
still accepted at every consumed position (RV-002/SR-001), and the projection-aware later-read duty
judged the use path instead of the moved path, accepting whole-move use-after-free (F1/F2). The
corrections record every conflicting moved path up to the use, judge each after substitution, and
keep a per-moved-path dedupe so a duplicable duty cannot hide a later owning use. `4110acc` also
incidentally closed `DEF-516`, `DEF-517`, and `DEF-520`; the remaining view-member precision and
opaque-lowering gap is `DEF-522`.

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `91d2de0` | Take/raw/field/assignment/projection duties and red/impl pins |
| C1 | `4110acc` | Bare projection at take/return/binding/field/assignment; three-arm arms; projection-aware later-read duty |
| C2 | `5180cf0` | Moved-path judgment (empty path = whole move), per-moved-path dedupe, conservative view-member rejection |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (`4110acc`) | **NOT CONFIRMED**: whole-move + duplicable-member read was accepted and SIGSEGVed; a duplicable duty hid a later owning use; view-member projections were falsely rejected |
| B2 (`5180cf0`) | **CONFIRMED**: whole-move and prefix-move later reads report E10020 once; the duplicable direct read is clean; the bare projections report every arm; the view-member shapes reject without invalid C; the focused suites are green under ASan |

### Residuals

`DEF-514` (take-parameter conditional move-out), `DEF-515` (unsafe-wrapped take), `DEF-518` (array
element), `DEF-519` (returned struct-literal carrier), `DEF-521` (unsafe/cast-wrapped source), and
`DEF-522` (view-member projection precision/opaque lowering) remain open. `DEF-516`, `DEF-517`, and
`DEF-520` are closed incidentally by `4110acc`/`5180cf0`.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS after corrections | The four filed rows reject once; the bare projection forms reject twice (three-arm three times); the moved-path and duplicable-read controls are green; the view-member conservative rejection is `DEF-522`. |
| semantics | PASS after corrections | A whole move followed by a duplicable-member read reports E10020 once; a moved prefix judges the prefix; a duplicable leaf no longer hides a later owning use; a whole use after a member move keeps E10025. |
| memory-safety | CONFIRMED — no in-scope finding | The four filed rows reject once with no TU; the duplicable controls are clean; the wider sibling probes (whole/element take, three-arm nested conditional, `raw raw`, nullable field, alias assignment) reject without a new false rejection. Six pre-existing siblings are `DEF-514`..`DEF-519`. |
| emission-integrity | CONFIRMED | All accepted official and inline programs are byte-identical base-to-head; 12/12 host and 16/16 amiga corpus units strict-build; the only acceptance changes are the intended red cases and controls. Two pre-existing generic-wrapper gaps are `DEF-520`/`DEF-521`; the two-source carrier is `DEF-509`; the null-shape class is the existing `DEF-500`. |

## Rulings

The following precision trade-offs were decided during the batch, under the user's direction to
choose the best path forward. They are recorded so a later precision batch can revisit them without
reopening a closed critical.

| Row | Ruling | Rationale |
|-----|--------|-----------|
| `DEF-506` | A local mixed aggregate whose recorded view positions are all known-safe is still rejected at an out-slot when the type owns viewable storage. | The caller cannot see which field the callee views; accepting would leave the `name`-viewing twin's use-after-free open. The safe tag-only twin is filed for a viewed-position summary. |
| `DEF-507` | A single-lend helper's view return is assumed to derive from that parameter; a helper that returns static storage is conservatively rejected when its argument carries a frame loan. | The alternative (accept) leaves the `pick`-style use-after-free open. The static-return twin is filed for a return-derivation summary. |
| `DEF-508`..`DEF-513` | Filed, not fixed in this batch. | Each is a pre-existing sibling class outside the fifteen named rows; the batch's criticals were the filed reproductions, and the residuals keep their base evidence. |

## Close-out

**Verification**: `make verify` passed every leg at `324fe76` and again at `5180cf0` (frozen `spec/check.sh`, the full
workspace suite, the hosted-amiga corpus, the vbcc build-only leg, both FS-UAE output legs, and the
six runtime-fault programs). Focused legs at head: borrow-store spec 39/39 and impl 13/13,
eval-emission spec 53/53 and impl 47/47, borrow-generic spec 36/36 and impl 14/14, conditional
residue spec 224/224 and impl 224/224, lib 377/377. Accepted controls are ASan/LSan-clean; rejected
shapes emit no translation unit.

**Row accounting**: batch P closes **15/15** named criticals — `DEF-485`..`489`, `493`, `495`..`499`,
`501`..`504`. Three pre-existing siblings closed incidentally: `DEF-516`, `DEF-517`, `DEF-520`. The
roadmap now holds **511 tracker rows: 358 Done, 152 open (10 critical, 44 major, 98 minor), 1
Deferred** (`DEF-425`). The batch plan is **35/35 tasks**.

**Code record**: plan `432191f`; carrier red cases `25721e3`; phase-1 fix `e80c2c0`; phase-2 fix
`17025f0`; phase-3 red cases `f060470`; phase-3 fix and corrections `91d2de0`, `617ce40`,
`4110acc`, `5180cf0`; pins `28dbe05`, `725812e`. The working tree is clean apart from the untracked
`.opencode/` tooling catalog; no `spec/` oracle, registry, or generated C was edited.

**Residuals**: `DEF-506`/`507` (conservative false rejections), `DEF-508`..`512` (carrier/callee
views), `DEF-513` (inline-iterable alias return), `DEF-514`..`519` (conditional take/array/return
moves), `DEF-521` (unsafe/cast-wrapped generic source), `DEF-522` (view-member projection precision).
None is silently fixed or reopened.
