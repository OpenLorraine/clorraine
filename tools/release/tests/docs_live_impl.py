"""Implementation tests for the live docs-check internals."""

from __future__ import annotations

import http.server
import os
import stat
import tempfile
import threading
import unittest
from pathlib import Path

from docs_fixture import Fixture
from docs_live_fixture import LiveSite, Response

import docs_live
from docs_live import Client, Unreachable, page_text

FOREVER = float("inf")


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
        docs_live.clock = __import__("time").monotonic

    def test_should_honour_retry_after_up_to_a_minute(self) -> None:
        answers = [Response(429, {"Retry-After": "120"}), Response(503, {"Retry-After": "5"}), Response(200)]
        client = Client(lambda url, read_body=True: answers.pop(0), FOREVER)
        self.assertEqual(client.get("https://discord.com/x").status, 200)
        self.assertEqual(self.waits, [60, 5])

    def test_should_not_retry_a_404(self) -> None:
        calls = []
        client = Client(lambda url, read_body=True: calls.append(url) or Response(404), FOREVER)
        self.assertEqual(client.get("https://discord.com/x").status, 404)
        self.assertEqual(len(calls), 1)

    def test_should_refuse_plain_http_unknown_hosts_userinfo_and_ports(self) -> None:
        client = Client(lambda url, read_body=True: Response(200), FOREVER)
        for url in ("http://openlorraine.github.io/clorraine/", "https://example.com/",
                    "https://evil.com@marketplace.visualstudio.com/items",
                    "https://marketplace.visualstudio.com:8443/items"):
            with self.subTest(url=url), self.assertRaises(Unreachable):
                client.get(url)

    def test_should_follow_at_most_five_redirects(self) -> None:
        calls = []

        def loop(url, read_body=True):
            calls.append(url)
            return Response(302, {"Location": "https://discord.com/loop"})

        with self.assertRaises(Unreachable) as caught:
            Client(loop, FOREVER).get("https://discord.com/start")
        self.assertIn("redirects", str(caught.exception))
        self.assertEqual(len(calls), 6, "the first request and five redirects")

    def test_should_allow_the_cdn_only_after_a_marketplace_redirect(self) -> None:
        hops = {"https://marketplace.visualstudio.com/p": Response(302, {"Location": "https://a.gallerycdn.vsassets.io/f"}),
                "https://discord.com/p": Response(302, {"Location": "https://a.gallerycdn.vsassets.io/f"}),
                "https://a.gallerycdn.vsassets.io/f": Response(200)}
        client = Client(lambda url, read_body=True: hops[url], FOREVER)
        self.assertEqual(client.get("https://marketplace.visualstudio.com/p").status, 200)
        with self.assertRaises(Unreachable):
            client.get("https://discord.com/p")

    def test_should_retry_a_broken_answer_and_refuse_an_unusable_url(self) -> None:
        import http.client
        answers = [http.client.IncompleteRead(b"x"), Response(200)]

        def flaky(url, read_body=True):
            answer = answers.pop(0)
            if isinstance(answer, Exception):
                raise answer
            return answer

        self.assertEqual(Client(flaky, FOREVER).get("https://discord.com/x").status, 200)
        with self.assertRaises(Unreachable):
            Client(lambda url, read_body=True: Response(200), FOREVER).get("https://[x/")

    def test_should_stop_when_the_run_budget_is_spent(self) -> None:
        now = [0.0]
        docs_live.clock = lambda: now[0]

        def slow(url, read_body=True):
            now[0] += 400
            return Response(302, {"Location": "https://discord.com/next"})

        with self.assertRaises(Unreachable) as caught:
            Client(slow, 600).get("https://discord.com/start")
        self.assertIn("budget", str(caught.exception))


class _Handler(http.server.BaseHTTPRequestHandler):
    seen: list[dict] = []

    def do_GET(self) -> None:  # noqa: N802 - http.server hook
        _Handler.seen.append(dict(self.headers))
        if self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "/elsewhere")
            self.end_headers()
        elif self.path == "/slow":
            self.send_response(200)
            self.end_headers()
            for _ in range(12):
                self.wfile.write(b"x")
                self.wfile.flush()
                __import__("time").sleep(0.25)
        elif self.path == "/big":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"x" * (docs_live.BODY_LIMIT + 10))
        else:
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"hello")

    def log_message(self, *args) -> None:
        pass


class TransportTests(unittest.TestCase):
    """The real transport against a server on 127.0.0.1 (plain HTTP, so the
    policy layer is bypassed on purpose; only the transport is under test)."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        cls.base = f"http://127.0.0.1:{cls.server.server_address[1]}"
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self) -> None:
        _Handler.seen.clear()

    def test_should_return_a_redirect_without_following_it(self) -> None:
        answer = docs_live.http_transport(self.base + "/redirect")
        self.assertEqual(answer.status, 302)
        self.assertEqual(len(_Handler.seen), 1)

    def test_should_refuse_an_oversized_body(self) -> None:
        with self.assertRaises(Unreachable):
            docs_live.http_transport(self.base + "/big")

    def test_should_skip_the_body_when_asked(self) -> None:
        self.assertEqual(docs_live.http_transport(self.base + "/ok", read_body=False).body, b"")
        self.assertEqual(docs_live.http_transport(self.base + "/ok").body, b"hello")

    def test_should_stop_reading_a_slow_answer_at_the_request_limit(self) -> None:
        saved = docs_live.REQUEST_TIMEOUT
        docs_live.REQUEST_TIMEOUT = 1
        try:
            with self.assertRaises(Unreachable) as caught:
                docs_live.http_transport(self.base + "/slow")
            self.assertIn("longer than", str(caught.exception))
        finally:
            docs_live.REQUEST_TIMEOUT = saved

    def test_should_send_the_fixed_user_agent_and_no_credentials(self) -> None:
        docs_live.http_transport(self.base + "/ok")
        headers = {key.lower(): value for key, value in _Handler.seen[0].items()}
        self.assertEqual(headers.get("user-agent"), docs_live.USER_AGENT)
        self.assertNotIn("cookie", headers)
        self.assertNotIn("authorization", headers)


class GhTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fx = Fixture()
        self.site = LiveSite(head=self.fx.head())
        docs_live.sleep = lambda seconds: None

    def tearDown(self) -> None:
        self.fx.cleanup()

    def run_live(self, gh=None) -> tuple[int, str]:
        self.site.deploy(self.fx.head(), "success")
        return self.fx.run("--live", fetcher=self.site.transport, gh=gh or self.site.gh)

    def test_should_treat_a_missing_or_failing_gh_as_unreachable(self) -> None:
        def missing(args):
            raise Unreachable("the GitHub CLI `gh` is not installed")

        code, out = self.run_live(missing)
        self.assertEqual(code, 3, out)
        self.assertIn("not installed", out)
        code, out = self.run_live(lambda args: (1, "", "HTTP 502: Bad Gateway"))
        self.assertEqual(code, 3, out)

    def test_should_stop_calling_gh_when_the_run_budget_is_spent(self) -> None:
        now = [0.0]
        docs_live.clock = lambda: now[0]
        calls = []

        def slow_gh(args):
            calls.append(args)
            now[0] += 700
            return self.site.gh(args)

        try:
            code, out = self.run_live(slow_gh)
        finally:
            docs_live.clock = __import__("time").monotonic
        self.assertEqual(code, 3, out)
        self.assertIn("budget", out)
        self.assertEqual(len(calls), 1, calls)

    def test_should_treat_a_deployment_without_a_numeric_id_as_unreachable(self) -> None:
        import json
        from docs_live_fixture import DEPLOYMENTS, api
        self.site.deploy(self.fx.head(), "success")
        self.site.gh_answers[api(DEPLOYMENTS)] = (0, json.dumps([{"id": None, "sha": self.fx.head()}]), "")
        code, out = self.fx.run("--live", fetcher=self.site.transport, gh=self.site.gh)
        self.assertEqual(code, 3, out)

    def test_should_treat_a_non_json_answer_as_unreachable(self) -> None:
        code, out = self.run_live(lambda args: (0, "not json", ""))
        self.assertEqual(code, 3, out)

    def test_should_pin_gh_to_github_com(self) -> None:
        self.run_live()
        api_calls = [call for call in self.site.gh_calls if call[0] == "api"]
        self.assertTrue(api_calls)
        self.assertTrue(all(call[1:3] == ("--hostname", "github.com") for call in api_calls), api_calls)

    def test_should_not_follow_a_link_with_dot_segments(self) -> None:
        html = self.site.pages["status"].replace(
            "</main>", '<a href="https://github.com/OpenLorraine/clorraine/blob/main/x/../../../user">x</a></main>')
        self.site.http["https://openlorraine.github.io/clorraine/status"] = Response(200, {}, html.encode())
        code, out = self.run_live()
        self.assertEqual(code, 1, out)
        self.assertIn("path segment", out)
        self.assertFalse([call for call in self.site.gh_calls if ".." in " ".join(call)])

    def test_should_probe_a_link_once_however_many_pages_carry_it(self) -> None:
        self.run_live()
        discord = [url for url in self.site.requests if "api/v10/invites" in url]
        self.assertEqual(len(discord), 1)

    def test_should_report_a_stale_rendered_token_once_per_page(self) -> None:
        for path in ("status", "project/brief"):
            html = self.site.pages[path].replace("</main>", "<p>v1.2.0 and again v1.2.0</p></main>")
            self.site.http[f"https://openlorraine.github.io/clorraine/{path}"] = Response(200, {}, html.encode())
        code, out = self.run_live()
        stale = [line for line in out.splitlines() if "[stale-version]" in line]
        self.assertEqual(code, 1, out)
        self.assertEqual(len(stale), 2, out)

    def test_should_strip_gh_host_and_repo_from_the_gh_environment(self) -> None:
        folder = Path(tempfile.mkdtemp(dir=self.fx.root))
        fake = folder / "gh"
        fake.write_text("#!/bin/sh\necho \"host=${GH_HOST:-none} repo=${GH_REPO:-none}\"\n")
        fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
        saved = dict(os.environ)
        try:
            os.environ.update(PATH=f"{folder}{os.pathsep}{saved['PATH']}", GH_HOST="evil.example", GH_REPO="x/y")
            code, out, _ = docs_live.gh_runner(["api", "x"])
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertEqual(code, 0)
        self.assertEqual(out.strip(), "host=none repo=none")


if __name__ == "__main__":
    unittest.main()
