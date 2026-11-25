## Ambiguity Register: Independent Assessment of Clorraine

> **Status**: ✅ GATE PASSED — all 20 items resolved
> **Last Updated**: 2026-11-25 13:17

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which parts of the repository the assessment covers | A) whole repository / B) compiler, codegen, shims, libraries, performance only | User: "only focus on the compiler, codegen, shims, libs*, performance" | ✅ Resolved |
| 2 | Non-functional | Which Amiga target lens drives performance and size judgments | A) A500/KS1.3 (68000, OCS, 512 KB chip) / B) A1200 (68020, AGA) / C) both | User: "both" | ✅ Resolved |
| 3 | Scope | Whether the assessment re-runs the acceptance gates (`make check`, `make verify`) as a baseline | A) re-run both / B) no re-run; both are known green | User: "this process has run hundreds of times, all is already working and compiling. no need to recheck" | ✅ Resolved |
| 4 | Non-functional | Reasoning-effort budget for the assessment phases | low / medium / high / xhigh / max | User: "my AI budget for now is only high for now and it should do", then revised: "xhigh is also ok. max is not!" — phases may use up to `xhigh`; `max` is never used | ✅ Resolved |
| 5 | Technical | Whether hand-written C reference programs may be written to measure Clorraine's overhead | A) yes / B) no | User: "write handwritten c programs if you need to to verify" | ✅ Resolved |
| 6 | Technical | Whether FS-UAE may be booted for targeted shim probes | A) yes / B) no | User: "you may boot fs-uae" | ✅ Resolved |
| 7 | Scope | When the plan is created versus executed | A) create and execute now / B) create now, execute after the working tree is settled | User: "create the full plan first and then we wait for the working tree" | ✅ Resolved |
| 8 | Naming | Plan folder and deliverable file names | A) `plans/independent-assessment/` with per-workstream `findings/WS<n>-<slug>.md`, merged ledger `05-findings.md`, final `10-assessment-report.md` / B) other names | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 9 | Scope | Exact boundary inside the confirmed scope | A) all shim families (host, hosted-amiga 1.3/2.0 plus optional suffixes, freestanding); the release-flag tables in `compiler/src/clorraine/toolchain.rs` as a performance input; vbcc as a C-validity leg; LSP, docgen, driver CLI, installer, site out / B) narrower | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 10 | Technical | How specification-first ordering applies to an assessment that changes no code | A) each workstream phase: Step 1 write probes with expected outcomes from `spec/` or the C reference before running; Step 2 run and analyze; Step 3 self-refute and harden findings; the `07` ST-cases are the acceptance contract of the deliverables / B) lightweight mini-plan without the ordering | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 11 | Data & state | Where probe sources and outputs live | A) probe sources (`.clo`, hand-written `.c`, scripts) committed under `plans/independent-assessment/probes/<WS>/`; outputs under `build/tmp/assessment/` / B) everything under `build/tmp/assessment/` | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 12 | Behavioral | What happens when a critical defect is found mid-assessment | A) no source change; record it, notify the user at once, continue; fixes go to remediation plans / B) pause the assessment and fix it immediately | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 13 | Integration | How findings enter the roadmap's DEF ledger | A) at synthesis, each verified critical/major finding gets the next `DEF-n` row linking the report; minor and opportunity stay in the report / B) every finding becomes a DEF row / C) no DEF rows until remediation planning | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 14 | Technical | Which measurement runs are allowed despite AR #3 | A) one baseline run each of `make bench`, `make bench-compile`, `make bench-flags`, plus targeted harness reruns, sanitizer builds of single probes, `vamos`, and FS-UAE; no `make check`/`verify`/`asan`/`ubsan` suites / B) reuse existing `build/perf/*.json` only | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 15 | Non-functional | Severity rubric, performance threshold, and confidence labels | A) Critical / Major / Minor / Opportunity; ≥10% net m68k instructions or `.text` on a benchmark is Major; confidence verified / probable / hypothesis, hypothesis never ranked above Minor / B) other thresholds | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 16 | Technical | How the strict quality loop applies to assessment phases | A) the adversarial verification phase (at `xhigh`) is the independent review of every workstream (fresh agents plus the specialist auditors); one correctness review of the final report; no per-phase exec-plan review loop / B) full per-phase review loop as well | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 17 | Technical | How optimizer and A1200 opportunities are measured | A) optimizer gains estimated by hand-patching emitted C in `build/tmp`, no compiler prototype; A1200 measured by rebuilding emitted C with `-m68020` under `vamos`, FS-UAE only where shim behavior differs / B) prototype IR passes on a scratch branch | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 18 | Scope | Workstream partition | A) WS1 semantic core, WS2 ownership/borrow/drop, WS3 IR and C codegen correctness, WS4 generated-code quality and optimizer, WS5 runtime shims, WS6 standard library, WS7 runtime and compile-time performance, WS8 test and verification integrity (limited to the confirmed scope) plus a defect-history root-cause analysis of the DEF ledger / B) drop WS8 and fold coverage gaps into each workstream | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 19 | Non-functional | Effort per phase within the AR #4 ceiling | A) baseline `medium`; WS1, WS4, WS6, WS7, WS8 `high`; WS2, WS3, WS5 `xhigh`; adversarial verification `xhigh`; synthesis `high` / B) uniform `high` | ✅ Resolved — User accepted recommendation: option A | ✅ Resolved |
| 20 | Technical | The verify gate for each assessment phase | Derived from AR #3, #10, #14: the phase gate is the `07` deliverable checks (ST cases) plus "no change outside `plans/independent-assessment/` and `build/`"; `make check`/`make verify` are not run | ✅ Resolved — follows from the user's accepted AR #3, #10, #14 | ✅ Resolved |

## Coverage Map

> **Coverage**: 4/4 clusters · 12/12 categories · 11 candidates · 2 residual limits · Effort: high/xhigh→inline (C4 requested xhigh; run inline in the lead session) · Independence: uniform-effort (inline fallback)
> **Freshness**: Coverage as of 2026-11-25; artifacts AGENTS.md, plans/00-roadmap.md, plans/real-workload-audit/99-execution-plan.md, compiler/src/clorraine/toolchain.rs, compiler/runtime/, stdlib/, compiler/tests/

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | Scope boundary, deliverables, naming | User answers, AGENTS.md layout, roadmap conventions | — | CV-001 (AR #8), CV-002 (AR #9), CV-003 (AR #18) | inline fallback: `preflight-auditor` is not dispatchable from this session | medium |
| C2 Behavior and edges | Mid-assessment defects, roadmap integration, quality loop | Strict quality profile in AGENTS.md, DEF ledger format | — | CV-004 (AR #12), CV-005 (AR #13), CV-006 (AR #16) | inline fallback | medium |
| C3 Technical and data | Spec-first adaptation, probe storage, measurement methods | spec-first protocol, bench harness, toolchain flags | `vamos` 68020 CPU support not yet confirmed | CV-007 (AR #10), CV-008 (AR #11), CV-009 (AR #14), CV-010 (AR #17) | inline fallback; the `vamos` CPU option is verified in the baseline phase | medium |
| C4 Risk and stakeholders | Severity, thresholds, budget | User budget (AR #4), real-workload audit thresholds | No security surface (read-only assessment) | CV-011 (AR #15) | inline fallback | medium |

**Source note:** CV-001→AR #8, CV-002→AR #9, CV-003→AR #18, CV-004→AR #12, CV-005→AR #13, CV-006→AR #16, CV-007→AR #10, CV-008→AR #11, CV-009→AR #14, CV-010→AR #17, CV-011→AR #15. No Discovery keys.

### Resolution Notes

**AR-1..AR-7:** recorded from the user's answers in the planning conversation on 2026-11-25.
**AR-8..AR-19:** the user replied "accept all recommendations" on 2026-11-25; each row records the recommended option.
**AR-20:** not a new choice; it is the consequence of AR #3 (no gate reruns), AR #10 (ST cases are the deliverable contract), and AR #14 (allowed measurement runs).
