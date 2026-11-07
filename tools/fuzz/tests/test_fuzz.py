"""Unit tests for the deterministic differential fuzzer.

The tests use synthetic inputs and a temporary fake compiler. The template
acceptance test invokes the real `clc`; `make fuzz-test` builds the workspace
first so the binary is present.
"""

from __future__ import annotations

import contextlib
import io
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import fuzz  # noqa: E402


class GenerationTests(unittest.TestCase):
    def test_generation_is_deterministic_across_processes(self) -> None:
        script = Path(fuzz.__file__)
        outputs = []
        for hash_seed in ("0", "1"):
            env = os.environ.copy()
            env["PYTHONHASHSEED"] = hash_seed
            result = subprocess.run(
                [
                    sys.executable,
                    str(script),
                    "generate",
                    "--seed-start",
                    "0",
                    "--count",
                    "4",
                ],
                capture_output=True,
                text=True,
                env=env,
                check=True,
            )
            outputs.append(result.stdout)
        self.assertEqual(outputs[0], outputs[1])

    def test_a_different_seed_changes_the_program(self) -> None:
        def body(source: str) -> str:
            lines = source.splitlines()
            return "\n".join(lines[1:])

        self.assertNotEqual(body(fuzz.generate_program(0)), body(fuzz.generate_program(1)))

    def test_decide_matches_sha256(self) -> None:
        import hashlib

        for seed, index, label, modulus in [
            (0, 0, "family", 6),
            (17, 3, "fill-value", 200),
            (255, 9, "float-n", 6),
        ]:
            digest = hashlib.sha256(f"{seed}:{index}:{label}".encode("ascii")).digest()
            expected = int.from_bytes(digest[:8], "big") % modulus
            self.assertEqual(fuzz.decide(seed, index, label, modulus), expected)

    def test_all_families_appear_over_a_range(self) -> None:
        families = {
            fuzz.decide(seed, 0, "family", len(fuzz.GENERATORS))
            for seed in range(256)
        }
        self.assertEqual(families, set(range(len(fuzz.GENERATORS))))

    def test_generated_extents_are_bounded(self) -> None:
        import re

        for seed in range(256):
            source = fuzz.generate_program(seed)
            for extent in re.findall(r"u8\[(\d+)\]", source):
                self.assertGreaterEqual(int(extent), 1, f"seed {seed}")
                self.assertLessEqual(int(extent), 8, f"seed {seed}")
            for base in re.findall(r"usize\(f32\((\d+)\)\) \+ 1", source):
                self.assertLessEqual(int(base) + 1, 8, f"seed {seed}")

    def test_all_three_fill_shapes_are_reachable(self) -> None:
        import re

        shapes = set()
        for seed in range(256):
            source = fuzz._fill(seed)
            match = re.search(r"let a: u8\[\d+\] = \[; (.*)\];", source)
            if match:
                shapes.add(match.group(1))
        self.assertGreaterEqual(len(shapes), 3, f"fill shapes: {sorted(shapes)}")

    def test_generated_matrix_counts(self) -> None:
        smoke = list(fuzz.generated_matrix("smoke", 0, 16))
        self.assertEqual(sum(1 for _, sanitizer, _ in smoke if sanitizer), 4)
        self.assertEqual(sum(1 for _, _, target in smoke if target), 4)
        host = list(fuzz.generated_matrix("smoke-host", 0, 16))
        self.assertEqual(sum(1 for _, _, target in host if target), 0)
        full = list(fuzz.generated_matrix("full", 0, 64))
        self.assertEqual(sum(1 for _, sanitizer, _ in full if sanitizer), 64)
        self.assertEqual(sum(1 for _, _, target in full if target), 8)

    def test_generate_mode_is_deterministic_in_one_process(self) -> None:
        namespace = type(
            "Args", (), {"seed_start": 7, "count": 3}
        )()
        import contextlib
        import io

        first = io.StringIO()
        second = io.StringIO()
        with contextlib.redirect_stdout(first):
            fuzz.generate_mode(namespace)
        with contextlib.redirect_stdout(second):
            fuzz.generate_mode(namespace)
        self.assertEqual(first.getvalue(), second.getvalue())

    def test_every_template_family_is_accepted(self) -> None:
        clc = fuzz.ROOT / "target" / "debug" / "clc"
        self.assertTrue(
            clc.exists(),
            "build the workspace before running the template acceptance test",
        )
        self.assertEqual(len(fuzz.GENERATORS), 6)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, generator in enumerate(fuzz.GENERATORS):
                for sample in (0, 17, 99):
                    with self.subTest(family=index, sample=sample):
                        source = root / f"main-{index}-{sample}.clo"
                        source.write_text(generator(sample), encoding="utf-8")
                        c_path = root / f"main-{index}-{sample}.c"
                        result = subprocess.run(
                            [
                                str(clc),
                                "--profile",
                                "host",
                                "-o",
                                str(c_path),
                                str(source),
                            ],
                            capture_output=True,
                            text=True,
                        )
                        self.assertEqual(
                            result.returncode,
                            0,
                            f"family {index} sample {sample} was rejected:\n{result.stderr}",
                        )


class SeedTests(unittest.TestCase):
    def test_seed_set_and_goldens_are_complete(self) -> None:
        seeds = fuzz.load_seeds()
        self.assertEqual(len(seeds), 12)
        for seed in seeds:
            self.assertTrue(
                (seed.expected_stdout is None) != (seed.expected_fault is None),
                f"{seed.name}: exactly one golden is required",
            )

    def test_seed_order_is_sorted(self) -> None:
        names = [seed.name for seed in fuzz.load_seeds()]
        self.assertEqual(names, sorted(names))

    def test_seeds_are_safe(self) -> None:
        import re

        forbidden = [
            "embed(",
            "unsafe",
            "assert(",
            "openFile",
            "createFile",
            "writeBytes",
            "readFile",
            "openInto",
        ]
        for seed in fuzz.load_seeds():
            for token in forbidden:
                self.assertNotIn(token, seed.source, f"{seed.name}: forbidden {token!r}")
            self.assertIsNone(
                re.search(r"\bffi\b", seed.source), f"{seed.name}: forbidden ffi"
            )
            self.assertIsNone(
                re.search(r"\braw\b", seed.source), f"{seed.name}: forbidden raw"
            )


class TextTests(unittest.TestCase):
    def test_normalization_strips_crlf(self) -> None:
        self.assertEqual(fuzz.normalize("a\r\nb\r\n"), "a\nb")
        self.assertEqual(fuzz.normalize("a\rb"), "a\nb")

    def test_fault_line_normalizes_crlf(self) -> None:
        self.assertEqual(
            fuzz.fault_line("noise\r\nerror E11211: index out of range\r\n"),
            "error E11211: index out of range",
        )


class ClassifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.runner = fuzz.Runner(Path("/bin/true"), Path(tempfile.mkdtemp()), keep=False)

    def test_a_fault_seed_passes_on_the_matching_line(self) -> None:
        self.runner._check_oracle(
            "host",
            None,
            "error E11211: index out of range",
            -6,
            "",
            "error E11211: index out of range\n",
        )

    def test_a_fault_seed_fails_on_a_mismatch(self) -> None:
        with self.assertRaises(fuzz.CaseFailure) as raised:
            self.runner._check_oracle(
                "host",
                None,
                "error E11211: index out of range",
                0,
                "",
                "",
            )
        self.assertEqual(raised.exception.kind, fuzz.GOLDEN_MISMATCH)

    def test_a_host_fault_on_stdout_does_not_pass(self) -> None:
        with self.assertRaises(fuzz.CaseFailure) as raised:
            self.runner._check_oracle(
                "host",
                None,
                "error E11211: index out of range",
                -6,
                "error E11211: index out of range\n",
                "",
            )
        self.assertEqual(raised.exception.kind, fuzz.GOLDEN_MISMATCH)

    def test_a_leak_is_a_sanitizer_failure(self) -> None:
        fake = self._fake_compiler(
            "cat > \"$out\" <<'C'\n"
            "#include <stdlib.h>\n"
            "int main(void) { void *p = malloc(16); (void)p; return 0; }\n"
            "C"
        )
        with tempfile.TemporaryDirectory() as directory:
            workdir = Path(directory)
            case_dir = workdir / "work" / "leak"
            case_dir.mkdir(parents=True)
            (case_dir / "program.clo").write_text("module x;\n", encoding="utf-8")
            runner = fuzz.Runner(fake, workdir, keep=True)
            with contextlib.redirect_stdout(io.StringIO()):
                runner.run_case(
                    "leak",
                    "module x;\n",
                    expected_stdout="",
                    expected_fault=None,
                    sanitizer=True,
                    target=False,
                )
            self.assertEqual(runner.failures, 1)
            self.assertTrue((workdir / "failures" / "sanitizer-leak").exists())

    def test_a_stdout_seed_fails_on_a_crash(self) -> None:
        with self.assertRaises(fuzz.CaseFailure) as raised:
            self.runner._check_oracle("host", "v: 1\n", None, 1, "", "boom")
        self.assertEqual(raised.exception.kind, fuzz.CRASH)

    def test_a_stdout_seed_fails_on_a_value_change(self) -> None:
        with self.assertRaises(fuzz.CaseFailure) as raised:
            self.runner._check_oracle("host", "v: 1\n", None, 0, "v: 2\n", "")
        self.assertEqual(raised.exception.kind, fuzz.GOLDEN_MISMATCH)

    def test_a_generated_program_fails_on_a_crash(self) -> None:
        with self.assertRaises(fuzz.CaseFailure) as raised:
            self.runner._check_oracle("host", None, None, 1, "", "boom")
        self.assertEqual(raised.exception.kind, fuzz.CRASH)

    def test_a_compiler_internal_error_is_internal(self) -> None:
        fake = self._fake_compiler('echo "internal compiler error: boom" >&2; exit 1')
        case_dir = Path(tempfile.mkdtemp())
        (case_dir / "program.clo").write_text("module x;\n", encoding="utf-8")
        runner = fuzz.Runner(fake, case_dir, keep=False)
        with self.assertRaises(fuzz.CaseFailure) as raised:
            runner._emit_and_build("host", case_dir, case_dir / "out.c")
        self.assertEqual(raised.exception.kind, fuzz.INTERNAL)

    def test_a_compiler_diagnostic_is_rejected(self) -> None:
        fake = self._fake_compiler('echo "error E20000: expected a declaration" >&2; exit 1')
        case_dir = Path(tempfile.mkdtemp())
        (case_dir / "program.clo").write_text("module x;\n", encoding="utf-8")
        runner = fuzz.Runner(fake, case_dir, keep=False)
        with self.assertRaises(fuzz.CaseFailure) as raised:
            runner._emit_and_build("host", case_dir, case_dir / "out.c")
        self.assertEqual(raised.exception.kind, fuzz.COMPILER_REJECTED)

    def test_an_accepted_source_with_bad_c_is_invalid_c(self) -> None:
        fake = self._fake_compiler('printf "this is not C\\n" > "$out"; exit 0')
        case_dir = Path(tempfile.mkdtemp())
        (case_dir / "program.clo").write_text("module x;\n", encoding="utf-8")
        runner = fuzz.Runner(fake, case_dir, keep=False)
        with self.assertRaises(fuzz.CaseFailure) as raised:
            runner._emit_and_build("host", case_dir, case_dir / "out.c")
        self.assertEqual(raised.exception.kind, fuzz.INVALID_C)

    def _fake_compiler(self, body: str) -> Path:
        directory = Path(tempfile.mkdtemp())
        path = directory / "clc"
        path.write_text(
            "#!/bin/sh\n"
            "out=\n"
            "while [ $# -gt 0 ]; do\n"
            "  case \"$1\" in\n"
            "    -o) shift; out=\"$1\" ;;\n"
            "  esac\n"
            "  shift\n"
            "done\n"
            + body
            + "\n",
            encoding="utf-8",
        )
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        return path


class SafetyTests(unittest.TestCase):
    def test_commands_are_argument_lists(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = fuzz.run_command(["/bin/true"], Path(directory), 5)
            self.assertEqual(result.returncode, 0)

    def test_source_uses_no_shell(self) -> None:
        source = Path(fuzz.__file__).read_text(encoding="utf-8")
        self.assertNotIn("shell=True", source)
        self.assertNotIn("os.system", source)
        self.assertNotIn("subprocess.call(", source)

    def test_private_working_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = fuzz.confined_workdir(str(Path(directory) / "work"))
            self.assertTrue(path.exists())
        with self.assertRaises(fuzz.HarnessError):
            fuzz.confined_workdir("/etc")

    def test_core_dumps_are_disabled(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = fuzz.run_command(
                ["/bin/sh", "-c", "ulimit -c"], Path(directory), 5
            )
            self.assertEqual(result.stdout.strip(), "0")


if __name__ == "__main__":
    unittest.main()
