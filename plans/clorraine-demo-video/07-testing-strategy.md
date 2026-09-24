# Testing Strategy: Clorraine Demo Video Toolkit

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The tool is a developer CLI, not shipped code. Tests use Node's built-in `node --test`
(AR-19) with `--experimental-test-coverage` (PF-020), and are split into specification
tests (`*.spec.test.ts`, from the ST-cases below) and implementation tests
(`*.impl.test.ts`, edge cases and internals). A single opt-in smoke test exercises the
real Chrome + VHS + ffmpeg pipeline.

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Scene model / validation / config loading | 90% |
| Renderer argument building and composition math | 80% |
| Process/glue wrappers (`proc`, `log`, `paths`) | 70% |

Measurement: `node --test --experimental-test-coverage` under `make demo-check` (PF-020).
Integration tests cover the scene→clip pipeline with fakes; the end-to-end smoke test
covers the complete render path but is skipped unless `DEMO_E2E=1` is set (it needs
Chrome, VHS, and an X display). The emulator leg is never run in tests (AR-12).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from `01-requirements.md`, the `03-XX` specs, and the Ambiguity
> Register. Immutable oracle: a failing ST-case means the implementation is wrong.
> The `Source` column lives here, not in code.

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `validateConfig` on configs that have no scenes; a duplicate `id`; a present `duration <= 0`; a missing `file` | Throws a `ConfigError` listing every problem with its scene `id`; nothing renders | 03-01 §Types; 01 §Security · PF-011, PF-019 |
| ST-2 | A `card` scene with `duration: 6` at `fps: 50` | The renderer requests exactly 300 frames via the fake Chrome `render(t)`; the encoded clip is 6.0s (verified in the smoke test) | 03-02 §card · PF-020 |
| ST-3 | Tokenize a synthetic fixture containing one token from every 03-03 table row, plus a comment and a `$"…"` template | Every token carries its expected scope; the pinned handle list `String File Buffer HandlerToken Screen` is scoped | 03-03 · AR-9 · PF-007 |
| ST-4 | A `terminal` scene with commands `["a", "b"]`, `duration: 10` | The tape contains `a` then `b`, each followed by `Enter` and one `Sleep 10s` | 03-02 §terminal · PF-020 |
| ST-5 | An `emulator` scene with `frame: true` and no `duration` | The recorder builds hosted-amiga, captures the window at 50 fps for the default 7 s, and the branded-frame overlay is applied | 03-02 §emulator · AR-12, AR-29 |
| ST-6 | A `.json` config with the same schema as a `.ts` config | Both load and validate identically; both produce the same scene list | AR-21 |
| ST-7 | A scene `file: "../../etc/passwd"`, and an absolute `out: "/tmp/x.mp4"` | Both rejected, with the offending path, before any render | 01 §Security · PF-016 |
| ST-8 | `CHROME_BIN` points at a nonexistent binary | The resolver returns that path; `render` fails with a clear Chrome error and exit code 1; no fallback renderer | AR-27 · PF-009 |
| ST-9 | `new` on a file whose first line is `// Foo bar.`, and on a file with no comment | Title `Foo bar` in the first case; the file stem in the second; an existing target is not overwritten without `--force` | 03-04 §scaffold · AR-14 |
| ST-10 | A config with 3 card scenes (`5s`, `4s`, `6s`) and `transition: 0.5`; and a 1-scene config | For `n=3`: clips trimmed to their durations, two chained `xfade` stages with offsets `4.5` and `8.5`, total `15 − 2·0.5 = 14.0s`. For `n=1`: no `xfade` stage and total `5.0s` | 03-02 §compose · AR-16 · PF-003, PF-004 |
| ST-11 | A config whose `gif.sceneIds` names an unknown scene; a config with no `card` scene but a `poster` | The unknown gif id is rejected at load; the poster falls back to frame 0 of the first scene | 03-02 §compose · PF-012 |
| ST-12 | `make -n demo` and `make -n demo-check` with no `node_modules` | The dry-run shows `npm ci` before the render; `demo-check` shows `tsc --noEmit` and `node --test`; neither re-enters `node --test` | 03-04 §Makefile · AR-24, AR-25 · PF-008 |

## Smoke Test (opt-in)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-13 | `DEMO_E2E=1`; render a 2-scene `card`(4s) + `code`(3s) config with `transition: 0.5` | An MP4 exists; `ffprobe` reports one video stream, no audio, 1100x700, 50 fps, duration ≈ `4 + 3 − 0.5 = 6.5s` | AR-18, AR-29 |

ST-13 depends on the whole pipeline, so it cannot be red before implementation and is
deliberately **not** in the mandatory red-first table (PF-006).

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `test/scenes.spec.test.ts` | ST-1, ST-2 | Scene model |
| `test/grammar.spec.test.ts` | ST-3 | Grammar/theme |
| `test/tape.spec.test.ts` | ST-4 | Terminal renderer |
| `test/emulator.spec.test.ts` | ST-5 | Emulator renderer |
| `test/config.spec.test.ts` | ST-6, ST-7 | Config loader/paths |
| `test/chrome.spec.test.ts` | ST-8 | Chrome wrapper |
| `test/scaffold.spec.test.ts` | ST-9 | Scaffold |
| `test/compose.spec.test.ts` | ST-10, ST-11 | Composition |
| `test/makefile.spec.test.ts` | ST-12 | Make integration |
| `test/render.smoke.test.ts` | ST-13 | End-to-end (opt-in) |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `test/ffmpeg.impl.test.ts` | Argument construction, stderr handling, duration parsing | High |
| `test/paths.impl.test.ts` | Root resolution (`--root`), symlink/escape edges, absolute paths | High |
| `test/vhs.impl.test.ts` | Tape quoting/escaping, missing-output detection | Med |
| `test/story.impl.test.ts` | Story chapter ids, kinds, captions, gif/poster wiring | Med |
| `test/log.impl.test.ts` | Error formatting, exit codes | Low |

## Test Data

### Fixtures Needed

Fixtures are created inside each spec-author task (PF-020): a synthetic all-token `.clo`;
a tiny `.clo` with a leading comment for scaffolding; a `.json` and a `.ts` config with the
same scene list (ST-6); a config with an escaping path (ST-7); and a temp fixture tree for
scaffold/path tests so nothing is written into the working tree (PF-020).

### Mock Requirements

- A fake `chrome` capture whose `render(t)` records the requested `t` and writes a small PNG.
- A fake `vhs` that writes its declared `Output` file.
- A fake emulator recorder that records its arguments (including the 50 fps flag).
- Real `ffmpeg`/`ffprobe` for the smoke test only; a logging fake elsewhere.

The emulator environment (Docker, FS-UAE, Kickstart) is never required by tests.

## Verification Checklist

- [ ] All ST cases defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written before implementation
- [ ] Specification tests verified to fail before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] `make demo-check` passes (`tsc --noEmit` + `node --test --experimental-test-coverage`)
- [ ] The opt-in smoke render passes with `ffprobe` checks
- [ ] No regressions in the existing `make check` suite
