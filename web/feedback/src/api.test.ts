import { describe, expect, test, vi } from "vitest";

import { api, filterQuery, onSignedOut, parseFilter, Refused, SignedOut, type Filter } from "./api";
import { serve } from "./test-utils";

describe("filters in the query string", () => {
  test("a filter goes out and comes back the same", () => {
    const filter: Filter = {
      kind: "crash",
      status: "in_progress",
      gateway: "eu",
      reporter: "5bdc",
      q: "50% & more",
      from: "2026-09-01",
      to: "2026-09-27",
      has_record: false,
      offset: 50,
    };
    const query = filterQuery(filter);
    expect(query).toContain("q=50%25+%26+more");
    expect(parseFilter(`?${query}`)).toEqual(filter);
  });

  test("an empty filter is an empty query, and blank text is no filter", () => {
    expect(filterQuery({})).toBe("");
    expect(filterQuery({ q: "   ", offset: 0 })).toBe("");
  });

  test("anything malformed in a hand-edited URL is dropped, not sent", () => {
    expect(
      parseFilter("?kind=rant&status=lost&from=2026-9-1&to=yesterday&has_record=maybe&offset=-5&gateway="),
    ).toEqual({});
    expect(parseFilter("?has_record=true&offset=3.5")).toEqual({ has_record: true });
  });
});

describe("requests", () => {
  test("a read carries no CSRF header and a change does", async () => {
    const calls = serve((call) =>
      call.method === "GET" ? { body: { total: 0, reports: [] } } : { body: { id: "x" } },
    );
    await api.reports({ kind: "bug" });
    await api.change("x", { status: "triaged" });
    await api.remove("x").catch(() => undefined);
    await api.logout().catch(() => undefined);
    const [list, patch, del, out] = calls;
    expect(list?.url).toBe("/ui/api/reports?limit=50&kind=bug");
    expect(list?.headers["x-baylee-csrf"]).toBeUndefined();
    expect(patch?.method).toBe("PATCH");
    expect(patch?.headers["x-baylee-csrf"]).toBe("1");
    expect(patch?.headers["content-type"]).toBe("application/json");
    expect(patch?.body).toEqual({ status: "triaged" });
    expect(del?.method).toBe("DELETE");
    expect(del?.headers["x-baylee-csrf"]).toBe("1");
    expect(out?.headers["x-baylee-csrf"]).toBe("1");
  });

  test("a 401 tells every listener the session is gone", async () => {
    serve(() => ({ status: 401, body: { error: "not signed in" } }));
    const listener = vi.fn<() => void>();
    const stop = onSignedOut(listener);
    await expect(api.reports({})).rejects.toBeInstanceOf(SignedOut);
    expect(listener).toHaveBeenCalledTimes(1);
    stop();
    await expect(api.me()).rejects.toBeInstanceOf(SignedOut);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  test("a failed sign-in is a refusal, not a lost session", async () => {
    serve(() => ({ status: 401, body: { error: "wrong name or password" } }));
    const listener = vi.fn<() => void>();
    const stop = onSignedOut(listener);
    const error: unknown = await api.login("a", "b").catch((e: unknown) => e);
    stop();
    expect(error).toBeInstanceOf(Refused);
    expect((error as Refused).status).toBe(401);
    expect((error as Refused).message).toBe("wrong name or password");
    expect(listener).not.toHaveBeenCalled();
  });

  test("a refusal carries the service's sentence, or the status when it has none", async () => {
    serve((call) =>
      call.url.endsWith("/audit")
        ? { status: 503, body: "not json" }
        : { status: 400, body: { error: "not an issue number" } },
    );
    await expect(api.change("x", { issue: 0 })).rejects.toThrow("not an issue number");
    await expect(api.audit("x")).rejects.toThrow(/^503/);
  });

  test("an id is put into the path encoded", async () => {
    const calls = serve(() => ({ body: {} }));
    await api.report("a/b?c");
    expect(calls[0]?.url).toBe("/ui/api/reports/a%2Fb%3Fc");
    expect(api.recordUrl("a/b")).toBe("/ui/api/reports/a%2Fb/record");
  });
});
