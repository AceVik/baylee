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
import { BUDGET_KEYS, fallbackChoices, otherSettings, readDefinition, type DefinitionForm } from "./Models";

// Typing through user-event in jsdom is slow, and a loaded machine slower.
vi.setConfig({ testTimeout: 20_000 });

function profile(id: string, label: string, over: Partial<LlmProfile> = {}): LlmProfile {
  const vendor = over.vendor ?? "Anthropic";
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
  profile("cc", "Claude CLI", {
    kind: "cli",
    key: "none_needed",
    definition: { label: "Claude CLI", vendor: "Anthropic", profile: { provider: "cli", model: "claude:opus" } },
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
 * Two seat agents. `alpha` holds the profile edited ("sonnet"), two more of its
 * vendor whose labels are out of insertion order, one of another vendor, and an
 * agent CLI; `beta` holds one more API profile. Each exclusion of the fallback
 * select therefore has a concrete candidate it rules out.
 */
function fleet(sonnet: LlmProfile = profile("sonnet", "Sonnet"), cc: LlmProfile = cliProfile()): SeatHost[] {
  return [
    seatHost("alpha", [sonnet, profile("opus", "Opus"), profile("haiku", "Haiku"), profile("gpt", "Gpt", { vendor: "OpenAI" }), cc]),
    seatHost("beta", [profile("gamma", "Gamma")]),
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

const PUT_SONNET = "/ui/api/admin/llm/seathosts/alpha/profiles/sonnet";

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

  test("a fallback: empty or blank is absent, a name is written trimmed", () => {
    expect(read({ ...FORM, fallback: "" })).not.toHaveProperty("fallback");
    expect(read({ ...FORM, fallback: "  " })).not.toHaveProperty("fallback");
    expect(read(FORM)).not.toHaveProperty("fallback");
    expect(read({ ...FORM, fallback: "x" })).toStrictEqual({ label: "X", vendor: "Y", enabled: true, profile: {}, fallback: "x" });
    expect(read({ ...FORM, fallback: " x " }).fallback).toBe("x");
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
    expect(fallbackChoices(hosts, "alpha", "sonnet", "Anthropic").map((p) => p.id)).toEqual(["haiku", "opus"]);
    // Another seat agent's profile is not on this one.
    expect(fallbackChoices(hosts, "beta", "sonnet", "Anthropic").map((p) => p.id)).toEqual(["gamma"]);
    expect(fallbackChoices(hosts, "nowhere", "sonnet", "Anthropic")).toEqual([]);
    // Never itself: a profile on its own id is left out, whichever one it is.
    expect(fallbackChoices(hosts, "alpha", "haiku", "Anthropic").map((p) => p.id)).toEqual(["opus", "sonnet"]);
    // A new profile has no id yet: every API profile of the vendor is a candidate.
    expect(fallbackChoices(hosts, "alpha", "", "Anthropic").map((p) => p.id)).toEqual(["haiku", "opus", "sonnet"]);
  });

  test("an agent CLI is never offered, though it is of the vendor", () => {
    expect(fallbackChoices(fleet(), "alpha", "sonnet", "Anthropic").map((p) => p.id)).not.toContain("cc");
  });

  test("a profile of another vendor on the same seat agent is not offered", () => {
    const hosts = fleet();
    expect(fallbackChoices(hosts, "alpha", "sonnet", "Anthropic").map((p) => p.id)).not.toContain("gpt");
    expect(fallbackChoices(hosts, "alpha", "sonnet", "OpenAI").map((p) => p.id)).toEqual(["gpt"]);
    expect(fallbackChoices(hosts, "alpha", "sonnet", "")).toEqual([]);
  });

  test("the vendor is compared without case and outer blanks", () => {
    const hosts = [seatHost("alpha", [profile("a", "A", { vendor: " ANTHROPIC " }), profile("b", "B", { vendor: "anthropic" }), profile("c", "C", { vendor: "Anthropic Inc" })])];
    expect(fallbackChoices(hosts, "alpha", "x", "  Anthropic").map((p) => p.id)).toEqual(["a", "b"]);
  });

  test("equal labels are told apart by id", () => {
    const hosts = [seatHost("alpha", [profile("b", "Same"), profile("a", "Same"), profile("c", "Before")])];
    expect(fallbackChoices(hosts, "alpha", "x", "Anthropic").map((p) => p.id)).toEqual(["c", "a", "b"]);
  });
});

describe("the profile editor", () => {
  test("it is in sections, and every field but a cap has a hint it is described by", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
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
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(fallback.disabled).toBe(false);
    // Haiku before Opus: by label, not as the seat agent listed them.
    expect(optionsOf(fallback)).toEqual(["none", "Haiku · haiku", "Opus · opus"]);
    expect(valuesOf(fallback)).toEqual(["", "haiku", "opus"]);
    expect(fallback.value).toBe("");
    // Not Gamma (beta's), not Gpt (OpenAI's), not the CLI, not Sonnet itself.
    const text = optionsOf(fallback).join("|");
    for (const left of ["Sonnet", "Claude CLI", "Gamma", "Gpt"]) expect(text).not.toContain(left);
    expect(describedBy(fallback)).toContain("same vendor");
    expect(describedBy(fallback)).toBe(t("en", "llm.hint.fallback"));
  });

  test("the choices follow the vendor box", async () => {
    site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    const vendor = input(editor, "Vendor (game data goes to)");
    await user.clear(vendor);
    await user.type(vendor, "openai ");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Gpt · gpt"]);
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
    await user.type(input(editor, "Vendor (game data goes to)"), "Anthropic");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Haiku · haiku", "Opus · opus", "Sonnet · sonnet"]);
    // The id typed is the profile itself.
    await user.type(input(editor, "Id"), "haiku");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Opus · opus", "Sonnet · sonnet"]);
    // Another seat agent holds other profiles.
    await user.selectOptions(select(editor, "Seat agent"), "beta");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none", "Gamma · gamma"]);
    await user.clear(input(editor, "Id"));
    await user.type(input(editor, "Id"), "gamma");
    expect(optionsOf(select(editor, "Fallback profile"))).toEqual(["none"]);
  });

  test("a save asks first; no leaves the form as it was, yes writes the fallback and the budget", async () => {
    const { calls } = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    await user.selectOptions(select(editor, "Fallback profile"), "haiku");
    await user.type(input(editor, "Per game (USD)"), "2");
    await user.type(input(editor, "Tokens per game"), "100000");
    await user.type(input(editor, "Calls per game"), "50");
    await user.type(input(editor, "Price in (USD / M tokens)"), "3");
    await user.type(input(editor, "Price out (USD / M tokens)"), "15");
    await user.type(input(editor, "Tokens per day"), "1000000");

    // The question names what is saved and for whom; nothing is sent yet.
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    const question = screen.getByRole("alertdialog", { name: "Save sonnet @ alpha? Games already running keep what they started with." });
    expect(changes(calls)).toEqual([]);

    // No: the question closes, the editor and what was typed stay, nothing is sent.
    await user.click(within(question).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Edit sonnet on alpha" })).toBe(editor);
    expect(changes(calls)).toEqual([]);
    expect(select(editor, "Fallback profile").value).toBe("haiku");
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
          label: "Sonnet",
          vendor: "Anthropic",
          enabled: true,
          max_games: 4,
          caps: { day_usd: 10, month_usd: 100, day_tokens: 1_000_000 },
          fallback: "haiku",
          profile: {
            provider: "anthropic",
            model: "model-sonnet",
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
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    await user.selectOptions(select(editor, "Fallback profile"), "haiku");
    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));

    const refusal = await within(editor).findByText("Not saved: a profile cannot be its own fallback");
    expect(refusal.getAttribute("role")).toBe("alert");
    expect(within(editor).getByRole("alert")).toBe(refusal);
    // The question is gone, the editor is still the one open, with what was chosen.
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Edit sonnet on alpha" })).toBe(editor);
    expect(select(editor, "Fallback profile").value).toBe("haiku");
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

  test("an agent CLI's editor has the fallback select disabled, and a stale fallback is saved as none", async () => {
    const { calls } = site(fleet(undefined, withDefinition(cliProfile(), { fallback: "haiku" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit cc @ alpha", "Edit cc on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(fallback.disabled).toBe(true);
    expect(fallback.value).toBe("");
    expect(describedBy(fallback)).toBe("An agent CLI takes no fallback.");
    expect(optionsOf(fallback).join("|")).not.toContain("not on this seat agent");

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put, ...rest] = changes(calls);
    expect(rest).toEqual([]);
    expect(put?.url).toBe("/ui/api/admin/llm/seathosts/alpha/profiles/cc");
    expect(put?.body).toEqual({ label: "Claude CLI", vendor: "Anthropic", enabled: true, profile: { provider: "cli", model: "claude:opus" } });
    expect(put?.body).not.toHaveProperty("fallback");
  });

  test("turning a profile's JSON into a CLI disables the fallback, and it is saved as none", async () => {
    const { calls } = site(fleet(withDefinition(profile("sonnet", "Sonnet"), { fallback: "haiku" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    expect(select(editor, "Fallback profile").disabled).toBe(false);
    expect(select(editor, "Fallback profile").value).toBe("haiku");

    fireEvent.change(within(editor).getByLabelText("Profile"), { target: { value: '{"provider":"cli","model":"claude:opus"}' } });
    const fallback = select(editor, "Fallback profile");
    expect(fallback.disabled).toBe(true);
    expect(fallback.value).toBe("");
    expect(describedBy(fallback)).toBe("An agent CLI takes no fallback.");

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put] = changes(calls);
    expect(put?.url).toBe(PUT_SONNET);
    expect(put?.body).toEqual({
      label: "Sonnet",
      vendor: "Anthropic",
      enabled: true,
      max_games: 4,
      caps: { day_usd: 10, month_usd: 100 },
      profile: { provider: "cli", model: "claude:opus" },
    });
    expect(put?.body).not.toHaveProperty("fallback");
  });

  test("a price needs both boxes: said in the editor, nothing asked, nothing sent", async () => {
    const { calls } = site();
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
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
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    fireEvent.change(within(editor).getByLabelText("Profile"), { target: { value: "[1]" } });
    await user.click(within(editor).getByRole("button", { name: "Save" }));
    expect(within(editor).getByRole("alert").textContent).toBe("The profile is not a JSON object.");
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(changes(calls)).toEqual([]);
  });

  test("a profile with a budget shows it in its boxes, not in the JSON, and saves back what it was", async () => {
    const stored: LlmDefinition = {
      label: "Sonnet",
      vendor: "Anthropic",
      enabled: true,
      max_games: 4,
      canary: true,
      fallback: "haiku",
      caps: { day_usd: 10, month_usd: 100, day_tokens: 5_000_000, month_tokens: 90_000_000 },
      profile: {
        provider: "anthropic",
        model: "model-sonnet",
        key_env: "ANTHROPIC_API_KEY",
        game_usd: 3,
        game_tokens: 2_000_000,
        game_calls: 200,
        price: { input: 3, output: 15 },
      },
    };
    const { calls } = site(fleet({ ...profile("sonnet", "Sonnet"), definition: stored }));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");

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
    expect(select(editor, "Fallback profile").value).toBe("haiku");
    expect(input(editor, "Canary").checked).toBe(true);

    // The JSON box holds the rest of the profile and none of the budget.
    const json = (within(editor).getByLabelText("Profile") as HTMLTextAreaElement).value;
    expect(JSON.parse(json)).toStrictEqual({ provider: "anthropic", model: "model-sonnet", key_env: "ANTHROPIC_API_KEY" });
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
    const { calls } = site(fleet(withDefinition(profile("sonnet", "Sonnet"), { fallback: "gamma" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    const fallback = select(editor, "Fallback profile");
    // gamma is beta's: offered after the real choices, with the reason, and chosen.
    expect(optionsOf(fallback)).toEqual(["none", "Haiku · haiku", "Opus · opus", "gamma (not on this seat agent)"]);
    expect(fallback.value).toBe("gamma");

    await ask(user, editor);
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Save" }));
    await screen.findByText("Profile saved.");
    const [put] = changes(calls);
    expect(put?.body).toHaveProperty("fallback", "gamma");
  });

  test("a fallback of another vendor on this seat agent is kept too, for the server to refuse", async () => {
    site(fleet(withDefinition(profile("sonnet", "Sonnet"), { fallback: "gpt" })));
    const user = userEvent.setup();
    render(<Admin lang="en" section="models" />);
    const editor = await openEdit(user, "Edit sonnet @ alpha", "Edit sonnet on alpha");
    const fallback = select(editor, "Fallback profile");
    expect(valuesOf(fallback)).toEqual(["", "haiku", "opus", "gpt"]);
    expect(fallback.value).toBe("gpt");
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
  const sonnet = withDefinition(profile("sonnet", "Sonnet"), { fallback: "haiku" });
  const opus = withDefinition(profile("opus", "Opus"), { fallback: "haiku" });
  return [seatHost("alpha", [{ ...sonnet, diverts: "haiku" }, opus, profile("haiku", "Haiku")])];
}

describe("a host row", () => {
  test("names the fallback, and shows where new games go while it diverts", async () => {
    site(diverting());
    render(<Admin lang="en" section="models" />);
    await screen.findAllByTestId("llm-profile");

    const sonnet = row("Sonnet", "alpha");
    expect(within(sonnet).getByText("Fallback: haiku")).toBeTruthy();
    expect(within(sonnet).getByText("new games on haiku")).toBeTruthy();

    // A fallback that is not in use names itself and no diversion.
    const opus = row("Opus", "alpha");
    expect(within(opus).getByText("Fallback: haiku")).toBeTruthy();
    expect(within(opus).queryByText(/new games on/)).toBeNull();

    // A profile without one says nothing of it.
    const haiku = row("Haiku", "alpha");
    expect(within(haiku).queryByText(/Fallback/)).toBeNull();
    expect(within(haiku).queryByText(/new games on/)).toBeNull();
  });

  test("says so in German", async () => {
    site(diverting());
    render(<Admin lang="de" section="models" />);
    await screen.findAllByTestId("llm-profile");
    const sonnet = row("Sonnet", "alpha");
    expect(within(sonnet).getByText("Ausweichprofil: haiku")).toBeTruthy();
    expect(within(sonnet).getByText("neue Partien auf haiku")).toBeTruthy();
  });
});

describe("the editor in German", () => {
  test("its sections, the price sentence, the question and a refusal", async () => {
    const { calls, server } = site();
    server.put = { status: 400, body: { error: "ein Profil kann nicht sein eigenes Ausweichprofil sein" } };
    const user = userEvent.setup();
    render(<Admin lang="de" section="models" />);
    const editor = await openEdit(user, "Bearbeiten sonnet @ alpha", "sonnet auf alpha bearbeiten");
    for (const name of ["Profil", "Partien und Ausweichprofil", "Budget je Partie", "Obergrenzen seines Ausgabenbuchs", "Modell"]) {
      expect(within(editor).getByRole("group", { name })).toBeTruthy();
    }
    const fallback = select(editor, "Ausweichprofil");
    expect(optionsOf(fallback)).toEqual(["keins", "Haiku · haiku", "Opus · opus"]);

    await user.type(input(editor, "Preis ein (USD / Mio. Tokens)"), "3");
    await user.click(within(editor).getByRole("button", { name: "Speichern" }));
    expect(within(editor).getByRole("alert").textContent).toBe("Ein Preis braucht beides: ein und aus.");
    expect(changes(calls)).toEqual([]);

    await user.type(input(editor, "Preis aus (USD / Mio. Tokens)"), "15");
    await user.selectOptions(fallback, "haiku");
    await user.click(within(editor).getByRole("button", { name: "Speichern" }));
    const question = screen.getByRole("alertdialog", { name: "sonnet @ alpha speichern? Laufende Partien behalten, womit sie begonnen haben." });
    await user.click(within(question).getByRole("button", { name: "Speichern" }));
    await within(editor).findByText("Nicht gespeichert: ein Profil kann nicht sein eigenes Ausweichprofil sein");
    expect(screen.getByRole("dialog", { name: "sonnet auf alpha bearbeiten" })).toBe(editor);
    expect(changes(calls)).toHaveLength(1);
  });
});
