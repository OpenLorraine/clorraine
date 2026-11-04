# Specification Stabilisation: Clorraine v1.0 Production Readiness

> **Document**: 03-02-spec-stabilization.md
> **Parent**: [Index](00-index.md)

## Overview

The specification is frozen for implementation but still labelled a working draft. Production
readiness requires the v1.0 rule set to be visibly stable: every chapter reviewed for stale text,
the residual ruling applied, and the status line changed to `stable (v1.0)`. This is a status and
accuracy review, not a semantic rewrite (AR-6).

The staged features stay staged. A chapter is stable when its v1.0 rules are complete; sections
that describe `comptime`, pools/arenas, `stores`/`transfers`, nested templates, user contract
tables, or C-hook trampolines keep their `E90xxx`/`W10005` staged markers and remain explicitly
out of v1.0.

## Architecture

### Current Architecture

`spec/README.md` declares the set a working draft (v0.4). Every chapter header is
`draft (Phase C)`; the diagnostics registry is normative for codes and templates. The decision
ledger is accepted and wins on conflict. `spec/check.sh` validates titles, status lines, links,
diagnostic registration, and compile-fail coverage.

### Proposed Changes

1. **Chapter review.** Read the v1.0 chapters against the ledger and grammar; correct stale
   statements, broken links, and status text. Do not change a rule without an explicit user
   ruling; the only semantic change in this programme is the AR-4 null-cast diagnostic. Record
   the E10012 exception in `spec/04-expressions.md` alongside the TY-14 cast table.
2. **Status change.** Mark the v1.0 chapters and appendices `stable (v1.0)`; update
   `spec/README.md` to "stable (v1.0)" and keep the staged-feature table.
3. **Stale project status.** Update the root `README.md` status table: NLL is implemented, not
   "the next step"; the compiler is production-readiness-track, not merely early development.
4. **Known limitations.** The README link and the `spec/README.md` "Known limitations" section
   link are both added in Phase 6, after `03-05` creates the document; `./spec/check.sh` is rerun
   then. `03-05` is the sole owner of the document.
5. **Validation.** Run `./spec/check.sh` after every chapter group; it must stay green.

## Implementation Details

### Chapter Groups

| Group | Files | Review focus |
| ----- | ----- | ------------ |
| Core language I | `00-introduction.md` – `05-statements.md` | Axioms, lexical rules, types, ownership, expressions, statements |
| Core language II | `06-functions.md` – `10-modules.md` | Functions, structs, arrays/strings, enums, modules |
| Runtime and profiles | `11-memory.md` – `15-profiles.md` | Memory, FFI, data inclusion, diagnostics, profiles |
| Borrow and codegen | `16-borrow-rules.md`, `17-codegen.md` | NLL rules, ABI, emitted-C contract |
| Appendices and anchors | `appendix-amiga.md`, `appendix-stdlib.md`, `grammar.ebnf.md`, `grammar-deltas.md`, `lineage.md`, `decisions.md`, `README.md` | Amiga and stdlib surfaces, grammar freeze, decision ledger, status |

### Target Status by File

The v1.0 freeze covers the chapters and appendices. The grammar, lineage, decision ledger, and
preflight report are anchors or audit records; they keep their existing status labels and are
explicitly outside the chapter freeze.

| File group | Target status | Notes |
| ---------- | ------------- | ----- |
| `00-introduction.md` – `17-codegen.md` | `stable (v1.0)` | v1.0 chapter set |
| `appendix-amiga.md`, `appendix-stdlib.md` | `stable (v1.0)` | v1.0 appendices; Phase 5 re-reviews them after `std.amiga.exec` lands |
| `README.md` | `stable (v1.0)` | Chapter index and status table |
| `grammar.ebnf.md` | keep `draft (Phase B anchor - the grammar freeze)` | Grammar anchor; reviewed, not relabelled |
| `grammar-deltas.md` | keep `draft (Phase B anchor)` | Anchor; reviewed, not relabelled |
| `lineage.md` | keep `draft (Phase B anchor)` | Semantic lineage anchor; reviewed, not relabelled |
| `decisions.md` | keep `accepted (Phase A complete ...)` | Decision ledger; wins on conflict |
| `preflight-report.md` | keep `passed with notes` | Audit record; not a chapter |

### New Types/Interfaces

None. The only code-facing artifact is the documentation status text; the known-limitations links
(in the root `README.md` and `spec/README.md`) are added in Phase 6, after `03-05` creates the
document.

### New Functions/Methods

None.

### Integration Points

- `spec/check.sh` is the validation command after each group.
- `docs/known-limitations.md` is owned by 03-05.
- The root `README.md` status table is shared with the release documentation in 03-05.

## Code Examples

### Status line

```markdown
> **Status:** stable (v1.0)
```

### Staged feature inside a stable chapter

```markdown
| E90004 | Pools and arenas | `pools and arenas are deferred to v2` |
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A chapter has a stale rule | Correct only stale text; a semantic change needs a separate ruling | AR-6 |
| A link breaks | Fix the link; rerun `spec/check.sh` | AR-6 |
| A staged feature is mislabelled | Keep the `E90xxx`/`W10005` marker; do not mark it v1.0 | AR-6 |
| The status change reveals a gap | File a DEF row; do not silently rewrite the rule | AR-6 |
| `spec/check.sh` fails | Stop; fix the document before the next group | AR-6 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- ST-11 and ST-12 in `07-testing-strategy.md`.
- `./spec/check.sh` after each chapter group.
- A final status scan: every v1.0 chapter and appendix header reads `stable (v1.0)`; the staged
  markers remain; the anchor/report statuses match the target table above; the README table
  reflects the same statuses. The Amiga and stdlib appendices are re-reviewed in Phase 5 after
  `std.amiga.exec` lands, and their stable status is re-recorded.
- The README and `spec/README.md` links to `docs/known-limitations.md` are added in Phase 6 after
  the document exists; `./spec/check.sh` is rerun there.
- `make check` after the documentation change.
