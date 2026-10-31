// The console's link to Baylee Central with a stand-in service: the state a
// pasted token is in, a token checked before it is sent, pasting one and
// removing it after a second step; never more of a token than its end.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import type { CentralLink } from "../api";
import { serve } from "../test-utils";
import { Central, TOKEN } from "./Central";

vi.setConfig({ testTimeout: 20_000 });

const LINKED: CentralLink = {
  source: "token",
  token: { last4: "ab12", set_at: "2026-10-30T10:00:00Z", set_by: "viktor" },
  central: {
    gateway: "0192d0c5-7b3e-7a1f-8c2d-3e4f5a6b7c8d",
    origin: "https://gateway.example",
    name: "Synthetic gateway",
    state: "pending",
    sso: true,
    listing: true,
    expires_at: "2027-01-28T10:00:00Z",
  },
  check: "ok",
};

const NONE: CentralLink = { source: "none", token: null, central: null, check: null };

describe("the link to Baylee Central", () => {
  test("shows a pasted token's state, never the token", async () => {
    serve(() => ({ body: LINKED }));
    render(<Central lang="en" />);
    expect(await screen.findByText("bgt_…ab12")).toBeTruthy();
    expect(screen.getByText("Awaiting approval")).toBeTruthy();
    expect(screen.getByText("sign-in · list entry")).toBeTruthy();
    expect(screen.getByText(/Until an administrator of Baylee Central approves/)).toBeTruthy();
  });

  test("says when the operator's file sets the link, and offers no field", async () => {
    serve(() => ({ body: { ...NONE, source: "file" } }));
    render(<Central lang="en" />);
    expect(await screen.findByText(/The operator's file/)).toBeTruthy();
    expect(screen.queryByLabelText("Token")).toBeNull();
  });

  test("checks a token's form before sending it, then links", async () => {
    const user = userEvent.setup();
    const calls = serve((call) => ({ body: call.method === "PUT" ? LINKED : NONE }));
    render(<Central lang="en" />);
    const field = await screen.findByLabelText("Token");
    await user.type(field, "bgt_not-a-token");
    await user.click(screen.getByRole("button", { name: "Link" }));
    expect(screen.getByText("A token starts with bgt_ and has 64 hex digits after it.")).toBeTruthy();
    expect(calls.filter((c) => c.method === "PUT")).toHaveLength(0);
    await user.clear(field);
    const token = `bgt_${"cd".repeat(32)}`;
    await user.type(field, token);
    await user.click(screen.getByRole("button", { name: "Link" }));
    await waitFor(() => expect(calls.filter((c) => c.method === "PUT")).toHaveLength(1));
    const put = calls.find((c) => c.method === "PUT");
    expect(put?.url).toBe("/ui/api/admin/central/token");
    expect(put?.body).toEqual({ token });
    expect(await screen.findByText("Linked. The gateway uses the token now.")).toBeTruthy();
    expect((field as HTMLInputElement).value).toBe("");
  });

  test("removes the token after a second step", async () => {
    const user = userEvent.setup();
    const calls = serve((call) => (call.method === "DELETE" ? { status: 204 } : { body: LINKED }));
    render(<Central lang="en" />);
    await user.click(await screen.findByRole("button", { name: "Remove the token" }));
    expect(calls.filter((c) => c.method === "DELETE")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "Remove the token" }));
    await waitFor(() => expect(calls.filter((c) => c.method === "DELETE")).toHaveLength(1));
  });

  test("knows a token by its form", () => {
    expect(TOKEN.test(`bgt_${"0".repeat(64)}`)).toBe(true);
    for (const bad of ["", "bgt_", `BGT_${"0".repeat(64)}`, `bgt_${"0".repeat(63)}`, `bgt_${"A".repeat(64)}`]) expect(TOKEN.test(bad)).toBe(false);
  });
});
