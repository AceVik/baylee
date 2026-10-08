import { describe, expect, test } from "vitest";

import { firstLine, newIssueUrl, parseIssue, REPOSITORY, SUMMARY_CHARS } from "./github";

const BUILD = "0.1.0-beta.1+build.42 (3f9a1c7e21)";

function issue(summary: string, category: "bug" | "crash" = "bug"): URL {
  const url = newIssueUrl({ summary, category, build: BUILD });
  if (url === null) throw new Error("no URL");
  return new URL(url);
}

describe("newIssueUrl", () => {
  test("opens this repository's new-issue page with the admin's summary encoded", () => {
    const text = "Mana & tapping: 50% of #turns?\nsecond line <b>\"quoted\"</b> → ünïcode";
    const url = issue(text);
    expect(`${url.origin}${url.pathname}`).toBe(`${REPOSITORY}/issues/new`);
    // Nothing in the text escapes its parameter: only title and body exist.
    expect([...url.searchParams.keys()]).toEqual(["title", "body"]);
    expect(url.searchParams.get("title")).toBe("Bug: Mana & tapping: 50% of #turns?");
    expect(url.searchParams.get("body")).toBe(`${text}\n\n- Category: bug\n- Build: ${BUILD}`);
    // Raw characters that would break a query string are escaped in the URL.
    const raw = newIssueUrl({ summary: text, category: "bug", build: BUILD }) ?? "";
    const query = raw.split("?")[1] ?? "";
    expect(query).not.toMatch(/[ \n#"<>→]/);
    expect(query.split("&")).toHaveLength(2);
  });

  test("files nothing before the admin has written a summary", () => {
    expect(newIssueUrl({ summary: "", category: "bug", build: BUILD })).toBeNull();
    expect(newIssueUrl({ summary: "  \n ", category: "crash", build: BUILD })).toBeNull();
  });

  test("carries no link back, no service address and no report id", () => {
    const url = issue("The stack resolves twice.").toString();
    expect(url).not.toMatch(/feedback|\/r\/|report/i);
    expect(url).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-/);
  });

  test("cuts a long summary and a long title", () => {
    const url = issue("word ".repeat(2000));
    const body = url.searchParams.get("body") ?? "";
    expect(body.length).toBeLessThan(SUMMARY_CHARS + 200);
    expect(body).toContain("[…]");
    expect((url.searchParams.get("title") ?? "").length).toBeLessThanOrEqual("Bug: ".length + 80);
  });

  test("names the category", () => {
    const crash = issue("Panics when the deck list opens.", "crash");
    expect(crash.searchParams.get("title")).toBe("Crash: Panics when the deck list opens.");
    expect(crash.searchParams.get("body")).toContain("- Category: crash");
  });
});

describe("parseIssue", () => {
  test.each([
    ["311", 311],
    ["#311", 311],
    [" 42 ", 42],
    ["https://github.com/AceVik/baylee/issues/311", 311],
    ["https://github.com/AceVik/baylee/issues/311/", 311],
    ["https://github.com/AceVik/baylee/issues/311#issuecomment-1", 311],
    ["https://github.com/AceVik/baylee/pull/12", 12],
  ])("%s is issue %d", (input, expected) => {
    expect(parseIssue(input)).toBe(expected);
  });

  test.each([
    "",
    "0",
    "#0",
    "-3",
    "abc",
    "3.5",
    "https://github.com/someone/else/issues/311",
    "https://evil.example/AceVik/baylee/issues/311",
    "javascript:alert(1)//311",
    "9999999999",
  ])("%j is no issue", (input) => {
    expect(parseIssue(input)).toBeNull();
  });
});

test("firstLine skips blank lines and cuts with an ellipsis", () => {
  expect(firstLine("\n\n  hello there \nnext", 80)).toBe("hello there");
  expect(firstLine("abcdefghij", 5)).toBe("abcd…");
  expect(firstLine("", 5)).toBe("");
});
