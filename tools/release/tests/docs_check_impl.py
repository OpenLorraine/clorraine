"""Implementation tests for the release docs check internals."""

from __future__ import annotations

import datetime as dt
import subprocess
import sys
import unittest
from pathlib import Path

from docs_fixture import Fixture

import docs_rules
from docs_rules import ManifestError, glob_regex, normalize, parse_day, tokens

CHECKER = Path(__file__).resolve().parents[1] / "docs_check.py"


class NormalizeTests(unittest.TestCase):
    def test_should_turn_links_into_label_and_url(self) -> None:
        self.assertEqual(normalize("see [the **guide**](https://x/y) now").text, "see the guide https://x/y now")

    def test_should_decode_entities_and_drop_zero_width_spaces(self) -> None:
        self.assertEqual(normalize("a &amp;&amp; b&#8203;​ &quot;c&quot;").text, 'a && b "c"')

    def test_should_collapse_whitespace_and_markers(self) -> None:
        self.assertEqual(normalize("  `code`\n\n**bold**  __u__\t end ").text, "code bold u end")

    def test_should_drop_blockquote_markers_at_line_starts(self) -> None:
        self.assertEqual(normalize("> **Preview.** The v1.3.0\n> builds are out; a > b").text,
                         "Preview. The v1.3.0 builds are out; a > b")

    def test_should_map_every_character_to_its_source_line(self) -> None:
        text = normalize("one\n\n**two**\nthree")
        self.assertEqual(text.line_at(text.text.index("two")), 3)
        self.assertEqual(text.line_at(text.text.index("three")), 4)
        self.assertEqual(text.line_at(10_000), 4)


class TokenTests(unittest.TestCase):
    def names(self, text: str) -> list[str]:
        return [token.text for token in tokens(normalize(text).text)]

    def test_should_find_two_and_three_part_tokens_with_boundaries(self) -> None:
        self.assertEqual(self.names("v1.3 and v1.3.0, then v1.3.10."), ["v1.3", "v1.3.0", "v1.3.10"])

    def test_should_keep_a_prerelease_suffix_with_its_token(self) -> None:
        found = tokens("try v1.4.0-rc.1 soon")
        self.assertEqual((found[0].text, found[0].prerelease), ("v1.4.0-rc.1", "rc.1"))
        self.assertFalse(found[0].is_current("1.4.0"))

    def test_should_find_versions_in_urls_and_asset_names(self) -> None:
        text = "releases/download/v1.2.0/install.sh and clorraine-1.2.0-x86_64-linux.tar.gz"
        self.assertEqual(self.names(text), ["releases/download/v1.2.0", "v1.2.0", "clorraine-1.2.0"])

    def test_should_skip_words_that_only_contain_a_v(self) -> None:
        self.assertEqual(self.names("nv1.2 dev1.2.3 x.v1.2 FS-UAE 3.2.35"), [])

    def test_should_judge_two_part_tokens_by_major_and_minor(self) -> None:
        token = tokens("v1.3")[0]
        self.assertTrue(token.is_current("1.3.7"))
        self.assertFalse(token.is_current("1.4.0"))


class ContextTests(unittest.TestCase):
    def test_should_suggest_a_phrase_inside_one_table_cell(self) -> None:
        text = normalize("| Area | State |\n|---|---|\n| Installer | Preview since v1.2.0: one command installs it |").text
        token = tokens(text)[0]
        self.assertEqual(docs_rules.context(text, token), "Preview since v1.2.0: one command installs it")

    def test_should_suggest_a_phrase_without_the_heading_marker(self) -> None:
        text = normalize("Intro text here.\n\n## Upgrading from v1.2.0\n\nRun the installer again.").text
        self.assertNotIn("##", docs_rules.context(text, tokens(text)[0]))


class DateAndGlobTests(unittest.TestCase):
    def test_should_read_english_month_names_only(self) -> None:
        self.assertEqual(parse_day("27", "November", "2026"), dt.date(2026, 11, 27))
        self.assertIsNone(parse_day("27", "november", "2026"))
        self.assertIsNone(parse_day("31", "November", "2026"))

    def test_should_report_future_and_old_dates(self) -> None:
        today = dt.date(2026, 11, 27)
        self.assertIsNone(docs_rules.date_problem(today, today))
        self.assertIn("future", docs_rules.date_problem(today + dt.timedelta(days=1), today))
        self.assertIn("15 days", docs_rules.date_problem(today - dt.timedelta(days=15), today))

    def test_should_glob_across_directories_only_with_double_star(self) -> None:
        rx = glob_regex("site/**/*.md")
        self.assertTrue(rx.match("site/index.md"))
        self.assertTrue(rx.match("site/guide/deep/page.md"))
        self.assertFalse(rx.match("site/guide/page.vue"))
        self.assertFalse(glob_regex("site/*.md").match("site/guide/page.md"))


class ManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()

    def tearDown(self) -> None:
        self.fx.cleanup()

    def load_error(self) -> str:
        self.fx.save_manifest()
        with self.assertRaises(ManifestError) as caught:
            docs_rules.load_manifest(self.fx.root / "tools/release/docs-manifest.json", self.fx.root)
        return str(caught.exception)

    def test_should_reject_unknown_and_missing_keys(self) -> None:
        self.fx.manifest["extra"] = 1
        self.assertIn("unknown keys", self.load_error())
        del self.fx.manifest["extra"], self.fx.manifest["banned"]
        self.assertIn("missing", self.load_error())

    def test_should_reject_a_file_git_does_not_track(self) -> None:
        self.fx.manifest["install_surfaces"].append("site/missing.md")
        self.assertIn("site/missing.md", self.load_error())

    def test_should_reject_an_allowlist_phrase_without_a_version(self) -> None:
        self.fx.manifest["allowlist"][0]["phrase"] = "remains the latest stable release"
        self.assertIn("names no version token", self.load_error())

    def test_should_reject_a_bad_regex_and_a_missing_group(self) -> None:
        self.fx.manifest["dated"][0]["pattern"] = "Updated (?P<day>\\d+"
        self.assertIn("not a valid regex", self.load_error())
        self.fx.manifest["dated"][0]["pattern"] = "Updated (?P<day>\\d+)"
        self.assertIn("needs the groups", self.load_error())

    def test_should_reject_a_platform_command_without_the_version(self) -> None:
        self.fx.manifest["platforms"][0]["command"] = "curl https://example.invalid/install.sh"
        self.assertIn("{version}", self.load_error())


class TemplateTests(unittest.TestCase):
    def test_should_parse_the_committed_review_template(self) -> None:
        template = (Path(__file__).resolve().parents[3] / "docs/release-reviews/TEMPLATE.md").read_text(encoding="utf-8")
        sections = docs_rules.checklist(template)
        for name in docs_rules.SECTIONS:
            self.assertTrue(sections.get(name), f"no items under ## {name}")
            self.assertFalse(any(ticked for ticked, _ in sections[name]), f"a template item under {name} is ticked")


class CliTests(unittest.TestCase):
    def run_cli(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, str(CHECKER), *args], capture_output=True, text=True, timeout=60)

    def test_should_print_usage_and_exit_two_without_a_version(self) -> None:
        result = self.run_cli()
        self.assertEqual(result.returncode, 2)
        self.assertIn("--version", result.stderr)

    def test_should_describe_itself_in_help(self) -> None:
        result = self.run_cli("--help")
        self.assertEqual(result.returncode, 0)
        self.assertIn("--live", result.stdout)

    def test_should_report_a_bad_date_as_usage(self) -> None:
        result = self.run_cli("--version", "1.3.0", "--today", "27-11-2026")
        self.assertEqual(result.returncode, 2)

    def test_should_deduplicate_identical_findings(self) -> None:
        fx = Fixture()
        try:
            fx.write("site/guide/roadmap.md", "# Roadmap\n\nnext release\n")
            fx.manifest["banned"].append({"pattern": "next release", "message": "name the release instead"})
            fx.save_manifest()
            code, out = fx.run()
            self.assertEqual(code, 1)
            self.assertEqual(out.count("roadmap.md:3 [banned]"), 1, out)
        finally:
            fx.cleanup()


if __name__ == "__main__":
    unittest.main()
