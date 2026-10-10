// A handful of pages need no router library: the list at `/` (its filters
// in the query string, so a filtered view can be linked), one report at
// `/r/{id}`, and the admin console under `/admin`: its overview, the live
// tables, the accounts (searched in the query string), one account, the
// pool's progress by set and one set, and the closed-beta keys.

import { useEffect, useState } from "react";

export type Route =
  | { page: "list"; search: string }
  | { page: "report"; id: string }
  | { page: "admin"; section: AdminSection; id?: string; search?: string };

export const ADMIN_SECTIONS = ["overview", "live", "accounts", "sets", "keys", "models"] as const;
export type AdminSection = (typeof ADMIN_SECTIONS)[number];

export function adminPath(section: AdminSection, id?: string): string {
  if (section === "overview") return "/admin";
  return id === undefined ? `/admin/${section}` : `/admin/${section}/${encodeURIComponent(id)}`;
}

export function parseRoute(pathname: string, search: string): Route {
  if (/^\/admin\/?$/.test(pathname)) return { page: "admin", section: "overview" };
  const admin = /^\/admin\/(live|accounts|sets|keys|models)(?:\/([^/]+))?\/?$/.exec(pathname);
  if (admin) {
    const section = admin[1] as AdminSection;
    const id = admin[2];
    if ((section === "accounts" || section === "sets") && id !== undefined) {
      return { page: "admin", section, id: decodeURIComponent(id) };
    }
    if (id === undefined) return section === "accounts" ? { page: "admin", section, search } : { page: "admin", section };
  }
  const match = /^\/r\/([^/]+)\/?$/.exec(pathname);
  const id = match?.[1];
  if (id !== undefined) return { page: "report", id: decodeURIComponent(id) };
  return { page: "list", search };
}

const EVENT = "baylee:navigate";

/** The list as last shown, filters and page, for the way back to it. */
let lastList = "/";

export function listPath(): string {
  return lastList;
}

function current(): Route {
  return parseRoute(window.location.pathname, window.location.search);
}

function remember(): void {
  if (current().page === "list") lastList = `/${window.location.search}`;
}

/** Goes to `to` without a reload. */
export function navigate(to: string, replace = false): void {
  if (replace) window.history.replaceState(null, "", to);
  else window.history.pushState(null, "", to);
  remember();
  window.dispatchEvent(new Event(EVENT));
}

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(current);
  useEffect(() => {
    remember();
    const update = () => {
      remember();
      setRoute(current());
    };
    window.addEventListener("popstate", update);
    window.addEventListener(EVENT, update);
    return () => {
      window.removeEventListener("popstate", update);
      window.removeEventListener(EVENT, update);
    };
  }, []);
  return route;
}
