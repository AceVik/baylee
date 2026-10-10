// The admin console (`/admin/…`, docs/feedback.md §"The admin console"):
// a gateway's overview, its live tables, its accounts and its closed-beta
// keys, behind one section bar that sits on top on a wide screen and at
// the bottom of a phone, under the thumb.

import { AccountDetail } from "./admin/AccountDetail";
import { Accounts } from "./admin/Accounts";
import { Keys } from "./admin/Keys";
import { Live } from "./admin/Live";
import { Overview } from "./admin/Overview";
import { go } from "./admin/shared";
import { t, type Key, type Lang } from "./i18n";
import { adminPath, ADMIN_SECTIONS, type AdminSection } from "./router";

export { REFRESH_MS, useVisibleInterval } from "./admin/shared";

const ICONS: Record<AdminSection, string> = {
  overview: "M3 13h8V3H3zm0 8h8v-6H3zm10 0h8V11h-8zm0-18v6h8V3z",
  live: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm-8.5 4a8.5 8.5 0 0 1 2.5-6l1.4 1.4a6.5 6.5 0 0 0 0 9.2L6 18a8.5 8.5 0 0 1-2.5-6zm17 0a8.5 8.5 0 0 1-2.5 6l-1.4-1.4a6.5 6.5 0 0 0 0-9.2L18 6a8.5 8.5 0 0 1 2.5 6z",
  accounts:
    "M16 11a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm-8 0a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm0 2c-2.3 0-7 1.2-7 3.5V19h14v-2.5C15 14.2 10.3 13 8 13zm8 0c-.3 0-.6 0-1 .1a4.2 4.2 0 0 1 2 3.4V19h6v-2.5c0-2.3-4.7-3.5-7-3.5z",
  keys: "M7 14a2 2 0 1 1 2-2 2 2 0 0 1-2 2zm5.6-4A6 6 0 1 0 12.6 14H17v4h4v-4h2v-4z",
};

const LABELS: Record<AdminSection, Key> = {
  overview: "sec.overview",
  live: "sec.live",
  accounts: "sec.accounts",
  keys: "sec.keys",
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
  return (
    <div className="admin-shell">
      <SectionBar lang={lang} section={section} />
      <main className="admin-page" id="admin-main">
        {section === "overview" && <Overview lang={lang} />}
        {section === "live" && <Live lang={lang} />}
        {section === "accounts" &&
          (id === undefined ? <Accounts lang={lang} search={search} /> : <AccountDetail key={id} lang={lang} id={id} />)}
        {section === "keys" && <Keys lang={lang} />}
      </main>
    </div>
  );
}
