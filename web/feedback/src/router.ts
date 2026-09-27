// Two pages need no router library: the list at `/` (its filters in the
// query string, so a filtered view can be linked) and one report at `/r/{id}`.

import { useEffect, useState } from "react";

export type Route = { page: "list"; search: string } | { page: "report"; id: string };

export function parseRoute(pathname: string, search: string): Route {
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
