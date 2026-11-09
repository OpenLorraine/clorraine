# Execution Plan: Volatile Nested Fill Fix (PLAN-23)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-09
> **Last Updated**: 2026-11-09
> **Progress**: 30/44 tasks (68%)
> **CodeOps Artifact Schema**: 1
> **Execution mode**: inline primary agent
> **Commit mode**: `--auto-commit` with pushes held until the full fix gate

## Overview

Reproduce the invalid C, pin both nested fill copy sites with behaviour and
emission tests, add the volatility signal and the per-leaf copy, run the full
fix gate set and the four codegen reviews, close the fix, then prepare and
publish v1.1.1 as the separate PLAN-24 row.

**Update this document after EACH completed task.** `[ ]` not started, `[~]`
implemented with verification pending, `[x]` verified, `[!]` blocked.

## Dependency Graph

```
0.1 preflight ──> 0.2 plan-docs commit + PLAN-23 row
0.2 ──> 1.1 behaviour cases ──> 1.2 emission pins ──> 1.3 red run
1.3 ──> 2.1 flag + predicate ──> 2.2 copy helper + fill branch ──> 2.2.2 prefix branch
2.2.2 ──> 2.3 focused green ──> 2.4 integrity ──> 2.5 fixture + bench-shape
2.5 ──> 2.6 make check ──> 2.7 commit
2.7 ──> 3.1 bench base + head + compare ──> 3.2 emission audit
3.2 ──> 3.3-3.6 reviews ──> 3.7 fixes ──> 3.8 sanitizers ──> 3.9 findings commit
3.9 ──> 4.1 acceptance ──> 4.2 roadmap/docs ──> 4.3 closeout commit
4.3 ──> 4.4 full fix gates ──> 4.5 push
4.5 ──> 5.1-5.3 v1.1.1 prep ──> 5.4 release gates ──> 5.5 tag + artifacts
5.5 ──> 5.6 explicit publication authorization ──> publish
publish ──> 5.7 publication record ──> 6.1 final report
```

## Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Plan and preflight | 3 |
| 1 | Red reproduction | 3 |
| 2 | Implementation and focused green | 9 |
| 3 | Measurement, audit, reviews, sanitizers | 10 |
| 4 | Closeout and fix gates | 5 |
| 5 | v1.1.1 release (PLAN-24) | 13 |
| 6 | Final report | 1 |

**Total: 44 tasks across 7 phases.**

---

## Phase 0: Plan and Preflight

> **Reasoning**: medium - the defect, the two copy sites, and the scope are
> fixed here.
> **Verify**: the preflight report has no open critical or major finding.

- [x] 0.1.1 Run the plan preflight against the repository; record every
      finding with severity, evidence, and ruling in
      `00-preflight-report.md`; re-check the critical and major findings after
      the revision.
- [x] 0.1.2 Resolve every critical and major finding; update the owning
      document and record the verdict.
- [x] 0.2.1 Add the PLAN-23 roadmap row as `Executing`, update the progress
      header (566/569, 1 open, `Active - volatile nested fill fix`), review
      `git diff --cached`, and commit
      `docs(plans): add the volatile nested fill plan`; confirm a clean tree.

**Verify**: preflight clean; plan docs and roadmap row committed.

---

## Phase 1: Red Reproduction

> **Reasoning**: medium - the tests are the oracle and must fail on the base
> for the right reason.
> **Verify**: the new behaviour cases fail to build on the base; the new pins
> fail; the guards pass.

- [x] 1.1.1 Add ST-V1..V12 to
      `compiler/tests/nested_fill_compaction_spec_test.rs`, including the
      build-only overlay, field-index, and pointer-field cases, and update the
      file header: the existing cases pass on the base; the new volatile cases
      are deliberately red on the base.
- [x] 1.2.1 Replace the legacy `should_keep_a_volatile_global_target` test
      (old IP-11) with IP-V1..V18 in
      `compiler/tests/nested_fill_compaction_impl_test.rs`, using the full
      mangled global symbols; IP-V12 pins the unchanged whole-array copy, and
      IP-V13..V18 pin the runtime-index, impure-prefix, pointer-field, and
      volatile-source index shapes. The old IP-11 and the new IP-V11 are different pins.
- [x] 1.3.1 Run both focused suites on the unmodified compiler; record the
      exact failures in [Findings](05-findings.md).

**Verify**: base failures recorded; guards pass.

---

## Phase 2: Implementation and Focused Green

> **Reasoning**: high - emission and C validity change.
> **Lenses**: codegen, C99, volatile semantics, ownership (unchanged)
> **Verify**: focused suites green; integrity mutations reproduce; strict
> builds warning-free; `make check` green.

- [x] 2.1.1 Add `ArrayTarget::is_volatile`, propagate it through `element`,
      and compute it in `array_target` from a new `fill_target_volatile`
      predicate that mirrors `place()` for `.len` and walks `Raw` layers.
- [x] 2.2.1 Add `emit_volatile_array_copy` and branch the nested fill row copy
      on `target.is_volatile`; guard the compact `memset` branch with
      `!target.is_volatile`.
- [x] 2.2.2 Add `emit_prefix_row_assign_to` and call it from the nested fill
      element loop. A stable storage value copies leaf by leaf directly; an
      impure value is evaluated once into a row temporary with the shared
      core, then copied. Do not change the shared final branch.
- [x] 2.2.3 In the `StorePointer` fixed-array branch, derive the volatility
      from the addressed value tree and set it on both target constructions.
- [x] 2.3.1 Run both focused suites; all green.
- [x] 2.4.1 Run the integrity mutations (revert each copy branch, remove the
      prefix wrapper, force the predicate false, force it true, remove the
      `.len` handling, remove the recursion, treat an impure prefix as
      stable, disable the runtime-index volatility, reject a constant index
      as stable) and record each failure;
      restore.
- [x] 2.5.1 Add the volatile global and overlay-field shapes to
      `tools/perf/fixtures/shapes.clo`, call the overlay helper from `main`,
      and run `make bench-shape`.
- [x] 2.6.1 Run `make check`.
- [x] 2.7.1 Review `git diff --cached`; commit
      `fix(codegen): copy volatile nested fills element by element`;
      confirm a clean tree.

**Verify**: focused green; mutations recorded; `bench-shape` and `make check`
green; commit exists.

---

## Phase 3: Measurement, Audit, Reviews, Sanitizers

> **Reasoning**: high - the no-regression and C-validity claims are decided
> here.
> **Verify**: no scored regression against `7cdbb71`; no non-volatile
> emission diff; no open critical/major finding; sanitizers green.

- [x] 3.1.1 Run `make bench-base COMMIT=7cdbb71`; record the base compiler
      path, SHA-256, and `dirty=false`.
- [x] 3.1.2 Run `make bench`; record the head SHA-256 and `dirty=false`, then
      run `make bench-compare` with no `REQUIRE` (the 1% guard).
- [x] 3.2.1 Emit base and head C over the corpus, benchmarks, and Amiga
      examples with the `7cdbb71` base compiler; diff and classify every
      hunk.
- [x] 3.3.1 Dispatch `correctness-reviewer` on the codegen diff.
- [x] 3.4.1 Dispatch read-only `memory-safety-auditor` on the volatile shapes.
- [x] 3.5.1 Dispatch read-only `emission-integrity-auditor` on the diff and
      strict builds.
- [x] 3.6.1 Dispatch read-only `performance-auditor` on the non-volatile path
      and the benchmark comparison.
- [x] 3.7.1 Merge findings; resolve all critical/major. The run-time index,
      impure prefix, and doc findings were fixed in the second code commit;
      rerun 2.3, 3.1, and 3.2 on the new clean commit and re-review the fix
      diff once.
- [x] 3.8.1 Run `make asan` and `make ubsan`; record the suite counts.
- [x] 3.9.1 Fill the findings ledger; commit
      `docs(plans): record the volatile nested fill review`.

**Verify**: reviews and sanitizers recorded; no blocking finding.

---

## Phase 4: Closeout and Fix Gates

> **Reasoning**: medium - mechanical closeout; the full gate is the
> acceptance.
> **Verify**: all fix gates green on the committed closeout tree; tree clean;
> `origin/main..HEAD` zero.

- [x] 4.1.1 Fill the final acceptance table and the accepted limits in
      [Findings](05-findings.md).
- [x] 4.2.1 Mark PLAN-23 `Done`; add the PLAN-24 release row as `Executing`;
      update the header (567/570, 1 open,
      `Active - v1.1.1 release closeout`); add the fixed statement and the
      remaining limitations to `docs/known-limitations.md`.
- [x] 4.3.1 Review `git diff --cached`; commit
      `docs(plans): close the volatile nested fill fix`; confirm a clean tree.
- [x] 4.4.1 Run `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
      `make differential`, `make bench-shape`, `make bench-base
      COMMIT=7cdbb71`, `make bench`, `make bench-compare`, and `make verify`
      on the clean closeout commit; record each final line.
- [x] 4.5.1 Push `main`; confirm a clean tree and
      `git rev-list --count origin/main..HEAD` is 0.

**Verify**: every fix gate green; pushed; tree clean.

---

## Phase 5: v1.1.1 Release (PLAN-24)

> **Row mapping**: Phases 0-4 deliver PLAN-23 (R1-R4). Phase 5 is the checklist
> for the separate PLAN-24 roadmap row and follows `RELEASE.md`; it does not
> add a PLAN-23 requirement. Phase 6 reports both rows.
> **Reasoning**: high - the version of record, the notes, the artifacts, and
> the public release are created here.
> **Verify**: versions match; every release gate green; artifacts checksummed
> and audited; publication explicitly authorized; tag and release published.

- [ ] 5.1.1 Set `compiler/Cargo.toml` to `1.1.1`; mirror it in `Cargo.lock`;
      bump `tools/vscode/package.json` and the root package entry in
      `tools/vscode/package-lock.json` to `1.1.1`. There is no root
      `package-lock.json`.
- [ ] 5.2.1 Add the `1.1.1` `CHANGELOG.md` entry and replace
      `RELEASE-NOTES.md` with the v1.1.1 notes.
- [ ] 5.2.2 Rewrite `RELEASE.md` as the v1.1.1 runbook with the gate-evidence
      table, licence/metadata audit, exact artifact names, conditional
      publication steps, and the historical v1.1.0 section; update the
      `README.md` current-release line and the
      `docs/known-limitations.md` release-packaging line.
- [ ] 5.2.3 Update the PLAN-24 row with the release facts.
- [ ] 5.3.1 Review `git diff --cached`; commit
      `chore(release): prepare v1.1.1`; confirm a clean tree.
- [ ] 5.4.1 Run the release gate set on the clean release commit:
      `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
      `make differential`, `make bench-shape`, `make bench-base
      COMMIT=7cdbb71`, `make bench`, `make bench-compare`, `make verify`, and
      `make vscode-check`.
- [ ] 5.4.2 Fill the runbook gate-evidence table and licence/metadata audit
      from the observed results.
- [ ] 5.4.3 Build the release binaries; assert `clc 1.1.1` and
      `clorraine 1.1.1`; compile and run a hello-world; confirm Linux x86_64
      ELF.
- [ ] 5.5.1 Create the annotated `v1.1.1` tag on the release commit.
- [ ] 5.5.2 Build `build/release/v1.1.1/`: the source tarball from the tag
      (`clorraine-1.1.1-src.tar.gz`), the Linux x86_64 tarball
      (`clorraine-1.1.1-linux-x86_64.tar.gz`), and `SHA256SUMS`; audit the
      contents for NDK, ROM, Workbench media, `build/`, `target/`, and local
      environment files.
- [ ] 5.6.1 Ask the user for explicit publication authorization in this
      session. If it is granted, push `main` and the tag and create the
      GitHub release with
      `gh release create v1.1.1 --title "Clorraine v1.1.1" --notes-file RELEASE-NOTES.md`
      and the three assets; verify the page and checksums. If it is not
      granted, keep the tag local, record the artifact hashes, and stop with
      the local release complete and publication blocked.
- [ ] 5.7.1 Record the publication status and asset hashes in `RELEASE.md`;
      mark PLAN-24 `Done`; set the header to 568/570, 0 open, `Done`.
- [ ] 5.7.2 Commit `docs(release): record the v1.1.1 publication`; push;
      confirm a clean tree and `origin/main..HEAD` zero.

**Verify**: release published; publication record committed and pushed.

---

## Phase 6: Final Report

- [ ] 6.1.1 Report the fix, the emitted C, the tests, the review verdicts,
      the fix gates, the release gates, the artifact hashes, and the
      publication URL.

**Verify**: every acceptance item is reported.

---

## Commit Plan

| Order | Message | Content |
|---|---|---|
| 1 | `docs(plans): add the volatile nested fill plan` | Plan set, preflight, PLAN-23 row |
| 2 | `fix(codegen): copy volatile nested fills element by element` | Predicate, three copy sites, tests, fixture |
| 2b | `fix(codegen): evaluate impure volatile rows once` | Run-time indexed targets, impure prefix temporary, extra pins |
| 3 | `docs(plans): record the volatile nested fill review` | Red/green, benchmark, audit, reviews, sanitizers |
| 4 | `docs(plans): close the volatile nested fill fix` | Findings, roadmap, known limitations |
| 5 | `chore(release): prepare v1.1.1` | Version, changelog, notes, runbook, README, PLAN-24 |
| 6 | `docs(release): record the v1.1.1 publication` | Publication status, asset hashes, PLAN-24 Done |

No push before the Phase 4 full gate. The release tag is created only after
the Phase 5 release gate.

## Risks and Controls

| Risk | Control |
|---|---|
| A volatile shape is missed | The predicate covers global bases and volatile fields; the two nested fill copy sites are both changed; ST/IP cases cover both, a prefix place, a field named `len`, and a 3D shape |
| Non-volatile emission changes | The flag defaults false; IP-V7/IP-V8 guards pin the old spelling |
| The prefix-row branch changes a non-fill copy | It only triggers on `target.is_volatile`; the non-volatile branch is byte-identical |
| The 3D recursion is wrong | ST-V2 and IP-V2 pin the leaf indices |
| The benchmark base includes the prior optimization | The base is rebuilt at `7cdbb71`, the fix's parent; no `REQUIRE` is used |
| The release ships a dirty tree | The tag is created after the release gates on the committed release tree |
| The artifacts contain media or generated output | Content audit before publication |
