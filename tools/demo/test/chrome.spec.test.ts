import { test } from "node:test";
import assert from "node:assert/strict";
import { assertChromeAvailable, resolveChrome } from "../src/util/chrome.ts";

const missing = "/definitely/not-a-chrome-binary";

test("CHROME_BIN overrides the chrome command, and a missing binary fails clearly", async () => {
  const previous = process.env.CHROME_BIN;
  process.env.CHROME_BIN = missing;
  try {
    assert.equal(resolveChrome(), missing);
    await assert.rejects(() => assertChromeAvailable(resolveChrome()), /Chrome/i);
  } finally {
    if (previous === undefined) {
      delete process.env.CHROME_BIN;
    } else {
      process.env.CHROME_BIN = previous;
    }
  }
});

test("resolveChrome defaults to google-chrome when CHROME_BIN is unset", async () => {
  const previous = process.env.CHROME_BIN;
  delete process.env.CHROME_BIN;
  try {
    assert.equal(resolveChrome(), "google-chrome");
  } finally {
    if (previous !== undefined) {
      process.env.CHROME_BIN = previous;
    }
  }
});
