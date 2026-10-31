// The admin console (`/admin/…`, docs/feedback.md §"The admin console"):
// a gateway's overview, its live tables and server, its accounts, the
// pool's progress by set, its blocked cards, its closed-beta keys and its
// link to Baylee Central,
// behind one section bar that sits on top on a wide screen and at the
// bottom of a phone, under the thumb.

import { useEffect } from "react";

import { AccountDetail } from "./admin/AccountDetail";
import { Accounts } from "./admin/Accounts";
import { Cards } from "./admin/Cards";
import { Central } from "./admin/Central";
import { Keys } from "./admin/Keys";
import { Live } from "./admin/Live";
import { Models } from "./admin/Models";
import { Overview } from "./admin/Overview";
import { SetPage, Sets } from "./admin/Sets";
import { go } from "./admin/shared";
import { BRAND } from "./Brand";
import { t, type Key, type Lang } from "./i18n";
import { adminPath, ADMIN_SECTIONS, type AdminSection } from "./router";

export { REFRESH_MS, useVisibleInterval } from "./admin/shared";

const ICONS: Record<AdminSection, string> = {
  overview: "M3 13h8V3H3zm0 8h8v-6H3zm10 0h8V11h-8zm0-18v6h8V3z",
  live: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm-8.5 4a8.5 8.5 0 0 1 2.5-6l1.4 1.4a6.5 6.5 0 0 0 0 9.2L6 18a8.5 8.5 0 0 1-2.5-6zm17 0a8.5 8.5 0 0 1-2.5 6l-1.4-1.4a6.5 6.5 0 0 0 0-9.2L18 6a8.5 8.5 0 0 1 2.5 6z",
  accounts:
    "M16 11a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm-8 0a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm0 2c-2.3 0-7 1.2-7 3.5V19h14v-2.5C15 14.2 10.3 13 8 13zm8 0c-.3 0-.6 0-1 .1a4.2 4.2 0 0 1 2 3.4V19h6v-2.5c0-2.3-4.7-3.5-7-3.5z",
  // Layers, not squares: on a phone the icons stand without their words.
  sets: "M12 3 2 8l10 5 10-5zm-7.6 8.2L2 12.4l10 5 10-5-2.4-1.2L12 15zm0 4.6L2 17l10 5 10-5-2.4-1.2L12 19.6z",
  cards:
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 2a8 8 0 0 1 6.3 12.9L7.1 5.7A8 8 0 0 1 12 4zM4 12a8 8 0 0 1 1.7-4.9l11.2 11.2A8 8 0 0 1 4 12z",
  models: "M9 2v2H7a3 3 0 0 0-3 3v2H2v2h2v2H2v2h2v2a3 3 0 0 0 3 3h2v2h2v-2h2v2h2v-2h2a3 3 0 0 0 3-3v-2h2v-2h-2v-2h2V9h-2V7a3 3 0 0 0-3-3h-2V2h-2v2h-2V2zm0 6h6v8H9z",
  keys: "M7 14a2 2 0 1 1 2-2 2 2 0 0 1-2 2zm5.6-4A6 6 0 1 0 12.6 14H17v4h4v-4h2v-4z",
  // A link of two rings: the gateway joined to Baylee Central.
  central: "M10.6 13.4a1 1 0 0 1 0-1.4l3.5-3.5a3 3 0 1 1 4.2 4.2l-1.8 1.8-1.4-1.4 1.8-1.8a1 1 0 0 0-1.4-1.4L12 13.4a1 1 0 0 1-1.4 0zm2.8-2.8a1 1 0 0 1 0 1.4l-3.5 3.5a3 3 0 1 1-4.2-4.2l1.8-1.8 1.4 1.4-1.8 1.8a1 1 0 0 0 1.4 1.4L12 10.6a1 1 0 0 1 1.4 0z",
};

const LABELS: Record<AdminSection, Key> = {
  overview: "sec.overview",
  live: "sec.live",
  accounts: "sec.accounts",
  sets: "sec.sets",
  cards: "sec.cards",
  keys: "sec.keys",
  models: "sec.models",
  central: "sec.central",
};

function SectionBar({ lang, section }: { lang: Lang; section: AdminSection }) {
  return (
    <nav className="sections" aria-label={t(lang, "sec.label")}>
      {ADMIN_SECTIONS.map((s) => (
        <a
          key={s}
          href={adminPath(s)}
          aria-current={s === section ? "page" : undefined}
          onClick={go(adminPath(s))}
        >
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" focusable="false">
            <path d={ICONS[s]} fill="currentColor" />
          </svg>
          <span>{t(lang, LABELS[s])}</span>
        </a>
      ))}
    </nav>
  );
}

export function Admin({
  lang,
  section = "overview",
  id,
  search = "",
}: {
  lang: Lang;
  section?: AdminSection;
  id?: string | undefined;
  search?: string | undefined;
}) {
  // The window's title names the section (WCAG 2.4.2), and one set its code.
  useEffect(() => {
    const name = t(lang, LABELS[section]);
    const set = section === "sets" && id !== undefined ? `${id.toUpperCase()} · ` : "";
    document.title = `${set}${name} · ${BRAND}`;
  }, [lang, section, id]);
  return (
    <div className="admin-shell">
      <SectionBar lang={lang} section={section} />
      <main className="admin-page" id="admin-main">
        {section === "overview" && <Overview lang={lang} />}
        {section === "live" && <Live lang={lang} />}
        {section === "accounts" &&
          (id === undefined ? <Accounts lang={lang} search={search} /> : <AccountDetail key={id} lang={lang} id={id} />)}
        {section === "sets" && (id === undefined ? <Sets lang={lang} /> : <SetPage key={id} lang={lang} code={id} />)}
        {section === "cards" && <Cards lang={lang} />}
        {section === "keys" && <Keys lang={lang} />}
        {section === "models" && <Models lang={lang} />}
        {section === "central" && <Central lang={lang} />}
      </main>
    </div>
  );
}
