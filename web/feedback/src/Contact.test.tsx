import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import { App } from "./App";
import type { AuditEntry, ContactDetail, ContactListing, ContactMessage } from "./api";
import { ContactList, ContactPage } from "./Contact";
import { contactPath, parseRoute } from "./router";
import { serve, type Call } from "./test-utils";

const SLOW = 15_000;

function message(over: Partial<ContactMessage> = {}): ContactMessage {
  return {
    id: "0199bbbb-0000-7000-8000-000000000001",
    created_at: "2026-10-01T10:00:00Z",
    updated_at: "2026-10-01T10:00:00Z",
    name: "Ada",
    email: "ada@example.org",
    message: "Is there a Mac build of the beta?",
    language: "en",
    status: "new",
    done_at: null,
    ...over,
  };
}

function detail(over: Partial<ContactDetail> = {}): ContactDetail {
  return { ...message(), audit: [], ...over };
}

function listing(messages: ContactMessage[], over: Partial<ContactListing> = {}): ContactListing {
  return { open: messages.filter((m) => m.status === "new").length, total: messages.length, messages, ...over };
}

describe("the contact routes", () => {
  test("/contact is the list and keeps its query string", () => {
    expect(parseRoute("/contact", "")).toEqual({ page: "contact", search: "" });
    expect(parseRoute("/contact/", "")).toEqual({ page: "contact", search: "" });
    expect(parseRoute("/contact", "?status=done")).toEqual({ page: "contact", search: "?status=done" });
  });

  test("/contact/{id} is one message, the id decoded", () => {
    expect(parseRoute("/contact/abc", "")).toEqual({ page: "contact", search: "", id: "abc" });
    expect(parseRoute("/contact/a%20b%2Fc", "")).toEqual({ page: "contact", search: "", id: "a b/c" });
  });

  test("contactPath names the list, a filter or one message", () => {
    expect(contactPath()).toBe("/contact");
    expect(contactPath(undefined, "new")).toBe("/contact?status=new");
    expect(contactPath(undefined, "")).toBe("/contact");
    expect(contactPath("x y")).toBe("/contact/x%20y");
    expect(parseRoute(contactPath("x y/z"), "")).toMatchObject({ page: "contact", id: "x y/z" });
  });
});

describe("the contact list", () => {
  test("asks for every message when no filter is set", async () => {
    const calls = serve(() => ({ body: listing([message()]) }));
    render(<ContactList lang="en" search="" />);
    await screen.findByText("Ada");
    expect(calls.map((c) => c.url)).toEqual(["/ui/api/contact"]);
  });

  test("passes the status filter on", async () => {
    const calls = serve(() => ({ body: listing([message()]) }));
    render(<ContactList lang="en" search="?status=new" />);
    await screen.findByText("Ada");
    expect(calls.map((c) => c.url)).toEqual(["/ui/api/contact?status=new"]);
  });

  test("shows sender, excerpt, status, language and the open count", async () => {
    serve(() =>
      ({
        body: listing(
          [
            message(),
            message({
              id: "2",
              name: null,
              email: null,
              message: "Hallo, gibt es Android?",
              language: "de",
              status: "done",
              done_at: "2026-10-02T10:00:00Z",
            }),
          ],
          { open: 7 },
        ),
      }),
    );
    render(<ContactList lang="en" search="" />);
    const items = await screen.findAllByTestId("contact");
    expect(items).toHaveLength(2);
    const [first, second] = items as [HTMLElement, HTMLElement];
    expect(within(first).getByText("Ada")).toBeTruthy();
    expect(within(first).getByText("Is there a Mac build of the beta?")).toBeTruthy();
    expect(within(first).getByText("Open")).toBeTruthy();
    expect(within(first).getByText("EN")).toBeTruthy();
    expect(within(second).getByText("No name given")).toBeTruthy();
    expect(within(second).getByText("Hallo, gibt es Android?")).toBeTruthy();
    expect(within(second).getByText("Done")).toBeTruthy();
    expect(within(second).getByText("DE")).toBeTruthy();
    expect(screen.getByText(/^7 open/)).toBeTruthy();
    expect(within(first).getByRole("link").getAttribute("href")).toBe("/contact/0199bbbb-0000-7000-8000-000000000001");
  });

  test("says how many of the total are shown", async () => {
    serve(() => ({ body: listing([message()], { total: 250, open: 3 }) }));
    render(<ContactList lang="en" search="" />);
    expect(await screen.findByText("1 of 250")).toBeTruthy();
  });

  test("says so when there is nothing", async () => {
    serve(() => ({ body: listing([]) }));
    render(<ContactList lang="en" search="?status=done" />);
    expect(await screen.findByText("No messages here.")).toBeTruthy();
  });

  test("shows the service's refusal as an alert", async () => {
    serve(() => ({ status: 500, body: { error: "the store is down" } }));
    render(<ContactList lang="en" search="" />);
    expect((await screen.findByRole("alert")).textContent).toBe("the store is down");
    expect(screen.queryByText("No messages here.")).toBeNull();
  });

  test("says when the service does not answer", async () => {
    serve(() => ({ status: 503, body: {} }));
    render(<ContactList lang="en" search="" />);
    expect(await screen.findByRole("alert")).toBeTruthy();
  });

  test(
    "the Done filter moves to /contact?status=done and asks again",
    async () => {
      const calls = serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url.startsWith("/ui/api/contact")) return { body: listing([message()]) };
        return undefined;
      });
      window.history.pushState(null, "", "/contact");
      render(<App />);
      await screen.findByText("Ada");
      await userEvent.click(screen.getByRole("button", { name: "Done" }));
      await waitFor(() => {
        expect(window.location.pathname).toBe("/contact");
        expect(window.location.search).toBe("?status=done");
      });
      await waitFor(() => {
        expect(calls.some((c) => c.url === "/ui/api/contact?status=done")).toBe(true);
      });
      expect(screen.getByRole("button", { name: "Done" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByRole("button", { name: "All" }).getAttribute("aria-pressed")).toBe("false");
    },
    SLOW,
  );

  test(
    "the All filter drops the query string",
    async () => {
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url.startsWith("/ui/api/contact")) return { body: listing([message()]) };
        return undefined;
      });
      window.history.pushState(null, "", "/contact?status=new");
      render(<App />);
      await screen.findByText("Ada");
      await userEvent.click(screen.getByRole("button", { name: "All" }));
      await waitFor(() => {
        expect(window.location.search).toBe("");
      });
      expect(window.location.pathname).toBe("/contact");
    },
    SLOW,
  );

  test("is German in German", async () => {
    serve(() => ({ body: listing([message({ name: null })]) }));
    render(<ContactList lang="de" search="" />);
    expect((await screen.findByRole("heading", { level: 1 })).textContent).toBe("Kontaktnachrichten");
    expect(screen.getByText("Ohne Namen")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Erledigt" })).toBeTruthy();
    expect(screen.getByText(/^1 offen/)).toBeTruthy();
  });
});

function startPage(first: ContactDetail): Call[] {
  let current = first;
  return serve((call) => {
    if (!call.url.startsWith(`/ui/api/contact/${encodeURIComponent(first.id)}`)) return undefined;
    if (call.method === "PATCH") {
      const change = call.body as { status: ContactDetail["status"] };
      current = {
        ...current,
        status: change.status,
        done_at: change.status === "done" ? "2026-10-03T10:00:00Z" : null,
      };
      return { body: current };
    }
    if (call.method === "DELETE") return { status: 204 };
    return { body: current };
  });
}

describe("one contact message", () => {
  const id = "0199bbbb-0000-7000-8000-000000000001";

  test("loads it by id and shows the full text and the address", async () => {
    const long = "Line one of a long message.\nAnd a second line that the list would cut short.";
    const calls = startPage(detail({ message: long }));
    render(<ContactPage lang="en" id={id} />);
    expect((await screen.findByRole("heading", { level: 1 })).textContent).toBe("Ada");
    expect(calls[0]?.url).toBe(`/ui/api/contact/${id}`);
    expect(calls[0]?.method).toBe("GET");
    expect(
      screen.getByText((_, el) => el?.className === "contact-text" && el.textContent === long),
    ).toBeTruthy();
    expect(screen.getByText("ada@example.org")).toBeTruthy();
    expect(screen.getByText("English")).toBeTruthy();
  });

  test("a reply link opens the mail program with the address and a subject", async () => {
    startPage(detail());
    render(<ContactPage lang="en" id={id} />);
    const link = await screen.findByRole("link", { name: "Reply by e-mail" });
    const href = link.getAttribute("href") ?? "";
    expect(href.startsWith("mailto:ada@example.org?subject=")).toBe(true);
    expect(decodeURIComponent(href.split("subject=")[1] ?? "")).toBe("Your message to Baylee");
  });

  test("the reply subject is in the message's language, not the admin's", async () => {
    startPage(detail({ language: "de", message: "Hallo!" }));
    render(<ContactPage lang="en" id={id} />);
    const link = await screen.findByRole("link", { name: "Reply by e-mail" });
    const href = link.getAttribute("href") ?? "";
    expect(href).toContain("ada@example.org");
    expect(decodeURIComponent(href.split("subject=")[1] ?? "")).toBe("Deine Nachricht an Baylee");
    expect(screen.getByText("Deutsch")).toBeTruthy();
  });

  test("an address with odd characters stays one address", async () => {
    startPage(detail({ email: "a+b&c@example.org" }));
    render(<ContactPage lang="en" id={id} />);
    const href = (await screen.findByRole("link", { name: "Reply by e-mail" })).getAttribute("href") ?? "";
    const to = href.slice("mailto:".length, href.indexOf("?"));
    expect(decodeURIComponent(to)).toBe("a+b&c@example.org");
    expect(to).not.toContain("&");
  });

  test("without an address there is no reply link, and it says why", async () => {
    startPage(detail({ email: null, name: null }));
    render(<ContactPage lang="en" id={id} />);
    expect(await screen.findByText("no address to reply to")).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Reply by e-mail" })).toBeNull();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("No name given");
  });

  test("Mark done patches the status with the CSRF header, then offers Reopen", async () => {
    const calls = startPage(detail());
    render(<ContactPage lang="en" id={id} />);
    await userEvent.click(await screen.findByRole("button", { name: "Mark done" }));
    expect(await screen.findByRole("button", { name: "Reopen" })).toBeTruthy();
    const patch = calls.find((c) => c.method === "PATCH");
    expect(patch?.url).toBe(`/ui/api/contact/${id}`);
    expect(patch?.body).toEqual({ status: "done" });
    expect(patch?.headers["x-baylee-csrf"]).toBe("1");
    expect(screen.queryByRole("button", { name: "Mark done" })).toBeNull();
    expect(screen.getByText("Marked done")).toBeTruthy();
  });

  test("Reopen sends status new", async () => {
    const calls = startPage(detail({ status: "done", done_at: "2026-10-02T10:00:00Z" }));
    render(<ContactPage lang="en" id={id} />);
    await userEvent.click(await screen.findByRole("button", { name: "Reopen" }));
    expect(await screen.findByRole("button", { name: "Mark done" })).toBeTruthy();
    expect(calls.find((c) => c.method === "PATCH")?.body).toEqual({ status: "new" });
  });

  test("a refused change shows the service's sentence and keeps the page", async () => {
    serve((call) => {
      if (call.method === "PATCH") return { status: 409, body: { error: "already changed" } };
      return { body: detail() };
    });
    render(<ContactPage lang="en" id={id} />);
    await userEvent.click(await screen.findByRole("button", { name: "Mark done" }));
    expect((await screen.findByRole("alert")).textContent).toBe("already changed");
    expect(screen.getByRole("button", { name: "Mark done" })).toBeTruthy();
  });

  test("Delete asks first and sends nothing until it is confirmed", async () => {
    const calls = startPage(detail());
    render(<ContactPage lang="en" id={id} />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete" }));
    expect(screen.getByText(/Delete this message for good/)).toBeTruthy();
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Keep it" }));
    expect(screen.queryByText(/Delete this message for good/)).toBeNull();
    expect(screen.getByRole("button", { name: "Delete" })).toBeTruthy();
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
  });

  test("Delete it sends DELETE with the CSRF header and goes back to the list", async () => {
    const calls = startPage(detail());
    window.history.pushState(null, "", `/contact/${id}`);
    render(<ContactPage lang="en" id={id} />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete it" }));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/contact");
    });
    const del = calls.find((c) => c.method === "DELETE");
    expect(del?.url).toBe(`/ui/api/contact/${id}`);
    expect(del?.headers["x-baylee-csrf"]).toBe("1");
  });

  test("the history lists who did what, or says nothing was done", async () => {
    const audit: AuditEntry[] = [
      { at: "2026-10-02T10:00:00Z", actor: "viktor", action: "contact.done", detail: null },
      { at: "2026-10-03T10:00:00Z", actor: "mara", action: "contact.reopen", detail: "wrote back" },
    ];
    startPage(detail({ audit }));
    const { unmount } = render(<ContactPage lang="en" id={id} />);
    const history = (await screen.findByRole("heading", { name: "History" })).parentElement as HTMLElement;
    const entries = within(history).getAllByRole("listitem");
    expect(entries).toHaveLength(2);
    expect(within(entries[0] as HTMLElement).getByText("viktor")).toBeTruthy();
    expect(within(entries[0] as HTMLElement).getByText("contact.done")).toBeTruthy();
    expect(within(entries[1] as HTMLElement).getByText("mara")).toBeTruthy();
    expect(within(entries[1] as HTMLElement).getByText("wrote back")).toBeTruthy();
    expect(screen.queryByText("Nothing done with it yet.")).toBeNull();
    unmount();

    startPage(detail({ audit: [] }));
    render(<ContactPage lang="en" id={id} />);
    expect(await screen.findByText("Nothing done with it yet.")).toBeTruthy();
  });

  test("a message that is gone shows the refusal and a way back", async () => {
    serve(() => ({ status: 404, body: { error: "no such message" } }));
    render(<ContactPage lang="en" id={id} />);
    expect((await screen.findByRole("alert")).textContent).toBe("no such message");
    expect(screen.getByRole("link", { name: /All messages/ }).getAttribute("href")).toBe("/contact");
  });
});

describe("the contact tab", () => {
  test(
    "a signed-in admin without a console sees Reports and Contact only",
    async () => {
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url.startsWith("/ui/api/reports")) return { body: { total: 0, reports: [] } };
        return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
      });
      render(<App />);
      const nav = await screen.findByRole("navigation", { name: "Sections" });
      expect(within(nav).getAllByRole("link").map((a) => a.textContent)).toEqual(["Reports", "Contact"]);
    },
    SLOW,
  );

  test(
    "with the console, Admin comes third",
    async () => {
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor", gateway_admin: true } };
        if (call.url.startsWith("/ui/api/reports")) return { body: { total: 0, reports: [] } };
        return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
      });
      render(<App />);
      const nav = await screen.findByRole("navigation", { name: "Sections" });
      await waitFor(() => {
        expect(within(nav).getAllByRole("link").map((a) => a.textContent)).toEqual(["Reports", "Contact", "Admin"]);
      });
    },
    SLOW,
  );

  test(
    "/contact renders the list with the Contact tab current",
    async () => {
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url.startsWith("/ui/api/contact")) return { body: listing([message()]) };
        return undefined;
      });
      window.history.pushState(null, "", "/contact");
      render(<App />);
      expect(await screen.findByRole("heading", { name: "Contact messages" })).toBeTruthy();
      expect(await screen.findByText("Is there a Mac build of the beta?")).toBeTruthy();
      const nav = screen.getByRole("navigation", { name: "Sections" });
      expect(within(nav).getByRole("link", { name: "Contact" }).getAttribute("aria-current")).toBe("page");
      expect(within(nav).getByRole("link", { name: "Reports" }).getAttribute("aria-current")).toBeNull();
    },
    SLOW,
  );

  test(
    "/contact/{id} renders the message, and the tab stays current",
    async () => {
      const id = "0199bbbb-0000-7000-8000-000000000001";
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url === `/ui/api/contact/${id}`) return { body: detail() };
        return undefined;
      });
      window.history.pushState(null, "", `/contact/${id}`);
      render(<App />);
      expect(await screen.findByRole("link", { name: "Reply by e-mail" })).toBeTruthy();
      const nav = screen.getByRole("navigation", { name: "Sections" });
      expect(within(nav).getByRole("link", { name: "Contact" }).getAttribute("aria-current")).toBe("page");
    },
    SLOW,
  );

  test(
    "clicking a message in the list opens it",
    async () => {
      const id = "0199bbbb-0000-7000-8000-000000000001";
      serve((call) => {
        if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
        if (call.url === `/ui/api/contact/${id}`) return { body: detail() };
        if (call.url.startsWith("/ui/api/contact")) return { body: listing([message()]) };
        return undefined;
      });
      window.history.pushState(null, "", "/contact");
      render(<App />);
      await userEvent.click(await screen.findByRole("link", { name: /Ada/ }));
      expect(window.location.pathname).toBe(`/contact/${id}`);
      expect(await screen.findByRole("button", { name: "Mark done" })).toBeTruthy();
    },
    SLOW,
  );
});
