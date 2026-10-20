# Requirements

## Scope

The batch closes exactly the eleven open critical rows: `DEF-508`..`DEF-512`, `DEF-514`,
`DEF-515`, `DEF-518`, `DEF-519`, `DEF-521`, and `DEF-523`. Each row's filed reproduction is the
acceptance case. The batch does not close a major or minor row, and it does not change a frozen
rule.

## Functional requirements

| ID | Requirement |
|----|-------------|
| Q-R1 | A projected member of a carrier-returning call is judged at the frame that receives it. `store(make(a.v).names, borrow slot)` reports the local-owner escape (`E10030`) once, exactly as `store(make(a.v), borrow slot)` does. |
| Q-R2 | A carrier-returning call with several lend sources contributes every source's view positions. `makePair(a.v, b.v)` and `makeHolder("AA", b)` report the frame-backed source (`E10030`) once at the receiving call. |
| Q-R3 | A carrier return through a `take` argument or an owned-aggregate argument follows that source. `id(take h1)` records `h1`'s view positions on the result; the caller's later store reports (`E10030`) once. Returning the taken carrier itself stays accepted when its views derive from caller storage. |
| Q-R4 | A carrier call nested in a returned struct literal is judged at the return. `return S { h: c ? makeFrom(a) : Holder { ... } }` reports the local-owner escape (`E10030`) once. |
| Q-R5 | An inline lambda callee resolves its own signature in the call check and in both source walks. `store(((ys: str, j: i32) => ys)(s, 0), borrow slot)` reports the local-owner escape (`E10030`) once; a local control stays accepted and clean. |
| Q-R6 | A conditional over func values resolves the common signature and lowers through the selected callable value. The escaping form reports `E10030` once; an accepted local form emits valid C that strict-builds and runs. |
| Q-R7 | A capture-backed out-slot write of a view reports the owning escape. `([a, borrow slot] () => { slot = a.v; })()` reports the local-owner escape (`E10030`) once; a control whose capture is a parameter-backed view stays accepted and clean. |
| Q-R8 | A generic conditional `take` of a `take` parameter's owning member moves the selected member out of the parameter per arm. The concrete twin's accepted program runs clean under ASan; the generic body emits the same per-arm move-out. |
| Q-R9 | An `unsafe`- or cast-wrapped conditional take is judged and lowered exactly like its bare twin. |
| Q-R10 | An owned array-literal element whose value is a generic conditional records the per-instance duty. At `T = H { name: String }` the element move from a lend parameter reports `E10034` once; a duplicable instance stays accepted and clean. |
| Q-R11 | An `unsafe`- or cast-wrapped generic conditional source is judged like its bare twin. The struct-literal field, projected assignment, take, and binding spellings report `E10034` once at an owning instance. |
| Q-R12 | A nested write below a partially moved field keeps the moved prefix. The write reports the moved-value error (`E10020`) once instead of clearing the field's moved state. A whole-field re-initialization stays accepted and clean. |

## Non-functional requirements

| ID | Requirement |
|----|-------------|
| Q-N1 | Every rejected case reports exactly one diagnostic and writes no translation unit. |
| Q-N2 | Every accepted control builds under the fixed strict flags and runs ASan/LSan clean. |
| Q-N3 | Accepted programs emit byte-identical C across two emissions. |
| Q-N4 | `make check` and `make verify` pass at the batch head. |
| Q-N5 | No `spec/` oracle, diagnostic registry, or generated C is edited. |
| Q-N6 | The roadmap records each row's closing commit; no row is closed without its red case. |

## Acceptance criteria

- The eleven filed reproductions reject once (508, 509, 510-escape, 511, 512, 518, 519, 521, 523)
  or run clean with the twin's move-out (514, 515).
- The accepted controls run clean and strict-build; the lambda/conditional controls emit valid C.
- The focused suites and the full workspace suite are green.
