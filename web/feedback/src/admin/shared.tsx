// What the admin pages share: the refresh that pauses while the page is
// hidden, a refusal in words, and the small pieces every section draws.

import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from "react";

import { Refused } from "../api";
import { t, type Lang } from "../i18n";
import { navigate } from "../router";

/** How often the numbers are asked again while the page is visible. */
export const REFRESH_MS = 15_000;

/** How often the live tables are asked again. */
export const LIVE_MS = 5_000;

function visible(): boolean {
  return typeof document === "undefined" || document.visibilityState !== "hidden";
}

/**
 * Calls `tick` now and every `ms` while the document is visible; stops
 * while it is hidden and calls it again the moment it shows. `ms` of 0
 * calls it once and never again.
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
      if (ms > 0) {
        timer = setInterval(() => {
          latest.current();
        }, ms);
      }
    };
    const stop = () => {
      if (timer !== undefined) clearInterval(timer);
      timer = undefined;
    };
    const changed = () => {
      const now = visible();
      setShown(now);
      if (now) {
        if (ms > 0) start();
      } else stop();
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

/** A refusal in the service's words, or that nothing answered. */
export function describe(lang: Lang, e: unknown): string {
  return e instanceof Refused ? e.message : t(lang, "admin.noAnswer");
}

/** A link's click as an in-page move; a modified click opens as usual. */
export const go = (to: string) => (event: MouseEvent<HTMLAnchorElement>) => {
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
  event.preventDefault();
  navigate(to);
  window.scrollTo?.({ top: 0 });
};

/** An in-page link. */
export function Link({ to, className, children }: { to: string; className?: string; children: ReactNode }) {
  return (
    <a href={to} className={className} onClick={go(to)}>
      {children}
    </a>
  );
}

export type Tone = "ok" | "info" | "warn" | "muted" | "accent" | "danger";

/** A small label with a tone; the tone never carries meaning alone. */
export function Badge({ tone = "muted", dot = false, children }: { tone?: Tone; dot?: boolean; children: ReactNode }) {
  return (
    <span className={`badge tone-${tone}`}>
      {dot && <span className="badge-dot" aria-hidden="true" />}
      {children}
    </span>
  );
}

/** One headline number with what it counts and a line under it. */
export function Kpi({
  label,
  value,
  sub,
  testId,
  to,
  live = false,
}: {
  label: string;
  value: string;
  sub?: string;
  testId?: string;
  to?: string;
  live?: boolean;
}) {
  const body = (
    <>
      <span className="kpi-label">
        {live && <span className="pulse" aria-hidden="true" />}
        {label}
      </span>
      <span className="kpi-value" data-testid={testId}>
        {value}
      </span>
      {sub !== undefined && <span className="kpi-sub">{sub}</span>}
    </>
  );
  return to === undefined ? (
    <div className="kpi">{body}</div>
  ) : (
    <a className="kpi kpi-link" href={to} onClick={go(to)}>
      {body}
    </a>
  );
}

/** A titled panel. */
export function Panel({
  id,
  title,
  actions,
  className,
  children,
}: {
  id: string;
  title: string;
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section className={`panel ${className ?? ""}`} aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{title}</h2>
        {actions}
      </div>
      {children}
    </section>
  );
}

/** Rows of term and value. */
export function Facts({ rows }: { rows: [string, ReactNode][] }) {
  return (
    <dl className="kv">
      {rows.map(([term, value]) => (
        <div className="kv-row" key={term}>
          <dt>{term}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** The current time, ticking every `ms`, for "5 minutes ago". */
export function useNow(ms = 30_000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, ms);
    return () => {
      clearInterval(timer);
    };
  }, [ms]);
  return now;
}

/** The text on the clipboard, true when it got there. */
export async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** A status line under a page's title: when it was updated, or paused. */
export function Freshness({ lang, at, shown }: { lang: Lang; at: string | null; shown: boolean }) {
  return (
    <p className="freshness muted small" aria-live="polite">
      {at === null ? (
        t(lang, "admin.loading")
      ) : shown ? (
        <>
          <span className="pulse" aria-hidden="true" />
          {t(lang, "admin.updated", {
            time: new Intl.DateTimeFormat(lang === "de" ? "de-DE" : "en-GB", { timeStyle: "medium" }).format(
              new Date(at),
            ),
          })}
        </>
      ) : (
        t(lang, "admin.paused")
      )}
    </p>
  );
}
