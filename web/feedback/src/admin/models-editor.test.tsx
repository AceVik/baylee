// The profile editor of the models page: its sections and hints, the fallback
// it offers, the budget boxes, the question before a save and a refusal the
// server answers (beta.8). `models.test.tsx` holds the page; this holds the form.

import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import { Admin } from "../Admin";
import type { LlmDefinition, LlmProfile, SeatHost } from "../api";
import { t, type Key } from "../i18n";
import { serve, type Call } from "../test-utils";
import { BUDGET_KEYS, chainOf, fallbackChoices, otherSettings, readDefinition, type DefinitionForm } from "./Models";

// Typing through user-event in jsdom is slow, and a loaded machine slower.
vi.setConfig({ testTimeout: 20_000 });

function profile(id: string, label: string, over: Partial<LlmProfile> = {}): LlmProfile {
  const vendor = over.vendor ?? "Example";
  return {
    id,
    label,
    vendor,
    kind: "api",
    model: `model-${id}`,
    state: "available",
    until_unix: null,
    games: 0,
    max_games: 4,
    enabled: true,
    canary: false,
    caps: null,
    spent: null,
    key: "kept",
    last_error: null,
    last_ok_unix: null,
    definition: {
      label,
      vendor,
      enabled: true,
      max_games: 4,
      caps: { day_usd: 10, month_usd: 100 },
      profile: { provider: "anthropic", model: `model-${id}` },
    },
    ...over,
  };
}

function withDefinition(p: LlmProfile, over: Partial<LlmDefinition>): LlmProfile {
  return { ...p, definition: { ...p.definition, ...over } };
}

const cliProfile = (): LlmProfile =>
  profile("cc", "Agent CLI", {
    kind: "cli",
    key: "none_needed",
    definition: { label: "Agent CLI", vendor: "Example", profile: { provider: "cli", model: "claude:opus" } },
  });

const seatHost = (name: string, profiles: LlmProfile[]): SeatHost => ({
  name,
  local: false,
  capacity: 0,
  games: 0,
  connected_secs: 60,
  profiles,
});

/**
 * Two seat agents. `alpha` holds the profile edited ("primary"), two more of its
 * vendor whose labels are out of insertion order, one of another vendor, and an
 * agent CLI; `beta` holds one more API profile. Each exclusion of the fallback
 * select therefore has a concrete candidate it rules out.
 */
function fleet(primary: LlmProfile = profile("primary", "Model A"), cc: LlmProfile = cliProfile()): SeatHost[] {
  return [
    seatHost("alpha", [primary, profile("spare", "Model C"), profile("backup", "Model B"), profile("other", "Model D", { vendor: "Other" }), cc]),
    seatHost("beta", [profile("remote", "Model E")]),
  ];
}

interface Server {
  hosts: SeatHost[];
  /** What a PUT of a profile is answered with; a test may change it. */
  put: { status: number; body: unknown };
}

function site(hosts: SeatHost[] = fleet()): { calls: Call[]; server: Server } {
  const server: Server = { hosts, put: { status: 200, body: { ok: true } } };
  const calls = serve((call) => {
    if (call.url === "/ui/api/admin/llm/seathosts") return { body: { seathosts: server.hosts } };
    if (call.url === "/ui/api/admin/llm/access") return { body: { profiles: [] } };
    if (call.url === "/ui/api/admin/audit") return { body: [] };
    if (call.url.startsWith("/ui/api/admin/llm/seathosts/")) return call.method === "PUT" ? server.put : { status: 204 };
    return undefined;
  });
  return { calls, server };
}

const changes = (calls: Call[]) => calls.filter((c) => c.method !== "GET");

const PUT_SONNET = "/ui/api/admin/llm/seathosts/alpha/profiles/primary";

type User = ReturnType<typeof userEvent.setup>;

async function openEdit(user: User, button: string, dialog: string): Promise<HTMLElement> {
  await user.click(await screen.findByRole("button", { name: button }));
  return screen.getByRole("dialog", { name: dialog });
}

const select = (editor: HTMLElement, label: string) => within(editor).getByLabelText(label) as HTMLSelectElement;
const input = (editor: HTMLElement, label: string) => within(editor).getByLabelText(label) as HTMLInputElement;
const optionsOf = (s: HTMLSelectElement) => Array.from(s.options).map((o) => o.textContent);
const valuesOf = (s: HTMLSelectElement) => Array.from(s.options).map((o) => o.value);

/** The text of what an element's `aria-describedby` names. */
function describedBy(el: HTMLElement): string {
  return (el.getAttribute("aria-describedby") ?? "")
    .split(" ")
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ")
    .trim();
}

/** Presses the editor's own Save, which asks first; returns the question. */
async function ask(user: User, editor: HTMLElement, save = "Save"): Promise<HTMLElement> {
  await user.click(within(editor).getByRole("button", { name: save }));
  return screen.getByRole("alertdialog");
}

const FORM: DefinitionForm = {
  label: "X",
  vendor: "Y",
  enabled: true,
  canary: false,
  maxGames: "",
  dayUsd: "",
  monthUsd: "",
  profile: "{}",
};

/** The definition a form reads into; a refusal fails the test. */
function read(form: DefinitionForm): LlmDefinition {
  const result = readDefinition(form);
  if (typeof result === "string") throw new Error(`the form was refused: ${result}`);
  return result;
}

describe("what the form reads", () => {
  test("the budget boxes go into the profile, whole counts floored", () => {
    const definition = read({
      ...FORM,
      profile: '{"provider":"anthropic","model":"m"}',
      gameUsd: "2.5",
      gameTokens: "1000.9",
      gameCalls: "12.7",
      priceIn: "3",
      priceOut: "15.5",
    });
    expect(definition).toStrictEqual({
      label: "X",
      vendor: "Y",
      enabled: true,
      profile: { provider: "anthropic", model: "m", game_usd: 2.5, game_tokens: 1000, game_calls: 12, price: { input: 3, output: 15.5 } },
    });
  });

  test("the boxes own the budget: the JSON's own keys give way, an empty box is an absent key", () => {
    const json = JSON.stringify({ provider: "anthropic", game_usd: 99, game_tokens: 7, game_calls: 3, price: { input: 1, output: 2 } });
    // All boxes empty: whatever the JSON said of the budget is gone.
    expect(read({ ...FORM, profile: json }).profile).toStrictEqual({ provider: "anthropic" });
    // One box filled: it replaces its own key; the others stay gone.
    expect(read({ ...FORM, profile: json, gameUsd: "5" }).profile).toStrictEqual({ provider: "anthropic", game_usd: 5 });
    // Each box replaces its own key, price included.
    expect(read({ ...FORM, profile: json, gameUsd: "5", gameTokens: "8", gameCalls: "9", priceIn: "4", priceOut: "6" }).profile).toStrictEqual({
      provider: "anthropic",
      game_usd: 5,
      game_tokens: 8,
      game_calls: 9,
      price: { input: 4, output: 6 },
    });
  });

  test("a price is both of its boxes or neither", () => {
    expect(readDefinition({ ...FORM, priceIn: "3" })).toBe("llm.badPrice");
    expect(readDefinition({ ...FORM, priceOut: "15" })).toBe("llm.badPrice");
    expect(readDefinition({ ...FORM, priceIn: "3", priceOut: "" })).toBe("llm.badPrice");
    expect(read({ ...FORM, priceIn: "", priceOut: "" }).profile).not.toHaveProperty("price");
    expect(read({ ...FORM, priceIn: "3", priceOut: "15" }).profile).toStrictEqual({ price: { input: 3, output: 15 } });
  });

  test("the caps: the four boxes, token caps floored, none at all is no caps", () => {
    expect(read({ ...FORM, dayUsd: "5", monthUsd: "50", dayTokens: "1000.7", monthTokens: "9000000" }).caps).toStrictEqual({
      day_usd: 5,
      month_usd: 50,
      day_tokens: 1000,
      month_tokens: 9_000_000,
    });
    expect(read({ ...FORM, dayTokens: "20" }).caps).toStrictEqual({ day_tokens: 20 });
    expect(read({ ...FORM, monthTokens: "30" }).caps).toStrictEqual({ month_tokens: 30 });
    expect(read(FORM)).not.toHaveProperty("caps");
    expect(read({ ...FORM, dayTokens: "", monthTokens: "" })).not.toHaveProperty("caps");
  });

  test("a fallback: empty or blank is absent, one is written as its id, a chain as the list", () => {
    expect(read({ ...FORM, fallback: [] })).not.toHaveProperty("fallback");
    expect(read({ ...FORM, fallback: [""] })).not.toHaveProperty("fallback");
    expect(read({ ...FORM, fallback: ["  ", ""] })).not.toHaveProperty("fallback");
    expect(read(FORM)).not.toHaveProperty("fallback");
    expect(read({ ...FORM, fallback: ["x"] })).toStrictEqual({ label: "X", vendor: "Y", enabled: true, profile: {}, fallback: "x" });
    expect(read({ ...FORM, fallback: [" x "] }).fallback).toBe("x");
    expect(read({ ...FORM, fallback: ["x", " y", ""] }).fallback).toStrictEqual(["x", "y"]);
    // Named twice is once, in the first place it was named.
    expect(read({ ...FORM, fallback: ["x", "y", "x"] }).fallback).toStrictEqual(["x", "y"]);
  });

  test("a definition's fallbacks read as a chain", () => {
    expect(chainOf(undefined)).toStrictEqual([]);
    expect(chainOf("x")).toStrictEqual(["x"]);
    expect(chainOf(["x", "y"])).toStrictEqual(["x", "y"]);
  });

  test("games at once is floored, a canary is written only when set", () => {
    expect(read({ ...FORM, maxGames: "3.9" }).max_games).toBe(3);
    expect(read(FORM)).not.toHaveProperty("max_games");
    expect(read({ ...FORM, canary: true }).canary).toBe(true);
    expect(read(FORM)).not.toHaveProperty("canary");
  });

  test("the profile's other settings are exactly what is not a budget key", () => {
    expect(BUDGET_KEYS).toEqual(["game_usd", "game_tokens", "game_calls", "price"]);
    const settings = {
      provider: "openai-compatible",
      model: "m",
      base_url: "https://x.example/v1",
      key_env: "X_KEY",
      options: { game_usd: 1 },
      game_usd: 1,
      game_tokens: 2,
      game_calls: 3,
      price: { input: 4, output: 5 },
    };
    const before = structuredClone(settings);
    expect(otherSettings(settings)).toStrictEqual({
      provider: "openai-compatible",
      model: "m",
      base_url: "https://x.example/v1",
      key_env: "X_KEY",
      options: { game_usd: 1 },
    });
    expect(settings).toStrictEqual(before);
    // Each budget key alone is stripped, and a key that only looks like one stays.
    for (const key of BUDGET_KEYS) expect(otherSettings({ [key]: 1, keep: true })).toStrictEqual({ keep: true });
    expect(otherSettings({ prices: 1, game_usd_cap: 2, price_in: 3 })).toStrictEqual({ prices: 1, game_usd_cap: 2, price_in: 3 });
  });
});

describe("the fallbacks a profile may name", () => {
  test("only API profiles of that seat agent and vendor, never the profile itself, sorted by label", () => {
    const hosts = fleet();
    expect(fallbackChoices(hosts, "alpha", "primary", "Example").map((p) => p.id)).toEqual(["backup", "spare"]);
    // Another seat agent's profile is not on this one.
    expect(fallbackChoices(hosts, "beta", "primary", "Example").map((p) => p.id)).toEqual(["remote"]);
    expect(fallbackChoices(hosts, "nowhere", "primary", "Example")).toEqual([]);
    // Never itself: a profile on its own id is left out, whichever one it is.
    expect(fallbackChoices(hosts, "alpha", "backup", "Example").map((p) => p.id)).toEqual(["primary", "spare"]);
    // A new profile has no id yet: every API profile of the vendor is a candidate.
    expect(fallbackChoices(hosts, "alpha", "", "Example").map((p) => p.id)).toEqual(["primary", "backup", "spare"]);
  });

  test("an agent CLI is never offered, though it is of the vendor", () => {
    expect(fallbackChoices(fleet(), "alpha", "primary", "Example").map((p) => p.id)).not.toContain("cc");
  });

  test("a profile of another vendor on the same seat agent is not offered", () => {
    const hosts = fleet();
    expect(fallbackChoices(hosts, "alpha", "primary", "Example").map((p) => p.id)).not.toContain("other");
    expect(fallbackChoices(hosts, "alpha", "primary", "Other").map((p) => p.id)).toEqual(["other"]);
    expect(fallbackChoices(hosts, "alpha", "primary", "")).toEqual([]);
  });

  test("the vendor is compared without case and outer blanks", () => {
    const hosts = [seatHost("alpha", [profile("a", "A", { vendor: " EXAMPLE " }), profile("b", "B", { vendor: "example" }), profile("c", "C", { vendor: "Example Inc" })])];
    expect(fallbackChoices(hosts, "alpha", "x", "  Example").map((p) => p.id)).toEqual(["a", "b"]);
  });

  test("equal labels are told apart by id", () => {
    const hosts = [seatHost("alpha", [profile("b", "Same"), profile("a", "Same"), profile("c", "Before")])];
    expect(fallbackChoices(hosts, "alpha", "x", "Example").map((p) => p.id)).toEqual(["c", "a", "b"]);
  });
});

describe("the profile editor", () => {
  test("it is in sections, and every field but a cap has a hint it is described by", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    const group = (name: string) => within(editor).getByRole("group", { name });

    const profileGroup = group("Profile");
    for (const label of ["Label", "Vendor (game data goes to)", "Enabled", "Canary"]) expect(within(profileGroup).getByLabelText(label)).toBeTruthy();
    const load = group("Games and fallback");
    for (const label of ["Games at once", "Fallback profile"]) expect(within(load).getByLabelText(label)).toBeTruthy();
    const budget = group("Budget per game");
    for (const label of ["Per game (USD)", "Tokens per game", "Calls per game", "Price in (USD / M tokens)", "Price out (USD / M tokens)"]) {
      expect(within(budget).getByLabelText(label)).toBeTruthy();
    }
    // The two price boxes are one field: a group of their own, named for the pair.
    const price = within(budget).getByRole("group", { name: "Price (USD / M tokens, in · out)" });
    for (const label of ["Price in (USD / M tokens)", "Price out (USD / M tokens)"]) expect(within(price).getByLabelText(label)).toBeTruthy();
    expect(within(price).queryByLabelText("Per game (USD)")).toBeNull();
    const caps = group("Caps of its spend book");
    for (const label of ["Cap per day (USD)", "Cap per month (USD)", "Tokens per day", "Tokens per month"]) {
      expect(within(caps).getByLabelText(label)).toBeTruthy();
    }
    // The caps share one sentence, written in their section.
    expect(within(caps).getByText(t("en", "llm.hint.caps"))).toBeTruthy();
    expect(within(group("Model")).getByLabelText("Profile")).toBeTruthy();

    const hints: [string, Key][] = [
      ["Enabled", "llm.hint.enabled"],
      ["Canary", "llm.hint.canary"],
      ["Games at once", "llm.hint.maxGames"],
      ["Fallback profile", "llm.hint.fallback"],
      ["Per game (USD)", "llm.hint.gameUsd"],
      ["Tokens per game", "llm.hint.gameTokens"],
      ["Calls per game", "llm.hint.gameCalls"],
      ["Price in (USD / M tokens)", "llm.hint.price"],
      ["Price out (USD / M tokens)", "llm.hint.price"],
      ["Profile", "llm.profileHint"],
    ];
    // Label and text together, so a failure names the field.
    const described = hints.map(([label]) => [label, describedBy(within(editor).getByLabelText(label))]);
    expect(described).toEqual(hints.map(([label, key]) => [label, t("en", key)]));
    expect(described.filter(([, text]) => text === "")).toEqual([]);
    // Each hint is one element: no id is written twice in the form.
    const ids = Array.from(editor.querySelectorAll("[id]")).map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("the fallback offers API profiles of this seat agent and vendor, not itself, a CLI or another vendor's", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(fallback.disabled).toBe(false);
    // Model B before Model C: by label, not as the seat agent listed them.
    expect(optionsOf(fallback)).toEqual(["none", "Model B · backup", "Model C · spare"]);
    expect(valuesOf(fallback)).toEqual(["", "backup", "spare"]);
    expect(fallback.value).toBe("");
    // Not Model E (beta's), not Model D (Other's), not the CLI, not Model A itself.
    const text = optionsOf(fallback).join("|");
    for (const left of ["Model A", "Agent CLI", "Model E", "Model D"]) expect(text).not.toContain(left);
    expect(describedBy(fallback)).toContain("same vendor");
    expect(describedBy(fallback)).toBe(t("en", "llm.hint.fallback"));
  });

  test("the choices follow the vendor box", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    const vendor = input(editor, "Vendor (game data goes to)");
    await user.clear(vendor);
    await user.type(vendor, "other ");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Model D · other"]);
    await user.clear(vendor);
    await user.type(vendor, "Nobody");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none"]);
  });

  test("a new profile's choices follow its vendor, its seat agent and its id", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    await user.click(await screen.findByRole("button", { name: "Add profile" }));
    const editor = screen.getByRole("dialog", { name: "Add profile" });
    // No vendor yet: nothing shares it.
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none"]);
    await user.type(input(editor, "Vendor (game data goes to)"), "Example");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Model A · primary", "Model B · backup", "Model C · spare"]);
    // The id typed is the profile itself.
    await user.type(input(editor, "Id"), "backup");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Model A · primary", "Model C · spare"]);
    // Another seat agent holds other profiles.
    await user.selectOptions(select(editor, "Seat agent"), "beta");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Model E · remote"]);
    await user.clear(input(editor, "Id"));
    await user.type(input(editor, "Id"), "remote");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none"]);
  });

  test("a save asks first; no leaves the form as it was, yes writes the fallback and the budget", async () => {
    const { calls } = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    await user.selectOptions(select(editor, "Fallback profile"), "backup");
    await user.type(input(editor, "Per game (USD)"), "2");
    await user.type(input(editor, "Tokens per game"), "100000");
    await user.type(input(editor, "Calls per game"), "50");
    await user.type(input(editor, "Price in (USD / M tokens)"), "3");
    await user.type(input(editor, "Price out (USD / M tokens)"), "15");
    await user.type(input(editor, "Tokens per day"), "1000000");

    // The question names what is saved and for whom; nothing is sent yet.
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    const question = screen.getByRole("alertdialog", { name: "Save primary @ alpha? Games already running keep what they started with." });
    expect(changes(calls)).toEqual([]);

    // No: the question closes, the editor and what was typed stay, nothing is sent.
    await user.click(within(question).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Edit primary on alpha" })).toBe(editor);
    expect(changes(calls)).toEqual([]);
    expect(select(editor, "Fallback profile").value).toBe("backup");
    expect(input(editor, "Tokens per game").value).toBe("100000");
    expect((within(editor).getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(false);

    // Yes: the one write.
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(changes(calls)).toEqual([
      expect.objectContaining({
        method: "PUT",
        url: PUT_SONNET,
        body: {
          label: "Model A",
          vendor: "Example",
          enabled: true,
          max_games: 4,
          caps: { day_usd: 10, month_usd: 100, day_tokens: 1_000_000 },
          fallback: "backup",
          profile: {
            provider: "anthropic",
            model: "model-primary",
            game_usd: 2,
            game_tokens: 100_000,
            game_calls: 50,
            price: { input: 3, output: 15 },
          },
        },
      }),
    ]);
  });

  test("a refusal from the server is said in the editor, which stays open and can be saved again", async () => {
    const { calls, server } = site();
    server.put = { status: 400, body: { error: "a profile cannot be its own fallback" } };
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    await user.selectOptions(select(editor, "Fallback profile"), "backup");
    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));

    const refusal = await within(editor).findByText("Not saved: a profile cannot be its own fallback");
    expect(refusal.getAttribute("role")).toBe("alert");
    expect(within(editor).getByRole("alert")).toBe(refusal);
    // The question is gone, the editor is still the one open, with what was chosen.
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Edit primary on alpha" })).toBe(editor);
    expect(select(editor, "Fallback profile").value).toBe("backup");
    expect(changes(calls)).toHaveLength(1);
    // Nothing was said behind the dialog: no notice, no alert on the page.
    expect(screen.queryByText("Profile saved.")).toBeNull();
    expect(document.querySelector(".llm-notice")?.textContent).toBe("");
    expect(screen.getAllByRole("alert")).toEqual([refusal]);

    // The server relents: the same editor saves, the refusal goes with the question.
    server.put = { status: 200, body: { ok: true } };
    await ask(user, editor);
    expect(within(editor).queryByRole("alert")).toBeNull();
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    expect(screen.queryByRole("dialog")).toBeNull();
    const puts = changes(calls);
    expect(puts).toHaveLength(2);
    expect(puts[1]).toEqual(puts[0]);
  });

  test("an agent CLI's editor offers API fallbacks of its vendor and keeps its chain", async () => {
    const { calls } = site(fleet(undefined, withDefinition(cliProfile(), { fallback: "backup" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit cc @ alpha", "Edit cc on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(fallback.disabled).toBe(false);
    expect(fallback.value).toBe("backup");
    expect(describedBy(fallback)).toBe(t("en", "llm.hint.fallbackCli"));
    expect(describedBy(fallback)).toContain("same model behind an API first");

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put, ...rest] = changes(calls);
    expect(rest).toEqual([]);
    expect(put?.url).toBe("/ui/api/admin/llm/seathosts/alpha/profiles/cc");
    expect(put?.body).toEqual({
      label: "Agent CLI",
      vendor: "Example",
      enabled: true,
      profile: { provider: "cli", model: "claude:opus" },
      fallback: "backup",
    });
  });

  test("a chain: each fallback adds the next select, none ends the chain there, the list is written", async () => {
    const { calls } = site(fleet(withDefinition(profile("primary", "Model A"), { fallback: ["backup", "spare"] })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    expect(select(editor, "Fallback profile").value).toBe("backup");
    expect(select(editor, "Fallback 2").value).toBe("spare");
    // One more to add the next; nothing earlier in the chain is offered again.
    expect(select(editor, "Fallback 3").value).toBe("");
    expect(optionsOf(select(editor, "Fallback 2"))).toEqual(["none", "Model C · spare"]);
    expect(optionsOf(select(editor, "Fallback 3"))).toEqual(["none"]);

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put] = changes(calls);
    expect(put?.body).toEqual(expect.objectContaining({ fallback: ["backup", "spare"] }));

    // None in the first place ends the chain there: nothing is written.
    const again = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    await user.selectOptions(select(again, "Fallback profile"), "");
    expect(within(again).queryByRole("combobox", { name: "Fallback 2" })).toBeNull();
    await ask(user, again);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findAllByText("Profile saved.");
    const puts = changes(calls);
    expect(puts.at(-1)?.body).not.toHaveProperty("fallback");
  });

  test("the profile list says a chain in its order", async () => {
    site(fleet(withDefinition(profile("primary", "Model A"), { fallback: ["backup", "spare"] })));
    render(<Admin lang="en" section="models" />);
    expect(await screen.findByText("Fallback: backup → spare")).toBeTruthy();
  });

  test("a price needs both boxes: said in the editor, nothing asked, nothing sent", async () => {
    const { calls } = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    await user.type(input(editor, "Price in (USD / M tokens)"), "3");
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    expect(within(editor).getByRole("alert").textContent).toBe("A price needs both: in and out.");
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(changes(calls)).toEqual([]);

    await user.type(input(editor, "Price out (USD / M tokens)"), "15");
    const question = await ask(user, editor);
    expect(within(editor).queryByRole("alert")).toBeNull();
    await user.click(within(question).getByRole("button", { name: "Cancel" }));
    expect(changes(calls)).toEqual([]);
  });

  test("a profile that is not a JSON object is said, not asked about", async () => {
    const { calls } = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    fireEvent.change(within(editor).getByLabelText("Profile"), { target: { value: "[1]" } });
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    expect(within(editor).getByRole("alert").textContent).toBe("The profile is not a JSON object.");
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(changes(calls)).toEqual([]);
  });

  test("a profile with a budget shows it in its boxes, not in the JSON, and saves back what it was", async () => {
    const stored: LlmDefinition = {
      label: "Model A",
      vendor: "Example",
      enabled: true,
      max_games: 4,
      canary: true,
      fallback: "backup",
      caps: { day_usd: 10, month_usd: 100, day_tokens: 5_000_000, month_tokens: 90_000_000 },
      profile: {
        provider: "anthropic",
        model: "model-primary",
        key_env: "ANTHROPIC_API_KEY",
        game_usd: 3,
        game_tokens: 2_000_000,
        game_calls: 200,
        price: { input: 3, output: 15 },
      },
    };
    const { calls } = site(fleet({ ...profile("primary", "Model A"), definition: stored }));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");

    expect(input(editor, "Per game (USD)").value).toBe("3");
    expect(input(editor, "Tokens per game").value).toBe("2000000");
    expect(input(editor, "Calls per game").value).toBe("200");
    expect(input(editor, "Price in (USD / M tokens)").value).toBe("3");
    expect(input(editor, "Price out (USD / M tokens)").value).toBe("15");
    expect(input(editor, "Games at once").value).toBe("4");
    expect(input(editor, "Cap per day (USD)").value).toBe("10");
    expect(input(editor, "Cap per month (USD)").value).toBe("100");
    expect(input(editor, "Tokens per day").value).toBe("5000000");
    expect(input(editor, "Tokens per month").value).toBe("90000000");
    expect(select(editor, "Fallback profile").value).toBe("backup");
    expect(input(editor, "Canary").checked).toBe(true);

    // The JSON box holds the rest of the profile and none of the budget.
    const json = (within(editor).getByLabelText("Profile") as HTMLTextAreaElement).value;
    expect(JSON.parse(json)).toStrictEqual({ provider: "anthropic", model: "model-primary", key_env: "ANTHROPIC_API_KEY" });
    for (const key of ["game_usd", "game_tokens", "game_calls", "price"]) expect(json).not.toContain(key);

    // Saved untouched, it is written as it was stored.
    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put, ...rest] = changes(calls);
    expect(rest).toEqual([]);
    expect(put?.body).toEqual(stored);
  });

  test("a fallback the seat agent does not hold is kept and named, not dropped", async () => {
    const { calls } = site(fleet(withDefinition(profile("primary", "Model A"), { fallback: "remote" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    const fallback = select(editor, "Fallback profile");
    // remote is beta's: offered after the real choices, with the reason, and chosen.
    expect(optionsOf(fallback)).toEqual(["none", "Model B · backup", "Model C · spare", "remote (not on this seat agent)"]);
    expect(fallback.value).toBe("remote");

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put] = changes(calls);
    expect(put?.body).toHaveProperty("fallback", "remote");
  });

  test("a fallback of another vendor on this seat agent is kept too, for the server to refuse", async () => {
    site(fleet(withDefinition(profile("primary", "Model A"), { fallback: "other" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit primary @ alpha", "Edit primary on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(valuesOf(fallback)).toEqual(["", "backup", "spare", "other"]);
    expect(fallback.value).toBe("other");
  });
});

/** The row of `host` under the profile card headed `label`. */
function row(label: string, host: string): HTMLElement {
  const card = screen.getAllByTestId("llm-profile").find((c) => within(c).queryByRole("heading", { name: label }) !== null);
  if (card === undefined) throw new Error(`no card for ${label}`);
  const found = within(card)
    .getAllByTestId("llm-on")
    .find((r) => within(r).queryByText(host) !== null);
  if (found === undefined) throw new Error(`no row for ${label} on ${host}`);
  return found;
}

function diverting(): SeatHost[] {
  const primary = withDefinition(profile("primary", "Model A"), { fallback: "backup" });
  const spare = withDefinition(profile("spare", "Model C"), { fallback: "backup" });
  return [seatHost("alpha", [{ ...primary, diverts: "backup" }, spare, profile("backup", "Model B")])];
}

describe("a host row", () => {
  test("names the fallback, and shows where new games go while it diverts", async () => {
    site(diverting());
    render(<Admin lang="en" section="models" />);
    await screen.findAllByTestId("llm-profile");

    const primary = row("Model A", "alpha");
    expect(within(primary).getByText("Fallback: backup")).toBeTruthy();
    expect(within(primary).getByText("new games on backup")).toBeTruthy();

    // A fallback that is not in use names itself and no diversion.
    const spare = row("Model C", "alpha");
    expect(within(spare).getByText("Fallback: backup")).toBeTruthy();
    expect(within(spare).queryByText(/new games on/)).toBeNull();

    // A profile without one says nothing of it.
    const backup = row("Model B", "alpha");
    expect(within(backup).queryByText(/Fallback/)).toBeNull();
    expect(within(backup).queryByText(/new games on/)).toBeNull();
  });

  test("says so in German", async () => {
    site(diverting());
    render(<Admin lang="de" section="models" />);
    await screen.findAllByTestId("llm-profile");
    const primary = row("Model A", "alpha");
    expect(within(primary).getByText("Ausweichprofil: backup")).toBeTruthy();
    expect(within(primary).getByText("neue Partien auf backup")).toBeTruthy();
  });
});

describe("the editor in German", () => {
  test("its sections, the price sentence, the question and a refusal", async () => {
    const { calls, server } = site();
    server.put = { status: 400, body: { error: "ein Profil kann nicht sein eigenes Ausweichprofil sein" } };
    const user = userEvent.setup();
    render(<Admin lang="de" section="models" />);
    const editor = await openEdit(user, "Bearbeiten primary @ alpha", "primary auf alpha bearbeiten");
    for (const name of ["Profil", "Partien und Ausweichprofil", "Budget pro Partie", "Ausgabengrenzen", "Modell"]) {
      expect(within(editor).getByRole("group", { name })).toBeTruthy();
    }
    const fallback = select(editor, "Ausweichprofil");
    expect(optionsOf(fallback)).toEqual(["keines", "Model B · backup", "Model C · spare"]);

    await user.type(input(editor, "Preis Input (USD / Mio. Tokens)"), "3");
    await user.click(within(editor).getByRole("button", { name: "Speichern" }));
    expect(within(editor).getByRole("alert").textContent).toBe("Ein Preis braucht beides: Input und Output.");
    expect(changes(calls)).toEqual([]);

    await user.type(input(editor, "Preis Output (USD / Mio. Tokens)"), "15");
    await user.selectOptions(fallback, "backup");
    await user.click(within(editor).getByRole("button", { name: "Speichern" }));
    const question = screen.getByRole("alertdialog", { name: "primary @ alpha speichern? Laufende Partien behalten, womit sie begonnen haben." });
    await user.click(within(question).getByRole("button", { name: "Speichern" }));
    await within(editor).findByText("Nicht gespeichert: ein Profil kann nicht sein eigenes Ausweichprofil sein");
    expect(screen.getByRole("dialog", { name: "primary auf alpha bearbeiten" })).toBe(editor);
    expect(changes(calls)).toHaveLength(1);
  });
});
