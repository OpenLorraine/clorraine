# Execution Plan: Clorraine Demo Video Toolkit

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-24 23:20
> **Progress**: 30/48 tasks (62%)
> **CodeOps Artifact Schema**: 1

## Overview

Build `@clorraine/demo` under `tools/demo/`: a zero-build TypeScript CLI that renders a
demo video from a scene script, using Shiki + a Clorraine grammar for code, headless
Chrome for HTML scenes, VHS for terminal scenes, FS-UAE for emulator scenes, and ffmpeg
for composition. Migrate `demo/story.sh` to `demo/story.ts`, add `new <file.clo>`, and
flip the result media to generated-and-gitignored with a committed poster.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Tool foundation | 9 |
| 2 | HTML renderer and Shiki viewer | 11 |
| 3 | Terminal, emulator, and composition | 10 |
| 4 | Migrate the Clorraine story | 9 |
| 5 | Per-file scaffold and documentation | 9 |

**Total: 48 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by
the plan's task-size criteria: one reviewable change per task)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent
> MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ✅ (completed: 2026-09-24 22:15)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last
>    Updated stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is
>    resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same
>    line. The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from
>    these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Tool foundation

> **Phase baseline tree**: 10048f620a01949c2afc4565ef6b73db9ca1bb12
> **Lenses**: api-surface — informational; activation stays profile-driven

### Step 1.1: Specification tests (red phase)

**Reference**: [03-01](03-01-tool-architecture.md) §Types · [07](07-testing-strategy.md) ST-1, ST-6, ST-7 · AR-1, AR-21
**Objective**: Write the scene-model and config-loader spec tests before the modules exist.

- [x] 1.1.1 [spec-author] Write `tools/demo/test/scenes.spec.test.ts` (ST-1) and `tools/demo/test/config.spec.test.ts` (ST-6, ST-7) from the 07 ST-cases ✅ (completed: 2026-09-24 21:53)
- [x] 1.1.2 Confirm the red phase: both tests fail because `src/scenes.ts` and `src/config.ts` do not exist ✅ (completed: 2026-09-24 21:53)

**Deliverables**:
- [ ] The two spec-test files exist and fail for the expected reason

**Verify**: `make demo-check`

---

### Step 1.2: Toolchain and core types

**Reference**: [03-01](03-01-tool-architecture.md) §Architecture, §Types, §Toolchain · AR-1, AR-19, AR-22, AR-23
**Objective**: Create the package boundary, the scene model, the config loader, and the CLI skeleton.

- [x] 1.2.1 Create the package boundary and install — `tools/demo/package.json`, `tools/demo/tsconfig.json`, `tools/demo/README.md`, committed `tools/demo/package-lock.json`; add `node_modules` to `.gitignore`; run `npm install`; add the `make demo-check` target (PF-005) ✅ (completed: 2026-09-24 21:57)
- [x] 1.2.2 Scene model, `validateConfig`, and `defineDemo` — `tools/demo/src/scenes.ts`, `tools/demo/src/index.ts` ✅ (completed: 2026-09-24 21:57)
- [x] 1.2.3 Config loader (`.ts`/`.json`) and repo-root escape guard (`--root`/`CLORRAINE_DEMO_ROOT` seam) — `tools/demo/src/config.ts`, `tools/demo/src/util/paths.ts` ✅ (completed: 2026-09-24 21:57)
- [x] 1.2.4 CLI skeleton (`render`/`new`/`list`, with `--only`/`--out`/`--root`) and process/log helpers — `tools/demo/src/cli.ts`, `tools/demo/src/util/proc.ts`, `tools/demo/src/util/log.ts` ✅ (completed: 2026-09-24 21:57)
- [x] 1.2.5 Green phase: the spec tests from Step 1.1 pass ✅ (completed: 2026-09-24 21:57)

**Deliverables**:
- [ ] `clorraine-demo list` prints scenes for a sample config
- [ ] All Step 1.1 spec tests pass

**Verify**: `make demo-check`

---

### Step 1.3: Implementation tests and hardening

**Reference**: [03-01](03-01-tool-architecture.md) §Error Handling · [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Cover path/escape and logging internals, and add the `demo-check` target.

- [x] 1.3.1 Path/escape and logging impl tests, using the `--root` temp-tree seam — `tools/demo/test/paths.impl.test.ts`, `tools/demo/test/log.impl.test.ts` ✅ (completed: 2026-09-24 22:00)
- [x] 1.3.2 Full verification of Phase 1 ✅ (completed: 2026-09-24 22:00)

**Deliverables**:
- [ ] `make demo-check` runs `tsc --noEmit` and `node --test`
- [ ] All verification passing

**Verify**: `make demo-check`

---

## Phase 2: HTML renderer and Shiki viewer

> **Phase baseline tree**: 0fbbb6ef913f1c62b0af3fb1194e878c9e6ac355
> **Lenses**: api-surface — informational

### Step 2.1: Specification tests (red phase)

**Reference**: [03-02](03-02-renderers.md) §card, §Chrome · [03-03](03-03-clorraine-grammar.md) · [07](07-testing-strategy.md) ST-2, ST-3, ST-8 · AR-2, AR-9, AR-20, AR-27
**Objective**: Write the card-render, grammar, and Chrome-failure spec tests before the renderers exist.

- [x] 2.1.1 [spec-author] Write `tools/demo/test/scenes.spec.test.ts` card case (ST-2), `tools/demo/test/grammar.spec.test.ts` (ST-3), and `tools/demo/test/chrome.spec.test.ts` (ST-8) ✅ (completed: 2026-09-24 22:07)
- [x] 2.1.2 Confirm the red phase: the tests fail because the grammar, Shiki wrapper, and Chrome wrapper do not exist ✅ (completed: 2026-09-24 22:07)

**Deliverables**:
- [ ] The three spec-test files exist and fail for the expected reason

**Verify**: `make demo-check`

---

### Step 2.2: Grammar and renderers

**Reference**: [03-02](03-02-renderers.md) · [03-03](03-03-clorraine-grammar.md) · AR-2, AR-9, AR-10, AR-18, AR-20, AR-26
**Objective**: Build the Clorraine grammar, the Shiki/Chrome/ffmpeg wrappers, and the card/code/poster renderers.

- [x] 2.2.1 Clorraine TextMate grammar and the Shiki theme — `tools/demo/grammars/clorraine.tmLanguage.json`, `tools/demo/assets/theme.json`, `tools/demo/assets/demo.css` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.2 Shiki wrapper with grammar registration — `tools/demo/src/util/shiki.ts` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.3 Headless Chrome CDP frame capture — `tools/demo/src/util/chrome.ts` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.4 ffmpeg/ffprobe wrapper — `tools/demo/src/util/ffmpeg.ts` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.5 Card renderer (and poster frame) — `tools/demo/src/renderer/card.ts`, `tools/demo/assets/card.html`, `tools/demo/src/renderer/poster.ts` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.6 Code renderer with scroll/highlight — `tools/demo/src/renderer/code.ts`, `tools/demo/assets/code.html` ✅ (completed: 2026-09-24 22:15)
- [x] 2.2.7 Green phase: the Step 2.1 spec tests pass ✅ (completed: 2026-09-24 22:15)

**Deliverables**:
- [ ] A `code` scene renders a highlighted clip from `examples/hello/hello.clo`
- [ ] A `card` scene renders a clip of the configured duration

**Verify**: `make demo-check`

---

### Step 2.3: Implementation tests and hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Cover ffmpeg argument construction and capture error paths.

- [x] 2.3.1 ffmpeg argument and stderr impl tests — `tools/demo/test/ffmpeg.impl.test.ts` ✅ (completed: 2026-09-24 22:18)
- [x] 2.3.2 Full verification of Phase 2 ✅ (completed: 2026-09-24 22:18)

**Deliverables**:
- [ ] All verification passing

**Verify**: `make demo-check`

---

## Phase 3: Terminal, emulator, and composition

> **Phase baseline tree**: 48a7fb6bdc08dbb37ba508336188194955c9bda1
> **Lenses**: api-surface — informational

### Step 3.1: Specification tests (red phase)

**Reference**: [03-02](03-02-renderers.md) §terminal, §emulator, §compose · [07](07-testing-strategy.md) ST-4, ST-5, ST-10 · AR-3, AR-7, AR-12, AR-16
**Objective**: Write the tape, emulator, and composition spec tests before the renderers exist.

- [x] 3.1.1 [spec-author] Write `tools/demo/test/tape.spec.test.ts` (ST-4), `tools/demo/test/emulator.spec.test.ts` (ST-5: 50 fps, default 7s), and `tools/demo/test/compose.spec.test.ts` (ST-10: trim, chained xfade, n=1 guard; ST-11: gif id, poster fallback) ✅ (completed: 2026-09-24 23:14)
- [x] 3.1.2 Confirm the red phase for the tape, emulator, and composition tests ✅ (completed: 2026-09-24 23:14)

**Deliverables**:
- [ ] The three spec-test files exist and fail for the expected reason

**Verify**: `make demo-check`

---

### Step 3.2: Terminal, emulator, and composition implementation

**Reference**: [03-02](03-02-renderers.md) · AR-3, AR-7, AR-12, AR-16, AR-17, AR-28
**Objective**: Implement tape generation, the refactored recorder, the branded frame, and composition.

- [x] 3.2.1 VHS tape generation and the terminal renderer — `tools/demo/src/util/vhs.ts`, `tools/demo/src/renderer/terminal.ts` ✅ (completed: 2026-09-24 23:17)
- [x] 3.2.2 Refactor the emulator recorder into a callable with the stem-collision fix — `demo/record-emulator.sh` ✅ (completed: 2026-09-24 23:17)
- [x] 3.2.3 Emulator renderer (50 fps window capture) and branded frame — `tools/demo/src/renderer/emulator.ts`, `tools/demo/assets/frame.html` ✅ (completed: 2026-09-24 23:17)
- [x] 3.2.4 Composition: normalize, caption, trim to `duration`, chained `xfade` (+ n=1 guard), concat, GIF, poster fallback — `tools/demo/src/renderer/compose.ts` ✅ (completed: 2026-09-24 23:17)
- [x] 3.2.5 Green phase: the Step 3.1 spec tests pass ✅ (completed: 2026-09-24 23:17)

**Deliverables**:
- [ ] A `terminal` scene records a real terminal clip
- [ ] An `emulator` scene composites the capture into the branded frame
- [ ] Composition writes an MP4 with crossfades and the expected duration

**Verify**: `make demo-check`

---

### Step 3.3: Implementation tests and hardening

**Reference**: [07](07-testing-strategy.md) §Smoke Test, §Implementation Tests · ST-13
**Objective**: Add the opt-in end-to-end smoke render and cover tape quoting.

- [x] 3.3.1 [spec-author] Write the opt-in smoke render test (ST-13) — `tools/demo/test/render.smoke.test.ts` ✅ (completed: 2026-09-24 23:20)
- [x] 3.3.2 VHS tape quoting and missing-output impl tests — `tools/demo/test/vhs.impl.test.ts` ✅ (completed: 2026-09-24 23:20)
- [x] 3.3.3 Full verification of Phase 3 ✅ (completed: 2026-09-24 23:20)

**Deliverables**:
- [ ] All verification passing

**Verify**: `make demo-check`

---

## Phase 4: Migrate the Clorraine story

> **Phase baseline tree**: `748e8b1ff31f2c35a1af3e9a04d03d136e259de5`
> **Lenses**: api-surface — informational

### Step 4.1: Specification tests (red phase)

**Reference**: [03-04](03-04-story-and-scaffold.md) §story, §Makefile · [07](07-testing-strategy.md) ST-12
**Objective**: Write the Makefile and story-shape spec tests before the story and targets exist.

- [x] 4.1.1 [spec-author] Write `tools/demo/test/makefile.spec.test.ts` (ST-12, `make -n` text) and a story-shape spec test asserting the 15 chapters, their kinds, and the gif/poster wiring
- [x] 4.1.2 Confirm the red phase: `demo/story.ts` and the `demo` target do not exist

**Deliverables**:
- [x] The spec-test files exist and fail for the expected reason

**Verify**: `make demo-check`

---

### Step 4.2: Story migration and integration

**Reference**: [03-04](03-04-story-and-scaffold.md) · AR-8, AR-11, AR-12, AR-13, AR-17
**Objective**: Port the story, wire the Makefile, delete the old pipeline, and regenerate the media.

- [x] 4.2.1 Port the story to `demo/story.ts` (15 chapters; see 03-04 table)
- [x] 4.2.2 Add the `demo` target (with the `npm ci` guard and the second `render demo/story.ts --only emulator --out demo/emulator.mp4` invocation) — `Makefile`
- [x] 4.2.3 Delete `demo/story.sh`, `demo/show-source.sh`, `demo/tape/*.tape`; untrack `demo/*.mp4` and `demo/demo.gif`; update `.gitignore`
- [ ] 4.2.4 Render the story; commit `demo/poster.png`; validate chapter order and total duration
- [x] 4.2.5 Green phase: the Step 4.1 spec tests pass

**Deliverables**:
- [ ] `make demo` writes `demo/demo-full.mp4`, `demo/demo.gif`, `demo/emulator.mp4`, `demo/poster.png`
- [x] Old scripts/tapes removed; result media untracked

**Verify**: `make demo` then `make demo-check`

---

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Story config impl test (chapter ids, kinds, captions) — `tools/demo/test/story.impl.test.ts`
- [ ] 4.3.2 Full verification of Phase 4

**Deliverables**:
- [ ] All verification passing

**Verify**: `make demo-check`

---

## Phase 5: Per-file scaffold and documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: api-surface — informational

### Step 5.1: Specification tests (red phase)

**Reference**: [03-04](03-04-story-and-scaffold.md) §scaffold · [07](07-testing-strategy.md) ST-9 · AR-14
**Objective**: Write the scaffold spec test before the scaffold exists.

- [ ] 5.1.1 [spec-author] Write `tools/demo/test/scaffold.spec.test.ts` (ST-9): title from a comment, title from the stem, overwrite guard
- [ ] 5.1.2 Confirm the red phase: `src/scaffold.ts` does not exist

**Deliverables**:
- [ ] The spec-test file exists and fails for the expected reason

**Verify**: `make demo-check`

---

### Step 5.2: Scaffold and documentation

**Reference**: [03-04](03-04-story-and-scaffold.md) §scaffold, §README · AR-14, AR-27
**Objective**: Implement `new <file.clo>` and update the README and AGENTS.md.

- [ ] 5.2.1 Scaffold generator and the `new` subcommand — `tools/demo/src/scaffold.ts`, `tools/demo/src/cli.ts`
- [ ] 5.2.2 README: embed `demo/poster.png`; state that media is generated — `README.md`
- [ ] 5.2.3 AGENTS.md: tool layout, `make demo`/`demo-check`, generated-media policy — `AGENTS.md`
- [ ] 5.2.4 Green phase: the Step 5.1 spec test passes

**Deliverables**:
- [ ] `new examples/hello/hello.clo` writes a renderable scaffold
- [ ] README and AGENTS.md updated

**Verify**: `make demo-check`

---

### Step 5.3: Implementation tests and hardening

- [ ] 5.3.1 Scaffold title-derivation and overwrite edge-case impl tests — `tools/demo/test/scaffold.impl.test.ts`
- [ ] 5.3.2 Confirm the strict-profile phase reviews cover the input-validation and edge cases in `01-requirements.md` (AC-8)
- [ ] 5.3.3 Full verification of Phase 5

**Deliverables**:
- [ ] All verification passing

**Verify**: `make demo-check`

---

## Dependencies

```
Phase 1
    ↓
Phase 2
    ↓
Phase 3
    ↓
Phase 4
    ↓
Phase 5
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make demo-check` passes (`tsc --noEmit` + `node --test`)
3. ✅ `make demo` regenerates the story, and `make check`/`make verify` still pass
4. ✅ No warnings/errors; no dead code
5. ✅ Scene file paths are validated against the repo root; spawned commands use argument arrays
6. ✅ README and AGENTS.md updated; `demo/clorraine.vim` and the refactored recorder remain
7. ✅ Phase reviews passed (strict profile)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
