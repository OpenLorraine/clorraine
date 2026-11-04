# Amiga SDK Registry

This directory is the long-lived map of the Amiga SDK surface. It is the source
of truth for what exists, what is missing, and which baseline each capability
needs. GitHub issues are a generated view, not the database.

## Files

| File | Role |
|---|---|
| `bundles.tsv` | One row per bundle: id, title, tier, minimum OS/hardware, status, tracking issue, plan path |
| `registry.tsv` | One row per capability |
| `coverage.md` | Generated summary; never hand-edit |
| `README.md` | This schema and workflow |

## Baselines

| Tier | Hardware | OS |
|---|---|---|
| `core` | A500: 68000, OCS, 512 KB chip | Kickstart 1.3 |
| `advanced` | ECS/68000/1 MB and up (A1200/AGA included) | Kickstart 2.0–3.1 |
| `optional` | 3.0+/AGA add-ons | Optional extras |

Every capability row records `min_os` and `min_hw`. A capability is not claimed
for a weaker machine unless its row says so.

## Capability columns

14 tab-separated columns, in this order:

```text
id  bundle  area  capability  priority  tier  min_os  min_hw  status  depends  acceptance  evidence  spec  notes
```

| Field | Rule |
|---|---|
| `id` | `SDK-<AREA>-<NNN>`, unique, never reused |
| `bundle` | an id from `bundles.tsv` |
| `area` | `graphics/bitmap`, `dos/files`, `intuition/events`, … |
| `capability` | one user-visible capability, not one C function |
| `priority` | `must`, `should`, `could`, `wont` |
| `tier` | `core`, `advanced`, `optional` |
| `min_os` | `1.3`, `2.0`, `2.1`, `3.0`, `3.1`, `any` |
| `min_hw` | `OCS/68000/512K`, `ECS/68000/1M`, `AGA/68020/2M`, `any` |
| `status` | `missing`, `planned`, `in-progress`, `blocked`, `done`, `deferred`, `out-of-scope` |
| `depends` | comma-separated IDs, or `-` |
| `acceptance` | test or demo id, or `-` |
| `evidence` | make target, FS-UAE run, checksum, or `-` |
| `spec` | appendix/section anchor, or `-` |
| `notes` | short constraint; no tabs |

## Workflow

```text
make sdk-registry     # validate the TSV and regenerate coverage.md
make sdk-issues       # generate one tracking-issue body per bundle
```

1. A capability is `done` only with an acceptance test/demo and evidence.
2. Update the row in the same commit as the implementation.
3. Regenerate `coverage.md`; a stale generated file is a defect.
4. Create one tracking issue per bundle from `build/amiga-sdk/issues/`.
5. Close the tracking issue when the bundle's `must`/`should` rows are `done`
   or explicitly deferred.

## Issue labels

`amiga-sdk`, `bundle:<id>`, `tier:core`, `tier:advanced`, `priority:must`,
`priority:should`, `good-first-issue`.

The issue body is generated from the registry. Do not hand-edit the checklist;
update the registry and regenerate the body instead.
