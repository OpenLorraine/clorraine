# Requirements: Clorraine v1.0 Production Readiness

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The Clorraine v1.0 core is feature-complete: the tracked defect backlog is zero, the
specification is frozen for implementation, and the host, hosted-amiga, and freestanding
profiles pass their acceptance legs. Production readiness adds the missing proof and release
artifacts without changing the v1.0 language surface.

The programme fixes the release-blocking residual classes, stabilises the specification,
adds a narrow sanitizer and differential signal, executes the already-planned DX slice, completes
a bounded Amiga acceptance slice, and prepares the MIT-licensed release. The full generator,
scheduled self-hosted verification, release automation, and deferred v1.1/v2 language features
remain out of scope.

## Functional Requirements

### Must Have

- [ ] **R1 — Release-blocking residuals.** Make the explicit `bool(null)`/`!null` spellings report
  E10012 (user ruling 2026-11-01), and fix the nested fill, multi-element view-array, initializer
  field call-root, and indexed module-view leaks. Each fix carries a specification test and an
  ASan probe; accepted residuals are documented in `docs/known-limitations.md` (AR-4, AR-14). The
  already-correct `let b: bool = null;` control stays unchanged.
- [ ] **R2 — Stable v1.0 specification.** Review the frozen v0.4 text, update stale statements,
  record the E10012 null-cast exception, and mark the v1.0 chapters `stable (v1.0)`.
  `./spec/check.sh` stays green; no semantic change beyond the AR-4 conformance fix (AR-6).
- [ ] **R3 — `make asan`.** A target runs every test file that honours `RDC_ASAN`, one bounded
  process per suite with serial tests and a `-no-pie` ASan `cc` wrapper, so the container's PIE
  startup failure cannot abort the batch (AR-11).
- [ ] **R4 — `make differential`.** A fixed runner emits the three host-portable corpus programs
  for host and hosted-amiga, runs both, and diffs their stdout. The target joins `make verify` as
  a convenience gate over the same recorded outputs the corpus already pins; it is not an
  independent signal (AR-10).
- [ ] **R5 — Host-only CI.** GitHub Actions runs `./spec/check.sh` and
  `cargo test --workspace` on pushes and pull requests. m68k-only tests skip when the cross
  toolchain is absent; `make verify` remains local (AR-9).
- [ ] **R6 — DX slice.** Execute the existing `plans/clorraine-dx/` plan to 28/28. The DX plan
  owns its tasks and ST cases; this programme only gates on its completion.
- [ ] **R7 — PLAN-13 acceptance.** After the user records the two programme names and licence
  audit at the Phase 5 gate, create and execute a PLAN-13 plan for `std.amiga.exec` plus those two
  allowlisted console programs. The two programs build and run on hosted-amiga against an
  independent expected-output oracle; every implementation, porting, and FFI gap they expose is
  fixed, while a gap that needs a language-surface change stops for a user ruling (AR-8, AR-13).
- [ ] **R8 — MIT licence and metadata.** Add the MIT `LICENSE`, set the Cargo package licence and
  version `1.0.0`, update the README licence section, and set `tools/demo/package.json` to MIT
  with a full manifest audit (AR-5).
- [ ] **R9 — Release preparation.** Write `RELEASE.md` with the release checklist and the AR-12
  decision. AR-12 is resolved to v1.0.0 source + Linux x86_64 binary tarball; the 45-task main
  plan closes production-ready, and publication is the separate conditional publish sub-plan.
- [ ] **R10 — Final production gate.** `make check`, `make verify`, `make asan`, and
  `make differential` pass; the DX plan is Done; PLAN-13 is Done; the spec is stable; CI is green
  or locally validated with the deferred run recorded; the licence, release checklist, and
  known-limitations documents exist.

### Should Have

- [ ] **R11 — Documentation sync.** Update `AGENTS.md` with the new targets, workflow, and
  `docs/known-limitations.md`; update the roadmap with PLAN-15.
- [ ] **R12 — Release notes.** In the conditional publish sub-plan, add concise v1.0 release
  notes and a changelog entry.
- [ ] **R13 — Host-binary smoke test.** In the conditional publish sub-plan, build the Linux
  x86_64 binary and run `clc --version` (asserting the exact output `clc 1.0.0`) and a
  hello-world compile. aarch64 release artifacts are deferred to PLAN-07; aarch64
  developer-machine builds remain supported.

### Won't Have (Out of Scope)

- `comptime`, pools/arenas, `stores`/`transfers`, nested templates, user contract tables, and
  C-hook trampolines (deferred language features, `plans/00-roadmap.md:594`).
- The full grammar-based program generator, a scheduled self-hosted `make verify`, and release
  automation (AR-2; deferred until a measured need).
- The Amiga SDK fetch/detect installer, LSP, debugger, package manager, and VSCode packaging.
- ARM64 target support, LorraineOS bring-up, and PLAN-14 optimisation.
- Any new Rust crate, build script, or external runtime dependency.

## Technical Requirements

### Compatibility

- Host support remains x86_64 and aarch64 developer machines; the v1.0 release artifact is Linux
  x86_64 only, with aarch64 release deferred to PLAN-07. The three profiles remain `host`,
  `hosted-amiga`, and `freestanding`.
- Emitted C remains valid C99 and keeps the existing strict host and Amiga toolchain legs.
- The new Makefile targets are additive; existing `make check` and `make verify` semantics
  stay unchanged except that `verify` also runs `differential`.
- The `bool(null)`/`!null` fix applies the frozen nullability rule (`E10012`, TY-7); it does not
  add a new bool conversion or change any accepted program. `spec/04-expressions.md` records the
  null-cast exception to TY-14.

### Security

- CI uses least privilege (`permissions: contents: read`), no repository secrets, a SHA-pinned
  checkout, no persisted credentials, and only the standard GitHub-hosted runner.
- Release artifacts are checksummed in the conditional publish sub-plan; the licence, source
  provenance, and every manifest licence (including `tools/demo/package.json`) are audited.
- The plan adds no server, network, credential, or stored-data surface.

### Performance

- `make check` remains the fast gate; `make asan` and `make differential` are separate targets.
- `make differential` uses bounded `timeout` calls and the existing `vamos` runner.
- No optimisation work is included; PLAN-14 remains measured-need-gated.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Programme scope | compiler-only / compiler + DX / full readiness / full language | Full v1.0 readiness | Matches the agreed sequence; deferred features stay out | AR-1 |
| Verification machinery | full generator / simplified / manual only | Simplified | Challenger returned `Simplify`; delivers sanitizer and differential signals without a framework | AR-2 |
| Release target | source only / source + Linux binaries / packages | Source tag + Linux x86_64 binary tarball | Host support is x86_64/aarch64; v1.0 artifacts are x86_64 only; no Amiga installer | AR-3 |
| Residual policy | fix all / fix blockers + document / accept all | Fix blockers, document rest | Keeps the release bounded; accepted risks are explicit | AR-4 |
| Licence | MIT / Apache-2.0 / GPL / defer | MIT | Simple, permissive, GPL-compatible | AR-5 |
| Spec status | stable / keep draft / changed chapters only | All v1.0 chapters stable | The frozen v0.4 is the production rule set | AR-6 |
| Roadmap | new PLAN-15 / link to PLAN-05 / none | New PLAN-15 | The programme spans more than DX | AR-7 |
| PLAN-13 slice | exec + two programs / full surface / plan decides | `std.amiga.exec` + two programs | Bounded acceptance; real Amiga code exercises the compiler | AR-8 |
| CI | host-only / self-hosted verify / none | Host-only | Hosted runners cannot supply FS-UAE, X, and ROMs | AR-9 |
| Differential seed | three portable programs / all corpus / new set | Three portable programs | They compile on both host and hosted-amiga | AR-10 |
| ASan set | all RDC_ASAN suites / subset / new suites | All RDC_ASAN suites | Keeps the target honest as suites are added | AR-11 |
| Release format | tarballs / source only / packages | v1.0.0 source tag + x86_64 binary tarball + SHA256SUMS | AR-12 resolved 2026-11-01; aarch64 deferred | AR-12 |
| PLAN-13 programs | pending | Deferred | User selects at the Phase 5 gate before execution | AR-13 |
| Known limitations | dedicated doc / README / none | `docs/known-limitations.md` | One durable place for accepted risks | AR-14 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved
> it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] All seven phases and 45 main-plan tasks are complete; the 2 conditional publish tasks are
   complete or explicitly blocked.
2. [ ] `make check` passes.
3. [ ] `make verify` reports all legs passed.
4. [ ] `make asan` passes.
5. [ ] `make differential` passes.
6. [ ] The v1.0 spec chapters are `stable (v1.0)` and `./spec/check.sh` passes.
7. [ ] The DX plan is 28/28 and its verification is green.
8. [ ] The PLAN-13 slice is Done and both programs match an independent expected-output oracle.
9. [ ] The host-only CI workflow is green on the final commit, or its two commands are validated
   locally and the deferred CI run is recorded when no external push is authorized.
10. [ ] MIT `LICENSE`, Cargo metadata, README licence text, and `tools/demo/package.json` MIT
    metadata are present.
11. [ ] `docs/known-limitations.md` records the accepted residuals.
12. [ ] `RELEASE.md` records the AR-12 decision (v1.0.0, source + x86_64 tarball); the conditional
    publish sub-plan produces the artifact, smoke test, checksums, release notes, and changelog
    entry, or records why publication is blocked.
