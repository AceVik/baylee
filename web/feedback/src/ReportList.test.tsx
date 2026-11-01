import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import type { Facets, Summary } from "./api";
import { applyChange, LIST_REFRESH_MS, ReportList } from "./ReportList";
import { Toasts } from "./reports/pieces";
import { useRoute } from "./router";
import { serve, summary, type Call } from "./test-utils";

const ME = "viktor";

const reports: Summary[] = [
  summary({ id: "id-1", text: "First report", kind: "bug", reporter: "aaaaaaaaaaaaaaaa", labels: ["ui"] }),
  summary({
    id: "id-2",
    text: "Second report\nwith more lines",
    kind: "crash",
    reporter: "bbbb",
    has_record: true,
    record_bytes: 2048,
    issue_number: 311,
    issue_url: "https://github.com/AceVik/baylee/issues/311",
    priority: "high",
    assignee: ME,
    notes: 2,
    seen: true,
  }),
  summary({
    id: "id-3",
    text: "Third report",
    kind: "feedback",
    status: "resolved",
    reporter: "cccc",
    source: "llm-seat",
  }),
];

const facets = (over: Partial<Facets> = {}): Facets => ({
  gateways: [
    { value: "eu", count: 2 },
    { value: "us", count: 1 },
  ],
  statuses: [
    { value: "new", count: 2 },
    { value: "resolved", count: 1 },
  ],
  kinds: [
    { value: "bug", count: 1 },
    { value: "crash", count: 1 },
    { value: "feedback", count: 1 },
  ],
  reporters: [{ value: "aaaaaaaaaaaaaaaa", count: 2 }],
  assignees: [
    { value: null, count: 2 },
    { value: ME, count: 1 },
  ],
  priorities: [
    { value: null, count: 2 },
    { value: "high", count: 1 },
  ],
  labels: [{ value: "ui", count: 1 }],
  channels: [{ value: "gateway", count: 3 }],
  sources: [
    { value: null, count: 2 },
    { value: "llm-seat", count: 1 },
  ],
  total: 3,
  ...over,
});

/** The list page as the app mounts it, following the URL, with the toasts the app draws. */
function Page() {
  const route = useRoute();
  return (
    <>
      {route.page === "list" ? (
        <ReportList search={route.search} me={ME} />
      ) : (
        <p>report {route.page === "report" ? route.id : ""}</p>
      )}
      <Toasts lang="en" />
    </>
  );
}

function start(total = reports.length, extra?: (call: Call) => { status?: number; body?: unknown } | undefined) {
  return serve((call) => {
    const answered = extra?.(call);
    if (answered !== undefined) return answered;
    if (call.url === "/ui/api/reports/bulk") {
      const ids = (call.body as { ids: string[] }).ids;
      return { body: { changed: ids.length } };
    }
    if (call.url === "/ui/api/seen") return { status: 204 };
    if (call.url === "/ui/api/admins") return { body: [ME, "astrid"] };
    if (call.url === "/ui/api/views") return { body: [] };
    const one = /^\/ui\/api\/reports\/([^/?]+)$/.exec(call.url)?.[1];
    if (one !== undefined) {
      const found = reports.find((r) => r.id === one);
      return found
        ? { body: { ...found, client: {}, note_list: [] } }
        : { status: 404, body: { error: "no such report" } };
    }
    if (call.url.endsWith("/audit")) return { body: [] };
    if (call.url.startsWith("/ui/api/facets")) return { body: facets() };
    if (call.url.startsWith("/ui/api/crashes")) {
      return {
        body: [
          {
            signature: "0123456789abcdef",
            count: 4,
            open: 3,
            first: "2026-09-01T10:00:00Z",
            last: "2026-09-27T10:00:00Z",
            latest: "id-2",
            message: "index out of bounds",
            location: "src/table.rs:1:2",
          },
        ],
      };
    }
    if (call.url.startsWith("/ui/api/reports")) return { body: { total, reports } };
    return undefined;
  });
}

const lastList = (calls: Call[]) =>
  new URLSearchParams(calls.findLast((c) => c.url.startsWith("/ui/api/reports?"))?.url.split("?")[1]);

const bulks = (calls: Call[]) => calls.filter((c) => c.url === "/ui/api/reports/bulk").map((c) => c.body);

const openFilters = async () => {
  await userEvent.click(screen.getByRole("button", { name: /^Filters/ }));
};

const chip = (group: string, name: RegExp) =>
  within(screen.getByRole("group", { name: group })).getByRole("button", { name });

describe("the report list", () => {
  test("shows each report's row with its triage marks", async () => {
    start();
    render(<Page />);
    const rows = await screen.findAllByRole("row");
    expect(rows).toHaveLength(4);
    const second = within(rows[2] as HTMLElement);
    expect(second.getByText("Second report")).toBeTruthy();
    expect(second.getByText("crash")).toBeTruthy();
    expect(second.getByText("High")).toBeTruthy();
    expect(second.getByRole("button", { name: ME })).toBeTruthy();
    expect(second.getByTitle("2 notes")).toBeTruthy();
    expect(second.getByText("2.0 KiB")).toBeTruthy();
    expect(second.getByRole("link", { name: "#311" }).getAttribute("href")).toBe(
      "https://github.com/AceVik/baylee/issues/311",
    );
    expect(second.getByRole("link", { name: "#311" }).getAttribute("rel")).toBe("noopener noreferrer");
    expect(within(rows[1] as HTMLElement).getByRole("button", { name: "ui" })).toBeTruthy();
    expect(screen.getByText(/1–3 of 3/)).toBeTruthy();
  });

  test("each filter goes into the URL and the request, several values at once", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.click(chip("Kind", /^crash/));
    expect(window.location.search).toBe("?kind=crash");
    expect(lastList(calls).get("kind")).toBe("crash");
    await userEvent.click(chip("Kind", /^bug/));
    expect(lastList(calls).get("kind")).toBe("bug,crash");

    await userEvent.click(chip("Status", /^Won't fix/));
    await userEvent.click(chip("Priority", /^None/));
    await userEvent.selectOptions(screen.getByLabelText("Gateway"), "us");
    await userEvent.selectOptions(screen.getByLabelText("Game record"), "true");
    await userEvent.selectOptions(screen.getByLabelText("Assignee"), "none");
    await userEvent.selectOptions(screen.getByLabelText("Label"), "ui");
    await userEvent.type(screen.getByLabelText("Text"), "swamp & more{Enter}");
    expect(Object.fromEntries(lastList(calls))).toEqual({
      limit: "50",
      kind: "bug,crash",
      status: "wont_fix",
      priority: "none",
      assignee: "none",
      label: "ui",
      gateway: "us",
      q: "swamp & more",
      has_record: "true",
    });
    // Each filter is a pill that removes it; Clear removes them all.
    await userEvent.click(screen.getByRole("button", { name: "Remove the filter Gateway: us" }));
    expect(lastList(calls).get("gateway")).toBeNull();
    await userEvent.click(within(screen.getByRole("list", { name: "Filters" })).getByRole("button", { name: "Clear" }));
    expect(window.location.search).toBe("");
    expect(Object.fromEntries(lastList(calls))).toEqual({ limit: "50" });
  });

  test("the source filter picks one source, none, or all but one, and a row's source picks it", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    const select = screen.getByLabelText("Sent by");
    expect(within(select).getByRole("option", { name: "llm-seat (1)" })).toBeTruthy();
    expect(within(select).getByRole("option", { name: "no source given (2)" })).toBeTruthy();
    await userEvent.selectOptions(select, "!llm-seat");
    expect(window.location.search).toBe("?source=%21llm-seat");
    expect(lastList(calls).get("source")).toBe("!llm-seat");
    expect(screen.getByRole("button", { name: "Remove the filter Sent by: all but llm-seat" })).toBeTruthy();
    await userEvent.selectOptions(select, "none");
    expect(lastList(calls).get("source")).toBe("none");
    await userEvent.click(screen.getByRole("button", { name: "Remove the filter Sent by: no source given" }));
    expect(lastList(calls).get("source")).toBeNull();
    // A report that names its source says it in its row, and picks it.
    await userEvent.click(screen.getByRole("button", { name: "llm-seat" }));
    expect(lastList(calls).get("source")).toBe("llm-seat");
  });

  test("the facets count under the filter, and the views each count over their own", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.click(chip("Kind", /^bug/));
    await waitFor(() => {
      expect(calls.some((c) => c.url === "/ui/api/facets?kind=bug")).toBe(true);
    });
    expect(
      calls.some((c) => c.url === "/ui/api/facets?status=new%2Ctriaged%2Cin_progress&assignee=viktor"),
    ).toBe(true);
    expect(calls.some((c) => c.url === "/ui/api/facets?trash=true")).toBe(true);
    const views = screen.getByRole("group", { name: "Views" });
    expect(within(views).getByRole("button", { name: /^Inbox/ }).textContent).toBe("Inbox2");
  });

  test("a view sets the status, the assignee and the trash, and keeps the other filters", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.click(chip("Kind", /^bug/));
    const views = screen.getByRole("group", { name: "Views" });
    await userEvent.click(within(views).getByRole("button", { name: /^Mine/ }));
    expect(Object.fromEntries(lastList(calls))).toEqual({
      limit: "50",
      status: "new,triaged,in_progress",
      kind: "bug",
      assignee: ME,
    });
    expect(within(views).getByRole("button", { name: /^Mine/ }).getAttribute("aria-pressed")).toBe("true");
    await userEvent.click(within(views).getByRole("button", { name: /^Trash/ }));
    expect(Object.fromEntries(lastList(calls))).toEqual({ limit: "50", trash: "true", kind: "bug" });
    expect(screen.getByText(/deleted for good after 30 days/)).toBeTruthy();
  });

  test("a date range and a pseudonym filter", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.type(screen.getByLabelText("From"), "2026-09-01");
    await userEvent.type(screen.getByLabelText("Reporter"), "bbbb{Enter}");
    const query = lastList(calls);
    expect(query.get("from")).toBe("2026-09-01");
    expect(query.get("reporter")).toBe("bbbb");
  });

  test("a pseudonym, an assignee or a label in a row filters by it", async () => {
    const calls = start();
    render(<Page />);
    await userEvent.click(await screen.findByRole("button", { name: "bbbb" }));
    expect(lastList(calls).get("reporter")).toBe("bbbb");
    await screen.findAllByRole("row");
    await userEvent.click(within(screen.getAllByRole("row")[2] as HTMLElement).getByRole("button", { name: ME }));
    expect(lastList(calls).get("assignee")).toBe(ME);
    await userEvent.click(within(screen.getAllByRole("row")[1] as HTMLElement).getByRole("button", { name: "ui" }));
    expect(lastList(calls).get("label")).toBe("ui");
  });

  test("a column header sorts the list, a second click turns it round", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.click(screen.getByRole("button", { name: /^Priority/ }));
    expect(lastList(calls).get("sort")).toBe("priority");
    expect(lastList(calls).get("dir")).toBeNull();
    expect(screen.getByRole("columnheader", { name: /Priority/ }).getAttribute("aria-sort")).toBe("descending");
    await userEvent.click(screen.getByRole("button", { name: /^Priority/ }));
    expect(lastList(calls).get("dir")).toBe("asc");
    await userEvent.click(screen.getByRole("button", { name: /^Status/ }));
    expect(lastList(calls).get("sort")).toBe("status");
    expect(lastList(calls).get("dir")).toBe("asc");
  });

  test("j and k move the cursor, Enter opens the drawer, o the page, / goes to the search", async () => {
    const calls = start();
    render(<Page />);
    const rows = await screen.findAllByRole("row");
    expect(rows[1]?.getAttribute("aria-selected")).toBe("true");
    await userEvent.keyboard("j");
    await userEvent.keyboard("j");
    await userEvent.keyboard("j");
    expect(screen.getAllByRole("row")[3]?.getAttribute("aria-selected")).toBe("true");
    await userEvent.keyboard("k");
    expect(screen.getAllByRole("row")[2]?.getAttribute("aria-selected")).toBe("true");
    await userEvent.keyboard("/");
    expect(document.activeElement).toBe(screen.getByLabelText("Text"));
    await act(async () => {
      (document.activeElement as HTMLElement).blur();
    });
    // Enter opens the report beside the list and asks for it; Escape shuts it.
    expect(screen.queryByTestId("drawer")).toBeNull();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByTestId("drawer")).toBeTruthy();
    expect(calls.some((c) => c.url === "/ui/api/reports/id-2")).toBe(true);
    expect(window.location.pathname).toBe("/");
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByTestId("drawer")).toBeNull();
    // o opens its page.
    await userEvent.keyboard("o");
    expect(window.location.pathname).toBe("/r/id-2");
    expect(await screen.findByText("report id-2")).toBeTruthy();
  });

  test("x selects under the cursor, a selects the page, and one request changes every selected report", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.keyboard("x");
    expect(screen.getByRole("toolbar", { name: "1 selected" })).toBeTruthy();
    await userEvent.keyboard("a");
    expect(screen.getByRole("toolbar", { name: "3 selected" })).toBeTruthy();
    await userEvent.selectOptions(screen.getByLabelText("Set status"), "triaged");
    expect(await screen.findByText("3 reports changed.")).toBeTruthy();

    await userEvent.selectOptions(screen.getByLabelText("Assign"), ME);
    await waitFor(() => {
      expect(bulks(calls)).toHaveLength(2);
    });
    await userEvent.type(screen.getByLabelText("Add label"), "lands{Enter}");
    await waitFor(() => {
      expect(bulks(calls)).toHaveLength(3);
    });
    await userEvent.selectOptions(screen.getByLabelText("Mark the others as duplicates of the one picked"), "id-1");
    await waitFor(() => {
      expect(bulks(calls)).toHaveLength(4);
    });
    expect(bulks(calls)).toEqual([
      { ids: ["id-1", "id-2", "id-3"], change: { status: "triaged" } },
      { ids: ["id-1", "id-2", "id-3"], change: { assignee: ME } },
      { ids: ["id-1", "id-2", "id-3"], change: { labels_add: ["lands"] } },
      { ids: ["id-2", "id-3"], change: { duplicate_of: "id-1" } },
    ]);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("toolbar")).toBeNull();
  });

  test("a refused bulk change says nothing was changed", async () => {
    start(3, (call) =>
      call.url === "/ui/api/reports/bulk" ? { status: 400, body: { error: "no admin of that name" } } : undefined,
    );
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.keyboard("2");
    expect(await screen.findByText("Nothing was changed: no admin of that name")).toBeTruthy();
  });

  test("# moves to the trash, and the toast's Undo takes it out again", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.keyboard("#");
    expect(await screen.findByText("1 report moved to the trash.")).toBeTruthy();
    expect(bulks(calls)).toEqual([{ ids: ["id-1"], action: "trash" }]);
    await userEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect(await screen.findByText("1 report restored.")).toBeTruthy();
    expect(bulks(calls)).toEqual([
      { ids: ["id-1"], action: "trash" },
      { ids: ["id-1"], action: "restore" },
    ]);
  });

  test("a report trashed in the drawer leaves the list, and its Undo brings it back", async () => {
    const calls = start(3, (call) =>
      call.method === "DELETE" || call.url.endsWith("/restore") ? { status: 204 } : undefined,
    );
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.keyboard("{Enter}");
    const drawer = await screen.findByTestId("drawer");
    await userEvent.click(await within(drawer).findByRole("button", { name: "Move to trash" }));
    expect(await screen.findByText("Report moved to the trash.")).toBeTruthy();
    expect(screen.queryByTestId("drawer")).toBeNull();
    expect(screen.getAllByRole("row")).toHaveLength(3);
    const asked = calls.filter((c) => c.url.startsWith("/ui/api/reports?")).length;
    await userEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect(await screen.findByText("Report restored.")).toBeTruthy();
    await waitFor(() => {
      expect(screen.getAllByRole("row")).toHaveLength(4);
    });
    expect(calls.filter((c) => c.url.startsWith("/ui/api/reports?")).length).toBeGreaterThan(asked);
    expect(calls.some((c) => c.url === "/ui/api/reports/id-1/restore" && c.method === "POST")).toBe(true);
  });

  test("m assigns to me, and again to nobody", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.keyboard("m");
    await waitFor(() => {
      expect(bulks(calls)).toHaveLength(1);
    });
    await userEvent.keyboard("j");
    await userEvent.keyboard("m");
    await waitFor(() => {
      expect(bulks(calls)).toEqual([
        { ids: ["id-1"], change: { assignee: ME } },
        { ids: ["id-2"], change: { assignee: null } },
      ]);
    });
  });

  test("a new report arriving in a quiet refresh is announced, not slid under the cursor", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let list = reports;
    serve((call) => {
      if (call.url.startsWith("/ui/api/facets")) return { body: facets() };
      if (call.url.startsWith("/ui/api/reports")) return { body: { total: list.length, reports: list } };
      return undefined;
    });
    render(<Page />);
    expect(await screen.findAllByRole("row")).toHaveLength(4);
    list = [summary({ id: "id-0", text: "Just in", reporter: "dddd" }), ...reports];
    await act(async () => {
      await vi.advanceTimersByTimeAsync(LIST_REFRESH_MS + 50);
    });
    expect(await screen.findByText("1 new report since you looked")).toBeTruthy();
    expect(screen.getAllByRole("row")).toHaveLength(4);
    await userEvent.click(screen.getByRole("button", { name: "Show" }));
    expect(screen.getAllByRole("row")).toHaveLength(5);
    expect(screen.getByText("Just in")).toBeTruthy();
    vi.useRealTimers();
  });

  test("what this admin opened is told to the service and no longer marked unread", async () => {
    const calls = start();
    render(<Page />);
    const rows = await screen.findAllByRole("row");
    expect(rows[1]?.className).toContain("unread");
    // The second was opened before, the third is resolved: neither is unread.
    expect(rows[2]?.className ?? "").not.toContain("unread");
    expect(rows[3]?.className ?? "").not.toContain("unread");
    await userEvent.keyboard("{Enter}");
    await screen.findByTestId("drawer");
    expect(screen.getAllByRole("row")[1]?.className ?? "").not.toContain("unread");
    expect(calls.find((c) => c.url === "/ui/api/seen")?.body).toEqual({ ids: ["id-1"], seen: true });
    await userEvent.keyboard("{Escape}");
    await userEvent.keyboard("u");
    expect(screen.getAllByRole("row")[1]?.className).toContain("unread");
    expect(calls.findLast((c) => c.url === "/ui/api/seen")?.body).toEqual({ ids: ["id-1"], seen: false });
  });

  test("keys typed into a field are the field's, not the list's", async () => {
    start();
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.click(screen.getByLabelText("Reporter"));
    await userEvent.keyboard("jjk");
    expect(screen.getAllByRole("row")[1]?.getAttribute("aria-selected")).toBe("true");
    expect(screen.getByLabelText("Reporter")).toHaveProperty("value", "jjk");
  });

  test("pages are asked of the server, by number and by size", async () => {
    const calls = start(120);
    render(<Page />);
    await screen.findAllByRole("row");
    expect(screen.getByText("Page 1 of 3")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Next ›" }));
    expect(lastList(calls).get("offset")).toBe("50");
    expect(await screen.findByText(/51–53 of 120/)).toBeTruthy();
    expect(screen.getByText("Page 2 of 3")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Last »" }));
    expect(lastList(calls).get("offset")).toBe("100");
    await screen.findByText(/101–103 of 120/);
    await userEvent.keyboard("[[");
    expect(lastList(calls).get("offset")).toBe("50");
    await userEvent.selectOptions(screen.getByLabelText("Per page"), "100");
    expect(lastList(calls).get("limit")).toBe("100");
    expect(lastList(calls).get("offset")).toBeNull();
    await openFilters();
    await userEvent.click(chip("Kind", /^bug/));
    expect(lastList(calls).get("offset")).toBeNull();
    expect(lastList(calls).get("limit")).toBe("100");
  });

  test("crashes are shown grouped, and a group shows its reports", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.click(await screen.findByRole("button", { name: "Crash groups" }));
    expect(await screen.findByText("index out of bounds")).toBeTruthy();
    expect(calls.some((c) => c.url === "/ui/api/crashes")).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "Show its reports" }));
    expect(lastList(calls).get("signature")).toBe("0123456789abcdef");
    expect(await screen.findByText("Second report")).toBeTruthy();
  });

  test("a view is saved under a name with the list's query", async () => {
    const calls = start(3, (call) =>
      call.method === "POST" && call.url === "/ui/api/views"
        ? { body: [{ id: "v1", ...(call.body as { name: string; query: string }) }] }
        : undefined,
    );
    render(<Page />);
    await screen.findAllByRole("row");
    await openFilters();
    await userEvent.click(chip("Kind", /^crash/));
    await userEvent.click(screen.getByText("Saved views"));
    await userEvent.click(screen.getByRole("button", { name: "Save this view…" }));
    await userEvent.type(screen.getByLabelText("Name of the view"), "Crashes{Enter}");
    await waitFor(() => {
      expect(calls.find((c) => c.method === "POST" && c.url === "/ui/api/views")?.body).toEqual({
        name: "Crashes",
        query: "kind=crash",
      });
    });
    expect(await screen.findByText("View “Crashes” saved.")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Crashes" }).getAttribute("href")).toBe("/?kind=crash");
  });

  test("an empty list says so", async () => {
    serve((call) =>
      call.url.startsWith("/ui/api/reports") ? { body: { total: 0, reports: [] } } : undefined,
    );
    render(<Page />);
    expect(await screen.findByText(/No reports match/)).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });

  test("applyChange starts again at the first page and drops cleared filters", () => {
    expect(
      applyChange(
        { kind: ["bug"], offset: 100, q: "x", limit: 100 },
        { q: undefined, status: ["new"], label: undefined },
      ),
    ).toEqual({
      kind: ["bug"],
      status: ["new"],
      limit: 100,
    });
    expect(applyChange({ kind: ["bug"] }, { kind: [] })).toEqual({});
  });
});
