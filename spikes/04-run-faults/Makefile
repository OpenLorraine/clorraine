# Spike S4 - run-fault patterns on the host and under vamos.
#
# Run from the repository root, or from this directory with `make -C`.
#
#   make        run the host legs and the Amiga (GCC and vbcc) legs
#   make host   build the host shim and check all six cases
#   make amiga  build the Amiga shim with GCC and vbcc, run under vamos
#   make clean  remove build artifacts
#
# The runner asserts, for every case, the exact first line, that the cleanup
# ladder did not run, and that the program did not exit 0. Captured output lands
# under results/ for the findings document.

SHELL := /bin/bash

REPO := $(realpath $(CURDIR)/../..)
DK   := $(REPO)/docker
COMPOSE := docker compose -p clorraine --env-file $(DK)/.env -f $(DK)/compose.yaml
DEV := $(COMPOSE) run --rm -T dev

HOST_CC := cc
HOST_FLAGS := -std=c99 -O2 -Wall -Wextra -Werror
GCC := m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -Wall -Wextra -Werror
VBCC := vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include
SPIKE := spikes/04-run-faults
RESULTS := $(SPIKE)/results

.PHONY: all host amiga clean

all: host amiga

host:
	@mkdir -p $(REPO)/$(RESULTS) $(REPO)/$(SPIKE)/build
	$(HOST_CC) $(HOST_FLAGS) -o $(REPO)/$(SPIKE)/build/traps-host $(REPO)/$(SPIKE)/traps.c
	cd $(REPO)/$(SPIKE) && ./run-faults.sh host ./build/traps-host results

amiga:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(GCC) -DAMIGA_SHIM -o $(SPIKE)/build/traps-amiga-gcc $(SPIKE)/traps.c -lamiga -lm && \
		$(VBCC) -DAMIGA_SHIM -o $(SPIKE)/build/traps-amiga-vbcc $(SPIKE)/traps.c -lamiga -lmieee'
	$(DEV) bash -lc 'cd /work && chmod +x $(SPIKE)/run-faults.sh && \
		KNOWN_MISMATCH="nan_float_to_int" \
		$(SPIKE)/run-faults.sh amiga-gcc $(SPIKE)/build/traps-amiga-gcc $(RESULTS) vamos'
	$(DEV) bash -lc 'cd /work && \
		KNOWN_MISMATCH="nan_float_to_int" \
		$(SPIKE)/run-faults.sh amiga-vbcc $(SPIKE)/build/traps-amiga-vbcc $(RESULTS) vamos'

clean:
	rm -rf $(REPO)/$(SPIKE)/build
