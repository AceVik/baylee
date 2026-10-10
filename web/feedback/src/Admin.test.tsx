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
  test("draws a month of days and says each one on hover and in a table", async () => {
    const daily = Array.from({ length: 30 }, (_, i) => ({
      day: `2026-09-${String(i + 1).padStart(2, "0")}`,
      registered: i === 29 ? 4 : 0,
      guests: i === 29 ? 2 : 1,
      started: i,
      finished: 0,
      players: 2,
    }));
    serve((call) => (call.url === "/ui/api/admin/stats" ? { body: stats({ daily }) } : undefined));
    render(<Admin lang="en" />);
    const chart = await screen.findByRole("figure", { name: "Games started per day" });
    expect(chart.textContent).toContain("435 in 30 days");
    const range = within(chart).getByRole("slider");
    range.focus();
    expect(range.getAttribute("aria-valuetext")).toBe("30 Sept: Games 29");
    await userEvent.keyboard("{ArrowLeft}");
    expect(within(chart).getByRole("status").textContent).toContain("Games: 28");
    const accounts = screen.getByRole("figure", { name: "New accounts per day" });
    expect(within(accounts).getAllByText("Registered").length).toBeGreaterThan(0);
    expect(within(accounts).getAllByRole("row")).toHaveLength(31);
  });

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
    render(<Admin lang="en" section="keys" />);
    await screen.findAllByTestId("invite");
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
    render(<Admin lang="en" section="keys" />);
    await userEvent.click(await screen.findByRole("button", { name: "Make keys" }));
    expect((await screen.findByRole("alert")).textContent).toBe('--uses takes 1 to 1000, not "0"');
  });

  test("revoking asks once more", async () => {
    const calls = start();
    render(<Admin lang="en" section="keys" />);
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

describe("the people pages", () => {
  const row = {
    id: "0199cccc-0000-7000-8000-000000000001",
    username: "alice",
    display_name: "Alice",
    tag: 1,
    handle: "Alice#0001",
    guest: false,
    created_at: "2026-10-01T10:00:00Z",
    has_email: false,
    confirmed: false,
    lang: "de",
    by_key: true,
    key_note: "for Alice",
    terms_version: null,
    decks: 2,
    games: 7,
    sessions: 1,
    active_at: "2026-10-08T11:00:00Z",
    in_lobby: false,
    playing: true,
    online: true,
  };

  test("the accounts are searched in the query string and listed by handle", async () => {
    const calls = serve((call) =>
      call.url.startsWith("/ui/api/admin/accounts")
        ? { body: { total: 1, online: 1, accounts: [row] } }
        : undefined,
    );
    window.history.replaceState(null, "", "/admin/accounts?kind=registered");
    render(<Admin lang="en" section="accounts" search="?kind=registered" />);
    const item = await screen.findByTestId("account");
    expect(item.textContent).toContain("Alice#0001");
    expect(item.textContent).toContain("in a game");
    expect(item.textContent).toContain("beta key");
    expect(calls[0]?.url).toBe("/ui/api/admin/accounts?limit=50&kind=registered");
    expect(screen.getByRole("button", { name: "Registered" }).getAttribute("aria-pressed")).toBe("true");
    await userEvent.type(screen.getByRole("searchbox", { name: "Search accounts" }), "ali");
    await waitFor(() => {
      expect(window.location.search).toBe("?q=ali&kind=registered");
    });
  });

  test("one account shows its decks and games, and nothing of a deck's cards", async () => {
    serve((call) =>
      call.url === `/ui/api/admin/accounts/${row.id}`
        ? {
            body: {
              ...row,
              terms_accepted_at: null,
              confirmed_at: null,
              table: { id: "g1", name: "Kitchen table", state: "playing" },
              deck_list: [
                { id: "d1", name: "Woods", format: "commander", cards: 100, version: 3, updated_at: "2026-10-02T10:00:00Z" },
              ],
              recent_games: [
                {
                  game_id: "g0",
                  started_at: "2026-10-05T10:00:00Z",
                  ended_at: "2026-10-05T10:42:00Z",
                  complete: true,
                  seat: 0,
                  chairs: [
                    { seat: 0, player: "Alice#0001", guest: false },
                    { seat: 1, player: null, guest: null },
                  ],
                },
              ],
            },
          }
        : undefined,
    );
    render(<Admin lang="en" section="accounts" id={row.id} />);
    expect(await screen.findByRole("heading", { name: "Alice#0001" })).toBeTruthy();
    expect(screen.getByText("Woods")).toBeTruthy();
    expect(screen.getByText(/100 cards/)).toBeTruthy();
    expect(screen.getByText("Kitchen table")).toBeTruthy();
    expect(screen.getByText("House AI or no account")).toBeTruthy();
    expect(screen.getByText(/lasted 42/)).toBeTruthy();
    expect(screen.getByText("for Alice")).toBeTruthy();
  });

  test("a gone account says so", async () => {
    serve(() => ({ status: 404, body: { error: "no such account" } }));
    render(<Admin lang="de" section="accounts" id="0199cccc-0000-7000-8000-000000000009" />);
    expect((await screen.findByRole("alert")).textContent).toBe("Dieses Konto gibt es nicht (mehr).");
  });

  test("live shows who is where and every table's chairs", async () => {
    serve((call) =>
      call.url === "/ui/api/admin/live"
        ? {
            body: {
              at: "2026-10-08T12:00:00Z",
              players: [
                { id: row.id, handle: "Alice#0001", guest: false, in_lobby: true, playing: null, waiting: "g1" },
                { id: "x", handle: "Visitor#0003", guest: true, in_lobby: true, playing: null, waiting: null },
              ],
              tables: [
                {
                  id: "g1",
                  name: "Kitchen table",
                  state: "waiting",
                  host: "Alice#0001",
                  host_id: row.id,
                  created_at: "2026-10-08T11:55:00Z",
                  locked: true,
                  rematch: false,
                  decide_secs: 180,
                  engine: false,
                  engine_local: false,
                  agent: null,
                  seats: [
                    { seat: 0, kind: "human", ai: null, account_id: row.id, player: "Alice#0001", guest: false, bridge: null, bridged_by: null, deck: "Woods", format: "commander", ready: true, team: null },
                    { seat: 1, kind: "ai", ai: "sharp", account_id: null, player: null, guest: null, bridge: null, bridged_by: null, deck: "", format: null, ready: true, team: null },
                    { seat: 2, kind: "human", ai: null, account_id: null, player: null, guest: null, bridge: null, bridged_by: null, deck: "", format: null, ready: false, team: null },
                  ],
                },
              ],
              agents: [],
            },
          }
        : undefined,
    );
    render(<Admin lang="en" section="live" />);
    const players = await screen.findAllByTestId("online-player");
    expect(players.map((p) => p.textContent)).toEqual([
      expect.stringContaining("Alice#0001"),
      expect.stringContaining("Visitor#0003guest"),
    ]);
    const table = screen.getByTestId("table");
    expect(table.textContent).toContain("Kitchen table");
    expect(table.textContent).toContain("2 of 3 chairs taken");
    expect(table.textContent).toContain("House AI · sharp");
    expect(table.textContent).toContain("open chair");
    expect(table.textContent).toContain("password");
    expect(screen.getByText("No agent is connected: no new game can start.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /playing/ }));
    expect(screen.queryByTestId("table")).toBeNull();
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
