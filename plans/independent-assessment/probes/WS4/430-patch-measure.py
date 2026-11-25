#!/usr/bin/env python3
"""Measure the WS4 patched-C estimates (expectations in each <NNN>-<slug>.expected).

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS4/430-patch-measure.py <pinned-clc> <out-dir> [<patch-id> ...]

For each `<NNN>-<slug>.diff`, the benchmark named in its expectation is emitted for
hosted-amiga-13 with the pinned clc (the same absolute-path spelling the Phase 0
matrix used), the diff is applied to a copy, and both units are built with the A500
shipped -O2 line (nix13, -m68000, -fwrapv, section GC, strip) and traced under vamos.
Reported: output check against benchmarks/<b>/expected-output.txt (after
bench.normalize_output) for both builds, raw instructions, .text/.data/.bss, and the
patched-minus-base deltas. The calibration program is unchanged by every patch, so
the raw delta equals the net delta; the net base is raw minus the Phase 0 A500 -O2
Clorraine calibration (2,919 instructions, findings/records/WS0-matrix.json).
With --seeded-mismatch, the expected output is replaced by a wrong line so the
output comparison must report a failure (red check).
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402

HERE = Path(__file__).resolve().parent
FLAGS = ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv", "-O2", "-ffunction-sections",
         "-fdata-sections", "-Wl,--gc-sections", "-s"]
CAL_NET_BASE = 2919


def main() -> int:
    seeded = "--seeded-mismatch" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--seeded-mismatch"]
    clc = Path(args[0]).resolve()
    out = ROOT / args[1]
    only = args[2:]
    out.mkdir(parents=True, exist_ok=True)
    failures = 0
    for diff in sorted(HERE.glob("*.diff")):
        pid = diff.stem
        if only and not any(pid.startswith(o) for o in only):
            continue
        meta = dict(line.split(":", 1) for line in (HERE / f"{pid}.expected").read_text().splitlines()
                    if ":" in line and not line.startswith("#"))
        name = meta["benchmark"].strip()
        work = out / pid
        work.mkdir(parents=True, exist_ok=True)
        base_c = work / "prog.c"
        bench.emit(clc, "hosted-amiga-13", ROOT / "benchmarks" / name / "program.clo", base_c)
        patched_c = work / "patched.c"
        patched_c.write_text(base_c.read_text())
        done = subprocess.run(["patch", "--quiet", str(patched_c), str(diff)], capture_output=True, text=True)
        if done.returncode != 0:
            print(f"FAIL {pid}: patch does not apply: {done.stdout}{done.stderr}")
            failures += 1
            continue
        expected = bench.expected_output(name) if not seeded else "seeded mismatch\n"
        row = {}
        for label, src in (("base", base_c), ("patched", patched_c)):
            binary = work / label
            bench.build_step(["m68k-amigaos-gcc", *FLAGS, "-o", str(binary), str(src), "-lamiga", "-lm"],
                             bench.M68K_BUILD_TIMEOUT)
            observed, count = bench.trace_vamos(binary)
            row[label] = (observed == expected, count, bench.size_sections(binary))
        ok = row["base"][0] and row["patched"][0]
        failures += not ok
        d_instr = row["patched"][1] - row["base"][1]
        base_net = row["base"][1] - CAL_NET_BASE
        sb, sp = row["base"][2], row["patched"][2]
        print(f"{'PASS' if ok else 'FAIL'} {pid} ({name}): output base={row['base'][0]} "
              f"patched={row['patched'][0]}; instr {row['base'][1]} -> {row['patched'][1]} "
              f"(delta {d_instr:+d}, {100.0 * d_instr / base_net:+.1f}% of net {base_net}); "
              f".text {sb['text']} -> {sp['text']} ({sp['text'] - sb['text']:+d}); "
              f".data {sb['data']} -> {sp['data']} ({sp['data'] - sb['data']:+d}); "
              f".bss {sb['bss']} -> {sp['bss']} ({sp['bss'] - sb['bss']:+d})")
    print(f"patch-measure: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
