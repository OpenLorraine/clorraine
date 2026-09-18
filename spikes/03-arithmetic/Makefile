# Spike S3 - arithmetic wrapping and shifts on the host, GCC, and vbcc.
#
# Run from the repository root, or from this directory with `make -C`.
#
#   make           run every leg and check the normative ones against expected.txt
#   make host      host cc -fwrapv (sanity leg)
#   make gcc       m68k GCC with -fwrapv
#   make gcc-nowrapv m68k GCC without -fwrapv (recorded, not normative)
#   make vbcc      vbcc with -DVBCC_IDIOM (the unsigned idiom)
#   make vbcc-plain vbcc without the idiom (recorded, not normative)
#   make check     diff the normative legs against expected.txt
#   make clean     remove build artifacts
#
# All m68k compilation and running happens in the SDK container; the host keeps
# the captured text under results/ for the findings document.

SHELL := /bin/bash

REPO := $(realpath $(CURDIR)/../..)
DK   := $(REPO)/docker
COMPOSE := docker compose -p clorraine --env-file $(DK)/.env -f $(DK)/compose.yaml
DEV := $(COMPOSE) run --rm -T dev

HOST_CC := cc
HOST_FLAGS := -std=c99 -fwrapv -O2 -Wall -Wextra -Werror
GCC := m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -Wall -Wextra -Werror
VBCC := vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include
SPIKE := spikes/03-arithmetic
RESULTS := $(SPIKE)/results
EXPECTED := $(REPO)/$(SPIKE)/expected.txt

.PHONY: all host gcc gcc-nowrapv vbcc vbcc-plain check clean

all: host gcc gcc-nowrapv vbcc vbcc-plain check

host:
	@mkdir -p $(REPO)/$(RESULTS) $(REPO)/$(SPIKE)/build
	$(HOST_CC) $(HOST_FLAGS) -o $(REPO)/$(SPIKE)/build/arith-host $(REPO)/$(SPIKE)/arith_gcc.c
	$(REPO)/$(SPIKE)/build/arith-host > $(REPO)/$(RESULTS)/host.txt

gcc:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(GCC) -fwrapv -o $(SPIKE)/build/arith-gcc $(SPIKE)/arith_gcc.c -lamiga'
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(SPIKE)/build/arith-gcc | tr -d "\r" > $(RESULTS)/gcc.txt'

gcc-nowrapv:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(GCC) -o $(SPIKE)/build/arith-gcc-nowrapv $(SPIKE)/arith_gcc.c -lamiga'
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(SPIKE)/build/arith-gcc-nowrapv | tr -d "\r" > $(RESULTS)/gcc-nowrapv.txt'

vbcc:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(VBCC) -DVBCC_IDIOM -o $(SPIKE)/build/arith-vbcc $(SPIKE)/arith_vbcc.c -lamiga'
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(SPIKE)/build/arith-vbcc | tr -d "\r" > $(RESULTS)/vbcc.txt'

vbcc-plain:
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		$(VBCC) -o $(SPIKE)/build/arith-vbcc-plain $(SPIKE)/arith_vbcc.c -lamiga'
	$(DEV) bash -lc 'set -o pipefail; cd /work && \
		vamos $(SPIKE)/build/arith-vbcc-plain | tr -d "\r" > $(RESULTS)/vbcc-plain.txt'

check:
	@diff -u $(EXPECTED) $(REPO)/$(RESULTS)/host.txt && echo "spike-s3: host matches"
	@diff -u $(EXPECTED) $(REPO)/$(RESULTS)/gcc.txt && echo "spike-s3: gcc -fwrapv matches"
	@diff -u $(EXPECTED) $(REPO)/$(RESULTS)/vbcc.txt && echo "spike-s3: vbcc unsigned idiom matches"
	@echo "spike-s3: normative legs match the expected table"
	@-diff -u $(EXPECTED) $(REPO)/$(RESULTS)/gcc-nowrapv.txt > $(REPO)/$(RESULTS)/gcc-nowrapv-diff.txt; \
		if [ -s $(REPO)/$(RESULTS)/gcc-nowrapv-diff.txt ]; then \
			echo "spike-s3: recorded - the no-fwrapv default differs (gcc-nowrapv-diff.txt)"; \
		else \
			echo "spike-s3: recorded - the no-fwrapv default matches at -O2"; \
		fi
	@-diff -u $(EXPECTED) $(REPO)/$(RESULTS)/vbcc-plain.txt > $(REPO)/$(RESULTS)/vbcc-plain-diff.txt; \
		if [ -s $(REPO)/$(RESULTS)/vbcc-plain-diff.txt ]; then \
			echo "spike-s3: recorded - plain signed vbcc differs (vbcc-plain-diff.txt)"; \
		else \
			echo "spike-s3: recorded - plain signed vbcc matches at -O2"; \
		fi

clean:
	rm -rf $(REPO)/$(SPIKE)/build
