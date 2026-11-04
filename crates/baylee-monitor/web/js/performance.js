/*
 * The performance view, after the Windows Task Manager: every resource in a
 * list with a live mini graph, and the chosen one large, with its details.
 */

import { h, keyed, setText, statRows } from "./dom.js";
import { disposeChartsIn, drawCoreStrip, drawSparkline } from "./chart.js";
import { chartBlock, gpuStats, renderCores, shownNetworks } from "./cards.js";
import { data, diskPoint, gpuPoint, netPoint, recentPoints } from "./data.js";
import { renderDisks } from "./disks.js";
import { bytes, celsius, integer, linkSpeed, mhz, number, pcieLink, percent, perSecond, rate, span, DASH } from "./format.js";
import { navigate } from "./navigation.js";
import { topSwapUsers } from "./processes.js";
import { saveSettings, settings } from "./store.js";

const LIST_SECONDS = 60;

const inOut = (inLabel, outLabel, read, write) => [
  { label: inLabel, color: "--s-in", value: read },
  { label: outLabel, color: "--s-out", value: write },
];

/** Every resource of the current snapshot, in the list's order. */
function resources(s) {
  const list = [
    { id: "cpu", title: "CPU", color: "--s-cpu", value: point => point.cpu, max: 100, line: () => `${number(s.point.cpu)} %${s.sensors.cpu_temp == null ? "" : ` · ${celsius(s.sensors.cpu_temp)}`}` },
    { id: "ram", title: "Arbeitsspeicher", color: "--s-ram", value: point => point.ram, max: 100, line: () => `${bytes(s.memory.used)} / ${bytes(s.memory.total)} (${number(s.point.ram)} %)` },
    { id: "swap", title: "Swap", color: "--s-swap", value: point => point.swap, max: 100, line: () => (s.swap.total ? `${bytes(s.swap.used)} / ${bytes(s.swap.total)}` : "Nicht eingerichtet") },
  ];
  for (const disk of s.storage.disks) {
    list.push({ id: `disk:${disk.name}`, title: disk.model, detail: disk.name, color: "--s-out", value: point => diskPoint(point, disk.name)?.[3], max: 100, line: () => `${number(disk.io?.busy_pct)} % · ${rate((disk.io?.read_bps ?? 0) + (disk.io?.write_bps ?? 0))}` });
  }
  list.push({ id: "net", title: "Netzwerk gesamt", color: "--s-in", value: point => point.net_rx + point.net_tx, line: () => `↓ ${rate(s.point.net_rx)} ↑ ${rate(s.point.net_tx)}` });
  for (const network of shownNetworks(s)) {
    const entry = point => netPoint(point, network.name);
    list.push({ id: `net:${network.name}`, title: network.name, color: "--s-in", value: point => { const reading = entry(point); return reading ? reading[1] + reading[2] : null; }, line: () => `↓ ${rate(network.rx_bps)} ↑ ${rate(network.tx_bps)}` });
  }
  for (const gpu of s.gpus) {
    list.push({ id: `gpu:${gpu.index}`, title: `GPU ${gpu.index}`, detail: gpu.name, color: "--s-gpu", value: point => gpuPoint(point, gpu.index)?.[1], max: 100, line: () => `${percent(gpu.utilization)}${gpu.temperature == null ? "" : ` · ${celsius(gpu.temperature)}`}` });
  }
  return list;
}

function memoryBar(host, s) {
  const { total, used, free } = s.memory;
  const reclaimable = Math.max(0, total - used - free);
  const parts = [
    ["Verwendet", used, "--s-ram"],
    ["Freigebbar (Cache)", reclaimable, "--part-mid"],
    ["Frei", free, "--part-low"],
  ];
  if (!host.firstChild) {
    host.append(h("div", { class: "segments" }, parts.map(([label, , color]) => {
      const part = h("i", { title: label });
      part.style.background = `var(${color})`;
      return part;
    })), h("div", { class: "legend" }, parts.map(([label, , color]) => {
      const key = h("i", { class: "key" });
      key.style.setProperty("--key", `var(${color})`);
      return h("span", { class: "legend-item" }, key, h("span", { text: label }), h("b"));
    })));
  }
  const [bar, key] = host.children;
  parts.forEach(([label, value], at) => {
    bar.children[at].style.flexGrow = String(value);
    bar.children[at].title = `${label}: ${bytes(value)}`;
    setText(key.children[at].lastChild, bytes(value));
  });
}

const memoryStats = s => [
  ["Verfügbar", bytes(s.memory.available)],
  ["Cache", bytes(s.memory.cached)],
  ["Puffer", bytes(s.memory.buffers)],
  ["Gemeinsam (shmem)", bytes(s.memory.shared)],
  ["Kernel-Slab", bytes(s.memory.slab)],
  ["Noch zu schreiben", bytes(s.memory.dirty + s.memory.writeback), "Dirty und Writeback: Daten, die noch auf den Datenträger müssen"],
  ["Harte Seitenfehler", perSecond(s.memory.major_faults_per_s)],
  ["OOM-Kills seit Start", integer(s.memory.oom_kills)],
  ["Wartezeit Speicher", s.pressure.memory ? `${percent(s.pressure.memory.some[0])} (10 s)` : DASH],
];

const cpuStats = (s, facts) => [
  ["Modell", facts?.cpu ?? DASH],
  ["Kerne / Threads", facts ? `${facts.physical_cores ?? "?"} / ${facts.threads}` : DASH],
  ["Nutzer", percent(s.cpu.shares.user)],
  ["System", percent(s.cpu.shares.system)],
  ["I/O-Wartezeit", percent(s.cpu.shares.iowait)],
  ["Interrupt-Zeit", percent(s.cpu.shares.irq)],
  ["Takt (min – max)", `${mhz(s.sensors.freq_min_mhz)} – ${mhz(s.sensors.freq_max_mhz)}`],
  ["Regler", s.sensors.governor ? `${s.sensors.governor} (${s.sensors.freq_driver})` : DASH],
  ["Load 1 / 5 / 15", s.load ? s.load.map(number).join(" / ") : DASH],
  ["Prozesse", `${integer(s.processes.length)} · ${s.cpu.running} laufend · ${s.cpu.blocked} blockiert`],
  ["Kontextwechsel", perSecond(s.cpu.context_switches_per_s)],
  ["Interrupts", perSecond(s.cpu.interrupts_per_s)],
  ["Neue Prozesse", perSecond(s.cpu.forks_per_s)],
  ["Laufzeit", s.uptime_secs == null ? DASH : span(s.uptime_secs * 1000)],
  ["Leistung", s.cpu.watts == null ? "nicht lesbar" : `${number(s.cpu.watts)} W`, s.cpu_power_status],
];

function cpuDetail(panel) {
  const render = chartBlock(panel, { series: [{ label: "Auslastung", color: "--s-cpu", value: point => point.cpu }, { label: "I/O-Wartezeit", color: "--s-out", value: point => point.iowait }], format: "percent", max: 100, label: "Verlauf der CPU-Auslastung und der I/O-Wartezeit" });
  const strip = h("canvas", { class: "core-strip", role: "img", "aria-label": "Auslastung je logischem Kern in den letzten fünf Minuten, seit diese Seite offen ist" });
  const cores = h("div", { class: "cores" });
  const stats = h("dl", { class: "stats wide" });
  panel.append(h("h3", { text: "Kerne" }), cores, h("h3", { text: "Kerne im Zeitverlauf" }), strip, h("p", { class: "scale-note" }, h("span", { text: "Nur seit diese Seite offen ist, höchstens 5 Min." }), h("i", { class: "heat-scale", "aria-hidden": "true" }), h("span", { text: "0 – 100 %" })), stats);
  return context => {
    render(context);
    renderCores(cores, context.s);
    drawCoreStrip(strip, data.coreHistory);
    statRows(stats, cpuStats(context.s, context.facts));
  };
}

function ramDetail(panel) {
  const render = chartBlock(panel, { series: [{ label: "Belegt", color: "--s-ram", value: point => point.ram }], format: "percent", max: 100, label: "Verlauf der RAM-Belegung" });
  const bar = h("div");
  const stats = h("dl", { class: "stats wide" });
  panel.append(h("h3", { text: "Zusammensetzung" }), bar, stats);
  return context => {
    render(context);
    memoryBar(bar, context.s);
    statRows(stats, memoryStats(context.s));
  };
}

function swapDetail(panel) {
  const used = chartBlock(panel, { series: [{ label: "Belegt", color: "--s-swap", value: point => point.swap }], format: "percent", max: 100, label: "Verlauf der Swap-Belegung" });
  panel.append(h("h3", { text: "Swap-Verkehr" }));
  const traffic = chartBlock(panel, { series: inOut("Einlagern", "Auslagern", point => point.swap_in, point => point.swap_out), format: "rate", label: "Rate, mit der Speicher ein- und ausgelagert wird", compact: true });
  const stats = h("dl", { class: "stats wide" });
  const top = h("div", { class: "mini-list" });
  panel.append(stats, h("h3", { text: "Meiste Swap-Nutzung" }), top);
  return context => {
    used(context);
    traffic(context);
    const { s } = context;
    statRows(stats, [
      ["Gesamt", bytes(s.swap.total)],
      ["Belegt", bytes(s.swap.used)],
      ["Frei", bytes(s.swap.free)],
      ["Im Cache", bytes(s.swap.cached)],
      ["Einlagern", rate(s.swap.in_bps)],
      ["Auslagern", rate(s.swap.out_bps)],
      ...s.swap.devices.map(device => [device.name, `${bytes(device.used)} / ${bytes(device.size)} · Priorität ${device.priority}`]),
    ]);
    keyed(top, topSwapUsers(s, 8), process => process.pid, () => h("div", { class: "sensor" }, h("span"), h("strong")), (row, process) => {
      setText(row.firstChild, `${process.name} · PID ${process.pid}`);
      setText(row.lastChild, bytes(process.swap));
    });
  };
}

function diskDetail(panel, name) {
  const busy = chartBlock(panel, { series: [{ label: "Auslastung", color: "--s-out", value: point => diskPoint(point, name)?.[3] }], format: "percent", max: 100, label: "Zeitanteil, in dem der Datenträger arbeitet" });
  panel.append(h("h3", { text: "Datenrate" }));
  const rates = chartBlock(panel, { series: inOut("Lesen", "Schreiben", point => diskPoint(point, name)?.[1], point => diskPoint(point, name)?.[2]), format: "rate", label: "Lese- und Schreibrate", compact: true });
  const stats = h("dl", { class: "stats wide" });
  const tree = h("div");
  panel.append(stats, h("h3", { text: "Partitionen" }), tree);
  return context => {
    busy(context);
    rates(context);
    const { s } = context;
    const disk = s.storage.disks.find(entry => entry.name === name);
    if (!disk) return;
    statRows(stats, [
      ["Größe", bytes(disk.size)],
      ["Anschluss", disk.transport],
      ["Bauart", disk.rotational ? "Festplatte" : "SSD"],
      ["Temperatur", celsius(disk.temperature)],
      disk.link ? ["PCIe", pcieLink(disk.link)] : null,
      ["Lesen", `${rate(disk.io?.read_bps)} · ${number(disk.io?.read_iops)} IOPS`],
      ["Schreiben", `${rate(disk.io?.write_bps)} · ${number(disk.io?.write_iops)} IOPS`],
      ["Warteschlange", integer(disk.io?.in_flight)],
    ]);
    renderDisks(tree, { disks: [disk], volumes: [], problem: null });
  };
}

function networkTotals(panel) {
  const render = chartBlock(panel, { series: inOut("Empfang", "Versand", point => point.net_rx, point => point.net_tx), format: "rate", label: "Datenrate aller physischen Anschlüsse" });
  const stats = h("dl", { class: "stats wide" });
  panel.append(stats, h("p", { class: "muted-note", text: "Summe der physischen Anschlüsse; Brücken und virtuelle Schnittstellen zählen nicht doppelt." }));
  return context => {
    render(context);
    const { s } = context;
    statRows(stats, [
      ["Empfang", rate(s.point.net_rx)],
      ["Versand", rate(s.point.net_tx)],
      ["TCP-Sockets", `${integer(s.sockets.tcp)} (+${integer(s.sockets.time_wait)} in TIME_WAIT)`],
      ["UDP-Sockets", integer(s.sockets.udp)],
    ]);
  };
}

const KIND_WORDS = { ethernet: "Ethernet", wlan: "WLAN", bridge: "Brücke", tunnel: "Tunnel", loopback: "Loopback", virtual: "Virtuell" };

function networkDetail(panel, name) {
  const entry = point => netPoint(point, name);
  const render = chartBlock(panel, { series: inOut("Empfang", "Versand", point => entry(point)?.[1], point => entry(point)?.[2]), format: "rate", label: `Datenrate von ${name}` });
  const stats = h("dl", { class: "stats wide" });
  panel.append(stats, h("p", { class: "muted-note", text: "Adressen zeigt Pulse absichtlich nicht: Die Seite hat kein Login, und eine globale IPv6- oder MAC-Adresse sagt zu viel über das Netz." }));
  return context => {
    render(context);
    const network = context.s.networks.find(item => item.name === name);
    if (!network) return;
    const usage = network.speed_mbps ? (((network.rx_bps + network.tx_bps) * 8) / (network.speed_mbps * 1e6)) * 100 : null;
    statRows(stats, [
      ["Art", KIND_WORDS[network.kind] ?? network.kind],
      ["Zustand", network.state],
      ["Verbindung", network.speed_mbps ? `${linkSpeed(network.speed_mbps)}${network.duplex ? ` · ${network.duplex}-duplex` : ""}` : DASH],
      ["Auslastung", usage == null ? DASH : percent(usage), "Empfang und Versand zusammen, gemessen an der Verbindungsgeschwindigkeit"],
      ["MTU", integer(network.mtu)],
      ["Empfang", `${rate(network.rx_bps)} · ${integer(network.rx_pps)} Pakete/s`],
      ["Versand", `${rate(network.tx_bps)} · ${integer(network.tx_pps)} Pakete/s`],
      ["Empfangen seit Start", bytes(network.rx_total)],
      ["Gesendet seit Start", bytes(network.tx_total)],
      ["Fehler ↓ / ↑", `${integer(network.rx_errors)} / ${integer(network.tx_errors)}`],
      ["Verworfen ↓ / ↑", `${integer(network.rx_drops)} / ${integer(network.tx_drops)}`],
    ]);
  };
}

function gpuDetail(panel, index) {
  const entry = point => gpuPoint(point, index);
  const load = chartBlock(panel, { series: [{ label: "Last", color: "--s-gpu", value: point => entry(point)?.[1] }, { label: "VRAM", color: "--s-vram", value: point => entry(point)?.[2] }], format: "percent", max: 100, label: "Verlauf von GPU-Last und VRAM" });
  const pair = h("div", { class: "pair" });
  const powerBox = h("div");
  const heatBox = h("div");
  pair.append(powerBox, heatBox);
  panel.append(pair);
  powerBox.append(h("h3", { text: "Leistung" }));
  heatBox.append(h("h3", { text: "Temperatur" }));
  const power = chartBlock(powerBox, { series: [{ label: "Leistung", color: "--s-gpu", value: point => entry(point)?.[3] }], format: "watts", label: "Verlauf der GPU-Leistung", compact: true });
  const heat = chartBlock(heatBox, { series: [{ label: "Temperatur", color: "--s-gpu", value: point => entry(point)?.[4] }], format: "celsius", label: "Verlauf der GPU-Temperatur", compact: true });
  const stats = h("dl", { class: "stats wide" });
  const top = h("div", { class: "mini-list" });
  panel.append(stats, h("h3", { text: "Prozesse auf dieser GPU" }), top);
  return context => {
    load(context);
    power(context);
    heat(context);
    const { s } = context;
    const gpu = s.gpus.find(item => item.index === index);
    if (!gpu) return;
    statRows(stats, [
      ...gpuStats(gpu),
      ["VRAM", `${bytes(gpu.memory_used)} von ${bytes(gpu.memory_total)}${gpu.memory_reserved ? ` · ${bytes(gpu.memory_reserved)} reserviert` : ""}`],
      ["Speicher-Controller", percent(gpu.memory_utilization)],
      ["Video kodieren / dekodieren", `${percent(gpu.encoder)} / ${percent(gpu.decoder)}`],
      ["Max. Takt", `${mhz(gpu.clock_graphics_max)} · Speicher ${mhz(gpu.clock_memory_max)}`],
      ["Architektur", gpu.architecture ? `${gpu.architecture}${gpu.cores ? ` · ${integer(gpu.cores)} Kerne` : ""}` : DASH],
      ["Treiber", s.gpu_driver ? `${s.gpu_driver}${s.gpu_cuda ? ` · CUDA ${s.gpu_cuda}` : ""}` : DASH],
      ["PCIe-Wiederholungen", integer(gpu.pcie_replays), "Übertragungen, die der Bus wegen Fehlern wiederholen musste"],
    ]);
    const names = new Map(s.processes.map(process => [process.pid, process.name]));
    keyed(top, [...gpu.processes].sort((a, b) => (b.vram ?? 0) - (a.vram ?? 0)), process => process.pid, () => h("div", { class: "sensor" }, h("span"), h("strong")), (row, process) => {
      setText(row.firstChild, `${names.get(process.pid) ?? "Prozess nicht lesbar"} · PID ${process.pid}`);
      setText(row.lastChild, `${bytes(process.vram)} · ${percent(process.utilization)}`);
    });
  };
}

function detailFor(id, panel) {
  const [kind, name] = id.split(":");
  if (kind === "cpu") return cpuDetail(panel);
  if (kind === "ram") return ramDetail(panel);
  if (kind === "swap") return swapDetail(panel);
  if (kind === "disk") return diskDetail(panel, name);
  if (kind === "gpu") return gpuDetail(panel, Number(name));
  if (kind === "net") return name ? networkDetail(panel, name) : networkTotals(panel);
  return null;
}

export class PerformanceView {
  constructor(host) {
    this.list = h("div", { class: "perf-list", role: "listbox", "aria-label": "Ressourcen", "aria-orientation": "vertical", tabindex: "-1" });
    this.title = h("h2");
    this.subtitle = h("p", { class: "perf-subtitle" });
    this.panel = h("div", { class: "perf-panel" });
    this.virtual = h("label", { class: "toggle" }, h("input", { type: "checkbox", onchange: event => { settings.showVirtual = event.target.checked; saveSettings(); } }), h("span", { text: "Virtuelle Schnittstellen" }));
    const side = h("aside", { class: "perf-side" }, this.list, this.virtual);
    const detail = h("section", { class: "perf-detail", "aria-live": "off" }, h("header", { class: "perf-head" }, h("div", null, this.title, this.subtitle)), this.panel);
    host.append(side, detail);
    this.selected = null;
    this.drawDetail = null;
    this.list.addEventListener("keydown", event => this.onKey(event));
  }

  /** Up and down walk the list; the button with focus shows at once. */
  onKey(event) {
    const items = [...this.list.children];
    const at = items.indexOf(document.activeElement);
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
    if (!step || at < 0) return;
    event.preventDefault();
    const next = items[Math.max(0, Math.min(items.length - 1, at + step))];
    next.focus();
    next.click();
  }

  select(id) {
    if (this.selected === id) return;
    this.selected = id;
    disposeChartsIn(this.panel);
    this.panel.replaceChildren();
    this.drawDetail = detailFor(id, this.panel);
    if (!this.drawDetail) this.panel.append(h("p", { class: "muted-note", text: "Diese Ressource gibt es nicht mehr." }));
  }

  render(context, wanted) {
    const { s } = context;
    this.virtual.firstChild.checked = settings.showVirtual;
    const items = resources(s);
    const id = items.some(item => item.id === wanted) ? wanted : wanted?.startsWith("net:") || wanted?.startsWith("disk:") ? wanted : "cpu";
    this.select(id);
    const spark = recentPoints(LIST_SECONDS);
    keyed(this.list, items, item => item.id, item => {
      const button = h("button", { class: "perf-item", type: "button", role: "option", onclick: () => navigate(`#leistung/${item.id}`) }, h("span", { class: "perf-name" }, h("strong"), h("small")), h("span", { class: "perf-line" }), h("canvas", { class: "perf-spark", "aria-hidden": "true" }));
      return button;
    }, (button, item) => {
      const [name, line, canvas] = button.children;
      setText(name.firstChild, item.title);
      setText(name.lastChild, item.detail ?? "");
      setText(line, item.line());
      const selected = item.id === this.selected;
      if (selected && !button.classList.contains("selected")) {
        // In the phone's strip the chosen resource may be off to the side.
        requestAnimationFrame(() => button.scrollIntoView({ block: "nearest", inline: "nearest" }));
      }
      button.classList.toggle("selected", selected);
      button.setAttribute("aria-selected", String(selected));
      drawSparkline(canvas, spark, item.value, { color: item.color, max: item.max });
    });
    const shown = items.find(item => item.id === this.selected);
    setText(this.title, shown?.title ?? this.selected);
    setText(this.subtitle, shown?.detail ?? shown?.line() ?? "");
    this.drawDetail?.(context);
  }
}
