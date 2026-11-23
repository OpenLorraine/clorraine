# Clorraine v1.2.0 native Linux delivery

> **Status**: Superseded — 2026-11-23. Unpublished; replaced by the cross-platform
> CLI and FS-UAE delivery plan.
> **Created**: 2026-11-16
> **Implements**: — (standalone user-requested feature)
> **Roadmap**: PLAN-45
> **CodeOps Artifact Schema**: 1

> **Closure.** The user redirected the product goal to a CLI-first workflow that
> works on Windows, Linux and macOS and runs programs in FS-UAE. The upstream
> AmigaPorts toolchain is prebuilt for all three platforms, so the Python
> installer, support pipeline, audits, self-hosted runner and promotion
> workflows from this plan are no longer the chosen design. No asset from this
> plan was published. Reusable pieces (the FS-UAE driver path, discovery, editor
> actions, language server) carry into the new plan. See AR-50.

Deliver the confirmed Linux installation and editing/build/run workflow. The
approved decisions are in the [register](00-ambiguity-register.md), AR-12–29.
The first implementation phase proves support packaging on clean machines
before installer or release automation depends on it.

## Minimum-sufficient design

**Original goal:** Prebuild reusable support software in GitHub Actions and
download it through an installer, so users can develop Clorraine programs
without compiling installation dependencies.

**Smallest viable design:** Keep source locks, packaging, shell/Python
installation, driver integration, the existing VS Code extension and two
manually dispatched workflows in this repository. Prove archives manually,
then automate the same commands. Use system Ubuntu prerequisites and verified
local wheels rather than bundling another host C compiler or Python runtime.

**Approved support surface:** AR-17/22/27/28: two workflows, versioned release
assets, a shell entry with standard-library Python installer logic, pinned
prebuilt support/helper binaries and wheels, and local NDK preparation.
Independent challenge: **Simplify**, followed by direct user approval.

**Previously considered larger machinery:** QtIFW/Pages distribution, a channel
service, mirrors, a new signature verifier and a rollback manager are not part
of the approved implementation.

## Documents

| Document | Owns |
|---|---|
| [00-ambiguity-register](00-ambiguity-register.md) | User decisions and authority |
| [01-requirements](01-requirements.md) | Product requirements and acceptance scope |
| [02-current-state](02-current-state.md) | Observed code, dependencies and gaps |
| [03-01-support-builds](03-01-support-builds.md) | Source lock, support payloads and clean-machine proof |
| [03-02-installation](03-02-installation.md) | Manifest, bootstrap, installation and update state |
| [03-03-native-driver](03-03-native-driver.md) | SDK lookup, native commands and FS-UAE execution |
| [03-04-editor-and-release](03-04-editor-and-release.md) | VS Code actions, workflows and publication |
| [07-testing-strategy](07-testing-strategy.md) | Immutable specification cases and evidence |
| [99-execution-plan](99-execution-plan.md) | Sole task-progress authority |

## Scope and quality

Planning is complete only after the document checks; execution readiness also
requires the requested preflight. No implementation task or public release is
authorised by this plan's existence. See AR-22/25.

Authoring checks: 10 required documents present; local document links resolve;
60 distinct specification cases; 89 distinct unstarted tasks, ordered
specification tests → red → implementation → green → hardening in six phases.
These are structural checks, not a preflight pass.

Execution amendment AR-32 separates reference production from final lock
validation: that checklist had 90 tasks. The initial 89-task authoring
and preflight counts above describe their recorded historical baseline.

The subsequent AR-34 ruling separates measured reference proof from final
lock completion after its hashed producers exist, for 91 current tasks.
The eight previously verified task IDs and all acceptance gates are unchanged.

Selected domain lenses:
- **Compiler/language:** driver interfaces, existing profile semantics, C ABI
  flags, emitted-code conformance and LSP/editor integration.
- **Data/migration:** manifest schema, component identity, install activation,
  Python-version changes and coexistence of old/new installations.
- **Distributed/concurrent:** independent release jobs, partial publication,
  concurrent installers, running builds during activation, and emulator lifetime.

Review add-ons: security, API surface and concurrency. Authentication servers,
web sessions, CSRF and rate-limited public APIs are not involved. Trust boundaries
are HTTPS artifacts, archive contents, local processes, workspace settings,
GitHub publication credentials and licensed media.

## Specialist agents

| Role | Kind | Use | Authority |
|---|---|---|---|
| `amiga-platform-reviewer` | Existing read-only reviewer | Support header/runtime proof, native FS-UAE runner and acceptance gate reviews | AR-18/19/24; existing AGENTS.md dispatch |

**New specialist candidates: None.** Existing specialist plus generic
correctness/security/concurrency review packets cover this bounded work.
Evidence: `codeops/specialists/amiga-platform-reviewer.md:38-85`,
`compiler/src/clorraine/mod.rs:109-155`, and `docker/scripts/run.sh:65-148`.
No new permanent agent or routing layer is justified.

Each review packet must examine all callers/parallel paths and prove that new
assertions fail without the relevant implementation. A missing specialised role
uses a generic reviewer carrying the same read-only checklist.

## Delivery dependencies

Private-display gate setup → support proof → native driver with local core
staging → installer with packaged bootstrap → editor/source selection → release
automation → candidate review → approval → public verification.
The last phase requires local licensed-media gate evidence. WSL2, Remote-WSL and
Windows access are outside the approved Linux-only scope (AR-49); their absence
is not a release blocker because no such support is claimed.
