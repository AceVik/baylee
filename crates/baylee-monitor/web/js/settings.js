/* The settings drawer. Everything in it is stored in this browser only. */

import { $, h, setText } from "./dom.js";
import { enableNotifications, enableSound, notificationBlocker } from "./alerts.js";
import { TRAINING_PRESET } from "./cards.js";
import { ACCENTS, DEFAULT_ALERTS, exportJson, freshSettings, onSettingsChange, parseImport, replaceSettings, saveSettings, settings, storageWorks } from "./store.js";

const ACCENT_WORDS = { mint: "Mint", blau: "Blau", violett: "Violett", koralle: "Koralle", gold: "Gold", rosa: "Rosa" };

function segmented(label, options, read, write) {
  const group = h("div", { class: "segmented", role: "group", "aria-label": label }, options.map(([value, text]) =>
    h("button", { type: "button", "data-value": value, text, onclick: () => { write(value); saveSettings(); } })));
  group.sync = () => {
    for (const button of group.children) button.setAttribute("aria-pressed", String(button.dataset.value === read()));
  };
  return group;
}

function field(label, control) {
  return h("div", { class: "field" }, h("span", { text: label }), control);
}

function section(title, ...children) {
  return h("section", { class: "settings-section" }, h("h3", { text: title }), children);
}

/** A number box for one alert threshold. */
function threshold(key, label, unit) {
  const input = h("input", {
    type: "number",
    min: "0",
    max: key === "diskFreeGiB" ? "10000" : "200",
    "data-threshold": key,
    "aria-label": label,
    onchange: () => {
      const value = Number(input.value);
      settings.alerts[key] = Number.isFinite(value) && input.value !== "" ? value : DEFAULT_ALERTS[key];
      saveSettings();
    },
  });
  return h("label", { class: "threshold" }, h("span", { text: label }), h("span", { class: "threshold-box" }, input, h("small", { text: unit })));
}

const THRESHOLDS = [
  ["diskWarn", "Datenträger: Warnung ab", "%"],
  ["diskCrit", "Datenträger: kritisch ab", "%"],
  ["diskFreeGiB", "Datenträger: kritisch unter", "GiB frei"],
  ["ramWarn", "RAM: Warnung ab", "%"],
  ["ramCrit", "RAM: kritisch ab", "%"],
  ["swapWarn", "Swap: Warnung ab", "%"],
  ["swapCrit", "Swap: kritisch ab", "%"],
  ["cpuTempWarn", "CPU: Warnung ab", "°C"],
  ["cpuTempCrit", "CPU: kritisch ab", "°C"],
  ["gpuTempWarn", "GPU: Warnung ab", "°C"],
  ["gpuTempCrit", "GPU: kritisch ab", "°C"],
  ["diskTempWarn", "SSD: Warnung ab", "°C"],
  ["diskTempCrit", "SSD: kritisch ab", "°C"],
  ["gpuIdleMinutes", "GPU untätig im Training nach", "Min."],
];

function exportFile() {
  const blob = new Blob([exportJson()], { type: "application/json" });
  const link = h("a", { href: URL.createObjectURL(blob), download: "baylee-pulse-layout.json" });
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
}

export function initSettings(dashboard) {
  const groups = [
    segmented("Farbschema", [["system", "Wie System"], ["light", "Hell"], ["dark", "Dunkel"]], () => settings.theme, value => { settings.theme = value; }),
    segmented("Dichte", [["normal", "Normal"], ["compact", "Kompakt"]], () => settings.density, value => { settings.density = value; }),
    segmented("Datenrate", [["iec", "MiB/s"], ["si", "MB/s"], ["bits", "Mbit/s"]], () => settings.rateUnit, value => { settings.rateUnit = value; }),
    segmented("Prozess-CPU", [["core", "100 % = 1 Kern"], ["all", "100 % = alle"]], () => settings.cpuScale, value => { settings.cpuScale = value; }),
  ];
  const [theme, density, rateUnit, cpuScale] = groups;
  const accents = h("div", { class: "swatches", role: "group", "aria-label": "Akzentfarbe" }, ACCENTS.map(name =>
    h("button", { type: "button", class: "swatch", "data-swatch": name, "aria-label": ACCENT_WORDS[name], title: ACCENT_WORDS[name], onclick: () => { settings.accent = name; saveSettings(); } })));

  const cardList = h("div", { class: "card-list" }, dashboard.cards.map(card =>
    h("label", { class: "toggle" }, h("input", { type: "checkbox", "data-card": card.id, onchange: event => dashboard.setHidden(card.id, !event.target.checked) }), h("span", { text: card.title }))));
  const notify = h("input", { type: "checkbox", onchange: async event => { await enableNotifications(event.target.checked); sync(); } });
  const sound = h("input", { type: "checkbox", onchange: event => enableSound(event.target.checked) });
  const blocker = h("p", { class: "muted-note", hidden: true });
  const message = h("p", { class: "form-message", role: "status" });
  const paste = h("textarea", { rows: "5", spellcheck: "false", "aria-label": "Layout als JSON", placeholder: "Exportiertes JSON hier einfügen …" });
  const file = h("input", { type: "file", accept: "application/json,.json", hidden: true, onchange: () => {
    const chosen = file.files[0];
    if (chosen) chosen.text().then(importText);
    file.value = "";
  } });
  const storageNote = h("p", { class: "muted-note", hidden: true, text: "Dieser Browser erlaubt kein Speichern: Einstellungen gelten nur bis zum Neuladen." });

  function importText(text) {
    try {
      const imported = parseImport(text);
      const previous = JSON.parse(JSON.stringify(settings));
      replaceSettings(imported);
      dashboard.apply();
      dashboard.offerUndo("Einstellungen importiert", () => { replaceSettings(previous); dashboard.apply(); });
      setText(message, "Importiert.");
      paste.value = "";
    } catch (error) {
      setText(message, error.message);
    }
  }

  $("settings-body").append(
    storageNote,
    section("Darstellung", field("Farbschema", theme), field("Akzentfarbe", accents), field("Dichte", density)),
    section("Einheiten", field("Datenrate", rateUnit), field("Prozess-CPU", cpuScale)),
    section(
      "Karten",
      cardList,
      h("div", { class: "button-row" },
        h("button", { class: "pill-button", type: "button", text: "Standard wiederherstellen", onclick: () => dashboard.replaceLayout(freshSettings().layout, "Standard-Layout wiederhergestellt") }),
        h("button", { class: "pill-button", type: "button", text: "Training-Ansicht", onclick: () => dashboard.replaceLayout(JSON.parse(JSON.stringify(TRAINING_PRESET)), "Training-Ansicht aktiv") })),
    ),
    section(
      "Warnungen",
      h("div", { class: "thresholds" }, THRESHOLDS.map(([key, label, unit]) => threshold(key, label, unit))),
      h("label", { class: "toggle" }, notify, h("span", { text: "Browser-Benachrichtigung bei kritischen Warnungen" })),
      h("label", { class: "toggle" }, sound, h("span", { text: "Ton bei kritischen Warnungen" })),
      blocker,
      h("button", { class: "text-button", type: "button", text: "Schwellen zurücksetzen", onclick: () => { settings.alerts = { ...DEFAULT_ALERTS, notify: settings.alerts.notify, sound: settings.alerts.sound }; saveSettings(); } }),
    ),
    section(
      "Sichern und übertragen",
      h("p", { class: "muted-note", text: "Einstellungen und Layout als JSON, zum Beispiel für den Umzug auf das Handy. Nichts davon erreicht den Dienst." }),
      h("div", { class: "button-row" },
        h("button", { class: "pill-button", type: "button", text: "Als Datei exportieren", onclick: exportFile }),
        h("button", { class: "pill-button", type: "button", text: "Aus Datei importieren", onclick: () => file.click() }),
        h("button", { class: "pill-button", type: "button", text: "JSON anzeigen", onclick: () => { paste.value = exportJson(); paste.select(); } }),
        file),
      paste,
      h("button", { class: "pill-button", type: "button", text: "Eingefügtes JSON importieren", onclick: () => importText(paste.value) }),
      message,
    ),
  );

  function sync() {
    for (const group of groups) group.sync();
    for (const button of accents.children) button.setAttribute("aria-pressed", String(button.dataset.swatch === settings.accent));
    for (const box of cardList.querySelectorAll("input")) box.checked = !settings.layout.hidden.includes(box.dataset.card);
    for (const input of document.querySelectorAll("[data-threshold]")) {
      if (document.activeElement !== input) input.value = settings.alerts[input.dataset.threshold];
    }
    notify.checked = settings.alerts.notify;
    sound.checked = settings.alerts.sound;
    const reason = notificationBlocker();
    notify.disabled = reason != null;
    blocker.hidden = reason == null;
    setText(blocker, reason ?? "");
    storageNote.hidden = storageWorks;
  }

  onSettingsChange(sync);
  sync();
  $("open-settings").addEventListener("click", () => {
    sync();
    $("settings").showModal();
  });
  $("close-settings").addEventListener("click", () => $("settings").close());
  $("close-alerts").addEventListener("click", () => $("alerts-dialog").close());
  // A click on the backdrop lands on the dialog itself.
  for (const dialog of document.querySelectorAll("dialog")) {
    dialog.addEventListener("click", event => {
      if (event.target === dialog) dialog.close();
    });
  }
}
