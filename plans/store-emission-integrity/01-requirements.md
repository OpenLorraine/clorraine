# Requirements: Store-Emission Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The batch closes four pre-existing major defects filed by the `borrow-store-integrity` preflight
and Phase 3 reviews (filed 2026-10-10):

* **DEF-113** — a conditional-result store through a non-nullable lend root is accepted; when the
  non-slot arm runs, the emitted conditional store frees the caller's value and a later read is a
  heap-use-after-free.
* **DEF-114** — a write to an element of an immutable `str` view is accepted and emits an
  assignment to read-only storage, which the strict C build rejects.
* **DEF-115** — the emitted C defines the `ClSlice_*` typedef before its element typedef
  (`ClP`, `ClE`), so accepted programs fail the strict build.
* **DEF-116** — a module-level fixed-array initializer emits an array assignment (`backing = { … };`)
  and a module-level `slice(...)` is not lowered, so accepted programs fail the strict build.

Every fix enforces or completes behavior the frozen specification already states (BR-5, the
mutability table in Ch 02 §6, the emission contract in Ch 17); the only specification change is
the `E10042` registry row and its golden, which the user explicitly authorized (AR-2, AR-12).

## Functional Requirements

### Must Have

- [x] **R1 (DEF-113)** — a whole-value conditional store into a non-nullable `borrow` parameter
  that owns storage is rejected with exactly one `E10034` naming the target, for the concrete
  and the generic spellings alike; the judgment is per instance for a still-parameter-typed slot
  (owning → rejected, duplicable → accepted); the nullable out-slot conditional stays accepted
  (AR-4, AR-5).
- [x] **R2 (DEF-114)** — any write to an element of a `str` or `static str` view is rejected with
  exactly one new `E10042`, for plain, compound, and update forms over parameter, local, field,
  and module-level bases; whole-view rebinds, element reads, and writes through a `borrow`-derived
  `u8[]` view stay accepted (AR-6, AR-7).
- [x] **R3 (DEF-115)** — the emitted C defines every typedef before its uses: a slice of a user
  struct or enum builds under the strict flags, and a struct that embeds an enum builds; closure
  environments that capture a slice stay valid (AR-8).
- [x] **R4 (DEF-116)** — a module-level fixed-array initializer is emitted element-wise (flat,
  nested, fill, place-copy, and conditional values), and a module-level compiler-intrinsic call
  (`slice`) is lowered to the intrinsic view construction with operand-once evaluation and the
  existing bounds guards (AR-9, AR-10, AR-11, AR-18).

### Should Have

- [x] **R5** — byte-identical re-emission: compiling an accepted program twice produces identical
  translation units (the existing determinism convention).
- [x] **R6** — the memory cases of R1 are ASan-clean on the host probes, and the accepted
  alternative spellings keep running (AR-13).
- [x] **R7** — the new `E10042` golden participates in `./spec/check.sh`, and `spec/` changes are
  limited to the authorized modification set (AR-12).

### Won't Have (Out of Scope)

* `cstring` element writes — read-only modes already report `E10024`; the borrow-mode question is
  unfiled and separate (AR-15).
* `const` array initializers — currently rejected `E10224` (a rejection gap, not an
  accepted-program breakage; AR-15).
* DEF-97 and DEF-98 (strict-build failures recorded in other suites) — they stay in the backlog.
* Accepting the DEF-113 shape with arm-wise-correct lowering — it would contradict BR-5
  ("the callee may not release or store" a lend); the fix rejects instead (AR-4).
* New initializer or intrinsic syntax; `rawSlice`/`rawStr` module-level tests (the detection
  covers them; only `slice` is pinned by ST cases).
* Any change to the frozen specification beyond AR-12's registry row and golden.

## Technical Requirements

### Compatibility

- The emitted C keeps the strict C99 common subset (Ch 17 §1) for the `host`, `hosted-amiga`,
  and `freestanding` profiles; `make check` and `make verify` stay green.
- The workspace stays std-only; no new crates, build scripts, or dependencies.

### Performance

- No performance surface: the fixes change compile-time judgment and emission shapes only.

### Security

- Memory safety is the security surface of this compiler. R1 removes a live
  heap-use-after-free reachable from accepted source; the host AddressSanitizer probes (AR-13)
  are the verification oracle for the memory cases.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch membership | DEF-113–116 / DEF-113 only / all 12 open majors | DEF-113–116 | The four majors queued as the next batch; bounded, all repro-verified | AR-1 |
| DEF-114 diagnostic | New E10042 + registry row + golden / reuse E10221 / reuse E10023 | New E10042 (user ruling) | The frozen registry has no fitting trigger; a dedicated code keeps the contract honest | AR-2 |
| DEF-115 coverage | All three verified instances / filed case only | All three | One ordering pass fixes the class; covering is smaller than filing | AR-3 |
| DEF-113 mechanism | Target-aware suppression exclusion / new duty / arm-wise lowering | Suppression exclusion | Mirrors the marking path's existing self-arm exemption; smallest correct change | AR-4, AR-15 |
| `spec/` modification set | Registry row + golden / wider edits | Row + golden only | The user's E10042 ruling; the oracle otherwise stays frozen | AR-12 |
| Execution commit mode | `--auto-commit` | Commit + push per verified task | User instruction (repo precedent) | AR-16 |

> **Traceability:** every scope decision back-references the Ambiguity Register entry that
> resolved it — see [00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [x] All four Must-Have requirements verified by their specification suites (ST cases in
   [07-testing-strategy.md](07-testing-strategy.md) traced to their sources).
2. [x] `make check` green after every phase; `make verify` green at close-out.
3. [x] Host ASan probes clean for the R1 memory cases; the accepted alternatives run.
4. [x] `./spec/check.sh` green with the new `E10042` golden; no other `spec/` change.
5. [x] No warnings under `-Wall -Wextra -Werror` for every covered emitted unit.
6. [x] No dead code; no new dependencies; the workspace stays std-only.
7. [x] The four roadmap rows link this plan and advance appropriately.
