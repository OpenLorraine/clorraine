# Ergonomics findings - pre-Phase-B review

Status: rulings accepted (all F1-F17 as recommended). Evidence lives in
[`ergonomics-corpus.md`](ergonomics-corpus.md); decisions are traced to
[`../spec/decisions.md`](../spec/decisions.md), where the amendments are applied.

## Verdict

1. **The hard fights are few and known.** No identity reversal is needed for
   the garbage collector question, the ownership model, exceptions, tagged
   unions, or transpilation to C. The three things that make Rust hard in
   daily use - lifetime annotations, borrows held across statements, trait
   bounds - do not exist here.
2. **Three semantics must be frozen now**: the element loop, bounds checking,
   and reborrowing views. They gate the grammar and the standard library.
3. **The safety claim needs honest wording.** Memory safety holds for
   application logic. The hardware boundary (MMIO, interrupts, custom
   recursive structures) is audited `unsafe` in v1, and that share is not
   small on the Amiga. The standard library must shrink it, and the
   specification should say so plainly.
4. **The convenience layer is the biggest perceived gap**: dot-call syntax,
   doc comments, character literals, and hex formatting helpers.
5. **Error messages with fix hints are the top anti-friction investment.**
   With no ecosystem, first-contact friction equals diagnostics quality.

Method: five personas (C#/TS, Pascal, Go, Amiga C, Python) judged six
hand-written workloads. Severity: **Blocker**, **High**, **Medium**, **Low**.
A separate adversarial review challenged all recommendations; its material
points are marked **Challenge** below.

## Findings

### F1 - Element loop binding (copy or write-through view)

**Severity: Blocker (grammar freeze).** Evidence E4-F2: `moveAll` mutates
`e.x`, while `totalWidth` only reads. Today the binding mode is undecided.

**Challenge:** Go `range` and C# `foreach` bind copies. A write-through view
under familiar syntax silently mutates the caller's data in those languages'
eyes. The corpus sketch never tested the loan extent: a writable element view
would loan the whole slice (O8, O11), so `len(xs)`, `xs[0]`, or passing `xs`
inside the body would conflict.

**Options:** (A) `for (e in xs)` binds a copy of duplicable elements, and
`for (borrow e in xs)` binds an exclusive element view for write-through,
with per-element loan extent; non-duplicable elements require the `borrow`
form. (B) write-through by default. (C) copy only; mutation via index loops.

**Recommendation: A.** It matches C#/Go habits, reuses the V6 marker
philosophy (explicit when it matters), and keeps mutation possible without
whole-slice conflicts.

### F2 - Bounds checking

**Severity: High (semantics and codegen).** Evidence E4-F3.

**Challenge:** a build-wide opt-out removes safety everywhere, duplicates the
existing raw/`unsafe` path (T6), sits badly with the no-undefined-behavior
guarantee, and a bounds panic inside an interrupt handler aborts the machine.

**Options:** (A) `T[]`, `str`, and array indexing are always checked; the
only unchecked path is explicit raw/`unsafe` access. (B) a profile or flag
disables checks globally.

**Recommendation: A.** Safety is not a profile. Interrupt code uses
`unsafe` accessors or `unchecked` helpers, which are greppable and audited.

### F3 - Owning byte buffer and dynamic array

**Severity: High (functional gap).** Evidence E2-F5, E6-F6. The decisions
define `String`/`str`/`cstring` (T2), but not an owning byte buffer. Bitmaps,
samples, and file loads need one; without it, bulk IO requires `unsafe`.

**Options:** (A) standard library ships `Buffer` (owned bytes) and `Array<T>`
(growable) in v1; (B) new language builtins.

**Recommendation: A.** The `String` allocator path (D1, T2) is reusable, and
stdlib keeps the grammar small. Decide the accessor shape together with F1
and F11.

### F4 - Recursive structures and `Box<T>`

**Severity: High (credibility gap).** Evidence E6-F2/F4.

**Challenge:** `Box<T>` cannot be built today at all: `freedBy` must name an
FFI-declared function (X1) and application code may not write `ffi` blocks
(X6), so a user `Node` can never get a deleter.

**Options:** (A) v1: stdlib `Array<T>` covers most workloads; custom
recursive types are documented raw/`unsafe` patterns. v1.1: a built-in
`Box<T>` with compiler-generated drop glue (the monomorphization pass can
emit recursive drop functions), which also requires allowing ordinary
Clorraine functions as `freedBy` deleters. (B) build `Box<T>` into v1.
(C) accept the limitation permanently.

**Recommendation: A.** The compiler MVP should not carry generic drop glue;
v1.1 is the right moment, and the X1 amendment is part of that design.

### F5 - MMIO doctrine and the size of the unsafe surface

**Severity: High.** Evidence E1-F1 through E1-F4, cross-sketch summary.

**Challenge:** claiming "safe by default" while MMIO, interrupts, and
recursive structures are all `unsafe` is hollow. The doctrine question must
be answered, not deferred: should integer-address volatile intrinsics
(`writeVolatile16(base + off, v)`) require `unsafe`?

**Options:** (A) keep `unsafe` for every address-based access; allow
module-level `const raw Regs = <address>` constants so each function does not
re-derive its base; ship per-chip safe wrapper modules in stdlib; document
the wrapper-module pattern for unshipped chips. (B) make integer-address
intrinsics safe (destroys the meaning of the marker). (C) compiler-maintained
register-base whitelist (magic, rejected).

**Recommendation: A, plus honest wording in the introduction**: safety is
strong for application logic, audited at the hardware boundary.

### F6 - Volatile globals for interrupt-shared state

**Severity: High for the target.** Evidence E5-F1. `volatile` exists for
fields and intrinsics only (T7). A global counter written by an interrupt
handler and read by a polling loop is the classic stale-read bug.

**Options:** (A) allow a `volatile` qualifier on module-level globals;
(B) require intrinsics and address-of inside `unsafe`; (C) no support.

**Recommendation: A.** It is one qualifier on an existing declaration form
and removes a whole bug class.

### F7 - Interrupt and shared-library calling conventions

**Severity: Blocker for interrupt work.** Evidence E5-F2, E5-F5. A Clorraine
function cannot carry the m68k `__interrupt`/`__saveds` qualifiers, so it
cannot serve as a handler directly. Attributes are deferred (not in ledger).

**Options:** (A) standard library dispatcher: a C-side handler with the right
qualifier dispatches through a token-managed table of Clorraine functions;
the same pattern covers hooks and shared-library entries, documented
separately. (B) add calling-convention qualifiers to the language in v1.

**Recommendation: A.** Keeps the grammar clean and pushes the ABI knowledge
into one audited place. Token add/remove reuses the Phase 0 acquire/drop
pattern; handler functions are documented as allocation-free, drop-free,
panic-free.

### F8 - Nullable parameters

**Severity: Medium.** Evidence E5-F6, E2. T8 bans nullable parameters, which
forces status structs or sentinel values for optional handles.

**Challenge:** the modes must be spelled out, because "by value" is
ambiguous for move-only handles - a copy would create two owners.

**Options:** (A) allow `T?` in parameter position with the existing
conventions: unmarked or `const` = read-only lend, `borrow` = writable lend,
`take` = ownership transfer. (B) keep the ban.

**Recommendation: A.** No new semantics; it reuses V5 and makes optional
APIs expressible. This amends T8 for parameters only.

### F9 - Empty state for handles and out-parameter APIs

**Severity: Medium.** Evidence E2 (acquire patterns), E5-F6.

**Challenge:** the earlier idea `let f = File {};` is a double-free foot-gun:
it fabricates an opaque handle bit-identical to a live one and adds a second
spelling for empty beside `null` (O7, T8).

**Options:** (A) `null` is the only empty state; out-parameter acquisition
uses `borrow T?` (`let f: File? = null; openInto(path, borrow f);`).
(B) zero literals for handles. (C) status structs only.

**Recommendation: A.** One empty state, existing conventions, no fabricated
handles.

### F10 - One standard error carrier

**Severity: Medium.** Evidence E2-F2/F6, E5-F6. Today there are two models
(`T?` without detail, plus status structs) and no standard carrier, so
not-found, out-of-memory, and read errors are indistinguishable through
`T?`, and partial-IO return conventions are undefined.

**Options:** (A) standard library ships one status/error struct with a code
and an `ok` flag plus conventions for partial reads and writes; `lastError()`
stays FFI-only. (B) a language-level `Result` type (needs tagged unions;
rejected for v1).

**Recommendation: A.** One documented carrier, frozen in the chapters that
describe IO.

### F11 - Reborrowing views from borrows

**Severity: High (grammar freeze).** Evidence E2-F4. A writable view derived
from a borrowed handle or buffer (`borrow bufferBytes(buf)` where `buf` is a
parameter) is not covered by the decisions. Without a rule, the `Buffer`
write API and container slices are unusable in any function that received
the buffer as a parameter.

**Options:** (A) freeze explicit reborrow rules: a read-only view of a
borrow is a shared reborrow; a writable view of a writable borrow is an
exclusive reborrow limited to the expression. (B) ban it and require raw.

**Recommendation: A.** This is the same model Rust uses, but only at call
sites, so no annotations are needed. Must be frozen in Phase B.

### F12 - Dot-call syntax (methods by another name)

**Severity: Medium (the largest feel gap).** Evidence E3-F5. The easy
languages call operations on values (`text.len()`, `list.push(x)`); Clorraine
calls free functions (`len(text)`, `arrayPush(list, x)`).

**Challenge:** deferring this forces stdlib call sites into free-function
form that a later method syntax would have to rewrite (R1 forbids reopening
the front of the chain).

**Options:** (A) freeze dot-call syntax now and implement it as uniform
call sugar: `x.f(y)` resolves to `f(x, y)` with static resolution (field
first, then functions). No receiver semantics, no vtables, no `self` magic.
(B) keep free functions only; freeze the syntax with staged semantics
(`E90xxx`) like lambdas (L1). (C) full methods with receivers in v1.

**Recommendation: A**, with **B as the fallback** if the compiler MVP must
stay minimal. C is rejected for scope. Either way the standard library is
written in dot-call style from day one.

### F13 - Documentation comments

**Severity: Low-Medium.** Evidence: the challenge pass. The standard library
cannot be discoverable for P1/P6 without API documentation, and B9 currently
rejects doc comments.

**Options:** (A) freeze `///` as a documentation-comment form now; v1 emits
nothing, a future tool reads it. (B) keep C comments only.

**Recommendation: A.** `///` already lexes as a comment, so the change is a
lexer rule and a style rule, not a feature. No breaking change later.

### F14 - Character literals

**Severity: Low.** Evidence E3-F2. The type set has no `char`, and `'0'`
typing is undefined, yet text code needs it.

**Options:** (A) `'x'` is a byte literal of type `u8`; (B) introduce a
distinct `char` type.

**Recommendation: A.** The string model is byte-oriented (T2); a separate
`char` would invite encoding ambiguity.

### F15 - View fields in structs

**Severity: Medium (parser workloads).** Evidence: the challenge pass; T4/T9
ban view fields except `static str`, so a token struct cannot hold `str`.

**Challenge proposal:** allow one elided lifetime tied to the owning buffer.

**Options:** (A) keep the ban; parsers use offset pairs (`start`, `len`) and
a stdlib `Span`; (B) allow view fields with elided lifetime tracking.

**Recommendation: A.** Option B needs region tracking inside structs, which
is exactly the Rust fight the language exists to avoid. Offsets are the
documented pattern, and `Span` makes them ergonomic.

### F16 - Uninitialized locals

**Severity: Medium (correctness rule).** Blend65 defines uninitialized
variables as an unspecified value; C makes an uninitialized read undefined
behavior, so the no-UB promise requires a Clorraine rule.

**Options:** (A) definite-assignment analysis: reading before assignment on
some path is a compile error; (B) require an initializer on every local;
(C) zero-initialize all locals.

**Recommendation: A.** Flow-sensitive analysis already exists for moves (O6),
it costs no runtime cycles, and it matches C# habits. `borrow T?`
out-parameters are exempt.

### F17 - Floating point

**Severity: Medium (reach).** Evidence: the C delegation discussion. The
expensive part (soft-float runtime) is delegated to C; the surface rules are
not.

**Options:** (A) defer to v1.1; (B) add `f32`/`f64` in v1 for arithmetic,
comparison, casts, and formatting, with math functions in stdlib
(recommended); (C) add `f32` only.

**Recommendation: B**, with `1.5` typed `f64`, explicit int to float
conversions, out-of-range float-to-int conversion as a runtime fault, IEEE
division-by-zero semantics, and a new SDK verification leg for the GCC and
vbcc soft-float ABIs.

## Identity review (reopened per your approval)

| Identity item | Verdict | Reason |
|---|---|---|
| No garbage collector | Keep | Determinism suits Amiga targets; no evidence of a blocking fight |
| Ownership, moves, borrows (A1) | Keep | The fights are teaching problems, not model problems (E4-F1) |
| C-like surface | Keep | Add dot-call sugar (F12); no other change |
| Transpile to C | Keep | No friction found; `#line` keeps diagnostics mapped |
| No exceptions; explicit errors | Keep | Verbosity is real but Go shows it is survivable; carrier improves with F10 |
| No tagged unions, no `match` | Keep for v1 | Status structs plus `switch` cover the corpus; revisit only with evidence |
| No threads, no async | Keep | FFI tasks plus the event loop match the platform |
| "Safe by default" wording | Qualify | Safety is strong above the hardware boundary; say so (F5) |

## Accepted friction (deliberate, documented)

| Friction | Why it stays |
|---|---|
| Unmarked aggregates are read-only lends; mutation needs `borrow` | The lending model is the performance and safety foundation; diagnostics fix the habit |
| No lifetime annotations; view fields banned; narrowing dance | The alternative is Rust-grade annotation pressure |
| No overloading, defaults, or named arguments | Naming discipline instead; the C surface stays predictable |
| No labels or `goto` | Guarded loops are rare and have workarounds |
| C switch fallthrough | C exactness; the fallthrough warning is the mitigation |
| No format specs in templates | Helper functions cover it; a spec syntax stays additive |
| `unsafe` at the hardware boundary | Honest and greppable; stdlib wrappers shrink the area over time |
| No wildcard imports | Explicit dependencies, cheaper parsing |

## Standard library and tooling backlog (priority order)

| Priority | Item | Related finding |
|---|---|---|
| 1 | `Buffer` (owned bytes) and `Array<T>` | F3, F4 |
| 2 | Dot-call style API surface and naming rules | F12 |
| 3 | Error carrier plus partial-IO conventions | F10 |
| 4 | Formatting helpers (`toHex32`, padding) | E3-F1 |
| 5 | Safe per-chip MMIO wrappers and `const raw` bases | F5 |
| 6 | Interrupt/hook dispatcher with tokens | F7 |
| 7 | `Span` helper for offset-based parsing | F15 |
| 8 | Fix-hint text for every diagnostic code, `--explain` | All |
| 9 | C-to-Clorraine Rosetta guide and wrapper cookbook | E6, cross-sketch |
| 10 | Built-in `Box<T>` with generated drop glue (v1.1) | F4 |
| 11 | `--doc` tool reading `///` comments | F13 |

## Decisions requested

| ID | Decision | Recommendation | Type |
|---|---|---|---|
| F1 | Element loop binding | Copy by default; `for (borrow e in xs)` write-through with per-element loans | Grammar freeze |
| F2 | Bounds checking | Always checked; unchecked only via raw/`unsafe` | Grammar freeze |
| F11 | Reborrow of views from borrows | Freeze shared/exclusive reborrow rules | Grammar freeze |
| F3 | Owning `Buffer` and `Array<T>` in v1 | Standard library, not language builtins | Stdlib scope |
| F4 | Recursive structures | v1 raw pattern plus containers; `Box<T>` in v1.1 | Stdlib scope |
| F7 | Interrupt ABI | Stdlib dispatcher plus tokens; document shared-library ABI | Stdlib scope |
| F10 | Error carrier | One stdlib status/error struct; `lastError` FFI-only | Stdlib scope |
| F5 | MMIO doctrine | Keep `unsafe`; add `const raw` bases; stdlib wrappers; qualify the safety wording | Ledger amendment |
| F6 | Volatile globals | Allow `volatile` on module globals | Ledger amendment |
| F8 | Nullable parameters | Allow `T?` params with existing three conventions | Ledger amendment |
| F9 | Handle empty state | `null` only; `borrow T?` out-parameters | Ledger amendment |
| F12 | Dot-call syntax | Freeze and implement as uniform call sugar (fallback: freeze syntax, stage semantics) | Ledger amendment |
| F13 | Doc comments | Freeze `///` syntax; tooling later | Ledger amendment |
| F14 | Character literals | `'x'` is `u8` | Ledger amendment |
| F15 | View fields | Keep the ban; offsets plus `Span` | No change |
| F16 | Uninitialized locals | Definite-assignment analysis; `borrow T?` out-params exempt | Ledger amendment |
| F17 | Floating point | `f32`/`f64` in v1; math functions stdlib; SDK float ABI verification leg | Ledger amendment |
| - | Safety wording in the introduction | Qualify: strong above the hardware boundary, audited below | Ledger amendment |
| - | Recursion | Unrestricted; Blend65 `E10180`/`E10181` not adopted | Ledger amendment |

## Hardening notes

* One independent challenger (separate agent session) read the decision
  ledger and the corpus and attacked all provisional recommendations. Its
  material points are folded in: loop copy surprise, loan extent, reborrow,
  the `Box<T>` impossibility, volatile globals, the `File {}` foot-gun, the
  bounds opt-out critique, the `?` collision with nullability, and
  discoverability.
* Points I rejected: view fields with elided lifetimes (imports the Rust
  fight, F15), compiler-maintained register whitelists (magic, F5), and
  profile-based safety opt-outs (safety is not a profile, F2).
* Confidence: high for F1, F2, F6, F8, F9, F14; medium for F4, F5, F7, F10,
  F11, F12, F13; the F12 fallback exists because it depends on MVP scope.
