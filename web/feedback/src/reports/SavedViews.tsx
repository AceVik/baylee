// An admin's saved lists (`/ui/api/views`): a name for a query string,
// kept on the service, so a view made on the laptop is there on the phone.

import { useEffect, useState, type FormEvent } from "react";

import { api, Refused, SignedOut, type SavedView } from "../api";
import { t, type Lang } from "../i18n";
import { navigate } from "../router";
import { toast } from "./pieces";

export function SavedViews({ lang, query }: { lang: Lang; query: string }) {
  const [views, setViews] = useState<SavedView[]>([]);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  useEffect(() => {
    api
      .views()
      .then(setViews)
      .catch(() => {
        // The list works without its saved views.
      });
  }, []);
  const why = (e: unknown) => (e instanceof Refused ? e.message : t(lang, "admin.noAnswer"));
  const save = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const chosen = name.trim();
    if (chosen === "") return;
    api
      .saveView(chosen, query)
      .then((all) => {
        setViews(all);
        setNaming(false);
        setName("");
        toast("ok", t(lang, "rep.viewSaved", { name: chosen }));
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) toast("error", why(e));
      });
  };
  const remove = (view: SavedView) => {
    api
      .removeView(view.id)
      .then((all) => {
        setViews(all);
        toast("ok", t(lang, "rep.viewRemoved"));
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) toast("error", why(e));
      });
  };
  const current = views.find((v) => v.query === query);
  return (
    <details className="saved-views">
      <summary>
        {t(lang, "rep.saved")}
        {current !== undefined && <span className="muted"> · {current.name}</span>}
      </summary>
      <div className="menu">
        {views.length === 0 ? (
          <p className="muted small">{t(lang, "rep.noSaved")}</p>
        ) : (
          <ul>
            {views.map((view) => (
              <li key={view.id}>
                <a
                  href={view.query ? `/?${view.query}` : "/"}
                  aria-current={view.query === query ? "true" : undefined}
                  onClick={(event) => {
                    if (event.metaKey || event.ctrlKey || event.shiftKey) return;
                    event.preventDefault();
                    navigate(view.query ? `/?${view.query}` : "/", true);
                  }}
                >
                  {view.name}
                </a>
                <button
                  type="button"
                  className="quiet"
                  aria-label={t(lang, "rep.removeView", { name: view.name })}
                  onClick={() => {
                    remove(view);
                  }}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
        {naming ? (
          <form className="inline" onSubmit={save}>
            <label htmlFor="view-name" className="sr-only">
              {t(lang, "rep.viewName")}
            </label>
            <input
              id="view-name"
              maxLength={60}
              placeholder={t(lang, "rep.viewName")}
              value={name}
              // The form opens for typing in it.
              // oxlint-disable-next-line jsx-a11y/no-autofocus
              autoFocus
              onChange={(event) => {
                setName(event.target.value);
              }}
            />
            <button type="submit" className="primary" disabled={name.trim() === ""}>
              {t(lang, "rep.save")}
            </button>
            <button
              type="button"
              className="quiet"
              onClick={() => {
                setNaming(false);
              }}
            >
              {t(lang, "rep.cancel")}
            </button>
          </form>
        ) : (
          current === undefined && (
            <button
              type="button"
              className="link small"
              onClick={() => {
                setNaming(true);
              }}
            >
              {t(lang, "rep.saveView")}
            </button>
          )
        )}
      </div>
    </details>
  );
}
