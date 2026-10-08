// The UI in a real browser against a real baylee-feedback (harness.ts):
// sign in, filter, open a report, triage it, link an issue, sign out, and a
// report handed in through the intake route turns up.

import { expect, test, type Page } from "@playwright/test";

import { ADMIN, BASE_URL, handIn, PASSWORD, report } from "./harness.ts";

/** Fails the test on any script error or policy violation in the page. */
function watch(page: Page): string[] {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
  page.on("console", (m) => {
    // A 401 while asking who is signed in is the sign-in form's cue.
    if (m.type() === "error" && !m.text().includes("401")) problems.push(`console: ${m.text()}`);
  });
  return problems;
}

async function signIn(page: Page) {
  await page.goto("/");
  await page.getByLabel("Name").fill(ADMIN);
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("table")).toBeVisible();
}

test.describe.configure({ mode: "serial" });

test("a wrong password is refused and the right one lets the admin in", async ({ page }) => {
  const problems = watch(page);
  await page.goto("/");
  await page.getByLabel("Name").fill(ADMIN);
  await page.getByLabel("Password").fill("not the password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("alert")).toHaveText("Wrong name or password.");
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("row")).toHaveCount(5);
  const cookies = await page.context().cookies();
  const session = cookies.find((c) => c.name === "__Host-baylee-feedback");
  expect(session?.httpOnly).toBe(true);
  expect(session?.secure).toBe(true);
  expect(session?.sameSite).toBe("Strict");
  expect(await page.evaluate(() => document.cookie)).toBe("");
  expect(problems).toEqual([]);
});

test("the list filters by kind, text, pseudonym and record", async ({ page }) => {
  const problems = watch(page);
  await signIn(page);
  await page.getByLabel("Kind").selectOption("crash");
  await expect(page.getByRole("row")).toHaveCount(2);
  await expect(page).toHaveURL(/\?kind=crash$/);
  await page.getByRole("button", { name: "Clear" }).click();
  await expect(page.getByRole("row")).toHaveCount(5);

  await page.getByLabel("Text").fill("deck list");
  await page.getByLabel("Text").press("Enter");
  await expect(page.getByRole("row")).toHaveCount(3);

  await page.getByRole("button", { name: "Clear" }).click();
  await page.getByRole("button", { name: /^77aa01bc22/ }).first().click();
  await expect(page.getByRole("row")).toHaveCount(3);
  await expect(page).toHaveURL(/reporter=77aa01bc22dd33ee/);

  await page.getByRole("button", { name: "Clear" }).click();
  await page.getByLabel("Game record").selectOption("true");
  await expect(page.getByRole("row")).toHaveCount(2);
  // A filtered view is a link: reloading keeps it.
  await page.reload();
  await expect(page.getByRole("row")).toHaveCount(2);
  await expect(page.getByLabel("Game record")).toHaveValue("true");
  expect(problems).toEqual([]);
});

test("a report opens from the keyboard and shows its dump", async ({ page }) => {
  const problems = watch(page);
  await signIn(page);
  // Newest first: the music, the sorting, the crash, the Swamp. Down three,
  // up one, down one lands on the Swamp only if both keys move.
  await expect(page.getByRole("row")).toHaveCount(5);
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  for (const key of ["j", "j", "j", "k", "j"]) await page.keyboard.press(key);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "What the player wrote" })).toBeVisible();
  await expect(page.getByText("The Swamp untapped by itself during my upkeep.")).toBeVisible();

  const shot = page.getByRole("img", { name: "The player's window when the report was written" });
  await expect(shot).toBeVisible();
  expect(await shot.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(640);
  await expect(page.getByRole("region", { name: "The player's game log" }).getByText("Player A cast Lightning Bolt targeting you.")).toBeVisible();
  await expect(page.getByRole("region", { name: "System" }).getByText("Apple M1 Max")).toBeVisible();
  await expect(page.getByRole("region", { name: "Game record" }).getByText("22 B, complete")).toBeVisible();

  const download = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download" }).click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/\.jsonl\.gz$/);

  await page.keyboard.press("Escape");
  await expect(page.getByRole("table")).toBeVisible();
  await expect(page.getByRole("row")).toHaveCount(5);
  expect(problems).toEqual([]);
});

test("status, issue link and deletion stick, and are in the history", async ({ page }) => {
  const problems = watch(page);
  await signIn(page);
  await page.getByRole("link", { name: "Let me sort the deck list by colour." }).click();

  await page.getByLabel("Status").selectOption("triaged");
  await expect(page.getByText("Status: Triaged.")).toBeVisible();

  await page.getByLabel("Issue number or URL").fill("https://github.com/AceVik/baylee/issues/311");
  await page.getByRole("button", { name: "Link" }).click();
  const issue = page.getByRole("link", { name: "#311" });
  await expect(issue).toHaveAttribute("href", "https://github.com/AceVik/baylee/issues/311");

  // A new issue carries nothing of the report (owner, 08.10.2026): before
  // the admin writes a summary there is no link at all, and after it the
  // link holds their words, the category and the build, and no player text,
  // pseudonym, report id or address of this service.
  await expect(page.getByLabel("Summary for a new issue")).toHaveValue("");
  await expect(page.getByRole("button", { name: "Open a new issue on GitHub" })).toBeDisabled();
  await page.getByLabel("Summary for a new issue").fill("The deck list has no sort order to choose.");
  const opener = page.getByRole("link", { name: "Open a new issue on GitHub" });
  const raw = (await opener.getAttribute("href")) ?? "";
  const href = new URL(raw);
  expect(href.origin + href.pathname).toBe("https://github.com/AceVik/baylee/issues/new");
  expect(href.searchParams.get("title")).toBe("Improvement: The deck list has no sort order to choose.");
  expect(href.searchParams.get("body")).toContain("- Build: 0.1.0-beta.1+build.42 (3f9a1c7e21)");
  const reportId = page.url().split("/r/")[1] ?? "no id";
  // Read as GitHub reads it: in a query string `+` is a space, which
  // `decodeURIComponent` leaves standing, so a phrase would slip past it.
  const filed = `${raw}\n${href.searchParams.get("title") ?? ""}\n${href.searchParams.get("body") ?? ""}`;
  for (const leak of ["sort the deck list by colour", "77aa01bc22dd33ee", reportId, BASE_URL, "localhost", "/r/"]) {
    expect(filed, leak).not.toContain(leak);
  }

  await page.reload();
  await expect(page.getByLabel("Status")).toHaveValue("triaged");
  await expect(page.getByRole("link", { name: "#311" })).toBeVisible();
  await expect(page.getByText(`${ADMIN}: status triaged`)).toBeVisible();
  await expect(page.getByText(`${ADMIN}: issue #311`)).toBeVisible();

  await page.goto("/?status=triaged");
  await expect(page.getByRole("row")).toHaveCount(2);

  await page.goto("/");
  await page.getByRole("link", { name: "Lovely music in the lobby." }).click();
  await expect(page.getByText("The player did not include a screenshot.")).toBeVisible();
  await page.getByRole("button", { name: "Delete report…" }).click();
  await page.getByRole("button", { name: "Delete for good" }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(page.getByText("Lovely music in the lobby.")).toHaveCount(0);
  expect(problems).toEqual([]);
});

test("a report handed in through the intake route turns up", async ({ page }) => {
  await signIn(page);
  const text = `Handed in during the e2e run at ${Date.now()}`;
  const id = await handIn(report("other", text));
  await page.reload();
  await expect(page.getByRole("link", { name: text })).toBeVisible();
  await page.goto(`/r/${id}`);
  await expect(page.getByText(text)).toBeVisible();
});

test("signing out ends the session for good", async ({ page }) => {
  await signIn(page);
  const cookie = (await page.context().cookies()).find((c) => c.name === "__Host-baylee-feedback");
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
  // The old cookie, put back by hand, opens nothing: the server forgot it.
  const answer = await page.request.get("/ui/api/me", {
    headers: { Cookie: `${cookie?.name ?? ""}=${cookie?.value ?? ""}` },
  });
  expect(answer.status()).toBe(401);
});

test("the page carries its policy and a request from another origin is refused", async ({ page }) => {
  const response = await page.goto("/");
  const headers = response?.headers() ?? {};
  expect(headers["content-security-policy"]).toContain("script-src 'self'");
  expect(headers["x-frame-options"]).toBe("DENY");
  await signIn(page);
  const cookie = (await page.context().cookies()).find((c) => c.name === "__Host-baylee-feedback");
  const answer = await page.request.patch("/ui/api/reports/00000000-0000-7000-8000-000000000000", {
    headers: {
      Cookie: `${cookie?.name ?? ""}=${cookie?.value ?? ""}`,
      Origin: "https://evil.example",
      "X-Baylee-CSRF": "1",
    },
    data: { status: "resolved" },
  });
  expect(answer.status()).toBe(403);
});

test("screenshots for the record", async ({ page, browser }) => {
  const dir = process.env["SCREENSHOTS_DIR"];
  test.skip(!dir, "SCREENSHOTS_DIR is not set");
  for (const colorScheme of ["light", "dark"] as const) {
    for (const [name, viewport] of [
      ["desktop", { width: 1400, height: 900 }],
      ["phone", { width: 390, height: 844 }],
    ] as const) {
      const context = await browser.newContext({ colorScheme, viewport });
      const tab = await context.newPage();
      await signIn(tab);
      await tab.screenshot({ path: `${dir}/list-${name}-${colorScheme}.png`, fullPage: true });
      await tab.getByRole("link", { name: "The Swamp untapped by itself during my upkeep." }).click();
      await expect(tab.getByRole("img", { name: /window/ })).toBeVisible();
      await tab.screenshot({ path: `${dir}/detail-${name}-${colorScheme}.png`, fullPage: true });
      await context.close();
    }
  }
  expect(page).toBeTruthy();
});
