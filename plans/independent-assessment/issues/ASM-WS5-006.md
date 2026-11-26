<!-- title: [bug] Prevent use after free of installed clip regions and AmigaGuide messages -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R5 -->

## Summary
A program the compiler accepts can make the layers library use a freed region on the next refresh, or reply a message into a closed context — use-after-free from the safe surface. The regions rule is documented, so the risk is a user mistake the type system does not catch.

| Field | Value |
|---|---|
| Finding | ASM-WS5-006 · DEF-593 |
| Severity / Priority / Rank | Critical / P0 / 12 of 58 |
| Domain / Lens | shims / A1200 shipped (`hosted-amiga` modules) |
| Confidence / Verdict | probable / confirmed (Critical, probable) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading against layers `InstallClipRegion` ("You MUST remember to call InstallClipRegion(l,NULL) before calling DeleteLayer(l) … if you have installed a non-NULL ClipRegion"). `layerSetClipRegion(borrow layer, borrow region)` ends its borrow at the call, so `freeRegion(region)` or the layer's drop is accepted afterwards; the shim then disposes a region still referenced by `layer->ClipRegion`, or deletes a layer with a user clip installed. `waitAmigaGuideMessage` returns a `GuideMessage` with no link to its `AmigaGuide`; `closeAmigaGuide(g)` followed by `replyGuideMessage(m)` (or the message's drop) calls `ReplyAmigaGuideMsg` after `CloseAmigaGuide` freed the context.

## Root cause
the ownership model sees only call-scoped borrows; the OS keeps the pointer past the call, and the shim does not track the relationship.

Location: `compiler/runtime/shim_hosted_amiga_optional.c:938-949` (`cl_free_region` disposes without checking installation), `:1031-1041` (`cl_layer_set_clip`), `shim_hosted_amiga.c:5891-5899` (`cl_layer_free` calls `DeleteLayer` without `InstallClipRegion(l, NULL)`), `stdlib/std.amiga.regions.clo:5-6` and `:85-92` (the rule is a doc comment), `shim_hosted_amiga_optional_net.c:211-220`, `:291-300` (`cl_guide_close`, `cl_guide_reply`), at e24a8752

## Recommended fix
track it in the shim: a region records the layer it is installed in, and `cl_free_region` uninstalls it; `cl_layer_free` installs NULL first; a guide context keeps a list of outstanding messages, `cl_guide_close` replies them, and a later reply of an orphaned message is a no-op. Alternatively model the relationship in the stdlib types (the layer takes the region).

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS6-02 (handle lifetimes), ASM-WS5-019 (semaphore freed while owned). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
