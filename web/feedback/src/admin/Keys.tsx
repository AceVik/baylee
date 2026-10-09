// The closed-beta keys (`/admin/keys`): made, shown once, listed and
// revoked through the service, with the changes made here.

import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";

import {
  api,
  INVITE_STATES,
  SignedOut,
  type ConsoleChange,
  type Invite,
  type InviteOrder,
  type InviteState,
  type MadeKeys,
} from "../api";
import { formatAgo, formatCount, formatWhen, t, type Key, type Lang } from "../i18n";
import { Badge, copy, describe, Panel, useNow } from "./shared";

/** The expiries the form offers, as `invite create --expires` spells them. */
const EXPIRIES: { value: string; label: Key }[] = [
  { value: "", label: "invites.never" },
  { value: "12h", label: "invites.hours12" },
  { value: "1d", label: "invites.day1" },
  { value: "7d", label: "invites.days7" },
  { value: "30d", label: "invites.days30" },
  { value: "90d", label: "invites.days90" },
];

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


export function Keys({ lang }: { lang: Lang }) {
  const [invites, setInvites] = useState<Invite[] | null>(null);
  const [changes, setChanges] = useState<ConsoleChange[]>([]);
  const [made, setMade] = useState<MadeKeys | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [state, setState] = useState<InviteState | "all">("all");
  const now = useNow();

  const load = useCallback(() => {
    api.admin
      .invites()
      .then((list) => {
        setInvites(list);
        setError(null);
      })
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
  useEffect(load, [load]);

  const counts = (s: InviteState | "all") => (invites ?? []).filter((i) => s === "all" || i.state === s).length;
  const shown = (invites ?? []).filter((i) => state === "all" || i.state === state);
  const active = (invites ?? []).filter((i) => i.state === "active");

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "invites.title")}</h1>
          {invites !== null && (
            <p className="muted small">
              {t(lang, "keys.summary", {
                active: formatCount(lang, active.length),
                left: formatCount(
                  lang,
                  active.reduce((sum, i) => sum + i.uses_left, 0),
                ),
                admitted: formatCount(
                  lang,
                  invites.reduce((sum, i) => sum + i.admitted, 0),
                ),
              })}
            </p>
          )}
        </div>
        <button type="button" onClick={load}>
          {t(lang, "admin.refresh")}
        </button>
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="invites" aria-labelledby="invites-title">
        <h2 id="invites-title" className="sr-only">
          {t(lang, "invites.title")}
        </h2>
        {made === null ? (
          <CreateForm
            lang={lang}
            onMade={(keys) => {
              setMade(keys);
              setNotice(null);
              load();
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
        <div className="panel-head list-head">
          <h3>{t(lang, "invites.list")}</h3>
          <fieldset className="segmented">
<legend className="sr-only">{t(lang, "invites.state")}</legend>
            {(["all", ...INVITE_STATES] as const).map((s) => (
              <button
                key={s}
                type="button"
                aria-pressed={state === s}
                onClick={() => {
                  setState(s);
                }}
              >
                {s === "all" ? t(lang, "live.all") : t(lang, `state.${s}`)}{" "}
                <span className="seg-count">{formatCount(lang, counts(s))}</span>
              </button>
            ))}
          </fieldset>
        </div>
        <p className="muted small">{t(lang, "invites.noKeyShown")}</p>
        {invites === null ? (
          <p className="muted">{t(lang, "admin.loading")}</p>
        ) : shown.length === 0 ? (
          <p className="empty">{t(lang, "invites.none")}</p>
        ) : (
          <ul className="invite-list">
            {shown.map((invite) => (
              <InviteItem
                key={invite.id}
                lang={lang}
                invite={invite}
                onRevoked={() => {
                  setNotice(t(lang, "invites.revoked"));
                  load();
                }}
                onError={setError}
              />
            ))}
          </ul>
        )}
        <Panel id="audit-title" title={t(lang, "audit.title")} className="audit-panel">
          {changes.length === 0 ? (
            <p className="empty">{t(lang, "audit.none")}</p>
          ) : (
            <ol className="audit console-audit">
              {changes.map((c) => (
                <li key={`${c.at}-${c.action}-${c.detail ?? ""}`}>
                  <span className="muted" title={formatWhen(lang, c.at)}>
                    {formatAgo(lang, c.at, now)}
                  </span>{" "}
                  <strong>{c.actor}</strong> <Badge>{c.action}</Badge>
                  {c.detail !== null && <span className="muted small audit-detail"> {c.detail}</span>}
                </li>
              ))}
            </ol>
          )}
        </Panel>
      </section>
    </>
  );
}
