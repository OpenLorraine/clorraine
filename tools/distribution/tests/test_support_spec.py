"""Behavioral contracts for support inputs, failed builds and payload audits."""

from __future__ import annotations

import copy
import os
import shlex
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zlib
from pathlib import Path

from support_artifact_spec import SupportArtifactAcceptance
from support_build_spec import SupportBuildTests
from support_fixtures import SupportFixture
from support_preparation_spec import SupportPreparationTests
from support_ownership_spec import SupportOwnershipTests
from support_wheels_spec import SupportWheelTests
from support_wheel_selection_spec import SupportWheelSelectionTests


class SupportContractTests(unittest.TestCase):
    """Exercise public commands with real files/Git and only external Docker replaced."""

    def setUp(self) -> None:
        """Create an independently owned fixture and register cleanup immediately."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-spec-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.fixture = SupportFixture(Path(scratch.name))

    def _invoke(self, entry: str, *arguments: object) -> subprocess.CompletedProcess[str]:
        """Require an executable entry point before testing its observable behavior."""
        script = Path(__file__).resolve().parents[1] / entry
        self.assertTrue(script.is_file(), f"Missing public command: {script}")
        if script.suffix == ".py":
            command = [sys.executable, str(script)]
        else:
            self.assertTrue(os.access(script, os.X_OK), f"Command is not executable: {script}")
            command = [str(script)]
        return subprocess.run(
            [*command, *(str(argument) for argument in arguments)],
            cwd=self.fixture.root, env=self.fixture.environment,
            capture_output=True, text=True, timeout=30)

    def _support(self, operation: str) -> subprocess.CompletedProcess[str]:
        """Run a source/lock command against the persisted synthetic inputs."""
        self.fixture.save()
        arguments = ["--lock", self.fixture.lock_path]
        if operation == "verify-sources":
            arguments.extend(["--sources", self.fixture.sources])
        return self._invoke("support.py", operation, *arguments)

    def _build(self, image_id: str | None = None) -> subprocess.CompletedProcess[str]:
        """Run the public build command; Docker cannot contact the real daemon."""
        self.fixture.save()
        if image_id is None:
            image_id = self.fixture.image_id
        return self._invoke(
            "build-support.sh", "--lock", self.fixture.lock_path,
            "--sources", self.fixture.sources, "--image-id", image_id,
            "--output", self.fixture.build_output)

    def _package(self) -> subprocess.CompletedProcess[str]:
        """Package the actual candidate and its independently supplied audit."""
        self.fixture.save()
        return self._invoke(
            "package_support.py", "--lock", self.fixture.lock_path,
            "--payload", self.fixture.payload, "--audit", self.fixture.audit_path,
            "--output", self.fixture.archive_output)

    def _failed(self, result: subprocess.CompletedProcess[str], identity: str) -> None:
        """Require a named behavioral rejection rather than just a process failure."""
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(identity, result.stderr, result.stderr)

    def _no_build(self) -> None:
        """Reject both successful artifacts and abandoned private build staging."""
        output = self.fixture.build_output
        self.assertFalse((output / "payload").exists())
        self.assertFalse((output / "provenance.json").exists())
        if output.exists():
            self.assertEqual(list(output.iterdir()), [], "failed build left private staging")

    def _rejected_payload(self, identity: str) -> None:
        """Require audit failure without a final or temporary published archive."""
        self._failed(self._package(), identity)
        self.assertEqual(list(self.fixture.archive_output.parent.iterdir()), [])

    @staticmethod
    def _option(arguments: list[str], *names: str) -> str | None:
        """Accept equivalent Docker long/short option spellings."""
        for index, argument in enumerate(arguments):
            for name in names:
                if argument == name and index + 1 < len(arguments):
                    return arguments[index + 1]
                if argument.startswith(name + "="):
                    return argument[len(name) + 1:]
        return None

    def test_should_validate_a_complete_lock(self) -> None:
        """Complete synthetic metadata is valid without becoming a release candidate."""
        result = self._support("validate-lock")
        self.assertEqual(result.returncode, 0, result.stderr)
        self._no_build()
        self.assertFalse(self.fixture.archive_output.exists())

    def test_should_verify_exact_staged_sources(self) -> None:
        """Real clean Git, exact archive bytes and hashed inputs satisfy verification."""
        result = self._support("verify-sources")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.fixture.git("status", "--porcelain", "--ignored").strip(), "")

    def test_should_accept_unqualified_debian_package_names(self) -> None:
        """Real package names may contain plus signs without becoming source identifiers."""
        self.fixture.lock["build"]["packages"]["g++"] = "1"
        for cohort in ("22.04", "24.04"):
            self.fixture.lock["output"]["runtime_packages"][cohort]["libstdc++6"] = {
                "version": "1", "sonames": ["libstdc++.so.6"],
                "paths": ["/usr/lib/x86_64-linux-gnu/libstdc++.so.6"]}
        result = self._support("validate-lock")
        self.assertEqual(result.returncode, 0, result.stderr)
        self._no_build()

    def test_should_reject_unsafe_or_qualified_debian_package_names(self) -> None:
        """Package fields reject aliases, architecture suffixes, paths and shell syntax."""
        original = copy.deepcopy(self.fixture.lock)
        for location in ("build", "runtime"):
            for name in ("bad_name", "bad/name", "libc6:amd64", "bad;name",
                         "Badname", "a", "../bad", "bad name"):
                with self.subTest(location=location, name=name):
                    self.fixture.lock = copy.deepcopy(original)
                    if location == "build":
                        self.fixture.lock["build"]["packages"][name] = "1"
                    else:
                        packages = self.fixture.lock["output"]["runtime_packages"]["22.04"]
                        packages[name] = copy.deepcopy(packages["dash"])
                    self._failed(self._support("validate-lock"), name)
                    self._no_build()

    def test_should_keep_source_identifiers_strict_with_package_names(self) -> None:
        """The package grammar does not permit plus signs in source IDs."""
        name = "extra+source"
        self.fixture.lock["sources"][name] = copy.deepcopy(self.fixture.lock["sources"]["lha"])
        self._failed(self._support("validate-lock"), name)
        self._no_build()

    def test_should_reject_staged_tracked_source_changes(self) -> None:
        """A staged edit is not the locked source even when HEAD has not moved."""
        (self.fixture.checkout / "LICENSE").write_text("changed source notice\n", encoding="utf-8")
        self.fixture.git("add", "LICENSE")
        self._failed(self._support("verify-sources"), "lha")

    def test_should_reject_extra_ignored_source_inputs(self) -> None:
        """Git ignore rules cannot conceal an extra input from source verification."""
        name = "ignored-build-input"
        (self.fixture.checkout / ".git/info/exclude").write_text(name + "\n", encoding="utf-8")
        (self.fixture.checkout / name).write_text("unlocked input\n", encoding="utf-8")
        self._failed(self._support("verify-sources"), name)

    def test_should_reject_unlocked_git_attribute_overrides(self) -> None:
        """Git-private attributes cannot alter packaged source selection without a locked input."""
        (self.fixture.checkout / ".git/info/attributes").write_text(
            "payload/* export-ignore\n", encoding="utf-8")
        self.assertEqual(self.fixture.git("status", "--porcelain").strip(), "")
        self._failed(self._support("verify-sources"), "lha")

    def test_should_verify_internal_source_links(self) -> None:
        """A tracked relative link stays valid when its target is inside the checkout."""
        (self.fixture.checkout / "payload/bin/internal").symlink_to("lha")
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-m", "Internal source link")
        self.fixture.lock["sources"]["lha"]["commit"] = self.fixture.git("rev-parse", "HEAD").strip()
        result = self._support("verify-sources")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_should_reject_source_links_to_outside_inputs(self) -> None:
        """A tracked link cannot introduce undeclared data outside its component."""
        sentinel = self.fixture.root / "outside-source.txt"
        sentinel.write_text("outside remains unchanged\n", encoding="utf-8")
        (self.fixture.checkout / "outside-input").symlink_to(sentinel)
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-m", "Escaping source link")
        self.fixture.lock["sources"]["lha"]["commit"] = self.fixture.git("rev-parse", "HEAD").strip()
        self._failed(self._support("verify-sources"), "outside-input")
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "outside remains unchanged\n")

    def test_should_not_execute_git_filters_during_source_verification(self) -> None:
        """Executable checkout metadata is rejected before any external filter can run."""
        attributes = self.fixture.checkout / ".gitattributes"
        attributes.write_text("LICENSE filter=untrusted\n", encoding="utf-8")
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-m", "Source attributes")
        self.fixture.lock["sources"]["lha"]["commit"] = self.fixture.git("rev-parse", "HEAD").strip()
        sentinel = self.fixture.root / "unexpected-filter-execution"
        script = self.fixture.root / "filter-probe.py"
        script.write_text(
            "from pathlib import Path\nimport sys\n"
            f"Path({str(sentinel)!r}).write_text('unexpected execution')\n"
            "sys.stdout.buffer.write(sys.stdin.buffer.read())\n", encoding="utf-8")
        self.fixture.git("config", "filter.untrusted.clean", shlex.join([sys.executable, str(script)]))
        notice = self.fixture.checkout / "LICENSE"
        info = notice.stat()
        os.utime(notice, ns=(info.st_atime_ns, info.st_mtime_ns + 2_000_000_000))
        self._failed(self._support("verify-sources"), "lha")
        self.assertFalse(sentinel.exists(), "source verification executed an undeclared Git filter")

    def test_should_reject_corrupted_git_object_identity(self) -> None:
        """An object stored under the locked name must still have that actual hash."""
        original = self.fixture.lock["sources"]["lha"]["commit"]
        (self.fixture.checkout / "LICENSE").write_text("different tree\n", encoding="utf-8")
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-m", "Different source tree")
        replacement = self.fixture.git("cat-file", "commit", "HEAD").encode("utf-8")
        # Keep the index/worktree consistent with the replacement tree, but
        # forge its containing commit's filename and HEAD. A name-only identity
        # check would miss this; actual Git object integrity must be checked.
        object_path = self.fixture.checkout / ".git/objects" / original[:2] / original[2:]
        object_path.chmod(0o644)
        object_path.write_bytes(zlib.compress(
            b"commit " + str(len(replacement)).encode("ascii") + b"\0" + replacement))
        (self.fixture.checkout / ".git/HEAD").write_text(original + "\n", encoding="ascii")
        self._failed(self._support("verify-sources"), "lha")

    def _ident_source(self) -> tuple[Path, bytes]:
        """Commit raw text whose attributes would expand its identity on checkout."""
        path = self.fixture.checkout / "ident.c"
        content = b"/* $Id$ */\nint source_value(void) { return 1; }\n"
        path.write_bytes(content)
        (self.fixture.checkout / ".gitattributes").write_text("ident.c ident\n", encoding="utf-8")
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-m", "Source with built-in ident attribute")
        self.fixture.lock["sources"]["lha"]["commit"] = self.fixture.git("rev-parse", "HEAD").strip()
        return path, content

    def test_should_verify_raw_blob_bytes_despite_checkout_attributes(self) -> None:
        """Source identity is stored bytes, not Git's default checkout representation."""
        path, content = self._ident_source()
        self.assertEqual(self.fixture.git("show", "HEAD:ident.c").encode("utf-8"), content)
        result = self._support("verify-sources")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(path.read_bytes(), content, "verification rewrote source bytes")

    def test_should_reject_ident_expansion_without_rewriting_sources(self) -> None:
        """Even a Git-clean built-in conversion must be canonicalised before verification."""
        path, content = self._ident_source()
        path.unlink()
        self.fixture.git("checkout", "HEAD", "--", "ident.c")
        expanded = path.read_bytes()
        object_id = self.fixture.git("rev-parse", "HEAD:ident.c").strip().encode("ascii")
        self.assertEqual(expanded, content.replace(b"$Id$", b"$Id: " + object_id + b" $"))
        self.assertEqual(self.fixture.git("status", "--porcelain").strip(), "")
        self._failed(self._support("verify-sources"), "ident.c")
        self.assertEqual(path.read_bytes(), expanded, "rejection repaired source bytes")

    def test_should_reject_a_mutable_execution_image_before_docker(self) -> None:
        """A tag cannot select a prepared image or create build work."""
        image = "clorraine-prepared:latest"
        self._failed(self._build(image), image)
        self.assertEqual(self.fixture.docker_calls(), [])
        self.assertFalse(self.fixture.build_output.exists())

    def test_should_reject_a_lock_without_an_environment_recipe(self) -> None:
        """The base image alone does not identify the prepared build environment."""
        del self.fixture.lock["build"]["environment_recipe"]
        self._failed(self._support("validate-lock"), "environment_recipe")
        self._failed(self._build(), "environment_recipe")
        self._no_build()

    def test_should_reject_an_environment_recipe_without_a_locked_hash(self) -> None:
        """The environment Dockerfile must resolve to a hashed input."""
        recipe = self.fixture.lock["build"]["environment_recipe"]
        del self.fixture.lock["inputs"][recipe]
        self._failed(self._support("validate-lock"), recipe)
        self._failed(self._build(), recipe)
        self._no_build()

    def test_should_reject_unverified_prepared_image_metadata(self) -> None:
        """Mismatched identity, labels or base ancestry prevent the recipe and publication."""
        environment = self.fixture.environment.copy()
        base = self.fixture.lock["build"]["image"]
        recipe = self.fixture.lock["build"]["environment_recipe"]
        wrong_id = "sha256:" + "e" * 64
        for setting, value, identities in (
                ("SUPPORT_DOCKER_INSPECTED_ID", wrong_id, (self.fixture.image_id, wrong_id)),
                ("SUPPORT_DOCKER_BASE_LABEL", "docker.io/library/ubuntu@" + wrong_id,
                 (self.fixture.image_id, base, "org.clorraine.base-image")),
                ("SUPPORT_DOCKER_RECIPE_LABEL", "e" * 64,
                 (self.fixture.image_id, recipe, "org.clorraine.environment-recipe-sha256")),
                ("SUPPORT_DOCKER_WRONG_BASE_PREFIX", "1", (self.fixture.image_id, base))):
            with self.subTest(setting=setting):
                self.fixture.environment = dict(environment, **{setting: value})
                self.fixture.docker_log.unlink(missing_ok=True)
                result = self._build()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertTrue(any(identity in result.stderr for identity in identities), result.stderr)
                calls = self.fixture.docker_calls()
                self.assertTrue(any(
                    self.fixture.image_id in call
                    and (call[:2] == ["image", "inspect"] or call[:1] == ["inspect"])
                    for call in calls), "the prepared image was not inspected")
                argv = self.fixture.lock["build"]["argv"]
                self.assertFalse(any(
                    call[:1] == ["run"] and call[-len(argv):] == argv for call in calls),
                    "unverified image reached the build recipe")
                self._no_build()
        self.fixture.environment = environment

    def test_should_reject_a_branch_instead_of_a_commit(self) -> None:
        """A moving ref cannot identify a reproducible source input."""
        self.fixture.lock["sources"]["lha"]["commit"] = "refs/heads/fixture"
        self._failed(self._support("validate-lock"), "lha")
        self._failed(self._build(), "lha")
        self._no_build()

    def test_should_reject_an_abbreviated_commit(self) -> None:
        """A short object prefix is not a complete locked source identity."""
        source = self.fixture.lock["sources"]["lha"]
        source["commit"] = source["commit"][:9]
        self._failed(self._support("validate-lock"), "lha")
        self._failed(self._build(), "lha")
        self._no_build()

    def test_should_reject_a_lock_without_lha(self) -> None:
        """The mandatory archive helper cannot disappear from the source closure."""
        sources = self.fixture.lock["sources"]
        sources["other-helper"] = sources.pop("lha")
        self.fixture.lock["preparation"]["helpers"] = {
            "other-helper": {"source": "other-helper", "path": "bin/lha"}}
        self._failed(self._support("validate-lock"), "lha")
        self._failed(self._build(), "lha")
        self._no_build()

    def test_should_reject_an_archive_without_its_hash(self) -> None:
        """A version and byte length alone cannot authorize an archive input."""
        del self.fixture.lock["sources"]["seed"]["sha256"]
        self._failed(self._support("validate-lock"), "seed")
        self._failed(self._build(), "seed")
        self._no_build()

    def test_should_stop_building_when_staged_head_differs(self) -> None:
        """A different clean full commit is rejected before the locked recipe runs."""
        self.fixture.git("commit", "--allow-empty", "-m", "Different source identity")
        self._failed(self._support("verify-sources"), "lha")
        self._failed(self._build(), "lha")
        recipe = self.fixture.lock["build"]["argv"]
        for call in self.fixture.docker_calls():
            self.assertNotEqual(call[-len(recipe):], recipe, "unverified recipe was invoked")
        self._no_build()

    def test_should_publish_nothing_when_docker_reports_an_undeclared_fetch(self) -> None:
        """An external container failure must propagate without successful artifacts."""
        self.assertNotEqual(os.getuid(), 0, "build contracts require an ordinary user")
        result = self._build()
        self._failed(result, "recipe.sh")
        runs = [call for call in self.fixture.docker_calls() if call[:1] == ["run"]]
        self.assertTrue(runs, "failure injection never reached the external Docker boundary")
        recipe = self.fixture.lock["build"]["argv"]
        self.assertTrue(
            any(call[-len(recipe):] == recipe for call in runs),
            "a failed prerequisite probe must not substitute for the failed build recipe")
        for call in runs:
            self.assertEqual(self._option(call, "--network", "--net"), "none")
            self.assertEqual(self._option(call, "--pull"), "never")
            self.assertEqual(self._option(call, "--user", "-u"), f"{os.getuid()}:{os.getgid()}")
            self.assertIn(self.fixture.image_id, call)
            if call[-len(recipe):] == recipe:
                self.assertEqual(call[-len(recipe) - 1], self.fixture.image_id)
        self._no_build()

    def test_should_preserve_existing_build_outputs(self) -> None:
        """A nonempty output directory is refused without replacing prior artifacts."""
        output = self.fixture.build_output
        (output / "payload").mkdir(parents=True)
        (output / "payload/prior").write_bytes(b"previous payload\n")
        (output / "provenance.json").write_bytes(b"previous provenance\n")
        self._failed(self._build(), output.name)
        self.assertEqual((output / "payload/prior").read_bytes(), b"previous payload\n")
        self.assertEqual((output / "provenance.json").read_bytes(), b"previous provenance\n")
        self.assertEqual({path.name for path in output.iterdir()}, {"payload", "provenance.json"})

    def test_should_package_exact_audited_bytes_with_notices_and_source(self) -> None:
        """A valid archive contains the audited files, including its legal obligations."""
        result = self._package()
        self.assertEqual(result.returncode, 0, result.stderr)
        with tarfile.open(self.fixture.archive_output) as archive:
            members = archive.getmembers()
            names = [member.name for member in members]
            self.assertEqual(names, sorted(names))
            self.assertEqual(len(names), len(set(names)))
            files = {member.name: member for member in members if member.isfile()}
            self.assertEqual(set(files), set(self.fixture.audit["files"]))
            for name, member in files.items():
                with archive.extractfile(member) as content:
                    self.assertEqual(content.read(), (self.fixture.payload / name).read_bytes())
            for member in members:
                self.assertFalse(member.name.startswith("/"))
                self.assertNotIn("..", member.name.split("/"))
                self.assertTrue(member.isfile() or member.isdir())
                self.assertEqual(member.mode & 0o7000, 0)

    def test_should_reject_ndk_origin_headers_in_the_ordinary_include_tree(self) -> None:
        """Origin auditing also covers headers outside a specially named NDK directory."""
        self.fixture.add_component("seed")
        self.fixture.add_file("m68k-amigaos/include/exec.h", b"/* derived header */\n", "header", "seed")
        self._rejected_payload("m68k-amigaos/include/exec.h")

    def test_should_reject_forbidden_bytes_under_an_unrelated_origin(self) -> None:
        """A misleading origin label cannot hide a known excluded header's bytes."""
        self.fixture.add_file(
            "m68k-amigaos/include/hidden.h", b"Synthetic excluded NDK-derived header.\n", "header")
        self._rejected_payload("m68k-amigaos/include/hidden.h")

    def test_should_reject_the_ndk_archive_inside_the_payload(self) -> None:
        """The locked preparation archive itself cannot be redistributed as source."""
        self.fixture.add_component("seed")
        self.fixture.add_file(
            "sources/ndk.tar", (self.fixture.sources / "archives/seed").read_bytes(), "source", "seed")
        self._rejected_payload("sources/ndk.tar")

    def test_should_reject_an_unaccounted_library(self) -> None:
        """An extra archive library is detected even outside the normal library tree."""
        path = self.fixture.payload / "share/hidden.a"
        path.parent.mkdir()
        path.write_bytes(b"!<arch>\n")
        self._rejected_payload("share/hidden.a")

    def test_should_reject_host_elf_misdeclared_as_an_amiga_object(self) -> None:
        """A target-object label cannot hide actual host-machine ELF bytes."""
        source = self.fixture.root / "host-object.c"
        source.write_text("int fixture_value(void) { return 1; }\n", encoding="utf-8")
        output = self.fixture.root / "host-object.o"
        subprocess.run(
            ["cc", "-std=c99", "-c", str(source), "-o", str(output)],
            cwd=self.fixture.root, env=self.fixture.environment,
            capture_output=True, text=True, timeout=30, check=True)
        content = output.read_bytes()
        self.assertEqual(content[:4], b"\x7fELF")
        self.assertGreaterEqual(len(content), 20)
        self.assertIn(content[5], (1, 2))
        byteorder = "little" if content[5] == 1 else "big"
        # ELF e_machine 62 identifies x86_64, not an Amiga target object.
        self.assertEqual(int.from_bytes(content[18:20], byteorder), 62)
        name = "m68k-amigaos/libnix/lib/ncrt0.o"
        self.fixture.add_file(name, content, "library")
        self.fixture.audit["native"][name] = {
            "format": "amiga-object", "machine": "m68k", "soname": None,
            "needed": {}, "glibc": []}
        self._rejected_payload(name)

    def test_should_reject_unaccounted_media(self) -> None:
        """Adding media-shaped fixture bytes cannot bypass exact payload coverage."""
        (self.fixture.payload / "fixture.rom").write_bytes(b"synthetic media sentinel\n")
        self._rejected_payload("fixture.rom")

    def test_should_reject_a_missing_licence_record(self) -> None:
        """A payload cannot be published when its source has no licence record."""
        del self.fixture.lock["sources"]["lha"]["licence"]
        self._rejected_payload("lha")

    def test_should_reject_a_missing_notice_file(self) -> None:
        """An obligation referencing unavailable notice bytes is not satisfied."""
        self.fixture.remove_file("notices/lha.txt")
        self._rejected_payload("notices/lha.txt")

    def test_should_reject_missing_corresponding_source(self) -> None:
        """A source-required component must include actual corresponding-source bytes."""
        self.fixture.remove_file("sources/lha.sh")
        self._rejected_payload("sources/lha.sh")

    def test_should_reject_an_executable_builder_shebang(self) -> None:
        """Even correctly hashed executable bytes cannot depend on the builder root."""
        self.fixture.add_file("bin/lha", b"#!/work/build/bin/sh\nexit 0\n", "program")
        self._rejected_payload("bin/lha")

    def test_should_allow_builder_prefixes_in_inert_notices_and_source(self) -> None:
        """Quoting a builder path in documentation is not an executable dependency."""
        for name, role in (("notices/lha.txt", "notice"), ("sources/lha.sh", "source")):
            content = (self.fixture.payload / name).read_bytes() + b"\n# Paths: /sources and /work\n"
            self.fixture.add_file(name, content, role)
        result = self._package()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.fixture.archive_output.is_file())

    def test_should_preserve_an_existing_output_archive(self) -> None:
        """Packaging refuses replacement even when the new candidate is valid."""
        output = self.fixture.archive_output
        output.write_bytes(b"previous archive\n")
        self._failed(self._package(), output.name)
        self.assertEqual(output.read_bytes(), b"previous archive\n")
        self.assertEqual(list(output.parent.iterdir()), [output])

    def test_should_reject_an_escaping_payload_link(self) -> None:
        """An external link cannot enter the archive or alter its outside target."""
        sentinel = self.fixture.root / "outside.txt"
        sentinel.write_bytes(b"outside remains unchanged\n")
        (self.fixture.payload / "bin/external").symlink_to("../../outside.txt")
        self.fixture.audit["files"]["bin/external"] = {
            "type": "symlink", "target": "../../outside.txt", "origins": ["lha"], "role": "link"}
        self._rejected_payload("bin/external")
        self.assertEqual(sentinel.read_bytes(), b"outside remains unchanged\n")

    def test_should_preserve_internal_relative_payload_links(self) -> None:
        """Safe internal aliases remain relative symlinks in the published archive."""
        (self.fixture.payload / "bin/lha-alias").symlink_to("lha")
        self.fixture.audit["files"]["bin/lha-alias"] = {
            "type": "symlink", "target": "lha", "origins": ["lha"], "role": "link"}
        result = self._package()
        self.assertEqual(result.returncode, 0, result.stderr)
        with tarfile.open(self.fixture.archive_output) as archive:
            alias = archive.getmember("bin/lha-alias")
            self.assertTrue(alias.issym())
            self.assertEqual(alias.linkname, "lha")

    def test_should_preserve_an_existing_output_symlink(self) -> None:
        """Refusing an output link preserves both that link and its external target."""
        sentinel = self.fixture.root / "prior.tar"
        sentinel.write_bytes(b"previous archive\n")
        output = self.fixture.archive_output
        output.symlink_to(sentinel)
        self._failed(self._package(), output.name)
        self.assertTrue(output.is_symlink())
        self.assertEqual(output.readlink(), sentinel)
        self.assertEqual(sentinel.read_bytes(), b"previous archive\n")
        self.assertEqual(list(output.parent.iterdir()), [output])


if __name__ == "__main__":
    unittest.main()
