"""Specification tests for the live release docs check.

A clean fixture release (`docs_fixture`) plus a fake deployed site and fake
`gh` answers (`docs_live_fixture`) pass; each case changes one answer.
"""

from __future__ import annotations

import unittest

from docs_fixture import VERSION, Fixture, record
from docs_live_fixture import DISCORD, PACKAGE, HASHES, LiveSite, Response, release_body

import docs_live  # noqa: F401  (imported for the fixture's sleep override)


class LiveSpecTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()
        self.site = LiveSite()

    def tearDown(self) -> None:
        self.fx.cleanup()

    def live(self) -> tuple[int, str]:
        return self.fx.run("--live", fetcher=self.site.transport, gh=self.site.gh)

    def assert_finding(self, code: int, out: str, rule: str, *needles: str, exit_code: int = 1) -> None:
        self.assertEqual(code, exit_code, out)
        lines = [line for line in out.splitlines() if f"[{rule}]" in line]
        self.assertTrue(lines, f"no [{rule}] finding in:\n{out}")
        for needle in needles:
            self.assertTrue(any(needle in line for line in lines), f"{needle!r} not in [{rule}] lines:\n{out}")

    def test_should_pass_a_clean_deployed_release(self) -> None:
        code, out = self.live()
        self.assertEqual(code, 0, out)
        self.assertIn("0 findings, 0 unreachable", out)
        # Only the theme's edit link would be probed through the repository API.
        self.assertNotIn(("api", "repos/OpenLorraine/clorraine"), self.site.gh_calls, "edit links are not probed")

    def test_should_require_the_after_deploy_items_in_the_live_check(self) -> None:
        self.fx.write(f"docs/release-reviews/v{VERSION}.md", record(after=False))
        code, out = self.live()
        self.assert_finding(code, out, "review", "After deploy")

    def test_should_stop_when_the_deployed_site_is_older_than_the_last_site_commit(self) -> None:
        self.site.lastmod = "2026-11-27T10:00:00.000Z"
        code, out = self.live()
        self.assert_finding(code, out, "deploy-not-current", "sitemap", exit_code=4)
        self.assertEqual([url for url in self.site.requests if not url.endswith("sitemap.xml")], [],
                         "no page is fetched before the deploy is current")

    def test_should_check_rendered_versions_against_the_page_source_allowlist(self) -> None:
        status = self.site.pages["status"].replace("</main>", "<p>v1.2.0 notes</p></main>")
        self.site.http["https://openlorraine.github.io/clorraine/status"] = Response(200, {}, status.encode())
        self.fx.manifest["allowlist"].append({
            "file": "site/.vitepress/theme/Layout.vue", "phrase": "v0.9.0 legacy",
            "reason": "the theme keeps the first preview's label", "reviewed_for": VERSION,
        })
        self.fx.write("site/.vitepress/theme/Layout.vue",
                      f"<template><span>v{VERSION} for Linux and Windows</span><i>v0.9.0 legacy</i></template>\n")
        self.fx.save_manifest()
        brief = self.site.pages["project/brief"].replace("</main>", "<i>v0.9.0 legacy</i></main>")
        self.site.http["https://openlorraine.github.io/clorraine/project/brief"] = Response(200, {}, brief.encode())
        code, out = self.live()
        self.assert_finding(code, out, "stale-version", "/clorraine/status", "v1.2.0")
        self.assertNotIn("v0.9.0", "\n".join(line for line in out.splitlines() if "[stale-version]" in line), out)

    def test_should_probe_install_urls_found_only_in_code_blocks(self) -> None:
        self.site.set_release(assets=[{"name": name} for name in HASHES if name != "install.ps1"] + [{"name": "SHA256SUMS"}])
        code, out = self.live()
        self.assert_finding(code, out, "link", f"releases/download/v{VERSION}/install.ps1")

    def test_should_fail_a_dead_discord_invite(self) -> None:
        self.site.http["https://discord.com/api/v10/invites/7ckPjharmD"] = Response(404, {}, b'{"message": "Unknown Invite"}')
        code, out = self.live()
        self.assert_finding(code, out, "link", DISCORD)

    def test_should_fail_when_the_marketplace_lacks_this_version(self) -> None:
        self.site.http[PACKAGE] = Response(404)
        code, out = self.live()
        self.assert_finding(code, out, "link", "marketplace.visualstudio.com")

    def test_should_check_the_published_release_body(self) -> None:
        self.site.set_release(body=release_body() + "\nPENDING-CHECKSUMS: oops\n")
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "PENDING-")
        changed = dict(HASHES, **{"install.sh": "e" * 64})
        self.site.set_release(body=release_body(changed))
        code, out = self.live()
        self.assert_finding(code, out, "release-body", "install.sh")

    def test_should_report_a_probe_that_stays_unreachable(self) -> None:
        attempts = []

        def timeout():
            attempts.append(1)
            raise TimeoutError("timed out")

        self.site.http["https://discord.com/api/v10/invites/7ckPjharmD"] = timeout
        code, out = self.live()
        self.assert_finding(code, out, "unreachable", "discord.com", exit_code=3)
        self.assertEqual(len(attempts), 3, "one try and two retries")

    def test_should_refuse_a_redirect_to_a_host_outside_the_allowlist(self) -> None:
        self.site.http[PACKAGE] = Response(302, {"Location": "https://evil.example.com/clorraine.vsix"})
        code, out = self.live()
        self.assert_finding(code, out, "unreachable", "evil.example.com", exit_code=3)
        self.assertNotIn("https://evil.example.com/clorraine.vsix", self.site.requests)


if __name__ == "__main__":
    unittest.main()
