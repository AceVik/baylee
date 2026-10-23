// One report on a page of its own (`/r/{id}`): the shared view in full,
// a way back to the list as it was, the reports above and below it in
// that list (`j`/`k`), and `Escape` for the way back.

import { useEffect, useMemo, useState } from "react";

import { api, parseFilter, type Neighbours } from "./api";
import { formatCount, t, type Lang } from "./i18n";
import { ReportView } from "./reports/ReportView";
import { listPath, navigate } from "./router";

/** Whether a key press belongs to a field rather than to the page. */
function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName))
  );
}

/** A way to the report above or below, or its label alone where there is none. */
function step(to: string | null | undefined, label: string, key: string) {
  return to ? (
    <a
      href={`/r/${to}`}
      title={key}
      onClick={(event) => {
        if (event.metaKey || event.ctrlKey || event.shiftKey) return;
        event.preventDefault();
        navigate(`/r/${to}`, true);
      }}
    >
      {label}
    </a>
  ) : (
    <span className="muted">{label}</span>
  );
}

export function ReportDetail({ id, lang = "en", me = "" }: { id: string; lang?: Lang; me?: string }) {
  const back = listPath();
  const filter = useMemo(() => parseFilter(back.replace(/^\/\??/, "")), [back]);
  const [around, setAround] = useState<Neighbours | null>(null);

  useEffect(() => {
    api.seen([id], true).catch(() => {
      // The marker is a convenience.
    });
  }, [id]);

  useEffect(() => {
    let live = true;
    api
      .neighbours(id, filter)
      .then((n) => {
        if (live) setAround(n);
      })
      .catch(() => {
        // The page reads without its neighbours.
      });
    return () => {
      live = false;
    };
  }, [id, filter]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      if (event.key === "Escape") {
        event.preventDefault();
        navigate(back);
      } else if (event.key === "j" && around?.next) {
        event.preventDefault();
        navigate(`/r/${around.next}`, true);
      } else if (event.key === "k" && around?.previous) {
        event.preventDefault();
        navigate(`/r/${around.previous}`, true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [around, back]);

  return (
    <main className="detail-page" id="main">
      <nav className="detail-nav" aria-label={t(lang, "rep.key.neighbours")}>
        <a
          href={back}
          onClick={(event) => {
            event.preventDefault();
            navigate(back);
          }}
        >
          {t(lang, "rep.back")}
        </a>
        <span className="spacer" />
        {around !== null && (
          <>
            {step(around.previous, `‹ ${t(lang, "rep.prev")}`, "k")}
            <span className="muted small">
              {around.position === null
                ? t(lang, "rep.outsideList")
                : t(lang, "rep.position", {
                    n: formatCount(lang, around.position),
                    total: formatCount(lang, around.total),
                  })}
            </span>
            {step(around.next, `${t(lang, "rep.next")} ›`, "j")}
          </>
        )}
      </nav>
      <ReportView id={id} lang={lang} me={me} />
    </main>
  );
}
