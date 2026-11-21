# Fixed NDK transformation inputs

`ndk-inputs/` contains data for the standalone preparation command. It does
not contain the NDK archive or a modified NDK include tree. These inputs are
hashed individually before any preparation helper runs.

| Data | Count | Exact upstream identity |
|---|---|---|
| Networking overlay | 69 files | `amiga-netinclude` `7d155ca3a3090afe3466f819dd178f2af6f5c176`, `include/` |
| 1.3 selection lists | 4 files | `amiga-gcc` `e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c`, `patches/ndk13/` |
| Independent inline headers | 3 files | `fd2sfd` `189ac23b5b2d0d0555381404d75e0508c2e95920`, `cross/share/m68k-amigaos/alib.h`; `fd2pragma` `bcffac5c89295baf1db033addc984312b4b4a722`, `Include/inline/{stubs,macros}.h` |
| Exact unified patches | 7 files | Same `amiga-gcc` revision's `patches/NDK3.2/`, re-expressed against the locked archive to reproduce the measured reference |

The first three groups retain their exact upstream bytes and embedded notices.
The patches are derived transformation data, not a new source revision.
Complete notice/source assembly and redistribution review remain required;
this inventory does not claim licence clearance.

## Exact patch derivation

The locked NDK archive has SHA-256
`96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb`.
Its `exec_lib.sfd` has a newer version banner than the original upstream patch
context. The existing reference build applies that patch using GNU patch's
fuzzy matching and retains `exec_lib.sfd.orig`.

For installation, each patch here was derived as an exact unified difference
between the authenticated archive member and the already measured patched
reference member. All seven were applied with exact offsets/context and
compared byte-for-byte with that reference before inclusion. No fuzzy choice
or external patch program is needed at installation. The original upstream
patch hashes, exact patch hashes and before/after file hashes are recorded in
the private generated `ndk-inputs-proof.json`.

Preparation preserves the reference's `exec_lib.sfd.orig` from the user's
original archive. This backup is generated locally; it is not bundled data.

## Helper and path handling

The actual preparation pipeline executes only prebuilt LHA, `sfdc` and
`fd2sfd`. It needs fd2pragma's two independent source headers, but not its
executable. The latter is therefore omitted from the selected support
candidate once this helper closure is established.

`fd2sfd` includes its input paths in a three-line comment banner. Preparation
first verifies that exact banner, then uses the measured reference spelling
`/work/reference/sdk/` in those comments only. The generated interface body
is unchanged. These are inert source comments, not executable dependencies.
All final file hashes must still match the full measured inventory.
