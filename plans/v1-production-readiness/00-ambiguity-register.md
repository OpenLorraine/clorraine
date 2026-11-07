# Ambiguity Register: Clorraine v1.0 Production Readiness

> **Status**: ✅ GATE PASSED — all 16 items resolved or explicitly deferred
> **Last Updated**: 2026-11-07

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | How wide is the production-readiness programme? | A: full v1.0 readiness (compiler hardening + spec stabilisation + thin DX + PLAN-13 acceptance + release gate) / B: compiler-only / C: compiler + DX / D: full language incl. deferred v2 features | Full v1.0 production readiness; deferred language features stay out | ✅ Resolved |
| AR-2 | Technical (complexity escalation) | Which verification and release machinery is authorised? | A: simplified scope (`make asan`, host-only CI, fixed differential seed runner) / B: full generator + scheduled self-hosted `make verify` + release automation / C: revised middle scope / D: defer all extra machinery | **A** — simplified scope; full generator and release automation deferred | ✅ Resolved |
| AR-3 | Scope | What is the production release target? | A: source + Linux x86_64/aarch64 host binaries / B: source only / C: include Amiga SDK installer / D: other | **Revised 2026-11-01:** source tag plus Linux x86_64 host binaries; aarch64 release is deferred to PLAN-07 (developer-machine support remains). Format/version resolved in AR-12 | ✅ Resolved |
| AR-4 | Scope | How should the known report-only residuals be handled? | A: fix release blockers, document the rest / B: fix all / C: accept all / D: other | **A** — fix the explicit `bool(null)`/`!null` diagnostic, nested fill, multi-element view-array, initializer field call-root, and indexed module-view leaks; document the top-level-`while`-only loop split and the ASan `-no-pie` build-environment requirement. **Revised 2026-11-01:** the explicit cast/condition spelling reports E10012; update `spec/04-expressions.md` to name the exception | ✅ Resolved |
| AR-5 | Security & compliance | Which licence should the release use? | MIT / Apache-2.0 / GPL-3.0-or-later / defer | **MIT** | ✅ Resolved |
| AR-6 | Scope | How should the specification be stabilised? | A: mark all v1.0 chapters stable after review / B: keep draft / C: stabilise only changed chapters / D: other | **A** — review the frozen v0.4 text, resolve residual rulings, update stale statements, then mark the v1.0 chapters stable | ✅ Resolved |
| AR-7 | Naming & terminology | Where should the programme live in the roadmap and plan tree? | A: new `PLAN-15 — v1.0 production readiness`, `plans/v1-production-readiness/` / B: link to PLAN-05 / C: no roadmap row / D: other | **A** — new PLAN-15 row and `plans/v1-production-readiness/` | ✅ Resolved |
| AR-8 | Scope | What is the minimum PLAN-13 acceptance slice? | A: `std.amiga.exec` + two allowlisted console programs / B: full PLAN-13 surface / C: let the PLAN-13 plan decide / D: other | **A** — `std.amiga.exec` plus two allowlisted console programs; every implementation, porting, and FFI gap is fixed. An accepted-program gap blocks PLAN-13 Done; a language-surface gap stops for a user ruling and is outside the slice | ✅ Resolved |
| AR-9 | Integration points | What is the CI topology? | Host-only GitHub Actions / also scheduled self-hosted `make verify` / no CI | Host-only GitHub Actions: `./spec/check.sh` plus `cargo test --workspace`; m68k legs skip when the cross toolchain is absent. No self-hosted runner in this plan | ✅ Resolved |
| AR-10 | Technical unknowns | Which tracked programs form the fixed differential seed? | A: the three host-portable corpus programs / B: every corpus program / C: a new seed set / D: other | **A** — `buffer-container`, `file-copier`, and `string-parser`, the corpus programs whose imports compile on both the host and hosted-amiga profiles; the Amiga-only programs keep their existing corpus legs | ✅ Resolved |
| AR-11 | Technical unknowns | Which suites should `make asan` run? | A: every suite that honours `RDC_ASAN` / B: a hand-picked subset / C: only the new residual suites / D: other | **A** — every test file that references `RDC_ASAN` (21 at base; 22+ after Phase 1 adds the production-readiness suite), each suite in its own bounded process with a `-no-pie` ASan `cc` wrapper; the wrapper removes the PIE startup failure | ✅ Resolved |
| AR-12 | Technical unknowns | What release artifact format and version should be published? | v1.0.0 tarballs + checksums / source-only tag / distribution packages / other | **Resolved 2026-11-01:** v1.0.0; source tag plus a Linux x86_64 binary tarball with `SHA256SUMS` and release notes/changelog. Publication is the conditional sub-plan, not part of the 45-task main plan | ✅ Resolved |
| AR-13 | Technical unknowns | Which two allowlisted console programs should PLAN-13 port? | Candidate public-domain/BSD/MIT/APL/GPL-compatible console programs / let PLAN-13 planning select them / other | **Resolved 2026-11-03 (user):** program 1 is **Hamurabi** (`michaelcmartin/bumbershoot`, `full/hamurabi.c`, MIT, © 2015 Michael C. Martin); program 2 is **`exec-self-test`**, a first-party MIT console program written for this slice. Licence audit: the repository `LICENSE` is MIT; only `full/hamurabi.c` is ported, and its upstream notice is preserved in `THIRD_PARTY_NOTICES.md`; the first-party program is MIT by construction. One external port plus one first-party acceptance program, as the user approved. | ✅ Resolved |
| AR-14 | UX & presentation | Where should accepted residuals be documented? | A: `docs/known-limitations.md` / B: `README.md` only / C: no document / D: other | **A** — a dedicated `docs/known-limitations.md`, linked from the README | ✅ Resolved |
| AR-15 | Naming & terminology | What names should the new artifacts use? | A: proposed names / B: other names | **A** — plan `v1-production-readiness`; tests `v1_production_readiness_{spec,impl}_test.rs`; targets `make asan`, `make differential`; scripts `docker/scripts/asan-suite.sh`, `docker/scripts/differential-seed.sh`; workflow `.github/workflows/check.yml`; document `docs/known-limitations.md` | ✅ Resolved |
| AR-16 | Measurement | Which target-side metric does `make bench` use? | A: `vamos` instruction count + linked section sizes / B: FS-UAE wall time / C: FS-UAE cycle counter / D: none | **A** — guest instruction count plus section sizes; host time is advisory. FS-UAE has no stable headless cycle counter in the pinned version, and wall time includes boot and display work. The measurement boundary is recorded in `docs/known-limitations.md` §9. | ✅ Resolved — added 2026-11-07 |

## Resolution Notes

**AR-1 — programme scope.** The v1.0 scope remains the roadmap's bounded core
(`plans/00-roadmap.md:589`); the deferred language features (`comptime`, pools/arenas,
`stores`/`transfers`, nested templates, user contract tables, C-hook trampolines) remain out
(`plans/00-roadmap.md:594`). The programme sequences compiler hardening, spec stabilisation,
the existing DX plan, PLAN-13 acceptance, and release readiness.

**AR-2 — complexity escalation evidence.**
- **Original goal:** make the v1.0 compiler production-ready and releaseable.
- **Extra system or support code:** `make asan`, a fixed host-vs-m68k differential seed runner,
  and a host-only GitHub Actions workflow.
- **Why it may be needed:** the tracked backlog is zero (`plans/00-roadmap.md:7`), but the ASAN
  suites are opt-in and there is no push-time verification; the production claim needs a
  sanitizer signal and a differential signal.
- **Evidence:** 81 integration test files; 21 test files reference `RDC_ASAN`; the existing
  `docker/scripts/corpus-amiga.sh` runs only the m68k corpus and diffs recorded output; there
  is no `.github` workflow; `make verify` needs FS-UAE, X, and local ROMs (`Makefile:87-91`).
- **Smallest solution that still works:** rely on `make check`/`make verify` and prepare
  releases manually.
- **Extra cost:** one Makefile target, one shell script, one differential script, and one CI
  workflow; the full generator and release automation are deferred.
- **Independent verdict:** `Simplify` — add `make asan`, a narrow host-only CI job, and a fixed
  differential seed runner; defer the full generator and release automation.
- **Direct user decision:** adopt the simplified scope (A above).

**AR-3 / AR-12 — release target and version.** Revised 2026-11-01: the release is a source tag
plus a Linux x86_64 binary tarball with `SHA256SUMS`, release notes, and a changelog entry. The
version of record is v1.0.0. aarch64 host support remains a developer-machine build and is not a
v1.0 release artifact; an aarch64 release is deferred to PLAN-07. Publication is a conditional
sub-plan outside the 45-task main plan, so the main plan can close production-ready if the
artifact cannot be published.

**AR-4 — residual evidence.** The release-blocking residual shapes are the module-level nested
fill (`Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0]`), the module-level
multi-element view array (`Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0]`), the local initializer
field call root (`Box { n: sink(mk().xs[0]) }`), and the module-level indexed view global
(`let g: str = mk()[pick()];`); each leaks 11 bytes at the current revision. **User ruling
2026-11-01:** the explicit `bool(null)` and `!null` spellings report E10012; the annotated
`let b: bool = null;` control already does. `spec/04-expressions.md` records the null-cast
exception in Phase 2; there is no new bool conversion. The accepted residuals are the top-level-
`while`-only loop split (`plans/def-zero-closeout/05-findings.md:316-318`) and the host-only CI
limitation; the ASan PIE startup failure is a documented build-environment requirement handled by
`make asan`. The multi-element view-array leak is in this fix list, not the accepted list.

**AR-8 / AR-13 — PLAN-13 acceptance.** The slice requires `std.amiga.exec` and two allowlisted
console programs. Phase 5 has an explicit gate before task 5.1.1: the user selects the two
programmes and records the licence audit. The user resolved the gate on 2026-11-03 with Hamurabi
(MIT external port) and `exec-self-test` (first-party MIT console program); the plan records
those names. Porting and FFI gaps are fixed; an accepted-program gap blocks PLAN-13 Done; a gap
that needs a language-surface change stops for a user ruling and is outside the slice.

**AR-9 — CI scope.** The workflow runs `./spec/check.sh` and `cargo test --workspace` on a
GitHub-hosted Ubuntu runner with `permissions: contents: read`, a SHA-pinned checkout, no
persisted credentials, and a 30-minute timeout. Tests that need `m68k-amigaos-gcc` skip themselves
(`compiler/tests/freestanding_spec_test.rs:149-152`,
`compiler/tests/codegen_ownership_impl_test.rs:210-213`). `make verify` stays a local
Acceptance gate until a runner with an X display and ROMs exists.

**AR-10 — differential seed evidence.** `buffer-container`, `file-copier`, and `string-parser`
import only `std.mem`, `std.io`, `std.text`, and `std.fmt`; `graphics`, `interrupt-counter`, and
`volatile-mmio` use profile-specific or Amiga-only surfaces. `freestanding` also compiles on the
host, but it is the freestanding-profile program and is not part of the hosted-amiga seed. The
target is a convenience gate over the same recorded outputs the host and m68k corpus legs already
check; it is not an independent differential signal.

**AR-11 — `make asan` evidence.** The 21 base files are the `compiler/tests/*.rs` files that
reference `RDC_ASAN`; Phase 1 adds `v1_production_readiness_spec_test.rs`, which also references
`RDC_ASAN`, so the discovered count becomes 22+ (23 if the implementation suite also references
it). The target runs each file's test target separately with `--test-threads=1` and a `-no-pie`
`cc` wrapper for the ASan child builds, so one suite failure cannot abort the batch and the
container's PIE startup failure is removed.

**AR-14 — accepted residuals.** `docs/known-limitations.md` records exactly the SDK image's ASan
`-no-pie` build requirement (applied by `make asan`), the top-level-`while`-only loop split, and
the host-only CI limitation. There is no open-ended catch-all; a new accepted residual must be added
to the list and this register before release. It is documentation, not a new test harness. `03-05`
is the sole owner of the document; `03-01` supplies the verified facts. The multi-element
view-array leak is in the AR-4 fix list, not the accepted list.
