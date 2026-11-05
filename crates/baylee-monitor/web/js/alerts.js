/*
 * Threshold alerts. A full disk must be impossible to miss: on 4 November 2026
 * one ran full unnoticed and broke several jobs. A critical alert therefore
 * shows in a red band on every view, in the tab's title and icon, on the
 * disk's own row and, where the browser allows it, as a notification.
 *
 * Alerts live in the page because thresholds are the viewer's own setting: the
 * service only measures. They need an open page.
 */

import { $, h, icon, keyed, setText } from "./dom.js";
import { bytes, celsius, number, percent, rate, shortTime, span, time } from "./format.js";
import { allFilesystems, usageLevel } from "./disks.js";
import { recentPoints } from "./data.js";
import { saveSettings, settings } from "./store.js";

/** An alert ends only after this long without its condition, so it cannot flap. */
const CLEAR_AFTER_MS = 15000;
/** A condition still present is announced again after this long. */
const RENOTIFY_MS = 30 * 60000;
const MUTE_MS = 3600000;
const LOG_SIZE = 40;
const STALE_MS = 30000;
const PIPELINE_URGENT_MS = 6 * 3600000;
const PIPELINE_NEWS_MS = 24 * 3600000;

const active = new Map();
const muted = new Map();
const log = [];
let oomBaseline = null;
let audio = null;

function diskAlerts(storage, add) {
  const limits = settings.alerts;
  for (const { mount, usage, readOnly, device } of allFilesystems(storage)) {
    const level = usageLevel(usage, readOnly);
    if (level) {
      const title = level === "crit" ? "Datenträger fast voll" : "Datenträger wird voll";
      const forecast = usage.full_in_secs == null ? "" : `, voll in ca. ${span(usage.full_in_secs * 1000)}`;
      add(level, `disk:${mount}`, `${title}: ${mount}`, `${mount} (${device}): Noch ${bytes(usage.available)} frei, ${number(usage.used_pct)} % belegt${forecast}.`);
    }
    if (!readOnly && usage.inodes_pct >= 90) add(usage.inodes_pct >= 97 ? "crit" : "warn", `inodes:${mount}`, `Inodes fast aufgebraucht: ${mount}`, `${number(usage.inodes_pct)} % der Inodes belegt; neue Dateien schlagen bald fehl.`);
  }
  for (const disk of storage.disks) {
    if (disk.temperature == null) continue;
    if (disk.temperature >= limits.diskTempCrit) add("crit", `disktemp:${disk.name}`, `SSD sehr heiß: ${disk.model}`, celsius(disk.temperature), 10000);
    else if (disk.temperature >= limits.diskTempWarn) add("warn", `disktemp:${disk.name}`, `SSD heiß: ${disk.model}`, celsius(disk.temperature), 10000);
  }
  if (storage.checked_ms && Date.now() - storage.checked_ms > STALE_MS) add("warn", "storage-stale", "Datenträgerdaten veraltet", `Letzte Messung ${time(storage.checked_ms)}. Ein Datenträger antwortet womöglich nicht.`);
}

function memoryAlerts(s, add) {
  const limits = settings.alerts;
  const ram = s.point.ram;
  if (ram >= limits.ramCrit) add("crit", "ram", "Arbeitsspeicher fast voll", `${percent(ram)} belegt, noch ${bytes(s.memory.available)} verfügbar.`, 5000);
  else if (ram >= limits.ramWarn) add("warn", "ram", "Arbeitsspeicher wird knapp", `${percent(ram)} belegt, noch ${bytes(s.memory.available)} verfügbar.`, 5000);
  const swap = s.point.swap;
  if (s.swap.total > 0) {
    if (swap >= limits.swapCrit) add("crit", "swap", "Swap fast voll", `${percent(swap)} belegt von ${bytes(s.swap.total)}.`, 5000);
    else if (swap >= limits.swapWarn) add("warn", "swap", "Swap stark belegt", `${percent(swap)} belegt von ${bytes(s.swap.total)}.`, 5000);
  }
  const traffic = s.swap.in_bps + s.swap.out_bps;
  if (traffic > 50 * 2 ** 20) add("warn", "swap-traffic", "Starker Swap-Verkehr", `${rate(s.swap.in_bps)} herein, ${rate(s.swap.out_bps)} hinaus. Das bremst alles.`, 30000);
  oomBaseline ??= s.memory.oom_kills;
  const killed = s.memory.oom_kills - oomBaseline;
  if (killed > 0) add("crit", "oom", "Speichermangel: Prozess beendet", `Der Kernel hat seit dem Öffnen dieser Seite ${killed} Prozess${killed === 1 ? "" : "e"} beendet (OOM).`);
  const stalled = s.pressure.memory?.full[0];
  if (stalled >= 10) add("warn", "psi-memory", "Alles wartet auf Speicher", `${percent(stalled)} der Zeit standen alle Aufgaben still.`, 10000);
  const blocked = s.pressure.io?.full[0];
  if (blocked >= 30) add("warn", "psi-io", "Alles wartet auf Datenträger", `${percent(blocked)} der Zeit standen alle Aufgaben still.`, 15000);
}

const THROTTLES = { sw_thermal: "Temperatur (Software)", hw_thermal: "Temperatur (Hardware)", hw_slowdown: "Hardware-Bremse", power_brake: "externe Leistungsbremse" };

function heatAlerts(s, add) {
  const limits = settings.alerts;
  const cpu = s.sensors.cpu_temp;
  if (cpu >= limits.cpuTempCrit) add("crit", "cpu-temp", "CPU sehr heiß", celsius(cpu), 10000);
  else if (cpu >= limits.cpuTempWarn) add("warn", "cpu-temp", "CPU heiß", celsius(cpu), 10000);
  for (const gpu of s.gpus) {
    if (gpu.temperature >= limits.gpuTempCrit) add("crit", `gpu-temp:${gpu.index}`, "GPU sehr heiß", `${gpu.name}: ${celsius(gpu.temperature)}`, 10000);
    else if (gpu.temperature >= limits.gpuTempWarn) add("warn", `gpu-temp:${gpu.index}`, "GPU heiß", `${gpu.name}: ${celsius(gpu.temperature)}`, 10000);
    const reasons = gpu.throttle.filter(code => THROTTLES[code]).map(code => THROTTLES[code]);
    if (reasons.length) add("warn", `gpu-throttle:${gpu.index}`, "GPU drosselt", `${gpu.name}: ${reasons.join(", ")}.`, 10000);
  }
}

function trainingAlerts(s, training, add) {
  if (!training) return;
  const running = training.units.some(unit => unit.gone_ms == null && unit.active_state === "active");
  for (const unit of training.units) {
    if (unit.gone_ms == null && unit.active_state === "failed") add("crit", `unit:${unit.name}`, `Dienst fehlgeschlagen: ${unit.name}`, unit.result ? `Ergebnis ${unit.result}.` : "");
  }
  for (const pipeline of training.pipelines) {
    // A pipeline that failed days ago stays in its panel, not in a red band.
    const age = s.point.at_ms - (pipeline.updated_ms ?? s.point.at_ms);
    if (age > PIPELINE_NEWS_MS) continue;
    if (pipeline.phase === "failed" || pipeline.error) add(age < PIPELINE_URGENT_MS ? "crit" : "warn", `pipeline:${pipeline.name}`, `Pipeline fehlgeschlagen: ${pipeline.name}`, pipeline.error ?? `Phase ${pipeline.phase}.`);
    else if (pipeline.phase !== "complete" && pipeline.controller_alive === false) add("warn", `pipeline:${pipeline.name}`, `Pipeline ohne Controller: ${pipeline.name}`, `Phase ${pipeline.phase ?? "?"}, aber ihr Controller läuft nicht mehr.`);
  }
  const failed = training.services?.failed_names ?? [];
  if (failed.length) add("warn", "services", `${training.services.failed} Systemdienst${failed.length === 1 ? "" : "e"} fehlgeschlagen`, failed.join(", "));
  const minutes = settings.alerts.gpuIdleMinutes;
  if (running && minutes > 0 && s.gpus.length) {
    const window = recentPoints(minutes * 60);
    const covered = window.length && s.point.at_ms - window[0].at_ms >= minutes * 54000;
    if (covered && window.every(point => point.gpu != null && point.gpu < 5)) add("warn", "gpu-idle", "GPU untätig, obwohl ein Training läuft", `Seit ${minutes} Min. unter 5 % Auslastung.`);
  }
}

function collect(s, training) {
  const found = [];
  const add = (level, key, title, detail, holdMs = 0) => found.push({ level, key, title, detail, holdMs });
  diskAlerts(s.storage, add);
  memoryAlerts(s, add);
  heatAlerts(s, add);
  trainingAlerts(s, training, add);
  return found;
}

function remember(entry, ended) {
  const stamp = shortTime(Date.now());
  log.unshift({ id: `${entry.alert.key}:${Date.now()}`, level: ended ? "ok" : entry.alert.level, text: ended ? `${entry.alert.title}: behoben` : entry.alert.title, stamp });
  log.length = Math.min(log.length, LOG_SIZE);
}

function beep() {
  try {
    audio ??= new AudioContext();
    if (audio.state === "suspended") audio.resume();
    [0, 0.22].forEach(delay => {
      const tone = audio.createOscillator();
      const gain = audio.createGain();
      tone.frequency.value = 880;
      gain.gain.value = 0.08;
      tone.connect(gain).connect(audio.destination);
      tone.start(audio.currentTime + delay);
      tone.stop(audio.currentTime + delay + 0.14);
    });
  } catch {
    /* A browser without audio still shows the band. */
  }
}

/** Why this page cannot notify, or null when it can. */
export function notificationBlocker() {
  if (!("Notification" in window)) return "Dieser Browser kennt keine Benachrichtigungen.";
  if (!window.isSecureContext) return "Browser erlauben Benachrichtigungen nur über HTTPS oder localhost. Über die LAN-Adresse bleiben roter Balken, Tab-Titel und Ton.";
  if (Notification.permission === "denied") return "Benachrichtigungen sind für diese Seite im Browser gesperrt.";
  return null;
}

function announce(entry) {
  const now = Date.now();
  if (entry.notified === entry.alert.level && now - entry.notifiedAt < RENOTIFY_MS) return;
  entry.notified = entry.alert.level;
  entry.notifiedAt = now;
  if (entry.alert.level !== "crit") return;
  if (settings.alerts.sound) beep();
  if (settings.alerts.notify && !notificationBlocker() && Notification.permission === "granted") {
    try {
      new Notification(entry.alert.title, { body: entry.alert.detail, tag: entry.alert.key });
    } catch {
      /* Some mobile browsers only notify through a service worker. */
    }
  }
}

function favicon(level) {
  const line = { crit: "%23ff5a5a", warn: "%23fab219" }[level] ?? "%2362e0ba";
  const ground = { crit: "%233a1414", warn: "%233a2c10" }[level] ?? "%23123030";
  return `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 40 40'%3E%3Crect width='40' height='40' rx='12' fill='${ground}'/%3E%3Cpath d='M5 21h8l4-10 6 19 4-9h8' fill='none' stroke='${line}' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E`;
}

function renderBand(shown) {
  const band = $("banner");
  const now = Date.now();
  const critical = shown.filter(entry => entry.alert.level === "crit" && !(muted.get(entry.alert.key) > now));
  band.hidden = !critical.length;
  if (!critical.length) return;
  const first = critical[0].alert;
  const key = `${first.title}|${first.detail}|${critical.length}`;
  if (band.dataset.key === key) return;
  band.dataset.key = key;
  band.replaceChildren(
    icon("alert", "icon big"),
    h("div", { class: "banner-text" }, h("strong", { text: first.title }), h("span", { text: first.detail })),
    ...(critical.length > 1 ? [h("button", { class: "pill-button on-danger", type: "button", text: `+${critical.length - 1} weitere`, onclick: openAlerts })] : []),
    h("button", {
      class: "pill-button on-danger",
      type: "button",
      text: "1 Std. ausblenden",
      onclick: () => {
        for (const entry of critical) muted.set(entry.alert.key, Date.now() + MUTE_MS);
        renderAlerts();
      },
    }),
  );
}

function shownEntries() {
  const order = { crit: 0, warn: 1 };
  return [...active.values()].filter(entry => entry.shown).sort((a, b) => order[a.alert.level] - order[b.alert.level] || a.since - b.since);
}

function fillList(list, entries) {
  keyed(
    list,
    entries,
    entry => entry.alert.key,
    () => h("li", { class: "alert-row" }, h("span", { class: "alert-mark" }), h("div", null, h("strong"), h("small"))),
    (row, entry) => {
      const { level, title, detail } = entry.alert;
      row.className = `alert-row ${level}`;
      const mark = row.firstChild;
      if (mark.dataset.level !== level) {
        mark.dataset.level = level;
        mark.replaceChildren(icon(level === "crit" ? "alert" : "warn"));
      }
      setText(row.children[1].firstChild, title);
      setText(row.children[1].lastChild, `${detail}${detail ? " " : ""}Seit ${shortTime(entry.since)}.`);
    },
  );
}

function fillLog(list) {
  keyed(
    list,
    log,
    item => item.id,
    item => h("li", { class: `log-row ${item.level}` }, h("time", { text: item.stamp }), h("span", { text: item.text })),
    () => {},
  );
}

let shownCount = -1;

/** Everything that shows alerts, redrawn from the current set. */
export function renderAlerts() {
  const shown = shownEntries();
  renderBand(shown);
  const critical = shown.filter(entry => entry.alert.level === "crit").length;
  const worst = critical ? "crit" : shown.length ? "warn" : "";
  const pill = $("alert-pill");
  pill.className = `pill-button alert-pill ${worst}`.trim();
  setText($("alert-count"), shown.length ? `${shown.length} ${shown.length === 1 ? "Warnung" : "Warnungen"}` : "Alles gut");
  const mark = pill.querySelector("use");
  mark?.setAttribute("href", worst === "crit" ? "#i-alert" : worst === "warn" ? "#i-warn" : "#i-check");
  document.title = worst ? `${worst === "crit" ? "⛔" : "⚠"} ${shown[0].alert.title} · Pulse` : "Pulse · Baylee";
  if (worst !== document.documentElement.dataset.alarm) {
    document.documentElement.dataset.alarm = worst;
    $("favicon").href = favicon(worst);
  }
  fillList($("alerts-active"), shown);
  $("alerts-none").hidden = shown.length > 0;
  fillLog($("alerts-log"));
  shownCount = shown.length;
  for (const list of document.querySelectorAll("[data-alert-list]")) fillList(list, shown);
  for (const note of document.querySelectorAll("[data-alert-none]")) note.hidden = shown.length > 0;
}

/** Called for every sample, even while the page is paused or hidden. */
export function evaluateAlerts(snapshot, training) {
  const now = Date.now();
  for (const alert of collect(snapshot, training)) {
    const entry = active.get(alert.key) ?? { since: now, shown: false, notified: null, notifiedAt: 0 };
    entry.alert = alert;
    entry.lastSeen = now;
    active.set(alert.key, entry);
    if (!entry.shown && now - entry.since >= alert.holdMs) {
      entry.shown = true;
      remember(entry, false);
    }
    if (entry.shown) announce(entry);
  }
  for (const [key, entry] of active) {
    if (now - entry.lastSeen > CLEAR_AFTER_MS) {
      active.delete(key);
      if (entry.shown) remember(entry, true);
    }
  }
  renderAlerts();
}

export function openAlerts() {
  $("alerts-dialog").showModal();
}

/** Turns notifications on, asking the browser where it wants to be asked. */
export async function enableNotifications(on) {
  settings.alerts.notify = on;
  if (on && !notificationBlocker() && Notification.permission === "default") {
    try {
      const answer = await Notification.requestPermission();
      if (answer !== "granted") settings.alerts.notify = false;
    } catch {
      settings.alerts.notify = false;
    }
  }
  saveSettings();
}

export function enableSound(on) {
  settings.alerts.sound = on;
  saveSettings();
  if (on) beep();
}

export const alertCount = () => shownCount;
