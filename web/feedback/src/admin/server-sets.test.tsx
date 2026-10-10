import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import { Admin } from "../Admin";
import type { Metrics, SetDetail, SetsOverview } from "../api";
import { parseRoute } from "../router";
import { serve } from "../test-utils";
import { reportDays } from "./Overview";

const BOLT = "e3285e6b-3e79-4d7c-bf96-d920f973b80b";

function metrics(over: Partial<Metrics> = {}): Metrics {
  const n = 12;
  const at = Array.from({ length: n }, (_, i) => 1_700_000_000 + i * 5);
  return {
    at: "2026-10-10T12:00:00Z",
    interval_secs: 5,
    keep: 720,
    platform: "linux",
    cores: 4,
    process_uptime_secs: 3_700,
    host: { uptime_secs: 864_000, mem_total: 8_000_000_000, disk_total: 50_000_000_000, disk_path: "/", load: [0.52, 0.4, 0.31] },
    now: {
      at: at[n - 1] ?? 0,
      cpu: 0.125,
      load1: 0.52,
      mem_used: 2_500_000_000,
      disk_used: 20_000_000_000,
      net_in: 12_000,
      net_out: 3_000,
      sockets: 17,
      requests: 2.5,
      games: [{ game_id: "g1", pid: 4242, cpu: 0.25, rss: 150_000_000 }],
    },
    history: {
      at,
      cpu: at.map((_, i) => (i === 0 ? null : 0.1)),
      load1: at.map(() => 0.5),
      mem_used: at.map(() => 2_500_000_000),
      disk_used: at.map(() => 20_000_000_000),
      net_in: at.map((_, i) => (i === 0 ? null : 12_000)),
      net_out: at.map((_, i) => (i === 0 ? null : 3_000)),
      sockets: at.map(() => 17),
      requests: at.map(() => 2.5),
    },
    games: [{ game_id: "g1", pid: 4242, cpu: at.map((_, i) => (i < 2 ? null : 0.25)), rss: at.map(() => 150_000_000) }],
    ...over,
  };
}

const LIVE = {
  at: "2026-10-10T12:00:00Z",
  players: [],
  tables: [],
  agents: [{ id: "a1", name: "box", local: true, capacity: 0, games: 1 }],
};

const SETS: SetsOverview = {
  at: "2026-10-10T12:00:00Z",
  version: "0.1.0-beta.7 (abc)",
  corpus: 1000,
  pool: { cards: 400, implemented: 300, partial: 50, unimplemented: 50, absent: 600 },
  sets: [
    { code: "lea", position: 0, total: 295, implemented: 290, partial: 3, unimplemented: 2, absent: 0 },
    { code: "leb", position: 1, total: 7, implemented: 7, partial: 0, unimplemented: 0, absent: 0 },
    { code: "arn", position: 2, total: 78, implemented: 3, partial: 0, unimplemented: 0, absent: 75 },
  ],
};

const ALPHA: SetDetail = {
  code: "lea",
  position: 0,
  total: 295,
  implemented: 290,
  partial: 3,
  unimplemented: 2,
  absent: 0,
  cards: [
    { index: 160, name: "Lightning Bolt", coverage: "implemented", note: null, type_line: "Instant", scryfall_id: BOLT, oracle_id: "o1" },
    { index: 5, name: "Ankh of Mishra", coverage: "partial", note: "no damage to players yet", type_line: "Artifact", scryfall_id: BOLT, oracle_id: "o2" },
    { index: 9, name: "Badlands", coverage: "absent", note: null, type_line: "", scryfall_id: null, oracle_id: "o3" },
  ],
};

describe("the reports per day", () => {
  test("fill every day of the window and put bugs and crashes apart", () => {
    const days = reportDays(
      {
        days: 30,
        rows: [
          { day: "2026-10-10", kind: "bug", count: 2 },
          { day: "2026-10-10", kind: "crash", count: 1 },
          { day: "2026-10-10", kind: "feedback", count: 4 },
          { day: "2026-09-11", kind: "bug", count: 1 },
          { day: "2026-09-10", kind: "bug", count: 9 },
        ],
      },
      new Date("2026-10-10T15:00:00Z"),
    );
    expect(days).toHaveLength(30);
    expect(days[0]).toEqual({ day: "2026-09-11", bugs: 1, other: 0 });
    expect(days[29]).toEqual({ day: "2026-10-10", bugs: 3, other: 4 });
    expect(days[10]).toEqual({ day: "2026-09-21", bugs: 0, other: 0 });
  });
});

describe("the routes", () => {
  test("know the sets and one set", () => {
    expect(parseRoute("/admin/sets", "")).toEqual({ page: "admin", section: "sets" });
    expect(parseRoute("/admin/sets/LEA", "")).toEqual({ page: "admin", section: "sets", id: "LEA" });
    expect(parseRoute("/admin/live/x", "")).toEqual({ page: "list", search: "" });
  });
});

describe("the server on the live page", () => {
  test("shows the machine's numbers, a line for each, and every engine process", async () => {
    serve((call) => {
      if (call.url === "/ui/api/admin/live") return { body: LIVE };
      if (call.url === "/ui/api/admin/metrics") return { body: metrics() };
      if (call.url === "/ui/api/admin/sets") return { body: SETS };
      return undefined;
    });
    render(<Admin lang="en" section="live" />);
    const server = within(await screen.findByRole("region", { name: "Server" }));
    expect(server.getByTestId("m-cpu").textContent).toContain("13 %");
    expect(server.getByTestId("m-cpu").textContent).toContain("load 0.52 · 0.4 · 0.31");
    expect(server.getByTestId("m-mem").textContent).toContain("2.5 GB");
    expect(server.getByTestId("m-mem").textContent).toContain("of 8 GB");
    expect(server.getByTestId("m-disk").textContent).toContain("20 GB");
    expect(server.getByTestId("m-net").textContent).toContain("↓ 12 kB/s");
    expect(server.getByTestId("m-net").textContent).toContain("↑ 3 kB/s");
    expect(server.getByTestId("m-req").textContent).toContain("2.5/s");
    expect(server.getByTestId("m-req").textContent).toContain("17 open sockets");
    expect(server.getByTestId("m-up").textContent).toContain("host up 10d");
    // Every tile carries the last hour as a titled line.
    expect(server.getAllByTitle(/the last hour/)).toHaveLength(6 + 1);
    const game = server.getByTestId("engine-process");
    expect(game.textContent).toContain("g1");
    expect(game.textContent).toContain("pid 4242");
    expect(game.textContent).toContain("0.25 cores");
    expect(game.textContent).toContain("150 MB");
    // The pool is on the page too, with the sets still open first.
    const pool = within(screen.getByRole("region", { name: "Pool progress" }));
    expect((await pool.findByTestId("pool-implemented")).textContent).toContain("300");
    expect(pool.getByTestId("pool-implemented").textContent).toContain("of 1,000 cards implemented (30.0 %)");
    expect(pool.getAllByRole("link").map((a) => a.textContent)).toEqual(["All sets →", "LEA", "ARN"]);
  });

  test("a host that cannot say shows dashes, never zeros, and says why there is no process", async () => {
    const m = metrics({ platform: "macos" });
    m.now = {
      at: 0,
      cpu: null,
      load1: null,
      mem_used: null,
      disk_used: 20_000_000_000,
      net_in: null,
      net_out: null,
      sockets: null,
      requests: 0,
      games: [],
    };
    m.host = { ...m.host, uptime_secs: null, mem_total: null, load: null };
    serve((call) => {
      if (call.url === "/ui/api/admin/live") return { body: LIVE };
      if (call.url === "/ui/api/admin/metrics") return { body: m };
      return undefined;
    });
    render(<Admin lang="de" section="live" />);
    const server = within(await screen.findByRole("region", { name: "Server" }));
    expect(server.getByTestId("m-cpu").textContent).toBe("CPU—CPU, die letzte Stunde");
    expect(server.getByTestId("m-net").textContent).toContain("↓ —");
    expect(server.getByTestId("m-up").textContent).toContain("dieser Gateway-Prozess");
    expect(server.getByText(/Nur unter Linux/)).toBeTruthy();
    expect(server.queryByTestId("engine-process")).toBeNull();
  });
});

describe("the pool's progress", () => {
  test("lists every set in release order, sorted and filtered on request", async () => {
    serve((call) => (call.url === "/ui/api/admin/sets" ? { body: SETS } : undefined));
    render(<Admin lang="en" section="sets" />);
    const rows = await screen.findAllByTestId("set");
    expect(rows.map((r) => within(r).getByRole("link").textContent)).toEqual(["LEA", "LEB", "ARN"]);
    expect(rows[0]?.textContent).toContain("295");
    expect(rows[0]?.textContent).toContain("98 %");
    expect(screen.getByText("3 of 3 sets")).toBeTruthy();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Order" }), "open");
    expect(screen.getAllByTestId("set").map((r) => within(r).getByRole("link").textContent)).toEqual(["ARN", "LEA", "LEB"]);
    await userEvent.click(screen.getByRole("button", { name: "Unfinished only" }));
    expect(screen.getAllByTestId("set")).toHaveLength(2);
    await userEvent.type(screen.getByRole("searchbox", { name: "Set code" }), "le");
    expect(screen.getAllByTestId("set").map((r) => within(r).getByRole("link").textContent)).toEqual(["LEA"]);
    expect(screen.getByText(/counts the cards first printed in it/)).toBeTruthy();
  });

  test("one set lists its cards as chips with how far each is", async () => {
    serve((call) => (call.url === "/ui/api/admin/sets/lea" ? { body: ALPHA } : undefined));
    render(<Admin lang="en" section="sets" id="lea" />);
    expect(await screen.findByRole("heading", { level: 1, name: "LEA" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "On Scryfall" }).getAttribute("href")).toBe("https://scryfall.com/sets/lea");
    const cards = screen.getAllByTestId("set-card");
    expect(cards).toHaveLength(3);
    const bolt = within(cards[0] as HTMLElement);
    expect(bolt.getByRole("link", { name: "Lightning Bolt" }).getAttribute("href")).toBe(`https://scryfall.com/card/${BOLT}`);
    expect(bolt.getByText("Implemented")).toBeTruthy();
    expect(cards[1]?.textContent).toContain("no damage to players yet");
    // An absent card has no page to link.
    expect(within(cards[2] as HTMLElement).queryByRole("link")).toBeNull();
    expect(cards[2]?.textContent).toContain("Not started");
    await userEvent.click(screen.getByRole("button", { name: /^Partial/ }));
    expect(screen.getAllByTestId("set-card")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: /^All/ }));
    await userEvent.type(screen.getByRole("searchbox", { name: "Card name" }), "bad");
    expect(screen.getAllByTestId("set-card").map((c) => c.textContent)).toEqual([expect.stringContaining("Badlands")]);
  });

  test("a set the ledger has not says so", async () => {
    serve(() => ({ status: 404, body: { error: "no such set" } }));
    render(<Admin lang="en" section="sets" id="zzzz" />);
    expect((await screen.findByRole("alert")).textContent).toBe("no such set");
  });
});

describe("the overview's services and reports", () => {
  test("say whether each service answers, their builds, and the reports per day", async () => {
    const none = { today_utc: 0, last_7d: 0, last_30d: 0 };
    serve((call) => {
      if (call.url === "/ui/api/admin/stats") {
        return {
          body: {
            at: "2026-10-10T12:00:00Z",
            gateway: { name: "Baylee EU", version: "0.1.0-beta.7 (gw)", registration: "invite", uptime_secs: 7200 },
            accounts: { registered: 1, with_email: 0, confirmed_email: 0, admitted_by_key: 0, created: none },
            guests: { enabled: true, live: 0, cap: null },
            online: { players: 0, in_lobby: 0, seated: 0, sessions_live: 0, accounts_signed_in: 0 },
            games: {
              running: 0,
              local_running: 0,
              waiting: 0,
              seats_awaiting_engine: 0,
              recorded: 0,
              started: none,
              finished: 0,
              finished_since: none,
            },
            agents: { connected: 0, local: 0, capacity: null, games: 0 },
            invites: { total: 0, active: 0, used_up: 0, expired: 0, revoked: 0, uses_left: 0, admitted: 0 },
            daily: [{ day: "2026-10-10", registered: 0, guests: 0, started: 1, finished: 0, players: 2 }],
          },
        };
      }
      if (call.url === "/health") return { body: { ok: true, version: "0.1.0-beta.7 (fb)", source: "x" } };
      if (call.url === "/ui/api/stats") {
        return { body: { days: 30, rows: [{ day: new Date().toISOString().slice(0, 10), kind: "crash", count: 3 }] } };
      }
      return undefined;
    });
    render(<Admin lang="en" />);
    const services = within(await screen.findByRole("region", { name: "Services" }));
    expect((await services.findByTestId("feedback-version")).textContent).toBe("0.1.0-beta.7 (fb)");
    expect(services.getByText(/answered in \d+ ms/)).toBeTruthy();
    expect(services.getByText("0.1.0-beta.7 (gw)")).toBeTruthy();
    const chart = await screen.findByRole("figure", { name: "Reports per day" });
    expect(chart.textContent).toContain("3 in 30 days");
    expect(within(chart).getAllByText("Bugs and crashes").length).toBeGreaterThan(0);
  });
});
