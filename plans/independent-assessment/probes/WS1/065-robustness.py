#!/usr/bin/env python3
"""WS1-10 robustness probe: pathological inputs must not crash or hang clc.

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS1/065-robustness.py <pinned-clc> <compile-baseline.json> [<out-dir>]

Each case generates one source under the output folder and runs
``clc --profile host`` on it. Expectation (065-robustness.expected): exit 0 or
1 (a diagnostic), never a panic, a signal, or a timeout; wall time and peak
RSS at most 10x the compile-time baseline of the generated workload nearest in
source size, scaled linearly by size above the largest baseline input. With
``--seeded`` the checker is run against a seeded failure (a 1 ms time limit
on the first case) to show it reports a violation.
"""

import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
HDR = "module Probe.Robust;\n\nimport { println } from std.io;\n\n"


def wrap(body: str) -> str:
    return HDR + "func main(): i32 {\n" + body + "\n    return 0;\n}\n"


def cases() -> dict[str, str]:
    c = {}
    c["parens-10k"] = wrap("    let x: i32 = " + "(" * 10000 + "1" + ")" * 10000 + ";\n    println($\"{x}\");")
    c["parens-100k"] = wrap("    let x: i32 = " + "(" * 100000 + "1" + ")" * 100000 + ";\n    println($\"{x}\");")
    c["blocks-10k"] = wrap("    " + "{ " * 10000 + "println(\"deep\");" + " }" * 10000)
    c["unary-100k"] = wrap("    let x: i32 = " + "-(" * 100000 + "1" + ")" * 100000 + ";\n    println($\"{x}\");")
    c["addchain-100k"] = wrap("    let x: i32 = " + " + ".join(["1"] * 100000) + ";\n    println($\"{x}\");")
    c["ifchain-5k"] = wrap("    let n: i32 = 4999;\n    " + " else ".join(
        f"if (n == {i}) {{ println(\"{i}\"); }}" for i in range(5000)))
    c["switch-5k"] = wrap("    let n: i32 = 4999;\n    switch (n) {\n" + "".join(
        f"        case {i}:\n            println(\"{i}\");\n            break;\n" for i in range(5000))
        + "        default:\n            break;\n    }")
    c["ident-1m"] = wrap("    let " + "a" * 1_000_000 + ": i32 = 1;\n    println($\"{" + "a" * 1_000_000 + "}\");")
    c["literal-10k-digits"] = wrap("    let x = " + "1" * 10000 + ";\n    println($\"{x}\");")
    c["string-1mb"] = HDR + "const S: static str = \"" + "a" * 1_000_000 + "\";\n\nfunc main(): i32 {\n    println($\"{len(S)}\");\n    return 0;\n}\n"
    c["calls-10k"] = HDR + "func f(x: i32): i32 {\n    return x + 1;\n}\n\nfunc main(): i32 {\n    let x: i32 = " + "f(" * 10000 + "0" + ")" * 10000 + ";\n    println($\"{x}\");\n    return 0;\n}\n"
    c["locals-50k"] = wrap("".join(f"    let v{i}: i32 = {i % 100};\n" for i in range(50000)) + "    println($\"{v49999}\");")
    c["template-10k"] = wrap("    let n: i32 = 1;\n    let s = $\"" + "{n}," * 10000 + "\";\n    println(s);")
    c["array-literal-100k"] = HDR + "const T: i32[100000] = [" + ", ".join(str(i % 1000) for i in range(100000)) + "];\n\nfunc main(): i32 {\n    println($\"{T[99999]}\");\n    return 0;\n}\n"
    c["generic-recursion"] = HDR + ("struct W<T> {\n    v: T;\n}\n\nfunc g<T>(x: T, n: i32): i32 {\n"
                                    "    if (n == 0) {\n        return 0;\n    }\n"
                                    "    let w: W<T> = W<T> { v: x };\n    return g<W<T>>(w, n - 1);\n}\n\n"
                                    "func main(): i32 {\n    println($\"{g<i32>(1, 3)}\");\n    return 0;\n}\n")
    return c


def measured(clc: Path, src: Path, out_c: Path, timeout: int) -> tuple[int, float, int, str]:
    """Run one case in a child Python so ru_maxrss covers that clc run only."""
    code = ("import resource,subprocess,sys,time;s=time.monotonic();"
            "p=subprocess.run(sys.argv[1:],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,"
            f"timeout={timeout});"
            "print(p.returncode,time.monotonic()-s,resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss);"
            "sys.stderr.write(p.stderr.decode('utf-8','replace'))")
    try:
        done = subprocess.run([sys.executable, "-c", code, str(clc), "--profile", "host", "-o",
                               str(out_c), str(src)], capture_output=True, text=True,
                              timeout=timeout + 30)
    except subprocess.TimeoutExpired:
        return -999, float(timeout), 0, "timeout"
    if done.returncode != 0 or not done.stdout.strip():
        return -999, float(timeout), 0, done.stderr[-400:]
    rc, secs, rss = done.stdout.split()
    return int(rc), float(secs), int(rss), done.stderr


def baseline(path: Path) -> list[tuple[int, float, int]]:
    """(source bytes, median ms, max RSS KB) for each generated and benchmark workload."""
    doc = json.loads(path.read_text())
    rows = []
    for key, w in doc["workloads"].items():
        if "path" not in w:
            continue
        src = ROOT / w["path"]
        if src.is_file():
            rows.append((src.stat().st_size, w["median_ns"] / 1e6, w["max_rss_kb"]))
    return sorted(rows)


def limits(size: int, base: list[tuple[int, float, int]]) -> tuple[float, int]:
    near = min(base, key=lambda r: abs(r[0] - size))
    scale = max(1.0, size / near[0]) if near == base[-1] else 1.0
    return 10 * near[1] * scale, int(10 * near[2] * scale)


def main() -> int:
    seeded = "--seeded" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--seeded"]
    clc = Path(args[0]).resolve()
    base = baseline(ROOT / args[1])
    out = ROOT / (args[2] if len(args) > 2 else "build/tmp/assessment/WS1/robustness")
    out.mkdir(parents=True, exist_ok=True)
    failures = 0
    for i, (name, text) in enumerate(cases().items()):
        src = out / f"{name}.clo"
        src.write_text(text)
        size = src.stat().st_size
        limit_ms, limit_rss = limits(size, base)
        if seeded and i == 0:
            limit_ms = 0.001
        rc, secs, rss, err = measured(clc, src, out / f"{name}.c", timeout=600)
        (out / f"{name}.log").write_text(err)
        code = next((tok for tok in err.split() if tok.startswith("E") and tok[1:6].isdigit()), "-")
        panicked = "panicked" in err or "overflowed its stack" in err
        ok = rc in (0, 1) and not panicked and secs * 1000 <= limit_ms and rss <= limit_rss
        failures += not ok
        print(f"{'PASS' if ok else 'FAIL'} {name}: rc={rc} {secs*1000:.0f}ms (limit {limit_ms:.0f}) "
              f"rss={rss}KB (limit {limit_rss}) size={size} first={code}"
              + (" PANIC" if panicked else ""))
        if seeded:
            break
    print(f"robustness: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
