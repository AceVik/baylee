import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import { App } from "./App";
import { Login } from "./Login";
import { serve, summary } from "./test-utils";

describe("the sign-in form", () => {
  test("says a wrong password or unknown name in one sentence, and keeps the name", async () => {
    const calls = serve(() => ({ status: 401, body: { error: "wrong name or password" } }));
    const onSignedIn = vi.fn<(name: string) => void>();
    render(<Login onSignedIn={onSignedIn} />);
    await userEvent.type(screen.getByLabelText("Name"), " viktor ");
    await userEvent.type(screen.getByLabelText("Password"), "nope");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Wrong name or password.");
    expect(onSignedIn).not.toHaveBeenCalled();
    expect(calls[0]?.body).toEqual({ name: "viktor", password: "nope" });
    expect(calls[0]?.headers["x-baylee-csrf"]).toBe("1");
    expect(screen.getByLabelText("Name")).toHaveProperty("value", " viktor ");
  });

  test("says when sign-in is held back", async () => {
    serve(() => ({ status: 429, body: { error: "too many failed sign-ins; try again later" } }));
    render(<Login onSignedIn={vi.fn<(name: string) => void>()} />);
    await userEvent.type(screen.getByLabelText("Name"), "a");
    await userEvent.type(screen.getByLabelText("Password"), "b");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Too many failed sign-ins/);
  });

  test("says when the service does not answer", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("offline"))));
    render(<Login onSignedIn={vi.fn<(name: string) => void>()} />);
    await userEvent.type(screen.getByLabelText("Name"), "a");
    await userEvent.type(screen.getByLabelText("Password"), "b");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect((await screen.findByRole("alert")).textContent).toBe("The service did not answer.");
  });

  test("hands the name on after a sign-in and forgets the password", async () => {
    serve(() => ({ body: { name: "viktor" } }));
    const onSignedIn = vi.fn<(name: string) => void>();
    render(<Login onSignedIn={onSignedIn} />);
    await userEvent.type(screen.getByLabelText("Name"), "viktor");
    await userEvent.type(screen.getByLabelText("Password"), "right");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => {
      expect(onSignedIn).toHaveBeenCalledWith("viktor");
    });
    expect(screen.getByLabelText("Password")).toHaveProperty("value", "");
  });
});

describe("the app", () => {
  test("shows the sign-in form without a session, and the list after signing in", async () => {
    let signedIn = false;
    serve((call) => {
      if (call.url === "/ui/api/me") return signedIn ? { body: { name: "viktor" } } : { status: 401 };
      if (call.url === "/ui/api/login") {
        signedIn = true;
        return { body: { name: "viktor" } };
      }
      if (!signedIn) return { status: 401 };
      if (call.url.startsWith("/ui/api/reports")) return { body: { total: 1, reports: [summary()] } };
      if (call.url === "/ui/api/facets") return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
      return undefined;
    });
    render(<App />);
    await userEvent.type(await screen.findByLabelText("Name"), "viktor");
    await userEvent.type(screen.getByLabelText("Password"), "right");
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("The Swamp untapped by itself.")).toBeTruthy();
    expect(screen.getByText("viktor")).toBeTruthy();
  });

  test("goes back to the sign-in form when a request finds the session gone", async () => {
    let lapsed = false;
    serve((call) => {
      if (lapsed) return { status: 401, body: { error: "not signed in" } };
      if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
      if (call.url.startsWith("/ui/api/reports")) return { body: { total: 1, reports: [summary()] } };
      if (call.url === "/ui/api/facets") return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
      return undefined;
    });
    render(<App />);
    expect(await screen.findByText("The Swamp untapped by itself.")).toBeTruthy();
    lapsed = true;
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "crash");
    expect(await screen.findByRole("button", { name: "Sign in" })).toBeTruthy();
  });

  test("signs out on the server and shows the form", async () => {
    const calls = serve((call) => {
      if (call.url === "/ui/api/me") return { body: { name: "viktor" } };
      if (call.url === "/ui/api/logout") return { status: 204 };
      if (call.url.startsWith("/ui/api/reports")) return { body: { total: 0, reports: [] } };
      return { body: { gateways: [], statuses: [], kinds: [], reporters: [] } };
    });
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "Sign out" }));
    expect(await screen.findByRole("button", { name: "Sign in" })).toBeTruthy();
    const out = calls.find((c) => c.url === "/ui/api/logout");
    expect(out?.method).toBe("POST");
    expect(out?.headers["x-baylee-csrf"]).toBe("1");
  });
});
