"""Independent public-producer and isolated packaged-helper specifications.

Miniature inputs establish packaging and import behavior only. Successful
installed compiler/host/vamos proof remains a separate real-context invocation.
"""

from __future__ import annotations

import io
import json
import os
import shlex
import shutil
import stat
import zipfile

from bootstrap_package_fixtures import (
    BootstrapCase, assert_zip_contract, input_snapshot, output_bytes,
)
from bootstrap_package_closure_spec import BootstrapClosureTests
from native_install_fixture import tree_state
from native_manifest_data import COMPONENTS, SUPPORT, encode


class BootstrapPackageTests(BootstrapCase):
    """Require a valid production assembly before every malformed-input assertion."""

    def _success(self, output, **overrides):
        """Require exact publication, the printed destination and untouched input bytes."""
        result = self.fixture.invoke(self, self.fixture.arguments(output, **overrides))
        self.assertEqual(result.returncode, 0, result.stderr)
        displayed = str(output) in result.stdout
        if not displayed:
            # A safe POSIX-quoted path may split an embedded apostrophe into
            # several quote segments. Judge the displayed value, not that spelling.
            try:
                displayed = any(str(output) in token for token in shlex.split(result.stdout))
            except ValueError:
                displayed = False
        self.assertTrue(displayed, "producer did not print its completed output path")
        self.assertEqual({path.name for path in output.iterdir()},
                         {"install.sh", "install.pyz", "manifest.json"})
        return result

    def _rejected(self, output, arguments, *, code: int = 1) -> None:
        """Require a named producer error and no final or abandoned sibling output."""
        before = tree_state(self.fixture.outputs)
        result = self.fixture.invoke(self, arguments)
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertTrue(result.stderr.strip(), "invalid input must name its failed step")
        self.assertFalse(output.exists() or output.is_symlink(), "failed producer published output")
        self.assertEqual(tree_state(self.fixture.outputs), before, "failed producer left private staging")

    def test_should_publish_exact_entry_assets_and_safe_fixed_zip_metadata(self) -> None:
        """Three normalized files include all required modules while permitting runtime data."""
        output = self.fixture.outputs / "complete"
        self._success(output)
        for name, mode in (("install.sh", 0o755), ("install.pyz", 0o755), ("manifest.json", 0o644)):
            path = output / name
            self.assertFalse(path.is_symlink())
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), mode)
        manifest = json.loads((output / "manifest.json").read_bytes())
        self.assertEqual(manifest, self.fixture.source.manifest)
        self.assertEqual((output / "manifest.json").read_bytes(), encode(manifest))
        assert_zip_contract(self, output / "install.pyz")

    def test_should_reproduce_all_three_files_despite_input_metadata_and_output_path_changes(self) -> None:
        """Identical input/source bytes and identities yield identical output bytes."""
        expected = output_bytes(self.fixture.output)
        for path in self.fixture.source.paths.values():
            os.utime(path, (946684800, 946684800))
            path.chmod(0o600)
        output = self.fixture.outputs / "another output ; literal"
        self._success(output)
        self.assertEqual(output_bytes(output), expected)

    def test_should_handle_literal_input_and_output_paths_without_shell_evaluation(self) -> None:
        """Spaces, quotes and shell syntax in paths remain ordinary argument data."""
        relocated = self.root / "input paths ' ; touch INJECTED ; #"
        relocated.mkdir()
        support = relocated / "support files"
        support.mkdir()
        selected = {}
        for identity, original in self.fixture.source.paths.items():
            if identity == SUPPORT + "-ndk":
                continue  # NDK bytes are not a producer input.
            parent = relocated if identity in (COMPONENTS + "-core", COMPONENTS + "-vsix") else support
            target = parent / original.name
            shutil.copyfile(original, target)
            selected[identity] = target
        before = input_snapshot(selected)
        output = self.fixture.outputs / "output ' ; touch INJECTED ; #"
        self._success(output, core=selected[COMPONENTS + "-core"],
                      vsix=selected[COMPONENTS + "-vsix"], **{"support-inputs": support})
        self.assertEqual(input_snapshot(selected), before)
        self.assertEqual(output_bytes(output), output_bytes(self.fixture.output))
        self.assertFalse(any(self.root.rglob("INJECTED")), "producer evaluated path text as a shell")

    def test_should_ignore_extra_support_files_instead_of_packaging_or_reading_them(self) -> None:
        """Only nine named ordinary support inputs affect the distribution outputs."""
        directory = self.fixture.source.paths[SUPPORT + "-lock"].parent
        extra = directory / "unrelated unreadable metadata"
        extra.write_bytes(b"not JSON and not a declared support input\n")
        extra.chmod(0)
        output = self.fixture.outputs / "extras ignored"
        self._success(output)
        self.assertEqual(output_bytes(output), output_bytes(self.fixture.output))

    def test_should_refuse_existing_outputs_of_every_type_without_clobber(self) -> None:
        """Files, empty/nonempty directories and even dangling aliases are not fresh outputs."""
        outside = self.root / "unrelated outside"
        outside.mkdir()
        (outside / "sentinel").write_bytes(b"preserve outside bytes\n")
        for kind in ("file", "empty-directory", "directory", "symlink", "dangling-symlink"):
            with self.subTest(kind=kind):
                output = self.fixture.outputs / kind
                if kind == "file":
                    output.write_bytes(b"preserve existing output file\n")
                elif kind in ("empty-directory", "directory"):
                    output.mkdir()
                    if kind == "directory":
                        (output / "sentinel").write_bytes(b"preserve existing directory\n")
                else:
                    output.symlink_to(outside if kind == "symlink" else outside / "absent")
                before, before_outside = tree_state(self.fixture.outputs), tree_state(outside)
                result = self.fixture.invoke(self, self.fixture.arguments(output))
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertTrue("output" in result.stderr.lower() or kind in result.stderr)
                self.assertEqual(tree_state(self.fixture.outputs), before)
                self.assertEqual(tree_state(outside), before_outside)

    def test_should_report_unknown_or_missing_arguments_as_usage_failures(self) -> None:
        """A missing producer cannot accidentally satisfy exit-two negative controls."""
        for kind in ("unknown", "missing-value", "missing-required"):
            with self.subTest(kind=kind):
                output = self.fixture.outputs / ("usage-" + kind)
                arguments = self.fixture.arguments(output)
                if kind == "unknown":
                    arguments += ["--not-a-bootstrap-option"]
                else:
                    index = arguments.index("--core")
                    del arguments[index + 1]
                    if kind == "missing-required":
                        del arguments[index]
                self._rejected(output, arguments, code=2)

    def test_should_reject_terminal_controls_and_unsafe_identity_values(self) -> None:
        """Bad argument data is rejected with printable diagnostics, not interpreted."""
        for key, value in (("components", "../escape"), ("support", "Uppercase"),
                           ("release", "components-fixture-core"), ("components", "bad\x1b[31m"),
                           ("support", "bad\ridentity"), ("release", "v1.2.0\n-r1")):
            with self.subTest(key=key, value=value):
                output = self.fixture.outputs / ("argument-" + str(len(list(self.fixture.outputs.iterdir()))))
                arguments = self.fixture.arguments(output, **{key: value})
                result = self.fixture.invoke(self, arguments)
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(result.stderr.strip())
                self.assertFalse(output.exists())
                self.assertFalse(any(ord(character) < 32 and character not in "\n\t"
                                     or ord(character) == 127 for character in result.stderr))
                self.assertNotIn("\x1b", result.stderr)

    def test_should_require_core_vsix_and_each_of_the_nine_support_inputs(self) -> None:
        """Missing ordinary inputs name a failure and leave no partial distribution."""
        for identity, path in self.fixture.source.paths.items():
            if identity == SUPPORT + "-ndk":
                continue
            with self.subTest(identity=identity):
                saved = self.root / ("saved-" + identity)
                path.rename(saved)
                output = self.fixture.outputs / ("missing-" + identity)
                try:
                    self._rejected(output, self.fixture.arguments(output))
                finally:
                    saved.rename(path)

    def test_should_refuse_nonordinary_required_support_inputs_without_blocking(self) -> None:
        """A directory, alias or FIFO cannot be consumed as the exact lock file."""
        selected = self.fixture.source.paths[SUPPORT + "-lock"]
        saved = self.root / "saved lock"
        selected.rename(saved)
        try:
            for kind in ("directory", "symlink", "fifo"):
                with self.subTest(kind=kind):
                    if kind == "directory":
                        selected.mkdir()
                    elif kind == "symlink":
                        selected.symlink_to(saved)
                    else:
                        os.mkfifo(selected)
                    try:
                        output = self.fixture.outputs / ("nonordinary-" + kind)
                        self._rejected(output, self.fixture.arguments(output))
                    finally:
                        if selected.is_dir() and not selected.is_symlink():
                            selected.rmdir()
                        else:
                            selected.unlink()
        finally:
            saved.rename(selected)

    def test_should_reject_changed_archive_lock_or_audit_bytes_before_publication(self) -> None:
        """Byte-level and cross-component validation is shared with the published installer."""
        for identity in (COMPONENTS + "-core", COMPONENTS + "-vsix",
                         SUPPORT + "-lock", SUPPORT + "-amiga-audit"):
            with self.subTest(identity=identity):
                path = self.fixture.source.paths[identity]
                original = path.read_bytes()
                if identity == SUPPORT + "-lock":
                    changed = original + b"\n"  # Still valid JSON, but no longer the audited lock bytes.
                elif identity.endswith("-audit"):
                    value = json.loads(original)
                    value["lock_sha256"] = "f" * 64
                    changed = encode(value)
                elif identity == COMPONENTS + "-vsix":
                    # Remove the ZIP directory, rather than relying on a
                    # consumer noticing an unused member's local-header damage.
                    changed = original[:len(original) // 2]
                else:
                    changed = bytes([original[0] ^ 1]) + original[1:]
                path.write_bytes(changed)
                try:
                    output = self.fixture.outputs / ("changed-" + identity)
                    self._rejected(output, self.fixture.arguments(output))
                finally:
                    path.write_bytes(original)

    def test_should_bind_vsix_identity_and_version_to_actual_package_metadata(self) -> None:
        """A matching ZIP suffix cannot substitute for the current extension metadata."""
        path = self.fixture.source.paths[COMPONENTS + "-vsix"]
        original = path.read_bytes()
        for field, value in (("name", "unrelated"), ("publisher", "unrelated"), ("version", "7.8.9")):
            with self.subTest(field=field):
                with zipfile.ZipFile(io.BytesIO(original)) as source:
                    output_bytes_stream = io.BytesIO()
                    with zipfile.ZipFile(output_bytes_stream, "w") as changed:
                        for entry in source.infolist():
                            data = source.read(entry.filename)
                            if entry.filename == "extension/package.json":
                                metadata = json.loads(data)
                                metadata[field] = value
                                data = encode(metadata)
                            changed.writestr(entry, data)
                path.write_bytes(output_bytes_stream.getvalue())
                try:
                    output = self.fixture.outputs / ("invalid-vsix-" + field)
                    self._rejected(output, self.fixture.arguments(output))
                finally:
                    path.write_bytes(original)
