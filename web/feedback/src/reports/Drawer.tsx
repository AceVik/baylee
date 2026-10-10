// A report beside the list (`ReportList`): the same view as its page, in
// its compact form, with a way to the page and a close that `Escape`
// also does. Over the list on a narrow screen.

import { useEffect, useRef } from "react";

import type { Report } from "../api";
import { t, type Lang } from "../i18n";
import { navigate } from "../router";
import { ReportView } from "./ReportView";

export function Drawer({
  id,
  lang,
  onClose,
  onChanged,
  onDeleted,
}: {
  id: string;
  lang: Lang;
  onClose: () => void;
  onChanged: (report: Report) => void;
  onDeleted: (id: string) => void;
}) {
  const panel = useRef<HTMLElement>(null);
  // Keyed by the report's id, so a new report is a new drawer.
  useEffect(() => {
    panel.current?.focus();
  }, []);
  return (
    <aside className="drawer" aria-label={t(lang, "rep.drawerLabel")} ref={panel} tabIndex={-1} data-testid="drawer">
      <div className="drawer-head">
        <a
          href={`/r/${id}`}
          className="button"
          onClick={(event) => {
            if (event.metaKey || event.ctrlKey || event.shiftKey) return;
            event.preventDefault();
            navigate(`/r/${id}`);
          }}
        >
          {t(lang, "rep.openPage")}
        </a>
        <span className="spacer" />
        <button type="button" className="quiet" onClick={onClose} aria-label={t(lang, "rep.close")} title="Esc">
          ×
        </button>
      </div>
      <ReportView id={id} lang={lang} compact onChanged={onChanged} onDeleted={onDeleted} />
    </aside>
  );
}
