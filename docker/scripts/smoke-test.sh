#!/usr/bin/env bash
# Smoke test for the c-next Amiga SDK image.
#
# Verifies, inside the image:
#   1. GCC builds and runs a C program under vamos (no ROM required).
#   2. G++ builds a C++ program.
#   3. NDK headers and the m68k-amigaos-gcc ABI work (compile-only probe).
#   4. vbcc builds the same C program and it runs under vamos.
#   5. gdb and vamos are present.
#
# Exits non-zero on the first failure. Prints a short summary at the end.
set -euo pipefail

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
cd "${work}"

run_amiga() {
    # Normalize CRLF line endings emitted by AmigaOS console output.
    vamos "$@" | tr -d '\r'
}

cat > hello.c <<'EOF'
#include <stdio.h>

int main(void) {
    printf("hello from c-next\n");
    return 0;
}
EOF

cat > hello.cpp <<'EOF'
#include <cstdio>

int main() {
    std::printf("hello from c-next (c++)\n");
    return 0;
}
EOF

cat > ndk_probe.c <<'EOF'
#include <exec/types.h>
#include <proto/exec.h>

/* Compile-only probe: proves NDK headers and library inline stubs work. */
void ndk_probe(void) {
    struct Library *lib = OpenLibrary((STRPTR)"exec.library", 0);
    if (lib != 0) {
        CloseLibrary(lib);
    }
}
EOF

echo "=== tool versions ==="
m68k-amigaos-gcc --version | head -1
m68k-amigaos-g++ --version | head -1
m68k-amigaos-gdb --version | head -1
vamos --help >/dev/null 2>&1 && echo "vamos: present"

echo "=== gcc: C hello ==="
m68k-amigaos-gcc -mcrt=nix20 -O2 -o hello_gcc hello.c
run_amiga hello_gcc

echo "=== gcc: C++ hello ==="
m68k-amigaos-g++ -mcrt=nix20 -O2 -o hello_cpp hello.cpp
run_amiga hello_cpp

echo "=== gcc: NDK headers ==="
m68k-amigaos-gcc -mcrt=nix20 -c -o ndk_probe.o ndk_probe.c
echo "ndk_probe.o built"

echo "=== vbcc: C hello ==="
vbcc_config=""
for cfg in +aos68k +a68k; do
    if vc "${cfg}" -c99 -O2 -o hello_vbcc hello.c >/dev/null 2>&1; then
        vbcc_config="${cfg}"
        break
    fi
done
if [ -z "${vbcc_config}" ]; then
    echo "ERROR: no known vbcc m68k config matched; available configs:"
    vc -h 2>&1 | head -20 || true
    exit 1
fi
echo "using vc ${vbcc_config}"
vc "${vbcc_config}" -c99 -O2 -o hello_vbcc hello.c
run_amiga hello_vbcc

echo "=== SMOKE OK ==="
