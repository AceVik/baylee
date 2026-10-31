// Who comes in, with a stand-in service: the mode in force and where it
// comes from, a choice sent as one field, the operator's lock, and the
// warning for a gateway nobody can enter with its two ways out.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import type { AuthModeState } from "../api";
import { serve } from "../test-utils";
import { Access, nobodyComesIn } from "./Access";

vi.setConfig({ testTimeout: 20_000 });

const DEFAULT: AuthModeState = { mode: "central", source: "default", set_at: null, set_by: null, connected: true };

describe("who comes in", () => {
  test("says the default, and saves a choice as one field", async () => {
    const user = userEvent.setup();
    const calls = serve((call) =>
      call.method === "PUT"
        ? { body: { mode: "mixed", source: "console", set_at: "2026-10-30T12:00:00Z", set_by: "viktor", connected: true } }
        : { body: DEFAULT },
    );
    render(<Access lang="en" />);
    expect(await screen.findByText(/the default applies: Baylee accounts only/)).toBeTruthy();
    const save = screen.getByRole("button", { name: "Save" });
    expect((save as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByLabelText(/Baylee accounts and guests/));
    await user.click(save);
    await waitFor(() => expect(calls.filter((c) => c.method === "PUT")).toHaveLength(1));
    const put = calls.find((c) => c.method === "PUT");
    expect(put?.url).toBe("/ui/api/admin/auth-mode");
    expect(put?.body).toEqual({ mode: "mixed" });
    expect(await screen.findByText("Saved. The next sign-in follows it.")).toBeTruthy();
    expect(screen.getByText(/Chosen by viktor/)).toBeTruthy();
  });

  test("locks the choice the operator's environment sets", async () => {
    serve(() => ({ body: { ...DEFAULT, mode: "guests", source: "environment" } }));
    render(<Access lang="en" />);
    expect(await screen.findByText(/Set by the operator \(BAYLEE_AUTH_MODE\)/)).toBeTruthy();
    expect(screen.getByLabelText(/Guests only/).matches(":disabled")).toBe(true);
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
  });

  test("warns when nobody can come in, and points at the Central token", async () => {
    serve(() => ({ body: { ...DEFAULT, connected: false } }));
    render(<Access lang="en" />);
    expect(await screen.findByText(/Nobody can come in/)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Paste a Central token" }).getAttribute("href")).toBe("/admin/central");
    expect(screen.queryByRole("link", { name: "Allow guests" })).toBeNull();
  });

  test("knows when nobody comes in", () => {
    expect(nobodyComesIn("central", false)).toBe(true);
    for (const [mode, connected] of [
      ["central", true],
      ["mixed", false],
      ["guests", false],
      [undefined, false],
      ["central", undefined],
    ] as const) {
      expect(nobodyComesIn(mode, connected)).toBe(false);
    }
  });
});
