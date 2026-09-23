# Roadmap: Clorraine

> **Feature-Set**: Clorraine
> **Status**: In Progress
> **Created**: 2026-09-17
> **Last Updated**: 2026-09-23 08:20
> **Progress**: n/a (plan-based; no RDs drafted yet)
> **CodeOps Artifact Schema**: 1

## Legend

⬜ Backlog · ✏️ RD Drafted · 🔎 RD Preflighted · 📋 Plan Created · 🔬 Plan Preflighted · 🔄 Executing · ✅ Done · ⛔ Blocked · ⏸️ Deferred

## Tracker

| ID | Title | RD | Plan | Stage | Status | Last Updated | Depends-on / Blocker |
|----|-------|----|------|-------|--------|--------------|----------------------|
| PLAN-01 | Clorraine compiler MVP | — | [clorraine-compiler-mvp](clorraine-compiler-mvp/00-index.md) | Done | ✅ | 2026-09-21 | — |
| PLAN-02 | Clorraine build/run driver | — | [clorraine-driver](clorraine-driver/00-index.md) | Plan Created | 📋 | 2026-09-23 | PLAN-01 |

## Backlog / Known defects

| ID | Title | Detected | Notes |
|----|-------|----------|-------|
| DEF-01 | Compiler accepts a bare `return;` in a non-void function | 2026-09-23 | Should report `E10200` (`spec/05-statements.md:191`). Surfaced while building the driver's diagnostic-pass-through test; the driver plan is not the owner. Repro: `clc --profile host` on a `func main(): i32 { return ; }` exits 0. |
