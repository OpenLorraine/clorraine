#!/usr/bin/env python3
"""WS3-11 generated-C shape robustness probe (expectation in 380-shapes.expected).

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS3/380-shapes.py <pinned-clc> <emission-record.tsv> [<out-dir>]

Each case is a large but valid program, kept below the nesting depths at which clc
itself fails (see ASM-WS1-004). It is emitted for host and hosted-amiga and compiled
(compile only, -O2) with host cc, m68k-amigaos-gcc, and vbcc. Each compile must
succeed within 10x the Phase 0 compile time of the emitted program nearest in C
line count for the same compiler and profile (record: findings/WS0-emission-record.tsv).
"""

import csv
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
HDR = "module Probe.Shape;\n\nimport { println } from std.io;\n\n"


def cases() -> dict[str, str]:
    c = {}
    c["parens-3000"] = HDR + "func main(): i32 {\n    let x: i32 = " + "(" * 3000 + "1" + ")" * 3000 + ";\n    println($\"{x}\");\n    return 0;\n}\n"
    c["addchain-1000"] = HDR + "func f(n: i32): i32 {\n    return " + " + ".join(f"n * {i}" for i in range(1000)) + ";\n}\n\nfunc main(): i32 {\n    println($\"{f(1)}\");\n    return 0;\n}\n"
    c["switch-3000"] = HDR + "func f(n: i32): i32 {\n    let r: i32 = 0;\n    switch (n) {\n" + "".join(f"        case {i}:\n            r = {i * 7 % 1000};\n            break;\n" for i in range(3000)) + "        default:\n            r = -1;\n            break;\n    }\n    return r;\n}\n\nfunc main(): i32 {\n    println($\"{f(2999)}\");\n    return 0;\n}\n"
    c["statements-20000"] = HDR + "func main(): i32 {\n    let x: i32 = 0;\n" + "".join(f"    x = x + {i % 17};\n" for i in range(20000)) + "    println($\"{x}\");\n    return 0;\n}\n"
    c["locals-8000"] = HDR + "func main(): i32 {\n" + "".join(f"    let v{i}: i32 = {i % 100};\n" for i in range(8000)) + "    println($\"{v7999}\");\n    return 0;\n}\n"
    c["functions-3000"] = HDR + "".join(f"func f{i}(x: i32): i32 {{\n    return x + {i};\n}}\n\n" for i in range(3000)) + "func main(): i32 {\n    println($\"{f2999(1)}\");\n    return 0;\n}\n"
    return c


def baseline(record: Path) -> dict[tuple[str, str], list[tuple[int, int]]]:
    rows: dict[tuple[str, str], list[tuple[int, int]]] = {}
    with record.open() as f:
        for r in csv.DictReader(f, delimiter="\t"):
            if r["accepted"] != "yes":
                continue
            for comp in ("cc", "gcc", "vbcc"):
                if r[comp] == "ok":
                    rows.setdefault((comp, r["profile"]), []).append((int(r["c_lines"]), int(r[comp + "_ms"])))
    return rows


def limit(base, comp, profile, lines):
    near = min(base[(comp, profile)], key=lambda r: abs(r[0] - lines))
    return 10 * near[1], near


def main() -> int:
    clc = Path(sys.argv[1]).resolve()
    base = baseline(ROOT / sys.argv[2])
    out = ROOT / (sys.argv[3] if len(sys.argv) > 3 else "build/tmp/assessment/WS3/shapes")
    out.mkdir(parents=True, exist_ok=True)
    failures = 0
    compilers = {
        ("cc", "host"): ["cc", "-std=c99", "-O2", "-c"],
        ("gcc", "hosted-amiga"): ["m68k-amigaos-gcc", "-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2", "-c"],
        ("vbcc", "hosted-amiga"): ["vc", "+aos68k", "-c99", "-O2", "-cpu=68000",
                                   "-I/opt/amiga/m68k-amigaos/ndk-include", "-c"],
    }
    for name, text in cases().items():
        src = out / f"{name}.clo"
        src.write_text(text)
        for profile in ("host", "hosted-amiga"):
            c_file = out / f"{name}.{profile}.c"
            done = subprocess.run([str(clc), "--profile", profile, "-o", str(c_file), str(src)],
                                  capture_output=True, text=True, timeout=600)
            if done.returncode != 0:
                failures += 1
                print(f"FAIL {name} {profile} clc rc={done.returncode} {done.stderr.strip()[:120]}")
                continue
            lines = c_file.read_text().count("\n")
            for (comp, prof), argv in compilers.items():
                if prof != profile:
                    continue
                cap, near = limit(base, comp, profile, lines)
                start = time.monotonic()
                run = subprocess.run([*argv, "-o", str(out / f"{name}.{comp}.o"), str(c_file)],
                                     capture_output=True, text=True, timeout=1200)
                if run.returncode != 0 and "No config file" in run.stdout + run.stderr:
                    start = time.monotonic()
                    run = subprocess.run([*argv, "-o", str(out / f"{name}.{comp}.o"), str(c_file)],
                                         capture_output=True, text=True, timeout=1200)
                ms = int((time.monotonic() - start) * 1000)
                (out / f"{name}.{comp}.log").write_text(run.stdout + run.stderr)
                ok = run.returncode == 0 and ms <= cap
                failures += not ok
                print(f"{'PASS' if ok else 'FAIL'} {name} {profile} {comp}: rc={run.returncode} {ms}ms "
                      f"(limit {cap}ms from {near[0]} lines/{near[1]}ms) lines={lines}")
    print(f"shapes: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
