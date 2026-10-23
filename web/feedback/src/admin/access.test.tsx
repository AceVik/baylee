import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import type { AccountCandidate, LlmAccess, LlmAccessDetail, LlmAllowed } from "../api";
import { t } from "../i18n";
import { serve, type Call } from "../test-utils";
import { accessPlan, AccessPanel, planIsEmpty, shownList, type Person } from "./ModelAccess";

// Typing through user-event in jsdom is slow, and a loaded machine slower.
vi.setConfig({ testTimeout: 20_000 });

function allowed(over: Partial<LlmAllowed> & { id: string }): LlmAllowed {
  return {
    username: null,
    display_name: over.id,
    handle: `${over.id}#0000`,
    added_at: "2026-10-01T10:00:00Z",
    added_by: "viktor",
    ...over,
  };
}

const ADA = allowed({ id: "a1", username: "ada", display_name: "Ada L", handle: "Ada#a1f0", added_at: "2026-10-01T10:00:00Z" });
const BOB = allowed({ id: "b2", username: null, display_name: "Bob", handle: "Bob#b2e1", added_at: "2026-10-03T10:00:00Z" });
const CLEO = allowed({ id: "c3", username: "cleo", display_name: "Cleo", handle: "Cleo#c3d2", added_at: "2026-10-02T10:00:00Z" });
const ADAM = allowed({ id: "d4", username: "adam", display_name: "Adam", handle: "Adam#d4c3", added_at: "2026-10-04T10:00:00Z" });
// The username is in no part of the handle: only a username match finds it.
const ZED = allowed({ id: "z5", username: "zedmaster", display_name: "Zed", handle: "Zed#9f00", added_at: "2026-10-05T10:00:00Z" });

const person = (a: LlmAllowed): Person => ({ id: a.id, handle: a.handle, username: a.username });

function detail(over: Partial<LlmAccessDetail> = {}): LlmAccessDetail {
  return {
    id: "sonnet",
    policy: "registered",
    stored: true,
    default_for_registered: false,
    listed: 0,
    updated_at: "2026-10-05T10:00:00Z",
    updated_by: "viktor",
    connected: true,
    label: "Sonnet",
    vendor: "Anthropic",
    accounts: [],
    ...over,
  };
}

/** The row of the profiles list that goes with a detail. */
function summaryOf(d: LlmAccessDetail): LlmAccess {
  const { accounts, ...rest } = d;
  return { ...rest, listed: accounts.length };
}

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

const handles = (list: LlmAllowed[]) => list.map((a) => a.handle);

describe("shownList", () => {
  const LIST = [ADA, BOB, CLEO, ADAM, ZED];

  test("no query keeps everything", () => {
    expect(shownList(LIST, "", "name")).toHaveLength(5);
    expect(shownList(LIST, "   ", "name")).toHaveLength(5);
  });

  test("filters by handle, in any case", () => {
    expect(handles(shownList(LIST, "b2E1", "name"))).toEqual(["Bob#b2e1"]);
    expect(handles(shownList(LIST, "CLEO#", "name"))).toEqual(["Cleo#c3d2"]);
  });

  test("filters by username, in any case, and by a part", () => {
    expect(handles(shownList(LIST, "MASTER", "name"))).toEqual(["Zed#9f00"]);
    expect(handles(shownList(LIST, "ada", "name"))).toEqual(["Ada#a1f0", "Adam#d4c3"]);
  });

  test("a query nothing matches leaves nothing; an account without a username is found by handle only", () => {
    expect(shownList(LIST, "nobody-here", "name")).toEqual([]);
    expect(handles(shownList([BOB], "bob", "name"))).toEqual(["Bob#b2e1"]);
  });

  test("sorts by name (username, else display name), newest and oldest", () => {
    expect(handles(shownList(LIST, "", "name"))).toEqual(["Ada#a1f0", "Adam#d4c3", "Bob#b2e1", "Cleo#c3d2", "Zed#9f00"]);
    expect(handles(shownList(LIST, "", "newest"))).toEqual(["Zed#9f00", "Adam#d4c3", "Bob#b2e1", "Cleo#c3d2", "Ada#a1f0"]);
    expect(handles(shownList(LIST, "", "oldest"))).toEqual(["Ada#a1f0", "Cleo#c3d2", "Bob#b2e1", "Adam#d4c3", "Zed#9f00"]);
  });

  test("the same moment falls back to the name; the input is not reordered", () => {
    const same = [allowed({ id: "q", username: "quinn", added_at: "2026-10-01T10:00:00Z" }), allowed({ id: "p", username: "pia", added_at: "2026-10-01T10:00:00Z" })];
    const before = [...same];
    expect(shownList(same, "", "newest").map((a) => a.id)).toEqual(["p", "q"]);
    expect(shownList(same, "", "oldest").map((a) => a.id)).toEqual(["p", "q"]);
    expect(same).toEqual(before);
  });
});

describe("accessPlan and planIsEmpty", () => {
  const stored = detail({ policy: "listed", accounts: [ADA, BOB] });

  test("nothing changed is an empty plan", () => {
    const plan = accessPlan(stored, "listed", false, [], []);
    expect(plan).toEqual({ policy: null, makeDefault: null, add: [], remove: [] });
    expect(planIsEmpty(plan)).toBe(true);
  });

  test("a policy that moves is planned, without a default change", () => {
    const plan = accessPlan(stored, "nobody", false, [], []);
    expect(plan.policy).toBe("nobody");
    expect(plan.makeDefault).toBeNull();
    expect(planIsEmpty(plan)).toBe(false);
  });

  test("a profile with no stored policy always plans one, even the one it shows", () => {
    const unset = detail({ policy: "nobody", stored: false });
    const plan = accessPlan(unset, "nobody", false, [], []);
    expect(plan.policy).toBe("nobody");
    expect(planIsEmpty(plan)).toBe(false);
  });

  test("an account already on the list is not added; one not on it is not taken off", () => {
    const plan = accessPlan(stored, "listed", false, [person(ADA), person(CLEO)], [person(BOB), person(ZED)]);
    expect(plan.add).toEqual([person(CLEO)]);
    expect(plan.remove).toEqual([person(BOB)]);
    expect(planIsEmpty(plan)).toBe(false);
    const nothing = accessPlan(stored, "listed", false, [person(ADA)], [person(ZED)]);
    expect(planIsEmpty(nothing)).toBe(true);
  });

  test("the default flag is planned only when it moves, and then with the policy", () => {
    const plan = accessPlan(stored, "registered", true, [], []);
    expect(plan.makeDefault).toBe(true);
    expect(plan.policy).toBe("registered");
    const was = detail({ policy: "registered", default_for_registered: true });
    expect(accessPlan(was, "registered", true, [], []).makeDefault).toBeNull();
    const off = accessPlan(was, "registered", false, [], []);
    expect(off.makeDefault).toBe(false);
    expect(off.policy).toBe("registered");
  });
});

const CANDIDATES: AccountCandidate[] = [
  { id: "a1", username: "ada", display_name: "Ada L", handle: "Ada#a1f0" },
  { id: "f6", username: "adele", display_name: "Adele", handle: "Adele#f600" },
  { id: "e5", username: null, display_name: "Adrian", handle: "Adrian#e500" },
];

/** A gateway holding one profile's detail; lookups answer by prefix. */
function site(d: LlmAccessDetail): Call[] {
  return serve((call) => {
    if (call.url === `/ui/api/admin/llm/access/${d.id}` && call.method === "GET") return { body: d };
    if (call.url === `/ui/api/admin/llm/access/${d.id}` && call.method === "PUT") return { status: 204 };
    if (call.url.startsWith(`/ui/api/admin/llm/access/${d.id}/`) && call.method === "POST") return { body: d };
    if (call.url.startsWith("/ui/api/admin/accounts/lookup?q=")) {
      const q = decodeURIComponent(call.url.slice("/ui/api/admin/accounts/lookup?q=".length)).toLowerCase();
      return {
        body: {
          accounts: CANDIDATES.filter((c) => (c.username ?? c.display_name).toLowerCase().startsWith(q)),
        },
      };
    }
    return undefined;
  });
}

const changes = (calls: Call[]) => calls.filter((c) => c.method !== "GET");
const lookups = (calls: Call[]) => calls.filter((c) => c.url.startsWith("/ui/api/admin/accounts/lookup"));

/** Renders the panel for one profile and opens its editor. */
async function openEditor(d: LlmAccessDetail, lang: "en" | "de" = "en") {
  const calls = site(d);
  const onChanged = vi.fn<(text: string) => void>();
  const user = userEvent.setup();
  render(<AccessPanel lang={lang} access={[summaryOf(d)]} onChanged={onChanged} />);
  await user.click(screen.getByRole("button", { name: `${t(lang, "access.manage")} ${d.label ?? d.id}` }));
  await screen.findByRole("dialog");
  // The list (or its empty sentence) is there once the detail has loaded.
  await screen.findByRole("combobox", { name: t(lang, "access.find") });
  return { calls, onChanged, user };
}

const rows = () => screen.getAllByTestId("access-row");
const rowOf = (handle: string) => {
  const found = rows().find((r) => r.textContent?.includes(handle));
  if (found === undefined) throw new Error(`no row for ${handle}`);
  return found;
};
/** The picker's options (the order select has options of its own). */
const found = async () => within(await screen.findByRole("listbox")).getAllByRole("option");
const picker = () => screen.getByRole("combobox", { name: "Add a player" });

describe("the panel", () => {
  test("each profile shows its policy, its list size, default and whether it is reported", () => {
    const profiles: LlmAccess[] = [
      summaryOf(detail({ id: "a", label: "Open", policy: "registered", default_for_registered: true })),
      summaryOf(detail({ id: "b", label: "Few", policy: "listed", accounts: [ADA, BOB], connected: false })),
      summaryOf(detail({ id: "c", label: "Fresh", policy: "nobody", stored: false })),
    ];
    render(<AccessPanel lang="en" access={profiles} onChanged={() => undefined} />);
    const [a, b, c] = screen.getAllByTestId("access-profile") as [HTMLElement, HTMLElement, HTMLElement];
    expect(a.textContent).toContain("Every registered player");
    expect(a.textContent).toContain("default for registered players");
    expect(a.textContent).not.toContain("no seat agent reports it now");
    expect(b.textContent).toContain("Only the players on the list");
    expect(b.textContent).toContain("2 on the list");
    expect(b.textContent).toContain("no seat agent reports it now");
    expect(c.textContent).toContain("not set: nobody");
    expect(c.textContent).not.toContain("on the list");
  });

  test("no profiles, and still loading, are said", () => {
    const { rerender } = render(<AccessPanel lang="en" access={[]} onChanged={() => undefined} />);
    expect(screen.getByText("No profile is reported or has a policy yet.")).toBeTruthy();
    rerender(<AccessPanel lang="en" access={null} onChanged={() => undefined} />);
    expect(screen.getByText("Loading…", { exact: false })).toBeTruthy();
  });

  test("in German", async () => {
    const d = detail({ policy: "nobody", stored: false });
    render(<AccessPanel lang="de" access={[summaryOf(d)]} onChanged={() => undefined} />);
    expect(screen.getByText("Wer welches Modell einsetzen darf")).toBeTruthy();
    expect(screen.getByText("nicht gesetzt: niemand")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Zugang verwalten Sonnet" })).toBeTruthy();
  });

  test("the editor in German", async () => {
    site(detail({ accounts: [ADA] }));
    const user = userEvent.setup();
    render(<AccessPanel lang="de" access={[summaryOf(detail({ accounts: [ADA] }))]} onChanged={() => undefined} />);
    await user.click(screen.getByRole("button", { name: "Zugang verwalten Sonnet" }));
    expect(await screen.findByRole("heading", { name: "Wer Sonnet einsetzen darf" })).toBeTruthy();
    expect(await screen.findByRole("combobox", { name: "Spieler hinzufügen" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Speichern…" })).toBeTruthy();
    expect(screen.getByRole("radio", { name: /Nur die Spieler auf der Liste/ })).toBeTruthy();
  });
});

describe("the account picker", () => {
  test("one character asks nothing, and says how many are needed", async () => {
    const { calls, user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    await user.type(picker(), "a");
    await new Promise((r) => setTimeout(r, 400));
    expect(lookups(calls)).toEqual([]);
    expect(screen.getByText("Type at least 2 characters of a username or display name.")).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  test("two characters ask exactly once, after the debounce, not on every keystroke", async () => {
    const { calls } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    const box = picker();
    // Five keystrokes in one tick: none can wait out the debounce between them.
    for (const text of ["a", "ad", "ade", "adel", "adele"]) fireEvent.change(box, { target: { value: text } });
    expect(lookups(calls)).toEqual([]);
    await waitFor(() => {
      expect(lookups(calls)).toHaveLength(1);
    });
    await found();
    await new Promise((r) => setTimeout(r, 400));
    expect(lookups(calls)).toHaveLength(1);
    expect(lookups(calls)[0]?.url).toBe("/ui/api/admin/accounts/lookup?q=adele");
    expect(lookups(calls)[0]?.method).toBe("GET");
  });

  test("each keystroke restarts the wait", async () => {
    const { calls } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    const box = picker();
    fireEvent.change(box, { target: { value: "ad" } });
    await pause(100);
    expect(lookups(calls)).toEqual([]);
    fireEvent.change(box, { target: { value: "ade" } });
    await pause(100);
    expect(lookups(calls)).toEqual([]);
    await waitFor(() => {
      expect(lookups(calls)).toHaveLength(1);
    });
    expect(lookups(calls)[0]?.url).toBe("/ui/api/admin/accounts/lookup?q=ade");
  });

  test("the answer to an older text is not shown for the new one", async () => {
    const { calls } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    const box = picker();
    fireEvent.change(box, { target: { value: "ad" } });
    await found();
    fireEvent.change(box, { target: { value: "adr" } });
    // Until the new answer arrives the old options are gone.
    expect(screen.queryByRole("listbox")).toBeNull();
    const options = await found();
    expect(options).toHaveLength(1);
    expect(options[0]?.textContent).toContain("Adrian#e500");
    expect(lookups(calls).map((c) => c.url)).toEqual(["/ui/api/admin/accounts/lookup?q=ad", "/ui/api/admin/accounts/lookup?q=adr"]);
  });

  test("ArrowDown moves the active option and Enter adds it as a chip", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    await user.type(picker(), "ad");
    const options = await found();
    expect(options).toHaveLength(3);
    expect(options[0]?.getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{ArrowDown}");
    expect(options[1]?.getAttribute("aria-selected")).toBe("true");
    expect(picker().getAttribute("aria-activedescendant")).toBe(options[1]?.id);
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(options[0]?.getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{ArrowUp}");
    expect(options[2]?.getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{ArrowUp}");
    expect(options[1]?.getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect((picker() as HTMLInputElement).value).toBe("");
    expect(screen.getByText("To add (1):")).toBeTruthy();
    const chip = screen.getByRole("button", { name: "Do not add Adele#f600" }).closest("li");
    expect(chip?.textContent).toContain("Adele#f600");
    expect(chip?.textContent).toContain("@adele");
  });

  test("an account already on the list is marked and cannot be added", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    await user.type(picker(), "ad");
    const options = await found();
    expect(options[0]?.textContent).toContain("Ada#a1f0");
    expect(options[0]?.textContent).toContain("already on the list");
    expect(options[0]?.getAttribute("aria-disabled")).toBe("true");
    expect(options[1]?.getAttribute("aria-disabled")).toBe("false");
    // Enter on the active, taken option adds nothing.
    await user.keyboard("{Enter}");
    expect(screen.queryByText(/To add/)).toBeNull();
    // Nor does a click.
    fireEvent.mouseDown(options[0] as HTMLElement);
    expect(screen.queryByText(/To add/)).toBeNull();
    // Once picked, an account is taken as well.
    fireEvent.mouseDown(options[1] as HTMLElement);
    expect(screen.getByText("To add (1):")).toBeTruthy();
    await user.type(picker(), "ad");
    const again = await found();
    expect(again[1]?.textContent).toContain("already on the list");
    fireEvent.mouseDown(again[1] as HTMLElement);
    expect(screen.getByText("To add (1):")).toBeTruthy();
  });

  test("Escape closes the options without closing the dialog", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    await user.type(picker(), "ad");
    await found();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect((picker() as HTMLInputElement).value).toBe("ad");
  });

  test("a chip's × removes it again", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    await user.type(picker(), "ad");
    await found();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(screen.getByText("To add (1):")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Do not add Adele#f600" }));
    expect(screen.queryByText(/To add/)).toBeNull();
    expect(screen.queryByRole("button", { name: "Do not add Adele#f600" })).toBeNull();
    expect((screen.getByRole("button", { name: "Save…" }) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("the editor's save flow", () => {
  test("Save… is disabled until something changes", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA, BOB] }));
    const save = screen.getByRole("button", { name: "Save…" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await user.click(screen.getByRole("radio", { name: /Nobody/ }));
    expect(save.disabled).toBe(false);
    await user.click(screen.getByRole("radio", { name: /Only the players on the list/ }));
    expect(save.disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Take off Ada#a1f0" }));
    expect(save.disabled).toBe(false);
    await user.click(within(rowOf("Ada#a1f0")).getByRole("button", { name: "Keep Ada#a1f0" }));
    expect(save.disabled).toBe(true);
  });

  test("policy, one add and one take off are reviewed, then sent in order with the CSRF header", async () => {
    const { calls, onChanged, user } = await openEditor(detail({ policy: "registered", accounts: [ADA, BOB, CLEO] }));
    await user.click(screen.getByRole("radio", { name: /Only the players on the list/ }));
    await user.type(picker(), "ade");
    await found();
    await user.keyboard("{Enter}");
    expect(screen.getByText("To add (1):")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Take off Bob#b2e1" }));
    expect(screen.getByText("To take off (1):")).toBeTruthy();
    expect(rowOf("Bob#b2e1").className).toContain("going");

    await user.click(screen.getByRole("button", { name: "Save…" }));
    const review = screen.getByRole("region", { name: "Apply these changes?" });
    const items = within(review).getAllByRole("listitem");
    expect(items.map((li) => li.textContent)).toEqual([
      "Policy: Only the players on the list.",
      "Add 1 player: Adele#f600",
      "Take 1 player off: Bob#b2e1",
    ]);
    // Nothing has been sent yet.
    expect(changes(calls)).toEqual([]);

    await user.click(within(review).getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(onChanged).toHaveBeenCalledWith("Access to Sonnet saved.");
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(changes(calls)).toEqual([
      expect.objectContaining({ method: "PUT", url: "/ui/api/admin/llm/access/sonnet", body: { policy: "listed" } }),
      expect.objectContaining({ method: "POST", url: "/ui/api/admin/llm/access/sonnet/allow", body: { accounts: ["f6"] } }),
      expect.objectContaining({ method: "POST", url: "/ui/api/admin/llm/access/sonnet/disallow", body: { accounts: ["b2"] } }),
    ]);
    for (const call of changes(calls)) {
      expect(call.headers["x-baylee-csrf"]).toBe("1");
      expect(call.headers["content-type"]).toBe("application/json");
    }
  });
});

describe("more ways to save", () => {
  test("Back from the review returns to the form with the changes kept", async () => {
    const { calls, user } = await openEditor(detail({ policy: "listed", accounts: [ADA, BOB] }));
    await user.click(screen.getByRole("button", { name: "Take off Bob#b2e1" }));
    await user.click(screen.getByRole("button", { name: "Save…" }));
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.queryByRole("region", { name: "Apply these changes?" })).toBeNull();
    expect(screen.getByText("To take off (1):")).toBeTruthy();
    expect(rowOf("Bob#b2e1").className).toContain("going");
    expect(changes(calls)).toEqual([]);
  });

  test("a profile with no stored policy saves its policy even when nothing was touched", async () => {
    const { calls, onChanged, user } = await openEditor(detail({ policy: "nobody", stored: false }));
    const save = screen.getByRole("button", { name: "Save…" }) as HTMLButtonElement;
    expect(save.disabled).toBe(false);
    await user.click(save);
    await user.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(onChanged).toHaveBeenCalled();
    });
    expect(changes(calls)).toEqual([expect.objectContaining({ method: "PUT", body: { policy: "nobody" } })]);
  });

  test("a policy and the default flag go in one PUT", async () => {
    const { calls, user } = await openEditor(detail({ policy: "registered", accounts: [] }));
    await user.click(screen.getByRole("checkbox", { name: /The default model for registered players/ }));
    await user.click(screen.getByRole("button", { name: "Save…" }));
    expect(screen.getByText(/It becomes the default for registered players\./)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({ method: "PUT", url: "/ui/api/admin/llm/access/sonnet", body: { policy: "registered", default_for_registered: true } }),
    );
  });

  test("dropping the default is reviewed and sent as false", async () => {
    const { calls, user } = await openEditor(detail({ policy: "registered", default_for_registered: true }));
    await user.click(screen.getByRole("checkbox", { name: /The default model for registered players/ }));
    await user.click(screen.getByRole("button", { name: "Save…" }));
    expect(screen.getByText(/It is no longer the default\./)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]?.body).toEqual({ policy: "registered", default_for_registered: false });
  });
});

const box = () => screen.getByRole("checkbox", { name: /The default model for registered players/ }) as HTMLInputElement;
const radio = (name: RegExp) => screen.getByRole("radio", { name }) as HTMLInputElement;
const shownHandles = () => rows().map((r) => r.querySelector("strong")?.textContent);

describe("the default checkbox", () => {
  test("checking it selects Every registered player", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA] }));
    expect(radio(/Only the players on the list/).checked).toBe(true);
    await user.click(box());
    expect(box().checked).toBe(true);
    expect(radio(/Every registered player/).checked).toBe(true);
    expect(radio(/Only the players on the list/).checked).toBe(false);
  });

  test("choosing another policy unchecks it", async () => {
    const { user } = await openEditor(detail({ policy: "registered", default_for_registered: true }));
    expect(box().checked).toBe(true);
    await user.click(radio(/Nobody/));
    expect(radio(/Nobody/).checked).toBe(true);
    expect(box().checked).toBe(false);
    await user.click(box());
    expect(radio(/Every registered player/).checked).toBe(true);
    await user.click(radio(/Only the players on the list/));
    expect(box().checked).toBe(false);
  });

  test("choosing Every registered player keeps it checked", async () => {
    const { user } = await openEditor(detail({ policy: "registered", default_for_registered: true }));
    await user.click(radio(/Every registered player/));
    expect(box().checked).toBe(true);
  });
});

describe("the list: filter, sort and bulk", () => {
  const LIST = detail({ policy: "listed", accounts: [ADA, BOB, CLEO, ADAM] });

  test("Select all shown with a filter selects only the filtered rows", async () => {
    const { user } = await openEditor(LIST);
    expect(shownHandles()).toEqual(["Ada#a1f0", "Adam#d4c3", "Bob#b2e1", "Cleo#c3d2"]);
    await user.type(screen.getByRole("searchbox", { name: "Filter the list" }), "ada");
    expect(shownHandles()).toEqual(["Ada#a1f0", "Adam#d4c3"]);
    await user.click(screen.getByRole("checkbox", { name: "Select all shown" }));
    expect((screen.getByRole("checkbox", { name: "Select Ada#a1f0" }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("checkbox", { name: "Select Adam#d4c3" }) as HTMLInputElement).checked).toBe(true);
    const take = screen.getByRole("button", { name: "Take off selected (2)" }) as HTMLButtonElement;
    expect(take.disabled).toBe(false);

    // The rows hidden by the filter were never selected.
    await user.clear(screen.getByRole("searchbox", { name: "Filter the list" }));
    expect((screen.getByRole("checkbox", { name: "Select Bob#b2e1" }) as HTMLInputElement).checked).toBe(false);
    expect((screen.getByRole("checkbox", { name: "Select Cleo#c3d2" }) as HTMLInputElement).checked).toBe(false);
    expect((screen.getByRole("checkbox", { name: "Select all shown" }) as HTMLInputElement).checked).toBe(false);
  });

  test("Take off selected (n) marks them, and nothing else", async () => {
    const { calls, user } = await openEditor(LIST);
    expect((screen.getByRole("button", { name: "Take off selected (0)" }) as HTMLButtonElement).disabled).toBe(true);
    await user.type(screen.getByRole("searchbox", { name: "Filter the list" }), "ada");
    await user.click(screen.getByRole("checkbox", { name: "Select all shown" }));
    await user.click(screen.getByRole("button", { name: "Take off selected (2)" }));
    expect(screen.getByText("To take off (2):")).toBeTruthy();
    expect(rows().map((r) => r.className.includes("going"))).toEqual([true, true]);
    expect((screen.getByRole("button", { name: "Take off selected (0)" }) as HTMLButtonElement).disabled).toBe(true);
    await user.clear(screen.getByRole("searchbox", { name: "Filter the list" }));
    expect(rows().map((r) => r.className.includes("going"))).toEqual([true, true, false, false]);
    await user.click(screen.getByRole("button", { name: "Save…" }));
    const review = screen.getByRole("region", { name: "Apply these changes?" });
    expect(within(review).getByText("Take 2 players off:").parentElement?.textContent).toContain("Ada#a1f0, Adam#d4c3");
    await user.click(within(review).getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(changes(calls)).toHaveLength(1);
    });
    expect(changes(calls)[0]).toEqual(
      expect.objectContaining({ url: "/ui/api/admin/llm/access/sonnet/disallow", body: { accounts: ["a1", "d4"] } }),
    );
  });

  test("Select all shown toggles, and a row is selected one by one", async () => {
    const { user } = await openEditor(LIST);
    const all = screen.getByRole("checkbox", { name: "Select all shown" }) as HTMLInputElement;
    await user.click(all);
    expect(screen.getByRole("button", { name: "Take off selected (4)" })).toBeTruthy();
    await user.click(all);
    expect(screen.getByRole("button", { name: "Take off selected (0)" })).toBeTruthy();
    await user.click(screen.getByRole("checkbox", { name: "Select Bob#b2e1" }));
    expect(screen.getByRole("button", { name: "Take off selected (1)" })).toBeTruthy();
    expect(all.checked).toBe(false);
  });

  test("the order can be changed", async () => {
    const { user } = await openEditor(LIST);
    await user.selectOptions(screen.getByRole("combobox", { name: "Order" }), "newest");
    expect(shownHandles()).toEqual(["Adam#d4c3", "Bob#b2e1", "Cleo#c3d2", "Ada#a1f0"]);
    await user.selectOptions(screen.getByRole("combobox", { name: "Order" }), "oldest");
    expect(shownHandles()).toEqual(["Ada#a1f0", "Cleo#c3d2", "Bob#b2e1", "Adam#d4c3"]);
  });

  test("a per-row Take off marks the row and Keep brings it back", async () => {
    const { user } = await openEditor(LIST);
    await user.click(screen.getByRole("button", { name: "Take off Cleo#c3d2" }));
    expect(rowOf("Cleo#c3d2").className).toContain("going");
    await user.click(within(rowOf("Cleo#c3d2")).getByRole("button", { name: "Keep Cleo#c3d2" }));
    expect(rowOf("Cleo#c3d2").className).not.toContain("going");
    expect(screen.queryByText(/To take off/)).toBeNull();
  });
});

describe("empty states", () => {
  test("a profile with no accounts says nobody is on the list", async () => {
    await openEditor(detail({ policy: "listed", accounts: [] }));
    expect(screen.getByText("Nobody is on the list yet.")).toBeTruthy();
    expect(screen.queryByRole("searchbox", { name: "Filter the list" })).toBeNull();
    expect(screen.queryAllByTestId("access-row")).toEqual([]);
  });

  test("a filter matching nothing says so, and the list returns when it is cleared", async () => {
    const { user } = await openEditor(detail({ policy: "listed", accounts: [ADA, BOB] }));
    const filter = screen.getByRole("searchbox", { name: "Filter the list" });
    await user.type(filter, "zzz");
    expect(screen.getByText("No player on the list matches the filter.")).toBeTruthy();
    expect(screen.queryAllByTestId("access-row")).toEqual([]);
    expect(screen.queryByText("Nobody is on the list yet.")).toBeNull();
    await user.clear(filter);
    expect(rows()).toHaveLength(2);
  });
});
