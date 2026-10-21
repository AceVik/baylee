import { expect, test } from "vitest";

import { releasesUrl } from "./releases";

test("a GitHub repository's releases page, and nothing for another address", () => {
  expect(releasesUrl("https://github.com/AceVik/baylee")).toBe("https://github.com/AceVik/baylee/releases");
  expect(releasesUrl("https://github.com/AceVik/baylee/")).toBe("https://github.com/AceVik/baylee/releases");
  for (const other of ["", "x", "http://github.com/a/b", "https://gitlab.com/a/b", "https://github.com/a", "https://github.com/a/b/tree/main"]) {
    expect(releasesUrl(other)).toBeNull();
  }
});
