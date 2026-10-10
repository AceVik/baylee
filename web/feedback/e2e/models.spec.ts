// The Models page in a real browser, signed in through a real
// baylee-feedback (harness.ts); the gateway's hosted-model console is
// stood in for in the browser (`page.route`), since a seat agent would
// need a model: two seat agents, three profiles, every change kept in
// memory so the page's refresh shows it. Renders at a phone and a desktop,
// in both languages and both themes, with nothing wider than the screen.

import { expect, test, type Page, type Route } from "@playwright/test";

import { ADMIN, PASSWORD } from "./harness.ts";

interface Profile {
  id: string;
  label: string;
  vendor: string;
  kind: "api" | "cli";
  model: string;
  state: string;
  until_unix: number | null;
  games: number;
  max_games: number | null;
  enabled: boolean;
  canary: boolean;
  caps: Record<string, number> | null;
  spent: Record<string, number> | null;
  key: string;
  last_error: string | null;
  last_ok_unix: number | null;
  definition: Record<string, unknown>;
}

function profile(over: Partial<Profile>): Profile {
  const base: Profile = {
    id: "sonnet",
    label: "Sonnet",
    vendor: "Anthropic",
    kind: "api",
    model: "claude-sonnet-5-5",
    state: "available",
    until_unix: null,
    games: 1,
    max_games: 4,
    enabled: true,
    canary: false,
    caps: { day_usd: 10, month_usd: 100 },
    spent: { day_usd: 0.42, month_usd: 3.1, day_tokens: 0, month_tokens: 0 },
    key: "kept",
    last_error: null,
    last_ok_unix: Math.floor(Date.now() / 1000) - 120,
    definition: {},
  };
  const p = { ...base, ...over };
  p.definition = { label: p.label, vendor: p.vendor, enabled: p.enabled, profile: { provider: p.kind === "cli" ? "cli" : "anthropic", model: p.model } };
  return p;
}

/** The stand-in gateway: what it holds, and what it was asked. */
async function standIn(page: Page) {
  const hosts = [
    { name: "seat-eu-1", local: true, capacity: 4, games: 2, connected_secs: 7_200, profiles: [profile({ games: 2 }), profile({ id: "cc", label: "Claude CLI", kind: "cli", model: "claude:opus", key: "none_needed", state: "needs_login", games: 0, max_games: 2, caps: null })] },
    {
      name: "seat-eu-2",
      local: false,
      capacity: 0,
      games: 0,
      connected_secs: 300,
      profiles: [
        profile({ state: "exhausted", until_unix: Math.floor(Date.now() / 1000) + 3_600, games: 0, spent: { day_usd: 10, month_usd: 40, day_tokens: 0, month_tokens: 0 } }),
        profile({ id: "deepseek", label: "DeepSeek", vendor: "DeepSeek", model: "deepseek-chat", key: "absent", state: "failing", last_error: "no key", games: 0, canary: true, caps: { day_usd: 2 } }),
      ],
    },
  ];
  const asked: { method: string; path: string; body: unknown }[] = [];
  await page.route("**/ui/api/admin/llm/**", async (route: Route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const body = request.postData() === null ? undefined : (JSON.parse(request.postData() ?? "null") as unknown);
    if (request.method() === "GET") {
      await route.fulfill({ json: { seathosts: hosts } });
      return;
    }
    asked.push({ method: request.method(), path, body });
    const match = /seathosts\/([^/]+)\/profiles\/([^/]+)(?:\/(\w+))?$/.exec(path);
    const host = hosts.find((h) => h.name === match?.[1]);
    const id = match?.[2] ?? "";
    const p = host?.profiles.find((x) => x.id === id);
    if (request.method() === "DELETE" && host) host.profiles = host.profiles.filter((x) => x.id !== id);
    if (request.method() === "PUT" && host) {
      const d = body as { label: string; vendor: string };
      host.profiles = [...host.profiles.filter((x) => x.id !== id), profile({ id, label: d.label, vendor: d.vendor, games: 0 })];
    }
    if (match?.[3] === "enabled" && p) {
      p.enabled = (body as { enabled: boolean }).enabled;
      p.state = p.enabled ? "available" : "disabled";
    }
    if (match?.[3] === "key" && p) p.key = (body as { key: string | null }).key === null ? "absent" : "kept";
    await route.fulfill({ status: request.method() === "PUT" ? 200 : 204, body: request.method() === "PUT" ? '{"ok":true}' : "" });
  });
  return asked;
}

const SIZES = [
  { name: "phone", width: 360, height: 740, phone: true, locale: "de-DE", colorScheme: "dark" },
  { name: "desktop", width: 1280, height: 860, phone: false, locale: "en-GB", colorScheme: "light" },
] as const;

const WORDS = {
  "de-DE": { section: "Modelle", title: "Sprachmodelle", key: "Schlüssel…", send: "Schlüssel senden", del: "Löschen", sent: "Schlüssel gesendet.", removed: "Profil entfernt.", on: "Eingeschaltet" },
  "en-GB": { section: "Models", title: "Language models", key: "Key…", send: "Send key", del: "Delete", sent: "Key sent.", removed: "Profile removed.", on: "Enabled" },
} as const;

async function noSideways(page: Page) {
  const widths = await page.evaluate(() => ({
    page: document.documentElement.scrollWidth,
    window: document.documentElement.clientWidth,
  }));
  expect(widths.page).toBeLessThanOrEqual(widths.window);
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

    test(`the models page at ${size.width}×${size.height}`, async ({ page }) => {
      const problems: string[] = [];
      page.on("pageerror", (e) => problems.push(e.message));
      const words = WORDS[size.locale];
      const asked = await standIn(page);
      await page.goto("/");
      await page.getByLabel("Name").fill(ADMIN);
      await page.getByLabel("Password").fill(PASSWORD);
      await page.getByRole("button", { name: "Sign in" }).click();
      await expect(page.getByRole("navigation").first()).toBeVisible();
      await page.getByRole("link", { name: "Admin" }).click();
      await page.getByRole("link", { name: words.section }).click();
      await expect(page.getByRole("heading", { name: words.title })).toBeVisible();
      await expect(page.getByTestId("llm-host")).toHaveCount(2);
      await expect(page.getByTestId("llm-profile")).toHaveCount(3);
      await noSideways(page);
      const dir = process.env["SCREENSHOTS_DIR"];
      if (dir) {
        await page.screenshot({ path: `${dir}/models-${size.name}.png`, fullPage: true });
        const other = size.colorScheme === "light" ? "dark" : "light";
        await page.emulateMedia({ colorScheme: other });
        await page.screenshot({ path: `${dir}/models-${size.name}-${other}.png`, fullPage: true });
        await page.emulateMedia({ colorScheme: size.colorScheme });
      }

      // A key: typed into a password field, sent, gone from the page.
      await page.getByRole("button", { name: `${words.key} deepseek @ seat-eu-2` }).click();
      const field = page.locator("input[type=password]");
      await field.fill("sk-e2e-not-a-real-key");
      await page.getByRole("button", { name: words.send }).click();
      await expect(page.getByText(words.sent)).toBeVisible();
      expect(await page.content()).not.toContain("sk-e2e-not-a-real-key");

      // Switching off, then deleting after the question.
      await page.getByRole("button", { name: `${words.on} cc @ seat-eu-1` }).click();
      await expect(page.getByRole("button", { name: `${words.on} cc @ seat-eu-1` })).toHaveAttribute("aria-pressed", "false");
      await page.getByRole("button", { name: `${words.del} cc @ seat-eu-1` }).click();
      const dialog = page.getByRole("alertdialog");
      await expect(dialog).toBeVisible();
      if (dir) {
        await page.waitForTimeout(300);
        await page.screenshot({ path: `${dir}/models-${size.name}-confirm.png` });
      }
      await dialog.getByRole("button", { name: words.del }).click();
      await expect(page.getByText(words.removed)).toBeVisible();
      await expect(page.getByTestId("llm-profile")).toHaveCount(2);

      expect(asked.map((a) => `${a.method} ${a.path}`)).toEqual([
        "POST /ui/api/admin/llm/seathosts/seat-eu-2/profiles/deepseek/key",
        "POST /ui/api/admin/llm/seathosts/seat-eu-1/profiles/cc/enabled",
        "DELETE /ui/api/admin/llm/seathosts/seat-eu-1/profiles/cc",
      ]);
      expect(problems).toEqual([]);
    });
  });
}
