import { test } from "node:test";
import assert from "node:assert/strict";
import { tokenize } from "../src/util/shiki.ts";

const fixture = [
  "// a comment",
  "module demo.Hello;",
  "import { Screen as S } from std.amiga.intuition;",
  "handle File freedBy closeFile;",
  "func main(): i32 {",
  '    let who: str = "Amiga";',
  "    let n: u8 = 0x2a;",
  "    let c: u8 = 'x';",
  "    let size: usize = sizeof(i32);",
  "    do { n = n - 1; } while (n > 0);",
  "    for (x in xs) { }",
  "    let f: func(i32): i32 = (x) => x;",
  '    let e: [u8; 4] = embed("x");',
  '    println($"Hi, {who}!");',
  "    return n;",
  "}",
].join("\n");

test("every grammar category is scoped", async () => {
  const scopes = (await tokenize(fixture)).flatMap((token) => token.scopes);
  const has = (needle: string): boolean => scopes.some((scope) => scope.includes(needle));

  assert.ok(has("comment.line"), "comment");
  assert.ok(has("entity.name.type.module"), "module name");
  assert.ok(has("storage.modifier.clorraine"), "module/func/let keywords");
  assert.ok(has("keyword.control.clorraine"), "return");
  assert.ok(has("storage.type.clorraine"), "i32 / u8 / str");
  assert.ok(has("support.type.clorraine"), "Screen handle");
  assert.ok(has("string.quoted.double"), "string literal");
  assert.ok(has("constant.character.clorraine"), "char literal");
  assert.ok(has("constant.numeric"), "number literal");
  assert.ok(has("entity.name.function"), "call name");
  assert.ok(has("support.function.builtin"), "sizeof / embed");
  assert.ok(has("keyword.operator.arrow"), "arrow operator");
});

test("only template strings mark interpolation", async () => {
  const templateScopes = (await tokenize('println($"Hi")')).flatMap((token) => token.scopes);
  assert.ok(templateScopes.some((scope) => scope.includes("punctuation.definition.template")));

  const plainScopes = (await tokenize('let s: str = "a{b}c";')).flatMap((token) => token.scopes);
  assert.ok(
    !plainScopes.some((scope) => scope.includes("punctuation.section.interpolation")),
    "plain strings must not mark braces as interpolation",
  );
});
