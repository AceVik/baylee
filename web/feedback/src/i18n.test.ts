import { FluentBundle, FluentResource, type Message } from "@fluent/bundle";
import { describe, expect, test } from "vitest";
import { languages, localeOf, t } from "./i18n";
import { KEYS } from "./i18n-keys";

// Every language's file, found on disk rather than through `languages`, so a
// file nobody wired in is noticed too.
const files = import.meta.glob<string>("../locales/*/feedback.ftl", { query: "?raw", import: "default", eager: true });
const sources = new Map(
  Object.entries(files).map(([path, source]) => [path.split("/").at(-2) ?? path, source] as const),
);

type Pattern = NonNullable<Message["value"]>;
type Expression = Exclude<Exclude<Pattern, string>[number], string>;

/** The arguments a pattern reads, in selectors and variants too. */
function variables(pattern: Pattern | null, into: Set<string> = new Set()): Set<string> {
  if (pattern === null || typeof pattern === "string") return into;
  for (const element of pattern) if (typeof element !== "string") expression(element, into);
  return into;
}

function expression(e: Expression, into: Set<string>): void {
  switch (e.type) {
    case "var":
      into.add(e.name);
      break;
    case "select":
      expression(e.selector, into);
      for (const variant of e.variants) variables(variant.value, into);
      break;
    case "func":
    case "term":
      for (const arg of e.args) if (arg.type !== "narg") expression(arg, into);
      break;
    default:
      break;
  }
}

/** Every plural category a pattern's selectors name. */
function categories(pattern: Pattern | null, into: Set<string> = new Set()): Set<string> {
  if (pattern === null || typeof pattern === "string") return into;
  for (const element of pattern) {
    if (typeof element === "string" || element.type !== "select") continue;
    for (const variant of element.variants) {
      if (variant.key.type === "str") into.add(variant.key.value);
      categories(variant.value, into);
    }
  }
  return into;
}

/** A file's messages by id, after checking nothing in it was skipped. */
function parse(lang: string): Map<string, Message> {
  const source = sources.get(lang);
  if (source === undefined) throw new Error(`no locales/${lang}/feedback.ftl`);
  // The runtime parser drops a message it cannot read without a word, and
  // ignores a line that starts no message, so both are counted here: every
  // line is blank, a comment, a message's first line or an indented one.
  const stray = source.split("\n").filter((line) => !/^(|#.*|[a-zA-Z][\w-]* *=.*| .*)$/.test(line));
  expect(stray, `lines in ${lang} that start no message, comment or continuation`).toEqual([]);
  const starts = [...source.matchAll(/^[a-zA-Z][\w-]* *=/gm)].map((m) => m[0].replace(/ *=$/, ""));
  const body = new FluentResource(source).body;
  expect(
    body.map((m) => m.id),
    `messages in ${lang} the parser could not read`,
  ).toEqual(starts);
  const byId = new Map(body.map((m) => [m.id, m] as const));
  expect(byId.size, `an id written twice in ${lang}`).toBe(body.length);
  return byId;
}

const english = parse("en");

describe("the language files", () => {
  test("each language has a file, and each file is a language", () => {
    expect([...sources.keys()].sort()).toEqual([...languages].sort());
  });

  test("the call sites' keys are the English ids", () => {
    expect(KEYS.map((key) => key.replaceAll(".", "-"))).toEqual([...english.keys()]);
    expect(english.size).toBeGreaterThan(800);
  });

  test.each(languages)("%s has every English message, no other, each with a value", (lang) => {
    const messages = parse(lang);
    const ids = [...messages.keys()];
    expect(
      [...english.keys()].filter((id) => !messages.has(id)),
      "missing",
    ).toEqual([]);
    expect(
      ids.filter((id) => !english.has(id)),
      "not in English",
    ).toEqual([]);
    expect(ids.filter((id) => messages.get(id)?.value === null)).toEqual([]);
  });

  test.each(languages)("%s reads the same arguments as English in each message", (lang) => {
    const messages = parse(lang);
    const differing = [...english].flatMap(([id, en]) => {
      const want = [...variables(en.value)].sort();
      const got = [...variables(messages.get(id)?.value ?? null)].sort();
      return want.join() === got.join() ? [] : [`${id} reads [${got.join(", ")}], English [${want.join(", ")}]`];
    });
    expect(differing).toEqual([]);
  });

  test.each(languages)("%s names only plural forms the language has", (lang) => {
    const own = new Set<string>(new Intl.PluralRules(localeOf(lang)).resolvedOptions().pluralCategories);
    const unknown = [...parse(lang)].flatMap(([id, m]) => [...categories(m.value)].filter((c) => !own.has(c)).map((c) => `${id}: ${c}`));
    expect(unknown).toEqual([]);
  });

  test.each(languages)("%s formats every message without an error", (lang) => {
    const bundle = new FluentBundle(localeOf(lang), { useIsolating: false });
    bundle.addResource(new FluentResource(sources.get(lang) ?? ""));
    const failing = [...english].flatMap(([id, en]) => {
      const pattern = bundle.getMessage(id)?.value ?? null;
      if (pattern === null) return [`${id}: no value`];
      const errors: Error[] = [];
      for (const value of [1, 3, "x"]) {
        bundle.formatPattern(pattern, Object.fromEntries([...variables(en.value)].map((name) => [name, value])), errors);
      }
      return errors.map((e) => `${id}: ${e.message}`);
    });
    expect(failing).toEqual([]);
  });
});

describe("t", () => {
  test("fills arguments, numbers as the language writes them", () => {
    expect(t("en", "contact.summary", { open: 3 })).toBe("3 open · from the landing page's form");
    expect(t("en", "acc.count", { from: 1, to: 50, total: 1234 })).toBe("1–50 of 1,234");
    expect(t("de", "acc.count", { from: 1, to: 50, total: 1234 })).toBe("1–50 von 1.234");
  });

  test("a count picks the plural form, in each language by its own rules", () => {
    expect(t("en", "rep.exported", { n: 1 })).toBe("1 report exported.");
    expect(t("en", "rep.exported", { n: 0 })).toBe("0 reports exported.");
    expect(t("en", "rep.exported", { n: 3 })).toBe("3 reports exported.");
    expect(t("en", "llm.summary", { hosts: 1, profiles: 2, games: 1 })).toBe("1 seat agent · 2 profiles · 1 game running");
    expect(t("en", "kpi.gamesSub", { waiting: 1 })).toBe("1 table waiting");
  });

  test("puts no isolation marks around an argument", () => {
    expect(t("de", "mod.banTitle", { who: "Ada" })).not.toMatch(/[⁨⁩]/);
  });

  test("a key no file has reads as itself", () => {
    // Every language has every message (above), so the English fallback is
    // there for a file in the making; a key unknown to English is shown as is.
    const unknown = "nav.nowhere" as (typeof KEYS)[number];
    expect(t("de", unknown)).toBe("nav.nowhere");
  });
});
