"""Real GCC specs-path controls; use the existing isolated archive acceptance inputs."""

from __future__ import annotations

import os
from pathlib import Path
import resource
import shlex
import shutil
import unittest

from support_artifact_fixtures import ArtifactFixture


class SupportGccRelocationTests(unittest.TestCase):
    """Require literal filename arguments at the compiler's internal specs boundary."""

    @classmethod
    def setUpClass(cls) -> None:
        """Audit real candidates before invoking their compiler, never a synthetic substitute."""
        value = os.environ.get("CLORRAINE_SUPPORT_ACCEPTANCE")
        if not value:
            raise AssertionError("CLORRAINE_SUPPORT_ACCEPTANCE must name real acceptance.json inputs")
        cls.candidate = ArtifactFixture(Path(value))
        cls.addClassCleanup(cls.candidate.scratch.cleanup)
        cls.candidate.stage()
        old_limit = resource.getrlimit(resource.RLIMIT_CORE)
        resource.setrlimit(resource.RLIMIT_CORE, (0, old_limit[1]))
        cls.addClassCleanup(resource.setrlimit, resource.RLIMIT_CORE, old_limit)

    def _sdk(self, suffix: str) -> Path:
        """Copy only the audited SDK into a real directory whose name exercises path handling."""
        root = self.candidate.root / (self._testMethodName + suffix)
        shutil.copytree(self.candidate.payloads["amiga"], root, symlinks=True)
        return root

    def _trace(self, sdk: Path, section: str, fragment: str, *, check: bool = True):
        """Use documented specs extension syntax with an actual compiler dry-run."""
        work = self.candidate.root / (self._testMethodName + "-commands")
        work.mkdir(exist_ok=True)
        source = work / "input.c"
        source.write_text("int main(void){return 0;}\n", encoding="utf-8")
        specs = work / "options.specs"
        specs.write_text(
            f"%rename {section} saved_{section}\n*{section}:\n"
            f"%(saved_{section}) {fragment}\n\n", encoding="utf-8")
        arguments = [sdk / "bin/m68k-amigaos-gcc", "-specs=" + str(specs), "-###",
                     source, "-o", work / "program"]
        if section == "cpp":
            arguments.insert(-3, "-c")
        return self.candidate.run(arguments, check=check)

    def _command(self, trace: bytes, program: str) -> list[str]:
        """Read the compiler's quoted command trace without evaluating a shell command."""
        for line in trace.decode("utf-8").splitlines():
            try:
                arguments = shlex.split(line)
            except ValueError:
                continue
            if arguments and Path(arguments[0]).name == program:
                return arguments
        self.fail(f"missing actual {program} command in compiler trace")

    def test_should_find_runtime_startup_files_under_plain_and_spaced_prefixes(self) -> None:
        """The separate -B value generated for both supported runtimes stays one filename."""
        for suffix in ("-plain", " relocated with spaces"):
            sdk = self._sdk(suffix)
            for runtime in ("nix13", "nix20"):
                with self.subTest(prefix=suffix, runtime=runtime):
                    result = self.candidate.run([
                        sdk / "bin/m68k-amigaos-gcc", "-mcrt=" + runtime,
                        "-print-file-name=ncrt0.o"])
                    actual = Path(result.stdout.decode("utf-8").strip()).resolve()
                    self.assertEqual(actual, (sdk / "m68k-amigaos/libnix/lib/ncrt0.o").resolve())
                    self.assertTrue(actual.is_file())

    def test_should_pass_a_found_include_directory_as_one_argument(self) -> None:
        """A separated -isystem path is literal even when the SDK directory contains spaces."""
        sdk = self._sdk(" include root with spaces")
        result = self._trace(sdk, "cpp", "-isystem %:find-file(../libnix/include)")
        arguments = self._command(result.stderr, "cc1")
        offset = arguments.index("-isystem")
        self.assertEqual(Path(arguments[offset + 1]).resolve(),
                         (sdk / "m68k-amigaos/libnix/include").resolve())

    def test_should_preserve_a_missing_filename_fallback_with_spaces(self) -> None:
        """A missing-file result remains the original literal filename, not extra options."""
        sdk = self._sdk("-plain")
        result = self._trace(sdk, "cpp", r"-isystem %:find-file(missing-specs\ filename)")
        arguments = self._command(result.stderr, "cc1")
        offset = arguments.index("-isystem")
        self.assertEqual(arguments[offset + 1], "missing-specs filename")

    def test_should_preserve_attached_link_paths_and_literal_specs_characters(self) -> None:
        """Active bytes in a find-file result remain literal, not recursively evaluated specs."""
        sdk = self._sdk(" link root with spaces")
        missing = r"missing specs % pipe| back\slash"
        escaped = "".join("\\" + char if char in " %|\\" else char for char in missing)
        result = self._trace(sdk, "link",
                             "-L%:find-file(../libnix/lib/) -L%:find-file(" + escaped + ")")
        arguments = self._command(result.stderr, "collect2")
        directories = [Path(argument[2:]).resolve()
                       for argument in arguments if argument.startswith("-L") and len(argument) > 2]
        self.assertIn((sdk / "m68k-amigaos/libnix/lib").resolve(), directories)
        self.assertEqual(arguments.count("-L" + missing), 1)

    def test_should_preserve_rejection_of_empty_and_multiple_function_arguments(self) -> None:
        """Quoting results does not broaden the existing one-argument specs function contract."""
        sdk = self._sdk("-plain")
        for expression in ('%:find-file()', '%:find-file(%"")', '%:find-file(one two)'):
            with self.subTest(expression=expression):
                result = self._trace(sdk, "cpp", "-isystem " + expression, check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"find_file_spec_function", result.stderr)
