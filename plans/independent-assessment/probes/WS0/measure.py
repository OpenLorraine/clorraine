#!/usr/bin/env python3
"""Measurement matrix for a Clorraine assessment run.

Usage (inside the SDK container, from the repository root):

    python3 <plan>/probes/WS0/measure.py <plan-folder> <baseline-sha> \
        --clc build/tmp/assessment/baseline/clc [--out DIR] \
        [--bench NAME ...] [--lens LENS ...] [--side clo|c ...] \
        [--expect-dir DIR] [--ref-dir DIR] [--no-trace]

For every benchmark (plus ``calibration``), every side (the Clorraine
program and its equivalent C reference), and every lens, the script builds
the program with the lens's explicit flag array, checks its output against
the committed expectation, and on the m68k lenses records the ``vamos``
instruction count and the section sizes. Each side is netted against its own
calibration in the same lens: ``net = raw - calibration raw`` for
instructions and ``.text``. That subtraction is the only arithmetic.

It is a probe runner, not a harness: it calls only the existing
``tools/perf/bench.py`` functions ``emit``, ``build_step``, ``run_host``,
``run_vamos``, ``trace_vamos``, ``size_sections``, and ``normalize_output``,
and writes only under ``build/tmp/assessment/``. The 68020 probe lens selects
the ``vamos`` CPU through a private ``HOME`` holding a ``.vamosrc``, because
``trace_vamos`` takes no CPU argument.

Exit status: 0 when every output matched its expectation, 1 when any output
differed or a build or run failed (each mismatch is printed), 2 on a usage
error.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
)
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402  (the existing harness module)

BENCHMARKS = ROOT / "benchmarks"
SCRATCH = ROOT / "build" / "tmp" / "assessment"
CALIBRATION = "calibration"
SCORED = [
    "container",
    "local-table",
    "lookup",
    "matrix",
    "moves",
    "sieve",
    "strings",
    "text-scan",
]

# The shipped flag lines, written out per lens (compiler/src/clorraine/toolchain.rs).
HOST_RELEASE = ["-std=c99", "-O2", "-ffunction-sections", "-fdata-sections",
                "-Wl,--gc-sections", "-s"]
M68K_RELEASE = ["-O2", "-ffunction-sections", "-fdata-sections",
                "-Wl,--gc-sections", "-s"]
M68K_SIZE = ["-Os", "-ffunction-sections", "-fdata-sections",
             "-Wl,--gc-sections", "-s"]
AMIGA_LIBS = ["-lamiga", "-lm"]
NIX13_68000 = ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv"]
NIX20_68000 = ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv"]
NIX20_68020 = ["-std=c99", "-mcrt=nix20", "-m68020", "-fwrapv"]

# lens -> (emit profile, compiler, flags before the sources, libs, vamos CPU)
LENSES: dict[str, tuple[str, str, list[str], list[str], str | None]] = {
    "host": ("host", "cc", HOST_RELEASE, [], None),
    "a500-O2": ("hosted-amiga-13", "m68k-amigaos-gcc", NIX13_68000 + M68K_RELEASE,
                AMIGA_LIBS, "68000"),
    "a500-Os": ("hosted-amiga-13", "m68k-amigaos-gcc", NIX13_68000 + M68K_SIZE,
                AMIGA_LIBS, "68000"),
    "a1200": ("hosted-amiga", "m68k-amigaos-gcc", NIX20_68000 + M68K_RELEASE,
              AMIGA_LIBS, "68000"),
    "a1200-020": ("hosted-amiga", "m68k-amigaos-gcc", NIX20_68020 + M68K_RELEASE,
                  AMIGA_LIBS, "68020"),
}
LABELS = {
    "host": "Host (output check only)",
    "a500-O2": "A500 shipped -O2 (hosted-amiga-13, nix13, 68000)",
    "a500-Os": "A500 shipped -Os (hosted-amiga-13, nix13, 68000)",
    "a1200": "A1200 shipped (hosted-amiga, nix20, 68000)",
    "a1200-020": "A1200 68020 probe build (hosted-amiga emission, -m68020, vamos -C 68020)",
}


def expectation(name: str, expect_dir: Path | None, ref_dir: Path) -> str:
    """Return the normalized expected stdout for one program.

    The committed ``<ref-dir>/<name>.expected`` file is the default; lines
    starting with ``#`` cite the source and are not part of the output.
    """
    folder = expect_dir if expect_dir is not None else ref_dir
    text = (folder / f"{name}.expected").read_text(encoding="utf-8")
    body = "".join(line for line in text.splitlines(True) if not line.startswith("#"))
    return bench.normalize_output(body.encode())


def cell(clc: Path, name: str, side: str, lens: str, out: Path, ref_dir: Path,
         expected: str, trace: bool) -> dict[str, object]:
    """Build, check, and measure one (program, side, lens) cell."""
    profile, cc, flags, libs, cpu = LENSES[lens]
    work = out / lens / side / name
    work.mkdir(parents=True, exist_ok=True)
    binary = work / "prog"
    if side == "clo":
        source = work / "prog.c"
        bench.emit(clc, profile, BENCHMARKS / name / "program.clo", source)
    else:
        source = ref_dir / f"{name}.c"
    argv = [cc, *flags, "-o", str(binary), str(source), *libs]
    record: dict[str, object] = {"command": " ".join(argv), "cpu": cpu or "host"}
    try:
        timeout = bench.HOST_BUILD_TIMEOUT if cc == "cc" else bench.M68K_BUILD_TIMEOUT
        bench.build_step(argv, timeout)
        if cpu is None:
            observed = bench.run_host(binary)
        else:
            observed = bench.run_vamos(binary)
        record["output_ok"] = observed == expected
        record["output"] = observed
        if cpu is not None:
            record.update(bench.size_sections(binary))
            if trace:
                traced, count = bench.trace_vamos(binary)
                record["instructions_raw"] = count
                record["trace_output_ok"] = traced == expected
    except bench.HarnessError as error:
        record["error"] = str(error)
        record["output_ok"] = False
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("plan", help="the run's plan folder, e.g. plans/assessment-2027-01-10")
    parser.add_argument("baseline", help="the baseline commit SHA recorded for the run")
    parser.add_argument("--clc", required=True, help="the pinned clc binary")
    parser.add_argument("--out", default=None, help="output folder under build/tmp/assessment/")
    parser.add_argument("--bench", nargs="+", default=None, help="scored benchmarks to run")
    parser.add_argument("--lens", nargs="+", default=list(LENSES), choices=list(LENSES))
    parser.add_argument("--side", nargs="+", default=["clo", "c"], choices=["clo", "c"])
    parser.add_argument("--expect-dir", default=None,
                        help="read <name>.expected from here instead of the reference folder")
    parser.add_argument("--ref-dir", default=None,
                        help="the C references (default <plan>/probes/WS0/ref)")
    parser.add_argument("--no-trace", action="store_true", help="skip the instruction trace")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()

    plan = (ROOT / args.plan).resolve()
    if not plan.is_dir():
        print(f"measure: plan folder not found: {args.plan}", file=sys.stderr)
        return 2
    ref_dir = Path(args.ref_dir).resolve() if args.ref_dir else plan / "probes" / "WS0" / "ref"
    expect_dir = Path(args.expect_dir).resolve() if args.expect_dir else None
    out = (ROOT / (args.out or "build/tmp/assessment/WS0/matrix")).resolve()
    try:
        out.relative_to(SCRATCH.resolve())
    except ValueError:
        print(f"measure: --out must stay under {SCRATCH}", file=sys.stderr)
        return 2
    clc = (ROOT / args.clc).resolve()
    if not clc.is_file():
        print(f"measure: clc not found: {args.clc}", file=sys.stderr)
        return 2
    names = [CALIBRATION] + (args.bench or SCORED)
    unknown = sorted(set(names) - set(SCORED) - {CALIBRATION})
    if unknown:
        print(f"measure: unknown benchmark(s): {', '.join(unknown)}", file=sys.stderr)
        return 2

    expected = {name: expectation(name, expect_dir, ref_dir) for name in names}
    results: dict[str, dict[str, dict[str, object]]] = {}
    saved_home = os.environ.get("HOME")
    for lens in args.lens:
        cpu = LENSES[lens][4]
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="vamos-home-") as home:
            if cpu is not None:
                Path(home, ".vamosrc").write_text(f"[vamos]\ncpu={cpu}\n", encoding="utf-8")
                os.environ["HOME"] = home
            jobs = [(name, side) for side in args.side for name in names]
            with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
                futures = {
                    pool.submit(cell, clc, name, side, lens, out, ref_dir,
                                expected[name], not args.no_trace): (name, side)
                    for name, side in jobs
                }
                for future in concurrent.futures.as_completed(futures):
                    name, side = futures[future]
                    results.setdefault(lens, {}).setdefault(side, {})[name] = future.result()
                    print(f"measure: {lens} {side} {name} done", flush=True)
            if saved_home is not None:
                os.environ["HOME"] = saved_home

    # Net each side against its own calibration in the same lens.
    for lens, sides in results.items():
        for side, cells in sides.items():
            cal = cells.get(CALIBRATION, {})
            for name, record in cells.items():
                if "instructions_raw" in record and "instructions_raw" in cal:
                    record["instructions_net"] = (
                        int(record["instructions_raw"]) - int(cal["instructions_raw"]))
                if "text" in record and "text" in cal:
                    record["text_net"] = int(record["text"]) - int(cal["text"])

    failures = []
    for lens, sides in results.items():
        for side, cells in sides.items():
            for name, record in cells.items():
                bad = (not record.get("output_ok")
                       or record.get("trace_output_ok") is False or "error" in record)
                if bad:
                    failures.append(f"{lens} {side} {name}: "
                                    f"{record.get('error') or 'output differs from expectation'}")

    document = {
        "created": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "plan": args.plan,
        "baseline": args.baseline,
        "clc": {"path": str(clc), "sha256": hashlib.sha256(clc.read_bytes()).hexdigest()},
        "lenses": {lens: {"label": LABELS[lens], "profile": LENSES[lens][0],
                          "compiler": LENSES[lens][1], "flags": LENSES[lens][2],
                          "libs": LENSES[lens][3], "vamos_cpu": LENSES[lens][4]}
                   for lens in args.lens},
        "expectations": str(expect_dir or ref_dir),
        "results": results,
        "failures": failures,
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "matrix.json").write_text(json.dumps(document, indent=2, sort_keys=True) + "\n",
                                     encoding="utf-8")
    print(f"measure: wrote {out / 'matrix.json'}")
    for failure in failures:
        print(f"MISMATCH: {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
