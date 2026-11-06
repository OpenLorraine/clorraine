#!/usr/bin/env bash
# Build the release Bundle 9 hosted-amiga demos used by the acceptance gates.
# Runs inside the SDK container; the working directory is the repository root.
set -euo pipefail

cargo build --workspace --quiet
for demo in \
    optional-exec20 \
    optional-pool \
    optional-args \
    optional-services \
    optional-intuition \
    optional-boopsi \
    optional-datatypes \
    optional-lowlevel \
    optional-nonvolatile \
    optional-network \
    optional-guide; do
    ./target/debug/clorraine build --release --profile hosted-amiga \
        "examples/amiga/${demo}.clo"
done
