// The console's link to Baylee Central in a real browser, through a real
// service and gateway (harness.ts): not linked, a token checked before it is
// sent, and a well-formed one the gateway cannot check because its Central
// (a closed loopback port in this harness, never the live service) does not
// answer. Nothing is kept.

import { expect, test, type Page } from "@playwright/test";

import { ADMIN, PASSWORD } from "./harness.ts";

async function signIn(page: Page) {
  await page.goto("/");
  await page.getByLabel("Name").fill(ADMIN);
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("navigation")).toBeVisible();
}

test.use({ viewport: { width: 1280, height: 860 }, locale: "en-GB" });

test("a token is checked before the gateway keeps it", async ({ page }) => {
  await signIn(page);
  await page.goto("/admin/central");
  await expect(page.getByRole("heading", { name: "Link to Baylee Central" })).toBeVisible();
  await expect(page.getByText(/Not linked/)).toBeVisible();
  const field = page.getByLabel("Token");
  await field.fill("bgt_short");
  await page.getByRole("button", { name: "Link" }).click();
  await expect(page.getByText("A token starts with bgt_ and has 64 hex digits after it.")).toBeVisible();
  await field.fill(`bgt_${"ab".repeat(32)}`);
  await page.getByRole("button", { name: "Link" }).click();
  await expect(page.getByText("Baylee Central could not be asked about the token")).toBeVisible();
  const dir = process.env["SCREENSHOT_DIR"];
  if (dir) await page.screenshot({ path: `${dir}/console-central.png`, fullPage: true });
  await page.reload();
  await expect(page.getByText(/Not linked/)).toBeVisible();
});
