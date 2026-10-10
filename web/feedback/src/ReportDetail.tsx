// One report on a page of its own (`/r/{id}`): the shared view in full,
// a way back to the list as it was, and `Escape` for the same.

import { useEffect } from "react";

import { t, type Lang } from "./i18n";
import { ReportView } from "./reports/ReportView";
import { markSeen } from "./reports/seen";
import { listPath, navigate } from "./router";

export function ReportDetail({ id, lang = "en" }: { id: string; lang?: Lang }) {
  useEffect(() => {
    markSeen(id);
  }, [id]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target;
      const typing = target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);
      if (event.key === "Escape" && !typing) {
        event.preventDefault();
        navigate(listPath());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  return (
    <main className="detail-page">
      <p>
        <a
          href="/"
          onClick={(event) => {
            event.preventDefault();
            navigate(listPath());
          }}
        >
          {t(lang, "rep.back")}
        </a>
      </p>
      <ReportView id={id} lang={lang} />
    </main>
  );
}
