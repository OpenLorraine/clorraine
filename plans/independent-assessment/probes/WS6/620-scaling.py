#!/usr/bin/env python3
"""WS6-03: hidden costs — does a common stdlib operation scale linearly?

Each workload is a Clorraine program with one size parameter N. The script builds it
for the A500 lens (hosted-amiga-13, m68k GCC -O2, the shipped release flags), counts
the dynamic 68000 instructions under `vamos` at N, 2N, and 4N, and reports

    ratio = (I(4N) - I(2N)) / (I(2N) - I(N))

which is about 2 for linear work and about 4 for quadratic work. A workload passes
when the ratio is at most 2.5. The `seeded-quadratic` workload is the detector: it
must be reported (ratio at least 3.5), or the run fails.

Usage (SDK container, repo root): 620-scaling.py <clc> <out-dir> [--seeded-only]
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402

FLAGS = ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv", "-O2", "-ffunction-sections",
         "-fdata-sections", "-Wl,--gc-sections", "-s"]
BASE_N = 256

HEAD = """module Probe.Main;

import { println } from std.io;
import { Array, arrayNew, Buffer, bufferNew, bufferBytes, bufferFrom, bufferLen } from std.mem;
import { padLeft, toDecimal } from std.fmt;
import { clone, stringOf, bytes } from std.text;
import { ilbmPixelHash } from std.iff;

func main(): i32 {
    let n: i32 = @N@;
"""
WORKLOADS = {
    "array-push": """    let xs: Array<i32> = arrayNew<i32>();
    let i: i32 = 0;
    while (i < n) { xs.push(i); i = i + 1; }
    println($"{xs.len()}");
""",
    "pad-left": """    let s: String = padLeft("x", usize(n), ' ');
    println($"{len(bytes(s))}");
""",
    "clone": """    let s: String = padLeft("x", usize(n), ' ');
    let t: String = clone(s);
    println($"{len(bytes(t))}");
""",
    "template-loop": """    let total: usize = 0;
    let i: i32 = 0;
    while (i < n) { let s: String = $"{i}-{i}"; total = total + len(bytes(s)); i = i + 1; }
    println($"{total}");
""",
    "decimal-loop": """    let total: usize = 0;
    let i: i32 = 0;
    while (i < n) { let s: String = toDecimal(i); total = total + len(bytes(s)); i = i + 1; }
    println($"{total}");
""",
    "buffer-from-hash": """    let b: Buffer? = bufferNew(usize(n));
    if (b == null) { return 1; }
    let b1: Buffer = b;
    let c: Buffer? = bufferFrom(bufferBytes(b1));
    if (c == null) { return 1; }
    let c1: Buffer = c;
    println($"{ilbmPixelHash(bufferBytes(c1))} {bufferLen(c1)}");
""",
    "string-of": """    let s: String = padLeft("x", usize(n), ' ');
    let t: String = stringOf(s);
    println($"{len(bytes(t))}");
""",
    "seeded-quadratic": """    let total: i32 = 0;
    let i: i32 = 0;
    while (i < n) { let j: i32 = 0; while (j < n) { total = total + 1; j = j + 1; } i = i + 1; }
    println($"{total}");
""",
}


def count(clc: Path, name: str, n: int, out: Path) -> int:
    work = out / f"{name}-{n}"
    work.mkdir(parents=True, exist_ok=True)
    src = work / "prog.clo"
    src.write_text(HEAD.replace("@N@", str(n)) + WORKLOADS[name] + "    return 0;\n}\n")
    c = work / "prog.c"
    bench.emit(clc, "hosted-amiga-13", src, c)
    binary = work / "prog"
    bench.build_step(["m68k-amigaos-gcc", *FLAGS, "-o", str(binary), str(c), "-lamiga", "-lm"],
                     bench.M68K_BUILD_TIMEOUT)
    _, instructions = bench.trace_vamos(binary)
    return instructions


def main() -> int:
    clc, out = Path(sys.argv[1]), Path(sys.argv[2])
    names = ["seeded-quadratic"] if "--seeded-only" in sys.argv else list(WORKLOADS)
    failures = 0
    for name in names:
        counts = [count(clc, name, BASE_N * k, out) for k in (1, 2, 4)]
        ratio = (counts[2] - counts[1]) / max(1, counts[1] - counts[0])
        if name == "seeded-quadratic":
            ok = ratio >= 3.5
        else:
            ok = ratio <= 2.5
        failures += not ok
        per = (counts[2] - counts[1]) / (BASE_N * 2)
        print(f"{'PASS' if ok else 'FAIL'} {name}: N={BASE_N},{BASE_N*2},{BASE_N*4} "
              f"instr={counts} ratio={ratio:.2f} marginal={per:.1f}/element")
    print(f"scaling: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
