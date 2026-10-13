import { describe, expect, test, vi } from "vitest";

import {
  accountQuery,
  api,
  filterQuery,
  onSignedOut,
  parseAccountFilter,
  parseFilter,
  Refused,
  senderTag,
  SignedOut,
  type Filter,
} from "./api";
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

describe("the sender a chat report names", () => {
  test("a tag is lowercase hex of one to eight digits, with a # and a name before it dropped", () => {
    expect(senderTag("af03")).toBe("af03");
    expect(senderTag("0001")).toBe("0001");
    expect(senderTag("AF03")).toBe("af03");
    expect(senderTag("#af03")).toBe("af03");
    expect(senderTag("Alice#af03")).toBe("af03");
    expect(senderTag("  Alice#AF03 ")).toBe("af03");
    expect(senderTag("7fffffff")).toBe("7fffffff");
    expect(senderTag("1")).toBe("1");
  });

  test("anything else is no tag", () => {
    for (const value of ["", "  ", "#", "Alice#", "zz", "af 03", "123456789", "af03x", "0x1f", null, undefined, 4, {}]) {
      expect(senderTag(value)).toBeNull();
    }
  });

  test("an account filter with a tag goes out and comes back the same", () => {
    const filter = { q: "ali", tag: "af03", kind: "guest", sort: "name", online: true, offset: 50 } as const;
    const query = accountQuery(filter);
    expect(query).toBe("q=ali&tag=af03&kind=guest&sort=name&online=true&offset=50");
    expect(parseAccountFilter(`?${query}`)).toEqual(filter);
  });

  test("a tag typed with its name or # in the URL is read as the tag alone, and a bad one is dropped", () => {
    expect(parseAccountFilter("?tag=Alice%23AF03")).toEqual({ tag: "af03" });
    expect(accountQuery({ tag: "#af03" })).toBe("tag=af03");
    expect(parseAccountFilter("?tag=zz&q=x")).toEqual({ q: "x" });
    expect(accountQuery({ tag: "zz" })).toBe("");
    expect(accountQuery({})).toBe("");
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
