// Who the gateway lets in (`/admin/access`, docs/central-login.md §"Who a
// gateway lets in"): guests, Baylee Central's accounts, or both. An admin
// chooses here and the gateway keeps it; the operator's BAYLEE_AUTH_MODE
// wins, and then the choice is shown and locked. A change applies to the
// sign-ins from then on; players signed in already stay until they leave.
// A gateway that is not connected to Central and takes Central's accounts
// only lets nobody in, which `NobodyComesIn` says on this page and on the
// overview, with the two ways out.

import { useCallback, useEffect, useState, type FormEvent } from "react";

import { api, SignedOut, type AuthModeName, type AuthModeState } from "../api";
import { formatWhen, t, type Key, type Lang } from "../i18n";
import { adminPath } from "../router";
import { describe, Link, Panel } from "./shared";

/** Each mode's name, as the console says it. */
export const AUTH_MODE_WORDS: Record<AuthModeName, Key> = {
  guests: "entry.guests",
  central: "entry.central",
  mixed: "entry.mixed",
};

const MODES: { mode: AuthModeName; label: Key; hint: Key }[] = [
  { mode: "central", label: "entry.central", hint: "entry.centralHint" },
  { mode: "mixed", label: "entry.mixed", hint: "entry.mixedHint" },
  { mode: "guests", label: "entry.guests", hint: "entry.guestsHint" },
];

/** Whether a gateway in `mode`, connected to Central or not, lets anybody in. */
export function nobodyComesIn(mode: AuthModeName | undefined, connected: boolean | undefined): boolean {
  return mode === "central" && connected === false;
}

/**
 * The warning for a gateway nobody can enter, with its two ways out: a
 * Central token (the console's Baylee Central page) and guests (this page).
 */
export function NobodyComesIn({ lang, here = false }: { lang: Lang; here?: boolean }) {
  return (
    <div className="empty warn-text" role="alert">
      <p>{t(lang, "entry.nobody")}</p>
      <div className="row-actions">
        <Link to="/admin/central" className="button small-button">
          {t(lang, "entry.pasteToken")}
        </Link>
        {!here && (
          <Link to={adminPath("access")} className="button small-button">
            {t(lang, "entry.allowGuests")}
          </Link>
        )}
      </div>
    </div>
  );
}

export function Access({ lang }: { lang: Lang }) {
  const [state, setState] = useState<AuthModeState | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [mode, setMode] = useState<AuthModeName | null>(null);
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ bad: boolean; text: string } | null>(null);
  const read = useCallback(() => {
    api.admin.authMode.get().then(
      (s) => {
        setState(s);
        setMode(s.mode);
      },
      (e: unknown) => {
        if (!(e instanceof SignedOut)) setFailed(describe(lang, e));
      },
    );
  }, [lang]);
  useEffect(read, [read]);

  const locked = state?.source === "environment";
  const choose = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (mode === null || locked) return;
    setBusy(true);
    setSaid(null);
    try {
      const chosen = await api.admin.authMode.choose(mode);
      setState(chosen);
      setMode(chosen.mode);
      setSaid({ bad: false, text: t(lang, "entry.saved") });
    } catch (error: unknown) {
      if (!(error instanceof SignedOut)) setSaid({ bad: true, text: describe(lang, error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Panel id="access-mode" title={t(lang, "entry.title")}>
        <p className="muted">{t(lang, "entry.lead")}</p>
        {failed !== null && <p className="error">{failed}</p>}
        {state === null && failed === null && <p className="muted">{t(lang, "admin.loading")}</p>}
        {state !== null && nobodyComesIn(state.mode, state.connected) && <NobodyComesIn lang={lang} here />}
        {state !== null && (
          <form onSubmit={(e) => void choose(e)} className="card">
            <fieldset className="field" disabled={locked || busy}>
              <legend>{t(lang, "entry.legend")}</legend>
              {MODES.map((option) => (
                <label key={option.mode}>
                  <input
                    type="radio"
                    name="auth-mode"
                    value={option.mode}
                    checked={mode === option.mode}
                    onChange={() => setMode(option.mode)}
                  />
                  <span>
                    <strong>{t(lang, option.label)}</strong>
                    <span className="muted small"> {t(lang, option.hint)}</span>
                  </span>
                </label>
              ))}
            </fieldset>
            <p className="muted small">
              {state.source === "environment" && t(lang, "entry.fromEnvironment")}
              {state.source === "default" && t(lang, "entry.fromDefault")}
              {state.source === "console" &&
                t(lang, "entry.fromConsole", {
                  who: state.set_by ?? "",
                  when: state.set_at === null ? "" : formatWhen(lang, state.set_at),
                })}
            </p>
            {!locked && (
              <>
                <p className="muted small">{t(lang, "entry.applies")}</p>
                <button type="submit" className="primary" disabled={busy || mode === state.mode}>
                  {t(lang, "entry.save")}
                </button>
              </>
            )}
          </form>
        )}
      </Panel>
      <output className={said?.bad === true ? "error" : "muted"}>{said?.text ?? ""}</output>
    </>
  );
}
