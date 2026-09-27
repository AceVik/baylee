import { describe, expect, test } from "vitest";

import { firstLine, newIssueUrl, parseIssue, REPOSITORY, TEXT_CHARS } from "./github";
import { summary } from "./test-utils";

describe("newIssueUrl", () => {
  test("opens this repository's new-issue page with the title and body encoded", () => {
    const text = "Mana & tapping: 50% of #turns?\nsecond line <b>\"quoted\"</b> → ünïcode";
    const url = new URL(newIssueUrl(summary({ kind: "bug", text }), "https://feedback.example"));
    expect(`${url.origin}${url.pathname}`).toBe(`${REPOSITORY}/issues/new`);
    // Nothing in the text escapes its parameter: only title and body exist.
    expect([...url.searchParams.keys()]).toEqual(["title", "body"]);
    expect(url.searchParams.get("title")).toBe("Bug: Mana & tapping: 50% of #turns?");
    const body = url.searchParams.get("body") ?? "";
    expect(body).toContain("> Mana & tapping: 50% of #turns?\n> second line <b>\"quoted\"</b> → ünïcode");
    expect(body).toContain("- Report: https://feedback.example/r/0199aaaa-0000-7000-8000-000000000001");
    expect(body).toContain("- Build: 0.1.0-beta.1+build.42 (3f9a1c7e21)");
    // Raw characters that would break a query string are escaped in the URL.
    const raw = newIssueUrl(summary({ text }), "https://feedback.example");
    const query = raw.split("?")[1] ?? "";
    expect(query).not.toMatch(/[ \n#"<>→]/);
    expect(query.split("&")).toHaveLength(2);
  });

  test("leaves the pseudonym and the client's details out", () => {
    const url = newIssueUrl(summary({ reporter: "deadbeefcafe" }), "https://x");
    expect(url).not.toContain("deadbeefcafe");
  });

  test("cuts a long text and a long title", () => {
    const long = "word ".repeat(2000);
    const url = new URL(newIssueUrl(summary({ text: long }), "https://x"));
    const body = url.searchParams.get("body") ?? "";
    expect(body.length).toBeLessThan(TEXT_CHARS + 600);
    expect(body).toContain("[…]");
    expect((url.searchParams.get("title") ?? "").length).toBeLessThanOrEqual("Bug: ".length + 80);
  });

  test("names each kind and says how much of the record there is", () => {
    const crash = new URL(newIssueUrl(summary({ kind: "crash", has_record: true, record_complete: false }), "https://x"));
    expect(crash.searchParams.get("title")).toMatch(/^Crash: /);
    expect(crash.searchParams.get("body")).toContain("- Game record: partial");
    const empty = new URL(newIssueUrl(summary({ text: "  \n " }), "https://x"));
    expect(empty.searchParams.get("title")).toBe("Bug: (no text)");
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
