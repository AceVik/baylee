/* Every card of the dashboard: what it is, how big it starts, how it draws. */

import { h, icon, keyed, meter, meterBar, setText, setVar, statRows } from "./dom.js";
import { Chart, drawSparkline, formats } from "./chart.js";
import { recentPoints } from "./data.js";
import { renderDisks } from "./disks.js";
import { bytes, celsius, integer, linkSpeed, mhz, number, pcieLink, percent, perSecond, rate, span, time, watts, DASH } from "./format.js";
import { ProcessTable, topSwapUsers } from "./processes.js";
import { settings } from "./store.js";
import { buildTraining } from "./training.js";
import { cardTools } from "./layout.js";

export const SERIES = {
  cpu: { label: "CPU", color: "--s-cpu", value: point => point.cpu },
  gpu: { label: "GPU", color: "--s-gpu", value: point => point.gpu },
  ram: { label: "RAM", color: "--s-ram", value: point => point.ram },
  vram: { label: "VRAM", color: "--s-vram", value: point => point.vram },
  swap: { label: "Swap", color: "--s-swap", value: point => point.swap },
};

const TILE_SECONDS = 120;
const THROTTLE_WORDS = { power_cap: "Leistungslimit", hw_slowdown: "Hardware-Bremse", sw_thermal: "Temperatur (Software)", hw_thermal: "Temperatur (Hardware)", power_brake: "Externe Bremse", sync_boost: "Sync Boost" };

export function legend(series, format) {
  const items = series.map(entry => {
    const key = h("i", { class: "key" });
    key.style.setProperty("--key", `var(${entry.color})`);
    const value = h("b", { text: DASH });
    return { entry, value, node: h("span", { class: "legend-item" }, key, h("span", { text: entry.label }), value) };
  });
  return {
    node: h("div", { class: "legend" }, items.map(item => item.node)),
    update(point) {
      for (const { entry, value } of items) {
        const reading = point ? entry.value(point) : null;
        setText(value, reading == null ? DASH : formats[format].value(reading));
      }
    },
  };
}

/** A chart with a live legend, the usual body of a time card. */
export function chartBlock(body, { series, format, max, label, compact }) {
  const key = legend(series, format);
  const chart = new Chart({ series, format, max, label, compact });
  body.append(key.node, chart.host);
  return ({ s, range }) => {
    key.update(s.point);
    chart.draw(range);
  };
}

function tile({ key, title, target, color, value, max, read }) {
  return {
    id: `tile-${key}`,
    title,
    kind: "tile",
    w: 2,
    h: 2,
    minW: 2,
    minH: 2,
    build(body) {
      const big = h("span", { class: "tile-value" });
      const unit = h("small");
      const caption = h("p", { class: "tile-caption" });
      const bar = meterBar();
      const spark = h("canvas", { class: "tile-spark", "aria-hidden": "true" });
      const link = h("a", { class: "tile-link", href: `#leistung/${target}`, "aria-label": `${title} im Detail` }, h("div", { class: "tile-number" }, big, unit), bar.node, caption);
      body.append(spark, link);
      return ({ s }) => {
        const shown = read(s);
        setText(big, shown.big);
        setText(unit, shown.unit);
        setText(caption, shown.caption);
        bar.node.hidden = shown.percent == null;
        if (shown.percent != null) meter(bar.fill, shown.percent, shown.level);
        drawSparkline(spark, recentPoints(TILE_SECONDS), value, { color, max });
      };
    },
  };
}

const ramLevel = (value, warn, crit) => (value >= crit ? "crit" : value >= warn ? "warn" : "");
const fixedRate = value => rate(value).split(" ");

const TILES = [
  tile({
    key: "cpu",
    title: "CPU",
    target: "cpu",
    color: "--s-cpu",
    value: point => point.cpu,
    max: 100,
    read: s => {
      const frequencies = s.sensors.freq_mhz;
      const average = frequencies.length ? frequencies.reduce((sum, value) => sum + value, 0) / frequencies.length : null;
      return { big: number(s.point.cpu), unit: "%", percent: s.point.cpu, caption: [s.sensors.cpu_temp == null ? null : celsius(s.sensors.cpu_temp), average == null ? null : `Ø ${mhz(average)}`].filter(Boolean).join(" · ") || DASH };
    },
  }),
  tile({
    key: "gpu",
    title: "GPU",
    target: "gpu:0",
    color: "--s-gpu",
    value: point => point.gpu,
    max: 100,
    read: s => {
      const gpu = s.gpus[0];
      if (!gpu) return { big: DASH, unit: "", percent: null, caption: s.gpu_status };
      return { big: number(gpu.utilization), unit: "%", percent: gpu.utilization, caption: `${celsius(gpu.temperature)} · ${watts(gpu.watts)}` };
    },
  }),
  tile({
    key: "ram",
    title: "Arbeitsspeicher",
    target: "ram",
    color: "--s-ram",
    value: point => point.ram,
    max: 100,
    read: s => ({ big: number(s.memory.used / 2 ** 30), unit: "GiB", percent: s.point.ram, level: ramLevel(s.point.ram, settings.alerts.ramWarn, settings.alerts.ramCrit), caption: `${percent(s.point.ram)} von ${bytes(s.memory.total)}` }),
  }),
  tile({
    key: "vram",
    title: "GPU-Speicher",
    target: "gpu:0",
    color: "--s-vram",
    value: point => point.vram,
    max: 100,
    read: s => {
      const gpu = s.gpus[0];
      if (!gpu || gpu.memory_used == null) return { big: DASH, unit: "", percent: null, caption: "Keine NVIDIA-GPU erkannt" };
      return { big: number(gpu.memory_used / 2 ** 30), unit: "GiB", percent: s.point.vram, caption: `${percent(s.point.vram)} von ${bytes(gpu.memory_total)}` };
    },
  }),
  tile({
    key: "swap",
    title: "Swap",
    target: "swap",
    color: "--s-swap",
    value: point => point.swap,
    max: 100,
    read: s => {
      if (!s.swap.total) return { big: DASH, unit: "", percent: null, caption: "Kein Swap eingerichtet" };
      return { big: number(s.swap.used / 2 ** 30), unit: "GiB", percent: s.point.swap, level: ramLevel(s.point.swap, settings.alerts.swapWarn, settings.alerts.swapCrit), caption: `${percent(s.point.swap)} von ${bytes(s.swap.total)}` };
    },
  }),
  tile({
    key: "net",
    title: "Netzwerk",
    target: "net",
    color: "--s-in",
    value: point => point.net_rx + point.net_tx,
    read: s => {
      const [amount, unit] = fixedRate(s.point.net_rx + s.point.net_tx);
      return { big: amount, unit, percent: null, caption: `↓ ${rate(s.point.net_rx)} · ↑ ${rate(s.point.net_tx)}` };
    },
  }),
];

const inOut = (inLabel, outLabel, read, write) => [
  { label: inLabel, color: "--s-in", value: read },
  { label: outLabel, color: "--s-out", value: write },
];

const sensorChips = { k10temp: "CPU", coretemp: "CPU", zenpower: "CPU", asusec: "Mainboard", acpitz: "Mainboard", nct6798: "Mainboard" };

function sensorName(reading, disks) {
  const { chip, label } = reading;
  if (/^nvme\d+$/.test(chip)) {
    const disk = disks.find(entry => entry.name.startsWith(`${chip}n`));
    return `${disk?.model ?? chip} · ${label}`;
  }
  if (chip.startsWith("iwlwifi")) return `WLAN · ${label}`;
  if (reading.kind === "fan") return `Lüfter · ${label}`;
  return `${sensorChips[chip] ?? chip} · ${label}`;
}

const PRIMARY_LABELS = /^(Tctl|Tdie|Package id 0|Composite|Chipset|Motherboard|VRM)$/;

function sensorText(reading) {
  if (reading.kind === "temp") return celsius(reading.value);
  if (reading.kind === "fan") return `${integer(reading.value)} U/min`;
  if (reading.kind === "voltage") return `${number(reading.value)} V`;
  if (reading.kind === "current") return `${number(reading.value)} A`;
  return `${number(reading.value)} W`;
}

function buildSensors(body) {
  const list = h("div", { class: "sensor-list" });
  const note = h("p", { class: "muted-note", hidden: true });
  let all = false;
  const toggle = h("button", { class: "text-button", type: "button", onclick: () => { all = !all; } });
  body.append(list, note, toggle);
  return ({ s }) => {
    const readings = s.gpus.filter(gpu => gpu.temperature != null).map(gpu => ({ chip: "GPU", label: gpu.name, kind: "temp", value: gpu.temperature, max: gpu.temp_slowdown, crit: gpu.temp_shutdown }));
    readings.push(...s.sensors.readings);
    const primary = readings.filter(reading => reading.chip === "GPU" || (reading.kind === "temp" && PRIMARY_LABELS.test(reading.label)) || (reading.kind === "fan" && reading.value > 0));
    const shown = all || !primary.length ? readings : primary;
    keyed(
      list,
      shown,
      reading => `${reading.chip}|${reading.kind}|${reading.label}`,
      () => h("div", { class: "sensor" }, h("span"), h("strong")),
      (row, reading) => {
        setText(row.firstChild, reading.chip === "GPU" ? `GPU · ${reading.label}` : sensorName(reading, s.storage.disks));
        setText(row.lastChild, sensorText(reading));
        const limit = reading.max ?? reading.crit;
        const hot = reading.kind === "temp" && reading.value >= (limit ?? 80);
        row.classList.toggle("hot", hot);
        row.title = [reading.max ? `Warnung ab ${celsius(reading.max)}` : null, reading.crit ? `Kritisch ab ${celsius(reading.crit)}` : null].filter(Boolean).join(" · ");
      },
    );
    note.hidden = readings.length > 0;
    setText(note, "Keine Sensoren verfügbar");
    toggle.hidden = readings.length <= primary.length;
    setText(toggle, all ? "Weniger anzeigen" : `Alle ${readings.length} Sensoren`);
  };
}

function coreCell() {
  return h("div", { class: "core" }, h("b"), h("small"));
}

export function renderCores(host, s) {
  const frequencies = s.sensors.freq_mhz;
  const list = s.cores.map((usage, index) => ({ usage, index, frequency: frequencies[index] }));
  keyed(host, list, entry => entry.index, coreCell, (cell, { usage, index, frequency }) => {
    setText(cell.firstChild, Math.round(usage));
    setText(cell.lastChild, frequency == null ? "" : `${number(frequency / 1000)} GHz`);
    setVar(cell, "--load", `${Math.min(100, usage)}%`);
    cell.classList.toggle("busy", usage >= 55);
    cell.title = `CPU ${index} · ${percent(usage)}${frequency == null ? "" : ` · ${mhz(frequency)}`}`;
  });
}

function buildCores(body, setTag) {
  const cores = h("div", { class: "cores" });
  const stats = h("dl", { class: "stats" });
  body.append(cores, h("p", { class: "scale-note" }, h("span", { text: "Auslastung je logischem Kern" }), h("i", { class: "heat-scale", "aria-hidden": "true" }), h("span", { text: "0 – 100 %" })), stats);
  return ({ s, facts }) => {
    setTag(facts ? `${facts.threads} Threads` : "");
    renderCores(cores, s);
    const { user, system, iowait } = s.cpu.shares;
    statRows(stats, [
      ["Anteile", `Nutzer ${number(user)} % · System ${number(system)} % · I/O-Wartezeit ${number(iowait)} %`],
      ["Takt", s.sensors.governor ? `${s.sensors.governor} · bis ${mhz(s.sensors.freq_max_mhz)}` : DASH],
    ]);
  };
}

function buildSwap(body) {
  const bar = meterBar();
  const usage = h("p", { class: "lead" });
  const stats = h("dl", { class: "stats" });
  const top = h("div", { class: "mini-list" });
  body.append(usage, bar.node);
  const chart = chartBlock(body, { series: inOut("Einlagern", "Auslagern", point => point.swap_in, point => point.swap_out), format: "rate", label: "Swap-Verkehr herein und hinaus", compact: true });
  const heading = h("h3", { text: "Meiste Swap-Nutzung" });
  body.append(stats, heading, top);
  return context => {
    const { s } = context;
    const swap = s.swap;
    chart(context);
    if (!swap.total) {
      setText(usage, "Kein Swap eingerichtet");
      bar.node.hidden = true;
      return;
    }
    setText(usage, `${bytes(swap.used)} von ${bytes(swap.total)} belegt`);
    meter(bar.fill, s.point.swap, ramLevel(s.point.swap, settings.alerts.swapWarn, settings.alerts.swapCrit));
    statRows(stats, [
      ["Frei", bytes(swap.free)],
      ["Im Cache", bytes(swap.cached)],
      ...swap.devices.map(device => [device.name, `${bytes(device.used)} von ${bytes(device.size)} · ${device.kind === "file" ? "Datei" : "Partition"}`]),
    ]);
    const users = topSwapUsers(s, 3);
    heading.hidden = !users.length;
    keyed(top, users, process => process.pid, () => h("div", { class: "sensor" }, h("span"), h("strong")), (row, process) => {
      setText(row.firstChild, process.name);
      setText(row.lastChild, bytes(process.swap));
    });
  };
}

/** The interfaces worth a row: real ones, plus virtual ones on request. */
export function shownNetworks(s) {
  return s.networks.filter(network => settings.showVirtual || (network.physical && (network.state === "up" || network.rx_total + network.tx_total > 0)));
}

const LINK_WORDS = { up: "verbunden", down: "getrennt", unknown: "unbekannt", dormant: "ruht", lowerlayerdown: "getrennt" };

function buildNetwork(body) {
  const chart = chartBlock(body, { series: inOut("Empfang", "Versand", point => point.net_rx, point => point.net_tx), format: "rate", label: "Datenrate der physischen Netzwerkanschlüsse", compact: true });
  const list = h("div", { class: "net-list" });
  const note = h("p", { class: "muted-note" });
  body.append(list, note);
  return context => {
    const { s } = context;
    chart(context);
    keyed(list, shownNetworks(s), network => network.name, () => h("a", { class: "net-row" }, h("strong"), h("small"), h("span")), (row, network) => {
      row.href = `#leistung/net:${network.name}`;
      setText(row.firstChild, network.name);
      setText(row.children[1], `${LINK_WORDS[network.state] ?? network.state}${network.speed_mbps ? ` · ${linkSpeed(network.speed_mbps)}` : ""}`);
      setText(row.lastChild, `↓ ${rate(network.rx_bps)} ↑ ${rate(network.tx_bps)}`);
    });
    setText(note, `TCP ${s.sockets.tcp} · UDP ${s.sockets.udp} · Summe nur physischer Anschlüsse`);
  };
}

function buildDiskIo(body) {
  const chart = chartBlock(body, { series: inOut("Lesen", "Schreiben", point => point.disk_read, point => point.disk_write), format: "rate", label: "Lese- und Schreibrate aller Datenträger", compact: true });
  const list = h("div", { class: "net-list" });
  body.append(list);
  return context => {
    chart(context);
    keyed(list, context.s.storage.disks, disk => disk.name, () => h("a", { class: "net-row" }, h("strong"), h("small"), h("span")), (row, disk) => {
      row.href = `#leistung/disk:${disk.name}`;
      setText(row.firstChild, disk.model);
      setText(row.children[1], `${disk.name} · ${number(disk.io?.busy_pct)} % ausgelastet`);
      setText(row.lastChild, `↓ ${rate(disk.io?.read_bps)} ↑ ${rate(disk.io?.write_bps)}`);
    });
  };
}

function buildPower(body) {
  const chart = chartBlock(body, {
    series: [
      { label: "GPU", color: "--s-gpu", value: point => point.gpu_watts },
      { label: "CPU-Package", color: "--s-cpu", value: point => point.cpu_watts },
    ],
    format: "watts",
    label: "Verlauf der gemessenen GPU- und CPU-Leistung in Watt",
  });
  const note = h("p", { class: "muted-note" });
  body.append(note);
  return context => {
    chart(context);
    setText(note, `${context.s.cpu_power_status}. Der Verbrauch des gesamten PCs ist nicht messbar.`);
  };
}

function buildGpu(body) {
  const list = h("div", { class: "gpu-list" });
  const note = h("p", { class: "muted-note" });
  body.append(list, note);
  return ({ s }) => {
    keyed(list, s.gpus, gpu => gpu.index, () => {
      const bar = meterBar();
      const memory = meterBar();
      const node = h("article", { class: "gpu-item" }, h("strong"), h("div", { class: "meter-row" }, h("span", { text: "Last" }), bar.node, h("b")), h("div", { class: "meter-row" }, h("span", { text: "VRAM" }), memory.node, h("b")), h("div", { class: "chip-row" }), h("dl", { class: "stats" }));
      node.bars = { bar, memory };
      return node;
    }, (node, gpu) => {
      const [title, load, vram, chips, stats] = node.children;
      setText(title, `${gpu.index} · ${gpu.name}`);
      meter(node.bars.bar.fill, gpu.utilization);
      setText(load.lastChild, percent(gpu.utilization));
      const memoryPercent = gpu.memory_total ? (100 * gpu.memory_used) / gpu.memory_total : null;
      meter(node.bars.memory.fill, memoryPercent);
      setText(vram.lastChild, `${bytes(gpu.memory_used)} / ${bytes(gpu.memory_total)}`);
      const reasons = gpu.throttle.map(code => THROTTLE_WORDS[code] ?? code).join("|");
      if (chips.dataset.key !== reasons) {
        chips.dataset.key = reasons;
        chips.replaceChildren(...gpu.throttle.map(code => h("span", { class: "chip warn" }, icon("warn"), THROTTLE_WORDS[code] ?? code)));
      }
      statRows(stats, gpuStats(gpu));
    });
    note.hidden = s.gpus.length > 0;
    setText(note, s.gpu_status);
  };
}

export function gpuStats(gpu) {
  return [
    ["Temperatur", `${celsius(gpu.temperature)}${gpu.temp_slowdown ? ` · bremst ab ${gpu.temp_slowdown} °C` : ""}`],
    ["Leistung", `${watts(gpu.watts)} von ${watts(gpu.power_limit)}`],
    ["Takt", `${mhz(gpu.clock_graphics)} · Speicher ${mhz(gpu.clock_memory)}`],
    ["PCIe", pcieLink(gpu.pcie)],
    ["PCIe-Durchsatz", `↓ ${rate(gpu.pcie_rx_bps)} · ↑ ${rate(gpu.pcie_tx_bps)}`],
    ["Lüfter", gpu.fans.length ? gpu.fans.map(percent).join(" / ") : DASH],
    ["Leistungsstufe", gpu.pstate == null ? DASH : `P${gpu.pstate}`],
  ];
}

function buildPressure(body) {
  const chart = chartBlock(body, {
    series: [
      { label: "CPU", color: "--s-cpu", value: point => point.psi_cpu },
      { label: "Speicher", color: "--s-ram", value: point => point.psi_mem },
      { label: "Datenträger", color: "--s-out", value: point => point.psi_io },
    ],
    format: "percent",
    label: "Anteil der Zeit, in der Aufgaben auf CPU, Speicher oder Datenträger warten mussten",
    compact: true,
  });
  body.append(h("p", { class: "muted-note", text: "Zeitanteil, in dem mindestens eine Aufgabe auf die Ressource warten musste (10-s-Mittel). Hoch heißt: Der Rechner bremst, nicht nur, dass er arbeitet." }));
  return chart;
}

function buildTemps(body) {
  return chartBlock(body, {
    series: [
      { label: "CPU", color: "--s-cpu", value: point => point.cpu_temp },
      { label: "GPU", color: "--s-gpu", value: point => point.gpu_temp },
    ],
    format: "celsius",
    label: "Verlauf der CPU- und GPU-Temperatur",
    compact: true,
  });
}

function buildSystem(body) {
  const stats = h("dl", { class: "stats" });
  body.append(stats);
  return ({ s, facts }) => {
    statRows(stats, [
      ["Rechner", facts?.hostname ?? DASH],
      ["System", facts?.os ?? DASH],
      ["Kernel", facts?.kernel ?? DASH],
      ["CPU", facts ? `${facts.cpu} · ${facts.physical_cores ?? "?"} Kerne / ${facts.threads} Threads` : DASH],
      ["Laufzeit", s.uptime_secs == null ? DASH : span(s.uptime_secs * 1000)],
      ["Gestartet", s.cpu.boot_ms ? new Date(s.cpu.boot_ms).toLocaleString("de-DE", { dateStyle: "short", timeStyle: "short" }) : DASH],
      ["Load 1 / 5 / 15", s.load ? s.load.map(number).join(" / ") : DASH],
      ["Aufgaben", `${integer(s.processes.length)} Prozesse · ${s.cpu.running} laufend · ${s.cpu.blocked} blockiert`],
      ["Kontextwechsel", perSecond(s.cpu.context_switches_per_s)],
      ["Messung", `${number(s.sample_ms)} ms · ${time(s.point.at_ms)}`],
    ]);
  };
}

function buildServices(body) {
  const lead = h("p", { class: "lead" });
  const list = h("ul", { class: "plain-list" });
  body.append(lead, list);
  return ({ training }) => {
    const services = training?.services;
    if (!services) {
      setText(lead, training?.units_problem ?? "Warte auf systemd …");
      return;
    }
    setText(lead, `${services.active} von ${services.total} Diensten aktiv · ${services.failed} fehlgeschlagen`);
    keyed(list, services.failed_names, name => name, name => h("li", { class: "chip crit" }, icon("alert"), name), () => {});
  };
}

function buildAlertsCard(body) {
  body.append(h("p", { class: "lead", "data-alert-none": "", text: "Keine aktiven Warnungen." }), h("ul", { class: "alert-list", "data-alert-list": "" }));
  return () => {};
}

function buildProcessCard(body) {
  const table = new ProcessTable(body, { compact: true, limit: 14 });
  body.append(h("a", { class: "text-button", href: "#prozesse", text: "Alle Prozesse" }));
  return ({ s }) => table.render(s);
}

export const CARD_DEFINITIONS = [
  ...TILES,
  { id: "training", title: "Training", w: 12, h: 6, minW: 6, minH: 3, build: buildTraining },
  {
    id: "activity",
    title: "Auslastung",
    w: 8,
    h: 4,
    minW: 4,
    minH: 3,
    build: body => chartBlock(body, { series: [SERIES.cpu, SERIES.gpu, SERIES.ram, SERIES.vram], format: "percent", max: 100, label: "Verlauf der CPU-, GPU-, RAM- und VRAM-Auslastung in Prozent" }),
  },
  { id: "power", title: "Leistungsaufnahme", w: 4, h: 4, minW: 3, minH: 3, build: buildPower },
  { id: "disks", title: "Datenträger", w: 6, h: 9, minW: 4, minH: 3, build: (body) => { const host = h("div"); body.append(host); return ({ s }) => renderDisks(host, s.storage); } },
  { id: "processes", title: "Prozesse", w: 6, h: 9, minW: 4, minH: 3, build: buildProcessCard },
  { id: "swap", title: "Swap", w: 4, h: 6, minW: 3, minH: 3, build: buildSwap },
  { id: "network", title: "Netzwerk", w: 4, h: 6, minW: 3, minH: 3, build: buildNetwork },
  { id: "diskio", title: "Datenträger-Aktivität", w: 4, h: 6, minW: 3, minH: 3, build: buildDiskIo },
  { id: "cores", title: "CPU-Kerne", w: 4, h: 6, minW: 3, minH: 3, build: buildCores },
  { id: "gpu", title: "Grafikkarte", w: 4, h: 6, minW: 3, minH: 3, build: buildGpu },
  { id: "sensors", title: "Sensoren", w: 4, h: 6, minW: 3, minH: 3, build: buildSensors },
  { id: "temps", title: "Temperaturverlauf", w: 4, h: 4, minW: 3, minH: 3, build: buildTemps },
  { id: "pressure", title: "Wartezeiten", w: 4, h: 4, minW: 3, minH: 3, build: buildPressure },
  { id: "system", title: "System", w: 4, h: 4, minW: 3, minH: 3, build: buildSystem },
  { id: "services", title: "Systemdienste", w: 4, h: 3, minW: 2, minH: 2, build: buildServices },
  { id: "alerts", title: "Warnungen", w: 8, h: 3, minW: 2, minH: 2, build: buildAlertsCard },
];

/** A preset for watching a long run: training and what can stop it, first. */
export const TRAINING_PRESET = {
  order: ["tile-gpu", "tile-vram", "tile-ram", "tile-swap", "tile-cpu", "tile-net", "training", "alerts", "activity", "disks", "pressure", "temps", "gpu", "diskio", "processes", "sensors", "swap", "network", "cores", "power", "system", "services"],
  hidden: ["network", "power", "cores"],
  sizes: { alerts: { w: 12, h: 2 }, activity: { w: 7, h: 4 }, disks: { w: 5, h: 6 }, pressure: { w: 7, h: 3 }, temps: { w: 4, h: 3 } },
};

/** Wraps a definition in its card and returns the piece the grid needs. */
export function createCard(definition) {
  const tag = h("span", { class: "tag", hidden: true });
  const grip = h("button", { class: "icon-button grip", type: "button", hidden: true, "aria-label": `${definition.title} verschieben. Pfeiltasten: bewegen. Umschalt und Pfeiltasten: Größe. Entf: ausblenden.` }, icon("grip"));
  const tools = cardTools(definition.title);
  const head = h("header", { class: "card-head" }, grip, h("h2", { text: definition.title }), tag, tools);
  const body = h("div", { class: "card-body" });
  const handle = h("div", { class: "resize-handle", "aria-hidden": "true" }, icon("resize"));
  const node = h("article", { class: `card ${definition.kind ?? ""}`.trim(), "data-card": definition.id, "aria-label": definition.title }, head, body, handle);
  const setTag = value => {
    tag.hidden = !value;
    setText(tag, value);
  };
  return { ...definition, node, head, grip, tools, handle, render: definition.build(body, setTag) };
}
