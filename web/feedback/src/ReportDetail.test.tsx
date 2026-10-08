import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import type { AuditEntry, Report } from "./api";
import { ReportDetail } from "./ReportDetail";
import { fullReport, serve, type Call } from "./test-utils";

function start(report: Report, audit: AuditEntry[] = []): Call[] {
  let current = report;
  return serve((call) => {
    if (call.url.endsWith("/audit")) return { body: audit };
    if (call.method === "PATCH") {
      const change = call.body as { status?: Report["status"]; issue?: number | null };
      if (change.issue === 0) return { status: 400, body: { error: "not an issue number" } };
      current = { ...current };
      if (change.status) current.status = change.status;
      if (change.issue !== undefined) {
        current.issue_number = change.issue;
        current.issue_url = change.issue === null ? null : `https://github.com/AceVik/baylee/issues/${change.issue}`;
      }
      return { body: current };
    }
    if (call.method === "DELETE") return { status: 204 };
    if (call.url.startsWith("/ui/api/reports/")) return { body: current };
    return undefined;
  });
}

const section = (name: string) => within(screen.getByRole("region", { name }));

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
    expect(section("History").getByText(/viktor: status triaged/)).toBeTruthy();
    expect(screen.getByText("0.1.0-beta.1 (3f9a1c7e21)")).toBeTruthy();
    expect(screen.getByRole("link", { name: "5bdc0e1f9a7c33aa" }).getAttribute("href")).toBe(
      "/?reporter=5bdc0e1f9a7c33aa",
    );
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
    expect(patch?.body).toEqual({ status: "in_progress" });
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
    expect(calls.find((c) => c.method === "PATCH")?.body).toEqual({ issue: 311 });
    await userEvent.click(screen.getByRole("button", { name: "Unlink" }));
    expect(await screen.findByText("Not linked.")).toBeTruthy();
    expect(calls.findLast((c) => c.method === "PATCH")?.body).toEqual({ issue: null });
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
    for (const leak of ["Swamp untapped", "5bdc0e1f9a7c33aa", "0199aaaa", "/r/", "Apple M1", "eu.example"]) {
      expect(decodeURIComponent(href)).not.toContain(leak);
    }
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toBe("noopener noreferrer");
  });

  test("deleting asks once more, then goes back to the list", async () => {
    window.history.replaceState(null, "", "/r/x");
    const calls = start(fullReport());
    render(<ReportDetail id="x" />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete report…" }));
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Keep it" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete report…" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete for good" }));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/");
    });
    const del = calls.find((c) => c.method === "DELETE");
    expect(del?.url).toBe("/ui/api/reports/x");
    expect(del?.headers["x-baylee-csrf"]).toBe("1");
  });

  test("a report that is gone says so", async () => {
    serve(() => ({ status: 404, body: { error: "no such report" } }));
    render(<ReportDetail id="x" />);
    expect((await screen.findByRole("alert")).textContent).toBe("no such report");
  });
});
