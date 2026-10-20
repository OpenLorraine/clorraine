# Requirements

## Scope

The batch closes exactly these rows: `DEF-525` (critical), `DEF-526`, `DEF-527`, `DEF-528`,
`DEF-395`, `DEF-396`, `DEF-401`, `DEF-406`, `DEF-407`, and `DEF-402` (incidental verification).
Each row's filed reproduction is the acceptance case. The batch does not change a frozen rule.

## Functional requirements

| ID | Requirement |
|----|-------------|
| R-R1 | A view stored into an opaque type-parameter member that is returned with its `take` carrier reports the local-owner escape (`E10030`) once, exactly as the concrete twin does. `id<T>(take h: T): T { let a: Box; let s: str = a.v; h.names[0] = s; return h; }` at `T = Holder` rejects. |
| R-R2 | A view stored into an opaque member from caller-backed storage stays accepted when the carrier is returned. The emitted C widens an owned `String` source to the member's `str` view (`cl_string_view`) and does not move the source. |
| R-R3 | A view offered to a member that the instance makes owned reports the concrete implicit-conversion error (`E10200`) at the store and writes no translation unit. |
| R-R4 | A slice whose element is a `func` type emits a valid pointer-to-function-pointer typedef: `i32 (**ptr)(i32)` for `func(i32): i32`. The accepted slice strict-builds, runs, and indexes to a callable value. |
| R-R5 | A conditional whose two arms are distinct capturing closures reports `E10404` (closures are ephemeral) once at the conditional, not `E10200`. Capture-less lambda conditionals stay accepted. |
| R-R6 | A write-borrow capture that stores a caller-backed view (a parameter or a static view) into an enclosing local does not mark that local frame-backed; a later escape of the local is accepted and runs clean. A capture that stores frame-backed storage keeps the conservative rejection. |
| R-R7 | A generic dot call whose return type names a type parameter returns the inferred instance's substituted type. `let x: String = w.pick(s2, s3);` emits the `String` temporary, not `ClT`, and strict-builds. |
| R-R8 | A generic dot call inside a generic caller whose callee declares the same type-parameter spelling binds the callee's parameter from the receiver and arguments, not from the caller's instance. `outer<T>(take a: Wrap<String>, ...) { return a.pick(b, c); }` is accepted at every `outer` instance. |
| R-R9 | An explicit type-argument list on a dot call is checked: the count is validated, the arguments are resolved, and the bindings it fixes drive the parameter and return judgments. A wrong count reports the direct-call code (`E10300`) once. |
| R-R10 | A fully qualified module function call is routed through the same direct/generic checker as its bare twin: arity (`E10308`), explicit type-argument count (`E10300`), argument conversions, and the substituted return type. A qualified generic call returning a fixed array used as a loop iterable emits the concrete element (`i32`), not `Cl#element of T`. |
| R-R11 | A self-qualified generic call in a module initializer declares and uses its instance. The verification case is green at the batch head; it is closed as incidentally fixed only if no dedicated change is required. |

## Non-functional requirements

| ID | Requirement |
|----|-------------|
| R-N1 | Every rejected case reports exactly one diagnostic per site and writes no translation unit. |
| R-N2 | Every accepted control builds under the fixed strict flags and runs ASan/LSan clean. |
| R-N3 | Accepted programs emit byte-identical C across two emissions where the suite pins it. |
| R-N4 | `make check` and `make verify` pass at the batch head. |
| R-N5 | No `spec/` oracle, diagnostic registry, or generated C is edited. |
| R-N6 | The roadmap records each row's closing commit; no row is closed without its red case, except `DEF-402` under the recorded incidental rule. |

## Acceptance criteria

- `DEF-525` rejects `E10030` for the filed local-view reproduction, accepts the caller-backed
  twin, and rejects the owned-instance view store with `E10200`.
- `DEF-527` emits a strict-building typedef and the slice works.
- `DEF-528` reports `E10404` once; the capture-less conditional stays green.
- `DEF-526` accepts and runs the parameter-backed capture write; the frame-backed twin still
  rejects.
- `DEF-395`, `DEF-401`, `DEF-406`, `DEF-407`, and `DEF-396` each turn their accepted-invalid-C
  or false-rejection reproduction into the concrete twin's behavior.
- `DEF-402` is green and recorded as incidental or remains open with a reason.
- The focused suites and the full workspace suite are green.
