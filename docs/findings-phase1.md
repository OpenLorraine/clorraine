# Phase 1 findings - specification rewrite and compile-fail suite

Status: complete. Feed the Phase 2 compiler MVP.

## Scope

* **Phase C**: rewrite every chapter of the specification against the frozen
  decision ledger and the frozen grammar.
* **Phase D**: cross-validate diagnostics, add the golden compile-fail suite,
  extend `check.sh`, and record this report.

Artifacts: [`spec/`](../spec/README.md), [`spec/tests/compile-fail/`](../spec/tests/compile-fail/README.md), [`spec/tests/run/`](../spec/tests/run/README.md), [`spec/tests/staged/`](../spec/tests/staged/README.md).

## Results

| Check | Method | Result |
|---|---|---|
| Headers, status lines, relative links | `spec/check.sh` | OK (27 files) |
| Grammar productions defined vs indexed | extraction script | 137 productions, 0 mismatches |
| Chapter diagnostics vs registry (N7) | `spec/check.sh` | 0 unregistered codes |
| Compile-fail coverage | `spec/check.sh` | 101 tests; runtime faults and `E90000` excluded by design |
| Run suite created | `spec/tests/run/` | 6 runtime-fault cases (`E11210`-`E11213`, including MIN/-1 and NaN) |
| Staged suite created | `spec/tests/staged/` | 4 staged files (`E90000`, `E90003`, `E90004`, `W10005`) |
| Stale vocabulary | grep across chapters | none live; historic Blend65 codes appear only in negative statements |
| Independent consistency review | adversarial reviewer pass | 26 findings, all fixed or clarified |

Verification command:

```sh
cd spec && ./check.sh
# spec check: OK (27 files, 101 compile-fail tests)
```

## Grammar corrections found during Phase C

| Finding | Resolution |
|---|---|
| `integer_type` was referenced by `enum_decl` but never defined | `enum_decl` now takes `scalar_type`; a non-integer backing type is the semantic check E10211 |
| Empty structs could not parse, so E10206 was unreachable | `struct_decl` accepts zero fields; the semantic pass rejects E10206 |
| `export ffi` and struct-level `freedBy` could not parse, so E10103/E10104 were unreachable | Both forms parse and are rejected with the targeted codes |
| `union` declarations and misplaced `volatile` had no path to E10204/E10208 | Parser-only diagnostic starts, documented in grammar section 11 |

## Ledger clarifications recorded (decisions.md section 20)

| Ref | Gap closed |
|---|---|
| T1a | Same-signedness integer widening is implicit; mixed-sign, narrowing, `usize`/`isize`, and float conversions are explicit |
| B7a | Signed `>>` is sign-replicating, unsigned `>>` is zero-filling; counts are masked |
| T6a | `raw`-to-integer value casts are allowed outside `unsafe` |
| T2a | Core handles declare their own deleters (`FreeString`, `FreeBuffer`); the ledger's `handle String freedBy FreeVec` shorthand was not type-correct under X1 |
| T10a | Enums compare with `==`/`!=` only; members number sequentially from 0 |
| V9a | A lend instantiated with a duplicable type binds a copy; W10001 marks the needless lend |
| X1a | `freedBy` names resolve like any other qualified name |
| E112xx | Runtime fault codes live in the E112xx block and print the standard first line |
| N8a | Phase C codes: E10021, E10216, E11211-E11213, E10218-E10222; E10605 retired into E10208 |
| S2a | Float placeholders print six significant digits; the public conversion is `text` |
| P8a | Template desugaring is internal; helper names are not surface |
| M7a | A nullable module initializer is E10200 |
| W10004a | Stack-frame thresholds are profile defaults (hosted-amiga 1024, host 16384) |

## Compile-fail suite conventions and limits

* One `.clo` file per registry code; the first line declares
  `// expect-error CODE` or `// expect-warning CODE`.
* The suite is **not executable yet**: no compiler exists, so `check.sh`
  validates structure and coverage, not behavior.
* `E11210`-`E11213` are runtime faults; their cases live in `tests/run/`
  (6 files, including MIN/-1 and NaN). `E90000` is the generic staged fallback
  and has no unique trigger; it, `E90003`, `E90004`, and `W10005` are documented
  under `tests/staged/`.
* `E10100`/`E10101` depend on a harness convention for "uncontracted extern";
  the tests document the intent that an `ffi` declaration without ownership
  annotations is uncontracted.
* `E11100` uses a single-module initializer cycle; a true cross-module cycle
  needs the multi-file harness of Phase 2.
* `E90003`/`E90004` have no frozen syntax; their files are best-effort intent
  records.
* Staged codes carry `// stage: N` annotations and activate when that stage
  lands (E90001, E90002, E10306, E10401, E10402, E10404).
* Message text is still draft (Ch 14 section 15); the suite asserts codes
  first, message templates once the wording freezes.

## Open items for Phase 2

| Item | Owner |
|---|---|
| Implement the semantic stages behind the staged diagnostics | Compiler MVP |
| Turn the compile-fail suite into a harness that compares the first diagnostic line | Compiler MVP |
| Run the generated-C FFI spike: compile emitted prototypes against the NDK headers and confirm the ABI mapping (Ch 17 section 3) | Compiler MVP |
| Extend the run suite with the positive corpus (the six runtime-fault cases now exist) | Compiler MVP |
| Promote the staged tests to `tests/compile-fail/` as their features or harness support lands | Compiler MVP |
| Re-run `preflight` on the specification before the first compiler milestone | Phase E gate |
