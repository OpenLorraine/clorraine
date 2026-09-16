# Chapter 11 - Memory and Resources

> **Status:** planned
> **Depends on:** Ch 03 (ownership), Ch 12 (contracts), Ch 15 (profiles)
> **Replaces:** Blend65 Ch 11 (Static Frame Allocation) entirely

## Scope (planned)

* Memory classes per profile: stack (locals), heap (allocator contracts),
  static (globals), and profile regions (Amiga chip/fast memory; LorraineOS
  regions later).
* Allocators are explicit: an `own Buffer` is created by a named allocator
  contract (`AllocVec`, `AllocPooled`, ...) and knows its releaser; there is
  no language-level `new`.
* Pools and arenas: deferred (Ch 12 SS7); when added, bulk release invalidates
  outstanding borrows and the checker must reject those uses.
* Stack discipline: allocation on the stack is the default for aggregates;
  no large arrays on the stack in the m68k profile (warning W10020 above a
  profile threshold).
* Resource kinds beyond memory (libraries, files, ports) share the same
  ownership model; the release function is the only difference.
* No garbage collection and no reference counting in the core (A2).
* Alignment and layout guarantees come from Ch 07 and the profile appendix.

## Open questions

| Question | Recommendation |
|---|---|
| Allocator failure handling | Nullable owned result; check before use (Phase 0 pattern) |
| Chip-memory placement (Amiga) | Profile allocator variants (`AllocVec` flags) behind named contracts |
| Arena/pool model | v2; needs a region concept for bulk release |
