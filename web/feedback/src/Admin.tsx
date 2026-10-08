// The admin console's overview (`/admin`): a gateway's numbers, refreshed
// every 15 seconds while the page is visible, and its closed-beta keys,
// made, listed and revoked through the service (`/ui/api/admin/…`,
// docs/feedback.md §"The admin console").

import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";

import {
  api,
  Refused,
  SignedOut,
  type ConsoleChange,
  type Invite,
  type InviteOrder,
  type MadeKeys,
  type Since,
  type Stats,
} from "./api";
import { formatCount, formatWhen, t, type Key, type Lang } from "./i18n";

/** How often the numbers are asked again while the page is visible. */
export const REFRESH_MS = 15_000;

/** The expiries the form offers, as `invite create --expires` spells them. */
const EXPIRIES: { value: string; label: Key }[] = [
  { value: "", label: "invites.never" },
  { value: "12h", label: "invites.hours12" },
  { value: "1d", label: "invites.day1" },
  { value: "7d", label: "invites.days7" },
  { value: "30d", label: "invites.days30" },
  { value: "90d", label: "invites.days90" },
];

function visible(): boolean {
  return typeof document === "undefined" || document.visibilityState !== "hidden";
}

/**
 * Calls `tick` now and every `ms` while the document is visible; stops
 * while it is hidden and calls it again the moment it shows.
 */
export function useVisibleInterval(tick: () => void, ms: number): boolean {
  const [shown, setShown] = useState(visible);
  const latest = useRef(tick);
  useEffect(() => {
    latest.current = tick;
  });
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const start = () => {
      if (timer !== undefined) return;
      latest.current();
      timer = setInterval(() => {
        latest.current();
      }, ms);
    };
    const stop = () => {
      if (timer !== undefined) clearInterval(timer);
      timer = undefined;
    };
    const changed = () => {
      const now = visible();
      setShown(now);
      if (now) start();
      else stop();
    };
    if (visible()) start();
    document.addEventListener("visibilitychange", changed);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", changed);
    };
  }, [ms]);
  return shown;
}

function describe(lang: Lang, e: unknown): string {
  return e instanceof Refused ? e.message : t(lang, "admin.noAnswer");
}

function Card({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section className="card stat" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {children}
    </section>
  );
}

/** One big number with what it counts. */
function Big({ value, label, testId }: { value: string; label: string; testId: string }) {
  return (
    <p className="big" data-testid={testId}>
      <span className="big-number">{value}</span> <span className="big-label">{label}</span>
    </p>
  );
}

/** Small facts under a big number. */
function Facts({ rows }: { rows: [string, string][] }) {
  return (
    <dl className="facts stat-facts">
      {rows.map(([term, value]) => (
        <div className="fact" key={term}>
          <dt>{term}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function sinceRows(lang: Lang, since: Since): [string, string][] {
  return [
    [t(lang, "stats.today"), formatCount(lang, since.today_utc)],
    [t(lang, "stats.week"), formatCount(lang, since.last_7d)],
    [t(lang, "stats.month"), formatCount(lang, since.last_30d)],
  ];
}

function Dashboard({ lang, stats }: { lang: Lang; stats: Stats }) {
  const n = (value: number) => formatCount(lang, value);
  const cap =
    !stats.guests.enabled
      ? t(lang, "stats.guestsOff")
      : stats.guests.cap === null
        ? t(lang, "stats.noCap")
        : t(lang, "stats.guestCap", { cap: n(stats.guests.cap) });
  return (
    <div className="stats-grid">
      <Card id="st-players" title={t(lang, "stats.players")}>
        <Big value={n(stats.accounts.registered)} label={t(lang, "stats.registered")} testId="registered" />
        <Facts
          rows={[
            [t(lang, "stats.confirmed"), n(stats.accounts.confirmed_email)],
            [t(lang, "stats.guests"), `${n(stats.guests.live)} · ${cap}`],
          ]}
        />
        <h3>{t(lang, "stats.newAccounts")}</h3>
        <Facts rows={sinceRows(lang, stats.accounts.created)} />
      </Card>
      <Card id="st-online" title={t(lang, "stats.online")}>
        <Big value={n(stats.online.players)} label={t(lang, "stats.players")} testId="online" />
        <Facts
          rows={[
            [t(lang, "stats.inLobby"), n(stats.online.in_lobby)],
            [t(lang, "stats.seated"), n(stats.online.seated)],
            [t(lang, "stats.sessions"), n(stats.online.sessions_live)],
            [t(lang, "stats.signedIn"), n(stats.online.accounts_signed_in)],
          ]}
        />
      </Card>
      <Card id="st-games" title={t(lang, "stats.games")}>
        <Big value={n(stats.games.running)} label={t(lang, "stats.running")} testId="running" />
        <Facts
          rows={[
            [t(lang, "stats.local"), n(stats.games.local_running)],
            [t(lang, "stats.waiting"), n(stats.games.waiting)],
          ]}
        />
        <h3>{t(lang, "stats.started")}</h3>
        <Facts rows={sinceRows(lang, stats.games.started)} />
        <h3>{t(lang, "stats.finished")}</h3>
        <Facts
          rows={[[t(lang, "stats.total"), n(stats.games.finished)], ...sinceRows(lang, stats.games.finished_since)]}
        />
      </Card>
      <Card id="st-keys" title={t(lang, "stats.keys")}>
        <Big value={n(stats.invites.active)} label={t(lang, "stats.activeKeys")} testId="keys-active" />
        <Facts
          rows={[
            [t(lang, "stats.usesLeft"), n(stats.invites.uses_left)],
            [t(lang, "stats.admitted"), n(stats.invites.admitted)],
            [t(lang, "stats.usedUp"), n(stats.invites.used_up)],
            [t(lang, "stats.expired"), n(stats.invites.expired)],
            [t(lang, "stats.revoked"), n(stats.invites.revoked)],
          ]}
        />
      </Card>
      <Card id="st-server" title={t(lang, "stats.server")}>
        <Big value={n(stats.agents.connected)} label={t(lang, "stats.agents")} testId="agents" />
        <Facts
          rows={[
            [t(lang, "stats.local"), n(stats.agents.local)],
            [
              t(lang, "stats.capacity"),
              stats.agents.capacity === null ? t(lang, "stats.unlimited") : n(stats.agents.capacity),
            ],
            [t(lang, "stats.agentGames"), n(stats.agents.games)],
            [t(lang, "stats.registration"), stats.gateway.registration],
          ]}
        />
        <h3>{t(lang, "stats.version")}</h3>
        <p className="mono small build">{stats.gateway.version}</p>
      </Card>
    </div>
  );
}

async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** The keys just made, shown this once. */
function Made({ lang, made, onDone }: { lang: Lang; made: MadeKeys; onDone: () => void }) {
  const heading = useRef<HTMLHeadingElement>(null);
  const [copied, setCopied] = useState<string | null>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  const all = made.keys.map((k) => k.key).join("\n");
  const canShare = typeof navigator !== "undefined" && typeof navigator.share === "function";
  return (
    <section className="card made" aria-labelledby="made-title">
      <h3 id="made-title" ref={heading} tabIndex={-1}>
        {t(lang, "invites.made", { n: made.keys.length })}
      </h3>
      <p className="muted small">{t(lang, "invites.madeHint")}</p>
      <ul className="made-keys">
        {made.keys.map(({ id, key }) => (
          <li key={id}>
            <input
              className="key mono"
              readOnly
              value={key}
              aria-label={key}
              onFocus={(event) => {
                event.target.select();
              }}
            />
            <button
              type="button"
              onClick={() => {
                void copy(key).then((ok) => {
                  if (ok) setCopied(id);
                });
              }}
            >
              {copied === id ? t(lang, "invites.copied") : t(lang, "invites.copy")}
            </button>
          </li>
        ))}
      </ul>
      <div className="row-actions">
        {made.keys.length > 1 && (
          <button
            type="button"
            onClick={() => {
              void copy(all).then((ok) => {
                if (ok) setCopied("all");
              });
            }}
          >
            {copied === "all" ? t(lang, "invites.copied") : t(lang, "invites.copyAll")}
          </button>
        )}
        {canShare && (
          <button
            type="button"
            onClick={() => {
              navigator.share({ text: all }).catch(() => {
                // Dismissed: nothing to say.
              });
            }}
          >
            {t(lang, "invites.share")}
          </button>
        )}
        <button type="button" className="primary" onClick={onDone}>
          {t(lang, "invites.done")}
        </button>
      </div>
    </section>
  );
}

function CreateForm({ lang, onMade }: { lang: Lang; onMade: (made: MadeKeys) => void }) {
  const [count, setCount] = useState("1");
  const [uses, setUses] = useState("1");
  const [expires, setExpires] = useState("30d");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const order: InviteOrder = { count: Number(count), uses: Number(uses) };
    if (expires !== "") order.expires = expires;
    if (note.trim() !== "") order.note = note.trim();
    try {
      onMade(await api.admin.create(order));
      setNote("");
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form className="card create-form" onSubmit={(event) => void submit(event)} aria-labelledby="create-title">
      <h3 id="create-title">{t(lang, "invites.create")}</h3>
      <div className="form-grid">
        <div className="field">
          <label htmlFor="inv-count">{t(lang, "invites.count")}</label>
          <input
            id="inv-count"
            type="number"
            inputMode="numeric"
            min={1}
            max={100}
            required
            value={count}
            onChange={(event) => {
              setCount(event.target.value);
            }}
          />
        </div>
        <div className="field">
          <label htmlFor="inv-uses">{t(lang, "invites.uses")}</label>
          <input
            id="inv-uses"
            type="number"
            inputMode="numeric"
            min={1}
            max={1000}
            required
            value={uses}
            onChange={(event) => {
              setUses(event.target.value);
            }}
          />
        </div>
        <div className="field">
          <label htmlFor="inv-expires">{t(lang, "invites.expires")}</label>
          <select
            id="inv-expires"
            value={expires}
            onChange={(event) => {
              setExpires(event.target.value);
            }}
          >
            {EXPIRIES.map((e) => (
              <option key={e.value} value={e.value}>
                {t(lang, e.label)}
              </option>
            ))}
          </select>
        </div>
        <div className="field wide">
          <label htmlFor="inv-note">{t(lang, "invites.note")}</label>
          <input
            id="inv-note"
            maxLength={100}
            autoComplete="off"
            placeholder={t(lang, "invites.notePlaceholder")}
            value={note}
            onChange={(event) => {
              setNote(event.target.value);
            }}
          />
        </div>
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <button type="submit" className="primary" disabled={busy}>
        {busy ? t(lang, "invites.making") : t(lang, "invites.create")}
      </button>
    </form>
  );
}

function InviteItem({
  lang,
  invite,
  onRevoked,
  onError,
}: {
  lang: Lang;
  invite: Invite;
  onRevoked: () => void;
  onError: (message: string) => void;
}) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const revoke = async () => {
    setBusy(true);
    try {
      await api.admin.revoke(invite.id);
      setConfirm(false);
      onRevoked();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) onError(describe(lang, e));
    } finally {
      setBusy(false);
    }
  };
  const label = invite.note ?? invite.id.slice(-12);
  return (
    <li className={`invite state-${invite.state}`} data-testid="invite">
      <div className="invite-head">
        <span className="invite-note">{invite.note ?? <span className="muted mono">{invite.id.slice(-12)}</span>}</span>
        <span className={`tag invite-state state-${invite.state}`}>{t(lang, `state.${invite.state}`)}</span>
      </div>
      <dl className="facts invite-facts">
        <div className="fact">
          <dt>{t(lang, "invites.usesLeft")}</dt>
          <dd>{formatCount(lang, invite.uses_left)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "invites.admitted")}</dt>
          <dd>{formatCount(lang, invite.admitted)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "invites.expiresAt")}</dt>
          <dd>{invite.expires_at === null ? t(lang, "invites.neverExpires") : formatWhen(lang, invite.expires_at)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "invites.created")}</dt>
          <dd>{formatWhen(lang, invite.created_at)}</dd>
        </div>
      </dl>
      {invite.state !== "revoked" &&
        (confirm ? (
          <fieldset className="confirm">
            <legend>{t(lang, "invites.revokeConfirm")}</legend>
            <div className="row-actions">
              <button type="button" className="danger" disabled={busy} onClick={() => void revoke()}>
                {t(lang, "invites.revokeYes")}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setConfirm(false);
                }}
              >
                {t(lang, "invites.revokeNo")}
              </button>
            </div>
          </fieldset>
        ) : (
          <button
            type="button"
            className="danger-quiet"
            aria-label={`${t(lang, "invites.revoke")} ${label}`}
            onClick={() => {
              setConfirm(true);
            }}
          >
            {t(lang, "invites.revoke")}
          </button>
        ))}
    </li>
  );
}

export function Admin({ lang }: { lang: Lang }) {
  const [stats, setStats] = useState<Stats | null>(null);
  const [invites, setInvites] = useState<Invite[] | null>(null);
  const [changes, setChanges] = useState<ConsoleChange[]>([]);
  const [made, setMade] = useState<MadeKeys | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const loadStats = useCallback(() => {
    api.admin
      .stats()
      .then((s) => {
        setStats(s);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [lang]);

  const loadKeys = useCallback(() => {
    api.admin
      .invites()
      .then(setInvites)
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
    api.admin
      .audit()
      .then(setChanges)
      .catch(() => {
        // The changes are a footnote; the page reads without them.
      });
  }, [lang]);

  const shown = useVisibleInterval(loadStats, REFRESH_MS);
  useEffect(loadKeys, [loadKeys]);

  const afterChange = () => {
    loadStats();
    loadKeys();
  };

  return (
    <main className="admin-page">
      <div className="admin-head">
        <h1>{t(lang, "admin.title")}</h1>
        <p className="muted small" aria-live="polite">
          {stats === null
            ? t(lang, "admin.loading")
            : shown
              ? t(lang, "admin.updated", { time: formatWhen(lang, stats.at) })
              : t(lang, "admin.paused")}
        </p>
        <button type="button" onClick={afterChange}>
          {t(lang, "admin.refresh")}
        </button>
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {stats !== null && <Dashboard lang={lang} stats={stats} />}

      <section className="invites" aria-labelledby="invites-title">
        <h2 id="invites-title">{t(lang, "invites.title")}</h2>
        {made === null ? (
          <CreateForm
            lang={lang}
            onMade={(keys) => {
              setMade(keys);
              setNotice(null);
              afterChange();
            }}
          />
        ) : (
          <Made
            lang={lang}
            made={made}
            onDone={() => {
              setMade(null);
            }}
          />
        )}
        {notice !== null && <output className="notice">{notice}</output>}
        <h3>{t(lang, "invites.list")}</h3>
        <p className="muted small">{t(lang, "invites.noKeyShown")}</p>
        {invites === null ? (
          <p className="muted">{t(lang, "admin.loading")}</p>
        ) : invites.length === 0 ? (
          <p className="muted">{t(lang, "invites.none")}</p>
        ) : (
          <ul className="invite-list">
            {invites.map((invite) => (
              <InviteItem
                key={invite.id}
                lang={lang}
                invite={invite}
                onRevoked={() => {
                  setNotice(t(lang, "invites.revoked"));
                  afterChange();
                }}
                onError={setError}
              />
            ))}
          </ul>
        )}
        <h3>{t(lang, "audit.title")}</h3>
        {changes.length === 0 ? (
          <p className="muted">{t(lang, "audit.none")}</p>
        ) : (
          <ol className="audit console-audit">
            {changes.map((c) => (
              <li key={`${c.at}-${c.action}-${c.detail ?? ""}`}>
                <span className="muted">{formatWhen(lang, c.at)}</span> {c.actor}: {c.action}
                {c.detail !== null ? ` (${c.detail})` : ""}
              </li>
            ))}
          </ol>
        )}
      </section>
    </main>
  );
}
