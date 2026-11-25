#!/usr/bin/env python3
"""WS7-01: host wall time of every benchmark, Clorraine and the C reference.

Builds each benchmark (and the calibration program) for the host release lens of the
Phase 0 matrix (cc -std=c99 -O2 with section GC and strip), checks the output against
the benchmark golden (`benchmarks/<name>/expected-output.txt`; the C references share
it), then runs each binary RUNS times and reports min, median, max, the spread
(max/min - 1), and the median net of the side's own calibration median.

Usage (SDK container, repo root): 700-host-time.py <clc> <out-dir> [--runs N] [--seeded]
--seeded replaces the `sieve` golden with a wrong line; the run must reject it.
"""
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402

REF = ROOT / "plans" / "independent-assessment" / "probes" / "WS0" / "ref"
FLAGS = ["-std=c99", "-O2", "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections", "-s"]


def build(clc: Path, name: str, side: str, out: Path) -> Path:
    work = out / side / name
    work.mkdir(parents=True, exist_ok=True)
    if side == "clo":
        src = work / "prog.c"
        bench.emit(clc, "host", ROOT / "benchmarks" / name / "program.clo", src)
    else:
        src = REF / f"{name}.c"
    binary = work / "prog"
    bench.build_step(["cc", *FLAGS, "-I", str(REF), "-o", str(binary), str(src)], 300)
    return binary


def timed(binary: Path, runs: int) -> list[int]:
    samples = []
    for _ in range(runs):
        start = time.perf_counter_ns()
        subprocess.run([str(binary)], stdout=subprocess.DEVNULL, check=True, timeout=120)
        samples.append(time.perf_counter_ns() - start)
    return samples


def main() -> int:
    clc, out = Path(sys.argv[1]), Path(sys.argv[2])
    runs = int(sys.argv[sys.argv.index("--runs") + 1]) if "--runs" in sys.argv else 15
    seeded = "--seeded" in sys.argv
    names = ["calibration"] + bench.benchmark_names()
    failures, med = 0, {}
    for name in names:
        expected = (ROOT / "benchmarks" / name / "expected-output.txt").read_text()
        if seeded and name == "sieve":
            expected = "seeded mismatch\n"
        for side in ("clo", "c"):
            binary = build(clc, name, side, out)
            observed = bench.run_host(binary)
            ok = bench.normalize_output(observed.encode()) == bench.normalize_output(expected.encode())
            failures += not ok
            samples = timed(binary, runs)
            med[(name, side)] = statistics.median(samples)
            spread = max(samples) / min(samples) - 1
            print(f"{'PASS' if ok else 'FAIL'} {name} {side}: runs {runs} min {min(samples) // 1000} us "
                  f"median {int(med[(name, side)]) // 1000} us max {max(samples) // 1000} us spread {spread:.1%}")
    print("net (median minus own calibration median), clo vs c:")
    for name in names[1:]:
        nc = med[(name, "clo")] - med[("calibration", "clo")]
        nr = med[(name, "c")] - med[("calibration", "c")]
        delta = (nc - nr) / nr if nr > 0 else float("nan")
        print(f"  {name}: clo {int(nc) // 1000} us c {int(nr) // 1000} us delta {delta:+.1%}")
    print(f"host-time: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
