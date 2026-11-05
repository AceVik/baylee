/* Runs before the first paint, so a light-mode viewer sees no dark flash. */
try {
  const stored = JSON.parse(localStorage.getItem("baylee-pulse.settings.v1") ?? "{}");
  const root = document.documentElement;
  if (stored.theme === "light" || stored.theme === "dark") root.setAttribute("data-theme", stored.theme);
  if (typeof stored.accent === "string") root.setAttribute("data-accent", stored.accent);
  if (stored.density === "compact") root.setAttribute("data-density", "compact");
} catch {
  /* Storage may be blocked or hold anything; the defaults are fine. */
}
