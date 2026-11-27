"""Specification tests for the live release docs check.

A clean fixture release (`docs_fixture`) plus a fake deployed site and fake
`gh` answers (`docs_live_fixture`) pass; each case changes one answer.
"""

from __future__ import annotations

import unittest

from docs_fixture import VERSION, WINDOWS, Fixture, record
from docs_live_fixture import (DISCORD, HASHES, MARKETPLACE, PACKAGE, REPO, LiveSite, Response, api,
                               release_body)

import docs_live  # noqa: F401  (imported for the fixture's sleep override)

SITE = "https://openlorraine.github.io/clorraine/"


class LiveSpecTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()
        self.site = LiveSite(head=self.fx.head())
        self.deployed = None

    def tearDown(self) -> None:
        self.fx.cleanup()

    def live(self) -> tuple[int, str]:
        if self.deployed is None:
            self.site.deploy(self.fx.head(), "success")
        return self.fx.run("--live", fetcher=self.site.transport, gh=self.site.gh)

    def serve(self, path: str, html: str, status: int = 200) -> None:
        self.site.http[SITE + path] = Response(status, {}, html.encode())

    def page_with(self, path: str, extra: str) -> str:
        return self.site.pages[path].replace("</main>", extra + "</main>")

    def assert_finding(self, code: int, out: str, rule: str, *needles: str, exit_code: int = 1) -> None:
        self.assertEqual(code, exit_code, out)
        lines = [line for line in out.splitlines() if f"[{rule}]" in line]
        self.assertTrue(lines, f"no [{rule}] finding in:\n{out}")
        for needle in needles:
            self.assertTrue(any(needle in line for line in lines), f"{needle!r} not in [{rule}] lines:\n{out}")

    # Clean run -----------------------------------------------------------

    def test_should_pass_a_clean_deployed_release(self) -> None:
        code, out = self.live()
        self.assertEqual(code, 0, out)
        self.assertIn("0 findings, 0 unreachable", out)
        self.assertIn("published as a prerelease", out)
        # Only the theme's edit link would be probed through the repository API.
        self.assertNotIn(api("repos/OpenLorraine/clorraine"), self.site.gh_calls, "edit links are not probed")

    def test_should_fetch_pages_with_a_cache_busting_query(self) -> None:
        self.live()
        pages = [url for url in self.site.requests if url.startswith(SITE) and "sitemap" not in url]
        self.assertTrue(pages)
        self.assertTrue(all("?release-docs-check=" in url for url in pages), pages)

    def test_should_require_the_after_deploy_items_in_the_live_check(self) -> None:
        self.fx.write(f"docs/release-reviews/v{VERSION}.md", record(after=False))
        code, out = self.live()
        self.assert_finding(code, out, "review", "After deploy")

    # Freshness -----------------------------------------------------------

    def test_should_stop_while_the_newest_deployment_is_not_finished(self) -> None:
        self.deployed = True
        self.site.deploy(self.fx.head(), "in_progress")
        code, out = self.live()
        self.assert_finding(code, out, "deploy-not-current", "in_progress", exit_code=4)
        self.assertEqual(self.site.requests, [], "nothing is fetched before the deploy is current")

    def test_should_stop_when_the_site_changed_after_the_deployed_commit(self) -> None:
        self.deployed = True
        old = self.fx.head()
        self.fx.write("site/guide/new-page.md", "# New page\n")
        self.site.deploy(old, "success")
        code, out = self.live()
        self.assert_finding(code, out, "deploy-not-current", "site changed", exit_code=4)

    def test_should_accept_a_deployed_commit_when_later_commits_leave_the_site_alone(self) -> None:
        self.deployed = True
        old = self.fx.head()
        self.fx.write("docs/linux-cli.md", self.fx.read("docs/linux-cli.md") + "\nMore notes.\n")
        self.site.deploy(old, "success")
        code, out = self.live()
        self.assertEqual(code, 0, out)

    def test_should_stop_when_other_deployed_inputs_changed(self) -> None:
        for path in ("docs/amiga-sdk/coverage.md", "demo/poster.png", "editors/clorraine.tmLanguage.json"):
            with self.subTest(path=path):
                self.deployed = True
                old = self.fx.head()
                self.fx.write(path, f"changed {path}\n")
                self.site.deploy(old, "success")
                code, out = self.live()
                self.assert_finding(code, out, "deploy-not-current", "site changed", exit_code=4)

    def test_should_ignore_punctuation_after_a_release_url_in_text(self) -> None:
        url = f"https://github.com/OpenLorraine/clorraine/releases/download/v{VERSION}/install.sh"
        self.serve("status", self.page_with("status", f"<p>Download {url}.</p>"))
        code, out = self.live()
        self.assertEqual(code, 0, out)

    def test_should_stop_when_the_sitemap_lists_other_pages(self) -> None:
        self.site.extra_sitemap.append(SITE + "guide/old-page")
        code, out = self.live()
        self.assert_finding(code, out, "deploy-not-current", "old-page", exit_code=4)
        self.assertFalse([url for url in self.site.requests if "release-docs-check=" in url], "no page fetched")

    def test_should_print_static_findings_when_the_deploy_is_not_current(self) -> None:
        self.fx.write("site/guide/roadmap.md", "# Roadmap\n\nships with the next release\n")
        self.deployed = True
        self.site.deploy(self.fx.head(), "queued")
        code, out = self.live()
        self.assertEqual(code, 4, out)
        self.assertIn("[banned]", out)

    def test_should_report_a_sitemap_location_outside_the_site(self) -> None:
        self.site.extra_sitemap.append("https://example.com/clorraine/status")
        code, out = self.live()
        self.assert_finding(code, out, "sitemap", "example.com")

    # Rendered pages ------------------------------------------------------

    def test_should_check_rendered_versions_against_the_page_source_allowlist(self) -> None:
        self.serve("status", self.page_with("status", "<p>v1.2.0 notes</p>"))
        self.fx.manifest["allowlist"].append({
            "file": "site/.vitepress/theme/Layout.vue", "phrase": "v0.9.0 legacy",
            "reason": "the theme keeps the first preview's label", "reviewed_for": VERSION,
        })
        self.fx.write("site/.vitepress/theme/Layout.vue",
                      f"<template><span>v{VERSION} for Linux and Windows</span><i>v0.9.0 legacy</i></template>\n")
        self.fx.save_manifest()
        self.serve("project/brief", self.page_with("project/brief", "<i>v0.9.0 legacy</i>"))
        code, out = self.live()
        self.assert_finding(code, out, "stale-version", "/clorraine/status", "v1.2.0")
        self.assertNotIn("v0.9.0", "\n".join(line for line in out.splitlines() if "[stale-version]" in line), out)

    def test_should_apply_a_home_page_theme_entry_to_the_home_page_only(self) -> None:
        home = "site/.vitepress/theme/components/HomePage.vue"
        self.fx.manifest["allowlist"].append({
            "file": home, "phrase": "since v0.9.0", "reason": "the home page names the first preview",
            "reviewed_for": VERSION,
        })
        self.fx.write(home, self.fx.read(home).replace("</pre>", "</pre><p>since v0.9.0</p>"))
        self.fx.save_manifest()
        self.serve("", self.page_with("", "<p>since v0.9.0</p>"))
        self.serve("status", self.page_with("status", "<p>since v0.9.0</p>"))
        code, out = self.live()
        stale = [line for line in out.splitlines() if "[stale-version]" in line]
        self.assertEqual(code, 1, out)
        self.assertEqual(len(stale), 1, out)
        self.assertIn("/clorraine/status", stale[0])

    def test_should_report_banned_wording_on_a_rendered_page(self) -> None:
        self.serve("status", self.page_with("status", "<p>macOS ships with the Next Release.</p>"))
        code, out = self.live()
        self.assert_finding(code, out, "banned", "/clorraine/status")

    def test_should_report_a_version_only_in_the_rendered_description(self) -> None:
        html = self.site.pages["status"].replace('content="Status for Clorraine v1.3"', 'content="Status for Clorraine v1.2"')
        self.assertNotEqual(html, self.site.pages["status"])
        self.serve("status", html)
        code, out = self.live()
        self.assert_finding(code, out, "stale-version", "/clorraine/status", "v1.2")

    def test_should_require_both_install_commands_on_the_rendered_home_page(self) -> None:
        windows = WINDOWS.replace("{v}", VERSION).replace('"', "&quot;")
        self.serve("", self.site.pages[""].replace(windows, ""))
        code, out = self.live()
        self.assert_finding(code, out, "platform", "/clorraine/", "Windows")

    def test_should_require_both_install_commands_on_the_rendered_quick_start(self) -> None:
        windows = WINDOWS.replace("{v}", VERSION).replace('"', "&quot;")
        self.serve("guide/quick-start", self.site.pages["guide/quick-start"].replace(windows, ""))
        code, out = self.live()
        self.assert_finding(code, out, "platform", "/clorraine/guide/quick-start", "Windows")

    def test_should_report_a_page_that_does_not_answer_200(self) -> None:
        self.serve("project/brief", "gone", status=404)
        code, out = self.live()
        self.assert_finding(code, out, "page", "/clorraine/project/brief", "404")

    # Probes --------------------------------------------------------------

    def test_should_probe_install_urls_found_only_in_code_blocks(self) -> None:
        self.site.set_release(assets=[{"name": name} for name in HASHES if name != "install.ps1"] + [{"name": "SHA256SUMS"}])
        code, out = self.live()
        self.assert_finding(code, out, "link", f"releases/download/v{VERSION}/install.ps1")

    def test_should_report_a_project_link_github_does_not_know(self) -> None:
        del self.site.gh_answers[api("repos/OpenLorraine/clorraine/contents/docs/known-limitations.md?ref=main")]
        code, out = self.live()
        self.assert_finding(code, out, "link", f"{REPO}/blob/main/docs/known-limitations.md")

    def test_should_fail_a_dead_discord_invite(self) -> None:
        self.site.http["https://discord.com/api/v10/invites/7ckPjharmD"] = Response(404, {}, b'{"message": "Unknown Invite"}')
        code, out = self.live()
        self.assert_finding(code, out, "link", DISCORD)

    def test_should_fail_when_the_marketplace_lacks_this_version(self) -> None:
        self.site.http[PACKAGE] = Response(404)
        code, out = self.live()
        self.assert_finding(code, out, "link", "marketplace.visualstudio.com")

    def test_should_not_probe_another_marketplace_item(self) -> None:
        other = MARKETPLACE.replace("openlorraine.clorraine", "someone.else")
        self.serve("status", self.page_with("status", f'<a href="{other}">other</a>'))
        self.live()
        self.assertNotIn(other, self.site.requests)

    # Published release ---------------------------------------------------

    def test_should_check_the_published_release_body(self) -> None:
        self.site.set_release(body=release_body() + "\nPENDING-CHECKSUMS: oops\n")
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "PENDING-")
        changed = dict(HASHES, **{"install.sh": "e" * 64})
        self.site.set_release(body=release_body(changed))
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "install.sh")

    def test_should_require_the_body_to_name_every_asset(self) -> None:
        without = {name: digest for name, digest in HASHES.items() if name != "install.ps1"}
        self.site.set_release(body=release_body(without))
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "install.ps1")

    def test_should_report_a_release_without_checksums_as_a_finding(self) -> None:
        self.site.set_release(assets=[{"name": name} for name in HASHES])
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "SHA256SUMS")

    # Unreachable ---------------------------------------------------------

    def test_should_report_a_probe_that_stays_unreachable(self) -> None:
        attempts = []

        def timeout():
            attempts.append(1)
            raise TimeoutError("timed out")

        self.site.http["https://discord.com/api/v10/invites/7ckPjharmD"] = timeout
        code, out = self.live()
        self.assert_finding(code, out, "unreachable", "discord.com", exit_code=3)
        self.assertEqual(len(attempts), 3, "one try and two retries")

    def test_should_report_findings_before_unreachable_probes(self) -> None:
        self.site.http["https://discord.com/api/v10/invites/7ckPjharmD"] = TimeoutError("timed out")
        self.serve("status", self.page_with("status", "<p>ships with the next release</p>"))
        code, out = self.live()
        self.assertEqual(code, 1, out)
        self.assertIn("[unreachable]", out)
        self.assertIn("[banned]", out)

    def test_should_refuse_a_redirect_to_a_host_outside_the_allowlist(self) -> None:
        self.site.http[PACKAGE] = Response(302, {"Location": "https://evil.example.com/clorraine.vsix"})
        code, out = self.live()
        self.assert_finding(code, out, "unreachable", "evil.example.com", exit_code=3)
        self.assertNotIn("https://evil.example.com/clorraine.vsix", self.site.requests)


if __name__ == "__main__":
    unittest.main()
