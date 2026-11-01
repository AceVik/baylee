// The hosted language models (`/admin/models`, docs/feedback.md §"Hosted
// models"): the connected seat agents, the profiles each runs and their
// state, and the changes, each passed on to the gateway and answered as it
// answers. A key is write-only: typed, sent, cleared, never shown again.

import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";

import {
  api,
  LLM_STATES,
  SignedOut,
  type ConsoleChange,
  type LlmAccess,
  type LlmCaps,
  type LlmDefinition,
  type LlmProfile,
  type LlmState,
  type SeatHost,
} from "../api";
import { formatAgo, formatCount, formatDuration, formatWhen, localeOf, t, type Key, type Lang } from "../i18n";
import { AccessPanel } from "./ModelAccess";
import { Badge, describe, Freshness, LIVE_MS, Panel, useNow, useVisibleInterval, type Tone } from "./shared";

const STATE_TONE: Record<LlmState, Tone> = {
  available: "ok",
  busy: "info",
  probing: "info",
  exhausted: "warn",
  needs_login: "warn",
  failing: "danger",
  disabled: "muted",
};

const NAME = /^[A-Za-z0-9._-]{1,64}$/;

/** One profile id over every seat agent that holds it. */
export interface ProfileGroup {
  id: string;
  head: LlmProfile;
  on: { host: string; profile: LlmProfile }[];
}

/** Profiles by id, best state first, then by label, as the player list orders them. */
export function groupProfiles(hosts: SeatHost[]): ProfileGroup[] {
  const groups = new Map<string, ProfileGroup>();
  for (const host of [...hosts].sort((a, b) => a.name.localeCompare(b.name))) {
    for (const profile of host.profiles) {
      const group = groups.get(profile.id);
      if (group === undefined) groups.set(profile.id, { id: profile.id, head: profile, on: [{ host: host.name, profile }] });
      else group.on.push({ host: host.name, profile });
    }
  }
  const best = (g: ProfileGroup) => Math.min(...g.on.map((o) => LLM_STATES.indexOf(o.profile.state)));
  return [...groups.values()].sort((a, b) => best(a) - best(b) || a.head.label.localeCompare(b.head.label));
}

/** "until 14:30", or a date and time when it is not today. */
export function formatUntil(lang: Lang, unix: number, now: number = Date.now()): string {
  const at = new Date(unix * 1000);
  const sameDay = new Date(now).toDateString() === at.toDateString();
  const when = new Intl.DateTimeFormat(localeOf(lang), sameDay ? { timeStyle: "short" } : { dateStyle: "short", timeStyle: "short" }).format(at);
  return t(lang, "llm.until", { when });
}

function usd(lang: Lang, n: number): string {
  return new Intl.NumberFormat(localeOf(lang), {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(n);
}

/** Spend against its cap, "$0.42 / $10", or the spend alone. */
function spend(lang: Lang, spent: number | undefined, cap: number | undefined): string {
  const used = usd(lang, spent ?? 0);
  return cap === undefined ? used : `${used} / ${usd(lang, cap)}`;
}

/** A thin bar: `value` of `max`, or nothing when there is no bound. */
/** The numbers beside it say it in words; the bar is for the eye only. */
function Meter({ value, max }: { value: number; max: number | null }) {
  if (max === null || max <= 0) return null;
  const share = Math.min(1, value / max);
  return (
    <span className={`llm-meter${share >= 1 ? " full" : ""}`} aria-hidden="true">
      <span style={{ inlineSize: `${(share * 100).toFixed(1)}%` }} />
    </span>
  );
}

/** A modal on the browser's own `<dialog>`: focus held inside, Escape closes. */
export function Modal({
  labelledBy,
  alert = false,
  className = "",
  onClose,
  children,
}: {
  labelledBy: string;
  alert?: boolean;
  className?: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    if (dialog === null || dialog.open) return;
    if (typeof dialog.showModal === "function") dialog.showModal();
    else dialog.setAttribute("open", "");
  }, []);
  return (
    <dialog
      ref={ref}
      className={`llm-dialog card ${className}`}
      role={alert ? "alertdialog" : undefined}
      aria-labelledby={labelledBy}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      {children}
    </dialog>
  );
}

type Notice = { tone: "ok" | "danger"; text: string };

/** A question before a change. */
interface Ask {
  title: string;
  yes: string;
  /** A save is a plain yes; a removal is the danger tone (the default). */
  tone?: "primary" | "danger";
  run: () => Promise<void>;
}

function ConfirmDialog({ lang, ask, onClose }: { lang: Lang; ask: Ask; onClose: () => void }) {
  const yes = useRef<HTMLButtonElement>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    yes.current?.focus();
  }, []);
  return (
    <Modal labelledBy="llm-ask" alert onClose={onClose}>
        <h3 id="llm-ask">{ask.title}</h3>
        <div className="row-actions">
          <button
            ref={yes}
            type="button"
            className={ask.tone ?? "danger"}
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void ask.run().finally(onClose);
            }}
          >
            {ask.yes}
          </button>
          <button type="button" disabled={busy} onClick={onClose}>
            {t(lang, "llm.cancel")}
          </button>
        </div>
    </Modal>
  );
}

/** Where the editor writes: a new profile on a chosen agent, or one that exists. */
interface Editing {
  host: string | null;
  id: string | null;
  definition: LlmDefinition;
}

const BLANK: LlmDefinition = {
  label: "",
  vendor: "",
  enabled: true,
  profile: { provider: "anthropic", model: "" },
};

/** The profile's fields the form edits one by one; the rest stays in its JSON. */
export const BUDGET_KEYS = ["game_usd", "game_tokens", "game_calls", "price"] as const;

function numberOrUndefined(text: string | undefined): number | undefined {
  if (text === undefined || text.trim() === "") return undefined;
  const n = Number(text);
  return Number.isFinite(n) && n >= 0 ? n : undefined;
}

/** Whether a box holds something that is not a number of zero or more. */
function notANumber(text: string | undefined): boolean {
  return text !== undefined && text.trim() !== "" && numberOrUndefined(text) === undefined;
}

/** A number of the profile's, as a box shows it. */
function boxed(value: unknown): string {
  return typeof value === "number" ? value.toString() : "";
}

/** The profile without the fields the form edits one by one, as its JSON box shows it. */
export function otherSettings(profile: Record<string, unknown>): Record<string, unknown> {
  const rest: Record<string, unknown> = { ...profile };
  for (const key of BUDGET_KEYS) delete rest[key];
  return rest;
}

/** What the form holds; the boxes added for the budget, the token caps and the fallback may be absent. */
export interface DefinitionForm {
  label: string;
  vendor: string;
  enabled: boolean;
  canary: boolean;
  maxGames: string;
  dayUsd: string;
  monthUsd: string;
  dayTokens?: string;
  monthTokens?: string;
  gameUsd?: string;
  gameTokens?: string;
  gameCalls?: string;
  priceIn?: string;
  priceOut?: string;
  /** The fallback chain, in the order its profiles take over; blank entries are none. */
  fallback?: string[];
  profile: string;
}

/** The definition the form says, or the sentence that says why not. */
export function readDefinition(form: DefinitionForm): LlmDefinition | Key {
  const label = form.label.trim();
  const vendor = form.vendor.trim();
  if (label.length < 1 || label.length > 40 || vendor.length < 1 || vendor.length > 40) return "llm.badNames";
  let parsed: unknown;
  try {
    parsed = JSON.parse(form.profile);
  } catch {
    return "llm.badProfile";
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return "llm.badProfile";
  const boxes = [form.maxGames, form.dayUsd, form.monthUsd, form.dayTokens, form.monthTokens, form.gameUsd, form.gameTokens, form.gameCalls, form.priceIn, form.priceOut];
  if (boxes.some(notANumber)) return "llm.badNumber";
  // The boxes own the budget: whatever the JSON says of it is the boxes' to say.
  const profile = otherSettings(parsed as Record<string, unknown>);
  const gameUsd = numberOrUndefined(form.gameUsd);
  const gameTokens = numberOrUndefined(form.gameTokens);
  const gameCalls = numberOrUndefined(form.gameCalls);
  if (gameUsd !== undefined) profile["game_usd"] = gameUsd;
  if (gameTokens !== undefined) profile["game_tokens"] = Math.floor(gameTokens);
  if (gameCalls !== undefined) profile["game_calls"] = Math.floor(gameCalls);
  const priceIn = numberOrUndefined(form.priceIn);
  const priceOut = numberOrUndefined(form.priceOut);
  if ((priceIn === undefined) !== (priceOut === undefined)) return "llm.badPrice";
  if (priceIn !== undefined && priceOut !== undefined) profile["price"] = { input: priceIn, output: priceOut };
  const definition: LlmDefinition = { label, vendor, enabled: form.enabled, profile };
  const maxGames = numberOrUndefined(form.maxGames);
  if (maxGames !== undefined) definition.max_games = Math.floor(maxGames);
  const caps: LlmCaps = {};
  const day = numberOrUndefined(form.dayUsd);
  const month = numberOrUndefined(form.monthUsd);
  const dayTokens = numberOrUndefined(form.dayTokens);
  const monthTokens = numberOrUndefined(form.monthTokens);
  if (day !== undefined) caps.day_usd = day;
  if (month !== undefined) caps.month_usd = month;
  if (dayTokens !== undefined) caps.day_tokens = Math.floor(dayTokens);
  if (monthTokens !== undefined) caps.month_tokens = Math.floor(monthTokens);
  if (Object.keys(caps).length > 0) definition.caps = caps;
  if (form.canary) definition.canary = true;
  // One fallback is written as its id, as it always was; a chain as the list.
  const chain = [...new Set((form.fallback ?? []).map((id) => id.trim()).filter((id) => id !== ""))];
  const [only] = chain;
  if (chain.length === 1 && only !== undefined) definition.fallback = only;
  else if (chain.length > 1) definition.fallback = chain;
  return definition;
}

/** A definition's fallbacks as a chain: one id, a list of them, or none. */
export function chainOf(fallback: string | string[] | undefined): string[] {
  if (fallback === undefined) return [];
  return typeof fallback === "string" ? [fallback] : [...fallback];
}

/** Whether the profile's JSON names an agent CLI, whose fallbacks play its model behind an API. */
function playsCli(profile: string): boolean {
  try {
    const parsed: unknown = JSON.parse(profile);
    return typeof parsed === "object" && parsed !== null && (parsed as Record<string, unknown>)["provider"] === "cli";
  } catch {
    return false;
  }
}

/** One vendor, case and outer blanks aside, as the seat agent compares them. */
const sameVendor = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/**
 * The profiles of `host` that may stand in for `id`: models behind an API, not itself, of the
 * same `vendor` (the room shows the profile's vendor as where the game data goes).
 */
export function fallbackChoices(hosts: SeatHost[], host: string, id: string, vendor: string): LlmProfile[] {
  const on = hosts.find((h) => h.name === host);
  return (on?.profiles ?? [])
    .filter((p) => p.kind === "api" && p.id !== id && sameVendor(p.vendor, vendor))
    .sort((a, b) => a.label.localeCompare(b.label) || a.id.localeCompare(b.id));
}

/** Known API and CLI models with human-readable labels and supported thinking/effort levels. */
interface ModelPreset {
  provider: string;
  tool?: string;
  id: string;
  label: string;
  efforts: readonly string[];
  defaultEffort?: string;
}

const MODEL_PRESETS: readonly ModelPreset[] = [
  // Anthropic API
  { provider: "anthropic", id: "claude-opus-4-7", label: "Claude Opus 4.7", efforts: ["low", "medium", "high", "xhigh", "max"], defaultEffort: "high" },
  { provider: "anthropic", id: "claude-opus-4-6", label: "Claude Opus 4.6", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "anthropic", id: "claude-sonnet-4-6", label: "Claude Sonnet 4.6", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "anthropic", id: "claude-haiku-4-5", label: "Claude Haiku 4.5", efforts: [] },
  // OpenAI API
  { provider: "openai", id: "gpt-5.4", label: "GPT-5.4", efforts: ["none", "low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "openai", id: "gpt-5.4-mini", label: "GPT-5.4 Mini", efforts: ["none", "low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "openai", id: "gpt-5.4-nano", label: "GPT-5.4 Nano", efforts: ["none", "low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "openai", id: "gpt-5.4-pro", label: "GPT-5.4 Pro", efforts: ["medium", "high", "xhigh"], defaultEffort: "high" },
  // Gemini API
  { provider: "gemini", id: "gemini-3.1-pro-preview", label: "Gemini 3.1 Pro", efforts: ["low", "medium", "high"], defaultEffort: "high" },
  { provider: "gemini", id: "gemini-3-flash-preview", label: "Gemini 3 Flash", efforts: ["minimal", "low", "medium", "high"], defaultEffort: "high" },
  { provider: "gemini", id: "gemini-3.1-flash-lite-preview", label: "Gemini 3.1 Flash-Lite", efforts: ["minimal", "low", "medium", "high"], defaultEffort: "high" },
  { provider: "gemini", id: "gemini-2.5-pro", label: "Gemini 2.5 Pro", efforts: ["low", "medium", "high"], defaultEffort: "high" },
  { provider: "gemini", id: "gemini-2.5-flash", label: "Gemini 2.5 Flash", efforts: ["minimal", "low", "medium", "high"], defaultEffort: "medium" },
  // OpenAI-compatible (DeepSeek)
  { provider: "openai-compatible", id: "deepseek-reasoner", label: "DeepSeek Reasoner (R1)", efforts: [] },
  { provider: "openai-compatible", id: "deepseek-chat", label: "DeepSeek Chat (V3)", efforts: [] },
  // Claude Code CLI
  { provider: "cli", tool: "claude", id: "claude:claude-opus-4-7", label: "Claude Opus 4.7", efforts: ["low", "medium", "high", "xhigh", "max"], defaultEffort: "xhigh" },
  { provider: "cli", tool: "claude", id: "claude:claude-sonnet-4-6", label: "Claude Sonnet 4.6", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "cli", tool: "claude", id: "claude:claude-opus-4-6", label: "Claude Opus 4.6", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "cli", tool: "claude", id: "claude:claude-haiku-4-5", label: "Claude Haiku 4.5", efforts: [] },
  { provider: "cli", tool: "claude", id: "claude:opus", label: "Claude Opus (Alias)", efforts: ["low", "medium", "high", "xhigh", "max"], defaultEffort: "xhigh" },
  { provider: "cli", tool: "claude", id: "claude:sonnet", label: "Claude Sonnet (Alias)", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  // Antigravity CLI (agy)
  { provider: "cli", tool: "agy", id: "agy:claude-opus-4-6-thinking", label: "Claude Opus 4.6 (Thinking)", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "cli", tool: "agy", id: "agy:claude-sonnet-4-6", label: "Claude Sonnet 4.6", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "cli", tool: "agy", id: "agy:gemini-3.1-pro-high", label: "Gemini 3.1 Pro (High)", efforts: [] },
  { provider: "cli", tool: "agy", id: "agy:gemini-3.1-pro-low", label: "Gemini 3.1 Pro (Low)", efforts: [] },
  { provider: "cli", tool: "agy", id: "agy:gemini-3-flash-agent", label: "Gemini 3 Flash (Agent)", efforts: ["minimal", "low", "medium", "high"], defaultEffort: "high" },
  { provider: "cli", tool: "agy", id: "agy:gpt-oss-120b-medium", label: "GPT-OSS 120B (Medium)", efforts: [] },
  // Codex CLI
  { provider: "cli", tool: "codex", id: "codex:gpt-5.4", label: "GPT-5.4", efforts: ["low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "cli", tool: "codex", id: "codex:gpt-5.4-mini", label: "GPT-5.4 Mini", efforts: ["low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "cli", tool: "codex", id: "codex:gpt-5.3-codex", label: "GPT-5.3 Codex", efforts: ["low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "cli", tool: "codex", id: "codex:gpt-5.2-codex", label: "GPT-5.2 Codex", efforts: ["low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  // Opencode CLI
  { provider: "cli", tool: "opencode", id: "opencode:anthropic/claude-sonnet-4-6", label: "Claude Sonnet 4.6 (Anthropic)", efforts: ["low", "medium", "high", "max"], defaultEffort: "high" },
  { provider: "cli", tool: "opencode", id: "opencode:anthropic/claude-opus-4-7", label: "Claude Opus 4.7 (Anthropic)", efforts: ["low", "medium", "high", "xhigh", "max"], defaultEffort: "xhigh" },
  { provider: "cli", tool: "opencode", id: "opencode:openai/gpt-5.4", label: "GPT-5.4 (OpenAI)", efforts: ["none", "minimal", "low", "medium", "high", "xhigh"], defaultEffort: "medium" },
  { provider: "cli", tool: "opencode", id: "opencode:google/gemini-3.1-pro-preview", label: "Gemini 3.1 Pro (Google)", efforts: ["low", "medium", "high"], defaultEffort: "high" },
  { provider: "cli", tool: "opencode", id: "opencode", label: "opencode (Default)", efforts: ["minimal", "low", "medium", "high", "max"] },
  // Junie CLI
  { provider: "cli", tool: "junie", id: "junie:opus", label: "Claude Opus 4.6 (Junie)", efforts: [] },
  { provider: "cli", tool: "junie", id: "junie:sonnet", label: "Claude Sonnet 4.6 (Junie)", efforts: [] },
  { provider: "cli", tool: "junie", id: "junie:gpt", label: "GPT-5.4 (Junie)", efforts: [] },
  { provider: "cli", tool: "junie", id: "junie:gemini-pro", label: "Gemini 3.1 Pro (Junie)", efforts: [] },
  { provider: "cli", tool: "junie", id: "junie:gemini-flash", label: "Gemini 3 Flash (Junie)", efforts: [] },
];

const CLI_TOOLS = [
  { id: "claude", label: "Claude Code", defaultModel: "claude:claude-opus-4-7" },
  { id: "agy", label: "agy (Antigravity)", defaultModel: "agy:claude-opus-4-6-thinking" },
  { id: "codex", label: "Codex", defaultModel: "codex:gpt-5.4" },
  { id: "opencode", label: "opencode", defaultModel: "opencode:anthropic/claude-sonnet-4-6" },
  { id: "junie", label: "Junie", defaultModel: "junie:sonnet" },
] as const;

export function findModelPreset(modelId: string): ModelPreset | undefined {
  return MODEL_PRESETS.find((m) => m.id === modelId);
}

/** Human-readable model label when known and distinct from the raw ID/profile label. */
export function prettyModelName(modelId: string): string | null {
  const preset = findModelPreset(modelId);
  return preset !== undefined && preset.label !== modelId ? preset.label : null;
}

function parseProfileObject(raw: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // Hand-edited invalid JSON stays untouched.
  }
  return null;
}

function Editor({
  lang,
  hosts,
  editing,
  onSave,
  onClose,
}: {
  lang: Lang;
  hosts: SeatHost[];
  editing: Editing;
  /** Writes it; answers why not in a sentence, or `null` once it is written. */
  onSave: (host: string, id: string, definition: LlmDefinition) => Promise<string | null>;
  onClose: () => void;
}) {
  const d = editing.definition;
  const p = d.profile;
  const isNew = editing.id === null;
  const [host, setHost] = useState(editing.host ?? hosts[0]?.name ?? "");
  const [id, setId] = useState(editing.id ?? "");
  const [label, setLabel] = useState(d.label);
  const [vendor, setVendor] = useState(d.vendor);
  const [enabled, setEnabled] = useState(d.enabled ?? true);
  const [canary, setCanary] = useState(d.canary ?? false);
  const [maxGames, setMaxGames] = useState(d.max_games?.toString() ?? "");
  const [fallback, setFallback] = useState<string[]>(chainOf(d.fallback));
  const [gameUsd, setGameUsd] = useState(boxed(p["game_usd"]));
  const [gameTokens, setGameTokens] = useState(boxed(p["game_tokens"]));
  const [gameCalls, setGameCalls] = useState(boxed(p["game_calls"]));
  const price = p["price"] as { input?: unknown; output?: unknown } | undefined;
  const [priceIn, setPriceIn] = useState(boxed(price?.input));
  const [priceOut, setPriceOut] = useState(boxed(price?.output));
  const [dayUsd, setDayUsd] = useState(d.caps?.day_usd?.toString() ?? "");
  const [monthUsd, setMonthUsd] = useState(d.caps?.month_usd?.toString() ?? "");
  const [dayTokens, setDayTokens] = useState(d.caps?.day_tokens?.toString() ?? "");
  const [monthTokens, setMonthTokens] = useState(d.caps?.month_tokens?.toString() ?? "");
  const [profile, setProfile] = useState(JSON.stringify(otherSettings(p), null, 2));
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<LlmDefinition | null>(null);
  const first = useRef<HTMLInputElement>(null);
  useEffect(() => {
    first.current?.focus();
  }, []);

  const cli = playsCli(profile);
  const parsedProfile = parseProfileObject(profile);
  const currentProvider = typeof parsedProfile?.["provider"] === "string" ? parsedProfile["provider"] : "";
  const currentModel = typeof parsedProfile?.["model"] === "string" ? parsedProfile["model"] : "";
  const currentEffort = typeof parsedProfile?.["effort"] === "string" ? parsedProfile["effort"] : "";
  const currentCliTool =
    currentProvider === "cli"
      ? (CLI_TOOLS.find((tool) => currentModel === tool.id || currentModel.startsWith(`${tool.id}:`))?.id ?? "claude")
      : null;
  const activePreset = findModelPreset(currentModel);
  const providerPresets = MODEL_PRESETS.filter((m) =>
    currentProvider === "cli" ? m.provider === "cli" && m.tool === currentCliTool : m.provider === currentProvider,
  );
  const updateProfileJson = (mutate: (next: Record<string, unknown>) => void) => {
    const base = parseProfileObject(profile) ?? {};
    const next: Record<string, unknown> = { ...base };
    mutate(next);
    setProfile(JSON.stringify(next, null, 2));
  };
  const choices = fallbackChoices(hosts, host, id, vendor);
  const onHost = (fid: string) => hosts.find((h) => h.name === host)?.profiles.some((other) => other.id === fid) ?? false;
  // One select a fallback of the chain, and one more to add the next; choosing none ends the
  // chain there.
  const slots = [...fallback, ""];
  const choose = (at: number, value: string) => {
    if (value === "") {
      setFallback(fallback.slice(0, at));
      return;
    }
    const next = [...fallback];
    next[at] = value;
    setFallback(next.filter((other, i) => i <= at || other !== value));
  };
  const what = `${id} @ ${host}`;

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!NAME.test(host) || !NAME.test(id)) {
      setError(t(lang, "llm.badId"));
      return;
    }
    const read = readDefinition({
      label,
      vendor,
      enabled,
      canary,
      maxGames,
      dayUsd,
      monthUsd,
      dayTokens,
      monthTokens,
      gameUsd,
      gameTokens,
      gameCalls,
      priceIn,
      priceOut,
      fallback,
      profile,
    });
    if (typeof read === "string") {
      setError(t(lang, read));
      return;
    }
    setError(null);
    setConfirming(read);
  };

  const field = (fid: string, labelKey: Key, input: ReactNode, hint?: Key, wide = false) => (
    <div className={`field${wide ? " wide" : ""}`}>
      <label htmlFor={fid}>{t(lang, labelKey)}</label>
      {input}
      {hint !== undefined && (
        <p id={`${fid}-hint`} className="muted llm-hint">
          {t(lang, hint)}
        </p>
      )}
    </div>
  );

  /**
   * A number box; `whole` for a count, else cents. `hint` says it is described by the hint of its
   * own field (`<fid>-hint`), or by the paragraph `described` names; `named` labels a box that has
   * no label of its own.
   */
  const numberBox = (
    fid: string,
    value: string,
    set: (v: string) => void,
    whole: boolean,
    hint?: Key,
    named?: Key,
    described?: string,
  ) => (
    <input
      id={fid}
      type="number"
      inputMode={whole ? "numeric" : "decimal"}
      min={0}
      step={whole ? 1 : "0.01"}
      placeholder={t(lang, "llm.noBound")}
      aria-label={named === undefined ? undefined : t(lang, named)}
      aria-describedby={described ?? (hint === undefined ? undefined : `${fid}-hint`)}
      value={value}
      onChange={(e) => {
        set(e.target.value);
      }}
    />
  );

  return (
    <Modal labelledBy="llm-edit-title" className="llm-editor" onClose={onClose}>
      <form className="llm-editor-form" onSubmit={submit}>
        <h3 id="llm-edit-title">
          {isNew ? t(lang, "llm.add") : t(lang, "llm.editTitle", { id: editing.id ?? "", host: editing.host ?? "" })}
        </h3>

        <fieldset className="llm-section">
          <legend>{t(lang, "llm.section.profile")}</legend>
          <div className="form-grid">
            {isNew &&
              field(
                "llm-host",
                "llm.host",
                <select
                  id="llm-host"
                  value={host}
                  onChange={(e) => {
                    setHost(e.target.value);
                    // A fallback is a profile of the seat agent: another one's is none.
                    setFallback([]);
                  }}
                >
                  {hosts.map((h) => (
                    <option key={h.name} value={h.name}>
                      {h.name}
                    </option>
                  ))}
                </select>,
              )}
            {isNew &&
              field(
                "llm-id",
                "llm.id",
                <input
                  id="llm-id"
                  ref={first}
                  required
                  pattern="[A-Za-z0-9._\-]{1,64}"
                  autoComplete="off"
                  className="mono"
                  value={id}
                  onChange={(e) => {
                    setId(e.target.value);
                  }}
                />,
              )}
            {field(
              "llm-label",
              "llm.label",
              <input
                id="llm-label"
                ref={isNew ? undefined : first}
                required
                maxLength={40}
                value={label}
                onChange={(e) => {
                  setLabel(e.target.value);
                }}
              />,
            )}
            {field(
              "llm-vendor",
              "llm.vendor",
              <input
                id="llm-vendor"
                required
                maxLength={40}
                value={vendor}
                onChange={(e) => {
                  setVendor(e.target.value);
                }}
              />,
            )}
          </div>
          <div className="form-grid">
            <div className="field llm-check">
              <label>
                <input
                  type="checkbox"
                  checked={enabled}
                  aria-describedby="llm-enabled-hint"
                  onChange={(e) => {
                    setEnabled(e.target.checked);
                  }}
                />
                {t(lang, "llm.enabled")}
              </label>
              <p id="llm-enabled-hint" className="muted llm-hint">
                {t(lang, "llm.hint.enabled")}
              </p>
            </div>
            <div className="field llm-check">
              <label>
                <input
                  type="checkbox"
                  checked={canary}
                  aria-describedby="llm-canary-hint"
                  onChange={(e) => {
                    setCanary(e.target.checked);
                  }}
                />
                {t(lang, "llm.canary")}
              </label>
              <p id="llm-canary-hint" className="muted llm-hint">
                {t(lang, "llm.hint.canary")}
              </p>
            </div>
          </div>
        </fieldset>

        <fieldset className="llm-section">
          <legend>{t(lang, "llm.section.load")}</legend>
          <div className="form-grid">
            {field("llm-max", "llm.maxGames", numberBox("llm-max", maxGames, setMaxGames, true, "llm.hint.maxGames"), "llm.hint.maxGames")}
            {field(
              "llm-fallback",
              "llm.fallback",
              <div className="llm-fallback-chain">
                {slots.map((chosen, at) => {
                  // A profile already earlier in the chain is not offered again.
                  const offered = choices.filter((c) => !fallback.slice(0, at).includes(c.id));
                  const missing = chosen !== "" && !offered.some((c) => c.id === chosen);
                  return (
                    <select
                      key={at}
                      id={at === 0 ? "llm-fallback" : `llm-fallback-${at + 1}`}
                      value={chosen}
                      aria-label={at === 0 ? undefined : t(lang, "llm.fallbackNext", { n: at + 1 })}
                      aria-describedby="llm-fallback-hint"
                      onChange={(e) => {
                        choose(at, e.target.value);
                      }}
                    >
                      <option value="">{t(lang, "llm.fallbackNone")}</option>
                      {offered.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.label} · {c.id}
                        </option>
                      ))}
                      {missing && (
                        <option value={chosen}>
                          {t(lang, onHost(chosen) ? "llm.fallbackUnfit" : "llm.fallbackMissing", { id: chosen })}
                        </option>
                      )}
                    </select>
                  );
                })}
              </div>,
              cli ? "llm.hint.fallbackCli" : "llm.hint.fallback",
            )}
          </div>
        </fieldset>

        <fieldset className="llm-section">
          <legend>{t(lang, "llm.section.game")}</legend>
          <div className="form-grid">
            {field("llm-game-usd", "llm.gameUsd", numberBox("llm-game-usd", gameUsd, setGameUsd, false, "llm.hint.gameUsd"), "llm.hint.gameUsd")}
            <fieldset className="field llm-pair-field">
              <legend className="llm-label">{t(lang, "llm.price")}</legend>
              <div className="llm-pair">
                {numberBox("llm-price-in", priceIn, setPriceIn, false, "llm.hint.price", "llm.priceIn", "llm-price-hint")}
                {numberBox("llm-price-out", priceOut, setPriceOut, false, "llm.hint.price", "llm.priceOut", "llm-price-hint")}
              </div>
              <p id="llm-price-hint" className="muted llm-hint">
                {t(lang, "llm.hint.price")}
              </p>
            </fieldset>
            {field("llm-game-tokens", "llm.gameTokens", numberBox("llm-game-tokens", gameTokens, setGameTokens, true, "llm.hint.gameTokens"), "llm.hint.gameTokens")}
            {field("llm-game-calls", "llm.gameCalls", numberBox("llm-game-calls", gameCalls, setGameCalls, true, "llm.hint.gameCalls"), "llm.hint.gameCalls")}
          </div>
        </fieldset>

        <fieldset className="llm-section">
          <legend>{t(lang, "llm.section.caps")}</legend>
          <p id="llm-caps-hint" className="muted llm-hint">
            {t(lang, "llm.hint.caps")}
          </p>
          <div className="form-grid">
            {field("llm-day", "llm.capDay", numberBox("llm-day", dayUsd, setDayUsd, false, undefined, undefined, "llm-caps-hint"))}
            {field("llm-month", "llm.capMonth", numberBox("llm-month", monthUsd, setMonthUsd, false, undefined, undefined, "llm-caps-hint"))}
            {field("llm-day-tokens", "llm.capDayTokens", numberBox("llm-day-tokens", dayTokens, setDayTokens, true, undefined, undefined, "llm-caps-hint"))}
            {field("llm-month-tokens", "llm.capMonthTokens", numberBox("llm-month-tokens", monthTokens, setMonthTokens, true, undefined, undefined, "llm-caps-hint"))}
          </div>
        </fieldset>

        <fieldset className="llm-section">
          <legend>{t(lang, "llm.section.model")}</legend>
          {parsedProfile !== null && (
            <div className="llm-presets">
              <div className="row-actions">
                {(["anthropic", "openai", "gemini", "openai-compatible", "cli"] as const).map((prov) => (
                  <button
                    key={prov}
                    type="button"
                    className={currentProvider === prov ? "primary" : "quiet"}
                    onClick={() => {
                      updateProfileJson((next) => {
                        next["provider"] = prov;
                        const firstForProv = MODEL_PRESETS.find((m) => m.provider === prov);
                        if (firstForProv !== undefined) next["model"] = firstForProv.id;
                        delete next["effort"];
                      });
                    }}
                  >
                    {prov}
                  </button>
                ))}
              </div>
              {currentProvider === "cli" && (
                <div className="row-actions">
                  {CLI_TOOLS.map((tool) => (
                    <button
                      key={tool.id}
                      type="button"
                      className={currentCliTool === tool.id ? "primary" : "quiet"}
                      onClick={() => {
                        updateProfileJson((next) => {
                          next["provider"] = "cli";
                          next["model"] = tool.defaultModel;
                          delete next["effort"];
                        });
                      }}
                    >
                      {tool.label}
                    </button>
                  ))}
                </div>
              )}
              {providerPresets.length > 0 && (
                <div className="row-actions">
                  {providerPresets.map((preset) => (
                    <button
                      key={preset.id}
                      type="button"
                      className={currentModel === preset.id ? "primary" : "quiet"}
                      onClick={() => {
                        updateProfileJson((next) => {
                          next["model"] = preset.id;
                          if (typeof next["effort"] === "string" && !preset.efforts.includes(next["effort"])) {
                            delete next["effort"];
                          }
                        });
                      }}
                    >
                      {preset.label} · {preset.id}
                    </button>
                  ))}
                </div>
              )}
              {activePreset !== undefined && activePreset.efforts.length > 0 && (
                <div className="row-actions">
                  <button
                    type="button"
                    className={currentEffort === "" ? "primary" : "quiet"}
                    onClick={() => {
                      updateProfileJson((next) => {
                        delete next["effort"];
                      });
                    }}
                  >
                    {activePreset.defaultEffort === undefined ? "Default effort" : `Default effort (${activePreset.defaultEffort})`}
                  </button>
                  {activePreset.efforts.map((eff) => (
                    <button
                      key={eff}
                      type="button"
                      className={currentEffort === eff ? "primary" : "quiet"}
                      onClick={() => {
                        updateProfileJson((next) => {
                          next["effort"] = eff;
                        });
                      }}
                    >
                      Effort: {eff}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
          {field(
            "llm-profile",
            "llm.profile",
            <textarea
              id="llm-profile"
              className="mono"
              rows={6}
              spellCheck={false}
              aria-describedby="llm-profile-hint"
              value={profile}
              onChange={(e) => {
                setProfile(e.target.value);
              }}
            />,
            "llm.profileHint",
            true,
          )}
        </fieldset>

        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="row-actions">
          <button type="submit" className="primary" disabled={confirming !== null}>
            {t(lang, "llm.save")}
          </button>
          <button type="button" onClick={onClose}>
            {t(lang, "llm.cancel")}
          </button>
        </div>
      </form>
      {confirming !== null && (
        <ConfirmDialog
          lang={lang}
          ask={{
            title: t(lang, "llm.ask.save", { what }),
            yes: t(lang, "llm.save"),
            tone: "primary",
            run: async () => {
              const refused = await onSave(host, id, confirming);
              if (refused === null) onClose();
              else setError(t(lang, "llm.refused", { why: refused }));
            },
          }}
          onClose={() => {
            setConfirming(null);
          }}
        />
      )}
    </Modal>
  );
}

/** The key field: write-only, cleared once sent. */
function KeyForm({
  lang,
  host,
  profile,
  onSend,
  onForget,
  onClose,
}: {
  lang: Lang;
  host: string;
  profile: LlmProfile;
  onSend: (key: string) => Promise<boolean>;
  onForget: () => void;
  onClose: () => void;
}) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const fid = `key-${host}-${profile.id}`;
  return (
    <form
      className="llm-key"
      onSubmit={(event) => {
        event.preventDefault();
        if (key === "") return;
        setBusy(true);
        const sent = key;
        setKey("");
        void onSend(sent).then((ok) => {
          setBusy(false);
          if (ok) onClose();
        });
      }}
    >
      <label htmlFor={fid} className="small">
        {t(lang, "llm.keyFor", { id: profile.id, host })}
      </label>
      <div className="llm-key-row">
        <input
          id={fid}
          ref={input}
          type="password"
          autoComplete="off"
          spellCheck={false}
          className="mono"
          value={key}
          onChange={(e) => {
            setKey(e.target.value);
          }}
        />
        <button type="submit" className="primary" disabled={busy || key === ""}>
          {t(lang, "llm.keySend")}
        </button>
        {profile.key === "kept" && (
          <button type="button" className="danger-quiet" disabled={busy} onClick={onForget}>
            {t(lang, "llm.keyForget")}
          </button>
        )}
        <button type="button" className="quiet" disabled={busy} onClick={onClose}>
          {t(lang, "llm.cancel")}
        </button>
      </div>
      <p className="muted small">{t(lang, "llm.keyHint")}</p>
    </form>
  );
}

function HostRow({
  lang,
  host,
  profile,
  now,
  busy,
  act,
  ask,
  edit,
}: {
  lang: Lang;
  host: string;
  profile: LlmProfile;
  now: number;
  busy: boolean;
  act: (done: Key, run: () => Promise<void>) => Promise<boolean>;
  ask: (a: Ask) => void;
  edit: () => void;
}) {
  const [keying, setKeying] = useState(false);
  const p = profile;
  const label = `${p.id} @ ${host}`;
  return (
    <li className={`llm-on state-${p.state}`} data-testid="llm-on">
      <div className="llm-on-head">
        <span className="llm-host-name mono">{host}</span>
        <Badge tone={STATE_TONE[p.state]} dot>
          {t(lang, `llm.state.${p.state}`)}
          {p.until_unix !== null && ` · ${formatUntil(lang, p.until_unix, now)}`}
        </Badge>
      </div>
      <dl className="facts llm-facts">
        <div className="fact">
          <dt>{t(lang, "llm.games")}</dt>
          <dd>
            {formatCount(lang, p.games)} / {p.max_games === null ? "∞" : formatCount(lang, p.max_games)}
            <Meter value={p.games} max={p.max_games} />
          </dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "llm.today")}</dt>
          <dd>{spend(lang, p.spent?.day_usd, p.caps?.day_usd)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "llm.month")}</dt>
          <dd>{spend(lang, p.spent?.month_usd, p.caps?.month_usd)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "llm.key")}</dt>
          <dd>
            <Badge tone={p.key === "kept" || p.key === "none_needed" ? "ok" : "warn"}>{t(lang, `llm.key.${p.key}`)}</Badge>
          </dd>
        </div>
      </dl>
      {p.definition.fallback !== undefined && (
        <p className="muted small llm-fallback-line">
          {t(lang, "llm.fallbackIs", { id: chainOf(p.definition.fallback).join(" → ") })}
          {p.diverts !== undefined && (
            <>
              {" "}
              <Badge tone="info">{t(lang, "llm.diverts", { id: p.diverts })}</Badge>
            </>
          )}
        </p>
      )}
      {p.last_error !== null && (
        <p className="error small llm-error" title={p.last_error}>
          {p.last_error}
        </p>
      )}
      {p.last_ok_unix !== null && (
        <p className="muted small">
          {t(lang, "llm.lastOk", { ago: formatAgo(lang, new Date(p.last_ok_unix * 1000).toISOString(), now) })}
        </p>
      )}
      {keying ? (
        <KeyForm
          lang={lang}
          host={host}
          profile={p}
          onSend={(key) => act("llm.done.key", () => api.admin.llm.key(host, p.id, key))}
          onForget={() => {
            ask({
              title: t(lang, "llm.ask.forget", { what: label }),
              yes: t(lang, "llm.keyForget"),
              run: async () => {
                if (await act("llm.done.keyForgot", () => api.admin.llm.key(host, p.id, null))) setKeying(false);
              },
            });
          }}
          onClose={() => {
            setKeying(false);
          }}
        />
      ) : (
        <div className="llm-actions">
          <button
            type="button"
            aria-pressed={p.enabled}
            className={`llm-switch${p.enabled ? " on" : ""}`}
            disabled={busy}
            aria-label={`${t(lang, "llm.enabled")} ${label}`}
            onClick={() => {
              void act(p.enabled ? "llm.done.disable" : "llm.done.enable", () =>
                api.admin.llm.enable(host, p.id, !p.enabled),
              );
            }}
          >
            <span className="llm-knob" aria-hidden="true" />
            {p.enabled ? t(lang, "llm.on") : t(lang, "llm.off")}
          </button>
          <button
            type="button"
            disabled={busy || p.state === "probing"}
            aria-label={`${t(lang, "llm.probe")} ${label}`}
            onClick={() => {
              void act("llm.done.probe", () => api.admin.llm.probe(host, p.id));
            }}
          >
            {t(lang, "llm.probe")}
          </button>
          <button type="button" disabled={busy} aria-label={`${t(lang, "llm.edit")} ${label}`} onClick={edit}>
            {t(lang, "llm.edit")}
          </button>
          {p.kind === "api" && (
            <button
              type="button"
              disabled={busy}
              aria-label={`${t(lang, "llm.setKey")} ${label}`}
              onClick={() => {
                setKeying(true);
              }}
            >
              {t(lang, "llm.setKey")}
            </button>
          )}
          <button
            type="button"
            className="danger-quiet"
            disabled={busy}
            aria-label={`${t(lang, "llm.delete")} ${label}`}
            onClick={() => {
              ask({
                title: t(lang, "llm.ask.delete", { what: label }),
                yes: t(lang, "llm.delete"),
                run: async () => {
                  await act("llm.done.delete", () => api.admin.llm.remove(host, p.id));
                },
              });
            }}
          >
            {t(lang, "llm.delete")}
          </button>
        </div>
      )}
    </li>
  );
}

export function Models({ lang }: { lang: Lang }) {
  const [hosts, setHosts] = useState<SeatHost[] | null>(null);
  const [access, setAccess] = useState<LlmAccess[] | null>(null);
  const [at, setAt] = useState<string | null>(null);
  const [changes, setChanges] = useState<ConsoleChange[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busy, setBusy] = useState(false);
  const [asking, setAsking] = useState<Ask | null>(null);
  const [editing, setEditing] = useState<Editing | null>(null);
  const now = useNow(10_000);

  const load = useCallback(() => {
    api.admin.llm
      .seathosts()
      .then((list) => {
        setHosts(list);
        setAt(new Date().toISOString());
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
    api.admin.llm
      .access()
      .then(setAccess)
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
    api.admin
      .audit()
      .then((all) => {
        setChanges(all.filter((c) => c.action.startsWith("gateway.llm.")));
      })
      .catch(() => {
        // The changes are a footnote; the page reads without them.
      });
  }, [lang]);
  const shown = useVisibleInterval(load, LIVE_MS);

  useEffect(() => {
    if (notice === null) return;
    const timer = setTimeout(() => {
      setNotice(null);
    }, 5000);
    return () => {
      clearTimeout(timer);
    };
  }, [notice]);

  const act = useCallback(
    async (done: Key, run: () => Promise<void>): Promise<boolean> => {
      setBusy(true);
      try {
        await run();
        setNotice({ tone: "ok", text: t(lang, done) });
        load();
        return true;
      } catch (e: unknown) {
        if (!(e instanceof SignedOut)) setNotice({ tone: "danger", text: describe(lang, e) });
        return false;
      } finally {
        setBusy(false);
      }
    },
    [lang, load],
  );
  const closeAsk = useCallback(() => {
    setAsking(null);
  }, []);

  const groups = hosts === null ? [] : groupProfiles(hosts);
  const games = (hosts ?? []).reduce((sum, h) => sum + h.games, 0);

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "llm.title")}</h1>
          {hosts !== null && (
            <p className="muted small">
              {t(lang, "llm.summary", {
                hosts: hosts.length,
                profiles: groups.length,
                games,
              })}
            </p>
          )}
          <Freshness lang={lang} at={at} shown={shown} />
        </div>
        <div className="row-actions">
          <button type="button" onClick={load}>
            {t(lang, "admin.refresh")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={hosts === null || hosts.length === 0}
            onClick={() => {
              setEditing({ host: null, id: null, definition: BLANK });
            }}
          >
            {t(lang, "llm.add")}
          </button>
        </div>
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <output className={`notice llm-notice tone-${notice?.tone ?? "ok"}${notice === null ? " gone" : ""}`} aria-live="polite">
        {notice?.text ?? ""}
      </output>

      <Panel id="llm-hosts-title" title={t(lang, "llm.hosts")}>
        {hosts === null ? (
          <p className="muted">{t(lang, "admin.loading")}</p>
        ) : hosts.length === 0 ? (
          <p className="empty">{t(lang, "llm.noHosts")}</p>
        ) : (
          <ul className="llm-hosts">
            {hosts.map((h) => (
              <li key={h.name} className="llm-host card" data-testid="llm-host">
                <div className="llm-on-head">
                  <strong className="mono">{h.name}</strong>
                  <span className="llm-badges">
                    <Badge tone="ok" dot>
                      {t(lang, "llm.connected")}
                    </Badge>
                    {h.local && <Badge tone="info">{t(lang, "llm.local")}</Badge>}
                  </span>
                </div>
                <dl className="facts llm-facts">
                  <div className="fact">
                    <dt>{t(lang, "llm.load")}</dt>
                    <dd>
                      {formatCount(lang, h.games)} / {h.capacity === 0 ? "∞" : formatCount(lang, h.capacity)}
                      <Meter value={h.games} max={h.capacity === 0 ? null : h.capacity} />
                    </dd>
                  </div>
                  <div className="fact">
                    <dt>{t(lang, "llm.profiles")}</dt>
                    <dd>{formatCount(lang, h.profiles.length)}</dd>
                  </div>
                  <div className="fact">
                    <dt>{t(lang, "llm.since")}</dt>
                    <dd>{formatDuration(lang, h.connected_secs)}</dd>
                  </div>
                </dl>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel id="llm-profiles-title" title={t(lang, "llm.profiles")}>
        {hosts !== null && groups.length === 0 && <p className="empty">{t(lang, "llm.noProfiles")}</p>}
        <ul className="llm-profiles">
          {groups.map((g) => {
            const pretty = prettyModelName(g.head.model);
            const effort = typeof g.head.definition.profile["effort"] === "string" ? g.head.definition.profile["effort"] : null;
            return (
              <li key={g.id} className="llm-profile card" data-testid="llm-profile">
                <div className="llm-profile-head">
                  <div>
                    <h3>{g.head.label}</h3>
                    <p className="muted small">
                      {g.head.vendor}
                      {pretty !== null && pretty !== g.head.label && ` · ${pretty}`} · <span className="mono">{g.head.model}</span>
                      {effort !== null && effort !== "" && (
                        <>
                          {" "}
                          · <span className="mono">{effort}</span>
                        </>
                      )}{" "}
                      · <span className="mono">{g.id}</span>
                    </p>
                  </div>
                  <span className="llm-badges">
                    <Badge tone="accent">{t(lang, `llm.kind.${g.head.kind}`)}</Badge>
                    {g.on.some((o) => o.profile.canary) && <Badge tone="warn">{t(lang, "llm.canary")}</Badge>}
                  </span>
                </div>
                <ul className="llm-on-list">
                  {g.on.map(({ host, profile }) => (
                    <HostRow
                      key={host}
                      lang={lang}
                      host={host}
                      profile={profile}
                      now={now}
                      busy={busy}
                      act={act}
                      ask={setAsking}
                      edit={() => {
                        setEditing({ host, id: profile.id, definition: profile.definition });
                      }}
                    />
                  ))}
                </ul>
              </li>
            );
          })}
        </ul>
      </Panel>

      <AccessPanel
        lang={lang}
        access={access}
        onChanged={(text) => {
          setNotice({ tone: "ok", text });
          load();
        }}
      />

      {hosts !== null && hosts.length > 0 && groups.length > 0 && (
        <Panel id="llm-assign-title" title={t(lang, "llm.assign")}>
          <p className="muted small">{t(lang, "llm.assignHint")}</p>
          <div className="llm-matrix-wrap">
            <table className="llm-matrix">
              <thead>
                <tr>
                  <th scope="col">{t(lang, "llm.profile")}</th>
                  {hosts.map((h) => (
                    <th key={h.name} scope="col" className="mono">
                      {h.name}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {groups.map((g) => (
                  <tr key={g.id}>
                    <th scope="row">{g.head.label}</th>
                    {hosts.map((h) => {
                      const on = g.on.find((o) => o.host === h.name);
                      const what = `${g.id} @ ${h.name}`;
                      return (
                        <td key={h.name}>
                          <input
                            type="checkbox"
                            className="llm-cell"
                            checked={on !== undefined}
                            disabled={busy}
                            aria-label={what}
                            onChange={() => {
                              if (on === undefined) {
                                void act("llm.done.assign", () => api.admin.llm.write(h.name, g.id, g.head.definition));
                              } else {
                                setAsking({
                                  title: t(lang, "llm.ask.unassign", { what }),
                                  yes: t(lang, "llm.unassign"),
                                  run: async () => {
                                    await act("llm.done.delete", () => api.admin.llm.remove(h.name, g.id));
                                  },
                                });
                              }
                            }}
                          />
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>
      )}

      <Panel id="llm-audit-title" title={t(lang, "audit.title")} className="audit-panel">
        {changes.length === 0 ? (
          <p className="empty">{t(lang, "audit.none")}</p>
        ) : (
          <ol className="audit console-audit">
            {changes.map((c) => (
              <li key={`${c.at}-${c.action}-${c.detail ?? ""}`}>
                <span className="muted" title={formatWhen(lang, c.at)}>
                  {formatAgo(lang, c.at, now)}
                </span>{" "}
                <strong>{c.actor}</strong> <Badge>{c.action.replace(/^gateway\.llm\./, "")}</Badge>
                {c.detail !== null && <span className="muted small audit-detail mono"> {c.detail}</span>}
              </li>
            ))}
          </ol>
        )}
      </Panel>

      {asking !== null && <ConfirmDialog lang={lang} ask={asking} onClose={closeAsk} />}
      {editing !== null && hosts !== null && (
        <Editor
          lang={lang}
          hosts={hosts}
          editing={editing}
          onSave={async (host, id, definition) => {
            try {
              await api.admin.llm.write(host, id, definition);
            } catch (e: unknown) {
              return describe(lang, e);
            }
            setNotice({ tone: "ok", text: t(lang, "llm.done.write") });
            load();
            return null;
          }}
          onClose={() => {
            setEditing(null);
          }}
        />
      )}
    </>
  );
}
