# Spike S1 - does vbcc honor #line into usable debug and diagnostic output?
#
# Run from the repository root, or from this directory with `make -C`.
#
#   make          build, inspect the decoded line table, then compile -DERR
#   make build    compile demo.c with vbcc (-g -O0)
#   make inspect  dump the decoded line table (objdump; readelf fallback)
#   make err      compile the -DERR variant and capture the diagnostic
#   make gdb      optional: load the hunk in m68k-amigaos-gdb
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
SPIKE := spikes/01-vbcc-line
BIN := $(SPIKE)/build/demo
RESULTS := $(SPIKE)/results

.PHONY: all build inspect err gdb clean

all: build inspect err

build:
	@mkdir -p $(REPO)/$(RESULTS) $(REPO)/$(SPIKE)/build
	$(DEV) bash -lc 'set -o pipefail; cd /work && $(VBCC) -o $(BIN) $(SPIKE)/demo.c -lamiga' 2>&1 \
		| tee $(REPO)/$(RESULTS)/compile.txt

inspect:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		(m68k-amigaos-objdump --dwarf=decodedline $(BIN) \
		 || m68k-amigaos-readelf --debug-dump=decodedline $(BIN))' 2>&1 \
		| tee $(REPO)/$(RESULTS)/decodedline.txt

err:
	$(DEV) bash -lc 'cd /work && ($(VBCC) -DERR -o $(SPIKE)/build/demo-err $(SPIKE)/demo.c -lamiga || true)' 2>&1 \
		| tee $(REPO)/$(RESULTS)/err.txt

gdb:
	$(DEV) bash -lc 'cd /work && printf "info line copy_via_ladder\nlist\nquit\n" \
		| m68k-amigaos-gdb $(BIN)' 2>&1 | tee $(REPO)/$(RESULTS)/gdb.txt

clean:
	rm -rf $(REPO)/$(SPIKE)/build
