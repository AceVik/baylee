// The landing page's contact messages (`/contact`, docs/feedback.md
// §"The landing page's contact form"): the list, newest first, filtered by
// status in the query string, and one message at `/contact/{id}` to read,
// answer by e-mail, mark done or reopen, and delete, with what was done.

import { useCallback, useEffect, useState } from "react";

import { describe, go, Link, useNow } from "./admin/shared";
import {
  api,
  CONTACT_STATUSES,
  SignedOut,
  type ContactDetail,
  type ContactListing,
  type ContactMessage,
  type ContactStatus,
} from "./api";
import { formatAgo, formatCount, formatWhen, t, type Lang } from "./i18n";
import { contactPath, navigate } from "./router";

/** `?status=` as a filter; anything else is "all". */
function statusOf(search: string): ContactStatus | undefined {
  const asked = new URLSearchParams(search).get("status");
  return CONTACT_STATUSES.find((s) => s === asked);
}

/** Who wrote it, as the list and the page name them. */
function sender(lang: Lang, m: ContactMessage): string {
  return m.name ?? t(lang, "contact.anonymous");
}

function StatusBadge({ lang, status }: { lang: Lang; status: ContactStatus }) {
  return (
    <span className={`badge ${status === "new" ? "tone-accent" : "tone-ok"}`}>
      {t(lang, status === "new" ? "contact.open" : "contact.done")}
    </span>
  );
}

export function ContactList({ lang, search }: { lang: Lang; search: string }) {
  const status = statusOf(search);
  const [listing, setListing] = useState<ContactListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const now = useNow();

  const load = useCallback(() => {
    api.contact
      .list(status)
      .then((l) => {
        setListing(l);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [lang, status]);

  useEffect(load, [load]);

  const filters: { value: ContactStatus | undefined; label: "contact.open" | "contact.done" | "contact.all" }[] = [
    { value: "new", label: "contact.open" },
    { value: "done", label: "contact.done" },
    { value: undefined, label: "contact.all" },
  ];

  return (
    <main className="admin-page contact-page" id="contact-main">
      <div className="page-head">
        <div>
          <h1>{t(lang, "contact.title")}</h1>
          {listing !== null && (
            <p className="muted small">{t(lang, "contact.summary", { open: formatCount(lang, listing.open) })}</p>
          )}
        </div>
        <button type="button" onClick={load}>
          {t(lang, "admin.refresh")}
        </button>
      </div>
      <fieldset className="segmented contact-filter">
        <legend className="sr-only">{t(lang, "contact.filter")}</legend>
        {filters.map((f) => (
          <button
            key={f.label}
            type="button"
            aria-pressed={status === f.value}
            onClick={() => {
              navigate(contactPath(undefined, f.value), true);
            }}
          >
            {t(lang, f.label)}
          </button>
        ))}
      </fieldset>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {listing === null && error === null ? (
        <p className="muted">{t(lang, "admin.loading")}</p>
      ) : listing === null ? null : listing.messages.length === 0 ? (
        <p className="empty">{t(lang, "contact.none")}</p>
      ) : (
        <>
          {listing.total > listing.messages.length && (
            <p className="muted small">
              {t(lang, "contact.shown", {
                n: formatCount(lang, listing.messages.length),
                total: formatCount(lang, listing.total),
              })}
            </p>
          )}
          <ul className="contact-list">
            {listing.messages.map((m) => (
              <li key={m.id} className={`contact-item status-${m.status}`} data-testid="contact">
                <a href={contactPath(m.id)} onClick={go(contactPath(m.id))} className="contact-link">
                  <span className="contact-head">
                    <strong className="contact-sender">{sender(lang, m)}</strong>
                    <StatusBadge lang={lang} status={m.status} />
                    <span className="badge">{m.language.toUpperCase()}</span>
                    <span className="muted small contact-when" title={formatWhen(lang, m.created_at)}>
                      {formatAgo(lang, m.created_at, now)}
                    </span>
                  </span>
                  <span className="contact-excerpt">{m.message}</span>
                </a>
              </li>
            ))}
          </ul>
        </>
      )}
      <p className="muted small">{t(lang, "contact.kept")}</p>
    </main>
  );
}

export function ContactPage({ lang, id }: { lang: Lang; id: string }) {
  const [message, setMessage] = useState<ContactDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const now = useNow();

  useEffect(() => {
    let live = true;
    api.contact
      .one(id)
      .then((m) => {
        if (live) setMessage(m);
      })
      .catch((e: unknown) => {
        if (live && !(e instanceof SignedOut)) setError(describe(lang, e));
      });
    return () => {
      live = false;
    };
  }, [id, lang]);

  const act = async (what: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await what();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
    } finally {
      setBusy(false);
    }
  };

  const mark = (status: ContactStatus) =>
    act(async () => {
      setMessage(await api.contact.mark(id, status));
    });

  const remove = () =>
    act(async () => {
      await api.contact.remove(id);
      navigate(contactPath(), true);
    });

  const mailto =
    message !== null && message.email !== null
      ? `mailto:${encodeURIComponent(message.email).replace(/%40/g, "@")}?subject=${encodeURIComponent(
          t(message.language, "contact.replySubject"),
        )}`
      : null;

  return (
    <main className="admin-page contact-page" id="contact-main">
      <p className="contact-back">
        <Link to={contactPath()}>← {t(lang, "contact.back")}</Link>
      </p>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {message === null ? (
        error === null && <p className="muted">{t(lang, "admin.loading")}</p>
      ) : (
        <article className="card contact-detail" aria-labelledby="contact-sender">
          <div className="page-head">
            <div>
              <h1 id="contact-sender">{sender(lang, message)}</h1>
              <p className="muted small">
                {message.email ?? t(lang, "contact.noEmail")}
              </p>
            </div>
            <StatusBadge lang={lang} status={message.status} />
          </div>
          <dl className="kv contact-facts">
            <div className="kv-row">
              <dt>{t(lang, "contact.received")}</dt>
              <dd>
                {formatWhen(lang, message.created_at)}{" "}
                <span className="muted small">({formatAgo(lang, message.created_at, now)})</span>
              </dd>
            </div>
            <div className="kv-row">
              <dt>{t(lang, "contact.language")}</dt>
              <dd>{message.language === "de" ? "Deutsch" : "English"}</dd>
            </div>
            {message.done_at !== null && (
              <div className="kv-row">
                <dt>{t(lang, "contact.doneAt")}</dt>
                <dd>{formatWhen(lang, message.done_at)}</dd>
              </div>
            )}
          </dl>
          <p className="contact-text" lang={message.language}>
            {message.message}
          </p>
          <div className="row-actions">
            {mailto !== null && (
              <a className="button primary" href={mailto}>
                {t(lang, "contact.reply")}
              </a>
            )}
            {message.status === "new" ? (
              <button type="button" disabled={busy} onClick={() => void mark("done")}>
                {t(lang, "contact.markDone")}
              </button>
            ) : (
              <button type="button" disabled={busy} onClick={() => void mark("new")}>
                {t(lang, "contact.reopen")}
              </button>
            )}
            {!confirm && (
              <button
                type="button"
                className="danger-quiet"
                disabled={busy}
                onClick={() => {
                  setConfirm(true);
                }}
              >
                {t(lang, "contact.delete")}
              </button>
            )}
          </div>
          {confirm && (
            <fieldset className="confirm">
              <legend>{t(lang, "contact.deleteConfirm")}</legend>
              <div className="row-actions">
                <button type="button" className="danger" disabled={busy} onClick={() => void remove()}>
                  {t(lang, "contact.deleteYes")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    setConfirm(false);
                  }}
                >
                  {t(lang, "contact.deleteNo")}
                </button>
              </div>
            </fieldset>
          )}
          <h2 className="contact-history-title">{t(lang, "contact.history")}</h2>
          {message.audit.length === 0 ? (
            <p className="muted small">{t(lang, "contact.noHistory")}</p>
          ) : (
            <ol className="audit">
              {message.audit.map((a) => (
                <li key={`${a.at}-${a.action}-${a.detail ?? ""}`}>
                  <span className="muted" title={formatWhen(lang, a.at)}>
                    {formatAgo(lang, a.at, now)}
                  </span>{" "}
                  <strong>{a.actor}</strong> <span className="badge">{a.action}</span>
                  {a.detail !== null && <span className="muted small"> {a.detail}</span>}
                </li>
              ))}
            </ol>
          )}
        </article>
      )}
    </main>
  );
}
