# Libnix inherited source representation

`gnuplot-3.7.1-amiga.c` is the released-source candidate already identified
by the component review:
<https://raw.githubusercontent.com/gnuplot/gnuplot/938a1c42aca589b9ae7c1a526faacbc42eca67fc/amiga.c>.
Its SHA-256 is
`5154776f0eb250ed4087ba082f46d9fd2de8074bbf90d04ba8ea066760894c12`.
This is not a new compiler/build dependency or proof of the exact historical
import used by libnix.

`libnix-pipeoc.diff` reconstructs the exact pinned libnix
`sources/nix/extra/pipeoc.c` from that original. Its result has SHA-256
`b29a4daf8b5be0090449de5cd5eec1007261dbcf01e0ae0ef138cd43a07de970`.
The original copyright, permission and upstream contact text is retained.
No modified libnix file is substituted into the compiler build.

This source representation is for Clorraine's native support distribution,
not an upstream gnuplot release. Distribution-specific support belongs at
<https://github.com/OpenLorraine/clorraine/issues>, not with the upstream
author. The completed corresponding-source package must convey the original,
patch and this identification together, and must not silently replace this
representation with the complete modified file alone.

The component's original terms and the source-form reconstruction proof remain
separate from publication approval. Other inherited libnix source notices
still apply to their files.
