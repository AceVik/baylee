/* Which view is open, kept in the address so reload and back work. */

const ROUTES = { "": "dashboard", dashboard: "dashboard", leistung: "performance", prozesse: "processes" };
const listeners = new Set();

export const onRouteChange = listener => listeners.add(listener);

/** The open view and, for the performance view, the chosen resource. */
export function route() {
  const [name, resource] = decodeURIComponent(location.hash.slice(1)).split("/");
  return { view: ROUTES[name] ?? "dashboard", resource: resource || null };
}

export function navigate(hash) {
  if (location.hash === hash) return;
  location.hash = hash;
}

window.addEventListener("hashchange", () => {
  for (const listener of listeners) listener(route());
});
