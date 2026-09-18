# Clorraine - Semantic Lineage

> **Status:** draft (Phase B anchor)
> **Scope:** the normative semantic reference named by decision A1, pinned; what Clorraine inherits from Rust and what it deliberately changes
> **Decision ledger:** [decisions.md](decisions.md) — where this file and the ledger disagree, the ledger wins

## 1. Purpose

A1 makes **Rust the normative semantic reference** for ownership, moves,
borrows, drop scopes and drop order, and lifetime elision. This document pins
the exact sources, records the mapping, and lists every deliberate deviation.
With this pin, "what should this program do?" has an answer that does not
depend on memory or taste.

## 2. Pinned sources

| Source | Sections used | Used for |
|---|---|---|
| The Rust Reference, "Destructors" | Drop scopes, drop order | O2, D5 |
| The Rust Reference, "Lifetime elision" | Elision rules | D2, F2, T9 |
| The Rust Programming Language, ch. 4 | Ownership, moves, borrows, slices | O1-O11 |
| The Rust Programming Language, ch. 10 | Generics, monomorphization, lifetimes | G1-G8, D2 |
| RFC 2094, "Non-lexical lifetimes" (NLL) | Borrow regions, last use | O11 (target model) |

Pinned revision: stable-channel documentation, accessed **2026-09-17**:

* <https://doc.rust-lang.org/reference/destructors.html>
* <https://doc.rust-lang.org/reference/lifetime-elision.html>
* <https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html>
* <https://doc.rust-lang.org/book/ch10-00-generics.html>
* <https://rust-lang.github.io/rfcs/2094-nll.html>

Upgrading the pin is a ledger amendment, not an editorial change.

C is the **surface and codegen** ancestor, not the semantics reference: the
grammar derives from the Blend65 v4 grammar and the deltas are recorded in
[grammar-deltas.md](grammar-deltas.md); the emitted C subset is pinned in
[Ch 17](17-codegen.md).

## 3. What is inherited from Rust

| Clorraine rule | Rust source | Ledger |
|---|---|---|
| One owner per value; moving invalidates the source | Book ch. 4.1 | O1, O3 |
| Many readers XOR one writer; no move while borrowed | Book ch. 4.2 | O11 |
| Borrows never outlive the values they borrow | Book ch. 4.2 | O11 |
| Drop scope: locals in reverse declaration order at scope end | Reference "Destructors" | O2 |
| Struct fields drop in declaration order | Reference "Destructors" | O2 |
| Temporaries drop at the end of the statement (simplified) | Reference "Destructors" | D5 |
| No lifetime annotation syntax; elision decides | Reference "Lifetime elision" | D2 |
| Monomorphized generics (whole-program compilation) | Book ch. 10.1 | G3 |
| Callable parameters expressed as function types | Book ch. 10.3, adapted | G6, G7 |
| Borrow checking aims at last-use analysis (NLL) | RFC 2094 | O11 |

## 4. Lifetime elision, in full

D2 removes annotations, so elision is the whole lifetime system. Clorraine
uses these rules (derived from the Reference; there is no `self` case because
there are no methods):

| Rule | Statement |
|---|---|
| EL-1 | Every view-like parameter (view, handle, `raw`, `cstring`) carries its own elided lifetime |
| EL-2 | A function that returns a view type must derive it from **exactly one** view-like lend parameter (unmarked, `const`, or `borrow`) |
| EL-3 | Zero lending candidates, or two or more, with a view return is an error; annotations do not exist to break the tie |
| EL-4 | `static str` returns do not participate: static storage needs no input |
| EL-5 | A view parameter passed on to another call is a reborrow (O12), limited to the call expression |
| EL-6 | Nullable views add `null` as a value; the lifetime rule is unchanged |

Examples:

```clorraine
func head(borrow buf: u8[], n: usize): u8[] { return slice(buf, 0, n); }
// elides to buf: the single view-like lend

func pair(a: u8[], b: u8[]): u8[] { return a; }
// error (two candidates): no annotation syntax can choose

func greeting(): static str { return "hello"; }
// static storage: no candidate needed
```

## 5. Drop behavior: Rust compared

| Area | Rust | Clorraine | Ledger |
|---|---|---|---|
| Temporary scope | Temporary-scope table | Statement end | D5 |
| Drop flags | Hidden flags for possibly-moved values | Zero = empty; the zeroed value is the flag | O7 |
| Drop glue | Recursive user `Drop` impls | `freedBy` deleter per handle type; native structs drop structurally | D1, X1, X2 |
| Unwinding | Drops run while unwinding | `panic` aborts; nothing runs | D4, E4 |
| Statics at exit | Statics do not drop | Globals may not own (`own` globals banned) | O10 |
| Partial moves | Any field, supported by drop flags | Top-level fields of owned locals and `take` parameters only | O4, O5 |

`Box<T>` with generated drop glue is a v1.1 design item (X1, F4); until then
recursive structures use raw pointers and explicit release.

## 6. Borrow checking: target and v1

The target model is NLL (O11). **v1 implements a conservative lexical
approximation**: a borrow lives to the end of its scope and the checker rejects
some programs Rust would accept. The approximation never accepts a program
that NLL would reject. The Rust parallel is pre-2018 borrow checking, and the
upgrade path does not change the source-level contract, so no syntax is frozen
around the v1 weakness.

Reborrows are implicit at call sites in both languages; Clorraine spells
nothing extra (O12). Raw pointers are far simpler than Rust's: `raw T` with a
marker-only `unsafe` block, no provenance model, no aliasing guarantees (T6).
Interior mutability (D6), reference counting (D7), and threads (C1) are
absent, so `Send`/`Sync` and shared-mutation aliasing do not exist.

## 7. Generics and lambdas

* Copy obligations at instantiation (V9) play the role of Rust's trait-bound
  errors: the diagnostic names the instantiation site, not the declaration.
* Callable-only type parameters (G7) correspond to `Fn`-family bounds
  restricted to call and pass; values cannot be stored or returned.
* Lambda captures (L2) follow Rust-like defaults (duplicable types copy, other
  types borrow) without `move` closures and without `take` captures. Lambdas
  are ephemeral; C-hook trampolines arrive in v1.1 (L5).

## 8. Deliberate deviations

These are the ledger's D-numbers, restated with their Rust contrast. A
deviation is a design choice, not a bug to fix later.

| Ref | Rust does | Clorraine does | Rationale |
|---|---|---|---|
| D1 | `Drop` trait with drop glue | `freedBy` deleter on the type declaration | No drop glue in v1; C-friendly release functions |
| D2 | Lifetime annotations available | Elision only; ambiguity is an error | Removes the annotation pressure that makes Rust hard to use |
| D4 | Unwinding runs drops | `panic` aborts without drops | Deterministic release on m68k; no unwind tables |
| D5 | Complex temporary-scope table | Temporaries drop at statement end | One simple, teachable rule |
| D6 | `Cell`/`RefCell`/locks | No interior mutability | Keeps aliasing rules simple |
| D7 | `Rc`/`Arc`, `Send`/`Sync` | Absent; single-threaded | Matches the OS task model through FFI (C1) |
| D8 | `fn`, `&`, `&mut` spellings | `func`, `const`, `borrow`, `take` | C-like surface identity |
| D9 | `&` on a copy type is normal | Read-only lends of duplicables copy silently; a writable lend warns W10001 because writes affect only the copy | Generic code must compile for `T = u8` |
| F2 | `&T`/`&mut T` return types | View returns only; aggregate views deferred | Views come from view types and elision |

## 9. Resolution rule

When a language question is not answered by the ledger, this chapter, or the
normative chapter for its area, resolve it by the pinned Rust behavior unless
one of the deviations above applies. Record the resolution as a ledger
amendment so the specification stays derivable rather than remembered.

## 10. Memory-model alternatives considered

A1 was chosen against the alternatives below; the list is kept so the decision
stays reviewable. None of them dominates it under the Amiga constraints:
deterministic release, no runtime support in the emitted C, interrupt and MMIO
safety, 512K-2MB machines, and a future ARM64 profile.

| Family | Release | Lowers to C as | Deciding weakness |
|---|---|---|---|
| Rust-lineage affine ownership + borrows | static, statically known point | explicit release calls, no runtime | chosen (A1) |
| Linear or uniqueness types | static, exactly-once | explicit consume calls | no borrows; every read consumes or clones |
| Affine only (own + clone, no checker) | static, at-most-once | explicit frees | views and C-like patterns become painful |
| Region inference (MLKit, Cyclone) | static, bulk at region exit | region handle + one free | inference is hard; poor fit for OS handles and long-lived values |
| Ownership/capability types (Pony, Spec#) | static, alias-controlled | varies | high conceptual cost, no m68k precedent |
| Reference counting (Swift ARC, `shared_ptr`) | runtime, cycles leak | inc/dec helpers per assignment | per-assignment cost and a runtime; rejected in D7 |
| Precise RC (Koka, Perceus) | runtime, compiler-inserted free | RC and free ops | sophisticated analysis; still runtime cost |
| Tracing GC (Boehm, precise mark/sweep) | runtime, nondeterministic | GC runtime and root discovery | pauses and RAM tax; rejected by A2 |
| Hybrid GC + RAII (D, C#, Java `using`) | mixed | both runtimes | still a heap runtime and pauses |
| Manual C-style (`AllocMem`/`FreeMem`, pools) | programmer | direct C | no safety; kept only as the audited `raw`/`unsafe`/FFI edge |
| Lint-enforced C (GSL, `lifetimebound`) | best-effort tooling | nothing new | unsound; tooling is weak on Amiga |
| C++ lowering (RAII, smart pointers) | static, destructors | C++ toolchain | vbcc is C-only; GCC C++ is heavier |
| Direct 68k backend (no C) | static | native code | loses C interop, debuggers, and both existing backends |

Known v1 seams, to be validated before the checker is built: the conservative
pre-NLL approximation (section 6), the copy-on-lend rule for duplicable values
(D9, V9), and view elision without annotations (section 4). Validation is a
small, hand-checked corpus: a file copier, an interrupt-driven counter, a
string/parser workload, and a buffer/container workload. Repeated V9 or
pre-NLL rejections in those programs are grounds to amend the rule, not to
reopen the model.
