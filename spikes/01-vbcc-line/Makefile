# Spike S1 - does vbcc honor #line into usable debug and diagnostic output?
#
# Run from the repository root, or from this directory with `make -C`.
#
#   make          build, inspect the line info, compile -DERR, and compare
#   make build    compile demo.c with vbcc (-g -O0): object then executable
#   make inspect  dump the decoded line table with every decoder available
#   make err      compile the -DERR variant and capture the diagnostic
#   make gcc      build the same file with GCC and read its line info with gdb
#   make probe    record the object/executable container formats and tools
#   make clean    remove build artifacts
#
# The exact commands and their output are saved under results/ so the findings
# document can quote them. All compilation happens in the SDK container; the
# host only drives docker and keeps the captured text.

SHELL := /bin/bash

REPO := $(realpath $(CURDIR)/../..)
DK   := $(REPO)/docker
COMPOSE := docker compose -p clorraine --env-file $(DK)/.env -f $(DK)/compose.yaml
DEV := $(COMPOSE) run --rm -T dev

VBCC := vc +aos68k -c99 -g -O0 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include
GCC  := m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g
SPIKE := spikes/01-vbcc-line
OBJ := $(SPIKE)/build/demo.o
BIN := $(SPIKE)/build/demo
GCC_BIN := $(SPIKE)/build/demo-gcc
RESULTS := $(SPIKE)/results

.PHONY: all build inspect err gcc probe clean

all: build inspect err gcc probe

build:
	@mkdir -p $(REPO)/$(RESULTS) $(REPO)/$(SPIKE)/build
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(VBCC) -c -o $(OBJ) $(SPIKE)/demo.c && \
		$(VBCC) -o $(BIN) $(OBJ) -lamiga' 2>&1 \
		| tee $(REPO)/$(RESULTS)/compile.txt

inspect:
	$(DEV) bash -lc 'cd /work && \
		echo "== objdump --dwarf=decodedline ==" && \
		m68k-amigaos-objdump --dwarf=decodedline $(BIN); \
		echo "== readelf --debug-dump=decodedline ==" && \
		(m68k-amigaos-readelf --debug-dump=decodedline $(BIN) || true); \
		echo "== gdb: info line demo.clor:28 ==" && \
		(m68k-amigaos-gdb --batch -ex "info line demo.clor:28" \
			-ex "info line demo.clor:30" $(BIN) || true)' 2>&1 \
		| tee $(REPO)/$(RESULTS)/decodedline.txt

err:
	$(DEV) bash -lc 'cd /work && \
		($(VBCC) -DERR -o $(SPIKE)/build/demo-err $(SPIKE)/demo.c -lamiga || true)' 2>&1 \
		| sed -E 's#/tmp/[A-Za-z0-9_]+[.]asm#<tmp>.asm#g' \
		| tee $(REPO)/$(RESULTS)/err.txt

gcc:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(GCC) -o $(GCC_BIN) $(SPIKE)/demo.c -lamiga && \
		echo "== gdb: info line demo.clor:28 (GCC) ==" && \
		m68k-amigaos-gdb --batch -ex "info line demo.clor:28" \
			-ex "info line demo.clor:30" -ex "info sources" $(GCC_BIN); \
		echo "== GCC -DERR diagnostic ==" && \
		($(GCC) -DERR -c -o /tmp/demo-err.o $(SPIKE)/demo.c || true)' 2>&1 \
		| tee $(REPO)/$(RESULTS)/gcc-debug.txt

probe: build gcc
	$(DEV) bash -lc 'cd /work && \
		echo "== vbcc object header ==" && od -c $(OBJ) | head -3; \
		echo "== objdump -h vbcc executable ==" && m68k-amigaos-objdump -h $(BIN); \
		echo "== readelf -S vbcc executable ==" && (m68k-amigaos-readelf -S $(BIN) || true); \
		echo "== vobjdump vbcc object ==" && (vobjdump $(OBJ) || true); \
		echo "== mapped filename strings ==" && grep -a -o "demo[.]clor" $(OBJ) $(BIN) | head; \
		echo "== objdump -h GCC executable ==" && m68k-amigaos-objdump -h $(GCC_BIN) | grep -i -E "sections|debug"' 2>&1 \
		| tee $(REPO)/$(RESULTS)/format-probe.txt

clean:
	rm -rf $(REPO)/$(SPIKE)/build
