import { createHighlighterCore } from "shiki/core";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";
import type { HighlighterCore, LanguageRegistration, ThemeRegistration } from "shiki/core";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const toolDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const grammarPath = join(toolDir, "grammars", "clorraine.tmLanguage.json");
const themePath = join(toolDir, "assets", "theme.json");

/** The Shiki language id for Clorraine. */
export const LANG = "clorraine";
/** The Shiki theme id for the demo palette. */
export const THEME = "clorraine";

/** One highlighted run of source text and the TextMate scopes it belongs to. */
export interface Token {
  content: string;
  scopes: string[];
}

/** A run of source text with the resolved theme color. */
export interface StyledToken {
  content: string;
  color: string;
}

interface ExplainedToken {
  content: string;
  explanation?: Array<{ scopes: Array<{ scopeName: string }> }>;
}

let highlighterPromise: Promise<HighlighterCore> | undefined;

async function getHighlighter(): Promise<HighlighterCore> {
  if (highlighterPromise === undefined) {
    highlighterPromise = (async () => {
      const grammar = JSON.parse(await readFile(grammarPath, "utf8")) as LanguageRegistration;
      const theme = JSON.parse(await readFile(themePath, "utf8")) as ThemeRegistration;
      return createHighlighterCore({
        themes: [theme],
        langs: [{ ...grammar, name: LANG }],
        engine: createOnigurumaEngine(() => import("shiki/wasm")),
      });
    })();
  }
  return highlighterPromise;
}

/**
 * Tokenizes Clorraine source and returns each run with its TextMate scopes. Used by the
 * grammar tests to check that every category is highlighted.
 */
export async function tokenize(code: string): Promise<Token[]> {
  const highlighter = await getHighlighter();
  const result = highlighter.codeToTokens(code, { lang: LANG, theme: THEME, includeExplanation: true });
  const tokens = result.tokens.flat() as unknown as ExplainedToken[];
  return tokens.map((token) => ({
    content: token.content,
    scopes: (token.explanation ?? []).flatMap((entry) => entry.scopes.map((scope) => scope.scopeName)),
  }));
}

/**
 * Tokenizes Clorraine source line by line, resolving each run's color from the demo
 * theme. Used by the code renderer to build line-numbered, scrollable HTML.
 *
 * @returns one array of styled runs per source line.
 */
export async function tokenizeLines(code: string): Promise<StyledToken[][]> {
  const highlighter = await getHighlighter();
  const result = highlighter.codeToTokens(code, { lang: LANG, theme: THEME });
  return result.tokens.map((line) =>
    line.map((token) => ({ content: token.content, color: token.color ?? "#e6e6ee" })),
  );
}
