# Requirements

The rows are reproduced in `02-current-state.md`; the acceptance criteria below are the oracle.
Every report is the existing diagnostic the concrete twin uses; no new diagnostic is allowed.

| R | Row | Severity | Requirement and acceptance |
|---|-----|----------|----------------------------|
| R1 | `DEF-485` | critical | A `const` conditional carrier copy records both arms' element loans. After `const h2: Holder = c ? h1 : h3;`, writing through the loaned owner reports `E10031` once, exactly as the let form does; a static-arm control stays accepted and clean. |
| R2 | `DEF-486` | critical | A global conditional carrier copy is judged at the store. `g = c ? h1 : h3;` where an arm carries a frame-backed view reports `E10033` once at the store, exactly as the direct carrier store does; a module-backed control stays accepted. |
| R3 | `DEF-487` | critical | A function-returned call-mediated view consults the element loans of its source. `return pick(xs, 0)` after `xs[0] = a.v;` reports the concrete twin's `E10035` once at the return; a parameter-backed source stays accepted and clean. |
| R4 | `DEF-488` | critical | An inline call-result carrier with a frame-backed view is judged at the call boundary. `store(make(a.v), borrow slot)` reports `E10030` once at the call; the same call with a parameter-backed source stays accepted and clean. |
| R5 | `DEF-489` | critical | A conditional mix of a static literal and a call carrier records each arm's positions. `let h = c ? Holder { names: ["AA","BB"] } : make(a.v); store(h, borrow slot);` reports `E10030` once at the store; the all-static conditional stays accepted and clean. |
| R6 | `DEF-493` | critical | A plain carrier copy from a carrier-returning call transfers the single source argument's view positions. `let h = makeHolder(b.v); b.v = ...; read h` reports `E10031` once at the write; a parameter-backed source control stays accepted and clean. |
| R7 | `DEF-495` | critical | A direct call-mediated view resolves the closure, func-typed local, generic, and `unsafe`/conditional wrapper forms. Each filed shape reports the concrete twin's `E10030` once at the call; caller-backed and static wrappers stay accepted and clean. |
| R8 | `DEF-496` | critical | A dot-call receiver is judged as a view source at parameter zero. `b.store(borrow slot)` and `mk().store(borrow slot)` report `E10030` once; a caller-backed receiver stays accepted and clean. |
| R9 | `DEF-497` | critical | A mixed aggregate carrier is judged field by field. An owned field under a carrier with an accepted static position reports `E10030` once at the call; a view-only or caller-backed aggregate stays accepted and clean. |
| R10 | `DEF-498` | critical | A conditional shadow copy snapshots its arm sources before the declaration. `let h: Holder = c ? h : h;` where the outer binding carries a view reports `E10031` once at the later write; a static shadow stays accepted and clean. |
| R11 | `DEF-499` | critical | An alias element return resolves the container's element loans. Returning an element read through `for (borrow e in xs)` after `e = a.v` reports the concrete twin's `E10035` once; a parameter-backed container return stays accepted and clean. |
| R12 | `DEF-501` | critical | A generic conditional take of an opaque projected member records the per-instance duty. `sink(take (c ? h.name : null))` at `T = H { name: String }` reports `E10034` once at the instance and never double-frees; a duplicable member stays accepted. |
| R13 | `DEF-502` | critical | The instance audit strips a raw pointer before a lend field projection. `sink(take (c ? xs.name : null))` at `T = raw H` reports the concrete twin's `E10034` once; a duplicable selected member stays accepted. |
| R14 | `DEF-503` | critical | A generic conditional arm in an owning struct-literal field or projected assignment records the per-instance duty. Both forms report `E10034` once at the instance; the duplicable field instance stays accepted. |
| R15 | `DEF-504` | critical | A projection over a conditional of type-parameter places reports each lend arm. `return c ? (d ? h1 : h2).name : null` at `T = H { name: String }` reports `E10034` twice, one per lend arm, and never double-frees; a duplicable instance stays accepted. |

## Constraints

- Every fix is a conformance fix; `spec/` and the diagnostics registry are not edited.
- A spec test is the immutable oracle for the batch; an existing Rust pin is corrected only under a
  recorded ruling and with the correction recorded in `05-findings.md`.
- Reports are deterministic (same site, same count, same text) and the emitted C stays byte-identical
  for the accepted controls.
- The host ASan/LSan legs run for every accepted case that owns storage.
