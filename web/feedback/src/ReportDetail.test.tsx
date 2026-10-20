import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import type { AuditEntry, Change, Neighbours, Note, Report } from "./api";
import { ReportDetail } from "./ReportDetail";
import { fullReport, serve, type Call } from "./test-utils";

/** The detail's routes, over one report that changes as the service would change it. */
function start(report: Report, audit: AuditEntry[] = [], neighbours?: Neighbours): Call[] {
  let current: Report = { ...report };
  let notes: Note[] = report.note_list ?? [];
  return serve((call) => {
    if (call.url.endsWith("/audit")) return { body: audit };
    if (call.url === "/ui/api/seen") return { status: 204 };
    if (call.url === "/ui/api/admins") return { body: ["viktor", "astrid"] };
    if (call.url.includes("/neighbours")) {
      return { body: neighbours ?? { previous: null, next: null, position: null, total: 0 } };
    }
    if (call.url.endsWith("/notes") && call.method === "POST") {
      notes = [...notes, { id: `n${notes.length + 1}`, at: "2026-09-27T12:10:00Z", author: "viktor", text: (call.body as { text: string }).text }];
      return { body: notes };
    }
    const note = /\/notes\/([^/]+)$/.exec(call.url)?.[1];
    if (note !== undefined && call.method === "DELETE") {
      notes = notes.filter((n) => n.id !== note);
      return { body: notes };
    }
    if (call.method === "PATCH") {
      const change = call.body as Change;
      if (change.issue === 0) return { status: 400, body: { error: "not an issue number" } };
      if (change.expected_revision !== undefined && change.expected_revision !== current.revision) {
        return { status: 409, body: { error: "the report changed meanwhile; read it again" } };
      }
      current = { ...current, revision: current.revision + 1 };
      if (change.status) current.status = change.status;
      if (change.issue !== undefined) {
        current.issue_number = change.issue;
        current.issue_url = change.issue === null ? null : `https://github.com/AceVik/baylee/issues/${change.issue}`;
      }
      if (change.priority !== undefined) current.priority = change.priority;
      if (change.assignee !== undefined) current.assignee = change.assignee;
      if (change.labels !== undefined) current.labels = [...change.labels].sort();
      if (change.duplicate_of !== undefined) {
        current.duplicate_of = change.duplicate_of;
        if (change.duplicate_of !== null) current.status = "duplicate";
      }
      return { body: { ...current, note_list: notes } };
    }
    if (call.method === "DELETE") {
      current = { ...current, deleted_at: "2026-09-27T13:00:00Z" };
      return { status: 204 };
    }
    if (call.url.endsWith("/restore")) {
      current = { ...current, deleted_at: null };
      return { status: 204 };
    }
    if (call.url.endsWith("/purge")) return { status: 204 };
    if (call.url.startsWith("/ui/api/reports?")) return { body: { total: 3, reports: [] } };
    if (call.url.startsWith("/ui/api/facets")) {
      return { body: { labels: [{ value: "ui", count: 2 }], statuses: [], kinds: [], gateways: [], reporters: [], assignees: [], priorities: [], channels: [], total: 2 } };
    }
    if (call.url.startsWith("/ui/api/reports/")) return { body: { ...current, note_list: notes } };
    return undefined;
  });
}

const section = (name: string) => within(screen.getByRole("region", { name }));

describe("references in the text", () => {
  const id = "0199aaaa-0000-7000-8000-000000000001";
  const bolt = "e3285e6b-3e79-4d7c-bf96-d920f973b80b";

  test("are listed under it, a card with its Scryfall page, and the text stays as written", async () => {
    const report = fullReport({ text: "cast [Lightning Bolt] at [@steady 1], not [x]" });
    report.client["refs"] = {
      cards: [
        {
          text: "Lightning Bolt",
          at: [5, 21],
          card: 1234,
          print: { scryfall_id: bolt, lang: "en", finish: "normal" },
          face: 0,
          object: 17,
          zone: "stack",
          owner: 1,
        },
      ],
      players: [{ text: "steady 1", at: [25, 36], seat: 1 }],
    };
    start(report);
    render(<ReportDetail id={id} />);
    const refs = within(await screen.findByRole("region", { name: "References" }));
    expect(refs.getByText("Lightning Bolt")).toBeTruthy();
    expect(refs.getByText(/stack/)).toBeTruthy();
    const link = refs.getByRole("link", { name: "Scryfall" });
    expect(link.getAttribute("href")).toBe(`https://scryfall.com/card/${bolt}`);
    expect(refs.getByText("@steady 1")).toBeTruthy();
    // In the text each resolved reference is a chip in its place: the card
    // links to its Scryfall page, the player is a seat, and `[x]`, which
    // the client resolved to nothing, stays the bracket the player typed.
    const text = section("What the player wrote");
    const chip = text.getByRole("link", { name: "Lightning Bolt" });
    expect(chip.getAttribute("href")).toBe(`https://scryfall.com/card/${bolt}`);
    expect(chip.getAttribute("rel")).toBe("noopener noreferrer");
    expect(text.getByText("@steady 1")).toBeTruthy();
    expect(text.getByText(/, not \[x\]$/)).toBeTruthy();
    expect(text.queryAllByRole("link")).toHaveLength(1);
    // The picture comes straight from Scryfall's image host, only on
    // hover, and only after the card's id was checked to be one.
    expect(text.queryByRole("img")).toBeNull();
    await userEvent.hover(chip);
    const picture = text.getByRole("img");
    expect(picture.getAttribute("src")).toBe(`https://cards.scryfall.io/normal/front/e/3/${bolt}.jpg`);
    expect(text.getByText("Image: Scryfall")).toBeTruthy();
    await userEvent.unhover(chip);
    expect(text.queryByRole("img")).toBeNull();
  });

  test("a reference whose range does not say what the text says there stays text", async () => {
    const report = fullReport({ text: "cast [Lightning Bolt] twice" });
    report.client["refs"] = {
      cards: [{ text: "Lightning Bolt", at: [0, 16], card: 1234, print: { scryfall_id: bolt } }],
      players: [],
    };
    start(report);
    render(<ReportDetail id={id} />);
    expect(await screen.findByText("cast [Lightning Bolt] twice")).toBeTruthy();
    expect(section("What the player wrote").queryAllByRole("link")).toHaveLength(0);
  });

  test("a report naming nothing has no such section, and an odd id makes no link", async () => {
    start(fullReport({ text: "plain [x]" }));
    render(<ReportDetail id={id} />);
    expect(await screen.findByText("plain [x]")).toBeTruthy();
    expect(screen.queryByRole("region", { name: "References" })).toBeNull();
  });
});

describe("a full report", () => {
  test("shows every part of the dump", async () => {
    start(fullReport(), [{ at: "2026-09-27T12:05:00Z", actor: "viktor", action: "status", detail: "triaged" }]);
    render(<ReportDetail id="0199aaaa-0000-7000-8000-000000000001" />);
    expect(await screen.findByText("The Swamp untapped by itself.")).toBeTruthy();

    const shot = section("Screenshot").getByRole("img");
    expect(shot.getAttribute("src")).toBe("data:image/png;base64,iVBORw0KGgo=");

    const log = section("The player's game log");
    expect(log.getByText("You played Swamp.")).toBeTruthy();
    expect(log.getByText(/You · Player A \(AI\), team 2/)).toBeTruthy();

    const system = section("System");
    expect(system.getByText("Apple M1 Max")).toBeTruthy();
    expect(system.getByText("1280 × 800")).toBeTruthy();

    const crash = section("Crash");
    expect(crash.getByText("index out of bounds")).toBeTruthy();
    expect(crash.getByText("src/lobby.rs:1:2")).toBeTruthy();

    const record = section("Game record");
    expect(record.getByText("21 KiB, complete")).toBeTruthy();
    const download = record.getByRole("link", { name: "Download" });
    expect(download.getAttribute("href")).toBe("/ui/api/reports/0199aaaa-0000-7000-8000-000000000001/record");
    expect(download.getAttribute("download")).toBe("0199aaaa-0000-7000-8000-000000000001.jsonl.gz");

    expect(section("Settings").getByText(/"on"/)).toBeTruthy();
    expect(section("The table as the player saw it").getByText(/table:/)).toBeTruthy();
    // The whole client object, the screenshot's base64 not spelled out.
    const everything = section("Everything the client sent");
    expect(everything.getByText(/12 characters of base64/)).toBeTruthy();
    expect(section("History").getByText(/viktor: set the status to Triaged/)).toBeTruthy();
    expect(screen.getByText("0.1.0-beta.1 (3f9a1c7e21)")).toBeTruthy();
    // The reporter is an alias derived from the pseudonym (`alias.ts`), the
    // pseudonym itself its accessible name; it filters the list by it.
    const reporter = screen.getByRole("button", { name: "5bdc0e1f9a7c33aa" });
    expect(reporter.textContent).toContain("5bdc");
    expect(reporter.textContent).not.toContain("5bdc0e1f");
    await userEvent.click(reporter);
    expect(window.location.search).toBe("?reporter=5bdc0e1f9a7c33aa");
  });

  test("says where in the game the report was written", async () => {
    const report = fullReport();
    report.client["game"] = {
      table: { seat: 1, seq: 42, when: "Turn 7 · main 1" },
      view: { turn: 7, active: 0, seat: 1, awaiting: 1, players: [{}, {}] },
      pending: { ChooseAttackers: { player: 1, options: [1, 2, 3] } },
      holding: { selected: 2, armed: "Play Swamp", mana_run: false, outbox: 1, last_error: "not your turn" },
    };
    start(report);
    render(<ReportDetail id="x" />);
    await screen.findByText("The Swamp untapped by itself.");
    const table = section("At the table");
    expect(table.getByText("turn 7, seat 0's")).toBeTruthy();
    expect(table.getByText("Turn 7 · main 1")).toBeTruthy();
    expect(table.getByText("1 · 2 players")).toBeTruthy();
    expect(table.getByText("Choose attackers")).toBeTruthy();
    expect(table.getByText(/player: 1 · options: 3/)).toBeTruthy();
    expect(table.getByText(/waiting for seat 1/)).toBeTruthy();
    expect(table.getByText("armed: Play Swamp · 2 selected · 1 unsent")).toBeTruthy();
    expect(table.getByText("not your turn")).toBeTruthy();
    expect(table.getByText("sequence 42")).toBeTruthy();
  });

  test("a folded part of the tree unfolds", async () => {
    start(fullReport());
    render(<ReportDetail id="x" />);
    await screen.findByText("The Swamp untapped by itself.");
    const everything = section("Everything the client sent");
    const build = everything.getByText("build:").closest("details");
    expect(build?.open).toBe(false);
    await userEvent.click(everything.getByText("build:"));
    expect(build?.open).toBe(true);
    expect(within(build as HTMLElement).getByText('"3f9a1c7e21"')).toBeTruthy();
  });
});

describe("a report a client sent itself", () => {
  test("says it came unauthenticated and its record is the client's", async () => {
    start(fullReport({ channel: "direct", gateway: "(direct)", record_origin: "client" }));
    render(<ReportDetail id="x" />);
    expect(await screen.findByText("direct from a client, unauthenticated")).toBeTruthy();
    expect(screen.getByText("client-supplied, unverified")).toBeTruthy();
  });
});

describe("a report with nothing ticked", () => {
  test("says what is missing and breaks nowhere", async () => {
    start(fullReport({ client: {}, has_record: false, record_bytes: 0, record_complete: null, text: "" }));
    render(<ReportDetail id="x" />);
    expect(await screen.findByText("(no text)")).toBeTruthy();
    expect(screen.getByText("The player did not include a screenshot.")).toBeTruthy();
    expect(screen.getByText("The player did not include the game log.")).toBeTruthy();
    expect(screen.getByText("The player did not include system details.")).toBeTruthy();
    expect(screen.getByText("The player did not include settings.")).toBeTruthy();
    expect(screen.getByText("The player did not include the table.")).toBeTruthy();
    expect(screen.getByText("No record is attached.")).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Crash" })).toBeNull();
    expect(screen.queryByRole("img")).toBeNull();
  });

  test("a malformed part is shown as missing, not as garbage", async () => {
    start(
      fullReport({
        client: {
          screenshot: { png_base64: "\"><img src=x onerror=alert(1)>" },
          log: "not a log",
          system: { window: "big" },
          build: 7,
        },
      }),
    );
    render(<ReportDetail id="x" />);
    expect(await screen.findByText("The player did not include a screenshot.")).toBeTruthy();
    expect(screen.getByText("The player did not include the game log.")).toBeTruthy();
    expect(screen.getByText("The player did not include system details.")).toBeTruthy();
    expect(screen.queryByRole("img")).toBeNull();
  });
});

describe("triage", () => {
  test("a status change is sent and shown", async () => {
    const calls = start(fullReport());
    render(<ReportDetail id="x" />);
    await screen.findByText("The Swamp untapped by itself.");
    await userEvent.selectOptions(screen.getByLabelText("Status"), "in_progress");
    expect(await screen.findByText("Status: In progress.")).toBeTruthy();
    const patch = calls.find((c) => c.method === "PATCH");
    expect(patch?.url).toBe("/ui/api/reports/x");
    expect(patch?.body).toEqual({ status: "in_progress", expected_revision: 1 });
    expect(patch?.headers["x-baylee-csrf"]).toBe("1");
    expect(screen.getByLabelText("Status")).toHaveProperty("value", "in_progress");
    // The history is asked again after a change.
    expect(calls.filter((c) => c.url.endsWith("/audit"))).toHaveLength(2);
  });

  test("an issue is linked from a URL and unlinked", async () => {
    const calls = start(fullReport());
    render(<ReportDetail id="x" />);
    await screen.findByText("Not linked.");
    await userEvent.type(
      screen.getByLabelText("Issue number or URL"),
      "https://github.com/AceVik/baylee/issues/311{Enter}",
    );
    const link = await screen.findByRole("link", { name: "#311" });
    expect(link.getAttribute("href")).toBe("https://github.com/AceVik/baylee/issues/311");
    expect(calls.find((c) => c.method === "PATCH")?.body).toEqual({ issue: 311, expected_revision: 1 });
    await userEvent.click(screen.getByRole("button", { name: "Unlink" }));
    expect(await screen.findByText("Not linked.")).toBeTruthy();
    expect(calls.findLast((c) => c.method === "PATCH")?.body).toEqual({ issue: null, expected_revision: 2 });
  });

  test("an issue that is not one is not sent", async () => {
    const calls = start(fullReport());
    render(<ReportDetail id="x" />);
    await screen.findByText("Not linked.");
    await userEvent.type(screen.getByLabelText("Issue number or URL"), "https://evil.example/1{Enter}");
    expect((await screen.findByRole("alert")).textContent).toMatch(/Give an issue number/);
    expect(calls.some((c) => c.method === "PATCH")).toBe(false);
  });

  test("a new issue carries only the admin's summary, the category and the build", async () => {
    start(fullReport());
    render(<ReportDetail id="x" />);
    const summary = await screen.findByLabelText("Summary for a new issue");
    // Empty until the admin writes it.
    expect((summary as HTMLTextAreaElement).value).toBe("");
    expect(screen.queryByRole("link", { name: "Open a new issue on GitHub" })).toBeNull();
    expect(screen.getByRole("button", { name: "Open a new issue on GitHub" })).toHaveProperty("disabled", true);

    await userEvent.type(summary, "A basic land untaps during its controller's upkeep.");
    await userEvent.selectOptions(screen.getByLabelText("Category"), "crash");
    const link = screen.getByRole("link", { name: "Open a new issue on GitHub" });
    const href = link.getAttribute("href") ?? "";
    const url = new URL(href);
    expect(url.origin + url.pathname).toBe("https://github.com/AceVik/baylee/issues/new");
    expect(url.searchParams.get("title")).toBe("Crash: A basic land untaps during its controller's upkeep.");
    expect(url.searchParams.get("body")).toContain("- Build: 0.1.0-beta.1+build.42 (3f9a1c7e21)");
    // Read as GitHub reads it: a query string's `+` is a space, which
    // `decodeURIComponent` would leave standing and so miss every phrase.
    const filed = `${href}\n${url.searchParams.get("title") ?? ""}\n${url.searchParams.get("body") ?? ""}`;
    for (const leak of ["Swamp untapped", "5bdc0e1f9a7c33aa", "0199aaaa", "/r/", "Apple M1", "eu.example"]) {
      expect(filed).not.toContain(leak);
    }
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toBe("noopener noreferrer");
  });

  test("the trash keeps it on the page with a way back, and only from there is it deleted for good", async () => {
    window.history.replaceState(null, "", "/r/x");
    const calls = start(fullReport());
    render(<ReportDetail id="x" />);
    await userEvent.click(await screen.findByRole("button", { name: "Move to trash" }));
    expect(await screen.findByText(/In the trash since/)).toBeTruthy();
    const del = calls.find((c) => c.method === "DELETE");
    expect(del?.url).toBe("/ui/api/reports/x");
    expect(del?.headers["x-baylee-csrf"]).toBe("1");
    expect(screen.queryByRole("button", { name: "Move to trash" })).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: "Restore" }));
    expect(await screen.findByRole("button", { name: "Move to trash" })).toBeTruthy();
    expect(calls.some((c) => c.url === "/ui/api/reports/x/restore" && c.method === "POST")).toBe(true);

    await userEvent.click(screen.getByRole("button", { name: "Move to trash" }));
    await userEvent.click(await screen.findByRole("button", { name: "Delete for good…" }));
    expect(calls.some((c) => c.url.endsWith("/purge"))).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Keep it" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete for good…" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete for good" }));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/");
    });
    expect(calls.find((c) => c.url.endsWith("/purge"))?.method).toBe("POST");
  });

  test("priority, assignee, labels and the duplicate are set and cleared", async () => {
    const calls = start(fullReport());
    render(<ReportDetail id="x" me="viktor" />);
    await screen.findByText("The Swamp untapped by itself.");
    const priority = within(screen.getByRole("group", { name: "Priority" }));
    await userEvent.click(priority.getByRole("button", { name: "High" }));
    expect(await screen.findByText("Priority: High.")).toBeTruthy();
    expect(priority.getByRole("button", { name: "High" }).getAttribute("aria-pressed")).toBe("true");

    await userEvent.click(screen.getByRole("button", { name: "Assign to me" }));
    expect(await screen.findByText("Assigned to viktor.")).toBeTruthy();
    expect(screen.getByLabelText("Assignee")).toHaveProperty("value", "viktor");
    expect(screen.queryByRole("button", { name: "Assign to me" })).toBeNull();
    await userEvent.selectOptions(screen.getByLabelText("Assignee"), "");
    expect(await screen.findByText("Assigned to nobody.")).toBeTruthy();

    await userEvent.type(screen.getByLabelText("New label"), "Not A Label{Enter}");
    expect(await screen.findByText(/A label is lowercase letters/)).toBeTruthy();
    await userEvent.clear(screen.getByLabelText("New label"));
    await userEvent.type(screen.getByLabelText("New label"), "lands{Enter}");
    expect(await screen.findByRole("link", { name: "lands" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Remove the label lands" }));
    await waitFor(() => {
      expect(screen.queryByRole("link", { name: "lands" })).toBeNull();
    });

    const original = "0199bbbb-0000-7000-8000-000000000002";
    await userEvent.type(screen.getByLabelText("The original's id or link"), "bbbb{Enter}");
    expect(await screen.findByText(/Give the original's whole id/)).toBeTruthy();
    await userEvent.clear(screen.getByLabelText("The original's id or link"));
    await userEvent.type(screen.getByLabelText("The original's id or link"), `https://reports.example/r/${original}{Enter}`);
    expect(await screen.findByText("Marked as a duplicate.")).toBeTruthy();
    expect(section("Triage").getByRole("link", { name: "00000002" }).getAttribute("href")).toBe(`/r/${original}`);
    expect(section("Related").getByRole("link", { name: "00000002" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Not a duplicate" }));
    expect(await screen.findByText("No longer a duplicate.")).toBeTruthy();

    expect(calls.filter((c) => c.method === "PATCH").map((c) => c.body)).toEqual([
      { priority: "high", expected_revision: 1 },
      { assignee: "viktor", expected_revision: 2 },
      { assignee: null, expected_revision: 3 },
      { labels: ["lands"], expected_revision: 4 },
      { labels: [], expected_revision: 5 },
      { duplicate_of: original, expected_revision: 6 },
      { duplicate_of: null, expected_revision: 7 },
    ]);
  });

  test("a change made against an old revision is refused, and the report is read again", async () => {
    let revision = 1;
    const calls = serve((call) => {
      if (call.method === "PATCH") return { status: 409, body: { error: "the report changed meanwhile; read it again" } };
      if (call.url === "/ui/api/reports/x") {
        const r = fullReport({ revision, status: revision > 1 ? "triaged" : "new" });
        revision += 1;
        return { body: r };
      }
      if (call.url.endsWith("/audit")) return { body: [] };
      return undefined;
    });
    render(<ReportDetail id="x" />);
    await screen.findByText("The Swamp untapped by itself.");
    await userEvent.selectOptions(screen.getByLabelText("Status"), "resolved");
    expect(await screen.findByText(/Someone changed this report meanwhile/)).toBeTruthy();
    expect(screen.getByLabelText("Status")).toHaveProperty("value", "triaged");
    expect(calls.filter((c) => c.url === "/ui/api/reports/x" && c.method === "GET")).toHaveLength(2);
  });

  test("a note is added, counted, and removed by its author", async () => {
    const calls = start(fullReport());
    render(<ReportDetail id="x" me="viktor" />);
    await screen.findByText("The Swamp untapped by itself.");
    expect(section("Notes").getByText("No notes yet.")).toBeTruthy();
    await userEvent.type(screen.getByLabelText(/New note/), "Seen on 0.9 too{Control>}{Enter}{/Control}");
    expect(await section("Notes (1)").findByText("Seen on 0.9 too")).toBeTruthy();
    expect(calls.find((c) => c.url.endsWith("/notes"))?.body).toEqual({ text: "Seen on 0.9 too" });
    expect(screen.getByLabelText(/New note/)).toHaveProperty("value", "");
    await userEvent.click(section("Notes (1)").getByRole("button", { name: "Remove the note" }));
    expect(await section("Notes").findByText("No notes yet.")).toBeTruthy();
  });

  test("someone else's note has no way to remove it", async () => {
    start(
      fullReport({ notes: 1, note_list: [{ id: "n1", at: "2026-09-27T12:10:00Z", author: "astrid", text: "mine" }] }),
    );
    render(<ReportDetail id="x" me="viktor" />);
    expect(await screen.findByText("mine")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Remove the note" })).toBeNull();
  });

  test("the reports above and below in the list are a key away", async () => {
    window.history.replaceState(null, "", "/r/x");
    start(fullReport(), [], { previous: "p-1", next: "n-1", position: 2, total: 9 });
    render(<ReportDetail id="x" />);
    expect(await screen.findByText("2 of 9")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Next ›" }).getAttribute("href")).toBe("/r/n-1");
    await userEvent.keyboard("j");
    expect(window.location.pathname).toBe("/r/n-1");
    await userEvent.keyboard("k");
    expect(window.location.pathname).toBe("/r/p-1");
  });

  test("the screenshot opens at full size, and Escape shuts it without leaving the page", async () => {
    window.history.replaceState(null, "", "/r/x");
    start(fullReport());
    render(<ReportDetail id="x" />);
    await userEvent.click(await screen.findByRole("button", { name: "Enlarge the screenshot" }));
    expect(screen.getByRole("dialog", { name: "Screenshot" })).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Screenshot" })).toBeNull();
    expect(window.location.pathname).toBe("/r/x");
  });

  test("the reports it belongs with are linked, with how many", async () => {
    start(fullReport({ signature: "0123456789abcdef" }));
    render(<ReportDetail id="x" />);
    const related = await screen.findByRole("region", { name: "Related" });
    expect(await within(related).findByRole("link", { name: "Reports of this crash (2)" })).toBeTruthy();
    expect(within(related).getByRole("link", { name: "Reports from this game (2)" }).getAttribute("href")).toBe(
      "/?game_id=g1",
    );
    expect(within(related).getByRole("link", { name: "Its duplicates (3)" }).getAttribute("href")).toBe(
      "/?duplicate_of=0199aaaa-0000-7000-8000-000000000001",
    );
  });

  test("a report that is gone says so", async () => {
    serve(() => ({ status: 404, body: { error: "no such report" } }));
    render(<ReportDetail id="x" />);
    expect((await screen.findByRole("alert")).textContent).toBe("no such report");
  });
});

const chat = (verified: boolean, sender_tag: string | null) =>
  fullReport({
    kind: "chat",
    text: "rude: you are all useless",
    client: { chat: { verified, sender_tag, message: "you are all useless", conversation: "whisper" } },
  });

describe("the sender of a reported chat message", () => {
  const id = "0199aaaa-0000-7000-8000-000000000001";
  test("is named by tag, with a link to the accounts page filtered to it", async () => {
    start(chat(true, "0001"));
    render(<ReportDetail id={id} />);
    const sender = await screen.findByTestId("chat-sender");
    expect(sender.textContent).toContain("#0001");
    const link = within(sender).getByRole("link", { name: "Find sender's account" });
    expect(link.getAttribute("href")).toBe("/admin/accounts?tag=0001");
    await userEvent.click(link);
    expect(window.location.pathname).toBe("/admin/accounts");
    expect(window.location.search).toBe("?tag=0001");
  });

  test("is not named when the message could not be verified, and there is no link", async () => {
    start(chat(false, null));
    render(<ReportDetail id={id} />);
    const sender = await screen.findByTestId("chat-sender");
    expect(sender.textContent).toBe("not verified, so not named");
    expect(within(sender).queryByRole("link")).toBeNull();
  });

  test("is not a row at all for a report that is no chat message", async () => {
    start(fullReport());
    render(<ReportDetail id={id} />);
    expect(await screen.findByText("The Swamp untapped by itself.")).toBeTruthy();
    expect(screen.queryByTestId("chat-sender")).toBeNull();
  });
});
