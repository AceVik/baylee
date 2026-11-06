/* The page: connects, keeps the views current and owns the top bars. */

import { $, h, setText } from "./dom.js";
import { evaluateAlerts, openAlerts, renderAlerts } from "./alerts.js";
import { CARD_DEFINITIONS, createCard } from "./cards.js";
import { currentRange, data, receiveHello, receiveSample, shownSnapshot } from "./data.js";
import { number, shortTime, span, time } from "./format.js";
import { Dashboard, cancelDragOnEscape } from "./layout.js";
import { onRouteChange, route } from "./navigation.js";
import { PerformanceView } from "./performance.js";
import { ProcessTable } from "./processes.js";
import { initSettings } from "./settings.js";
import { RANGES, onSettingsChange, saveSettings, settings } from "./store.js";
import { initTheme } from "./theme.js";

const RANGE_WORDS = { 60: "1 Min.", 300: "5 Min.", 900: "15 Min.", 3600: "1 Std.", 21600: "6 Std.", 86400: "24 Std." };

initTheme();

const cards = CARD_DEFINITIONS.map(createCard);
const dashboard = new Dashboard($("grid"), cards);
const performanceView = new PerformanceView($("performance"));
const processTable = new ProcessTable($("process-host"), { limit: 60 });

let socket = null;
let reconnectTimer = null;
let lastArrival = 0;
let retry = 1000;
let view = route().view;
let resource = route().resource;

function context() {
  const s = shownSnapshot();
  return s ? { s, range: currentRange(), facts: data.facts, training: data.training } : null;
}

/** A card that throws must not take the rest of the page with it. */
function guarded(name, draw) {
  try {
    draw();
  } catch (error) {
    console.error(`Pulse: ${name}`, error);
  }
}

function renderHeader(s) {
  setText($("hostname"), data.facts?.hostname ?? "Workstation");
  const since = s.uptime_secs == null ? "Laufzeit nicht verfügbar" : `Läuft seit ${span(s.uptime_secs * 1000)}`;
  setText($("uptime"), [data.facts?.machine, since].filter(Boolean).join(" · "));
  setText($("sample-time"), `${time(s.point.at_ms)} · Messung ${number(s.sample_ms)} ms`);
  const range = currentRange();
  const first = range.points[0];
  const note = settings.range > 3600 ? "Minutenmittel" : "je Messung";
  setText($("range-note"), first ? `Verlauf ab ${shortTime(first.at_ms)} · ${note}` : "Verlauf wird aufgebaut");
}

/** The other machines' pages. The service has checked each address is a bare one. */
function renderPeers() {
  const peers = data.facts?.peers ?? [];
  const nav = $("peers");
  nav.hidden = !peers.length;
  nav.replaceChildren(...peers.map(peer => h("a", { class: "peer-link", href: peer.url, title: `Pulse auf ${peer.name} öffnen`, text: `→ ${peer.name}` })));
}

function render() {
  const current = context();
  if (!current || document.hidden) return;
  renderHeader(current.s);
  dashboard.sync(current);
  if (view === "dashboard") {
    for (const card of cards) if (!card.node.hidden) guarded(card.id, () => card.render(current));
  } else if (view === "performance") {
    guarded("performance", () => performanceView.render(current, resource));
  } else {
    guarded("processes", () => processTable.render(current.s));
  }
}

function setConnection(live, message) {
  setText($("connection"), message);
  $("connection-dot").classList.toggle("live", live);
  $("stale").hidden = live || !data.snapshot;
}

const staleAfter = () => Math.max(5000, (data.facts?.interval_ms ?? 1000) * 3);

function onMessage(event) {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch {
    return;
  }
  if (message.type === "hello") {
    receiveHello(message);
    renderPeers();
    return;
  }
  if (message.type !== "sample") return;
  if (!data.snapshot || message.snapshot.point.at_ms !== data.snapshot.point.at_ms) lastArrival = Date.now();
  receiveSample(message);
  retry = 1000;
  const fresh = Date.now() - lastArrival <= staleAfter();
  setConnection(fresh, fresh ? "Live verbunden" : "Messwerte veraltet");
  guarded("alerts", () => evaluateAlerts(message.snapshot, message.training));
  if (!data.frozen) render();
}

function connect() {
  clearTimeout(reconnectTimer);
  socket = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`);
  socket.onmessage = onMessage;
  socket.onclose = () => {
    setConnection(false, "Verbinde erneut …");
    reconnectTimer = setTimeout(connect, retry);
    retry = Math.min(15000, retry * 1.5);
  };
  socket.onerror = () => socket.close();
}

function showView(next) {
  view = next.view;
  resource = next.resource;
  for (const [name, node] of [["dashboard", $("view-dashboard")], ["performance", $("view-performance")], ["processes", $("view-processes")]]) node.hidden = name !== view;
  for (const tab of document.querySelectorAll("[data-tab]")) {
    const selected = tab.dataset.tab === view;
    if (selected) tab.setAttribute("aria-current", "page");
    else tab.removeAttribute("aria-current");
  }
  $("arrange").hidden = view !== "dashboard";
  render();
}

function setPaused(paused) {
  data.frozen = paused ? data.snapshot : null;
  const button = $("pause");
  button.setAttribute("aria-pressed", String(paused));
  setText($("pause-label"), paused ? "Pausiert" : "Pause");
  button.querySelector("use").setAttribute("href", paused ? "#i-play" : "#i-pause");
  document.body.classList.toggle("is-paused", paused);
  render();
}

function buildRanges() {
  const group = $("ranges");
  group.append(...RANGES.map(seconds => h("button", { type: "button", "data-range": seconds, text: RANGE_WORDS[seconds], onclick: () => { settings.range = seconds; saveSettings(); } })));
}

function syncRanges() {
  for (const button of $("ranges").children) button.setAttribute("aria-pressed", String(Number(button.dataset.range) === settings.range));
}

buildRanges();
syncRanges();
initSettings(dashboard);
cancelDragOnEscape(dashboard);
onRouteChange(showView);
onSettingsChange(() => {
  syncRanges();
  render();
});
$("grid").addEventListener("layoutchange", render);
$("pause").addEventListener("click", () => setPaused(!data.frozen));
$("alert-pill").addEventListener("click", openAlerts);
$("arrange").addEventListener("click", () => {
  const on = !dashboard.editing;
  dashboard.setEditing(on);
  $("arrange").setAttribute("aria-pressed", String(on));
  setText($("arrange-label"), on ? "Fertig" : "Anordnen");
  $("arrange-hint").hidden = !on;
});
document.addEventListener("visibilitychange", render);
setInterval(() => {
  if (data.snapshot && Date.now() - lastArrival > staleAfter()) {
    setConnection(false, "Messwerte veraltet");
    if (socket?.readyState === WebSocket.OPEN) socket.close();
  }
}, 2000);
window.addEventListener("pagehide", () => {
  socket.onclose = null;
  socket.close();
  clearTimeout(reconnectTimer);
});
window.addEventListener("pageshow", event => {
  if (event.persisted) connect();
});

showView(route());
renderAlerts();
connect();
