// The small pieces the reports pages share: a moment as "5 minutes ago"
// with the exact time behind it, a reporter as an alias, a card named in
// the text as a chip with its picture, and the toasts.

import { useId, useState, useSyncExternalStore, type ReactNode } from "react";

import { useNow } from "../admin/shared";
import type { Refs } from "../dump";
import { formatAgo, formatWhen, t, type Lang } from "../i18n";
import { alias } from "./alias";
import { cardUrl, imageUrl } from "./scryfall";

/** `iso` as how long ago, with the exact local time as the title and on request. */
export function Time({ lang, iso, exact = false }: { lang: Lang; iso: string; exact?: boolean }) {
  const now = useNow();
  const when = formatWhen(lang, iso);
  const ago = formatAgo(lang, iso, now);
  return (
    <time dateTime={iso} title={exact ? ago : when} className="when">
      {exact ? when : ago}
    </time>
  );
}

/** The exact local time of `iso` and, after it, how long ago. */
export function TimeBoth({ lang, iso }: { lang: Lang; iso: string }) {
  const now = useNow();
  return (
    <time dateTime={iso} className="when">
      {formatWhen(lang, iso)} <span className="muted">· {formatAgo(lang, iso, now)}</span>
    </time>
  );
}

/**
 * A reporter as an alias and a hue (`alias.ts`); the pseudonym itself is
 * the title and the accessible name, so a test or a screen reader can
 * still tell one from another by it.
 */
export function Reporter({
  pseudonym,
  onClick,
  lang,
}: {
  pseudonym: string;
  onClick?: () => void;
  lang: Lang;
}) {
  const a = alias(pseudonym);
  const body = (
    <>
      <span className="reporter-dot" style={{ background: `hsl(${a.hue} 60% 50%)` }} aria-hidden="true" />
      <span className="reporter-name">{a.name}</span>
      <span className="reporter-short mono muted">{a.short}</span>
    </>
  );
  return onClick ? (
    <button
      type="button"
      className="reporter link"
      title={t(lang, "rep.onlyBy", { who: pseudonym })}
      aria-label={pseudonym}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {body}
    </button>
  ) : (
    <span className="reporter" title={pseudonym}>
      {body}
    </span>
  );
}

/** A card the text names, with its picture on hover or focus. */
export function CardChip({
  name,
  scryfallId,
  lang,
  note,
}: {
  name: string;
  scryfallId: string | null;
  lang: Lang;
  note?: string | undefined;
}) {
  const [shown, setShown] = useState(false);
  const [failed, setFailed] = useState(false);
  const id = useId();
  const page = scryfallId === null ? null : cardUrl(scryfallId);
  const image = scryfallId === null ? null : imageUrl(scryfallId, "front", "normal");
  const label = note === undefined ? name : `${name} · ${note}`;
  if (page === null) {
    return (
      <span className="chip chip-card" title={label}>
        {name}
      </span>
    );
  }
  return (
    <span
      className="chip-wrap"
      onMouseEnter={() => {
        setShown(true);
      }}
      onMouseLeave={() => {
        setShown(false);
      }}
    >
      <a
        className="chip chip-card"
        href={page}
        target="_blank"
        rel="noopener noreferrer"
        title={label}
        aria-describedby={shown && image !== null && !failed ? id : undefined}
        onFocus={() => {
          setShown(true);
        }}
        onBlur={() => {
          setShown(false);
        }}
      >
        {name}
      </a>
      {shown && image !== null && !failed && (
        <span className="card-preview" role="tooltip" id={id}>
          <img
            src={image}
            alt={t(lang, "rep.cardImage", { name })}
            width={244}
            height={340}
            loading="lazy"
            onError={() => {
              setFailed(true);
            }}
          />
          <span className="card-credit">{t(lang, "rep.scryfallCredit")}</span>
        </span>
      )}
    </span>
  );
}

/** A player the text names: a seat, never an account. */
export function PlayerChip({ name, seat, lang }: { name: string; seat: number | null; lang: Lang }) {
  return (
    <span className="chip chip-player" title={seat === null ? name : t(lang, "rep.seatN", { n: seat })}>
      @{name}
    </span>
  );
}

interface Piece {
  from: number;
  to: number;
  node: ReactNode;
}

/**
 * `text` as written, with each `[Card]` and `[@Player]` the client
 * resolved (`refs`, `docs/feedback.md` §"References in the text") drawn as
 * a chip in its place. A reference whose range does not say exactly what
 * the text says there is left as text: the text is what the player wrote.
 */
export function RefText({ text, refs, lang }: { text: string; refs: Refs | null; lang: Lang }) {
  if (refs === null) return <>{text}</>;
  const chars = [...text];
  const pieces: Piece[] = [];
  const spans = (at: [number, number] | null, want: string): [number, number] | null => {
    if (at === null) return null;
    const [from, to] = at;
    if (!Number.isInteger(from) || !Number.isInteger(to) || from < 0 || to > chars.length || from >= to) return null;
    return chars.slice(from, to).join("") === want ? [from, to] : null;
  };
  refs.cards.forEach((card, i) => {
    const at = spans(card.at, `[${card.text}]`);
    if (at === null) return;
    const where = [card.zone?.replace("_", " "), card.owner === null ? null : t(lang, "rep.seatN", { n: card.owner })]
      .filter((p): p is string => typeof p === "string")
      .join(", ");
    pieces.push({
      from: at[0],
      to: at[1],
      node: <CardChip key={`c${i}`} name={card.text} scryfallId={card.scryfallId} lang={lang} note={where || undefined} />,
    });
  });
  refs.players.forEach((player, i) => {
    const at = spans(player.at, `[@${player.text}]`);
    if (at === null) return;
    pieces.push({ from: at[0], to: at[1], node: <PlayerChip key={`p${i}`} name={player.text} seat={player.seat} lang={lang} /> });
  });
  pieces.sort((a, b) => a.from - b.from);
  const out: ReactNode[] = [];
  let cursor = 0;
  pieces.forEach((piece, i) => {
    if (piece.from < cursor) return;
    if (piece.from > cursor) out.push(chars.slice(cursor, piece.from).join(""));
    out.push(piece.node);
    cursor = piece.to;
    if (i === pieces.length - 1 && cursor < chars.length) out.push(chars.slice(cursor).join(""));
  });
  if (pieces.length === 0) return <>{text}</>;
  return <>{out}</>;
}

// ---------------------------------------------------------------- toasts

export type ToastTone = "ok" | "error" | "info";

export interface Toast {
  id: number;
  tone: ToastTone;
  text: string;
}

type Listener = (toasts: Toast[]) => void;
const listeners = new Set<Listener>();
let toasts: Toast[] = [];
let nextId = 1;

function emit(): void {
  for (const l of listeners) l(toasts);
}

/** Shows `text` for a few seconds; an error stays until dismissed. */
export function toast(tone: ToastTone, text: string): void {
  const id = nextId;
  nextId += 1;
  toasts = [...toasts, { id, tone, text }].slice(-4);
  emit();
  if (tone !== "error") {
    setTimeout(() => {
      dismiss(id);
    }, 4000);
  }
}

export function dismiss(id: number): void {
  if (!toasts.some((item) => item.id === id)) return;
  toasts = toasts.filter((item) => item.id !== id);
  emit();
}

/** For tests. */
export function clearToasts(): void {
  toasts = [];
  emit();
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

const snapshot = () => toasts;

export function Toasts({ lang }: { lang: Lang }) {
  const shown = useSyncExternalStore(subscribe, snapshot, snapshot);
  if (shown.length === 0) return null;
  return (
    <div className="toasts" aria-live="polite">
      {shown.map((item) => (
        <div key={item.id} className={`toast tone-${item.tone}`} role={item.tone === "error" ? "alert" : "status"}>
          <span>{item.text}</span>
          <button
            type="button"
            className="quiet"
            aria-label={t(lang, "rep.dismiss")}
            onClick={() => {
              dismiss(item.id);
            }}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
