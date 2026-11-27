"""Implementation tests for the live docs-check internals."""

from __future__ import annotations

import unittest

from docs_fixture import Fixture
from docs_live_fixture import LiveSite, Response

import docs_live
from docs_live import Client, Unreachable, page_text


class PageTextTests(unittest.TestCase):
    def test_should_join_inline_elements_and_split_blocks(self) -> None:
        text, _ = page_text("<p>v<span>1.3</span>.0 is <b>out</b></p><p>next</p><td>v1.3.0</td><td>12</td>")
        self.assertIn("v1.3.0 is out next", text)
        self.assertIn("v1.3.0 12", text)

    def test_should_skip_scripts_styles_templates_and_the_generator(self) -> None:
        html = ('<head><meta name="generator" content="VitePress v1.6.4"><title>T v1.3</title>'
                '<meta name="description" content="About v1.3"><script>v0.1.0</script><style>v0.2.0</style></head>'
                "<body><template>v0.3.0</template><p>body</p></body>")
        text, _ = page_text(html)
        self.assertNotIn("v1.6.4", text)
        self.assertNotIn("v0.", text)
        self.assertIn("T v1.3", text)
        self.assertIn("About v1.3", text)

    def test_should_collect_hrefs_and_decode_entities(self) -> None:
        text, hrefs = page_text('<a href="https://x/a?b=1&amp;c=2">a &amp;&amp; b</a>')
        self.assertEqual(hrefs, ["https://x/a?b=1&c=2"])
        self.assertIn("a && b", text)


class ClientTests(unittest.TestCase):
    def setUp(self) -> None:
        self.waits: list[float] = []
        docs_live.sleep = self.waits.append

    def tearDown(self) -> None:
        docs_live.sleep = lambda seconds: None

    def test_should_honour_retry_after_up_to_a_minute(self) -> None:
        answers = [Response(429, {"Retry-After": "120"}), Response(503, {"Retry-After": "5"}), Response(200)]
        client = Client(lambda url, read_body=True: answers.pop(0))
        self.assertEqual(client.get("https://discord.com/x").status, 200)
        self.assertEqual(self.waits, [60, 5])

    def test_should_not_retry_a_404(self) -> None:
        calls = []
        client = Client(lambda url, read_body=True: calls.append(url) or Response(404))
        self.assertEqual(client.get("https://discord.com/x").status, 404)
        self.assertEqual(len(calls), 1)

    def test_should_refuse_plain_http_and_unknown_hosts(self) -> None:
        client = Client(lambda url, read_body=True: Response(200))
        with self.assertRaises(Unreachable):
            client.get("http://openlorraine.github.io/clorraine/")
        with self.assertRaises(Unreachable):
            client.get("https://example.com/")

    def test_should_stop_after_five_redirects(self) -> None:
        client = Client(lambda url, read_body=True: Response(302, {"Location": "https://discord.com/loop"}))
        with self.assertRaises(Unreachable) as caught:
            client.get("https://discord.com/start")
        self.assertIn("redirects", str(caught.exception))

    def test_should_allow_the_marketplace_cdn(self) -> None:
        hops = {"https://marketplace.visualstudio.com/p": Response(302, {"Location": "https://a.gallerycdn.vsassets.io/f"}),
                "https://a.gallerycdn.vsassets.io/f": Response(200)}
        client = Client(lambda url, read_body=True: hops[url])
        self.assertEqual(client.get("https://marketplace.visualstudio.com/p").status, 200)


class GhAndDedupTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()
        self.site = LiveSite()
        docs_live.sleep = lambda seconds: None

    def tearDown(self) -> None:
        self.fx.cleanup()

    def test_should_treat_a_missing_or_failing_gh_as_unreachable(self) -> None:
        def missing(args):
            raise Unreachable("the GitHub CLI `gh` is not installed")

        code, out = self.fx.run("--live", fetcher=self.site.transport, gh=missing)
        self.assertEqual(code, 3, out)
        self.assertIn("not installed", out)
        code, out = self.fx.run("--live", fetcher=self.site.transport,
                                gh=lambda args: (1, "", "HTTP 502: Bad Gateway"))
        self.assertEqual(code, 3, out)

    def test_should_probe_a_link_once_however_many_pages_carry_it(self) -> None:
        self.fx.run("--live", fetcher=self.site.transport, gh=self.site.gh)
        discord = [url for url in self.site.requests if "api/v10/invites" in url]
        self.assertEqual(len(discord), 1)

    def test_should_report_a_stale_rendered_token_once_per_page(self) -> None:
        for path in ("status", "project/brief"):
            html = self.site.pages[path].replace("</main>", "<p>v1.2.0 and again v1.2.0</p></main>")
            self.site.http[f"https://openlorraine.github.io/clorraine/{path}"] = Response(200, {}, html.encode())
        code, out = self.fx.run("--live", fetcher=self.site.transport, gh=self.site.gh)
        stale = [line for line in out.splitlines() if "[stale-version]" in line]
        self.assertEqual(code, 1, out)
        self.assertEqual(len(stale), 2, out)


if __name__ == "__main__":
    unittest.main()
