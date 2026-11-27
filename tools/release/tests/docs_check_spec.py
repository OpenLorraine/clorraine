"""Specification tests for the release docs check.

Each case starts from a clean fixture release (see `docs_fixture`), changes one
thing, and checks the exit code and the rule the checker reports. Exit codes:
0 pass, 1 findings, 2 usage or manifest error, 3 unreachable, 4 deploy not
current.
"""

from __future__ import annotations

import datetime as dt
import unittest

from docs_fixture import TODAY, VERSION, WINDOWS, Fixture, record


class StaticSpecTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()

    def tearDown(self) -> None:
        self.fx.cleanup()

    def assert_finding(self, code: int, out: str, rule: str, *needles: str) -> None:
        self.assertEqual(code, 1, out)
        lines = [line for line in out.splitlines() if f"[{rule}]" in line]
        self.assertTrue(lines, f"no [{rule}] finding in:\n{out}")
        for needle in needles:
            self.assertTrue(any(needle in line for line in lines), f"{needle!r} not in [{rule}] lines:\n{out}")

    def test_should_pass_a_clean_release(self) -> None:
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)
        self.assertIn("0 findings", out)

    def test_should_validate_the_version_argument(self) -> None:
        for bad in ("1.3", "1.3.0-rc.1", ""):
            with self.subTest(version=bad):
                code, out = self.fx.run("--version", bad)
                self.assertEqual(code, 2, out)
        code, out = self.fx.run("--version", "v1.3.0")
        self.assertEqual(code, 0, out)

    def test_should_refuse_a_version_the_tree_does_not_carry(self) -> None:
        self.fx.replace("compiler/Cargo.toml", 'version = "1.3.0"', 'version = "1.2.0"')
        code, out = self.fx.run()
        self.assert_finding(code, out, "version-input", "compiler/Cargo.toml")

    def test_should_report_a_stale_version_with_a_ready_allowlist_entry(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nToday v1.2.0 is the current release.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "site/guide/roadmap.md:3", "v1.2.0")
        self.assertIn('"file": "site/guide/roadmap.md"', out)
        self.assertIn('"reason": ""', out)
        self.assertIn(f'"reviewed_for": "{VERSION}"', out)

    def test_should_accept_an_allowlisted_historical_mention(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nDeveloper preview since v1.2.0.\n")
        self.fx.manifest["allowlist"].append({
            "file": "site/guide/roadmap.md", "phrase": "Developer preview since v1.2.0",
            "reason": "the installer first shipped in v1.2.0", "reviewed_for": VERSION,
        })
        self.fx.save_manifest()
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)

    def test_should_fail_an_allowlist_entry_whose_phrase_changed(self) -> None:
        self.fx.replace("site/status.md", "v1.1.2 remains the latest stable release", "v1.1.2 is still the latest stable release")
        code, out = self.fx.run()
        self.assert_finding(code, out, "allowlist-unused", "site/status.md")
        self.assert_finding(code, out, "stale-version", "v1.1.2")

    def test_should_require_every_allowlist_entry_to_be_reviewed_for_this_release(self) -> None:
        self.fx.manifest["allowlist"][0]["reviewed_for"] = "1.2.0"
        self.fx.save_manifest()
        code, out = self.fx.run()
        self.assert_finding(code, out, "allowlist-review", "site/status.md")

    def test_should_reject_an_allowlist_entry_without_a_reason(self) -> None:
        self.fx.manifest["allowlist"][0]["reason"] = ""
        self.fx.save_manifest()
        code, out = self.fx.run()
        self.assertEqual(code, 2, out)
        self.assertIn("reason", out)

    def test_should_report_a_stale_two_part_version(self) -> None:
        self.fx.write("site/guide/known-limitations.md", "# Limitations\n\nThe limitations of Clorraine v1.2.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "v1.2")

    def test_should_not_confuse_a_longer_version_with_the_release(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nv1.3.10 is planned; v1.3.0 is out.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "v1.3.10")
        stale = [line for line in out.splitlines() if "[stale-version]" in line]
        self.assertFalse(any("v1.3.0" in line and "v1.3.10" not in line for line in stale), out)

    def test_should_ignore_third_party_versions_without_a_v(self) -> None:
        self.fx.write("site/amiga/emulator.md", "# Emulator\n\nUse FS-UAE 3.2.35 or newer.\n")
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)

    def test_should_match_allowlisted_phrases_across_markup_and_line_wraps(self) -> None:
        self.fx.replace("site/status.md", "v1.1.2 remains the latest stable release",
                        "**v1.1.2** remains the latest\nstable release")
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)

    def test_should_require_every_platform_command_on_every_install_surface(self) -> None:
        home = "site/.vitepress/theme/components/HomePage.vue"
        self.fx.replace(home, WINDOWS.replace("{v}", VERSION), "")
        code, out = self.fx.run()
        self.assert_finding(code, out, "platform", "Windows", home)

    def test_should_require_an_install_entry_per_platform_in_the_sidebar(self) -> None:
        self.fx.replace("site/.vitepress/config.ts",
                        '      { text: "Install on Windows", link: "/guide/quick-start#install-on-windows" },\n', "")
        code, out = self.fx.run()
        self.assert_finding(code, out, "platform", "Windows", "site/.vitepress/config.ts")

    def test_should_report_an_install_command_pinned_to_an_old_release(self) -> None:
        self.fx.replace("docs/linux-cli.md", f"download/v{VERSION}/", "download/v1.2.0/")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "docs/linux-cli.md", "v1.2.0")
        self.assert_finding(code, out, "platform", "Linux", "docs/linux-cli.md")

    def test_should_report_banned_wording(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nmacOS ships with the Next Release.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "banned", "site/guide/roadmap.md:3", "next release")

    def test_should_require_exactly_one_checksum_placeholder_in_the_committed_notes(self) -> None:
        notes = self.fx.read("RELEASE-NOTES.md")
        self.fx.write("RELEASE-NOTES.md", notes.replace("PENDING-CHECKSUMS: the release steps replace this line with the asset table.\n",
                                                       "| a | b |\n"))
        code, out = self.fx.run()
        self.assert_finding(code, out, "placeholder", "RELEASE-NOTES.md")
        self.fx.write("RELEASE-NOTES.md", notes + "PENDING-CHECKSUMS: again.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "placeholder", "RELEASE-NOTES.md")

    def test_should_require_a_recent_brief_date(self) -> None:
        code, out = self.fx.run(today=TODAY + dt.timedelta(days=15))
        self.assert_finding(code, out, "dated", "site/project/brief.md")
        code, out = self.fx.run(today=TODAY - dt.timedelta(days=1))
        self.assert_finding(code, out, "dated", "site/project/brief.md")
        code, out = self.fx.run(today=TODAY + dt.timedelta(days=14))
        self.assertNotIn("[dated]", out)

    def test_should_exempt_old_changelog_sections_but_not_the_current_one(self) -> None:
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)
        self.fx.replace("CHANGELOG.md", "- Windows installer.\n", "- Windows installer, like v1.2.0.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "CHANGELOG.md", "v1.2.0")

    def test_should_require_a_dated_top_changelog_heading(self) -> None:
        self.fx.replace("CHANGELOG.md", f"## [{VERSION}] - {TODAY.isoformat()}", f"## [{VERSION}]")
        code, out = self.fx.run()
        self.assert_finding(code, out, "changelog", "CHANGELOG.md")

    def test_should_report_a_release_facing_file_the_manifest_does_not_classify(self) -> None:
        self.fx.write("docs/macos-cli.md", "# macOS CLI\n\nSoon.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "unclassified", "docs/macos-cli.md")

    def test_should_validate_the_review_record(self) -> None:
        path = f"docs/release-reviews/v{VERSION}.md"
        good = record()
        cases = {
            "missing": None,
            "unticked": good.replace("- [x] Status page matches this release", "- [ ] Status page matches this release"),
            "edited": good.replace("Status page matches this release", "Status page looks fine"),
            "extra": good.replace("## After deploy", "- [x] Something else\n\n## After deploy"),
            "email": good.replace("Prepared by: agent", "Prepared by: someone@example.com"),
            "handle": record(reviewer="maintainer"),
        }
        for name, text in cases.items():
            with self.subTest(case=name):
                if text is None:
                    (self.fx.root / path).unlink()
                    self.fx.commit("remove record")
                else:
                    self.fx.write(path, text)
                code, out = self.fx.run()
                self.assert_finding(code, out, "review", path)
                self.fx.write(path, good)

    def test_should_accept_an_after_deploy_section_still_unticked_before_the_tag(self) -> None:
        self.fx.write(f"docs/release-reviews/v{VERSION}.md", record(after=False))
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)

    def test_should_cover_only_the_tokens_inside_an_allowlisted_phrase(self) -> None:
        self.fx.replace("site/status.md", "FS-UAE 3.2.35 is the tested emulator.",
                        "FS-UAE 3.2.35 is the tested emulator. v1.0.0 is old.")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "site/status.md", "v1.0.0")

    def test_should_not_cover_a_token_the_phrase_only_partly_spans(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nPreview since v1.2.0 here.\n")
        self.fx.manifest["allowlist"].append({
            "file": "site/guide/roadmap.md", "phrase": "since v1.2", "reason": "test", "reviewed_for": VERSION,
        })
        self.fx.save_manifest()
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "v1.2.0")

    def test_should_require_each_release_claim(self) -> None:
        self.fx.replace("site/status.md", "is published as a developer preview", "is available")
        code, out = self.fx.run()
        self.assert_finding(code, out, "required", "site/status.md", "release state")

    def test_should_require_the_quick_start_heading_per_platform(self) -> None:
        self.fx.replace("site/guide/quick-start.md", "## Install on Windows", "## Windows")
        code, out = self.fx.run()
        self.assert_finding(code, out, "platform", "site/guide/quick-start.md", "Install on Windows")

    def test_should_tie_the_sidebar_text_to_its_link(self) -> None:
        self.fx.replace("site/.vitepress/config.ts",
                        '{ text: "Install on Windows", link: "/guide/quick-start#install-on-windows" }',
                        '{ text: "Install on Windows", link: "/guide/windows" },\n      '
                        '{ text: "Other", link: "/guide/quick-start#install-on-windows" }')
        code, out = self.fx.run()
        self.assert_finding(code, out, "platform", "site/.vitepress/config.ts", "Windows")

    def test_should_check_every_version_input(self) -> None:
        cases = {
            "tools/vscode/package.json": ('"version": "1.3.0"', '"version": "1.2.0"'),
            "tools/vscode/CHANGELOG.md": ("## 1.3.0", "## 1.3.1"),
            "RELEASE-NOTES.md": ("# Clorraine v1.3.0 developer preview", "# Clorraine developer preview"),
        }
        for path, (old, new) in cases.items():
            with self.subTest(path=path):
                original = self.fx.read(path)
                self.fx.replace(path, old, new)
                code, out = self.fx.run()
                self.assert_finding(code, out, "version-input", path)
                self.fx.write(path, original)

    def test_should_require_a_recent_changelog_date(self) -> None:
        self.fx.replace("CHANGELOG.md", f"## [{VERSION}] - {TODAY.isoformat()}", f"## [{VERSION}] - 2026-10-01")
        code, out = self.fx.run()
        self.assert_finding(code, out, "changelog", "CHANGELOG.md", "days old")

    def test_should_accept_the_published_notes_copy_in_the_working_tree(self) -> None:
        notes = self.fx.read("RELEASE-NOTES.md")
        filled = notes.replace("PENDING-CHECKSUMS: the release steps replace this line with the asset table.",
                               "| install.sh | 1 | abc |")
        (self.fx.root / "RELEASE-NOTES.md").write_text(filled)
        code, out = self.fx.run()
        self.assertEqual(code, 0, out)

    def test_should_check_the_placeholder_in_the_working_tree_too(self) -> None:
        notes = self.fx.read("RELEASE-NOTES.md")
        (self.fx.root / "RELEASE-NOTES.md").write_text(notes + "PENDING-CHECKSUMS: an uncommitted second line.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "placeholder", "RELEASE-NOTES.md", "working-tree")

    def test_should_exempt_only_the_placeholder_wording_on_the_placeholder_line(self) -> None:
        self.fx.replace("RELEASE-NOTES.md", "PENDING-CHECKSUMS: the release steps replace this line with the asset table.",
                        "PENDING-CHECKSUMS: filled in with the next release.")
        code, out = self.fx.run()
        self.assert_finding(code, out, "banned", "RELEASE-NOTES.md", "next release")

    def test_should_check_the_record_header_fields_and_item_order(self) -> None:
        path = f"docs/release-reviews/v{VERSION}.md"
        good = record()
        swapped = good.replace("- [x] Status page matches this release\n- [x] Project brief matches this release",
                               "- [x] Project brief matches this release\n- [x] Status page matches this release")
        self.assertNotEqual(swapped, good)
        cases = {
            "heading": good.replace(f"# Release review: v{VERSION}", "# Review"),
            "prepared": good.replace("Prepared by: agent\n", ""),
            "date": good.replace(f"Date: {TODAY.isoformat()}", "Date: today"),
            "order": swapped,
        }
        for name, text in cases.items():
            with self.subTest(case=name):
                self.fx.write(path, text)
                code, out = self.fx.run()
                self.assert_finding(code, out, "review", path)
        self.fx.write(path, good)

    def test_should_require_the_record_to_be_committed(self) -> None:
        path = f"docs/release-reviews/v{VERSION}.md"
        import subprocess
        subprocess.run(["git", "rm", "-q", "--cached", path], cwd=self.fx.root, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "untrack"], cwd=self.fx.root, check=True)
        code, out = self.fx.run()
        self.assert_finding(code, out, "review", path, "not committed")

    def test_should_reject_more_than_one_leading_v(self) -> None:
        code, out = self.fx.run("--version", "vv1.3.0")
        self.assertEqual(code, 2, out)

    def test_should_reject_manifest_entries_without_review_or_reason(self) -> None:
        self.fx.manifest["allowlist"][0]["reviewed_for"] = "soon"
        self.fx.save_manifest()
        self.assertEqual(self.fx.run()[0], 2)
        self.fx.manifest["allowlist"][0]["reviewed_for"] = VERSION
        self.fx.manifest["excluded"][0]["reason"] = ""
        self.fx.save_manifest()
        self.assertEqual(self.fx.run()[0], 2)

    def test_should_report_an_unreadable_file(self) -> None:
        (self.fx.root / "site/status.md").write_bytes(b"\xff\xfe not utf-8")
        code, out = self.fx.run()
        self.assert_finding(code, out, "unreadable", "site/status.md")

    def test_should_require_a_release_link_to_name_the_whole_version(self) -> None:
        self.fx.write("site/guide/roadmap.md",
                      "# Roadmap\n\nSee https://github.com/OpenLorraine/clorraine/releases/tag/v1.3 for details.\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "stale-version", "releases/tag/v1.3")
        self.assertEqual(out.count("[stale-version]"), 1, out)

    def test_should_classify_markdown_files_whatever_the_extension_case(self) -> None:
        self.fx.write("CONTRIBUTING.MD", "# Contributing\n")
        code, out = self.fx.run()
        self.assert_finding(code, out, "unclassified", "CONTRIBUTING.MD")


if __name__ == "__main__":
    unittest.main()
