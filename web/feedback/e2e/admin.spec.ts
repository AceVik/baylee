// The admin console in a real browser, through a real baylee-feedback, to
// a real baylee-gateway's console (harness.ts): the overview renders at a
// phone held upright, a phone on its side and a desktop, with nothing wider
// than the screen and targets a finger can hit; keys are made, shown once,
// let a player in, and are revoked.

import { expect, test, type Page } from "@playwright/test";

import { ADMIN, PASSWORD, register } from "./harness.ts";

interface Size {
  name: string;
  width: number;
  height: number;
  phone: boolean;
  locale: "de-DE" | "en-GB";
  colorScheme: "light" | "dark";
}

const SIZES: Size[] = [
  { name: "phone-portrait", width: 360, height: 740, phone: true, locale: "de-DE", colorScheme: "light" },
  { name: "phone-landscape", width: 740, height: 360, phone: true, locale: "en-GB", colorScheme: "dark" },
  { name: "desktop", width: 1280, height: 800, phone: false, locale: "en-GB", colorScheme: "light" },
];

const WORDS = {
  "de-DE": { tab: "Admin", registered: "Registrierte Konten", accounts: "Konten", live: "Live", sets: "Sets" },
  "en-GB": { tab: "Admin", registered: "Registered accounts", accounts: "Accounts", live: "Live", sets: "Sets" },
} as const;

/** Fails the test on any script error or policy violation in the page. */
function watch(page: Page): string[] {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error" && !m.text().includes("401")) problems.push(`console: ${m.text()}`);
  });
  return problems;
}

async function signIn(page: Page) {
  await page.goto("/");
  await page.getByLabel("Name").fill(ADMIN);
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("navigation")).toBeVisible();
}

/** Nothing on the page is wider than the window. */
async function noSideways(page: Page) {
  const widths = await page.evaluate(() => ({
    page: document.documentElement.scrollWidth,
    window: document.documentElement.clientWidth,
    // What sticks out, so a failure names it.
    beyond: [...document.querySelectorAll("body *")]
      .filter((el) => el.getBoundingClientRect().right > document.documentElement.clientWidth + 1 && el.closest(".sections") === null)
      .map((el) => el.outerHTML.slice(0, 100))
      .slice(0, 8),
  }));
  expect(widths.page, `horizontal scroll: ${widths.beyond.join("\n")}`).toBeLessThanOrEqual(widths.window);
  // Nor is anything wider than its own card: a card's overflow hides under
  // its neighbour and never shows as a page that scrolls.
  const spilled = await page.evaluate(() =>
    [...document.querySelectorAll(".admin-page .card, .admin-page .invite, .admin-page .panel, .admin-page .kpi")]
      .filter((el) => el.scrollWidth > el.clientWidth + 1)
      .map((el) => el.textContent?.slice(0, 40)),
  );
  expect(spilled, "content wider than its card").toEqual([]);
}

/** Every visible control in the console is at least 44 px high. */
async function fingerSized(page: Page) {
  const small = await page.evaluate(() =>
    [
      ...document.querySelectorAll(
        ".bar a, .bar button, .sections a, .admin-page button, .admin-page input, .admin-page select",
      ),
    ]
      .filter((el) => (el as HTMLElement).offsetParent !== null)
      .map((el) => ({ el: el.outerHTML.slice(0, 80), h: el.getBoundingClientRect().height }))
      .filter(({ h }) => h < 44),
  );
  expect(small).toEqual([]);
}

test.describe.configure({ mode: "serial" });

for (const size of SIZES) {
  test.describe(size.name, () => {
    test.use({
      viewport: { width: size.width, height: size.height },
      isMobile: size.phone,
      hasTouch: size.phone,
      locale: size.locale,
      colorScheme: size.colorScheme,
    });

    test(`the overview renders at ${size.width}×${size.height}`, async ({ page }) => {
      const problems = watch(page);
      const words = WORDS[size.locale];
      await signIn(page);
      await page.getByRole("link", { name: words.tab }).click();
      await expect(page).toHaveURL(/\/admin$/);
      await expect(page.getByTestId("registered")).toHaveText(new RegExp(`^2 ${words.registered}$`));
      await expect(page.getByTestId("agents")).toBeVisible();
      await expect(page.getByRole("figure").first()).toBeVisible();
      await noSideways(page);
      await fingerSized(page);
      // A reload lands on the overview again: it is a page of its own.
      await page.reload();
      await expect(page.getByTestId("registered")).toBeVisible();
      const dir = process.env["SCREENSHOTS_DIR"];
      if (dir) {
        await page.screenshot({ path: `${dir}/overview-${size.name}.png`, fullPage: true });
        await page.screenshot({ path: `${dir}/overview-${size.name}-viewport.png` });
        const other = size.colorScheme === "light" ? "dark" : "light";
        await page.emulateMedia({ colorScheme: other });
        await page.screenshot({ path: `${dir}/overview-${size.name}-${other}.png`, fullPage: true });
      }
      // The accounts: the seeded players, by handle, and one of them in full.
      await page.getByRole("navigation", { name: /sections|Bereiche/ }).getByRole("link", { name: words.accounts }).click();
      await expect(page).toHaveURL(/\/admin\/accounts$/);
      await expect(page.getByTestId("account")).toHaveCount(2);
      await noSideways(page);
      await fingerSized(page);
      await page.getByTestId("account").first().getByRole("link").click();
      await expect(page).toHaveURL(/\/admin\/accounts\/[0-9a-f-]{36}$/);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(/^player\d#/);
      await noSideways(page);
      // Live: nobody plays in the e2e, and it says so.
      await page.getByRole("navigation", { name: /sections|Bereiche/ }).getByRole("link", { name: words.live }).click();
      await expect(page).toHaveURL(/\/admin\/live$/);
      await expect(page.getByRole("region", { name: /Agent/ })).toBeVisible();
      // The server: its first sample is taken as the gateway starts, so the
      // tiles are there; what the host could not say is a dash, not a zero.
      const server = page.getByRole("region", { name: "Server" });
      await expect(server.getByTestId("m-req")).toBeVisible();
      await expect(server.getByTestId("m-up")).toBeVisible();
      // The pool: the sets being worked on, Alpha among them or done.
      await expect(page.getByTestId("pool-implemented")).toBeVisible();
      await noSideways(page);
      await fingerSized(page);
      if (dir) await page.screenshot({ path: `${dir}/live-${size.name}.png`, fullPage: true });
      // The sets: every set in release order, and one set's cards with
      // their pictures from Scryfall's host, never from this service.
      await page.getByRole("navigation", { name: /sections|Bereiche/ }).getByRole("link", { name: words.sets }).click();
      await expect(page).toHaveURL(/\/admin\/sets$/);
      const first = page.getByTestId("set").first();
      await expect(first).toContainText("LEA");
      await noSideways(page);
      await fingerSized(page);
      if (dir) {
        await page.screenshot({ path: `${dir}/sets-${size.name}.png`, fullPage: true });
        await page.screenshot({ path: `${dir}/sets-${size.name}-viewport.png` });
      }
      await first.getByRole("link").click();
      await expect(page).toHaveURL(/\/admin\/sets\/lea$/);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText("LEA");
      const bolt = page.getByTestId("set-card").filter({ hasText: "Lightning Bolt" }).first();
      await expect(bolt.getByRole("link")).toHaveAttribute("href", /^https:\/\/scryfall\.com\/card\//);
      await noSideways(page);
      if (dir) {
        await page.screenshot({ path: `${dir}/set-lea-${size.name}.png`, fullPage: true });
        await page.screenshot({ path: `${dir}/set-lea-${size.name}-viewport.png` });
      }
      expect(problems).toEqual([]);
    });
  });
}

test.describe("keys", () => {
  test.use({ viewport: { width: 360, height: 740 }, isMobile: true, hasTouch: true, locale: "en-GB" });

  test("are made, shown once, let a player in, and are revoked", async ({ page }) => {
    const problems = watch(page);
    await signIn(page);
    await page.getByRole("link", { name: "Admin" }).click();
    await expect(page.getByTestId("keys-active")).toBeVisible();
    await page.getByRole("link", { name: "Beta keys" }).first().click();
    await expect(page).toHaveURL(/\/admin\/keys$/);

    await page.getByLabel("How many").fill("2");
    await page.getByLabel("Expires").selectOption("7d");
    await page.getByLabel("Note (who it is for)").fill("for the e2e");
    await page.getByRole("button", { name: "Make keys" }).click();

    const made = page.getByRole("region", { name: "2 new key(s): shown only now" });
    await expect(made).toBeVisible();
    await expect(made.getByRole("heading")).toBeFocused();
    const keys = await made.locator("input.key").evaluateAll((inputs) =>
      inputs.map((i) => (i as HTMLInputElement).value),
    );
    expect(keys).toHaveLength(2);
    for (const key of keys) expect(key).toMatch(/^BAYLEE(-[0-9A-Z]{4}){4}$/);
    await noSideways(page);
    await fingerSized(page);
    const dir = process.env["SCREENSHOTS_DIR"];
    if (dir) await page.screenshot({ path: `${dir}/keys-made-phone-portrait.png`, fullPage: true });

    // The list never shows a key, and has the two new ones.
    const items = page.getByTestId("invite").filter({ hasText: "for the e2e" });
    await expect(items).toHaveCount(2);
    expect(await page.locator(".invite-list").textContent()).not.toContain("BAYLEE-");
    await made.getByRole("button", { name: "Done" }).click();
    await expect(page.getByText(keys[0] ?? "no key")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Make keys" })).toBeVisible();

    // A key the console made lets a player in.
    expect(await register("e2e-first", keys[0] ?? "")).toBe(200);

    // Revoking asks once more, and the gateway then refuses the key.
    const second = items.nth(0);
    await second.getByRole("button", { name: /^Revoke…/ }).click();
    await second.getByRole("button", { name: "Keep it" }).click();
    await second.getByRole("button", { name: /^Revoke…/ }).click();
    await second.getByRole("button", { name: "Revoke for good" }).click();
    await expect(page.getByText("Key revoked.")).toBeVisible();
    await expect(page.getByTestId("invite").filter({ hasText: "revoked" })).toHaveCount(1);
    const statuses = [await register("e2e-second", keys[0] ?? ""), await register("e2e-third", keys[1] ?? "")];
    // keys[0] is used up; whichever of the two was revoked, neither admits now.
    expect(statuses).toEqual([403, 403]);
    await expect(page.getByText(/gateway\.invite\.revoke/)).toBeVisible();
    await expect(page.getByText(/gateway\.invite\.create/)).toBeVisible();
    if (dir) await page.screenshot({ path: `${dir}/keys-list-phone-portrait.png`, fullPage: true });
    expect(problems).toEqual([]);
  });
});
