# Advisory reports

Reports from the planning-time advisors, written by `/advise`:

| Advisor | Lens | File | Proposal ids |
|---|---|---|---|
| `graphics-games-advisor` | Amiga graphics, multimedia, and game development, including the cost of emitted code on the 68000 | `<date>-graphics.md` | `GA-NNN` |
| `systems-os-advisor` | Clorraine as the systems language for LorraineOS (m68k now, ARM later), and the tools an OS needs | `<date>-os.md` | `OA-NNN` |

Run them when deciding what to build next: after a release or milestone,
before planning work in their domain, or when LorraineOS needs a capability.
Never during plan execution. A report is read-only advice; the owner chooses
which proposals become requirements (`make-requirements`). Reports are kept,
not overwritten: a second run on the same day gets a `-2` suffix.

## Report format

```markdown
# <Advisor> report: <date>

> **Clorraine**: <commit> (<version>)
> **Inputs read**: <files and directories, including the LorraineOS core paths for the OS advisor>
> **Not checked**: <evidence the advisor could not read or run>

## Summary

<three to six sentences: where Clorraine stands for this domain and the main gaps>

## Proposals

### GA-001 (or OA-001): <title>

- **Layer**: language change (needs owner ruling) | compiler | runtime/stdlib | tooling | docs/examples
- **Value**: high | medium | low, with one sentence on who gains what
- **Effort**: S | M | L
- **Targets**: minimum OS and hardware (graphics), or m68k / ARM (OS)
- **Today**: what exists, with `file:line`
- **Proposal**: what to add or change
- **Evidence**: repository facts and cited references
- **Risks**: safety, compatibility, scope
- **Next step**: a `make-requirements` seed, or a draft `docs/amiga-sdk/registry.tsv` row

## Already good

<what serves this domain well today, briefly>

## Considered, not recommended

- <idea>: <one-line reason>

## Questions for the owner

- <decisions the advisor cannot make>
```
