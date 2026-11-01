import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, test, vi } from "vitest";

import { Admin } from "../Admin";
import type { AccountBan } from "../api";
import { serve, type Call } from "../test-utils";
import { AccountActions, EndGame, untilOf, type Target } from "./Moderation";

// Many keystrokes each, on a machine that is often busy.
vi.setConfig({ testTimeout: 20_000 });

const ID = "0199cccc-0000-7000-8000-000000000001";

const alice: Target = { id: ID, handle: "Alice#0001", guest: false };
const BAN: AccountBan = {
  reason: "spam in names",
  until: null,
  at: "2026-10-12T10:00:00Z",
  by: "viktor",
};

const changes = (calls: Call[]) => calls.filter((c) => c.method !== "GET");

/** A service that accepts every act. */
function accepting(): Call[] {
  return serve((call) => {
    if (call.url.endsWith("/ban") && call.method === "PUT") return { body: { ban: { reason: "x", until: null } } };
    if (call.url.endsWith("/close")) return { status: 202 };
    if (call.method === "DELETE") return { status: 204 };
    return undefined;
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the end of a ban", () => {
  test("nothing chosen, or not a moment, is no end", () => {
    expect(untilOf("")).toBeNull();
    expect(untilOf("tomorrow")).toBeNull();
  });

  test("a moment in the browser's zone is said in UTC without fractions", () => {
    const local = new Date(2999, 0, 1, 12, 30);
    expect(untilOf("2999-01-01T12:30")).toBe(local.toISOString().replace(".000Z", "Z"));
    expect(untilOf("2999-01-01T12:30")).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:00Z$/);
  });
});

/** The ban dialog of Alice's actions, open. */
async function open() {
  const calls = accepting();
  const changed = vi.fn<() => void>();
  const user = userEvent.setup();
  render(<AccountActions lang="en" target={alice} onChanged={changed} />);
  await user.click(screen.getByRole("button", { name: "Ban" }));
  const dialog = screen.getByRole("alertdialog");
  return { calls, changed, user, dialog };
}

describe("banning", () => {
  test("a reason is needed, and nothing is sent without one", async () => {
    const { calls, user, dialog } = await open();
    await user.type(within(dialog).getByLabelText("Reason (shown to the player)"), "   ");
    await user.click(within(dialog).getByRole("button", { name: "Ban" }));
    expect((await within(dialog).findByRole("alert")).textContent).toBe("Give a reason.");
    expect(calls).toEqual([]);
  });

  test("an end in the past, or none chosen, is refused", async () => {
    const { calls, user, dialog } = await open();
    await user.type(within(dialog).getByLabelText("Reason (shown to the player)"), "spam");
    // No end chosen yet.
    await user.click(within(dialog).getByRole("button", { name: "Ban" }));
    expect((await within(dialog).findByRole("alert")).textContent).toBe("Choose when the ban ends, in the future.");
    // A moment gone by.
    fireEvent.change(within(dialog).getByLabelText("Until", { selector: "input[type=datetime-local]" }), {
      target: { value: "2020-01-01T10:00" },
    });
    await user.click(within(dialog).getByRole("button", { name: "Ban" }));
    expect((await within(dialog).findByRole("alert")).textContent).toBe("Choose when the ban ends, in the future.");
    expect(calls).toEqual([]);
  });

  test("a ban until a moment sends the reason trimmed and the end in UTC, then reads again", async () => {
    const { calls, changed, user, dialog } = await open();
    await user.type(within(dialog).getByLabelText("Reason (shown to the player)"), "  spam in names  ");
    fireEvent.change(within(dialog).getByLabelText("Until", { selector: "input[type=datetime-local]" }), {
      target: { value: "2999-01-01T12:30" },
    });
    await user.click(within(dialog).getByRole("button", { name: "Ban" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({
        method: "PUT",
        url: `/ui/api/admin/accounts/${ID}/ban`,
        body: {
          reason: "spam in names",
          until: new Date(2999, 0, 1, 12, 30).toISOString().replace(".000Z", "Z"),
        },
      }),
    );
    await waitFor(() => {
      expect(screen.queryByRole("alertdialog")).toBeNull();
    });
    expect(changed).toHaveBeenCalledTimes(1);
  });

  test("a permanent ban sends no end at all", async () => {
    const { calls, user, dialog } = await open();
    await user.type(within(dialog).getByLabelText("Reason (shown to the player)"), "for good");
    await user.click(within(dialog).getByRole("radio", { name: "Permanently" }));
    const end = within(dialog).getByLabelText("Until", { selector: "input[type=datetime-local]" }) as HTMLInputElement;
    expect(end.disabled).toBe(true);
    await user.click(within(dialog).getByRole("button", { name: "Ban" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    const body = changes(calls)[0]?.body as Record<string, unknown>;
    expect(Object.keys(body)).toEqual(["reason"]);
    expect(body["reason"]).toBe("for good");
  });

  test("cancelling sends nothing", async () => {
    const { calls, user, dialog } = await open();
    await user.type(within(dialog).getByLabelText("Reason (shown to the player)"), "spam");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(calls).toEqual([]);
  });

  test("a banned account says so and offers to lift the ban, not to ban again", async () => {
    const calls = accepting();
    const changed = vi.fn<() => void>();
    const user = userEvent.setup();
    render(<AccountActions lang="en" target={{ ...alice, ban: BAN }} onChanged={changed} />);
    expect(screen.getByText("Banned permanently by viktor: spam in names")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Ban" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Lift ban" }));
    const dialog = screen.getByRole("alertdialog");
    expect(dialog.textContent).toContain("Lift the ban on Alice#0001?");
    await user.click(within(dialog).getByRole("button", { name: "Lift ban" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({ method: "DELETE", url: `/ui/api/admin/accounts/${ID}/ban` }),
    );
    expect(changed).toHaveBeenCalledTimes(1);
  });

  test("a ban with an end names it", () => {
    serve(() => undefined);
    render(
      <AccountActions
        lang="en"
        target={{ ...alice, ban: { ...BAN, until: "2999-01-01T12:00:00Z" } }}
        onChanged={() => undefined}
      />,
    );
    const line = screen.getByText(/^Banned until .* by viktor: spam in names$/);
    expect(line.textContent).toContain("2999");
  });
});

describe("deleting an account", () => {
  test("the button wakes only when the handle is typed exactly", async () => {
    const calls = accepting();
    const user = userEvent.setup();
    render(<AccountActions lang="en" target={alice} onChanged={() => undefined} />);
    await user.click(screen.getByRole("button", { name: "Delete account" }));
    const dialog = screen.getByRole("alertdialog");
    const yes = within(dialog).getByRole("button", { name: "Delete for good" }) as HTMLButtonElement;
    const box = within(dialog).getByLabelText("Type Alice#0001 to confirm");
    expect(yes.disabled).toBe(true);
    await user.type(box, "Alice#000");
    expect(yes.disabled).toBe(true);
    await user.clear(box);
    await user.type(box, "alice#0001");
    expect(yes.disabled).toBe(true);
    await user.clear(box);
    await user.type(box, "Alice#0001");
    expect(yes.disabled).toBe(false);
    expect(calls).toEqual([]);

    await user.click(yes);
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({ method: "DELETE", url: `/ui/api/admin/accounts/${ID}` }),
    );
    // Gone: back to the list of accounts.
    await waitFor(() => {
      expect(window.location.pathname).toBe("/admin/accounts");
    });
  });

  test("cancelling deletes nothing", async () => {
    const calls = accepting();
    const user = userEvent.setup();
    render(<AccountActions lang="en" target={alice} onChanged={() => undefined} />);
    await user.click(screen.getByRole("button", { name: "Delete account" }));
    const dialog = screen.getByRole("alertdialog");
    await user.type(within(dialog).getByLabelText("Type Alice#0001 to confirm"), "Alice#0001");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(calls).toEqual([]);
  });
});

describe("ending a game", () => {
  test("asks for a reason in a dialog, and cancelling sends nothing", async () => {
    const calls = accepting();
    const user = userEvent.setup();
    render(<EndGame lang="en" gameId="g1" />);
    await user.click(screen.getByRole("button", { name: "End game" }));
    const dialog = screen.getByRole("alertdialog");
    expect((within(dialog).getByLabelText("Reason (shown to the players)") as HTMLInputElement).value).toBe(
      "Ended by an admin",
    );
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(calls).toEqual([]);
    expect(screen.getByRole("button", { name: "End game" })).toBeTruthy();
  });

  test("posts the reason, then says the engine was told", async () => {
    const calls = accepting();
    const done = vi.fn<() => void>();
    const user = userEvent.setup();
    render(<EndGame lang="en" gameId="g1" onDone={done} />);
    await user.click(screen.getByRole("button", { name: "End game" }));
    const dialog = screen.getByRole("alertdialog");
    const reason = within(dialog).getByLabelText("Reason (shown to the players)");
    await user.clear(reason);
    expect((within(dialog).getByRole("button", { name: "End game" }) as HTMLButtonElement).disabled).toBe(true);
    await user.type(reason, "  a rule dispute ");
    await user.click(within(dialog).getByRole("button", { name: "End game" }));
    await screen.findByText("The engine was told; the table closes in a moment.");
    expect(changes(calls)).toEqual([
      expect.objectContaining({
        method: "POST",
        url: "/ui/api/admin/games/g1/close",
        body: { reason: "a rule dispute" },
      }),
    ]);
    expect(done).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "End game" })).toBeNull();
  });

  test("a refusal stays in the dialog and the game can be tried again", async () => {
    serve(() => ({ status: 409, body: { error: "that table is not being played" } }));
    const user = userEvent.setup();
    render(<EndGame lang="en" gameId="g1" />);
    await user.click(screen.getByRole("button", { name: "End game" }));
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "End game" }));
    expect((await screen.findByRole("alert")).textContent).toBe("that table is not being played");
    expect(screen.getByRole("alertdialog")).toBeTruthy();
  });
});

describe("on the admin pages", () => {
  const row = {
    id: ID,
    username: "alice",
    display_name: "Alice",
    tag: 1,
    handle: "Alice#0001",
    guest: false,
    created_at: "2026-10-01T10:00:00Z",
    has_email: false,
    confirmed: false,
    lang: "de",
    by_key: false,
    key_note: null,
    terms_version: null,
    decks: 0,
    games: 0,
    sessions: 1,
    active_at: null,
    in_lobby: false,
    playing: false,
    online: false,
    terms_accepted_at: null,
    confirmed_at: null,
    table: null,
    deck_list: [],
    recent_games: [],
  };

  test("an account's page shows its ban and reads it again once the ban is lifted", async () => {
    let banned = true;
    const calls = serve((call) => {
      if (call.method === "DELETE") {
        banned = false;
        return { status: 204 };
      }
      if (call.url === `/ui/api/admin/accounts/${ID}`) return { body: { ...row, ban: banned ? BAN : null } };
      return undefined;
    });
    const user = userEvent.setup();
    render(<Admin lang="en" section="accounts" id={ID} />);
    expect(await screen.findByText("Banned permanently by viktor: spam in names")).toBeTruthy();
    expect(screen.getByText("banned")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Lift ban" }));
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Lift ban" }));
    await waitFor(() => {
      expect(screen.queryByText("Banned permanently by viktor: spam in names")).toBeNull();
    });
    expect(screen.getByRole("button", { name: "Ban" })).toBeTruthy();
    expect(calls.filter((c) => c.method === "GET" && c.url === `/ui/api/admin/accounts/${ID}`).length).toBeGreaterThan(1);
  });

  test("live offers End game on a table being played and on none other", async () => {
    const seat = {
      seat: 0,
      kind: "human",
      ai: null,
      account_id: ID,
      player: "Alice#0001",
      guest: false,
      bridge: null,
      bridged_by: null,
      deck: "Woods",
      format: "commander",
      ready: true,
      team: null,
    };
    const table = (id: string, name: string, state: "waiting" | "playing") => ({
      id,
      name,
      state,
      host: "Alice#0001",
      host_id: ID,
      created_at: "2026-10-08T11:55:00Z",
      locked: false,
      rematch: false,
      decide_secs: 180,
      engine: state === "playing",
      engine_local: false,
      agent: null,
      seats: [seat],
    });
    const calls = serve((call) =>
      call.url === "/ui/api/admin/live"
        ? {
            body: {
              at: "2026-10-08T12:00:00Z",
              players: [],
              tables: [table("g1", "Kitchen table", "playing"), table("g2", "Porch", "waiting")],
              agents: [],
            },
          }
        : call.url === "/ui/api/admin/games/g1/close"
          ? { status: 202 }
          : undefined,
    );
    const user = userEvent.setup();
    render(<Admin lang="en" section="live" />);
    const tables = await screen.findAllByTestId("table");
    expect(tables).toHaveLength(2);
    const playing = tables.find((t) => t.textContent?.includes("Kitchen table")) as HTMLElement;
    const waiting = tables.find((t) => t.textContent?.includes("Porch")) as HTMLElement;
    expect(within(waiting).queryByRole("button", { name: "End game" })).toBeNull();
    await user.click(within(playing).getByRole("button", { name: "End game" }));
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "End game" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({
        method: "POST",
        url: "/ui/api/admin/games/g1/close",
        body: { reason: "Ended by an admin" },
      }),
    );
  });
});
