# Spike S2 - do the emitted FFI prototypes compile and run against the NDK?
#
# Run from the repository root, or from this directory with `make -C`.
#
#   make          build both compilers, run both under vamos, and diff
#   make build    compile ffi_abi.c with GCC and vbcc
#   make run-vamos run both binaries under vamos and capture their output
#   make check    diff the GCC and vbcc program output
#   make clean    remove build artifacts
#
# Everything runs in the SDK container; the host keeps the captured text under
# results/ for the findings document.

SHELL := /bin/bash

REPO := $(realpath $(CURDIR)/../..)
DK   := $(REPO)/docker
COMPOSE := docker compose -p clorraine --env-file $(DK)/.env -f $(DK)/compose.yaml
DEV := $(COMPOSE) run --rm -T dev

GCC  := m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -Wall -Wextra -Werror
VBCC := vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include
SPIKE := spikes/02-ffi-abi
GCC_BIN := $(SPIKE)/build/ffi-abi-gcc
VBCC_BIN := $(SPIKE)/build/ffi-abi-vbcc
RESULTS := $(SPIKE)/results

.PHONY: all build run-vamos check clean

all: build run-vamos check

build:
	@mkdir -p $(REPO)/$(RESULTS) $(REPO)/$(SPIKE)/build
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(GCC) -o $(GCC_BIN) $(SPIKE)/ffi_abi.c -lamiga 2>&1 | tee $(RESULTS)/build-gcc.txt && \
		$(VBCC) -o $(VBCC_BIN) $(SPIKE)/ffi_abi.c -lamiga 2>&1 | tee $(RESULTS)/build-vbcc.txt'

run-vamos:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(GCC_BIN) | tr -d "\r" > $(RESULTS)/gcc.txt'
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(VBCC_BIN) | tr -d "\r" > $(RESULTS)/vbcc.txt'

check:
	@diff -u $(REPO)/$(RESULTS)/gcc.txt $(REPO)/$(RESULTS)/vbcc.txt \
		| tee $(REPO)/$(RESULTS)/diff.txt
	@echo "spike-s2: GCC and vbcc outputs match"

clean:
	rm -rf $(REPO)/$(SPIKE)/build
