<!-- title: [bug] Stop clc overflowing its stack on deeply nested code -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:semantics,tier:core,bug -->
<!-- parent: R4 -->

## Summary
Generated or table-like sources (long sums, deep `else if` dispatch, nested helper calls) crash the compiler with no diagnostic; the crash also takes down `clorraine-lsp`, which runs the same analysis on every keystroke.

| Field | Value |
|---|---|
| Finding | ASM-WS1-004 · DEF-590 |
| Severity / Priority / Rank | Critical / P0 / 9 of 58 |
| Domain / Lens | semantics / host (the compiler process; every profile is affected) |
| Confidence / Verdict | verified / confirmed (Critical) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — `probes/WS1/065-robustness.py`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/065-robustness.py build/tmp/assessment/baseline/clc plans/independent-assessment/findings/records/assessment-baseline-compile.json build/tmp/assessment/WS1/robustness"`. Expected (03-01 §Severity, `065-robustness.expected`): exit 0 or a coded diagnostic, never a panic or signal. Observed: `thread 'main' has overflowed its stack … aborting` (signal 6) for parens-10k (20 KB source), parens-100k, blocks-10k, unary-100k, addchain-100k, ifchain-5k, calls-10k. Bisection (`build/tmp/assessment/WS1/bisect.py`): the first failing size is 758 nested calls, 1,172 terms in one `+` chain, 1,378 nested blocks, 3,819 nested parentheses, 4,310 `else if` arms. `--parse-only` accepts a 3,000-term chain and 2,000 nested calls, so the overflow is in a later stage.

Probe `plans/independent-assessment/probes/WS1/065-robustness.py` (baseline `e24a8752`):

```python
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
… (46 more lines in the file)
```

## Root cause
recursive tree walks with large frames on the default 8 MB main-thread stack and no nesting limit.

Location: the recursive stages after parsing (sema, ownership, lowering, codegen) run on the main thread with no depth limit; no `stack_size`/depth guard exists anywhere in `compiler/src` (grep at e24a8752)

## Recommended fix
run the compiler pipeline on a thread with a larger explicit stack and add a nesting-depth limit with a coded diagnostic (an implementation limit), or convert the deepest walks (left-associated binary chains, `else if` chains) to iteration.

## Acceptance criteria
- The behavior the specification requires: exit 0 or a coded diagnostic, never a panic or signal (source: 03-01 §Severity, `065-robustness.expected`)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS7-06; the LSP shares `analysis.rs`. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS1-semantic-core.md`.
