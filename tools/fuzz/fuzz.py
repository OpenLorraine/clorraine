#!/usr/bin/env python3
"""Deterministic grammar-based differential fuzzer for Clorraine.

The harness generates accepted programs from a small typed template grammar,
compiles and runs them on the host and, for a fixed subset, on
`hosted-amiga` under `vamos`, and builds the host programs under
AddressSanitizer and UndefinedBehaviorSanitizer. A committed seed set carries
golden outputs (or a fault line) so a shared wrong value is visible even when
host and target agree.

Design rules:

* Every choice comes from a SHA-256 digest of `(seed, index, label)`. No
  `random`, hash order, filesystem order, environment value, or clock reaches
  the generated source.
* All scratch output stays under `build/fuzz/` or the system temporary
  directory. Successfully run cases are removed; a failure retains its source,
  emitted C, and logs under `build/fuzz/failures/`.
* Every child has a timeout. Timeouts only bound a hang; pass/fail never
  depends on wall-clock performance.
* Tools are invoked through argument lists, never a shell.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import resource
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths and constants
# ---------------------------------------------------------------------------

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SEEDS_DIR = HERE / "seeds"
FUZZ_BUILD = ROOT / "build" / "fuzz"
DEFAULT_CLC = ROOT / "target" / "debug" / "clc"

CC = "cc"
AMIGA_CC = "m68k-amigaos-gcc"
VAMOS = "vamos"

HOST_FLAGS = ["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]
AMIGA_FLAGS = [
    "-std=c99",
    "-mcrt=nix20",
    "-m68000",
    "-fwrapv",
    "-O0",
    "-g",
    "-Wall",
    "-Wextra",
    "-Werror",
]
SANITIZER_FLAGS = [
    "-fsanitize=address,undefined",
    "-fno-sanitize-recover=all",
    "-no-pie",
]

COMPILER_TIMEOUT = 60
BUILD_TIMEOUT = 120
HOST_RUN_TIMEOUT = 30
SANITIZER_RUN_TIMEOUT = 60
AMIGA_RUN_TIMEOUT = 120

FAULT_PATTERN = re.compile(r"^error [A-Z0-9]+:")

# Failure classes.
INVALID_C = "invalid-c"
DIVERGENCE = "divergence"
SANITIZER = "sanitizer"
CRASH = "crash"
HANG = "hang"
COMPILER_REJECTED = "compiler-rejected"
INTERNAL = "internal"
GOLDEN_MISMATCH = "golden-mismatch"
HARNESS = "harness"


class HarnessError(Exception):
    """The harness itself failed; this is not a compiler finding."""


class CaseFailure(Exception):
    """One generated or seed case failed its oracle."""

    def __init__(self, kind: str, detail: str):
        super().__init__(detail)
        self.kind = kind
        self.detail = detail


# ---------------------------------------------------------------------------
# Deterministic decisions and text helpers
# ---------------------------------------------------------------------------


def decide(seed: int, index: int, label: str, modulus: int) -> int:
    """A deterministic integer in `[0, modulus)` for one choice.

    The digest is stable across Python versions and machines; it is the only
    entropy source in the generator.
    """
    if modulus <= 0:
        raise HarnessError(f"invalid decision modulus {modulus}")
    digest = hashlib.sha256(f"{seed}:{index}:{label}".encode("ascii")).digest()
    return int.from_bytes(digest[:8], "big") % modulus


def normalize(text: str) -> str:
    """Normalizes CRLF and a trailing newline for an output comparison."""
    return text.replace("\r\n", "\n").replace("\r", "\n").rstrip("\n")


def fault_line(text: str) -> str | None:
    """The first compiler fault line in `text`, normalized, or `None`."""
    for line in normalize(text).splitlines():
        if FAULT_PATTERN.match(line):
            return line
    return None


def _core_limit() -> None:
    """Disables core files in a child process."""
    try:
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    except (ValueError, OSError):
        pass


def non_negative(value: str) -> int:
    """An argparse type that rejects a negative seed or count."""
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be non-negative")
    return parsed


# ---------------------------------------------------------------------------
# Generator
# ---------------------------------------------------------------------------


def _narrow(seed: int) -> str:
    a = 1 + decide(seed, 1, "narrow-a", 250)
    b = 1 + decide(seed, 2, "narrow-b", 250)
    shift = 1 + decide(seed, 3, "narrow-shift", 7)
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

const A: i32 = i32(u8({a}) + u8({b}));
const B: i32 = i32(u16({a}) * u16({b}));

func main(): i32 {{
    println($"v: {{A}} {{B}} {{i32(u8({a}) * u8({b}))}} {{i32(u8({a}) << {shift})}}");
    return 0;
}}
"""


def _fill(seed: int) -> str:
    extent = 4 + decide(seed, 1, "fill-extent", 4)
    index = decide(seed, 2, "fill-index", 8)
    value = 1 + decide(seed, 3, "fill-value", 200)
    kind = decide(seed, 4, "fill-kind", 3)
    if kind == 0:
        fill = f"u8({value})"
    elif kind == 1:
        fill = f"u8({value}) + u8({value})"
    else:
        fill = f"u8({value}) << 1"
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

func main(): i32 {{
    let src: u8[8] = [1, 2, 3, 4, 5, 6, 7];
    let a: u8[{extent}] = [; {fill}];
    let b: u8[{extent}] = [; src[{index}]];
    println($"v: {{i32(a[0])}} {{i32(a[{extent - 1}])}} {{i32(b[0])}} {{i32(b[{extent - 1}])}}");
    return 0;
}}
"""


def _enum(seed: int) -> str:
    a = decide(seed, 1, "enum-a", 100)
    c = 130 + decide(seed, 2, "enum-c", 100)
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

enum E: u8 {{ A = {a}, B, C = {c} }}

func main(): i32 {{
    let x: E = E.B;
    println($"v: {{i32(E.A)}} {{i32(E.B)}} {{i32(E.C)}} {{i32(x)}}");
    return 0;
}}
"""


def _float(seed: int) -> str:
    n = 2 + decide(seed, 1, "float-n", 6)
    half = 1 + decide(seed, 2, "float-half", 9)
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

const A: f32 = f32({n}.{half});
const N: usize = usize(f32({n})) + 1;

func main(): i32 {{
    let a: u8[N] = [; 0];
    println($"v: {{sizeof(u8[N])}} {{i32(N)}} {{i32(A)}} {{i32(a[0])}}");
    return 0;
}}
"""


def _shift(seed: int) -> str:
    value = 1 + decide(seed, 1, "shift-value", 250)
    divisor = 1 + decide(seed, 2, "shift-divisor", 250)
    count = 1 + decide(seed, 3, "shift-count", 7)
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

func main(): i32 {{
    let a: u8 = u8({value}) / (u8({divisor}) >> 8);
    let b: u8 = u8({value}) << {count};
    let c: u8 = u8({value}) >> {count};
    println($"v: {{i32(a)}} {{i32(b)}} {{i32(c)}}");
    return 0;
}}
"""


def _struct_loop(seed: int) -> str:
    a = 1 + decide(seed, 1, "struct-a", 200)
    x = decide(seed, 2, "struct-x", 20)
    y = decide(seed, 3, "struct-y", 20)
    z = decide(seed, 4, "struct-z", 20)
    return f"""module fuzz.S{seed};
import {{ println }} from std.io;

struct S {{ a: u8; b: i32[3]; }}

func main(): i32 {{
    let s: S = S {{ a: {a}, b: [{x}, {y}, {z}] }};
    let i: i32 = 0;
    let sum: i32 = 0;
    while (i < 6) {{
        sum = sum + s.b[i % 3];
        i = i + 1;
    }}
    println($"v: {{i32(s.a)}} {{sum}} {{i}}");
    return 0;
}}
"""


GENERATORS = [_narrow, _fill, _enum, _float, _shift, _struct_loop]


def generate_program(seed: int) -> str:
    """Generates one accepted program for `seed`."""
    family = decide(seed, 0, "family", len(GENERATORS))
    return GENERATORS[family](seed)


# ---------------------------------------------------------------------------
# Seeds
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Seed:
    """One committed seed and its oracle."""

    name: str
    source: str
    expected_stdout: str | None
    expected_fault: str | None


def load_seeds(directory: Path = SEEDS_DIR) -> list[Seed]:
    """Loads the committed seeds in filename order."""
    seeds: list[Seed] = []
    for path in sorted(directory.glob("*.clo")):
        name = path.stem
        stdout_path = directory / f"{name}.expected.txt"
        fault_path = directory / f"{name}.expected-fault.txt"
        if stdout_path.exists() == fault_path.exists():
            raise HarnessError(
                f"seed {name} must have exactly one of .expected.txt and .expected-fault.txt"
            )
        source = path.read_text(encoding="utf-8")
        if stdout_path.exists():
            seeds.append(
                Seed(
                    name=name,
                    source=source,
                    expected_stdout=normalize(stdout_path.read_text(encoding="utf-8")),
                    expected_fault=None,
                )
            )
        else:
            seeds.append(
                Seed(
                    name=name,
                    source=source,
                    expected_stdout=None,
                    expected_fault=normalize(fault_path.read_text(encoding="utf-8")).strip(),
                )
            )
    if not seeds:
        raise HarnessError(f"no seeds found in {directory}")
    return seeds


# ---------------------------------------------------------------------------
# Command helpers
# ---------------------------------------------------------------------------


def confined_workdir(value: str | None) -> Path:
    """Resolves and creates the scratch directory under an allowed root."""
    candidate = Path(value).resolve() if value else FUZZ_BUILD.resolve()
    allowed = [FUZZ_BUILD.resolve(), Path(tempfile.gettempdir()).resolve()]
    if not any(candidate == root or root in candidate.parents for root in allowed):
        raise HarnessError(
            f"work directory must stay under {FUZZ_BUILD} or {tempfile.gettempdir()}: {candidate}"
        )
    candidate.mkdir(parents=True, exist_ok=True)
    return candidate


def run_command(
    command: list[str],
    cwd: Path,
    timeout: int,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess:
    """Runs one child with a timeout and no core file."""
    child_env = os.environ.copy()
    if env:
        child_env.update(env)
    try:
        return subprocess.run(
            command,
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=timeout,
            env=child_env,
            preexec_fn=_core_limit,
        )
    except subprocess.TimeoutExpired as error:
        raise CaseFailure(HANG, f"timeout after {error.timeout}s: {' '.join(command)}") from error
    except FileNotFoundError as error:
        raise HarnessError(f"missing tool: {command[0]}") from error


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


class Runner:
    """Runs one case over the host, sanitizer, and target legs."""

    def __init__(self, clc: Path, workdir: Path, keep: bool):
        self.clc = clc
        self.workdir = workdir
        self.keep = keep
        self.cases = 0
        self.failures = 0
        self.host_legs = 0
        self.sanitizer_legs = 0
        self.target_legs = 0

    def run_case(
        self,
        name: str,
        source: str,
        expected_stdout: str | None,
        expected_fault: str | None,
        sanitizer: bool,
        target: bool,
    ) -> None:
        """Runs one case and records a failure instead of stopping the run."""
        self.cases += 1
        case_dir = self.workdir / "work" / name
        if case_dir.exists():
            shutil.rmtree(case_dir)
        case_dir.mkdir(parents=True)
        (case_dir / "program.clo").write_text(source, encoding="utf-8")
        try:
            self._run_case(case_dir, expected_stdout, expected_fault, sanitizer, target)
        except CaseFailure as failure:
            self._record(name, case_dir, failure)
        else:
            if not self.keep:
                shutil.rmtree(case_dir)

    def _run_case(
        self,
        case_dir: Path,
        expected_stdout: str | None,
        expected_fault: str | None,
        sanitizer: bool,
        target: bool,
    ) -> None:
        host_c = case_dir / "host.c"
        host_program = self._emit_and_build("host", case_dir, host_c, sanitize=False)
        host_code, host_out, host_err = self._run(
            host_program, case_dir, HOST_RUN_TIMEOUT, label="host-run"
        )
        self.host_legs += 1
        self._check_oracle("host", expected_stdout, expected_fault, host_code, host_out, host_err)

        if sanitizer:
            sanitizer_program = self._emit_and_build(
                "host", case_dir, host_c, sanitize=True, suffix="-san"
            )
            code, out, err = self._run(
                sanitizer_program,
                case_dir,
                SANITIZER_RUN_TIMEOUT,
                env={
                    "ASAN_OPTIONS": "detect_leaks=1:symbolize=0",
                    "UBSAN_OPTIONS": "print_stacktrace=0",
                },
                label="sanitizer-run",
            )
            self.sanitizer_legs += 1
            if expected_fault is not None:
                line = fault_line(err)
                if line != expected_fault:
                    if self._sanitizer_report(err) or self._sanitizer_report(out):
                        raise CaseFailure(SANITIZER, err or out)
                    raise CaseFailure(
                        GOLDEN_MISMATCH,
                        f"sanitizer fault {line!r} does not match {expected_fault!r}; stderr:\n{err}",
                    )
                if self._sanitizer_report(err):
                    raise CaseFailure(SANITIZER, err)
            else:
                if self._sanitizer_report(err):
                    raise CaseFailure(SANITIZER, err)
                self._check_oracle(
                    "sanitizer", expected_stdout, expected_fault, code, out, err
                )

        if target:
            amiga_c = case_dir / "amiga.c"
            amiga_program = self._emit_and_build(
                "hosted-amiga", case_dir, amiga_c, target_leg=True, suffix="-amiga"
            )
            code, out, err = self._run(
                [VAMOS, str(amiga_program)],
                case_dir,
                AMIGA_RUN_TIMEOUT,
                label="target-run",
            )
            self.target_legs += 1
            if expected_fault is not None:
                line = fault_line(out)
                if line != expected_fault:
                    raise CaseFailure(
                        GOLDEN_MISMATCH,
                        f"target fault {line!r} does not match {expected_fault!r}",
                    )
            else:
                if code != 0:
                    raise CaseFailure(CRASH, f"target exit {code}; stderr:\n{err}")
                if normalize(out) != normalize(host_out):
                    raise CaseFailure(
                        DIVERGENCE,
                        f"target stdout {normalize(out)!r} differs from host {normalize(host_out)!r}",
                    )

    @staticmethod
    def _sanitizer_report(text: str) -> bool:
        """True when a sanitizer report appears in `text`."""
        return (
            "AddressSanitizer" in text
            or "LeakSanitizer" in text
            or "runtime error:" in text
        )

    def _check_oracle(
        self,
        leg: str,
        expected_stdout: str | None,
        expected_fault: str | None,
        code: int,
        out: str,
        err: str,
    ) -> None:
        if expected_fault is not None:
            line = fault_line(err)
            if line != expected_fault:
                raise CaseFailure(
                    GOLDEN_MISMATCH,
                    f"{leg}: fault {line!r} does not match {expected_fault!r}; stderr:\n{err}",
                )
            return
        if expected_stdout is not None:
            if code != 0:
                raise CaseFailure(CRASH, f"{leg}: exit {code}; stderr:\n{err}")
            if normalize(out) != expected_stdout:
                raise CaseFailure(
                    GOLDEN_MISMATCH,
                    f"{leg}: stdout {normalize(out)!r} does not match {expected_stdout!r}",
                )
            return
        if code != 0:
            raise CaseFailure(CRASH, f"{leg}: generated program exited {code}; stderr:\n{err}")

    def _emit_and_build(
        self,
        profile: str,
        case_dir: Path,
        c_path: Path,
        sanitize: bool = False,
        target_leg: bool = False,
        suffix: str = "",
    ) -> Path:
        """Emits and builds one leg, returning the program path."""
        emit_command = [
            str(self.clc),
            "--profile",
            profile,
            "-o",
            str(c_path),
            "program.clo",
        ]
        try:
            emit = run_command(emit_command, case_dir, COMPILER_TIMEOUT)
        except CaseFailure as failure:
            (case_dir / f"emit{suffix}.log").write_text(
                f"command: {' '.join(emit_command)}\n{failure.kind}: {failure.detail}\n",
                encoding="utf-8",
            )
            raise
        (case_dir / f"emit{suffix}.log").write_text(
            f"command: {' '.join(emit_command)}\n"
            f"exit={emit.returncode}\nstdout:\n{emit.stdout}\nstderr:\n{emit.stderr}",
            encoding="utf-8",
        )
        if emit.returncode < 0:
            raise CaseFailure(CRASH, f"the compiler died on signal {-emit.returncode}")
        if emit.returncode != 0:
            if "internal compiler error" in emit.stderr or "panicked" in emit.stderr:
                raise CaseFailure(INTERNAL, emit.stderr)
            raise CaseFailure(COMPILER_REJECTED, emit.stderr)

        program = case_dir / f"program{suffix}"
        if target_leg:
            flags = AMIGA_FLAGS
            link = ["-lamiga", "-lm"]
        else:
            flags = list(HOST_FLAGS)
            if sanitize:
                flags += SANITIZER_FLAGS
            link = []
        command = [AMIGA_CC if target_leg else CC] + flags + ["-o", str(program), str(c_path)] + link
        try:
            build = run_command(command, case_dir, BUILD_TIMEOUT)
        except CaseFailure as failure:
            (case_dir / f"build{suffix}.log").write_text(
                f"command: {' '.join(command)}\n{failure.kind}: {failure.detail}\n",
                encoding="utf-8",
            )
            raise
        (case_dir / f"build{suffix}.log").write_text(
            f"command: {' '.join(command)}\n"
            f"exit={build.returncode}\nstdout:\n{build.stdout}\nstderr:\n{build.stderr}",
            encoding="utf-8",
        )
        if build.returncode != 0:
            raise CaseFailure(INVALID_C, build.stderr)
        return program

    def _run(
        self,
        command: str | list[str],
        case_dir: Path,
        timeout: int,
        env: dict[str, str] | None = None,
        label: str = "run",
    ) -> tuple[int, str, str]:
        argv = command if isinstance(command, list) else [command]
        argv = [str(part) for part in argv]
        try:
            result = run_command(argv, case_dir, timeout, env)
        except CaseFailure as failure:
            (case_dir / f"{label}.log").write_text(
                f"command: {' '.join(argv)}\n{failure.kind}: {failure.detail}\n",
                encoding="utf-8",
            )
            raise
        code = result.returncode
        (case_dir / f"{label}.log").write_text(
            f"command: {' '.join(argv)}\n"
            f"exit={code}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}",
            encoding="utf-8",
        )
        return code, result.stdout, result.stderr

    def _record(self, name: str, case_dir: Path, failure: CaseFailure) -> None:
        self.failures += 1
        destination = self.workdir / "failures" / f"{failure.kind}-{name}"
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(case_dir, destination)
        summary = {
            "case": name,
            "class": failure.kind,
            "detail": failure.detail,
            "clc": str(self.clc),
            "workdir": str(case_dir),
            "timeouts": {
                "compiler": COMPILER_TIMEOUT,
                "build": BUILD_TIMEOUT,
                "host_run": HOST_RUN_TIMEOUT,
                "sanitizer_run": SANITIZER_RUN_TIMEOUT,
                "target_run": AMIGA_RUN_TIMEOUT,
            },
        }
        (destination / "case.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(f"fuzz: FAIL {failure.kind}: {name} -> {destination}")
        print(f"fuzz:   {failure.detail.splitlines()[0] if failure.detail else ''}")


# ---------------------------------------------------------------------------
# Modes
# ---------------------------------------------------------------------------


def mode_counts(mode: str, count: int | None) -> int:
    if count is not None:
        return count
    return 256 if mode == "full" else 16


def generated_matrix(mode: str, seed_start: int, count: int):
    """Yields `(seed, sanitizer, target)` for the generated seed range."""
    for offset in range(count):
        seed = seed_start + offset
        if mode == "full":
            sanitizer = True
            target = offset % 8 == 0
        elif mode == "smoke-host":
            sanitizer = offset < 4
            target = False
        else:
            sanitizer = offset < 4
            target = offset < 4
        yield seed, sanitizer, target


def run_mode(args: argparse.Namespace) -> int:
    clc = Path(args.clc).resolve()
    if not clc.exists():
        raise HarnessError(f"compiler not found: {clc}")
    workdir = confined_workdir(args.workdir)
    runner = Runner(clc, workdir, args.keep)
    seeds = load_seeds()

    for seed in seeds:
        runner.run_case(
            name=seed.name,
            source=seed.source,
            expected_stdout=seed.expected_stdout,
            expected_fault=seed.expected_fault,
            sanitizer=True,
            target=args.mode != "smoke-host",
        )

    count = mode_counts(args.mode, args.count)
    for seed, sanitizer, target in generated_matrix(args.mode, args.seed_start, count):
        runner.run_case(
            name=f"generated-{seed}",
            source=generate_program(seed),
            expected_stdout=None,
            expected_fault=None,
            sanitizer=sanitizer,
            target=target,
        )

    print(
        "fuzz: "
        f"cases={runner.cases} host={runner.host_legs} "
        f"sanitizer={runner.sanitizer_legs} target={runner.target_legs} "
        f"failures={runner.failures}"
    )
    return 1 if runner.failures else 0


def generate_mode(args: argparse.Namespace) -> int:
    for seed in range(args.seed_start, args.seed_start + args.count):
        print(f"=== seed {seed} ===")
        print(generate_program(seed), end="")
    return 0


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    run = sub.add_parser("run", help="generate, compile, and run the matrix")
    run.add_argument("--mode", choices=["smoke", "smoke-host", "full"], default="smoke")
    run.add_argument("--seed-start", type=non_negative, default=0)
    run.add_argument("--count", type=non_negative, default=None)
    run.add_argument("--clc", default=str(DEFAULT_CLC))
    run.add_argument("--workdir", default=None)
    run.add_argument("--keep", action="store_true")
    run.set_defaults(handler=run_mode)

    gen = sub.add_parser("generate", help="print generated programs")
    gen.add_argument("--seed-start", type=non_negative, default=0)
    gen.add_argument("--count", type=non_negative, default=4)
    gen.set_defaults(handler=generate_mode)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.handler(args)
    except HarnessError as error:
        print(f"fuzz: {HARNESS} error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
