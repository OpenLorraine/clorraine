import { test } from "node:test";
import assert from "node:assert/strict";
import { error, info, messageOf, warn } from "../src/util/log.ts";

test("messageOf reads an Error message and stringifies other values", () => {
  assert.equal(messageOf(new Error("boom")), "boom");
  assert.equal(messageOf("plain"), "plain");
  assert.equal(messageOf(42), "42");
});

test("log helpers write to stderr without throwing", () => {
  assert.doesNotThrow(() => {
    info("info");
    warn("warning");
    error("error");
  });
});
