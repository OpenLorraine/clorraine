#!/usr/bin/env python3
"""Repeatable old-vs-shipped build-flag audit for the Clorraine compiler.

The scored harness measures compiler emission; this audit measures the
shipped release build. It emits each program once with ``clc``, builds the
old, GC-only, and shipped variants with explicit argument arrays, runs the
result, and records section sizes, file sizes, instruction counts, and output.
The ``--check`` mode asserts the size and runtime floors.

Only the Python standard library is used. Every generated artifact stays
under ``build/perf/`` and every subprocess runs with an argument list and a
timeout.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import bench

# The scored benchmark set; calibration is measured but not scored.
SCORED = ("sieve", "matrix", "strings", "moves", "lookup")
CALIBRATION = "calibration"

# The four portable 1.3 corpus programs. interrupt-counter is FS-UAE-only.
CORPUS = ("buffer-container", "file-copier", "string-parser", "volatile-mmio")

M68K_PROFILES = ("hosted-amiga", "hosted-amiga-13")
GC_FLAGS = ["-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"]
FREESTANDING_BASE = [
    "-std=c99",
    "-ffreestanding",
    "-fno-builtin",
    "-nostdlib",
    "-m68000",
    "-fwrapv",
]
REQUIRED_CONSTRUCTORS = (
    "___initstdio",
    "___initcpp",
    "___initlibraries",
    "___initmalloc",
)
TIMING_KEYS = {
    "duration_ms",
    "trace_duration_ms",
    "host_debug_ns",
    "host_release_ns",
    "host_release_median_ns",
    "host_release_net_ns",
}


class AuditError(RuntimeError):
    """A failed audit step with the exact context."""


def _old_from_shipped(new: list[str]) -> list[str]:
    """Strip the GC and strip suffix from a shipped release vector."""
    if len(new) < len(GC_FLAGS) + 2 or new[-1] != "-s":
        raise AuditError(f"unexpected shipped flags: {new}")
    if new[-1 - len(GC_FLAGS) : -1] != GC_FLAGS:
        raise AuditError(f"unexpected shipped flags: {new}")
    return new[: -1 - len(GC_FLAGS)]


def profile_spec(profile: str) -> dict[str, object]:
    """Return the compiler, flag arrays, and libraries for one profile."""
    if profile == "host":
        new = list(bench.HOST_RELEASE)
        return {
            "compiler": "cc",
            "old": _old_from_shipped(new),
            "gc": new[:-1],
            "new": new,
            "libs": [],
        }
    if profile == "hosted-amiga":
        new = list(bench.M68K_RELEASE)
        return {
            "compiler": "m68k-amigaos-gcc",
            "old": _old_from_shipped(new),
            "gc": new[:-1],
            "new": new,
            "libs": ["-lamiga", "-lm"],
        }
    if profile == "hosted-amiga-13":
        new = [
            flag.replace("-mcrt=nix20", "-mcrt=nix13")
            for flag in bench.M68K_RELEASE
        ]
        if new == list(bench.M68K_RELEASE):
            raise AuditError("the 1.3 release vector did not switch to nix13")
        return {
            "compiler": "m68k-amigaos-gcc",
            "old": _old_from_shipped(new),
            "gc": new[:-1],
            "new": new,
            "libs": ["-lamiga", "-lm"],
        }
    if profile == "freestanding":
        old = FREESTANDING_BASE + ["-O2"]
        return {
            "compiler": "m68k-amigaos-gcc",
            "old": old,
            "gc": old + GC_FLAGS,
            "new": old,
            "libs": ["-lg"],
        }
    raise AuditError(f"unknown profile: {profile}")


def profile_programs(profile: str) -> list[tuple[str, str]]:
    """Return (name, kind) pairs for one profile."""
    if profile in ("host", "hosted-amiga"):
        return [(CALIBRATION, "calibration")] + [(name, "scored") for name in SCORED]
    if profile == "hosted-amiga-13":
        return (
            [(CALIBRATION, "calibration"), ("sieve", "scored")]
            + [(name, "corpus") for name in CORPUS]
        )
    if profile == "freestanding":
        return [("freestanding", "freestanding")]
    raise AuditError(f"unknown profile: {profile}")


def program_source(name: str, kind: str) -> Path:
    """Return the committed source for a measured program."""
    if kind in ("calibration", "scored"):
        return bench.BENCHMARKS / name / "program.clo"
    if kind == "corpus":
        return bench.ROOT / "spec" / "tests" / "corpus" / name / "program.clo"
    if kind == "freestanding":
        return bench.ROOT / "spec" / "tests" / "corpus" / "freestanding" / "program.clo"
    raise AuditError(f"unknown program kind: {kind}")


def expected_output(name: str, kind: str) -> str:
    """Return the expected stdout for a measured program."""
    if kind in ("calibration", "scored"):
        return bench.expected_output(name)
    if kind == "corpus":
        path = bench.ROOT / "spec" / "tests" / "corpus" / name / "expected-output.txt"
    elif kind == "freestanding":
        path = (
            bench.ROOT
            / "spec"
            / "tests"
            / "corpus"
            / "freestanding"
            / "expected-output.txt"
        )
    else:
        raise AuditError(f"unknown program kind: {kind}")
    if not path.is_file():
        raise AuditError(f"missing expected output: {path}")
    return bench.normalize_output(path.read_bytes())


def run_in(binary: Path, profile: str, cwd: Path) -> str:
    """Run a built program in ``cwd`` and return normalized stdout."""
    argv = [str(binary)] if profile == "host" else ["vamos", str(binary)]
    try:
        result = subprocess.run(
            argv,
            cwd=cwd,
            capture_output=True,
            timeout=bench.VAMOS_TIMEOUT if profile != "host" else bench.HOST_RUN_TIMEOUT,
        )
    except FileNotFoundError as error:
        raise AuditError(f"missing runner: {argv[0]}") from error
    except subprocess.TimeoutExpired as error:
        raise AuditError(f"run timed out: {binary}") from error
    if result.returncode != 0:
        raise AuditError(
            f"program failed ({result.returncode}): {binary}\n"
            f"{bench.normalize_output(result.stderr)}"
        )
    return bench.normalize_output(result.stdout)


def trace_pcs(binary: Path) -> list[int]:
    """Run an instruction trace and return the executed program counters."""
    try:
        result = subprocess.run(
            ["vamos", "--instr-trace", str(binary)],
            cwd=bench.ROOT,
            capture_output=True,
            text=True,
            timeout=bench.TRACE_TIMEOUT,
        )
    except FileNotFoundError as error:
        raise AuditError(f"missing vamos: {binary}") from error
    except subprocess.TimeoutExpired as error:
        raise AuditError(f"trace timed out: {binary}") from error
    if result.returncode != 0:
        raise AuditError(f"trace failed ({result.returncode}): {binary}")
    pcs = [
        int(match.group(1), 16)
        for line in result.stderr.splitlines()
        if (match := re.search(r"\s([0-9a-f]{6})\s+", line))
    ]
    if not pcs:
        raise AuditError(f"trace produced no instruction lines: {binary}")
    return pcs


def measure_variant(
    spec: dict[str, object],
    c_path: Path,
    binary: Path,
    flags: list[str],
    run_dir: Path,
    expected: str,
    trace: bool,
    run: bool,
) -> dict[str, object]:
    """Build and run one variant to one fixed output path."""
    shutil.rmtree(run_dir, ignore_errors=True)
    run_dir.mkdir(parents=True, exist_ok=True)
    argv = [str(spec["compiler"]), *flags, "-o", str(binary), str(c_path)]
    argv.extend(spec["libs"])
    bench.build_step(argv, bench.M68K_BUILD_TIMEOUT)
    if spec["compiler"] == "m68k-amigaos-gcc":
        sections = bench.size_sections(binary)
        instructions_raw: int | None = None
        if trace:
            try:
                traced_output, instructions_raw = bench.trace_vamos(binary)
            except (FileNotFoundError, bench.HarnessError) as error:
                raise AuditError(f"cannot trace {binary}: {error}") from error
            if traced_output != expected:
                raise AuditError(f"trace output differs: {binary}")
    else:
        try:
            sections = bench.parse_size_output(
                bench.run_command(["size", str(binary)], timeout=60)
            )
        except bench.HarnessError as error:
            raise AuditError(f"cannot size {binary}: {error}") from error
        instructions_raw = None
    output = run_in(binary, str(spec["profile"]), run_dir) if run else None
    if run and output != expected:
        raise AuditError(f"output differs from expected: {binary}")
    return {
        "text": sections["text"],
        "data": sections["data"],
        "bss": sections["bss"],
        "bin": bench.binary_bytes(binary),
        "instructions_raw": instructions_raw,
        "instructions_net": None,
        "output": output,
    }


def host_symbols(binary: Path) -> bool:
    """Return true when a host release binary still has a .symtab section."""
    try:
        output = bench.run_command(["readelf", "-S", str(binary)], timeout=60)
    except bench.HarnessError as error:
        raise AuditError(f"cannot inspect host symbols: {binary}: {error}") from error
    return ".symtab" in output


def run_matrix(clc: Path, work_root: Path) -> dict[str, object]:
    """Build, run, and record the complete audit matrix once."""
    programs: dict[str, object] = {}
    calibration: dict[str, dict[str, int | None]] = {}
    constructors: dict[str, object] = {}
    for profile in ("host", "hosted-amiga", "hosted-amiga-13", "freestanding"):
        spec = profile_spec(profile)
        spec["profile"] = profile
        for name, kind in profile_programs(profile):
            key = f"{profile}/{name}"
            work = work_root / profile / name
            work.mkdir(parents=True, exist_ok=True)
            c_path = work / f"{name}.c"
            binary = work / name
            bench.emit(clc, profile, program_source(name, kind), c_path)
            c_bytes = c_path.stat().st_size
            expected = expected_output(name, kind)
            trace = profile in M68K_PROFILES and kind in ("calibration", "scored")
            run = profile != "freestanding"
            variants = {}
            for variant in ("old", "gc", "new"):
                variants[variant] = measure_variant(
                    spec,
                    c_path,
                    binary,
                    list(spec[variant]),
                    work / "run",
                    expected,
                    trace,
                    run,
                )
                if kind == "calibration" and variant == "gc":
                    shutil.copy2(binary, work / "calibration-gc")
            entry = {
                "profile": profile,
                "program": name,
                "kind": kind,
                "old_flags": list(spec["old"]),
                "gc_flags": list(spec["gc"]),
                "new_flags": list(spec["new"]),
                "libs": list(spec["libs"]),
                "c_bytes": c_bytes,
                "old": variants["old"],
                "gc": variants["gc"],
                "new": variants["new"],
                "host_symtab": host_symbols(binary)
                if profile == "host" and kind == "scored"
                else False,
            }
            if kind == "calibration":
                calibration[profile] = {
                    "old": variants["old"]["instructions_raw"],
                    "gc": variants["gc"]["instructions_raw"],
                    "new": variants["new"]["instructions_raw"],
                }
            if kind == "scored" and profile in M68K_PROFILES:
                base = calibration[profile]
                for variant in ("old", "gc", "new"):
                    raw = variants[variant]["instructions_raw"]
                    cal = base[variant]
                    if raw is None or cal is None:
                        raise AuditError(f"{key}: missing instruction count")
                    net = int(raw) - int(cal)
                    if net < 0:
                        raise AuditError(f"{key}: negative net instruction count")
                    variants[variant]["instructions_net"] = net
            programs[key] = entry
        if profile in M68K_PROFILES:
            gc_binary = work_root / profile / CALIBRATION / "calibration-gc"
            constructors[profile] = constructor_proof(profile, gc_binary)
    return {
        "schema": 1,
        "clc": bench.compiler_identity(clc, None),
        "toolchain": bench.toolchain(),
        "programs": programs,
        "calibration": calibration,
        "constructors": constructors,
    }


def constructor_proof(profile: str, binary: Path) -> dict[str, object]:
    """Prove the remaining init-list constructors still execute."""
    nm = bench.run_command(["m68k-amigaos-nm", "-n", str(binary)], timeout=60)
    symbols: list[tuple[int, str]] = []
    for line in nm.splitlines():
        match = re.match(r"^([0-9a-f]{8}) T (\S+)$", line.strip())
        if match:
            symbols.append((int(match.group(1), 16), match.group(2)))
    if not symbols:
        raise AuditError(f"no text symbols in {binary}")
    addresses = sorted({address for address, _ in symbols})
    starts = [address for address, name in symbols if name == "_start"]
    exits = [address for address, name in symbols if name == "__Exit"]
    if not starts or not exits:
        raise AuditError(f"missing _start or __Exit in {binary}")
    pcs = trace_pcs(binary)
    base = pcs[0] - starts[0]

    def hits(address: int) -> int:
        following = next((a for a in addresses if a > address), address + 0x1000)
        return sum(1 for pc in pcs if base + address <= pc < base + following)

    constructors = {
        name: hits(address)
        for address, name in symbols
        if name.startswith("___init")
    }
    if not constructors:
        raise AuditError(f"no remaining init constructors in {binary}")
    proof = {
        "profile": profile,
        "base": base,
        "init_list_walk": hits(exits[0]),
        "constructors": constructors,
    }
    if proof["init_list_walk"] == 0 or any(count == 0 for count in constructors.values()):
        raise AuditError(f"an init-list constructor did not run: {proof}")
    return proof


def check_constructor(matrix: dict) -> list[str]:
    """Return the constructor-proof failures; an empty list means pass."""
    proofs = matrix.get("constructors")
    if proofs is None:
        return ["constructors: missing proof"]
    if not isinstance(proofs, dict):
        raise AuditError("matrix has malformed constructors")
    failures: list[str] = []
    for profile in M68K_PROFILES:
        proof = proofs.get(profile)
        if not isinstance(proof, dict):
            failures.append(f"{profile}: missing constructor proof")
            continue
        if int(proof.get("init_list_walk", 0)) <= 0:
            failures.append(f"{profile}: the init-list walk did not run")
        constructors = proof.get("constructors", {})
        if not isinstance(constructors, dict):
            failures.append(f"{profile}: malformed constructor list")
            continue
        for name in REQUIRED_CONSTRUCTORS:
            if name not in constructors:
                failures.append(f"{profile}: missing {name}")
            elif int(constructors[name]) <= 0:
                failures.append(f"{profile}: {name} did not run")
    return failures


def check_floors(matrix: dict) -> list[str]:
    """Return the floor failures in ``matrix``; an empty list means pass."""
    failures: list[str] = []
    programs = matrix.get("programs")
    if not isinstance(programs, dict):
        raise AuditError("matrix has no programs")
    for key, entry in sorted(programs.items()):
        if not isinstance(entry, dict) or not all(
            field in entry for field in ("profile", "kind", "old", "new")
        ):
            raise AuditError(f"{key}: malformed entry")
        profile = entry["profile"]
        kind = entry["kind"]
        old = entry["old"]
        new = entry["new"]
        if kind == "scored" and profile in M68K_PROFILES:
            if int(new["text"]) * 100 > int(old["text"]) * 80:
                failures.append(f"{key}: m68k text {new['text']} > {old['text']} * 80%")
            if int(new["bin"]) * 100 > int(old["bin"]) * 80:
                failures.append(f"{key}: m68k binary {new['bin']} > {old['bin']} * 80%")
            if int(new["instructions_net"]) * 100 > int(old["instructions_net"]) * 101:
                failures.append(
                    f"{key}: net instructions {new['instructions_net']} > "
                    f"{old['instructions_net']} * 101%"
                )
        if kind == "scored" and profile == "host":
            if int(new["bin"]) * 100 > int(old["bin"]) * 85:
                failures.append(f"{key}: host binary {new['bin']} > {old['bin']} * 85%")
            if entry.get("host_symtab"):
                failures.append(f"{key}: host release binary still has .symtab")
        if kind == "corpus" and profile == "hosted-amiga-13":
            if int(new["text"]) * 100 > int(old["text"]) * 60:
                failures.append(f"{key}: 1.3 text {new['text']} > {old['text']} * 60%")
        if kind == "calibration":
            if (
                old["instructions_raw"] is not None
                and new["instructions_raw"] is not None
                and int(new["instructions_raw"]) * 100 > int(old["instructions_raw"]) * 90
            ):
                failures.append(
                    f"{key}: calibration {new['instructions_raw']} > "
                    f"{old['instructions_raw']} * 90%"
                )
        if kind == "freestanding":
            if new["text"] != old["text"] or new["bin"] != old["bin"]:
                failures.append(
                    f"{key}: freestanding image changed "
                    f"({old['text']}/{old['bin']} -> {new['text']}/{new['bin']})"
                )
        if new["output"] != old["output"]:
            failures.append(f"{key}: output differs")
    return failures


def _diff(first: object, second: object, path: str, out: list[str]) -> None:
    """Collect non-timing differences between two JSON values."""
    if isinstance(first, dict) and isinstance(second, dict):
        for key in sorted(set(first) | set(second)):
            if key in TIMING_KEYS:
                continue
            _diff(first.get(key), second.get(key), f"{path}.{key}", out)
        return
    if first != second:
        out.append(f"{path}: {first!r} != {second!r}")


def compare_repeats(first: dict, second: dict) -> list[str]:
    """Return the non-timing differences between two audit matrices."""
    differences: list[str] = []
    _diff(first, second, "matrix", differences)
    return differences


def print_table(matrix: dict) -> None:
    """Print one compact row per measured program."""
    print(f"{'program':<38}{'old .text':>12}{'new .text':>12}{'delta':>9}"
          f"{'old bin':>12}{'new bin':>12}{'delta':>9}")
    for key, entry in sorted(matrix["programs"].items()):
        old = entry["old"]
        new = entry["new"]
        text_delta = bench.format_delta(old["text"], new["text"])
        bin_delta = bench.format_delta(old["bin"], new["bin"])
        print(f"{key:<38}{old['text']!s:>12}{new['text']!s:>12}{text_delta:>9}"
              f"{old['bin']!s:>12}{new['bin']!s:>12}{bin_delta:>9}")


def main(argv: list[str] | None = None) -> int:
    """Parse the audit command line and run the gate."""
    parser = argparse.ArgumentParser(prog="flag_audit.py")
    parser.add_argument("command", choices=("run",))
    parser.add_argument("--clc", default="target/debug/clc")
    parser.add_argument("--out", default="build/perf/flags.json")
    parser.add_argument("--repeat", type=int, default=2)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    if args.repeat < 1:
        print("flag_audit: --repeat must be positive", file=sys.stderr)
        return 2
    try:
        clc = bench.path(args.clc)
        out = bench.confined(bench.path(args.out), bench.BUILD)
        work_root = bench.BUILD / "flags"
        first = run_matrix(clc, work_root)
        differences: list[str] = []
        for _ in range(args.repeat - 1):
            following = run_matrix(clc, work_root)
            differences.extend(compare_repeats(first, following))
        failures = check_floors(first)
        constructor_failures = check_constructor(first)
        first["runs"] = args.repeat
        first["checks"] = [
            {
                "name": "floors",
                "passed": not failures,
                "failures": failures,
            }
        ]
        first["checks"].append(
            {
                "name": "constructors",
                "passed": not constructor_failures,
                "failures": constructor_failures,
            }
        )
        if args.repeat > 1:
            first["checks"].append(
                {
                    "name": "repeat",
                    "passed": not differences,
                    "failures": differences,
                }
            )
        bench.write_json(first, out)
        print_table(first)
        if differences:
            print("\nrepeat: FAILED")
            for difference in differences:
                print(f"  - {difference}")
            return 1
        if failures or constructor_failures:
            print("\nfloors: FAILED")
            for failure in failures + constructor_failures:
                print(f"  - {failure}")
            return 1 if args.check else 0
        print(f"\nfloors and constructors: passed; wrote {out}")
        return 0
    except (AuditError, bench.HarnessError) as error:
        print(f"flag_audit: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
