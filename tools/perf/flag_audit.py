#!/usr/bin/env python3
"""Repeatable old-vs-shipped build-flag audit for the Clorraine compiler.

The scored harness measures compiler emission; this audit measures the
shipped release build. It emits each program once with ``clc``, builds the
old, GC-only, and shipped variants with explicit argument arrays, runs the
result, and records section sizes, file sizes, instruction counts, and output.
The ``--check`` mode asserts the size and runtime floors.

This module starts as the behavior-neutral seam for the specification tests;
the matrix, floor, and repeat implementations are filled in during the
measurement phase.
"""

from __future__ import annotations

import argparse

# The scored benchmark set; calibration is measured but not scored.
SCORED = ("sieve", "matrix", "strings", "moves", "lookup")
CALIBRATION = "calibration"

# The four portable 1.3 corpus programs. interrupt-counter is FS-UAE-only.
CORPUS = ("buffer-container", "file-copier", "string-parser", "volatile-mmio")


class AuditError(RuntimeError):
    """A failed audit step with the exact context."""


def check_floors(matrix: dict) -> list[str]:
    """Return the floor failures in ``matrix``; an empty list means pass."""
    raise NotImplementedError("floor checks are wired in the measurement phase")


def compare_repeats(first: dict, second: dict) -> list[str]:
    """Return the non-timing differences between two audit matrices."""
    raise NotImplementedError("repeat comparison is wired in the measurement phase")


def run_matrix(config: dict) -> dict:
    """Build and measure the configured matrix."""
    raise NotImplementedError("the matrix runner is wired in the measurement phase")


def main(argv: list[str] | None = None) -> int:
    """Parse the audit command line."""
    parser = argparse.ArgumentParser(prog="flag_audit.py")
    parser.add_argument("command", choices=("run",))
    parser.add_argument("--clc", default="target/debug/clc")
    parser.add_argument("--out", default="build/perf/flags.json")
    parser.add_argument("--repeat", type=int, default=2)
    parser.add_argument("--check", action="store_true")
    parser.parse_args(argv)
    raise NotImplementedError("the audit runner is wired in the measurement phase")


if __name__ == "__main__":
    raise SystemExit(main())
