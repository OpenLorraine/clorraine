# WS2 — Ownership, borrowing, and drops

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS2 (WS2a = WS2-01..06, WS2b = WS2-07..12)

## Scope and limits

Components: `own/`, `borrow.rs`, `init_graph.rs`. Lens: an accepted program that leaks,
double-frees, uses after free or move, or reads uninitialized memory is Critical. Probes
run on the host profile in four builds: the debug line, the shipped release line, and the
AddressSanitizer and UndefinedBehaviorSanitizer lines of 03-01 §Ground rules.

**Instrument.** `probes/WS2/tok.clo` declares a move-only handle `Tok` whose deleter prints
`drop <id>` and frees a one-byte block from the host C library's `strdup`. Drop order and
count are visible on stdout; leaks and double frees are visible to the sanitizers. The
instrument is host-only: libnix declares `strdup`, so the user `ffi` prototype conflicts on
the Amiga profiles (and the compiler, correctly, refuses user `ffi` bindings to the shim's
own symbols, E11101). The m68k lenses are therefore not exercised by WS2 probes; the
ownership lowering is profile-independent apart from the embedded shim.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (2.1.1, 2.1.2) | `probes/WS2/200`–`254`, `tok.clo`, `san-control.*` | done |

## Checklist outcomes

_(filled in Step 2.2)_

## Findings

_(filled in Step 2.2)_

## Spec questions

_(filled in Step 2.2)_

## Falsifiability log

_(filled in task 2.1.3)_

## Escalations

None yet.

## Probe index

Runner: `probes/WS1/probe.py --sanitize` (adds the ASan and UBSan host lines).

| Item | Probes |
|------|--------|
| instrument | 200 |
| WS2-01 moves | 201–209 |
| WS2-02 drop elaboration | 210–215 |
| WS2-03 loops | 213, 216–220 |
| WS2-04 `defer` | 221–225 |
| WS2-05 temporaries | 219, 226–230 |
| WS2-06 NLL | 231–234 |
| WS2-07 views and escapes | 235–238 |
| WS2-08 generics | 239–242 |
| WS2-09 closures | 243–245 |
| WS2-10 projections | 246–249 |
| WS2-11 initialization | 250–254 |
| WS2-12 structural review | code reading (task 2.2.4) |
| sanitizer self-test | `san-control.{c,sh,expected}` |
