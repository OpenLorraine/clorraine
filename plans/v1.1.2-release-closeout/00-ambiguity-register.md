# Ambiguity Register: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design active — eligible technical decisions delegated under
> policy version 1; the release version, publication, and product claims
> remain user-owned
> **Root invocation ID**: `v112-release-20261110`
> **Policy version**: 1

Auto-design active — eligible technical decisions are delegated and recorded.
The user chose the goal (v1.1.2 release closeout) and authorized publication
when every gate is green.

## Authority Boundary

| Area | Owner | Evidence |
|------|-------|----------|
| Ship a v1.1.2 release containing the landed post-v1.1.1 work | User | User selected "v1.1.2 release closeout" on 2026-11-10 |
| Publish the tag and GitHub release when green | User | User answered "Publish when green" on 2026-11-10 |
| Choose version surfaces, gate order, artifact layout, and record shape | AI — delegated | `--auto-design`, policy version 1, release-engineering mechanisms |
| Change the language, ABI, spec, or stdlib for this release | User — not authorized | A release closeout ships the existing tree; no new product surface |
| Bump the major/minor version or publish a second artifact target | User — not authorized | The chosen option names v1.1.2; aarch64 remains deferred |

## Decision Records

### AR-01 — Goal and scope

- **Status**: ✅ Resolved
- **Authority**: User — selected the v1.1.2 release closeout.
- **Objective**: deliver the post-v1.1.1 product improvements to users as a
  published patch release.
- **Decision**: the release ships every commit since `v1.1.1` that affects the
  shipped product: the constant-array codegen change (PLAN-25), the release
  flag/DCE/strip change (PLAN-26), and the symbol-shadowing fix that landed
  with PLAN-25. One behavior-neutral compiler cleanup (`902ef30`) also ships.
  No new language, ABI, spec, stdlib, or feature work is added by the
  closeout.
- **Evidence**: `git log v1.1.1..HEAD` (31 commits); `CHANGELOG.md` ends at
  1.1.1; `README.md:54` names v1.1.1 as current; `v1.1.1` tag is `2d1eb30`.
- **Rejected alternatives**: ship only PLAN-26 (hides the constant-array win);
  bundle a new feature (scope expansion); leave the wins unreleased (the user
  chose a release).
- **Strongest counterargument**: a release with no language change is mostly
  packaging. The two performance wins are user-visible and measurable; the
  release is how they reach users.
- **Confidence**: High.
- **Hardening**: the release preflight and the final correctness review check
  the scope and claims.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user adds a feature to the release; a gate shows a
  regression that must be fixed first.

### AR-02 — Version number

- **Status**: ✅ Resolved
- **Authority**: User — the chosen option names v1.1.2.
- **Decision**: version `1.1.2` (patch). No major/minor change.
- **Evidence**: the language and public CLI are unchanged; the release changes
  build/link flags and emitted data layout. Semver patch is the smallest
  truthful bump. `compiler/Cargo.toml:5` is the version of record.
- **Rejected alternatives**: `1.2.0` (no new capability); `1.1.1` re-tag
  (already published, immutable); `2.0.0` (no breaking change).
- **Strongest counterargument**: stripped release programs and section GC are
  a packaging behavior change; a minor bump could signal it. The CLI and
  language are unchanged, and the runbook treats packaging as patch-level.
- **Confidence**: High.
- **Hardening**: the release checklist asserts every version surface.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user asks for a different version.

### AR-03 — Release contents, claims, and measurement bases

- **Status**: ✅ Resolved
- **Authority**: AI — delegated release-note mechanism; measured claims are
  user-owned and must be exact.
- **Decision**: the changelog and release notes name exactly three changes:
  - **Changed** — module-level constant scalar array initializers emit one
    read-only data template plus a copy (PLAN-25).
  - **Changed** — release programs on Linux host, `hosted-amiga`, and
    `hosted-amiga-13` use `-O2 -ffunction-sections -fdata-sections
    -Wl,--gc-sections -s`; the frozen compile matrix's two GCC release rows
    match.
  - **Changed** — the constant-array template uses a collision-free symbol and
    no longer shadows a user `ffi` symbol; the collision existed only in
    unreleased builds.
  The notes use three separate measurement bases and never mix them:

  | Basis | Base | Head | Delta |
  |---|---|---|---|
  | Isolated emission, old flags (PLAN-25 evidence) | 13,653 net / 117,440 `.text` | 9,470 / 92,976 | −30.6% / −20.8% |
  | Release tree, shipped flags (`v1.1.1` → v1.1.2 head) | 13,653 net / 64,328 `.text` / 82,544 bin | 9,470 / 39,904 / 41,740 | −30.6% / −38.0% / −49.4% |
  | Same compiler, old vs shipped flags (`make bench-flags`) | 92,976 `.text` / 116,584 bin | 39,904 / 41,740 | −57.1% / −64.2% |

  The flag audit's five `hosted-amiga` scored benchmarks (sieve, matrix,
  strings, moves, lookup) are `.text` 57.1–69.3% and stripped binaries
  64.2–74.7% smaller. Host release binaries fall
  20.6–36.5%; the 1.3 audit set (calibration, sieve, and the four corpus
  programs) falls 49.1–88.9% in `.text`. Calibration is 3,372 → 2,928
  instructions on hosted-amiga and 3,348 → 2,947 on 1.3. Scored net instructions are unchanged
  by the flags; the constant-array change improves `lookup` net by 30.6%.
- **Evidence**: `plans/constant-array-initializer/05-findings.md`;
  `plans/performance-levers-closeout/05-findings.md`; the preflight built
  `v1.1.1` from the tag and measured the release-tree row.
- **Rejected alternatives**: one mixed table (misattributes the flag win to
  `v1.1.1`); a cumulative `v1.1.0` claim (double-counts the previous release);
  claim a runtime win from the flags (the scored net is unchanged).
- **Strongest counterargument**: three bases are more complex than one
  headline. The notes use the release-tree comparison as the headline
  (binaries 49.4% smaller, `.text` 38.0% smaller on `lookup`) and keep the
  same-compiler audit (five scored: `.text` 57.1–69.3%, binaries
  64.2–74.7%; 1.3 audit set 49.1–88.9%) as separate supporting evidence.
- **Confidence**: High.
- **Hardening**: Phase 1 re-measures the release-tree row on a clean tree; the
  correctness review checks every claim against the findings.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: a re-measurement differs materially from the recorded
  numbers.

### AR-04 — Comparison base and gate expectations

- **Status**: ✅ Resolved
- **Authority**: AI — delegated measurement mechanism.
- **Decision**: use `v1.1.1` (`2d1eb302…`) as the scored `bench-base` commit.
  Both sides are measured by the current harness with the shipped flags, so
  the comparison isolates emission; the flag win is carried by
  `make bench-flags`. The declared gate is
  `REQUIRE="lookup:instructions:20 lookup:text:20"`; the measured values are
  −30.6% net and −38.0% `.text`, so the floor has margin. No scored benchmark
  may be more than 1% worse.
- **Evidence**: `v1.1.1` is the previous published release; the preflight
  built it and confirmed it compiles the current `lookup` benchmark; the
  harness builds both sides with the shipped flags.
- **Rejected alternatives**: `v1.1.0` (two releases back); current `main`
  pre-release (not a published baseline); no base (cannot prove no
  regression); a 10% floor (a weaker gate with more slack than the measured
  30.6/38.0 result).
- **Strongest counterargument**: the harness flags differ from how v1.1.1
  actually shipped. The findings pair the emission comparison with the
  separate same-compiler flag audit so both attributions are explicit.
- **Confidence**: High.
- **Hardening**: the release findings record both comparisons and the exact
  `REQUIRE` string.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: `v1.1.1` cannot build the current benchmark set; a
  floor is missed.

### AR-05 — Version surfaces

- **Status**: ✅ Resolved
- **Authority**: AI — delegated release mechanism; follows `RELEASE.md`.
- **Decision**: bump `compiler/Cargo.toml`, regenerate `Cargo.lock`, bump
  `tools/vscode/package.json` and `package-lock.json`. Leave
  `tools/demo/package.json` at its own package version. Update `README.md`'s
  current-release line. `clc --version` and `clorraine --version` must print
  `1.1.2`.
- **Evidence**: `RELEASE.md:23-26`; the v1.1.1 runbook's audit table;
  `tools/vscode/test/manifest.test.ts` and `packaged.test.ts` compare the
  extension version to `compiler/Cargo.toml`, so the extension bump is
  enforced by tests.
- **Rejected alternatives**: bump only `Cargo.toml` (the extension and lock
  drift); bump the demo packages (they are separate packages).
- **Strongest counterargument**: the VS Code extension did not change. It
  ships from the same repository and the runbook keeps its version aligned.
- **Confidence**: High.
- **Hardening**: the checklist asserts each surface and both `--version`
  outputs.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: a version surface is added or removed.

### AR-06 — Documentation policy: current vs historical

- **Status**: ✅ Resolved
- **Authority**: AI — delegated documentation mechanism.
- **Decision**: update statements that describe the *current* release
  (`README.md` status line, `RELEASE.md` decision/publication status, release
  notes, changelog). Preserve historical statements that record when a fix
  landed (`docs/known-limitations.md` "fixed in v1.1.1") and the historical
  publication sections for v1.1.0 and v1.0.0. Do not rewrite history. Restate
  the LSP packaging exclusion generically so it is not tied to v1.1.1.
- **Evidence**: `docs/known-limitations.md:146,163` describe v1.1.1 fixes;
  `RELEASE.md:142-156` holds the v1.1.0 and v1.0.0 historical sections;
  `docs/known-limitations.md:225` names the v1.1.1 tarball.
- **Rejected alternatives**: rewrite every version mention (falsifies history);
  leave the current-release line at v1.1.1 (stale).
- **Strongest counterargument**: a reader may see both versions and be
  confused. The headings ("Historical publication") and the tense distinguish
  them.
- **Confidence**: High.
- **Hardening**: `./spec/check.sh` and the docs guide test; the correctness
  review checks the current/historical split.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: a doc owner wants a different policy.

### AR-07 — Pre-release gates

- **Status**: ✅ Resolved
- **Authority**: AI — delegated verification mechanism; the full gate is the
  user's standing requirement.
- **Decision**: run on the gated release commit:
  `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
  `make differential`, `make bench-shape`, `make bench-base COMMIT=v1.1.1`,
  `make bench`, `make bench-compare REQUIRE="lookup:instructions:20
  lookup:text:20"`, `make bench-flags`, `make verify`, and `make vscode-check`.
  The release Workbench/services gates are inside `make verify`. The
  top-level `make` targets run on the host and enter the container
  internally; `make vscode-check` needs Node on `PATH`, and `gh` runs on the
  host. Before any
  measurement, delete stale `build/perf/{base,head,flags}.json`, require
  `git status --porcelain` empty, and assert the recorded identities
  (`base.json.compiler.git == 2d1eb30…`, `head.json.compiler.git ==` the gated
  commit, both `dirty: false`; `flags.json.clc.git ==` the gated commit,
  `clc.dirty: false`, `clc.version == "clc 1.1.2"`,
  `clc.sha256 == head.json.compiler.sha256`, `runs: 2`).
  After `make verify`, check `git status` because the RTG gate regenerates
  tracked PNGs; restore an unintended change before the evidence commit, and
  investigate a material capture difference instead of committing it.
- **Evidence**: `RELEASE.md:34-52`; `AGENTS.md`; `Makefile:119-120,217-218`;
  `flag_audit.py` records `dirty` but does not enforce it; `rtg-smoke.sh:320`
  writes tracked captures.
- **Rejected alternatives**: a subset (a release is the one place the full
  gate is mandatory); omit `bench-flags` (this release changes the shipped
  flags); omit `vscode-check` (the runbook requires it).
- **Strongest counterargument**: the full gate is expensive. It is the release
  acceptance gate and has caught target-only defects.
- **Confidence**: High.
- **Hardening**: the findings record every gate result on the gated commit and
  the docs-only diff to the tag.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: a gate is unavailable; the user narrows the gate.

### AR-08 — Artifacts and packaging

- **Status**: ✅ Resolved
- **Authority**: AI — delegated packaging mechanism; publication is
  user-authorized.
- **Decision**: build in the pinned container with
  `cargo build --release --workspace`. Write under
  `build/release/v1.1.2/`:
  `clorraine-1.1.2-src.tar.gz` via
  `git archive --format=tar.gz --prefix=clorraine-1.1.2/ -o … v1.1.2`
  (the tag snapshot; publication status is recorded in the repository and
  release page, not inside the tag), and
  `clorraine-1.1.2-linux-x86_64.tar.gz` from a staging tree
  `clorraine-1.1.2/` containing `clc`, `clorraine`, `LICENSE`, `README.md`,
  `RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, and
  `docs/known-limitations.md`; then
  `sha256sum *.tar.gz > SHA256SUMS` and `sha256sum -c SHA256SUMS`.
  Assert the versions, a hello-world run, and Linux x86_64 ELF. Audit with an
  exact path allowlist: forbid `docker/.env`, `docker/compose.override.yaml`,
  any `build/` or `target/` output, NDK, ROM, and Workbench content; allow the
  committed `docker/.env.example` and `docker/compose.override.yaml.example`. aarch64 stays deferred;
  the LSP stays out of the binary tarball.
- **Evidence**: `RELEASE.md:96-117`; the v1.1.1 asset table; the v1.1.0
  tarball listing; `docs/known-limitations.md:225` records the LSP exclusion.
  Adding the notices and limitations file fixes a broken licensing link in the
  v1.1.1 binary tarball.
- **Rejected alternatives**: keep the six-file list (the shipped README and
  notes link a missing notices file); add `clorraine-lsp` (the established
  exclusion); ship aarch64 (deferred); publish uncompressed binaries (no
  integrity file).
- **Strongest counterargument**: adding two docs files changes the artifact
  shape. It makes the shipped licensing references resolvable and is recorded
  as a deliberate change.
- **Confidence**: High.
- **Hardening**: the archive audit and checksums; the correctness review
  checks the asset list.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user authorizes an additional artifact target.

### AR-09 — Tag placement and commit sequence

- **Status**: ✅ Resolved
- **Authority**: AI — delegated release mechanism, improving on v1.1.1.
- **Decision**: use four commits and a tag:
  0. `docs(plans): add the v1.1.2 release plan` — the plan folder and roadmap
     row.
  1. `chore(release): bump the version to 1.1.2` — version surfaces only.
  2. Run every gate on that commit; the measurement JSON names it.
  3. `docs(release): record the v1.1.2 gates` — changelog, release notes,
     runbook, findings, and plan. This is docs-only relative to the gated
     commit.
  4. Create the annotated tag `v1.1.2` locally on the evidence commit, build
     and audit the artifacts, then push `main` and the tag.
  5. Publish as a draft, verify the assets, then publish.
  6. `docs(release): record the v1.1.2 publication` — URL, sizes, hashes,
     status; then close the plan.
  The tag's diff from the gated commit is documentation-only; the findings
  record the gated and tagged hashes.
- **Evidence**: `v1.1.1` tag object points at `2d1eb30` "record the v1.1.1
  release gates" while its measured head is `2bc279e`; its tag was pushed
  only in the publication phase. The new sequence keeps the tag local until
  the release build and archive audit pass, and writes the changelog/notes
  after the measurement.
- **Rejected alternatives**: put the changelog in the bump commit (its
  measured values would precede the measurement); tag the bump commit (the
  source snapshot lacks the notes); tag after the publication commit (the
  source tarball would contain its own publication record); lightweight tag
  (the runbook uses annotated).
- **Strongest counterargument**: keeping the tag local until after the
  artifact audit delays publication by one build. That is the point: a bad
  artifact must not be frozen into a pushed tag.
- **Confidence**: High.
- **Hardening**: the checklist records the four commit hashes, the tag
  object, and the tag's parent.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: publication fails; the tag must be moved before it is
  pushed (never after).

### AR-10 — Publication

- **Status**: ✅ Resolved
- **Authority**: **User** — "Publish when green" on 2026-11-10.
- **Decision**: after every gate is green and the artifacts are built and
  audited, push `main` and the tag, then create the GitHub release as a
  **draft** with `RELEASE-NOTES.md` as the body. Upload both tarballs and
  `SHA256SUMS`, verify the asset names and `isDraft` state with
  `gh release view v1.1.2 --json isDraft,assets`, then publish with
  `gh release edit v1.1.2 --draft=false`. A partial upload is retried with
  `gh release upload`; an existing asset is never replaced silently. Record
  the URL, asset sizes, and SHA-256 values.
- **Evidence**: the v1.1.1 publication record (`RELEASE.md:28-32,119-136`);
  `gh release create` otherwise publishes before assets are uploaded;
  `docker/scripts/bench-base.sh` removes its worktree and retains only
  `build/perf/base-clc`.
- **Rejected alternatives**: publish immediately (a failed upload leaves a
  public release with missing assets); local-only release (the user chose
  publish); publish before the gates (forbidden).
- **Strongest counterargument**: publication is irreversible. The user
  explicitly authorized it, and the gates run first.
- **Confidence**: High.
- **Hardening**: the draft/verify/publish sequence and the recorded failure
  path.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user withdraws authorization; the GitHub release
  fails.

### AR-11 — No new product surface; the shipped diff is already reviewed

- **Status**: ✅ Resolved
- **Authority**: User — the goal is a release closeout, not a feature.
- **Decision**: the **closeout diff** (the commits made by this plan) changes
  only version metadata, release docs, and plan records. The **release diff**
  (the 31 commits since v1.1.1) ships the already-reviewed PLAN-25 and PLAN-26
  compiler and spec changes, plus one behavior-neutral compiler cleanup. No
  new language, grammar, diagnostic, ABI, stdlib, runtime-shim, or CLI
  behavior is added. The two spec rows changed by PLAN-26 were separately
  authorized (PLAN-26 AR-19) and ship as-is.
- **Evidence**: PLAN-25 and PLAN-26 phase reviews and gate evidence;
  `spec/check.sh` green; the closeout file list in the index.
- **Rejected alternatives**: add a last-minute fix (scope creep); revert the
  flags (defeats the release's main win).
- **Strongest counterargument**: a release is a good time to land small fixes.
  Without a separate authorized goal, they would bypass the normal gates.
- **Confidence**: High.
- **Hardening**: the plan preflight and final review compare the closeout diff
  against the allowed files and re-cite the shipped changes' review evidence.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user authorizes a specific fix.

### AR-12 — Failure handling

- **Status**: ✅ Resolved
- **Authority**: AI — delegated procedural handling of the user's "green
  before publish" rule.
- **Decision**:
  - A failed gate stops the release; fix and rerun the affected gate plus
    `make verify`.
  - A flaky environmental gate (FS-UAE/display, licensed media, container
    startup, or network) may be rerun once with the failing leg and output
    recorded; a second failure stops. A compiler or test failure is never
    treated as flaky.
  - A version mismatch is fixed at the source and `Cargo.lock` regenerated.
  - A dirty tree at measurement time stops the measurement; clean or commit,
    then rerun.
  - `v1.1.2` exists locally but is not on `origin`: delete and recreate after
    confirming the remote tag is absent. It exists on `origin`: stop; never
    force-move a published tag.
  - The release build fails after the local tag: fix and recreate the local
    tag before it is pushed. After the tag is pushed, fix the environment and
    rebuild from the same tag; never move it.
  - A gate fails: an environmental or tooling fix may be applied; a compiler
    or test failure stops the release and needs a user ruling (AR-11).
  - An existing draft release is resumed with `gh release upload` and
    `gh release edit`; an existing published release stops the release.
  - A `gh auth` or permission failure is recorded as a blocker; no asset is
    uploaded anonymously.
  - A push is rejected because `origin` advanced: stop; do not rebase the
    measured commit.
  - The `bench-base` worktree race is retried once with
    `python3 tools/perf/bench.py run --clc build/perf/base-clc
    --compiler-git 2d1eb302… --label base --out build/perf/base.json`;
    the commit identity must be unchanged. The worktree is temporary; only
    `base-clc` is retained.
  - A failed publication is recorded in `RELEASE.md` with the exact blocker;
    a partial upload is retried with `gh release upload`, and no asset is
    replaced silently.
- **Evidence**: `RELEASE.md:158-166`; PLAN-25 findings record the worktree
  race.
- **Rejected alternatives**: lower a gate; publish with a known failure
  (violates the authorization condition).
- **Strongest counterargument**: a flaky Amiga emulator gate can delay a
  release. One audited rerun is the documented compromise.
- **Confidence**: High.
- **Hardening**: the findings record every rerun and result.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: a gate flakes twice; publication is partially complete.

### AR-13 — Review dispatch

- **Status**: ✅ Resolved
- **Authority**: AI — delegated dispatch.
- **Decision**: dispatch the correctness reviewer for the version/docs/claims
  phase and the final release tree. The reviewer checks the fresh
  `bench-compare` output against the cited PLAN-25/PLAN-26 evidence. The
  emission-integrity auditor is not dispatched because the closeout diff
  changes no compiler source; the shipped codegen changes were reviewed in
  PLAN-25/PLAN-26. A memory-safety review is not applicable. No new specialist
  is warranted.
- **Evidence**: `AGENTS.md` phase dispatch; the closeout file list.
- **Rejected alternatives**: full codegen review (no codegen change in the
  closeout); memory-safety audit (no ownership change); skip review (the
  release claims and version surfaces need an independent check).
- **Strongest counterargument**: the release reports performance results; an
  independent check of the numbers is valuable. The correctness reviewer
  performs that check.
- **Confidence**: High.
- **Hardening**: the review packet requires a claim-by-claim evidence check.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the closeout diff grows to include compiler code.

### AR-14 — aarch64 and other artifacts

- **Status**: ✅ Resolved
- **Authority**: User — v1.1.1 deferred aarch64; this release does not
  broaden the product.
- **Decision**: ship only the Linux x86_64 binary tarball and the source
  tarball. aarch64 host builds stay supported from source.
- **Evidence**: `RELEASE.md:21`; the v1.1.1 decision.
- **Rejected alternatives**: add aarch64 now (untested release artifact);
  drop the binary tarball (regression from v1.1.1).
- **Strongest counterargument**: aarch64 users would benefit. It needs its own
  release gate and is not authorized.
- **Confidence**: High.
- **Hardening**: the artifact audit asserts exactly two tarballs plus sums.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user authorizes an aarch64 artifact.

### AR-15 — Branch and commit policy

- **Status**: ✅ Resolved
- **Authority**: User — the standing mode is `--auto-commit` with push when
  green.
- **Decision**: commit on `main` in the sequence AR-09. Do not push C1 or C2
  on their own; push `main` and the tag at 3.1.11, after the full gate, the
  release build, and the archive audit. Push C3 at 4.1.7. Keep `main` equal to
  `origin/main` at closeout. Never rewrite the published `v1.1.1` history.
- **Evidence**: the prior goal's standing mode; the repository uses `main` as
  the integration branch.
- **Rejected alternatives**: a release branch (adds merge ceremony for a
  linear release); force-push (forbidden).
- **Strongest counterargument**: direct-to-main gives no review window. The
  phase reviews and gates are the review.
- **Confidence**: High.
- **Hardening**: the final `git status` and tag checks.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user asks for a release branch.

### AR-16 — Release record and roadmap

- **Status**: ✅ Resolved
- **Authority**: AI — delegated record mechanism.
- **Decision**: add `plans/v1.1.2-release-closeout/` with requirements,
  register, design, testing, findings, and execution plan; add PLAN-27 to the
  roadmap and close it as Done with **571/573 rows done, 0 open, 2 parked**.
  Refresh `codeops/specialist-check.json` and the `AGENTS.md` coverage count
  (74 plan folders) at closeout. `RELEASE.md` becomes the v1.1.2 runbook and
  records the publication; v1.1.1 moves to a historical section after
  publication.
- **Evidence**: PLAN-24 was a roadmap row pointing at `RELEASE.md`; recent
  feature plans use a full folder. `AGENTS.md:192` currently says 73 folders;
  the new plan makes 74.
- **Rejected alternatives**: reuse `RELEASE.md` only (loses the requirements
  and decision trail); no roadmap row (breaks the tracker).
- **Strongest counterargument**: a release folder duplicates the runbook. The
  runbook owns the procedure; the plan owns the decisions and evidence.
- **Confidence**: High.
- **Hardening**: the roadmap review at closeout.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the user prefers runbook-only records.

### AR-17 — Evidence ownership and boundaries

- **Status**: ✅ Resolved
- **Authority**: AI — delegated record mechanism.
- **Decision**: `05-findings.md` is the full evidence ledger; `RELEASE.md` is
  the release summary and publication record. Before the tag, `RELEASE.md`
  and the findings hold the gate results, measured values, and the gated/tag
  commit hashes. After publication, the post-tag commit adds the URL, asset
  names, sizes, SHA-256 values, and final status. The source tarball is the
  pre-publication tag snapshot; the publication record lives in the repository
  and the release page.
- **Evidence**: the v1.1.1 split between `2d1eb30` (tag) and `a8f49b8`
  (publication record).
- **Rejected alternatives**: put hashes in the tag (the artifacts are built
  after the tag, so they cannot be known); duplicate full gate tables in both
  docs (drift).
- **Strongest counterargument**: a reader of the source tarball sees a
  pre-publication runbook. The release page is the authoritative publication
  record.
- **Confidence**: High.
- **Hardening**: the final review checks both documents agree.
- **Policy version**: 1
- **Root invocation ID**: `v112-release-20261110`
- **Reopen triggers**: the release process changes.

## Gate

**✅ GATE PASSED** — every material decision is resolved. Publication has the
user's explicit 2026-11-10 authorization; version and scope are user-chosen;
the remaining entries are eligible delegated release-engineering decisions
under policy version 1. No material item is open.
