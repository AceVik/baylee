// The crashes the list's filter matches, grouped by signature
// (`GET /ui/api/crashes`): the same crash, its numbers aside, is one row,
// with how often and how lately it came. A row's button shows its reports.

import { useEffect, useState } from "react";

import { api, SignedOut, type CrashGroup, type Filter } from "../api";
import { formatCount, t, type Lang } from "../i18n";
import { describe } from "./ReportView";
import { Time } from "./pieces";

export function CrashGroups({
  lang,
  filter,
  onPick,
}: {
  lang: Lang;
  filter: Filter;
  onPick: (signature: string) => void;
}) {
  const [groups, setGroups] = useState<CrashGroup[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    api
      .crashes(filter)
      .then((g) => {
        if (live) {
          setGroups(g);
          setError(null);
        }
      })
      .catch((e: unknown) => {
        if (live && !(e instanceof SignedOut)) setError(describe(lang, e));
      });
    return () => {
      live = false;
    };
  }, [filter, lang]);
  if (error !== null) {
    return (
      <p className="error" role="alert">
        {error}
      </p>
    );
  }
  if (groups === null) return <p className="muted">{t(lang, "rep.loading")}</p>;
  if (groups.length === 0) return <p className="muted">{t(lang, "rep.noGroups")}</p>;
  const n = (v: number) => formatCount(lang, v);
  return (
    <div className="table-wrap">
      <table className="reports groups">
        <thead>
          <tr>
            <th scope="col" className="wide">
              {t(lang, "rep.col.crash")}
            </th>
            <th scope="col" className="num">
              {t(lang, "rep.col.count")}
            </th>
            <th scope="col" className="num">
              {t(lang, "rep.col.openN")}
            </th>
            <th scope="col">{t(lang, "rep.col.first")}</th>
            <th scope="col">{t(lang, "rep.col.last")}</th>
            <th scope="col">
              <span className="sr-only">{t(lang, "rep.showGroup")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {groups.map((g) => (
            <tr key={g.signature}>
              <td className="wide">
                <span className="mono">{g.message ?? "—"}</span>
                {g.location !== null && <span className="muted mono small"> · {g.location}</span>}
              </td>
              <td className="num">{n(g.count)}</td>
              <td className="num">{g.open > 0 ? n(g.open) : <span className="muted">0</span>}</td>
              <td className="nowrap">
                <Time lang={lang} iso={g.first} />
              </td>
              <td className="nowrap">
                <Time lang={lang} iso={g.last} />
              </td>
              <td>
                <button
                  type="button"
                  onClick={() => {
                    onPick(g.signature);
                  }}
                >
                  {t(lang, "rep.showGroup")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
