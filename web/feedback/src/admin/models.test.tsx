import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import { Admin } from "../Admin";
import type { LlmProfile, SeatHost } from "../api";
import { parseRoute } from "../router";
import { serve, type Call } from "../test-utils";
import { formatUntil, groupProfiles, readDefinition } from "./Models";

function profile(over: Partial<LlmProfile> = {}): LlmProfile {
  return {
    id: "sonnet",
    label: "Sonnet",
    vendor: "Anthropic",
    kind: "api",
    model: "claude-sonnet-5-5",
    state: "available",
    until_unix: null,
    games: 1,
    max_games: 4,
    enabled: true,
    canary: false,
    caps: { day_usd: 10, month_usd: 100 },
    spent: { day_usd: 0.42, month_usd: 3.1, day_tokens: 0, month_tokens: 0 },
    key: "kept",
    last_error: null,
    last_ok_unix: null,
    definition: {
      label: "Sonnet",
      vendor: "Anthropic",
      enabled: true,
      max_games: 4,
      caps: { day_usd: 10, month_usd: 100 },
      profile: { provider: "anthropic", model: "claude-sonnet-5-5" },
    },
    ...over,
  };
}

const HOSTS: SeatHost[] = [
  { name: "alpha", local: true, capacity: 4, games: 1, connected_secs: 3600, profiles: [profile()] },
  {
    name: "beta",
    local: false,
    capacity: 0,
    games: 0,
    connected_secs: 60,
    profiles: [
      profile({ state: "exhausted", until_unix: 1_791_800_000, games: 0 }),
      profile({ id: "cc", label: "Claude CLI", kind: "cli", key: "none_needed", state: "failing", last_error: "program missing", definition: { label: "Claude CLI", vendor: "Anthropic", profile: { provider: "cli", model: "claude:opus" } } }),
    ],
  },
];

function site(calls: { hosts?: SeatHost[] } = {}): Call[] {
  return serve((call) => {
    if (call.url === "/ui/api/admin/llm/seathosts") return { body: { seathosts: calls.hosts ?? HOSTS } };
    if (call.url === "/ui/api/admin/audit")
      return {
        body: [
          { at: "2026-10-10T12:00:00Z", actor: "owner", action: "gateway.llm.key.set", detail: "alpha/sonnet" },
          { at: "2026-10-10T11:00:00Z", actor: "owner", action: "gateway.invite.create", detail: "k1" },
        ],
      };
    if (call.url.startsWith("/ui/api/admin/llm/seathosts/")) {
      return { status: call.method === "PUT" ? 200 : call.url.endsWith("/probe") ? 202 : 204, body: { ok: true } };
    }
    return undefined;
  });
}

const changes = (calls: Call[]) => calls.filter((c) => c.method !== "GET");

describe("the models page", () => {
  test("its route", () => {
    expect(parseRoute("/admin/models", "")).toEqual({ page: "admin", section: "models" });
  });

  test("profiles group by id, best state first", () => {
    const groups = groupProfiles(HOSTS);
    expect(groups.map((g) => [g.id, g.on.map((o) => o.host)])).toEqual([
      ["sonnet", ["alpha", "beta"]],
      ["cc", ["beta"]],
    ]);
  });

  test("an until today says only the time", () => {
    const at = new Date(2026, 9, 10, 14, 30).getTime();
    expect(formatUntil("de", at / 1000, at)).toBe("bis 14:30");
  });

  test("the form reads into a definition, or says why not", () => {
    const form = { label: "X", vendor: "Y", enabled: true, canary: false, maxGames: "", dayUsd: "5", monthUsd: "", profile: "{}" };
    expect(readDefinition(form)).toEqual({ label: "X", vendor: "Y", enabled: true, profile: {}, caps: { day_usd: 5 } });
    expect(readDefinition({ ...form, profile: "[1]" })).toBe("llm.badProfile");
    expect(readDefinition({ ...form, label: " " })).toBe("llm.badNames");
  });

  test("seat agents, profiles and only the models' changes are shown", async () => {
    site();
    render(<Admin lang="en" section="models" />);
    expect(await screen.findAllByTestId("llm-host")).toHaveLength(2);
    const profiles = screen.getAllByTestId("llm-profile");
    expect(profiles).toHaveLength(2);
    expect(within(profiles[0] as HTMLElement).getAllByTestId("llm-on")).toHaveLength(2);
    expect(within(profiles[0] as HTMLElement).getByText(/exhausted · until/)).toBeTruthy();
    expect(screen.getByText("program missing")).toBeTruthy();
    expect(await screen.findByText("key.set")).toBeTruthy();
    expect(screen.queryByText(/invite/)).toBeNull();
  });

  test("a key is sent once and cleared; nothing shows it again", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    await user.click(await screen.findByRole("button", { name: "Key… sonnet @ alpha" }));
    const field = screen.getByLabelText("New key for sonnet on alpha");
    expect(field.getAttribute("type")).toBe("password");
    await user.type(field, "sk-secret-123");
    await user.click(screen.getByRole("button", { name: "Send key" }));
    await screen.findByText("Key sent.");
    expect(changes(calls)).toEqual([
      expect.objectContaining({ method: "POST", url: "/ui/api/admin/llm/seathosts/alpha/profiles/sonnet/key", body: { key: "sk-secret-123" } }),
    ]);
    expect(document.body.innerHTML).not.toContain("sk-secret-123");
  });

  test("deleting asks first; no answers nothing", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    await user.click(await screen.findByRole("button", { name: "Delete cc @ beta" }));
    const dialog = screen.getByRole("alertdialog");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(changes(calls)).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Delete cc @ beta" }));
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }));
    await screen.findByText("Profile removed.");
    expect(changes(calls)).toEqual([expect.objectContaining({ method: "DELETE", url: "/ui/api/admin/llm/seathosts/beta/profiles/cc" })]);
  });

  test("switching, probing and assigning", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    await user.click(await screen.findByRole("button", { name: "Enabled sonnet @ alpha" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    await user.click(screen.getByRole("button", { name: "Probe sonnet @ alpha" }));
    await user.click(screen.getByRole("checkbox", { name: "cc @ alpha" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(3);
    });
    expect(changes(calls)).toEqual([
      expect.objectContaining({ url: "/ui/api/admin/llm/seathosts/alpha/profiles/sonnet/enabled", body: { enabled: false } }),
      expect.objectContaining({ url: "/ui/api/admin/llm/seathosts/alpha/profiles/sonnet/probe" }),
      expect.objectContaining({
        method: "PUT",
        url: "/ui/api/admin/llm/seathosts/alpha/profiles/cc",
        body: { label: "Claude CLI", vendor: "Anthropic", profile: { provider: "cli", model: "claude:opus" } },
      }),
    ]);
  });

  test("a new profile is written to the chosen seat agent", async () => {
    const calls = site();
    const user = userEvent.setup();
    render(<Admin lang="de" section="models" />);
    await user.click(await screen.findByRole("button", { name: "Profil hinzufügen" }));
    const form = screen.getByRole("dialog");
    await user.selectOptions(within(form).getByLabelText("Sitz-Agent"), "beta");
    await user.type(within(form).getByLabelText("Kennung"), "opus");
    await user.type(within(form).getByLabelText("Name"), "Opus");
    await user.type(within(form).getByLabelText("Anbieter (Spieldaten gehen an)"), "Anthropic");
    await user.type(within(form).getByLabelText("Spiele gleichzeitig"), "2");
    await user.click(within(form).getByRole("button", { name: "Speichern" }));
    await screen.findByText("Profil gespeichert.");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(changes(calls)).toEqual([
      expect.objectContaining({
        method: "PUT",
        url: "/ui/api/admin/llm/seathosts/beta/profiles/opus",
        body: { label: "Opus", vendor: "Anthropic", enabled: true, max_games: 2, profile: { provider: "anthropic", model: "" } },
      }),
    ]);
  });

  test("no seat agent: said, and nothing to add to", async () => {
    site({ hosts: [] });
    render(<Admin lang="en" section="models" />);
    expect(await screen.findByText(/No seat agent is connected/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Add profile" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
