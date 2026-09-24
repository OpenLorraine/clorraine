import { test } from "node:test";
import assert from "node:assert/strict";
import { tokenize } from "../src/util/shiki.ts";

const fixture = [
  "// a comment",
  "module demo.Hello;",
  "import { Screen } from std.amiga.intuition;",
  "func main(): i32 {",
  '    let who: str = "Amiga";',
  "    let n: u8 = 0x2a;",
  '    println($"Hi, {who}!");',
  "    return n;",
  "}",
].join("\n");

test("every grammar category is scoped", async () => {
  const scopes = (await tokenize(fixture)).flatMap((token) => token.scopes);
  const has = (needle: string): boolean => scopes.some((scope) => scope.includes(needle));

  assert.ok(has("comment.line"), "comment");
  assert.ok(has("storage.modifier.clorraine"), "module/func/let keywords");
  assert.ok(has("keyword.control.clorraine"), "return");
  assert.ok(has("storage.type.clorraine"), "i32 / u8 / str");
  assert.ok(has("support.type.clorraine"), "Screen handle");
  assert.ok(has("string.quoted.double"), "string literal");
  assert.ok(has("constant.numeric"), "number literal");
  assert.ok(has("entity.name.function"), "call name");
});

test("string templates mark the interpolation", async () => {
  const scopes = (await tokenize('println($"Hi")')).flatMap((token) => token.scopes);
  assert.ok(scopes.some((scope) => scope.includes("punctuation.definition.template")));
});
