import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import type { Summary } from "./api";
import { applyChange, ReportList } from "./ReportList";
import { useRoute } from "./router";
import { serve, summary } from "./test-utils";

const reports: Summary[] = [
  summary({ id: "id-1", text: "First report", kind: "bug", reporter: "aaaaaaaaaaaaaaaa" }),
  summary({
    id: "id-2",
    text: "Second report\nwith more lines",
    kind: "crash",
    reporter: "bbbb",
    has_record: true,
    record_bytes: 2048,
    issue_number: 311,
    issue_url: "https://github.com/AceVik/baylee/issues/311",
  }),
  summary({ id: "id-3", text: "Third report", kind: "feedback", status: "resolved", reporter: "cccc" }),
];

/** The list page as the app mounts it, following the URL. */
function Page() {
  const route = useRoute();
  return route.page === "list" ? <ReportList search={route.search} /> : <p>report {route.id}</p>;
}

function start(total = reports.length) {
  return serve((call) => {
    if (call.url === "/ui/api/facets")
      return {
        body: {
          gateways: [
            { value: "eu", count: 2 },
            { value: "us", count: 1 },
          ],
          statuses: [],
          kinds: [],
          reporters: [{ value: "aaaaaaaaaaaaaaaa", count: 2 }],
        },
      };
    if (call.url.startsWith("/ui/api/reports")) return { body: { total, reports } };
    return undefined;
  });
}

const lastQuery = (calls: { url: string }[]) =>
  new URLSearchParams(calls.findLast((c) => c.url.startsWith("/ui/api/reports"))?.url.split("?")[1]);

describe("the report list", () => {
  test("shows each report's row", async () => {
    start();
    render(<Page />);
    const rows = await screen.findAllByRole("row");
    expect(rows).toHaveLength(4);
    const second = within(rows[2] as HTMLElement);
    expect(second.getByText("Second report")).toBeTruthy();
    expect(second.getByText("crash")).toBeTruthy();
    expect(second.getByText("2.0 KiB")).toBeTruthy();
    expect(second.getByRole("link", { name: "#311" }).getAttribute("href")).toBe(
      "https://github.com/AceVik/baylee/issues/311",
    );
    expect(second.getByRole("link", { name: "#311" }).getAttribute("rel")).toBe("noopener noreferrer");
    expect(screen.getByText(/1–3 of 3/)).toBeTruthy();
  });

  test("each filter goes into the URL and the request", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "crash");
    expect(window.location.search).toBe("?kind=crash");
    expect(lastQuery(calls).get("kind")).toBe("crash");

    await userEvent.selectOptions(screen.getByLabelText("Status"), "wont_fix");
    await userEvent.selectOptions(screen.getByLabelText("Gateway"), "us");
    await userEvent.selectOptions(screen.getByLabelText("Game record"), "true");
    await userEvent.type(screen.getByLabelText("Text"), "swamp & more{Enter}");
    const query = lastQuery(calls);
    expect(Object.fromEntries(query)).toEqual({
      limit: "50",
      kind: "crash",
      status: "wont_fix",
      gateway: "us",
      q: "swamp & more",
      has_record: "true",
    });

    await userEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(window.location.search).toBe("");
    expect(Object.fromEntries(lastQuery(calls))).toEqual({ limit: "50" });
  });

  test("a date range and a pseudonym filter", async () => {
    const calls = start();
    render(<Page />);
    await screen.findAllByRole("row");
    const from = screen.getByLabelText("From");
    await userEvent.type(from, "2026-09-01");
    const reporter = screen.getByLabelText("Reporter");
    await userEvent.type(reporter, "bbbb{Enter}");
    const query = lastQuery(calls);
    expect(query.get("from")).toBe("2026-09-01");
    expect(query.get("reporter")).toBe("bbbb");
  });

  test("a pseudonym in a row filters by it", async () => {
    const calls = start();
    render(<Page />);
    await userEvent.click(await screen.findByRole("button", { name: "bbbb" }));
    expect(lastQuery(calls).get("reporter")).toBe("bbbb");
  });

  test("j and k move the selection, Enter opens it, / goes to the search", async () => {
    start();
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
    await userEvent.keyboard("{Enter}");
    expect(window.location.pathname).toBe("/r/id-2");
    expect(await screen.findByText("report id-2")).toBeTruthy();
  });

  test("keys typed into a field are the field's, not the list's", async () => {
    start();
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.click(screen.getByLabelText("Reporter"));
    await userEvent.keyboard("jjk");
    expect(screen.getAllByRole("row")[1]?.getAttribute("aria-selected")).toBe("true");
    expect(screen.getByLabelText("Reporter")).toHaveProperty("value", "jjk");
  });

  test("pages are asked of the server", async () => {
    const calls = start(120);
    render(<Page />);
    await screen.findAllByRole("row");
    await userEvent.click(screen.getByRole("button", { name: "Older →" }));
    expect(lastQuery(calls).get("offset")).toBe("50");
    expect(await screen.findByText(/51–53 of 120/)).toBeTruthy();
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "bug");
    expect(lastQuery(calls).get("offset")).toBeNull();
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
    expect(applyChange({ kind: "bug", offset: 100, q: "x" }, { q: undefined, status: "new" })).toEqual({
      kind: "bug",
      status: "new",
    });
  });
});
