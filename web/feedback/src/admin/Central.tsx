// The gateway's link to Baylee Central (`/admin/central`, docs/protocol.md
// §"Linking the gateway to Baylee Central"): where it comes from, and for a
// token pasted here what Central says about it (the gateway's origin and
// name, whether an administrator approved it, what the token allows and
// until when). An admin pastes a token made on baylee.app under "Your
// gateways"; the gateway checks it with Central, keeps it and uses it at
// once. The page never shows more of a token than its last four characters.

import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";

import { api, SignedOut, type CentralLink } from "../api";
import { formatWhen, t, type Key, type Lang } from "../i18n";
import { Badge, describe, Facts, Panel, type Tone } from "./shared";

const STATE: Record<NonNullable<CentralLink["central"]>["state"], { word: Key; tone: Tone }> = {
  pending: { word: "central.pending", tone: "warn" },
  approved: { word: "central.approved", tone: "ok" },
  disabled: { word: "central.disabled", tone: "danger" },
};

/** A token as the website shows it: `bgt_` and 64 lowercase hex digits. */
export const TOKEN = /^bgt_[0-9a-f]{64}$/;

function Status({ lang, link }: { lang: Lang; link: CentralLink }) {
  if (link.source === "file") return <p>{t(lang, "central.fromFile")}</p>;
  if (link.source === "none") return <p>{t(lang, "central.none")}</p>;
  const central = link.central;
  return (
    <>
      {link.check === "rejected" && <p className="error">{t(lang, "central.rejected")}</p>}
      {link.check === "unavailable" && <p className="muted">{t(lang, "central.unavailable")}</p>}
      <Facts
        rows={[
          [t(lang, "central.token"), <code key="t">bgt_…{link.token?.last4}</code>],
          [t(lang, "central.setBy"), `${link.token?.set_by ?? ""} · ${link.token === null ? "" : formatWhen(lang, link.token.set_at)}`],
          ...(central === null
            ? []
            : ([
                [t(lang, "central.gateway"), <span key="g">{central.name} · <code>{central.origin}</code></span>],
                [t(lang, "central.state"), <Badge key="s" tone={STATE[central.state].tone} dot>{t(lang, STATE[central.state].word)}</Badge>],
                [
                  t(lang, "central.allows"),
                  [central.sso ? t(lang, "central.sso") : null, central.listing ? t(lang, "central.listing") : null].filter((x) => x !== null).join(" · "),
                ],
                [t(lang, "central.expires"), formatWhen(lang, central.expires_at)],
              ] as [string, ReactNode][])),
        ]}
      />
      {central?.state === "pending" && <p className="muted small">{t(lang, "central.pendingHint")}</p>}
    </>
  );
}

export function Central({ lang }: { lang: Lang }) {
  const [link, setLink] = useState<CentralLink | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ bad: boolean; text: string } | null>(null);
  const read = useCallback(() => {
    api.admin.central.link().then(setLink, (e: unknown) => {
      if (!(e instanceof SignedOut)) setFailed(describe(lang, e));
    });
  }, [lang]);
  useEffect(read, [read]);

  const paste = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const text = token.trim();
    if (!TOKEN.test(text)) {
      setSaid({ bad: true, text: t(lang, "central.notToken") });
      return;
    }
    setBusy(true);
    setSaid(null);
    try {
      setLink(await api.admin.central.paste(text));
      setToken("");
      setSaid({ bad: false, text: t(lang, "central.linked") });
    } catch (error: unknown) {
      if (!(error instanceof SignedOut)) setSaid({ bad: true, text: describe(lang, error) });
    } finally {
      setBusy(false);
    }
  };

  const [confirm, setConfirm] = useState(false);
  const unlink = async () => {
    setBusy(true);
    try {
      await api.admin.central.remove();
      setConfirm(false);
      setSaid({ bad: false, text: t(lang, "central.unlinked") });
      read();
    } catch (error: unknown) {
      if (!(error instanceof SignedOut)) setSaid({ bad: true, text: describe(lang, error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Panel id="central-link" title={t(lang, "central.title")}>
        <p className="muted">{t(lang, "central.lead")}</p>
        {failed !== null && <p className="error">{failed}</p>}
        {link === null && failed === null && <p className="muted">{t(lang, "admin.loading")}</p>}
        {link !== null && <Status lang={lang} link={link} />}
        {link?.source === "token" &&
          (confirm ? (
            <div className="row-actions">
              <span>{t(lang, "central.unlinkConfirm")}</span>
              <button type="button" className="danger" onClick={() => void unlink()} disabled={busy}>
                {t(lang, "central.unlink")}
              </button>
              <button type="button" onClick={() => setConfirm(false)}>
                {t(lang, "central.keep")}
              </button>
            </div>
          ) : (
            <button type="button" className="danger" onClick={() => setConfirm(true)}>
              {t(lang, "central.unlink")}
            </button>
          ))}
      </Panel>
      {link !== null && link.source !== "file" && (
        <Panel id="central-paste" title={t(lang, link.source === "token" ? "central.replace" : "central.paste")}>
          <ol>
            <li>{t(lang, "central.step1")}</li>
            <li>{t(lang, "central.step2")}</li>
            <li>{t(lang, "central.step3")}</li>
          </ol>
          <form onSubmit={(e) => void paste(e)} className="card">
            <label className="field">
              <span>{t(lang, "central.token")}</span>
              <input
                className="mono"
                type="password"
                autoComplete="off"
                spellCheck={false}
                value={token}
                placeholder="bgt_…"
                onChange={(e) => setToken(e.target.value)}
              />
            </label>
            <button type="submit" className="primary" disabled={busy || token.trim() === ""}>
              {t(lang, "central.link")}
            </button>
          </form>
        </Panel>
      )}
      <output className={said?.bad === true ? "error" : "muted"}>{said?.text ?? ""}</output>
    </>
  );
}
