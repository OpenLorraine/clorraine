# Ambiguity Register: Clorraine Demo Video Toolkit

> **Status**: ✅ GATE PASSED — all 29 items resolved
> **Last Updated**: 2026-09-24 21:30
> **CodeOps Artifact Schema**: 1

This register records every decision with semantic weight for the `clorraine-demo-video`
plan. A row is `✅ Resolved` only with a user decision; a deferral must name the decision,
its owner, and its revisit trigger. No item is deferred.

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Technical | Rendering pipeline for the demo tool | TS orchestrator + existing tools / Remotion / Motion Canvas | User chose TS orchestrator + existing tools (VHS for terminal, ffmpeg for cards/concat) | ✅ Resolved |
| AR-2 | Technical | Source viewer / syntax highlighting | Shiki + Clorraine grammar / Pygments + lexer / styled terminal | User chose Shiki + a custom Clorraine TextMate grammar | ✅ Resolved |
| AR-3 | Scope | Terminal/command scenes | Keep VHS / HTML terminal card / drop them | User chose to keep VHS terminal scenes | ✅ Resolved |
| AR-4 | Naming | Plan folder and tool location | tools/demo + plan clorraine-demo-video / under demo/ / user-named | User chose `tools/demo/` and `plans/clorraine-demo-video/` | ✅ Resolved |
| AR-5 | Technical | npm lockfile policy | Commit lockfile / no lockfile | User chose to commit `package-lock.json` | ✅ Resolved |
| AR-6 | Scope | First-plan coverage | Story + per-file scaffold / story only / per-file only | User chose story migration **and** `new <file.clo>` scaffold | ✅ Resolved |
| AR-7 | UX | Emulator chapter presentation | Branded frame / raw capture + caption | User chose the branded frame | ✅ Resolved |
| AR-8 | Scope | Committing result media | Generate + gitignore / commit outputs / host externally | User: do **not** commit `*.mp4`/`*.gif`; generated and gitignored | ✅ Resolved |
| AR-9 | Technical | Grammar token source | From clorraine.vim + spec / invent / minimal | User: derive the token set from `demo/clorraine.vim` and the spec grammar | ✅ Resolved |
| AR-10 | UX | Theme/brand values | Reuse existing / new | User: reuse `0x1e1e2e` background, `0x88ccff` accent, DejaVu fonts | ✅ Resolved |
| AR-11 | Scope | Fate of `demo/clorraine.vim` | Keep / move / delete | User: it stays at `demo/clorraine.vim` (editors only) | ✅ Resolved |
| AR-12 | Scope | `make demo` requirements | Needs emulator, out of `verify` / CI-safe mode | User: needs the emulator; not part of `make verify` | ✅ Resolved |
| AR-13 | UX | README demo presentation once outputs are uncommitted | Commit a PNG poster / no image / host externally | User chose a committed `demo/poster.png` | ✅ Resolved |
| AR-14 | Behavioral | Scaffold defaults (`new <file.clo>`) | Proposed: `demo/out/<stem>.mp4`; card→code→terminal run→emulator→outro; title from a leading `//` comment else the filename | User: "yes please" — accept the proposed defaults | ✅ Resolved |
| AR-15 | Technical (complexity escalation) | New support surface: `tools/demo/` npm package + `shiki` + headless Chrome | Smaller: zero-dep TS + Pygments + ffmpeg / larger: shiki + Chrome | User approved the larger option after the independent challenge | ✅ Resolved |
| AR-16 | UX | Chapter transition style and duration | 0.5s crossfade / hard cut / longer | User accepted recommendation: 0.5s crossfade. Amended by PF-001 (Option A): scene ≡ chapter, so every adjacent scene crossfades | ✅ Resolved |
| AR-17 | UX | GIF composition | Current opening loop / full story / none | User accepted recommendation: the current opening loop (intro card, hello source, hello emulator, explain card) | ✅ Resolved |
| AR-18 | UX / Technical | Video format | 1100x700@30 yuv420p H.264 CRF20 silent / other | User accepted recommendation: 1100x700, yuv420p, H.264 CRF 20, silent. **Amended by AR-29:** output frame rate is 50 fps (was 30) to preserve emulator animation | ✅ Resolved |
| AR-19 | Technical | Node tooling | npm + `tsc --noEmit` + `node --test`, no ESLint / add a linter | User accepted recommendation: npm, `tsc --noEmit`, `node --test`, no ESLint | ✅ Resolved |
| AR-20 | Technical | Headless Chrome capture mechanism | CDP over built-in WebSocket / Playwright / per-frame spawn | User accepted recommendation: CDP over Node's built-in WebSocket, deterministic time `t`; no Playwright | ✅ Resolved |
| AR-21 | Scope | Config accepts JSON as well as TS | TS primary + same-schema JSON / TS only | User accepted recommendation: TS module primary; same-schema JSON also accepted | ✅ Resolved |
| AR-22 | Naming | npm package name and module system | `@clorraine/demo`, ESM / other | User accepted recommendation: `@clorraine/demo`, ESM | ✅ Resolved |
| AR-23 | Technical | Intermediates location | `build/demo/**` / `tools/demo/.cache` | User accepted recommendation: `build/demo/**` (already gitignored) | ✅ Resolved |
| AR-24 | Scope | Tool tests in `make check` / `make verify` | Separate `make demo-check` / fold into check | User accepted recommendation: separate host-side `make demo-check`; not in `check`/`verify` | ✅ Resolved |
| AR-25 | Technical | `make demo` dependency install | Run `npm ci` when `node_modules` is absent / require manual install | User accepted recommendation: `make demo` runs `npm ci` when `node_modules` is absent | ✅ Resolved |
| AR-26 | Technical | Grammar file location | `tools/demo/grammars/clorraine.tmLanguage.json` / elsewhere | User accepted recommendation: `tools/demo/grammars/clorraine.tmLanguage.json` | ✅ Resolved |
| AR-27 | Behavioral | Missing Chrome or headless failure | Fail fast with a clear error / degrade to Pygments | User accepted recommendation: require Chrome; fail fast with a clear error | ✅ Resolved |
| AR-28 | Technical | Branded emulator frame method | Chrome-rendered PNG frame + ffmpeg overlay / ffmpeg drawbox / animated device | User accepted recommendation: Chrome-rendered PNG frame overlaid with ffmpeg (static bezel) | ✅ Resolved |
| AR-29 | UX / Technical | Emulator capture frame rate and length | 25 fps whole clip / 50 fps + native crop / customizable | User: capture at a high frame rate for animations (50 fps, PAL) and make the emulator length customizable with a **7s default** | ✅ Resolved |

## Complexity Escalation — approval evidence (AR-15)

```text
Original goal: a TypeScript utility that turns a Clorraine file into a demonstration video
               (cards, a syntax-highlighted source viewer, emulator runs), replacing demo/story.sh.
Extra system or support code: a new tools/demo/ npm package, the shiki dependency, a custom
               Clorraine TextMate grammar, and a headless-Chrome frame-capture path.
Why it may be needed: enables an HTML/CSS-rendered code and card look (typography, layout,
               animation, a branded emulator frame) instead of the ffmpeg drawtext cards and the
               terminal-only Vim viewer the user rejected; shiki gives VS Code-grade highlighting.
Evidence: demo/story.sh:120-145 uses ffmpeg drawtext; demo/show-source.sh:13 is `vim -R`; the repo
               has no package.json; Cargo.toml:3-4 constrains only the Rust workspace, not dev tooling.
Smallest solution that still works: zero-dependency TypeScript CLI (node file.ts) + generated VHS
               tapes + ffmpeg cards + Pygments highlighting + the existing record-emulator.sh.
Extra cost: package.json, node_modules (gitignored), committed package-lock.json, shiki + dev
               typescript, a TextMate grammar, and a browser-capture path sensitive to Chrome/fonts.
Independent verdict: Simplify — Chrome + shiki are "a cold-start maintenance jump for a video
               tool"; Pygments + ffmpeg already work on this host.
Direct user decision: approved larger (2026-09-24 21:25)
```

## Resolution Notes

**AR-1–AR-14:** imported pre-resolved from the planning conversation; per the shared gate rule 3
these are not re-confirmed.

**AR-15:** the larger option was approved by the user after the visible stop packet and the
independent `Simplify` verdict; the recommendation to approve was grounded in the user's stated
visual goal.

**AR-16–AR-28:** presented as a batch with recommendations; the user accepted all recommendations
(bulk acceptance, 2026-09-24 21:30), which the shared gate accepts as an explicit decision.
