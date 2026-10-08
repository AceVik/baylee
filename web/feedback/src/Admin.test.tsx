import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, test, vi } from "vitest";

import { Admin, useVisibleInterval } from "./Admin";
import type { Invite, Stats } from "./api";
import { App } from "./App";
import { pickLang, t } from "./i18n";
import { serve, type Call } from "./test-utils";

const since = (n: number) => ({ today_utc: n, last_7d: n * 2, last_30d: n * 3 });

function stats(over: Partial<Stats> = {}): Stats {
  return {
    at: "2026-10-08T12:00:00Z",
    gateway: { name: "Baylee EU", version: "0.1.0-beta.5", registration: "invite" },
    accounts: { registered: 1234, with_email: 3, confirmed_email: 2, admitted_by_key: 40, created: since(1) },
    guests: { enabled: true, live: 7, cap: 1000 },
    online: { players: 17, in_lobby: 12, seated: 6, sessions_live: 30, accounts_signed_in: 25 },
    games: {
      running: 3,
      local_running: 3,
      waiting: 1,
      seats_awaiting_engine: 0,
      recorded: 900,
      started: since(4),
      finished: 880,
      finished_since: since(4),
    },
    agents: { connected: 1, local: 1, capacity: null, games: 3 },
    invites: { total: 50, active: 9, used_up: 30, expired: 6, revoked: 5, uses_left: 11, admitted: 40 },
    ...over,
  };
}

function invite(over: Partial<Invite> = {}): Invite {
  return {
    id: "0199aaaa-0000-7000-8000-0000000000aa",
    created_at: "2026-10-01T10:00:00Z",
    note: "for Max",
    uses_left: 1,
    expires_at: null,
    revoked_at: null,
    admitted: 0,
    state: "active",
    ...over,
  };
}

/** A console that makes keys and revokes them as the gateway would. */
function start(): Call[] {
  let keys: Invite[] = [invite()];
  return serve((call) => {
    if (call.url === "/ui/api/admin/stats") return { body: stats() };
    if (call.url === "/ui/api/admin/audit") return { body: [] };
    if (call.url === "/ui/api/admin/invites" && call.method === "GET") return { body: keys };
    if (call.url === "/ui/api/admin/invites" && call.method === "POST") {
      const made = [
        { id: "0199bbbb-0000-7000-8000-000000000001", key: "BAYLEE-AAAA-BBBB-CCCC-DDDD" },
        { id: "0199bbbb-0000-7000-8000-000000000002", key: "BAYLEE-EEEE-FFFF-GGGG-HHHH" },
      ];
      keys = [...made.map((m) => invite({ id: m.id, note: "for Ana" })), ...keys];
      return { status: 201, body: { keys: made, uses: 1, expires_at: null, note: "for Ana" } };
    }
    if (call.method === "DELETE") {
      keys = keys.map((k) => (call.url.endsWith(k.id) ? { ...k, state: "revoked" as const } : k));
      return { status: 204 };
    }
    return undefined;
  });
}

afterEach(() => {
  vi.useRealTimers();
});

describe("the overview", () => {
  test("shows the gateway's numbers, in English or German", async () => {
    start();
    const { unmount } = render(<Admin lang="en" />);
    expect((await screen.findByTestId("registered")).textContent).toBe("1,234 Registered accounts");
    expect(screen.getByTestId("online").textContent).toContain("17");
    expect(screen.getByTestId("running").textContent).toContain("3");
    const server = within(screen.getByRole("region", { name: "Server" }));
    expect(server.getByText("unlimited")).toBeTruthy();
    unmount();
    render(<Admin lang="de" />);
    expect((await screen.findByTestId("registered")).textContent).toBe("1.234 Registrierte Konten");
    expect(screen.getByRole("region", { name: "Jetzt online" })).toBeTruthy();
  });

  test("makes keys, shows them once, and forgets them when done", async () => {
    const calls = start();
    render(<Admin lang="en" />);
    await screen.findByTestId("registered");
    await userEvent.clear(screen.getByLabelText("How many"));
    await userEvent.type(screen.getByLabelText("How many"), "2");
    await userEvent.selectOptions(screen.getByLabelText("Expires"), "7d");
    await userEvent.type(screen.getByLabelText("Note (who it is for)"), "  for Ana ");
    await userEvent.click(screen.getByRole("button", { name: "Make keys" }));

    const made = await screen.findByRole("region", { name: "2 new key(s): shown only now" });
    expect(within(made).getByDisplayValue("BAYLEE-AAAA-BBBB-CCCC-DDDD")).toBeTruthy();
    expect(within(made).getByDisplayValue("BAYLEE-EEEE-FFFF-GGGG-HHHH")).toBeTruthy();
    const post = calls.find((c) => c.method === "POST");
    expect(post?.body).toEqual({ count: 2, uses: 1, expires: "7d", note: "for Ana" });
    expect(post?.headers["x-baylee-csrf"]).toBe("1");
    // The list is asked again, and never holds a key.
    await waitFor(() => {
      expect(screen.getAllByTestId("invite")).toHaveLength(3);
    });
    for (const item of screen.getAllByTestId("invite")) expect(item.textContent).not.toContain("BAYLEE-");

    await userEvent.click(within(made).getByRole("button", { name: "Done" }));
    expect(screen.queryByDisplayValue("BAYLEE-AAAA-BBBB-CCCC-DDDD")).toBeNull();
    expect(screen.getByRole("button", { name: "Make keys" })).toBeTruthy();
  });

  test("a gateway's refusal is said in its words", async () => {
    serve((call) => {
      if (call.method === "POST") return { status: 400, body: { error: '--uses takes 1 to 1000, not "0"' } };
      if (call.url === "/ui/api/admin/invites") return { body: [] };
      if (call.url === "/ui/api/admin/audit") return { body: [] };
      return { body: stats() };
    });
    render(<Admin lang="en" />);
    await screen.findByTestId("registered");
    await userEvent.click(screen.getByRole("button", { name: "Make keys" }));
    expect((await screen.findByRole("alert")).textContent).toBe('--uses takes 1 to 1000, not "0"');
  });

  test("revoking asks once more", async () => {
    const calls = start();
    render(<Admin lang="en" />);
    await userEvent.click(await screen.findByRole("button", { name: "Revoke… for Max" }));
    await userEvent.click(screen.getByRole("button", { name: "Keep it" }));
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Revoke… for Max" }));
    await userEvent.click(screen.getByRole("button", { name: "Revoke for good" }));
    expect(await screen.findByText("Key revoked.")).toBeTruthy();
    const del = calls.find((c) => c.method === "DELETE");
    expect(del?.url).toBe("/ui/api/admin/invites/0199aaaa-0000-7000-8000-0000000000aa");
    expect(del?.headers["x-baylee-csrf"]).toBe("1");
    await waitFor(() => {
      expect(screen.getByTestId("invite").textContent).toContain("revoked");
    });
    expect(screen.queryByRole("button", { name: /Revoke/ })).toBeNull();
  });
});

function Probe({ onTick }: { onTick: () => void }) {
  useVisibleInterval(onTick, 1000);
  return null;
}

function setHidden(hidden: boolean) {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => (hidden ? "hidden" : "visible"),
  });
  act(() => {
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

describe("the refresh", () => {
  test("runs while the page is visible and stops while it is hidden", () => {
    vi.useFakeTimers();
    const tick = vi.fn<() => void>();
    const { unmount } = render(<Probe onTick={tick} />);
    expect(tick).toHaveBeenCalledTimes(1);
    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(tick).toHaveBeenCalledTimes(4);
    setHidden(true);
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(tick).toHaveBeenCalledTimes(4);
    setHidden(false);
    expect(tick).toHaveBeenCalledTimes(5);
    unmount();
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(tick).toHaveBeenCalledTimes(5);
  });
});

describe("the app's sections", () => {
  test("the overview's tab shows only where a console is configured", async () => {
    let console = false;
    serve((call) => {
      if (call.url === "/ui/api/me") return { body: { name: "viktor", gateway_admin: console } };
      if (call.url.startsWith("/ui/api/reports")) return { body: { total: 0, reports: [] } };
      return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
    });
    const { unmount } = render(<App />);
    await screen.findByText("viktor");
    expect(screen.queryByRole("navigation")).toBeNull();
    unmount();
    console = true;
    render(<App />);
    const nav = await screen.findByRole("navigation");
    expect(within(nav).getAllByRole("link")).toHaveLength(2);
  });
});

test("the language is the browser's when it is German, English otherwise", () => {
  expect(pickLang(["de-DE", "en"])).toBe("de");
  expect(pickLang(["fr-FR", "de-AT"])).toBe("de");
  expect(pickLang(["en-US", "de"])).toBe("en");
  expect(pickLang(["fr"])).toBe("en");
  expect(pickLang([])).toBe("en");
  expect(t("de", "invites.made", { n: 3 })).toBe("3 neue(r) Schlüssel: nur jetzt sichtbar");
});
