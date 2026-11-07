#!/usr/bin/env python3
"""Deterministic benchmark harness for the Clorraine generated C.

The harness emits each benchmark with a chosen ``clc``, builds the host and
m68k artifacts, runs them, and records the metrics that the optimization plan
uses: emitted-C size, linked section sizes, host wall time, and the m68k
dynamic instruction count from a streamed ``vamos`` trace.

Only the Python standard library is used. All generated files stay under
``build/perf/`` and every subprocess runs with an argument list and a timeout.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

SCHEMA = 1
ROOT = Path(__file__).resolve().parents[2]
BUILD = ROOT / "build" / "perf"
BENCHMARKS = ROOT / "benchmarks"
CALIBRATION = "calibration"
DEFAULT_RUNS = 30

# The m68k compile matrix used by the repository.
M68K_BASE = ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv"]
HOST_DEBUG = ["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]
HOST_RELEASE = ["-std=c99", "-O2"]
M68K_DEBUG = M68K_BASE + ["-O0", "-g", "-Wall", "-Wextra", "-Werror"]
M68K_RELEASE = M68K_BASE + ["-O2"]

# Timeouts in seconds. A trace is much slower than a normal run.
EMIT_TIMEOUT = 120
HOST_BUILD_TIMEOUT = 120
M68K_BUILD_TIMEOUT = 300
HOST_RUN_TIMEOUT = 30
VAMOS_TIMEOUT = 60
TRACE_TIMEOUT = 300

LABEL_RE = re.compile(r"^[A-Za-z0-9._-]+$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
INSTRUCTION_RE = re.compile(r"instr:")
SIZE_ROW_RE = re.compile(
    r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+\d+\s+[0-9a-fA-F]+\s+(.+)$"
)


class HarnessError(RuntimeError):
    """A failed tool step with the exact command that failed."""


def path(value: str) -> Path:
    """Resolve a command-line path against the repository root."""
    candidate = Path(value)
    return candidate if candidate.is_absolute() else ROOT / candidate


def run_command(argv: list[str], *, timeout: int, cwd: Path | None = None) -> str:
    """Run one command, fail loudly, and return its stdout."""
    try:
        result = subprocess.run(
            argv,
            cwd=cwd or ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except FileNotFoundError as error:
        raise HarnessError(f"missing tool: {argv[0]}") from error
    except subprocess.TimeoutExpired as error:
        raise HarnessError(f"timeout after {timeout}s: {' '.join(argv)}") from error
    if result.returncode != 0:
        raise HarnessError(
            f"command failed ({result.returncode}): {' '.join(argv)}\n"
            f"{result.stderr.strip()}"
        )
    return result.stdout


def first_line(text: str) -> str:
    """Return the first non-empty line, or an empty string."""
    for line in text.splitlines():
        if line.strip():
            return line.strip()
    return ""


def tool_version(argv: list[str], timeout: int = 30) -> str:
    """Return a tool's first version line, or a stable fallback."""
    try:
        return first_line(run_command(argv, timeout=timeout))
    except HarnessError:
        return "unknown"


def vamos_version() -> str:
    """Return the amitools package version; ``vamos`` has no --version."""
    try:
        return "amitools " + importlib.metadata.version("amitools")
    except importlib.metadata.PackageNotFoundError:
        return "amitools unknown"


def toolchain() -> dict[str, str]:
    """Collect the tool identities that must match across a comparison."""
    return {
        "host_cc": tool_version(["cc", "--version"]),
        "m68k_gcc": tool_version(["m68k-amigaos-gcc", "--version"]),
        "vamos": vamos_version(),
        "python": platform.python_version(),
    }


def compiler_identity(clc: Path, compiler_git: str | None) -> dict[str, str]:
    """Return the compiler binary hash, commit, and version."""
    if not clc.is_file():
        raise HarnessError(f"compiler not found: {clc}")
    digest = hashlib.sha256(clc.read_bytes()).hexdigest()
    git = compiler_git or first_line(
        run_command(["git", "-C", str(ROOT), "rev-parse", "HEAD"], timeout=30)
    )
    version = first_line(run_command([str(clc), "--version"], timeout=30))
    dirty = bool(
        first_line(
            run_command(["git", "-C", str(ROOT), "status", "--porcelain"], timeout=30)
        )
    )
    return {
        "path": str(clc),
        "sha256": digest,
        "git": git,
        "version": version,
        "dirty": dirty,
    }


def harness_git() -> str:
    """Return the commit of the running harness, when available."""
    try:
        return first_line(
            run_command(["git", "-C", str(ROOT), "rev-parse", "HEAD"], timeout=30)
        )
    except HarnessError:
        return "unknown"


def normalize_output(raw: bytes) -> str:
    """Normalize stdout for a cross-profile comparison."""
    text = raw.decode("utf-8", errors="replace")
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    lines = text.split("\n")
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def expected_output(name: str) -> str:
    """Read a benchmark's expected stdout."""
    path = BENCHMARKS / name / "expected-output.txt"
    if not path.is_file():
        raise HarnessError(f"missing expected output: {path}")
    return normalize_output(path.read_bytes())


def benchmark_names() -> list[str]:
    """Return the scored benchmark names in stable order."""
    names = []
    for entry in sorted(BENCHMARKS.iterdir()):
        if entry.is_dir() and entry.name != CALIBRATION:
            if (entry / "program.clo").is_file():
                names.append(entry.name)
    if not names:
        raise HarnessError(f"no benchmarks under {BENCHMARKS}")
    return names


def validate_label(label: str) -> None:
    """Reject a label that is not a safe path component."""
    if not LABEL_RE.fullmatch(label) or label in {".", ".."}:
        raise HarnessError(f"unsafe label: {label!r}")


def confined(path_value: Path, base: Path) -> Path:
    """Resolve a path and require it to stay inside ``base``."""
    resolved = path_value.resolve()
    try:
        resolved.relative_to(base.resolve())
    except ValueError as error:
        raise HarnessError(f"path escapes {base}: {path_value}") from error
    return resolved


def parse_size_output(output: str) -> dict[str, int]:
    """Parse a size table row into text/data/bss bytes."""
    for line in output.splitlines():
        match = SIZE_ROW_RE.match(line)
        if match:
            return {
                "text": int(match.group(1)),
                "data": int(match.group(2)),
                "bss": int(match.group(3)),
            }
    raise HarnessError(f"cannot parse size output: {output.strip()}")


def size_sections(binary: Path) -> dict[str, int]:
    """Run ``m68k-amigaos-size`` and parse its section row."""
    return parse_size_output(
        run_command(["m68k-amigaos-size", str(binary)], timeout=60)
    )


def build_step(
    argv: list[str], timeout: int, *, cwd: Path | None = None
) -> None:
    """Run a build command; the helper exists for the unit-test seam."""
    run_command(argv, timeout=timeout, cwd=cwd)


def emit(clc: Path, profile: str, source: Path, output: Path) -> None:
    """Emit one C translation unit."""
    run_command(
        [
            str(clc),
            "--profile",
            profile,
            "-o",
            str(output),
            str(source),
        ],
        timeout=EMIT_TIMEOUT,
    )


def run_host(binary: Path, timeout: int = HOST_RUN_TIMEOUT) -> str:
    """Run a host binary and return its normalized stdout."""
    try:
        result = subprocess.run(
            [str(binary)],
            cwd=ROOT,
            capture_output=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        raise HarnessError(f"host run timed out after {timeout}s: {binary}") from error
    if result.returncode != 0:
        raise HarnessError(
            f"host program failed ({result.returncode}): {binary}\n"
            f"{normalize_output(result.stderr)}"
        )
    return normalize_output(result.stdout)


def run_vamos(binary: Path, timeout: int = VAMOS_TIMEOUT) -> str:
    """Run an m68k binary under vamos and return its normalized stdout."""
    try:
        result = subprocess.run(
            ["vamos", str(binary)],
            cwd=ROOT,
            capture_output=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        raise HarnessError(f"vamos run timed out after {timeout}s: {binary}") from error
    if result.returncode != 0:
        raise HarnessError(
            f"vamos program failed ({result.returncode}): {binary}\n"
            f"{normalize_output(result.stderr)}"
        )
    return normalize_output(result.stdout)


def trace_vamos(binary: Path) -> tuple[str, int]:
    """Run unter vamos with an instruction trace and count the instructions.

    The trace goes to stderr. A reader thread consumes it line by line so the
    harness never stores the multi-hundred-megabyte log.
    """
    process = subprocess.Popen(
        ["vamos", "--instr-trace", str(binary)],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        errors="replace",
    )
    counted: list[int] = [0]
    chunks: list[str] = []

    def consume_trace() -> None:
        assert process.stderr is not None
        for line in process.stderr:
            if INSTRUCTION_RE.search(line):
                counted[0] += 1

    def consume_stdout() -> None:
        assert process.stdout is not None
        for line in process.stdout:
            chunks.append(line)

    import threading

    trace_reader = threading.Thread(target=consume_trace, daemon=True)
    stdout_reader = threading.Thread(target=consume_stdout, daemon=True)
    trace_reader.start()
    stdout_reader.start()
    try:
        process.wait(timeout=TRACE_TIMEOUT)
    except subprocess.TimeoutExpired as error:
        process.kill()
        trace_reader.join(timeout=5)
        stdout_reader.join(timeout=5)
        raise HarnessError(
            f"trace timed out after {TRACE_TIMEOUT}s: {binary}"
        ) from error
    trace_reader.join(timeout=10)
    stdout_reader.join(timeout=10)
    if process.returncode != 0:
        raise HarnessError(
            f"trace program failed ({process.returncode}): {binary}"
        )
    return normalize_output("".join(chunks).encode()), counted[0]


def time_host(binary: Path, runs: int) -> tuple[int, int]:
    """Return the best and median host wall time in nanoseconds."""
    samples: list[int] = []
    for _ in range(runs):
        start = time.perf_counter_ns()
        run_host(binary)
        samples.append(time.perf_counter_ns() - start)
    samples.sort()
    return samples[0], samples[len(samples) // 2]


def measure(
    clc: Path,
    name: str,
    run_dir: Path,
    runs: int,
    trace: bool,
) -> dict[str, object]:
    """Emit, build, and measure one program."""
    source = BENCHMARKS / name / "program.clo"
    if not source.is_file():
        raise HarnessError(f"missing benchmark source: {source}")
    run_dir.mkdir(parents=True, exist_ok=True)
    host_c = run_dir / "host.c"
    amiga_c = run_dir / "amiga.c"
    emit(clc, "host", source, host_c)
    emit(clc, "hosted-amiga", source, amiga_c)

    host_debug_bin = run_dir / "host-debug"
    host_release_bin = run_dir / "host-release"
    amiga_debug_bin = run_dir / "amiga-debug"
    amiga_release_bin = run_dir / "amiga-release"
    build_step(["cc", *HOST_DEBUG, "-o", str(host_debug_bin), str(host_c)], HOST_BUILD_TIMEOUT)
    build_step(["cc", *HOST_RELEASE, "-o", str(host_release_bin), str(host_c)], HOST_BUILD_TIMEOUT)
    build_step(
        ["m68k-amigaos-gcc", *M68K_DEBUG, "-o", str(amiga_debug_bin), str(amiga_c), "-lamiga", "-lm"],
        M68K_BUILD_TIMEOUT,
    )
    build_step(
        ["m68k-amigaos-gcc", *M68K_RELEASE, "-o", str(amiga_release_bin), str(amiga_c), "-lamiga", "-lm"],
        M68K_BUILD_TIMEOUT,
    )

    expected = expected_output(name)
    host_debug_output = run_host(host_debug_bin)
    if host_debug_output != expected:
        raise HarnessError(f"{name}: host debug output differs")
    host_release_output = run_host(host_release_bin)
    if host_release_output != expected:
        raise HarnessError(f"{name}: host release output differs")
    amiga_start = time.perf_counter_ns()
    amiga_output = run_vamos(amiga_release_bin)
    amiga_duration_ns = time.perf_counter_ns() - amiga_start
    if amiga_output != expected:
        raise HarnessError(f"{name}: m68k output differs")

    debug_start = time.perf_counter_ns()
    run_host(host_debug_bin)
    host_debug_ns = time.perf_counter_ns() - debug_start
    host_best_ns, host_median_ns = time_host(host_release_bin, runs)

    instructions: int | None = None
    trace_duration_ns: int | None = None
    if trace:
        trace_start = time.perf_counter_ns()
        traced_output, instructions = trace_vamos(amiga_release_bin)
        trace_duration_ns = time.perf_counter_ns() - trace_start
        if traced_output != expected:
            raise HarnessError(f"{name}: trace run output differs")

    return {
        "c_bytes": amiga_c.stat().st_size,
        "c_lines": amiga_c.read_text(encoding="utf-8", errors="replace").count("\n"),
        "host_c_bytes": host_c.stat().st_size,
        "host_c_lines": host_c.read_text(encoding="utf-8", errors="replace").count("\n"),
        "host_debug_bytes": host_debug_bin.stat().st_size,
        "host_release_bytes": host_release_bin.stat().st_size,
        "host_debug_ns": host_debug_ns,
        "host_release_ns": host_best_ns,
        "host_release_median_ns": host_median_ns,
        "m68k_debug": size_sections(amiga_debug_bin),
        "m68k_release": size_sections(amiga_release_bin),
        "instructions_raw": instructions,
        "duration_ms": amiga_duration_ns // 1_000_000,
        "trace_duration_ms": (
            trace_duration_ns // 1_000_000 if trace_duration_ns is not None else None
        ),
        "output_sha256": hashlib.sha256(expected.encode()).hexdigest(),
    }


def write_json(document: dict[str, object], output: Path) -> None:
    """Write a stable, sorted JSON document."""
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def command_run(args: argparse.Namespace) -> int:
    """Run the benchmark suite and write one result document."""
    validate_label(args.label)
    output = confined(path(args.out), BUILD)
    clc = path(args.clc)
    runs = args.runs
    if runs < 1:
        raise HarnessError("--runs must be positive")
    names = benchmark_names()
    if args.bench:
        unknown = sorted(set(args.bench) - set(names))
        if unknown:
            raise HarnessError(f"unknown benchmark(s): {', '.join(unknown)}")
        names = [name for name in names if name in set(args.bench)]
    BUILD.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=BUILD, prefix=args.label + "-") as temporary:
        run_root = Path(temporary)
        calibration = measure(
            clc, CALIBRATION, run_root / CALIBRATION, runs, trace=not args.no_trace
        )
        calibration_instructions = calibration["instructions_raw"]
        if calibration_instructions is None and not args.no_trace:
            raise HarnessError("calibration produced no instruction count")
        results: dict[str, object] = {}
        for name in names:
            print(f"bench: {name}", flush=True)
            measured = measure(
                clc, name, run_root / name, runs, trace=not args.no_trace
            )
            if calibration_instructions is not None and measured["instructions_raw"] is not None:
                net = int(measured["instructions_raw"]) - int(calibration_instructions)
                if net < 0:
                    raise HarnessError(f"{name}: net instruction count is negative")
                measured["instructions_net"] = net
            else:
                measured["instructions_net"] = None
            measured["host_release_net_ns"] = max(
                0,
                int(measured["host_release_ns"]) - int(calibration["host_release_ns"]),
            )
            results[name] = measured
    document = {
        "schema": SCHEMA,
        "label": args.label,
        "created": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "compiler": compiler_identity(clc, args.compiler_git),
        "harness_git": harness_git(),
        "toolchain": toolchain(),
        "runs": runs,
        "profiles": {"host": "host", "m68k": "hosted-amiga"},
        "flags": {
            "host_debug": " ".join(HOST_DEBUG),
            "host_release": " ".join(HOST_RELEASE),
            "m68k_debug": " ".join(M68K_DEBUG),
            "m68k_release": " ".join(M68K_RELEASE),
        },
        "calibration": {
            "instructions": calibration_instructions,
            "host_release_ns": calibration["host_release_ns"],
        },
        "benchmarks": results,
    }
    write_json(document, output)
    print(f"bench: wrote {output}")
    return 0


def load_document(file: Path) -> dict[str, object]:
    """Load and minimally validate a result document."""
    if not file.is_file():
        raise HarnessError(f"missing result document: {file}")
    try:
        document = json.loads(file.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise HarnessError(f"malformed result document: {file}: {error}") from error
    required = {"schema", "label", "compiler", "toolchain", "runs", "flags", "benchmarks"}
    missing = sorted(required - set(document))
    if missing:
        raise HarnessError(f"{file}: missing fields: {', '.join(missing)}")
    if document["schema"] != SCHEMA:
        raise HarnessError(f"{file}: unsupported schema {document['schema']}")
    return document


def require_same(base: dict[str, object], head: dict[str, object], field: str) -> None:
    """Require one structural field to match across a comparison."""
    if base.get(field) != head.get(field):
        raise HarnessError(
            f"cannot compare: {field} differs ({base.get(field)!r} vs {head.get(field)!r})"
        )


def metric(document: dict[str, object], name: str, field: str) -> int | None:
    """Read one benchmark metric, or None when absent."""
    benchmarks = document["benchmarks"]
    assert isinstance(benchmarks, dict)
    entry = benchmarks.get(name)
    if not isinstance(entry, dict):
        return None
    value = entry.get(field)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return int(value)


def section(document: dict[str, object], name: str, section_name: str, field: str) -> int | None:
    """Read one section metric."""
    benchmarks = document["benchmarks"]
    assert isinstance(benchmarks, dict)
    entry = benchmarks.get(name)
    if not isinstance(entry, dict):
        return None
    nested = entry.get(section_name)
    if not isinstance(nested, dict):
        return None
    value = nested.get(field)
    return int(value) if isinstance(value, int) else None


def format_delta(base: int | None, head: int | None) -> str:
    """Render a signed percentage delta."""
    if base in (None, 0) or head is None:
        return "n/a"
    return f"{(head - base) * 100.0 / base:+.1f}%"


def command_compare(args: argparse.Namespace) -> int:
    """Compare a base and head result document under the plan thresholds."""
    base = load_document(path(args.base))
    head = load_document(path(args.head))
    for field in ("toolchain", "runs", "flags", "profiles"):
        require_same(base, head, field)
    base_benchmarks = base["benchmarks"]
    head_benchmarks = head["benchmarks"]
    assert isinstance(base_benchmarks, dict) and isinstance(head_benchmarks, dict)
    if set(base_benchmarks) != set(head_benchmarks):
        raise HarnessError("cannot compare: benchmark sets differ")

    failures: list[str] = []
    rows = []
    for name in sorted(base_benchmarks):
        base_out = base_benchmarks[name].get("output_sha256")
        head_out = head_benchmarks[name].get("output_sha256")
        if base_out != head_out:
            failures.append(f"{name}: output differs")
        base_i = metric(base, name, "instructions_net")
        head_i = metric(head, name, "instructions_net")
        if base_i is None or head_i is None:
            failures.append(f"{name}: missing net instruction count")
        elif head_i > base_i * 1.01:
            failures.append(f"{name}: instructions {head_i} > {base_i} * 1.01")
        base_t = section(base, name, "m68k_release", "text")
        head_t = section(head, name, "m68k_release", "text")
        if base_t is None or head_t is None:
            failures.append(f"{name}: missing m68k .text")
        elif head_t > base_t * 1.01:
            failures.append(f"{name}: .text {head_t} > {base_t} * 1.01")
        rows.append(
            (
                name,
                base_i,
                head_i,
                base_t,
                head_t,
                base_benchmarks[name].get("host_release_median_ns"),
                head_benchmarks[name].get("host_release_median_ns"),
            )
        )
        if name == "sieve":
            if base_i is not None and head_i is not None and head_i > base_i * 0.80:
                failures.append(f"sieve: instructions target missed ({head_i} > {base_i} * 0.80)")
            if base_t is not None and head_t is not None and head_t > base_t * 0.90:
                failures.append(f"sieve: .text target missed ({head_t} > {base_t} * 0.90)")
            base_c = metric(base, name, "c_bytes")
            head_c = metric(head, name, "c_bytes")
            if base_c is not None and head_c is not None and head_c > base_c * 0.75:
                failures.append(f"sieve: C-byte target missed ({head_c} > {base_c} * 0.75)")

    improved = any(
        (b_i is not None and h_i is not None and h_i <= b_i * 0.80)
        or (b_t is not None and h_t is not None and h_t <= b_t * 0.90)
        for _, b_i, h_i, b_t, h_t, _, _ in rows
    )
    if not improved:
        failures.append("no benchmark improved by 20% instructions or 10% .text")

    print(f"{'benchmark':<14}{'base ins':>12}{'head ins':>12}{'delta':>9}"
          f"{'base .text':>12}{'head .text':>12}{'delta':>9}")
    for name, b_i, h_i, b_t, h_t, _, _ in rows:
        print(
            f"{name:<14}{b_i if b_i is not None else 'n/a':>12}"
            f"{h_i if h_i is not None else 'n/a':>12}{format_delta(b_i, h_i):>9}"
            f"{b_t if b_t is not None else 'n/a':>12}"
            f"{h_t if h_t is not None else 'n/a':>12}{format_delta(b_t, h_t):>9}"
        )
    print("\n            host base ms  host head ms   C base   C head   C delta")
    for name, _, _, _, _, base_host, head_host in rows:
        base_c = metric(base, name, "c_bytes")
        head_c = metric(head, name, "c_bytes")
        print(
            f"{name:<14}{(base_host or 0) / 1e6:>12.2f}{(head_host or 0) / 1e6:>13.2f}"
            f"{base_c if base_c is not None else 'n/a':>9}"
            f"{head_c if head_c is not None else 'n/a':>9}"
            f"{format_delta(base_c, head_c):>10}"
        )
    if failures:
        print("\ncompare: FAILED")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print("\ncompare: passed")
    return 0


def command_test(_args: argparse.Namespace) -> int:
    """Run the harness unit tests."""
    tests = ROOT / "tools" / "perf" / "tests"
    top = ROOT / "tools" / "perf"
    suite = unittest.defaultTestLoader.discover(
        str(tests), top_level_dir=str(top)
    )
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse the harness command line."""
    parser = argparse.ArgumentParser(prog="bench.py")
    sub = parser.add_subparsers(dest="command", required=True)

    run = sub.add_parser("run", help="run the benchmark suite")
    run.add_argument("--clc", default=str(ROOT / "target" / "debug" / "clc"))
    run.add_argument("--compiler-git", default=None)
    run.add_argument("--label", default="head")
    run.add_argument("--out", default=None)
    run.add_argument("--runs", type=int, default=DEFAULT_RUNS)
    run.add_argument("--no-trace", action="store_true")
    run.add_argument("--bench", action="append", default=[])
    run.set_defaults(func=command_run)

    compare = sub.add_parser("compare", help="compare two result documents")
    compare.add_argument("base")
    compare.add_argument("head")
    compare.set_defaults(func=command_compare)

    test = sub.add_parser("test", help="run the harness unit tests")
    test.set_defaults(func=command_test)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    """Run the harness and translate a harness error into exit 2."""
    args = parse_args(argv)
    if args.command == "run" and args.out is None:
        validate_label(args.label)
        args.out = str(BUILD / f"{args.label}.json")
    try:
        return args.func(args)
    except HarnessError as error:
        print(f"bench: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
