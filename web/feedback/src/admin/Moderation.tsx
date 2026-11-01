// Acting on an account or a game (beta.8, `docs/feedback.md` §"The admin
// console"): ban and lift a ban, delete an account (typed to confirm), and
// end a running game. Every act asks first, in a dialog, and goes through the
// service, which logs it with the admin. There is no password to reset:
// accounts sign in at Baylee Central (`docs/central-login.md`).

import { useEffect, useRef, useState, type FormEvent } from "react";

import { request, type AccountBan } from "../api";
import { formatWhen, t, type Lang } from "../i18n";
import { navigate, adminPath } from "../router";
import { Modal } from "./Models";
import { describe } from "./shared";

/** What the acts need to know about the account. */
export interface Target {
  id: string;
  handle: string;
  guest: boolean;
  ban?: AccountBan | null;
}

const accountPath = (id: string) => `/ui/api/admin/accounts/${encodeURIComponent(id)}`;

/** The service's acts, each one request. */
export const moderation = {
  ban: async (id: string, reason: string, until: string | null): Promise<void> => {
    await request("PUT", `${accountPath(id)}/ban`, until === null ? { reason } : { reason, until });
  },
  unban: async (id: string): Promise<void> => {
    await request("DELETE", `${accountPath(id)}/ban`);
  },
  remove: async (id: string): Promise<void> => {
    await request("DELETE", accountPath(id));
  },
  closeGame: async (id: string, reason: string): Promise<void> => {
    await request("POST", `/ui/api/admin/games/${encodeURIComponent(id)}/close`, { reason });
  },
};

/** A `datetime-local` value (the browser's own zone) as an RFC 3339 moment in UTC, or null. */
export function untilOf(local: string): string | null {
  if (local === "") return null;
  const at = new Date(local);
  return Number.isNaN(at.getTime()) ? null : at.toISOString().replace(/\.\d{3}Z$/, "Z");
}

/** The line that says an account is banned, and by whom. */
export function BanLine({ lang, ban }: { lang: Lang; ban: AccountBan }) {
  return (
    <p className="notice tone-danger">
      {ban.until === null
        ? t(lang, "mod.bannedForever", { by: ban.by, reason: ban.reason })
        : t(lang, "mod.bannedUntil", { when: formatWhen(lang, ban.until), by: ban.by, reason: ban.reason })}
    </p>
  );
}

/** Asks one yes-or-no question before an act; says the refusal if it fails. */
function Ask({
  lang,
  id,
  question,
  yes,
  run,
  onClose,
}: {
  lang: Lang;
  id: string;
  question: string;
  yes: string;
  run: () => Promise<void>;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const yesRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    yesRef.current?.focus();
  }, []);
  return (
    <Modal labelledBy={id} alert onClose={onClose}>
      <h3 id={id}>{question}</h3>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="row-actions">
        <button
          ref={yesRef}
          type="button"
          className="danger"
          disabled={busy}
          onClick={() => {
            setBusy(true);
            run()
              .then(onClose)
              .catch((e: unknown) => {
                setBusy(false);
                setError(describe(lang, e));
              });
          }}
        >
          {yes}
        </button>
        <button type="button" disabled={busy} onClick={onClose}>
          {t(lang, "mod.cancel")}
        </button>
      </div>
    </Modal>
  );
}

/** The ban's form: a reason, and for good or until a moment. */
function BanForm({ lang, target, onClose, onDone }: { lang: Lang; target: Target; onClose: () => void; onDone: () => void }) {
  const [reason, setReason] = useState("");
  const [forever, setForever] = useState(false);
  const [until, setUntil] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (reason.trim() === "") {
      setError(t(lang, "mod.banNeedsReason"));
      return;
    }
    const end = forever ? null : untilOf(until);
    if (!forever && (end === null || new Date(end).getTime() <= Date.now())) {
      setError(t(lang, "mod.banNeedsEnd"));
      return;
    }
    setBusy(true);
    moderation
      .ban(target.id, reason.trim(), end)
      .then(() => {
        onDone();
        onClose();
      })
      .catch((e: unknown) => {
        setBusy(false);
        setError(describe(lang, e));
      });
  };
  return (
    <Modal labelledBy="mod-ban" alert onClose={onClose}>
      <form onSubmit={submit}>
        <h3 id="mod-ban">{t(lang, "mod.banTitle", { who: target.handle })}</h3>
        <p className="muted small">{t(lang, "mod.banWhat")}</p>
        <label className="field">
          <span>{t(lang, "mod.banReason")}</span>
          <input value={reason} maxLength={500} required onChange={(e) => setReason(e.target.value)} />
        </label>
        <fieldset className="field">
          <label>
            <input type="radio" name="mod-end" checked={!forever} onChange={() => setForever(false)} />{" "}
            {t(lang, "mod.banUntil")}
          </label>{" "}
          <input
            type="datetime-local"
            aria-label={t(lang, "mod.banUntil")}
            value={until}
            disabled={forever}
            onChange={(e) => setUntil(e.target.value)}
          />
          <label>
            <input type="radio" name="mod-end" checked={forever} onChange={() => setForever(true)} />{" "}
            {t(lang, "mod.banForever")}
          </label>
        </fieldset>
        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="row-actions">
          <button type="submit" className="danger" disabled={busy}>
            {t(lang, "mod.banYes")}
          </button>
          <button type="button" disabled={busy} onClick={onClose}>
            {t(lang, "mod.cancel")}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Deleting: the handle typed out before the button wakes. */
function DeleteForm({ lang, target, onClose }: { lang: Lang; target: Target; onClose: () => void }) {
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const matches = typed.trim() === target.handle;
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!matches) return;
    setBusy(true);
    moderation
      .remove(target.id)
      .then(() => {
        onClose();
        navigate(adminPath("accounts"));
      })
      .catch((e: unknown) => {
        setBusy(false);
        setError(describe(lang, e));
      });
  };
  return (
    <Modal labelledBy="mod-delete" alert onClose={onClose}>
      <form onSubmit={submit}>
        <h3 id="mod-delete">{t(lang, "mod.deleteTitle", { who: target.handle })}</h3>
        <p className="muted small">{t(lang, "mod.deleteWhat")}</p>
        <label className="field">
          <span>{t(lang, "mod.deleteType", { handle: target.handle })}</span>
          <input value={typed} autoComplete="off" onChange={(e) => setTyped(e.target.value)} />
        </label>
        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="row-actions">
          <button type="submit" className="danger" disabled={busy || !matches}>
            {t(lang, "mod.deleteYes")}
          </button>
          <button type="button" disabled={busy} onClick={onClose}>
            {t(lang, "mod.cancel")}
          </button>
        </div>
      </form>
    </Modal>
  );
}

type Open = "ban" | "unban" | "delete" | null;

/** The acts on one account, under its facts. `onChanged` reads it again. */
export function AccountActions({ lang, target, onChanged }: { lang: Lang; target: Target; onChanged: () => void }) {
  const [open, setOpen] = useState<Open>(null);
  const close = () => setOpen(null);
  return (
    <div className="mod-actions">
      <p className="muted small">{t(lang, "mod.hint")}</p>
      {target.ban != null && <BanLine lang={lang} ban={target.ban} />}
      <div className="row-actions">
        {target.ban != null ? (
          <button type="button" onClick={() => setOpen("unban")}>
            {t(lang, "mod.unban")}
          </button>
        ) : (
          <button type="button" className="danger-quiet" onClick={() => setOpen("ban")}>
            {t(lang, "mod.ban")}
          </button>
        )}
        <button type="button" className="danger" onClick={() => setOpen("delete")}>
          {t(lang, "mod.delete")}
        </button>
      </div>
      {open === "ban" && <BanForm lang={lang} target={target} onClose={close} onDone={onChanged} />}
      {open === "unban" && (
        <Ask
          lang={lang}
          id="mod-unban"
          question={t(lang, "mod.unbanAsk", { who: target.handle })}
          yes={t(lang, "mod.unban")}
          run={async () => {
            await moderation.unban(target.id);
            onChanged();
          }}
          onClose={close}
        />
      )}
      {open === "delete" && <DeleteForm lang={lang} target={target} onClose={close} />}
    </div>
  );
}

/** A running game's End game button, and the dialog it opens. */
export function EndGame({ lang, gameId, onDone }: { lang: Lang; gameId: string; onDone?: () => void }) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState(() => t(lang, "mod.endDefault"));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [asked, setAsked] = useState(false);
  const close = () => {
    setOpen(false);
    setBusy(false);
    setError(null);
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (reason.trim() === "") return;
    setBusy(true);
    moderation
      .closeGame(gameId, reason.trim())
      .then(() => {
        setAsked(true);
        close();
        onDone?.();
      })
      .catch((e: unknown) => {
        setBusy(false);
        setError(describe(lang, e));
      });
  };
  return (
    <>
      {asked ? (
        <span className="muted small">{t(lang, "mod.endAsked")}</span>
      ) : (
        <button type="button" className="danger-quiet" onClick={() => setOpen(true)}>
          {t(lang, "mod.endGame")}
        </button>
      )}
      {open && (
        <Modal labelledBy={`mod-end-${gameId}`} alert onClose={close}>
          <form onSubmit={submit}>
            <h3 id={`mod-end-${gameId}`}>{t(lang, "mod.endTitle")}</h3>
            <p className="muted small">{t(lang, "mod.endWhat")}</p>
            <label className="field">
              <span>{t(lang, "mod.endReason")}</span>
              <input value={reason} maxLength={200} required onChange={(e) => setReason(e.target.value)} />
            </label>
            {error !== null && (
              <p className="error" role="alert">
                {error}
              </p>
            )}
            <div className="row-actions">
              <button type="submit" className="danger" disabled={busy || reason.trim() === ""}>
                {t(lang, "mod.endYes")}
              </button>
              <button type="button" disabled={busy} onClick={close}>
                {t(lang, "mod.cancel")}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}
