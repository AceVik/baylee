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
  type LlmDefinition,
  type LlmProfile,
  type LlmState,
  type SeatHost,
} from "../api";
import { formatAgo, formatCount, formatDuration, formatWhen, t, type Key, type Lang } from "../i18n";
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
  const locale = lang === "de" ? "de-DE" : "en-GB";
  const when = new Intl.DateTimeFormat(locale, sameDay ? { timeStyle: "short" } : { dateStyle: "short", timeStyle: "short" }).format(at);
  return t(lang, "llm.until", { when });
}

function usd(lang: Lang, n: number): string {
  return new Intl.NumberFormat(lang === "de" ? "de-DE" : "en-GB", {
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
function Modal({
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

/** A question before a change that cannot be undone. */
interface Ask {
  title: string;
  yes: string;
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
            className="danger"
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

function numberOrUndefined(text: string): number | undefined {
  if (text.trim() === "") return undefined;
  const n = Number(text);
  return Number.isFinite(n) && n >= 0 ? n : undefined;
}

/** The definition the form says, or the sentence that says why not. */
export function readDefinition(form: {
  label: string;
  vendor: string;
  enabled: boolean;
  canary: boolean;
  maxGames: string;
  dayUsd: string;
  monthUsd: string;
  profile: string;
}): LlmDefinition | Key {
  const label = form.label.trim();
  const vendor = form.vendor.trim();
  if (label.length < 1 || label.length > 40 || vendor.length < 1 || vendor.length > 40) return "llm.badNames";
  let profile: unknown;
  try {
    profile = JSON.parse(form.profile);
  } catch {
    return "llm.badProfile";
  }
  if (typeof profile !== "object" || profile === null || Array.isArray(profile)) return "llm.badProfile";
  const definition: LlmDefinition = { label, vendor, enabled: form.enabled, profile: profile as Record<string, unknown> };
  const maxGames = numberOrUndefined(form.maxGames);
  if (maxGames !== undefined) definition.max_games = Math.floor(maxGames);
  const day = numberOrUndefined(form.dayUsd);
  const month = numberOrUndefined(form.monthUsd);
  if (day !== undefined || month !== undefined) {
    definition.caps = {};
    if (day !== undefined) definition.caps.day_usd = day;
    if (month !== undefined) definition.caps.month_usd = month;
  }
  if (form.canary) definition.canary = true;
  return definition;
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
  onSave: (host: string, id: string, definition: LlmDefinition) => Promise<boolean>;
  onClose: () => void;
}) {
  const d = editing.definition;
  const isNew = editing.id === null;
  const [host, setHost] = useState(editing.host ?? hosts[0]?.name ?? "");
  const [id, setId] = useState(editing.id ?? "");
  const [label, setLabel] = useState(d.label);
  const [vendor, setVendor] = useState(d.vendor);
  const [enabled, setEnabled] = useState(d.enabled ?? true);
  const [canary, setCanary] = useState(d.canary ?? false);
  const [maxGames, setMaxGames] = useState(d.max_games?.toString() ?? "");
  const [dayUsd, setDayUsd] = useState(d.caps?.day_usd?.toString() ?? "");
  const [monthUsd, setMonthUsd] = useState(d.caps?.month_usd?.toString() ?? "");
  const [profile, setProfile] = useState(JSON.stringify(d.profile, null, 2));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const first = useRef<HTMLInputElement>(null);
  useEffect(() => {
    first.current?.focus();
  }, []);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!NAME.test(host) || !NAME.test(id)) {
      setError(t(lang, "llm.badId"));
      return;
    }
    const read = readDefinition({ label, vendor, enabled, canary, maxGames, dayUsd, monthUsd, profile });
    if (typeof read === "string") {
      setError(t(lang, read));
      return;
    }
    setBusy(true);
    setError(null);
    if (await onSave(host, id, read)) onClose();
    setBusy(false);
  };

  const field = (fid: string, labelKey: Key, input: ReactNode, wide = false) => (
    <div className={`field${wide ? " wide" : ""}`}>
      <label htmlFor={fid}>{t(lang, labelKey)}</label>
      {input}
    </div>
  );

  return (
    <Modal labelledBy="llm-edit-title" className="llm-editor" onClose={onClose}>
      <form className="llm-editor-form" onSubmit={(event) => void submit(event)}>
        <h3 id="llm-edit-title">
          {isNew ? t(lang, "llm.add") : t(lang, "llm.editTitle", { id: editing.id ?? "", host: editing.host ?? "" })}
        </h3>
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
          {field(
            "llm-max",
            "llm.maxGames",
            <input
              id="llm-max"
              type="number"
              inputMode="numeric"
              min={0}
              placeholder={t(lang, "llm.noBound")}
              value={maxGames}
              onChange={(e) => {
                setMaxGames(e.target.value);
              }}
            />,
          )}
          {field(
            "llm-day",
            "llm.capDay",
            <input
              id="llm-day"
              type="number"
              inputMode="decimal"
              min={0}
              step="0.01"
              placeholder={t(lang, "llm.noBound")}
              value={dayUsd}
              onChange={(e) => {
                setDayUsd(e.target.value);
              }}
            />,
          )}
          {field(
            "llm-month",
            "llm.capMonth",
            <input
              id="llm-month"
              type="number"
              inputMode="decimal"
              min={0}
              step="0.01"
              placeholder={t(lang, "llm.noBound")}
              value={monthUsd}
              onChange={(e) => {
                setMonthUsd(e.target.value);
              }}
            />,
          )}
          <div className="field llm-checks">
            <label>
              <input
                type="checkbox"
                checked={enabled}
                onChange={(e) => {
                  setEnabled(e.target.checked);
                }}
              />{" "}
              {t(lang, "llm.enabled")}
            </label>
            <label>
              <input
                type="checkbox"
                checked={canary}
                onChange={(e) => {
                  setCanary(e.target.checked);
                }}
              />{" "}
              {t(lang, "llm.canary")}
            </label>
          </div>
          {field(
            "llm-profile",
            "llm.profile",
            <textarea
              id="llm-profile"
              className="mono"
              rows={6}
              spellCheck={false}
              value={profile}
              onChange={(e) => {
                setProfile(e.target.value);
              }}
            />,
            true,
          )}
        </div>
        <p className="muted small">{t(lang, "llm.profileHint")}</p>
        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="row-actions">
          <button type="submit" className="primary" disabled={busy}>
            {busy ? t(lang, "llm.saving") : t(lang, "llm.save")}
          </button>
          <button type="button" disabled={busy} onClick={onClose}>
            {t(lang, "llm.cancel")}
          </button>
        </div>
      </form>
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
                hosts: formatCount(lang, hosts.length),
                profiles: formatCount(lang, groups.length),
                games: formatCount(lang, games),
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
          {groups.map((g) => (
            <li key={g.id} className="llm-profile card" data-testid="llm-profile">
              <div className="llm-profile-head">
                <div>
                  <h3>{g.head.label}</h3>
                  <p className="muted small">
                    {g.head.vendor} · <span className="mono">{g.head.model}</span> · <span className="mono">{g.id}</span>
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
          ))}
        </ul>
      </Panel>

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
          onSave={(host, id, definition) => act("llm.done.write", () => api.admin.llm.write(host, id, definition))}
          onClose={() => {
            setEditing(null);
          }}
        />
      )}
    </>
  );
}
