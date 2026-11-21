"""Local prerequisite, path, interruption and real-lock failure specifications.

Observation programs establish only literal path/byte preservation. Successful
compiler installations, idempotent updates, completion and ABI transitions are
covered by the separately invoked actual-context classes.
"""

from __future__ import annotations

from native_install_fixture import StateShellFixture, tree_state
from native_manifest_data import RELEASE
from native_manifest_producer_fixture import ProducerFixture, ScratchCase
from native_transport_fixture import run_process


class NativeInstallLocalTests(ScratchCase):
    """Reach real public-shell failure boundaries using independently assembled inputs."""

    def case(self) -> tuple[ProducerFixture, StateShellFixture]:
        """Require valid production assembly before installing any negative observer."""
        root = self.root / ("case-" + str(len(list(self.root.iterdir()))))
        root.mkdir()
        fixture = ProducerFixture(self, root)
        private = root / "state-shell"
        private.mkdir()
        shell = StateShellFixture(private, fixture.bootstrap, fixture.manifest, fixture.paths,
                                  probes={"ubuntu": "22.04", "architecture": "x86_64",
                                          "python": [3, 10, 12]})
        return fixture, shell

    def failed_prerequisite(self, operation: str, package: str, *, missing: bool = False) -> None:
        """Require the selected failure child/launch error and actionable package guidance."""
        _, shell = self.case()
        canary = shell.root / "outside-canary"
        canary.write_bytes(b"unchanged outside installer state\n")
        shell.fault = {"operation": operation, "diagnostic": "fixture missing " + package,
                       "mode": "missing" if missing else "child"}
        if package == "libc6-dev":
            # A version query cannot legitimately fail because C headers are
            # absent. Leave it real and target the actual C-input probe instead.
            shell.fault["header_probe"] = True
        result = shell.invoke(["--release", RELEASE])
        shell.assert_fault_reached(self, operation)
        shell.assert_failed_safely(self, result)
        guidance = (result.stdout + result.stderr).lower()
        self.assertIn("apt", guidance)
        self.assertIn(package, guidance)
        self.assertFalse(any(line.startswith(("export PATH=", "code --install-extension "))
                             for line in result.stdout.splitlines()))
        self.assertEqual(canary.read_bytes(), b"unchanged outside installer state\n")

    def test_should_report_missing_compiler_without_launching_a_package_manager(self) -> None:
        """A real compiler launch error must produce a remedy, not dependency compilation."""
        self.failed_prerequisite("compiler", "gcc", missing=True)

    def test_should_report_missing_c_headers_without_installing_them(self) -> None:
        """A failed host header probe names the Ubuntu development-header remedy."""
        self.failed_prerequisite("compiler", "libc6-dev")

    def test_should_report_missing_validated_packages_without_sudo_or_apt_execution(self) -> None:
        """The package-availability failure must reach the real installer comparison."""
        self.failed_prerequisite("packages", "gcc")

    def test_should_reject_a_failed_preparation_child_before_activation(self) -> None:
        """Authenticated preparation that exits nonzero cannot complete an installation."""
        _, shell = self.case()
        shell.fault = {"operation": "preparation", "diagnostic": "fixture preparation failed"}
        result = shell.invoke(["--release", RELEASE])
        shell.assert_fault_reached(self, "preparation")
        shell.assert_failed_safely(self, result)
        self.assertTrue(any(word in result.stderr.lower() for word in ("ndk", "prepar")))

    def test_should_preserve_an_unrelated_root_and_literal_old_program_observer(self) -> None:
        """An inert program and current link are not a handwritten completed installation."""
        _, shell = self.case()
        root = shell.install_root
        observer = root / "installations" / "literal-observer" / "bin" / "old-program"
        observer.parent.mkdir(parents=True)
        observer.write_bytes(b"#!/bin/sh\nprintf 'literal observer only\\n'\n")
        observer.chmod(0o755)
        (root / "current").symlink_to("installations/literal-observer")
        (root / "unrelated-settings").write_bytes(b"never overwrite user state\n")
        before = tree_state(root)
        original = run_process([str(observer)], shell.root, shell.environment)
        self.assertEqual(original.returncode, 0, original.stderr)
        result = shell.invoke(["--release", RELEASE])
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(any(word in result.stderr.lower() for word in
                            ("root", "owned", "ownership", "unrelated", "nonempty", "non-empty")))
        self.assertEqual(tree_state(root), before)
        after = run_process([str(observer)], shell.root, shell.environment)
        self.assertEqual((after.returncode, after.stdout), (0, "literal observer only\n"))
        self.assertFalse([item for item in shell.events("command")
                          if item["name"] in ("sudo", "apt", "apt-get", "code")])

    def test_should_reject_escaping_managed_aliases_without_touching_outside_files(self) -> None:
        """Each public managed path must not redirect writes into another directory."""
        for name in ("bin", "components", "installations", "current"):
            with self.subTest(path=name):
                _, shell = self.case()
                outside = shell.root / "outside"
                outside.mkdir()
                (outside / "canary").write_bytes(b"outside remains unchanged\n")
                shell.install_root.mkdir()
                (shell.install_root / name).symlink_to(outside, target_is_directory=True)
                before_root, before_outside = tree_state(shell.install_root), tree_state(outside)
                result = shell.invoke(["--release", RELEASE])
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(any(word in result.stderr.lower() for word in
                                    ("symlink", "root", "destination", "owned", "nonempty", "non-empty")))
                self.assertEqual(tree_state(shell.install_root), before_root)
                self.assertEqual(tree_state(outside), before_outside)

    def test_should_exclude_a_canonical_alias_and_release_the_os_lock_after_death(self) -> None:
        """A held real lock blocks another spelling; a leftover file is not a live lock."""
        fixture, first = self.case()
        first.install_root.mkdir()
        asset = next(item for item in fixture.manifest["assets"] if item["kind"] == "core")
        first.http_url = asset["url"]
        running = first.start(["--release", RELEASE])
        self.addCleanup(running.close)
        running.wait_ready("http-ready", timeout=45)
        root_before = tree_state(first.install_root)
        peer = fixture.root / "peer-shell"
        peer.mkdir()
        second = StateShellFixture(peer, fixture.bootstrap, fixture.manifest, fixture.paths,
                                   probes=first.configuration["probes"])
        alias = fixture.root / "another root spelling"
        alias.symlink_to(first.install_root, target_is_directory=True)
        result = second.invoke(["--release", RELEASE], root=alias, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(any(word in result.stderr.lower() for word in
                            ("lock", "another installer", "running", "busy", "concurrent")))
        self.assertIsNone(running.process.poll(), "first installer lost its lock authority")
        self.assertEqual(tree_state(first.install_root), root_before)
        running.close()
        self.assertFalse((first.install_root / "current").exists())
        # Retry is deliberately stopped at a transport failure. Reaching this
        # new body read proves lock release and incomplete-download rejection,
        # not that miniature compilers or wheel environments actually work.
        second.route(asset["url"], path=fixture.paths[asset["identity"]], read_error="timeout")
        retry = second.invoke(["--release", RELEASE], root=alias, timeout=45)
        self.assertNotEqual(retry.returncode, 0)
        self.assertTrue(second.requested(asset["url"]))
        self.assertTrue(second.events("read-timeout"))
        self.assertFalse((first.install_root / "current").exists())
