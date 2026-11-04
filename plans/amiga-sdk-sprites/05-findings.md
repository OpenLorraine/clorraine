# Findings: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Implementation findings

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| F-1 | Raw `SPRxPT`/`DMACON` writes were overwritten by the custom display's compiled list; the sprite never appeared. | The raw stage showed only the background; the sprite debug bits showed `SprIns` present but the compiled list had no sprite moves. | Bundle 2 uses the 1.3 `graphics.library` simple-sprite machine attached to an Intuition screen. |
| F-2 | Channel 0 is the system pointer; `GetSprite(0, ...)` fails on the A500. | The first pair stage printed `sprite-stage2: even failed` with channel 0. | The demo uses channels 2/3 (pair), 4 (single), and 6 (animation). Docs warn that channel 0 is reserved. |
| F-3 | `GetSprite` fills only `num`; `posctldata` is NULL until `ChangeSprite`. | The review read the KS1.3 ROM; writing the ATTACH bit before `ChangeSprite` would touch address 2. | `spriteAttach` only records the intent; `ChangeSprite` paths set the ATTACH bit after the pointer exists. |
| F-4 | The attached pair and collision work on the real A500. | The pair stage read `collisions 0x91FE`; masking bits 9..14 gives `0x1000` (pair 2/3 vs single 4). | The demo parks the pair and single together and prints `sprite-demo: collision 0x1000`. |
| F-5 | Changing the sprite image pointer does not update the compiled screen list until `RethinkDisplay`. | `ChangeSprite` updates the viewport's sprite list; the screen's compiled copper list is separate. | `cl_sprite_change_image` calls `RethinkDisplay` after every image change; `MoveSprite` is live. |
| F-6 | The animation blank buffer leaked. | The review traced the animation free path: `image = NULL` then `cl_sprite_destroy(..., 0)` skipped the blank. | The animation free calls `cl_sprite_destroy(sprite, 1)` with `image = NULL`, freeing the blank. |

## Red phase

The spec and impl tests were written before the module existed:

```text
error E11102: 'Sprite' is not exported from module 'std.amiga.sprite'
error E11102: 'newSprite' is not exported from module 'std.amiga.sprite'
...
test result: FAILED. 0 passed; 2 failed
```

## Green phase

| Gate | Result |
|---|---|
| `cargo test --test amiga_sprite_spec_test` | 7 passed (host/A500 compile, host rejection, `nix13`/`nix20` cross-builds, screen attach, Exec still gated) |
| `cargo test --test amiga_sprite_impl_test` | 2 passed (wrapper routing, null-guarded drops) |
| `make sdk-a500-sprites` | OK; held display crop and collision line matched |
| `make verify` | exit 0; hello, Boing, sprites, differential, corpus, and fault legs passed |
| `make sdk-registry` | OK (79 capabilities, 10 bundles; Bundle 2 6/6) |

## Runtime evidence

| Evidence | Value |
|---|---|
| Live screen | `docs/amiga-sdk/a500-sprites.png` (640x512, held frame) |
| Golden collision line | `docs/amiga-sdk/a500-sprites-line.png` |
| Collision | `sprite-demo: collision 0x1000` |
| Machine | FS-UAE A500, real Kickstart 1.3 rev 34.5, decrypted licensed ROM |
| ROM SHA-1 | `891e9a547772fe0c6c19b610baf8bc4ea7fcb785` |
| Channels | pair 2/3, single 4, animation 6 |

## Review ledger

The independent review found two critical and six major findings. All critical
and major findings were fixed and re-tested.

| ID | Severity | Finding | Fix |
|---|---|---|---|
| C1 | Critical | `spriteAttach` wrote the ATTACH bit through a NULL `posctldata`. | The bit is set only after `ChangeSprite` created the pointer; attach before screen is a no-op. |
| C2 | Critical | Freeing a sprite after `CloseScreen` used a dangling screen pointer. | The shim tracks channels per screen; `cl_close_screen` clears each attached sprite's back-pointer. |
| M1 | Major | Image-pointer changes did not rebuild the screen list. | `cl_sprite_change_image` calls `RethinkDisplay`; the demo holds animation frame 2, distinct from frame 0. |
| M2 | Major | Freeing a sprite while its screen was open could free live image data. | The destroy path hides, rebuilds, waits a frame, then `FreeSprite` and rebuilds again. |
| M3 | Major | Animation frame 0/blank ownership leaked. | The animation free clears `image`, destroys the sprite with `blank` freeing, then frees all frames. |
| M4 | Major | The custom-display sprite path was dead and undocumented. | Removed; the public surface is Intuition-screen only and the docs say so. |
| M5 | Major | Registry/plan/roadmap/issue #3 were not updated. | Bundle 2 rows 6/6, plan 8/8, roadmap/programme/issue updated. |
| M6 | Major | Tests were compile-only; animation/show/hide/lifetime not exercised. | The demo uses frame 2 for the held golden, detaches the pair/single before close, and leaves the animation attached so the screen-close registry path runs. |
| M7 | Minor | Boing runtime evidence predates the display-init move. | `make verify` re-ran Boing and the sprite gate after the change; both passed. |
| m1 | Minor | Pair validation allowed different heights/screens. | `spriteAttach` requires equal height and the same screen. |
| m2 | Minor | `setPixels` doc claimed `spriteMove` shows the image. | Doc says `spriteShow` only. |
| m3 | Minor | Demo early returns after attach could leave screen pointers. | The show function has a single cleanup epilogue. |
| m4 | Minor | Smoke could miss the hold and include a moving pointer. | Hold is 800 ticks (16 s); the smoke polls from 30 s. |
| m5 | Minor | Unused `owns_image`, stale comments, null channel 0. | Removed `owns_image`; null channel returns `0xFF`; `MoveSprite` re-ORs ATTACH. |
| m6 | Minor | Specialist coverage record lacks the new plan folder. | `analyze-agents` ran on 2026-11-04 and recorded 57 plan folders; outcome `None`. |
| m7 | Minor | Docs disagreed on screen/sprite lifetime. | Both appendices and the module say hide, detach, close, free; the close hook is a safety net. |

Accepted residuals:

- The pair handles must be freed together; freeing only the even channel leaves
  the odd channel allocated.
- Channel 0 is normally the system pointer; the API does not reserve it.
- Animation frame changes rebuild the screen list, so a 50 Hz animation is
  display-library-bound rather than a sprite-register benchmark.
