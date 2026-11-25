#!/usr/bin/env python3
"""WS7-06/07: compile time and peak RSS scaling of the pinned release `clc`.

Generators the `compile_time.py` series lacks, each producing a valid program whose
size is the parameter N (functions, instantiations, or closures; locals for the
`locals` series that WS1 handed over):

  ownership  N functions, each with three owned `String` locals, one conditional move
             into a `take` parameter, a struct with two `String` fields, and drops on
             two exits
  generic    N struct types, each used through `Array<S_i>` (push, len), so N
             instantiations of the Array surface and its drop glue
  closure    N functions, each passing a capturing lambda to a callable-bound generic
  locals     one function with N `i32` locals (compile_time.local_farm)
  functions  N one-line functions (compile_time.function_farm; the Phase 0 baseline)

For every series and N in 1000, 2000, 4000, 8000 the script compiles to C on the host
profile with `tools/perf/compile_time.measure` (median wall and CPU time of 3 runs,
peak RSS), requires exit 0 and a non-empty C file, and reports the scaling exponent
log2(t(8000) / t(4000)) for wall time and RSS.

Usage (SDK container, repo root): 710-compile-scaling.py <clc> <out-dir> [--seeded]
--seeded adds a series whose program has a type error; the run must report it as a
failure (the comparator rejects an output that differs from "compiles").
"""
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import compile_time  # noqa: E402

SIZES = (1000, 2000, 4000, 8000)
RUNS = 3


def ownership(n: int) -> str:
    lines = ["module bench.Ownership;", "", "import { stringOf, bytes } from std.text;", "",
             "struct Pair {", "    a: String;", "    b: String;", "}", "",
             "func consume(take s: String): usize {", "    return len(bytes(s));", "}", ""]
    for i in range(n):
        lines += [f"func work{i}(flag: bool): usize {{",
                  f"    let s: String = stringOf(\"a{i % 97}\");",
                  "    let t: String = stringOf(\"b\");",
                  "    let p: Pair = Pair { a: stringOf(\"x\"), b: stringOf(\"y\") };",
                  "    if (flag) {",
                  "        return consume(s) + len(bytes(p.a));",
                  "    }",
                  "    return len(bytes(t)) + len(bytes(p.b));",
                  "}", ""]
    lines += ["func main(): i32 {", "    let total: usize = 0;"]
    lines += [f"    total = total + work{i}(true);" for i in range(min(n, 10))]
    lines += ["    return i32(total);", "}"]
    return "\n".join(lines) + "\n"


def generic(n: int) -> str:
    lines = ["module bench.Generic;", "", "import { Array, arrayNew } from std.mem;", ""]
    for i in range(n):
        lines += [f"struct S{i} {{", "    v: i32;", "}", "",
                  f"func use{i}(): usize {{",
                  f"    let xs: Array<S{i}> = arrayNew<S{i}>();",
                  f"    xs.push(S{i} {{ v: {i % 97} }});",
                  "    return xs.len();", "}", ""]
    lines += ["func main(): i32 {", "    let total: usize = 0;"]
    lines += [f"    total = total + use{i}();" for i in range(min(n, 10))]
    lines += ["    return i32(total);", "}"]
    return "\n".join(lines) + "\n"


def closure(n: int) -> str:
    lines = ["module bench.Closure;", "",
             "func apply<F: func(i32): i32>(f: F, x: i32): i32 {", "    return f(x);", "}", ""]
    for i in range(n):
        lines += [f"func call{i}(): i32 {{", f"    let k: i32 = {i % 97};",
                  "    return apply([k] (x: i32) => x + k, 1);", "}", ""]
    lines += ["func main(): i32 {", "    let total: i32 = 0;"]
    lines += [f"    total = total + call{i}();" for i in range(min(n, 10))]
    lines += ["    return total;", "}"]
    return "\n".join(lines) + "\n"


def seeded(n: int) -> str:
    return "module bench.Seeded;\n\nfunc main(): i32 {\n    let x: i32 = true;\n    return x;\n}\n"


SERIES = {"ownership": ownership, "generic": generic, "closure": closure,
          "locals": compile_time.local_farm, "functions": compile_time.function_farm}


def main() -> int:
    clc, out = Path(sys.argv[1]), Path(sys.argv[2])
    series = dict(SERIES)
    if "--seeded" in sys.argv:
        series = {"seeded": seeded}
    out.mkdir(parents=True, exist_ok=True)
    failures = 0
    for name, gen in series.items():
        times, rss = {}, {}
        for n in SIZES:
            src = out / f"{name}-{n}.clo"
            src.write_text(gen(n))
            c_out = out / f"{name}-{n}.c"
            if c_out.exists():
                c_out.unlink()
            record = compile_time.measure([str(clc), "--profile", "host", "-o", str(c_out), str(src)], RUNS, timeout=600)
            ok = record["exit_codes"] == [0] and c_out.exists() and c_out.stat().st_size > 0
            failures += not ok
            times[n] = record["median_ns"]
            rss[n] = record["max_rss_kb"]
            print(f"{'PASS' if ok else 'FAIL'} {name}-{n}: exit {record['exit_codes']} "
                  f"wall {record['median_ns'] // 1_000_000} ms cpu {record['median_cpu_ns'] // 1_000_000} ms "
                  f"rss {record['max_rss_kb']} KB c {c_out.stat().st_size if c_out.exists() else 0} B")
            if not ok:
                break
        if len(times) == len(SIZES):
            et = math.log2(times[8000] / max(1, times[4000]))
            er = math.log2(rss[8000] / max(1, rss[4000]))
            print(f"exponent {name}: time {et:.2f} rss {er:.2f}")
    print(f"compile-scaling: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
