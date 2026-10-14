import { render, screen, waitFor, within, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import { Admin } from "../Admin";
import type { CardBlock, CardHit } from "../api";
import { adminPath, parseRoute } from "../router";
import { serve, type Call } from "../test-utils";

// Typing through user-event in jsdom is slow, and a loaded machine slower:
// these tests take up to a few seconds each, which the default five would cut.
vi.setConfig({ testTimeout: 20_000 });

const BOLT_ID = "e3285e6b-3e79-4d7c-bf96-d920f973b80b";

function block(over: Partial<CardBlock> = {}): CardBlock {
  return {
    id: "b1",
    index: 160,
    name: "Lightning Bolt",
    reason: "crashes the engine",
    created_at: "2026-10-10T12:00:00Z",
    created_by: "viktor",
    until: null,
    lifted_at: null,
    lifted_by: null,
    state: "active",
    ...over,
  };
}

const BOLT_BLOCK = block();
const COUNTER_BLOCK = block({
  id: "b2",
  index: 77,
  name: "Counterspell",
  reason: "loops forever",
  created_by: "anna",
  until: "2027-01-02T10:00:00Z",
});
const OLD_BLOCK = block({
  id: "b3",
  index: 12,
  name: "Ancestral Recall",
  reason: "old trouble",
  state: "expired",
  until: "2026-09-01T10:00:00Z",
});
const LIFTED_BLOCK = block({
  id: "b4",
  index: 13,
  name: "Black Lotus",
  reason: "fixed meanwhile",
  state: "lifted",
  lifted_at: "2026-09-20T10:00:00Z",
  lifted_by: "anna",
});

const BOLT_HIT: CardHit = {
  index: 160,
  name: "Lightning Bolt",
  type_line: "Instant",
  mana_cost: "{R}",
  coverage: "implemented",
  scryfall_id: BOLT_ID,
  blocked: null,
};
const SHOCK_HIT: CardHit = {
  index: 200,
  name: "Shock",
  type_line: "Instant",
  mana_cost: "{R}",
  coverage: "generated",
  scryfall_id: "",
  blocked: null,
};

/** A gateway that keeps its blocks; every call is also in `calls`. */
function site(initial: CardBlock[] = [BOLT_BLOCK, COUNTER_BLOCK, OLD_BLOCK, LIFTED_BLOCK], hits: CardHit[] = [BOLT_HIT, SHOCK_HIT]) {
  let blocks = [...initial];
  const calls = serve((call) => {
    if (call.url === "/ui/api/admin/cards/blocks" && call.method === "GET") return { body: blocks };
    if (call.url === "/ui/api/admin/cards/blocks" && call.method === "POST") {
      const order = call.body as { index: number; reason: string };
      const made = block({ id: "new", index: order.index, name: hits.find((h) => h.index === order.index)?.name ?? "?", reason: order.reason });
      blocks = [made, ...blocks];
      return { status: 201, body: made };
    }
    if (call.url.startsWith("/ui/api/admin/cards/blocks/") && call.method === "DELETE") {
      const id = call.url.slice("/ui/api/admin/cards/blocks/".length);
      blocks = blocks.map((b) => (b.id === id ? { ...b, state: "lifted" as const, lifted_at: "2026-10-11T10:00:00Z", lifted_by: "viktor" } : b));
      return { status: 204 };
    }
    if (call.url.startsWith("/ui/api/admin/cards/search?q=")) return { body: { cards: hits } };
    if (call.url === "/ui/api/admin/audit") {
      return {
        body: [
          { at: "2026-10-10T12:00:00Z", actor: "viktor", action: "gateway.card.block", detail: "160" },
          { at: "2026-10-10T11:00:00Z", actor: "viktor", action: "gateway.invite.create", detail: "k1" },
        ],
      };
    }
    return undefined;
  });
  return calls;
}

const changes = (calls: Call[]) => calls.filter((c) => c.method !== "GET");
const searches = (calls: Call[]) => calls.filter((c) => c.url.startsWith("/ui/api/admin/cards/search"));
const blockReads = (calls: Call[]) => calls.filter((c) => c.method === "GET" && c.url === "/ui/api/admin/cards/blocks");

/** Types a name into the search and waits for its hits. */
async function find(user: ReturnType<typeof userEvent.setup>, text: string) {
  await user.type(await screen.findByRole("searchbox", { name: "Card name or index" }), text);
  await screen.findAllByTestId("card-hit");
}

const hit = (name: string) => {
  const found = screen.getAllByTestId("card-hit").find((h) => h.textContent?.includes(name));
  if (found === undefined) throw new Error(`no hit for ${name}`);
  return within(found);
};

describe("the route and the section bar", () => {
  test("/admin/cards is the cards section", () => {
    expect(parseRoute("/admin/cards", "")).toEqual({ page: "admin", section: "cards" });
    expect(parseRoute("/admin/cards/", "")).toEqual({ page: "admin", section: "cards" });
    expect(adminPath("cards")).toBe("/admin/cards");
  });

  test("the section bar links to the blocked cards", async () => {
    site();
    render(<Admin lang="en" section="cards" />);
    const link = screen.getByRole("link", { name: "Blocked cards" });
    expect(link.getAttribute("href")).toBe("/admin/cards");
    expect(link.getAttribute("aria-current")).toBe("page");
    expect(await screen.findByRole("heading", { level: 1, name: "Blocked cards" })).toBeTruthy();
  });
});

describe("the blocks in force and the earlier ones", () => {
  test("active blocks show name, reason, end and by; earlier ones are folded away", async () => {
    site();
    render(<Admin lang="en" section="cards" />);
    const items = await screen.findAllByTestId("card-block");
    expect(items).toHaveLength(4);
    const bolt = items[0] as HTMLElement;
    expect(bolt.textContent).toContain("Lightning Bolt");
    expect(bolt.textContent).toContain("crashes the engine");
    expect(bolt.textContent).toContain("Endsnever");
    expect(bolt.textContent).toContain("Byviktor");
    expect(bolt.textContent).toContain("in force");
    const counter = items[1] as HTMLElement;
    expect(counter.textContent).toContain("Counterspell");
    expect(counter.textContent).toContain("loops forever");
    expect(counter.textContent).not.toContain("never");
    expect(counter.textContent).toContain("Byanna");
    expect(screen.getByText("2 blocked now.", { exact: false })).toBeTruthy();

    const history = screen.getByText("Earlier blocks").closest("details");
    expect(history).not.toBeNull();
    expect(history?.hasAttribute("open")).toBe(false);
    const earlier = within(history as HTMLElement).getAllByTestId("card-block");
    expect(earlier.map((e) => e.textContent)).toEqual([
      expect.stringContaining("Ancestral Recall"),
      expect.stringContaining("Black Lotus"),
    ]);
    expect(earlier[0]?.textContent).toContain("ended");
    expect(earlier[1]?.textContent).toContain("lifted");
    expect(earlier[1]?.textContent).toContain("Lifted");
    // Only a block in force can be lifted.
    expect(within(earlier[0] as HTMLElement).queryByRole("button")).toBeNull();
    expect(within(earlier[1] as HTMLElement).queryByRole("button")).toBeNull();
    expect(screen.getAllByRole("button", { name: /^Unblock…/ })).toHaveLength(2);
  });

  test("with no blocks it says so", async () => {
    site([]);
    render(<Admin lang="en" section="cards" />);
    expect(await screen.findByText("No card is blocked.")).toBeTruthy();
    expect(screen.getByText("None yet.")).toBeTruthy();
    expect(screen.queryAllByTestId("card-block")).toHaveLength(0);
  });

  test("only the card changes of the console's log are shown", async () => {
    site();
    render(<Admin lang="en" section="cards" />);
    expect(await screen.findByText("gateway.card.block")).toBeTruthy();
    expect(screen.queryByText("gateway.invite.create")).toBeNull();
  });
});

describe("the search", () => {
  test("asks after a pause, with the typed text encoded, and lists the hits", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await screen.findAllByTestId("card-block");
    expect(searches(calls)).toHaveLength(0);
    await find(user, "Lightning & Bolt");
    // One question for the whole typing, not one per key.
    expect(searches(calls).map((c) => c.url)).toEqual([`/ui/api/admin/cards/search?q=${encodeURIComponent("Lightning & Bolt")}`]);
    expect(screen.getAllByTestId("card-hit")).toHaveLength(2);
    expect(hit("Shock").getByText("Instant")).toBeTruthy();
    expect(hit("Shock").getByText("#200")).toBeTruthy();
  });

  test("a search without a hit says so", async () => {
    site([], []);
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await user.type(await screen.findByRole("searchbox", { name: "Card name or index" }), "zzz");
    expect(await screen.findByText("No card of this gateway's pool matches.")).toBeTruthy();
  });

  test("a card blocked in the list shows the badge and its reason, not a Block button", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await screen.findAllByTestId("card-block");
    await find(user, "bolt");
    const bolt = hit("Lightning Bolt");
    expect(bolt.getByText("blocked")).toBeTruthy();
    expect(bolt.getByText("crashes the engine")).toBeTruthy();
    expect(bolt.queryByRole("button")).toBeNull();
    const shock = hit("Shock");
    expect(shock.queryByText("blocked")).toBeNull();
    expect(shock.getByRole("button", { name: "Block… Shock" })).toBeTruthy();
  });

  test("a card blocked with an end shows when it ends in the badge", async () => {
    site([COUNTER_BLOCK], [{ ...SHOCK_HIT, index: 77, name: "Counterspell" }]);
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await screen.findAllByTestId("card-block");
    await find(user, "counter");
    expect(hit("Counterspell").getByText(/^blocked · /)).toBeTruthy();
  });
});

/** Finds Shock and opens its block form. */
async function openForm(user: ReturnType<typeof userEvent.setup>) {
  await screen.findAllByTestId("card-block");
  await find(user, "shock");
  await user.click(hit("Shock").getByRole("button", { name: "Block… Shock" }));
  return within(screen.getByRole("form", { name: "Block… Shock" }));
}

describe("blocking a card", () => {
  test("is impossible without a reason", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    const submit = form.getByRole("button", { name: "Block…" });
    expect((submit as HTMLButtonElement).disabled).toBe(true);
    await user.type(form.getByLabelText("Reason (players see it)"), "   ");
    expect((submit as HTMLButtonElement).disabled).toBe(true);
    await user.type(form.getByLabelText("Reason (players see it)"), "bad");
    expect((submit as HTMLButtonElement).disabled).toBe(false);
  });

  test("without an end asks first, then sends the index and the reason and no end", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    await user.type(form.getByLabelText("Reason (players see it)"), "  deals too much  ");
    expect((form.getByLabelText("How long") as HTMLSelectElement).value).toBe("0");
    await user.click(form.getByRole("button", { name: "Block…" }));
    expect(changes(calls)).toEqual([]);
    const legend = screen.getByText(/^Block Shock without an end\?/);
    expect(legend.closest("fieldset")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Block it" }));
    await screen.findByText("Shock is blocked.");
    const sent = changes(calls);
    expect(sent).toHaveLength(1);
    expect(sent[0]).toEqual(
      expect.objectContaining({ method: "POST", url: "/ui/api/admin/cards/blocks", body: { index: 200, reason: "deals too much" } }),
    );
    expect(sent[0]?.headers["x-baylee-csrf"]).toBe("1");
    expect(Object.keys(sent[0]?.body as object)).not.toContain("until");
    // The list was asked again, and the hit now carries the badge and the reason.
    await waitFor(() => {
      expect(blockReads(calls).length).toBeGreaterThanOrEqual(2);
    });
    await waitFor(() => {
      expect(hit("Shock").getByText("blocked")).toBeTruthy();
    });
    expect(hit("Shock").getByText("deals too much")).toBeTruthy();
    expect(screen.queryByRole("form", { name: "Block… Shock" })).toBeNull();
    expect(screen.getAllByTestId("card-block")).toHaveLength(5);
  });

  test("Cancel in the confirmation sends nothing and leaves the form", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    await user.type(form.getByLabelText("Reason (players see it)"), "bad");
    await user.click(form.getByRole("button", { name: "Block…" }));
    await user.click(within(screen.getByText(/^Block Shock/).closest("fieldset") as HTMLElement).getByRole("button", { name: "Cancel" }));
    expect(changes(calls)).toEqual([]);
    expect(screen.queryByRole("button", { name: "Block it" })).toBeNull();
    // The form is open again with the reason kept.
    expect((screen.getByLabelText("Reason (players see it)") as HTMLInputElement).value).toBe("bad");
  });

  test("7 days sends an ISO end about a week ahead and the confirmation names it", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    await user.type(form.getByLabelText("Reason (players see it)"), "week");
    await user.selectOptions(form.getByLabelText("How long"), "7 days");
    const before = Date.now();
    await user.click(form.getByRole("button", { name: "Block…" }));
    expect(screen.getByText(/^Block Shock until /).textContent).not.toContain("without an end");
    await user.click(screen.getByRole("button", { name: "Block it" }));
    await screen.findByText("Shock is blocked.");
    const body = changes(calls)[0]?.body as { index: number; reason: string; until: string };
    expect(body.index).toBe(200);
    expect(body.reason).toBe("week");
    expect(body.until).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/);
    const ahead = new Date(body.until).getTime() - before;
    const week = 7 * 24 * 3600 * 1000;
    expect(ahead).toBeGreaterThan(week - 60_000);
    expect(ahead).toBeLessThan(week + 60_000);
  });

  test("a moment in the past is refused on the page, before anything is sent", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    await user.type(form.getByLabelText("Reason (players see it)"), "soon");
    await user.selectOptions(form.getByLabelText("How long"), "Until a moment…");
    fireEvent.change(form.getByLabelText("Until (your time)"), { target: { value: "2020-01-01T10:00" } });
    await user.click(form.getByRole("button", { name: "Block…" }));
    expect((await screen.findByRole("alert")).textContent).toBe("Pick a moment in the future.");
    expect(screen.queryByRole("button", { name: "Block it" })).toBeNull();
    expect(changes(calls)).toEqual([]);
  });

  test("a moment in the future is sent as it was picked", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    const form = await openForm(user);
    await user.type(form.getByLabelText("Reason (players see it)"), "later");
    await user.selectOptions(form.getByLabelText("How long"), "Until a moment…");
    const year = new Date().getFullYear() + 1;
    fireEvent.change(form.getByLabelText("Until (your time)"), { target: { value: `${year}-03-04T10:30` } });
    await user.click(form.getByRole("button", { name: "Block…" }));
    await user.click(await screen.findByRole("button", { name: "Block it" }));
    await screen.findByText("Shock is blocked.");
    const body = changes(calls)[0]?.body as { until: string };
    expect(new Date(body.until).getTime()).toBe(new Date(year, 2, 4, 10, 30).getTime());
  });

  test("a refusal of the gateway is shown as it came", async () => {
    const calls = serve((call) => {
      if (call.url === "/ui/api/admin/cards/blocks" && call.method === "GET") return { body: [] };
      if (call.url === "/ui/api/admin/cards/blocks" && call.method === "POST") {
        return { status: 409, body: { error: "Lightning Bolt is blocked already; lift that block first" } };
      }
      if (call.url.startsWith("/ui/api/admin/cards/search")) return { body: { cards: [BOLT_HIT] } };
      if (call.url === "/ui/api/admin/audit") return { body: [] };
      return undefined;
    });
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await screen.findByText("No card is blocked.");
    await find(user, "bolt");
    await user.click(hit("Lightning Bolt").getByRole("button", { name: "Block… Lightning Bolt" }));
    await user.type(screen.getByLabelText("Reason (players see it)"), "x");
    await user.click(screen.getByRole("button", { name: "Block…" }));
    await user.click(screen.getByRole("button", { name: "Block it" }));
    expect((await screen.findByRole("alert")).textContent).toBe("Lightning Bolt is blocked already; lift that block first");
    expect(changes(calls)).toHaveLength(1);
    // The confirmation is gone, so the person can read the refusal and try again.
    expect(screen.queryByRole("button", { name: "Block it" })).toBeNull();
    expect(screen.queryByText("Lightning Bolt is blocked.")).toBeNull();
  });
});

describe("lifting a block", () => {
  test("asks first, naming the card; Keep it sends nothing", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await user.click(await screen.findByRole("button", { name: "Unblock… Lightning Bolt" }));
    const legend = screen.getByText(/^Lift the block on Lightning Bolt\?/);
    const confirm = within(legend.closest("fieldset") as HTMLElement);
    await user.click(confirm.getByRole("button", { name: "Keep it" }));
    expect(changes(calls)).toEqual([]);
    expect(screen.queryByText(/^Lift the block on/)).toBeNull();
    expect(screen.getByRole("button", { name: "Unblock… Lightning Bolt" })).toBeTruthy();
  });

  test("Lift the block sends the delete with the CSRF header and asks the list again", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await user.click(await screen.findByRole("button", { name: "Unblock… Lightning Bolt" }));
    const readsBefore = blockReads(calls).length;
    await user.click(screen.getByRole("button", { name: "Lift the block" }));
    await screen.findByText("Block lifted.");
    const sent = changes(calls);
    expect(sent).toHaveLength(1);
    expect(sent[0]).toEqual(expect.objectContaining({ method: "DELETE", url: "/ui/api/admin/cards/blocks/b1" }));
    expect(sent[0]?.headers["x-baylee-csrf"]).toBe("1");
    await waitFor(() => {
      expect(blockReads(calls).length).toBeGreaterThan(readsBefore);
    });
    // The block moved to the earlier ones; one is left in force.
    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /^Unblock…/ })).toHaveLength(1);
    });
    expect(screen.getByText("1 blocked now.", { exact: false })).toBeTruthy();
  });

  test("a refusal while lifting is shown", async () => {
    serve((call) => {
      if (call.url === "/ui/api/admin/cards/blocks" && call.method === "GET") return { body: [BOLT_BLOCK] };
      if (call.method === "DELETE") return { status: 404, body: { error: "no such block" } };
      if (call.url === "/ui/api/admin/audit") return { body: [] };
      return undefined;
    });
    const user = userEvent.setup();
    render(<Admin lang="en" section="cards" />);
    await user.click(await screen.findByRole("button", { name: "Unblock… Lightning Bolt" }));
    await user.click(screen.getByRole("button", { name: "Lift the block" }));
    expect((await screen.findByRole("alert")).textContent).toBe("no such block");
  });
});

describe("in German", () => {
  test("the labels are German", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="de" section="cards" />);
    expect(await screen.findByRole("heading", { level: 1, name: "Gesperrte Karten" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Gesperrte Karten" }).getAttribute("href")).toBe("/admin/cards");
    expect(screen.getByText("In Kraft")).toBeTruthy();
    expect(screen.getByText("Frühere Sperren")).toBeTruthy();
    expect(screen.getAllByText("in Kraft").length).toBeGreaterThan(0);
    expect(screen.getAllByText("nie").length).toBeGreaterThan(0);
    await user.type(screen.getByRole("searchbox", { name: "Kartenname oder Index" }), "shock");
    await screen.findAllByTestId("card-hit");
    await user.click(screen.getByRole("button", { name: "Sperren… Shock" }));
    await user.type(screen.getByLabelText("Grund (Spieler sehen ihn)"), "x");
    await user.click(screen.getByRole("button", { name: "Sperren…" }));
    expect(screen.getByText(/^Shock ohne Ende sperren\?/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Sperren" })).toBeTruthy();
  });
});
