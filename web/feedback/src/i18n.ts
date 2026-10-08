// The admin console's words in English and German. The rest of the UI
// (report triage) is English only, as it was; this area is read by the
// owner on a phone too, so it speaks the browser's language when that is
// German. No library: two tables and a lookup.

export type Lang = "en" | "de";

const en = {
  "nav.reports": "Reports",
  "nav.admin": "Overview",
  "nav.label": "Sections",
  "nav.signOut": "Sign out",
  "admin.title": "Overview",
  "admin.updated": "Updated {time}",
  "admin.paused": "Paused while the page is hidden",
  "admin.refresh": "Refresh",
  "admin.loading": "Loading…",
  "admin.noAnswer": "The service did not answer.",
  "stats.players": "Players",
  "stats.registered": "Registered accounts",
  "stats.confirmed": "with a confirmed address",
  "stats.online": "Online now",
  "stats.inLobby": "in the lobby",
  "stats.seated": "at a table",
  "stats.sessions": "Live sessions",
  "stats.signedIn": "accounts signed in",
  "stats.guests": "Guests",
  "stats.guestCap": "of {cap}",
  "stats.guestsOff": "guests off",
  "stats.noCap": "no cap",
  "stats.newAccounts": "New accounts",
  "stats.games": "Games",
  "stats.running": "Running now",
  "stats.local": "on this machine",
  "stats.waiting": "Tables waiting",
  "stats.started": "Games started",
  "stats.finished": "Games finished",
  "stats.total": "in total",
  "stats.today": "today (UTC)",
  "stats.week": "7 days",
  "stats.month": "30 days",
  "stats.server": "Server",
  "stats.agents": "Agents",
  "stats.capacity": "Capacity",
  "stats.unlimited": "unlimited",
  "stats.agentGames": "games on agents",
  "stats.version": "Build",
  "stats.registration": "Registration",
  "stats.keys": "Beta keys",
  "stats.activeKeys": "Active keys",
  "stats.usesLeft": "uses left",
  "stats.admitted": "Accounts admitted",
  "stats.usedUp": "used up",
  "stats.expired": "expired",
  "stats.revoked": "revoked",
  "invites.title": "Closed beta keys",
  "invites.create": "Make keys",
  "invites.count": "How many",
  "invites.uses": "Accounts per key",
  "invites.expires": "Expires",
  "invites.never": "Never",
  "invites.hours12": "In 12 hours",
  "invites.day1": "In 1 day",
  "invites.days7": "In 7 days",
  "invites.days30": "In 30 days",
  "invites.days90": "In 90 days",
  "invites.note": "Note (who it is for)",
  "invites.notePlaceholder": "e.g. Max",
  "invites.making": "Making…",
  "invites.made": "{n} new key(s): shown only now",
  "invites.madeHint": "Only a hash of each key is kept. Copy them before you leave this page.",
  "invites.copy": "Copy",
  "invites.copyAll": "Copy all",
  "invites.copied": "Copied",
  "invites.share": "Share",
  "invites.done": "Done",
  "invites.list": "All keys",
  "invites.none": "No keys yet.",
  "invites.created": "Made",
  "invites.state": "State",
  "invites.usesLeft": "Uses left",
  "invites.admitted": "Admitted",
  "invites.expiresAt": "Expires",
  "invites.noteColumn": "Note",
  "invites.revoke": "Revoke…",
  "invites.revokeConfirm": "Revoke this key? Nobody can use it after this.",
  "invites.revokeYes": "Revoke for good",
  "invites.revokeNo": "Keep it",
  "invites.revoked": "Key revoked.",
  "invites.neverExpires": "never",
  "invites.noKeyShown": "Keys are not listed: only their hashes are kept.",
  "state.active": "active",
  "state.used_up": "used up",
  "state.expired": "expired",
  "state.revoked": "revoked",
  "audit.title": "Recent changes",
  "audit.none": "Nothing changed here yet.",
} as const;

export type Key = keyof typeof en;

const de: Record<Key, string> = {
  "nav.reports": "Meldungen",
  "nav.admin": "Übersicht",
  "nav.label": "Bereiche",
  "nav.signOut": "Abmelden",
  "admin.title": "Übersicht",
  "admin.updated": "Stand {time}",
  "admin.paused": "Pausiert, solange die Seite verborgen ist",
  "admin.refresh": "Aktualisieren",
  "admin.loading": "Lädt…",
  "admin.noAnswer": "Der Dienst hat nicht geantwortet.",
  "stats.players": "Spieler",
  "stats.registered": "Registrierte Konten",
  "stats.confirmed": "mit bestätigter Adresse",
  "stats.online": "Jetzt online",
  "stats.inLobby": "in der Lobby",
  "stats.seated": "am Tisch",
  "stats.sessions": "Aktive Sitzungen",
  "stats.signedIn": "Konten angemeldet",
  "stats.guests": "Gäste",
  "stats.guestCap": "von {cap}",
  "stats.guestsOff": "Gäste aus",
  "stats.noCap": "ohne Grenze",
  "stats.newAccounts": "Neue Konten",
  "stats.games": "Spiele",
  "stats.running": "Laufen gerade",
  "stats.local": "auf diesem Rechner",
  "stats.waiting": "Wartende Tische",
  "stats.started": "Begonnene Spiele",
  "stats.finished": "Beendete Spiele",
  "stats.total": "insgesamt",
  "stats.today": "heute (UTC)",
  "stats.week": "7 Tage",
  "stats.month": "30 Tage",
  "stats.server": "Server",
  "stats.agents": "Agenten",
  "stats.capacity": "Kapazität",
  "stats.unlimited": "unbegrenzt",
  "stats.agentGames": "Spiele auf Agenten",
  "stats.version": "Build",
  "stats.registration": "Registrierung",
  "stats.keys": "Beta-Schlüssel",
  "stats.activeKeys": "Gültige Schlüssel",
  "stats.usesLeft": "Einlösungen übrig",
  "stats.admitted": "Eingelassene Konten",
  "stats.usedUp": "aufgebraucht",
  "stats.expired": "abgelaufen",
  "stats.revoked": "widerrufen",
  "invites.title": "Schlüssel für die geschlossene Beta",
  "invites.create": "Schlüssel erzeugen",
  "invites.count": "Anzahl",
  "invites.uses": "Konten je Schlüssel",
  "invites.expires": "Läuft ab",
  "invites.never": "Nie",
  "invites.hours12": "In 12 Stunden",
  "invites.day1": "In 1 Tag",
  "invites.days7": "In 7 Tagen",
  "invites.days30": "In 30 Tagen",
  "invites.days90": "In 90 Tagen",
  "invites.note": "Notiz (für wen)",
  "invites.notePlaceholder": "z. B. Max",
  "invites.making": "Erzeuge…",
  "invites.made": "{n} neue(r) Schlüssel: nur jetzt sichtbar",
  "invites.madeHint":
    "Gespeichert wird nur ein Hash. Kopiere die Schlüssel, bevor du die Seite verlässt.",
  "invites.copy": "Kopieren",
  "invites.copyAll": "Alle kopieren",
  "invites.copied": "Kopiert",
  "invites.share": "Teilen",
  "invites.done": "Fertig",
  "invites.list": "Alle Schlüssel",
  "invites.none": "Noch keine Schlüssel.",
  "invites.created": "Erzeugt",
  "invites.state": "Status",
  "invites.usesLeft": "Übrig",
  "invites.admitted": "Eingelassen",
  "invites.expiresAt": "Läuft ab",
  "invites.noteColumn": "Notiz",
  "invites.revoke": "Widerrufen…",
  "invites.revokeConfirm": "Diesen Schlüssel widerrufen? Danach kann ihn niemand mehr nutzen.",
  "invites.revokeYes": "Endgültig widerrufen",
  "invites.revokeNo": "Behalten",
  "invites.revoked": "Schlüssel widerrufen.",
  "invites.neverExpires": "nie",
  "invites.noKeyShown": "Schlüssel werden nicht aufgelistet: gespeichert ist nur ihr Hash.",
  "state.active": "gültig",
  "state.used_up": "aufgebraucht",
  "state.expired": "abgelaufen",
  "state.revoked": "widerrufen",
  "audit.title": "Letzte Änderungen",
  "audit.none": "Hier wurde noch nichts geändert.",
};

/** The browser's language, if it is one of ours; English otherwise. */
export function pickLang(languages: readonly string[]): Lang {
  for (const tag of languages) {
    const base = tag.toLowerCase().split("-")[0];
    if (base === "de") return "de";
    if (base === "en") return "en";
  }
  return "en";
}

export function browserLang(): Lang {
  return typeof navigator === "undefined" ? "en" : pickLang(navigator.languages);
}

/** `key` in `lang`, `{name}` placeholders filled from `values`. */
export function t(lang: Lang, key: Key, values: Record<string, string | number> = {}): string {
  const text = lang === "de" ? de[key] : en[key];
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = values[name];
    return value === undefined ? whole : String(value);
  });
}

/** A number as `lang` writes it: 1,234 or 1.234. */
export function formatCount(lang: Lang, n: number): string {
  return new Intl.NumberFormat(lang === "de" ? "de-DE" : "en-GB").format(n);
}

/** An ISO time as `lang` reads it, in the browser's time zone, to the minute. */
export function formatWhen(lang: Lang, iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return new Intl.DateTimeFormat(lang === "de" ? "de-DE" : "en-GB", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(at);
}
