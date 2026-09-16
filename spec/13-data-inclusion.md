# Chapter 13 - Data Inclusion

> **Status:** planned
> **Derived from:** Blend65 Ch 13 (asset inclusion kept; loaders dropped)

## Scope (planned)

* `embed("path")` produces a `copy` byte array at compile time; the file is
  read at build time and baked into the binary; no runtime loader.
* Sizes are compile-time constants; `len` is available without runtime cost.
* Placement: hosted profile uses the normal data segment; `const` data is
  read-only in the image; a later profile feature may place data in specific
  regions (Amiga appendix).
* Reproducibility: embedded bytes make the generated C deterministic
  (Ch 17 SS11); paths appear in `#line` metadata only.
* `comptime function` (deferred) can transform embedded data at compile time.

## Open questions

| Question | Recommendation |
|---|---|
| Large asset policy | Document a profile size warning; no compression in v1 |
| `embed` of directories | No; one file per call |
| Checksum/manifest | Later tooling; not a language feature |
