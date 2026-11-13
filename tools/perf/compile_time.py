#!/usr/bin/env python3
"""Compiler-throughput harness for the Clorraine front end.

The scored benchmark suite measures generated-code quality. This harness
measures the other axis: how long ``clc`` takes and how much memory it uses
while compiling the repository's own inputs. It records one JSON document per
run and can compare two documents:

* the full specification corpus, including the compile-fail fixtures;
* the committed benchmark programs;
* the largest Amiga example programs;
* deterministic generated inputs that stress function count, call count,
  statement count, and local count.

Wall time is measured with ``perf_counter_ns`` around a forked child.
Peak resident set size comes from ``wait4``'s ``ru_maxrss`` (KiB on Linux).
Every child is bounded by an alarm and writes to ``/dev/null``, so a hung
compiler cannot wedge the harness and no program output pollutes the table.

Only the Python standard library is used. Generated inputs and result
documents stay under ``build/perf/``.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import statistics
import sys
import tempfile
import time
from pathlib import Path

import bench

SCHEMA = 1
BUILD = bench.BUILD
INPUT_ROOT = BUILD / "compile-time" / "inputs"
DEFAULT_RUNS = 5
DEFAULT_TIMEOUT = 120
CORPUS_PROFILE = "host"
BENCHMARK_PROFILE = "host"
REAL_PROFILE = "hosted-amiga"

# The largest committed Amiga examples. They exercise the standard library
# and the ownership/codegen paths a hand-written program reaches.
REAL_PROGRAMS = (
    "examples/amiga/boing.clo",
    "examples/amiga/exec-demo.clo",
    "examples/amiga/gui.clo",
    "examples/amiga/imageviewer.clo",
    "examples/amiga/services.clo",
    "examples/amiga/workbench.clo",
)

# Generated shapes. The names are stable so two documents compare directly.
GENERATED_FUNCTIONS = (1000, 2000, 4000, 8000)
GENERATED_CALLS = 4000
GENERATED_STATEMENTS = 4000
GENERATED_LOCALS = 4000
GENERATED_MIXED = 4000


class ThroughputError(RuntimeError):
    """A failed measurement step with the exact context."""


def function_farm(count: int) -> str:
    """Return a program with ``count`` small functions and one caller.

    Every declaration is independent, so the shape isolates the per-function
    cost of loading, ownership checking, and emission. The body calls only the
    first ten functions, keeping the call graph constant while the function
    count grows.
    """
    lines = ["module bench.GeneratedFunctions;", ""]
    for index in range(count):
        lines.append(
            f"func work{index}(value: i32): i32 {{ return value + {index % 97}; }}"
        )
    lines.extend(["", "func main(): i32 {", "    let total: i32 = 0;"])
    for index in range(min(count, 10)):
        lines.append(f"    total = total + work{index}({index});")
    lines.extend(["    return total;", "}"])
    return "\n".join(lines) + "\n"


def call_farm(count: int) -> str:
    """Return a program with ``count`` calls to one small function."""
    lines = [
        "module bench.GeneratedCalls;",
        "",
        "func target(value: i32): i32 { return value + 1; }",
        "",
        "func main(): i32 {",
        "    let total: i32 = 0;",
    ]
    for index in range(count):
        lines.append(f"    total = total + target({index % 31});")
    lines.extend(["    return total;", "}"])
    return "\n".join(lines) + "\n"


def statement_farm(count: int) -> str:
    """Return a program with ``count`` assignments in one function."""
    lines = [
        "module bench.GeneratedStatements;",
        "",
        "func main(): i32 {",
        "    let total: i32 = 0;",
    ]
    for index in range(count):
        lines.append(f"    total = total + {index % 97};")
    lines.extend(["    return total;", "}"])
    return "\n".join(lines) + "\n"


def local_farm(count: int) -> str:
    """Return a program with ``count`` locals in one function."""
    lines = [
        "module bench.GeneratedLocals;",
        "",
        "func main(): i32 {",
        "    let total: i32 = 0;",
    ]
    for index in range(count):
        lines.append(f"    let local{index}: i32 = {index % 97};")
        lines.append(f"    total = total + local{index};")
    lines.extend(["    return total;", "}"])
    return "\n".join(lines) + "\n"


def mixed_farm(count: int) -> str:
    """Return a program with ``count`` functions and one call to each.

    This is the shape a code generator produces when it emits one helper per
    item and a driver that uses every helper. It combines the function-count
    and call-count costs in one file.
    """
    lines = ["module bench.GeneratedMixed;", ""]
    for index in range(count):
        lines.append(
            f"func work{index}(value: i32): i32 {{ return value + {index % 97}; }}"
        )
    lines.extend(["", "func main(): i32 {", "    let total: i32 = 0;"])
    for index in range(count):
        lines.append(f"    total = total + work{index}({index % 31});")
    lines.extend(["    return total;", "}"])
    return "\n".join(lines) + "\n"


def generated_sources() -> dict[str, str]:
    """Return every generated workload's stable name and source text."""
    sources: dict[str, str] = {}
    for count in GENERATED_FUNCTIONS:
        sources[f"functions-{count}"] = function_farm(count)
    sources[f"calls-{GENERATED_CALLS}"] = call_farm(GENERATED_CALLS)
    sources[f"statements-{GENERATED_STATEMENTS}"] = statement_farm(GENERATED_STATEMENTS)
    sources[f"locals-{GENERATED_LOCALS}"] = local_farm(GENERATED_LOCALS)
    sources[f"mixed-{GENERATED_MIXED}"] = mixed_farm(GENERATED_MIXED)
    return sources


def write_generated_inputs(root: Path) -> dict[str, Path]:
    """Write the generated inputs under ``root`` and return their paths."""
    root.mkdir(parents=True, exist_ok=True)
    paths: dict[str, Path] = {}
    for name, text in generated_sources().items():
        path = root / f"{name}.clo"
        path.write_text(text, encoding="utf-8")
        paths[name] = path
    return paths


def summarize(
    samples: list[int],
    cpu_samples: list[int],
    max_rss_kb: int,
    codes: list[int],
) -> dict[str, object]:
    """Build one workload record from the raw wall and CPU samples.

    The median is the decision number: a single scheduling hiccup moves the
    minimum and the maximum, not the center of a short run. CPU time is
    recorded alongside wall time because it is not perturbed by other work on
    a shared machine; for this single-threaded compiler it is the cleaner
    signal on small inputs.
    """
    if not samples or len(samples) != len(cpu_samples):
        raise ThroughputError("a workload needs one CPU sample per wall sample")
    return {
        "median_ns": int(statistics.median(samples)),
        "min_ns": min(samples),
        "max_ns": max(samples),
        "median_cpu_ns": int(statistics.median(cpu_samples)),
        "min_cpu_ns": min(cpu_samples),
        "max_cpu_ns": max(cpu_samples),
        "max_rss_kb": int(max_rss_kb),
        "exit_codes": sorted(set(codes)),
    }


def measure(argv: list[str], runs: int, timeout: int = DEFAULT_TIMEOUT) -> dict[str, object]:
    """Run ``argv`` ``runs`` times and return its timing and memory record."""
    if runs < 1:
        raise ThroughputError("--runs must be positive")
    samples: list[int] = []
    cpu_samples: list[int] = []
    codes: list[int] = []
    max_rss_kb = 0
    for _ in range(runs):
        start = time.perf_counter_ns()
        pid = os.fork()
        if pid == 0:
            # The child replaces itself immediately. The alarm is the only
            # bound if the compiler loops; the default SIGALRM action exits.
            try:
                signal.alarm(timeout)
                devnull = os.open(os.devnull, os.O_RDWR)
                os.dup2(devnull, 1)
                os.dup2(devnull, 2)
                os.execv(argv[0], argv)
            finally:
                os._exit(127)
        _, status, usage = os.wait4(pid, 0)
        samples.append(time.perf_counter_ns() - start)
        cpu_samples.append(int((usage.ru_utime + usage.ru_stime) * 1_000_000_000))
        max_rss_kb = max(max_rss_kb, usage.ru_maxrss)
        if os.WIFEXITED(status):
            codes.append(os.WEXITSTATUS(status))
        elif os.WIFSIGNALED(status):
            codes.append(128 + os.WTERMSIG(status))
        else:
            codes.append(255)
    return summarize(samples, cpu_samples, max_rss_kb, codes)


def compile_record(
    clc: Path, profile: str, source: Path, output: Path, runs: int
) -> dict[str, object]:
    """Compile one source and return its timing record."""
    argv = [str(clc), "--profile", profile, "-o", str(output), str(source)]
    record = measure(argv, runs)
    record["path"] = str(source.relative_to(bench.ROOT))
    record["profile"] = profile
    return record


def require_success(name: str, record: dict[str, object]) -> None:
    """Reject a workload that did not compile cleanly on every run."""
    if record["exit_codes"] != [0]:
        raise ThroughputError(f"{name}: compiler exited with {record['exit_codes']}")


def corpus_workload(clc: Path, output: Path, runs: int) -> dict[str, object]:
    """Measure every specification-corpus file once and aggregate.

    The aggregate median is the sum of the per-file medians, not a median of
    whole-corpus totals: the corpus has no single invocation, and a sum keeps
    the per-file detail available. The aggregate minimum and maximum are sums
    of the per-file bounds for the same reason. Compile-fail fixtures are
    expected to exit nonzero, so their codes are recorded rather than
    rejected.
    """
    files = sorted(bench.ROOT.glob("spec/tests/**/*.clo"))
    items: dict[str, dict[str, object]] = {}
    median_total = 0
    min_total = 0
    max_total = 0
    median_cpu_total = 0
    min_cpu_total = 0
    max_cpu_total = 0
    max_rss_kb = 0
    ok = 0
    failed = 0
    for path in files:
        record = compile_record(clc, CORPUS_PROFILE, path, output, runs)
        items[str(path.relative_to(bench.ROOT))] = record
        median_total += int(record["median_ns"])
        min_total += int(record["min_ns"])
        max_total += int(record["max_ns"])
        median_cpu_total += int(record["median_cpu_ns"])
        min_cpu_total += int(record["min_cpu_ns"])
        max_cpu_total += int(record["max_cpu_ns"])
        max_rss_kb = max(max_rss_kb, int(record["max_rss_kb"]))
        if record["exit_codes"] == [0]:
            ok += 1
        else:
            failed += 1
    return {
        "category": "corpus",
        "profile": CORPUS_PROFILE,
        "files": len(files),
        "ok": ok,
        "failed": failed,
        "median_ns": median_total,
        "min_ns": min_total,
        "max_ns": max_total,
        "median_cpu_ns": median_cpu_total,
        "min_cpu_ns": min_cpu_total,
        "max_cpu_ns": max_cpu_total,
        "max_rss_kb": max_rss_kb,
        "items": items,
    }


def named_workloads(
    clc: Path, output: Path, runs: int
) -> dict[str, dict[str, object]]:
    """Measure the benchmark, example, and generated workloads."""
    workloads: dict[str, dict[str, object]] = {}
    for path in sorted(bench.BENCHMARKS.glob("*/program.clo")):
        name = f"benchmark/{path.parent.name}"
        record = compile_record(clc, BENCHMARK_PROFILE, path, output, runs)
        require_success(name, record)
        workloads[name] = record

    for relative in REAL_PROGRAMS:
        path = bench.ROOT / relative
        name = f"real/{path.stem}"
        record = compile_record(clc, REAL_PROFILE, path, output, runs)
        require_success(name, record)
        workloads[name] = record

    inputs = write_generated_inputs(INPUT_ROOT)
    for name, path in inputs.items():
        key = f"generated/{name}"
        record = compile_record(clc, CORPUS_PROFILE, path, output, runs)
        require_success(key, record)
        workloads[key] = record
    return workloads


def load_document(file: Path) -> dict[str, object]:
    """Load and validate one throughput result document."""
    if not file.is_file():
        raise ThroughputError(f"missing result document: {file}")
    try:
        document = json.loads(file.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ThroughputError(f"malformed result document: {file}: {error}") from error
    required = {"schema", "label", "compiler", "toolchain", "runs", "workloads"}
    missing = sorted(required - set(document))
    if missing:
        raise ThroughputError(f"{file}: missing fields: {', '.join(missing)}")
    if document["schema"] != SCHEMA:
        raise ThroughputError(f"{file}: unsupported schema {document['schema']}")
    return document


def percent_change(base: int, head: int) -> float:
    """Return the signed percentage change from ``base`` to ``head``."""
    if base == 0:
        return 0.0
    return (head - base) / base * 100.0


def compare_rows(
    base: dict[str, object], head: dict[str, object], metric: str = "median_ns"
) -> list[tuple[str, float, float, float, int, int]]:
    """Return the comparison rows for every workload present in both runs.

    ``metric`` selects the recorded field to compare. The median is the normal
    decision number; the minimum is useful on a shared machine because it is
    the sample least disturbed by scheduling and frequency changes.
    """
    if metric not in ("median_ns", "min_ns", "median_cpu_ns", "min_cpu_ns"):
        raise ThroughputError(f"unknown comparison metric: {metric}")
    base_workloads = base["workloads"]
    head_workloads = head["workloads"]
    assert isinstance(base_workloads, dict) and isinstance(head_workloads, dict)
    if set(base_workloads) != set(head_workloads):
        missing = sorted(set(base_workloads) - set(head_workloads))
        extra = sorted(set(head_workloads) - set(base_workloads))
        raise ThroughputError(
            f"cannot compare: workloads differ (missing {missing}, extra {extra})"
        )
    rows = []
    for name in sorted(base_workloads):
        before = base_workloads[name]
        after = head_workloads[name]
        assert isinstance(before, dict) and isinstance(after, dict)
        rows.append(
            (
                name,
                float(before[metric]),
                float(after[metric]),
                percent_change(int(before[metric]), int(after[metric])),
                int(before["max_rss_kb"]),
                int(after["max_rss_kb"]),
            )
        )
    return rows


def command_run(args: argparse.Namespace) -> int:
    """Measure one compiler and write a result document."""
    bench.validate_label(args.label)
    output = bench.confined(bench.path(args.out), BUILD)
    clc = bench.path(args.clc)
    if not clc.is_file():
        raise ThroughputError(f"compiler not found: {clc}")
    if args.runs < 1:
        raise ThroughputError("--runs must be positive")
    BUILD.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=BUILD, prefix=args.label + "-") as temporary:
        work = Path(temporary)
        emitted = work / "out.c"
        workloads: dict[str, object] = {}
        print("compile-time: corpus", flush=True)
        workloads["corpus"] = corpus_workload(clc, emitted, args.runs)
        for name, record in named_workloads(clc, emitted, args.runs).items():
            print(f"compile-time: {name}", flush=True)
            workloads[name] = record
    document = {
        "schema": SCHEMA,
        "label": args.label,
        "created": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "compiler": bench.compiler_identity(clc, args.compiler_git),
        "harness_git": bench.harness_git(),
        "toolchain": bench.toolchain(),
        "runs": args.runs,
        # A forked child inherits the parent's pages until exec, so every
        # ru_maxrss value has a floor. Recording the floor lets a reader tell
        # a small compile (near the floor) from a memory-heavy one (well
        # above it). /bin/true is the smallest realistic exec target.
        "rss_floor_kb": int(
            measure(["/bin/true"], 1, timeout=30)["max_rss_kb"]
        ),
        "profiles": {
            "corpus": CORPUS_PROFILE,
            "benchmark": BENCHMARK_PROFILE,
            "real": REAL_PROFILE,
            "generated": CORPUS_PROFILE,
        },
        "workloads": workloads,
    }
    bench.write_json(document, output)
    print(f"compile-time: wrote {output}")
    return 0


def require_same_outcomes(base: dict[str, object], head: dict[str, object]) -> None:
    """Reject a comparison that would turn a failure into a speedup.

    A compiler that exits immediately with an error looks very fast. Every
    workload must therefore have the same exit-code set on both sides, and the
    corpus must cover the same files with the same success/failure split.
    """
    base_workloads = base["workloads"]
    head_workloads = head["workloads"]
    assert isinstance(base_workloads, dict) and isinstance(head_workloads, dict)
    for name in sorted(base_workloads):
        before = base_workloads[name]
        after = head_workloads[name]
        assert isinstance(before, dict) and isinstance(after, dict)
        if before.get("exit_codes") != after.get("exit_codes"):
            raise ThroughputError(
                f"cannot compare: {name} exit codes differ "
                f"({before.get('exit_codes')} vs {after.get('exit_codes')})"
            )
        if name != "corpus":
            continue
        for field in ("files", "ok", "failed"):
            if before.get(field) != after.get(field):
                raise ThroughputError(
                    f"cannot compare: corpus {field} differs "
                    f"({before.get(field)} vs {after.get(field)})"
                )
        before_items = before.get("items", {})
        after_items = after.get("items", {})
        if not isinstance(before_items, dict) or not isinstance(after_items, dict):
            raise ThroughputError("cannot compare: malformed corpus items")
        if set(before_items) != set(after_items):
            raise ThroughputError("cannot compare: corpus file sets differ")
        for path in sorted(before_items):
            left = before_items[path].get("exit_codes")
            right = after_items[path].get("exit_codes")
            if left != right:
                raise ThroughputError(
                    f"cannot compare: {path} exit codes differ ({left} vs {right})"
                )


def command_compare(args: argparse.Namespace) -> int:
    """Print the base/head table and optionally enforce a regression limit."""
    base = load_document(bench.path(args.base))
    head = load_document(bench.path(args.head))
    if base["runs"] != head["runs"]:
        raise ThroughputError(
            f"cannot compare: runs differ ({base['runs']} vs {head['runs']})"
        )
    for field in ("toolchain", "profiles"):
        if base.get(field) != head.get(field):
            raise ThroughputError(
                f"cannot compare: {field} differs "
                f"({base.get(field)!r} vs {head.get(field)!r})"
            )
    require_same_outcomes(base, head)
    rows = compare_rows(base, head, args.metric)
    print(
        f"{'workload':32} {'base ms':>10} {'head ms':>10} {'delta':>8} "
        f"{'base rss':>9} {'head rss':>9}"
    )
    regressions = []
    for name, before, after, delta, base_rss, head_rss in rows:
        print(
            f"{name:32} {before / 1e6:10.2f} {after / 1e6:10.2f} "
            f"{delta:+7.1f}% {base_rss:9d} {head_rss:9d}"
        )
        if args.limit is not None and delta > args.limit:
            regressions.append((name, delta))
    if regressions:
        for name, delta in regressions:
            print(
                f"compile-time: {name} regressed {delta:+.1f}% "
                f"(limit {args.limit:+.1f}%)",
                file=sys.stderr,
            )
        return 1
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse the harness command line."""
    parser = argparse.ArgumentParser(prog="compile_time.py")
    sub = parser.add_subparsers(dest="command", required=True)

    run = sub.add_parser("run", help="measure one compiler")
    run.add_argument("--clc", default=str(bench.ROOT / "target" / "debug" / "clc"))
    run.add_argument("--compiler-git", default=None)
    run.add_argument("--label", default="compile-head")
    run.add_argument("--out", required=True)
    run.add_argument("--runs", type=int, default=DEFAULT_RUNS)
    run.set_defaults(func=command_run)

    compare = sub.add_parser("compare", help="compare two result documents")
    compare.add_argument(
        "--limit",
        type=float,
        default=None,
        metavar="PERCENT",
        help="fail when a workload regresses by more than PERCENT",
    )
    compare.add_argument(
        "--metric",
        choices=("median_ns", "min_ns", "median_cpu_ns", "min_cpu_ns"),
        default="median_ns",
        help="timing field to compare (default: median_ns)",
    )
    compare.add_argument("base")
    compare.add_argument("head")
    compare.set_defaults(func=command_compare)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    """Run the harness and translate a harness error into exit 2."""
    args = parse_args(argv)
    try:
        return args.func(args)
    except (ThroughputError, bench.HarnessError) as error:
        print(f"compile-time: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
