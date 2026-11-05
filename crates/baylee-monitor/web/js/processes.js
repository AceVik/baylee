/*
 * The process table: sorting, filtering, grouping, column choice and a detail
 * panel. It only displays: nothing here can reach a process. Per-process network
 * traffic does not exist in /proc, so the table shows open sockets instead.
 */

import { h, icon, keyed, setText, setVar } from "./dom.js";
import { bytes, integer, percent, rate, span, time } from "./format.js";
import { data } from "./data.js";
import { saveSettings, settings } from "./store.js";

const STATE_WORDS = { R: "läuft", S: "schläft", D: "wartet (I/O)", Z: "Zombie", T: "gestoppt", I: "leerlauf", "?": "unbekannt" };

function gpuValue(process, key) {
  const values = process.gpu.map(entry => entry[key]);
  if (!values.length || values.some(value => value == null)) return null;
  // VRAM adds up over devices; utilization is per device, so the highest counts.
  return key === "vram" ? values.reduce((sum, value) => sum + value, 0) : Math.max(...values);
}

/** Process CPU, scaled to the viewer's choice of what 100 % means. */
const cpuOf = process => (settings.cpuScale === "all" && data.facts ? process.cpu / data.facts.threads : process.cpu);
const runtime = (process, now) => (process.started_secs ? now - process.started_secs * 1000 : null);

export const COLUMNS = [
  { id: "name", label: "Prozess", fixed: true, sort: p => p.name.toLowerCase() },
  { id: "pid", label: "PID", sort: p => p.pid, text: p => (p.count > 1 ? "—" : p.pid) },
  { id: "state", label: "Status", sort: p => p.state, text: p => (p.count > 1 ? `${p.count}×` : STATE_WORDS[p.state] ?? p.state), hint: "Zustand laut Kernel" },
  { id: "cpu", label: "CPU", sort: cpuOf, text: p => percent(cpuOf(p)), hint: "100 % = ein logischer Kern (in den Einstellungen umschaltbar)" },
  { id: "ram", label: "RAM", sort: p => p.ram, text: p => bytes(p.ram), hint: "Residenter Speicher (RSS); gemeinsame Seiten können mehrfach zählen" },
  { id: "swap", label: "Swap", sort: p => p.swap ?? -1, text: p => (p.swap == null ? "—" : bytes(p.swap)) },
  { id: "gpu", label: "GPU-Zeit", sort: p => gpuValue(p, "utilization") ?? -1, text: p => percent(gpuValue(p, "utilization")), hint: "Letzte Einzelmessung des Treibers; fehlend = unbekannt" },
  { id: "vram", label: "VRAM", sort: p => gpuValue(p, "vram") ?? -1, text: p => bytes(gpuValue(p, "vram")) },
  { id: "read", label: "Lesen", sort: p => p.read_bps, text: p => rate(p.read_bps), hint: "Nur für eigene Prozesse lesbar" },
  { id: "write", label: "Schreiben", sort: p => p.write_bps, text: p => rate(p.write_bps), hint: "Nur für eigene Prozesse lesbar" },
  { id: "threads", label: "Threads", sort: p => p.threads ?? -1, text: p => integer(p.threads) },
  { id: "sockets", label: "Sockets", sort: p => p.sockets ?? -1, text: p => integer(p.sockets), hint: "Offene Sockets, nur für eigene Prozesse. Netzwerkverkehr je Prozess ist ohne Rechte nicht messbar." },
  { id: "runtime", label: "Laufzeit", sort: (p, now) => runtime(p, now) ?? -1, text: (p, now) => span(runtime(p, now)) },
];

/** Sums all processes of one name, as a task manager's app rows do. */
function grouped(processes) {
  const groups = new Map();
  for (const process of processes) {
    const group = groups.get(process.name);
    if (!group) {
      groups.set(process.name, { ...process, gpu: [...process.gpu], count: 1, pid: process.pid });
      continue;
    }
    group.count += 1;
    group.cpu += process.cpu;
    group.ram += process.ram;
    group.read_bps += process.read_bps;
    group.write_bps += process.write_bps;
    group.swap = process.swap == null ? group.swap : (group.swap ?? 0) + process.swap;
    group.threads = process.threads == null ? group.threads : (group.threads ?? 0) + process.threads;
    group.sockets = process.sockets == null ? group.sockets : (group.sockets ?? 0) + process.sockets;
    group.started_secs = group.started_secs && process.started_secs ? Math.min(group.started_secs, process.started_secs) : group.started_secs || process.started_secs;
    group.gpu.push(...process.gpu);
  }
  return [...groups.values()];
}

const isActive = p => p.cpu >= 0.5 || p.gpu.length || p.read_bps + p.write_bps > 1024;

export class ProcessTable {
  /** `compact` is the dashboard card: fewer columns, no detail panel. */
  constructor(host, { compact = false, limit = 60 }) {
    this.compact = compact;
    this.baseLimit = limit;
    this.limit = limit;
    this.query = "";
    this.selected = null;
    this.build(host);
  }

  columns() {
    const wanted = this.compact ? ["name", "cpu", "ram", "gpu", "vram"] : settings.processes.columns;
    return COLUMNS.filter(column => column.fixed || wanted.includes(column.id));
  }

  build(host) {
    this.search = h("input", { type: "search", placeholder: "Name oder PID", "aria-label": "Prozesse nach Name oder PID filtern", autocomplete: "off", spellcheck: "false", oninput: () => { this.query = this.search.value.trim().toLocaleLowerCase(); this.limit = this.baseLimit; this.refresh(); } });
    this.filters = h("div", { class: "segmented", role: "group", "aria-label": "Prozessfilter" }, [["all", "Alle"], ["gpu", "GPU"], ["active", "Aktiv"]].map(([value, label]) =>
      h("button", { type: "button", "data-filter": value, text: label, onclick: () => { settings.processes.filter = value; this.limit = this.baseLimit; saveSettings(); this.refresh(); } })));
    this.group = h("label", { class: "toggle" }, h("input", { type: "checkbox", onchange: event => { settings.processes.group = event.target.checked; saveSettings(); this.refresh(); } }), h("span", { text: "Nach Name gruppieren" }));
    const toolbar = h("div", { class: "process-toolbar" }, this.filters, h("label", { class: "search" }, icon("search"), this.search), this.compact ? null : this.group, this.compact ? null : this.columnMenu());
    this.head = h("tr");
    this.body = h("tbody");
    this.table = h("table", { class: "process-table" }, h("thead", null, this.head), this.body);
    this.scroll = h("div", { class: "table-scroll", tabindex: "0", role: "region", "aria-label": "Prozessliste, scrollbar" }, this.table);
    this.more = h("button", { class: "text-button", type: "button", hidden: true, text: "Mehr anzeigen", onclick: () => { this.limit += 100; this.refresh(); } });
    this.caption = h("span");
    host.append(toolbar, this.scroll, h("div", { class: "process-footer" }, this.caption, this.more));
    if (!this.compact) {
      this.detail = h("div", { class: "process-detail", hidden: true });
      host.append(this.detail, h("p", { class: "muted-note", text: "RAM = residenter Speicher (RSS), gemeinsam genutzte Seiten können mehrfach zählen. Lesen, Schreiben und Sockets sind nur für Prozesse dieses Benutzers lesbar. Netzwerkverkehr je Prozess ist ohne erhöhte Rechte nicht messbar, deshalb zeigt Pulse offene Sockets statt erfundener Datenraten." }));
    }
    this.buildHead();
  }

  columnMenu() {
    const items = COLUMNS.filter(column => !column.fixed).map(column =>
      h("label", { class: "toggle" }, h("input", { type: "checkbox", "data-column": column.id, onchange: event => this.toggleColumn(column.id, event.target.checked) }), h("span", { text: column.label })));
    this.menu = h("details", { class: "menu" }, h("summary", { class: "pill-button" }, icon("columns"), "Spalten"), h("div", { class: "menu-panel" }, items));
    return this.menu;
  }

  toggleColumn(id, on) {
    const columns = new Set(settings.processes.columns);
    if (on) columns.add(id);
    else columns.delete(id);
    settings.processes.columns = COLUMNS.map(column => column.id).filter(name => columns.has(name) || name === "name");
    saveSettings();
    this.buildHead();
    this.refresh();
  }

  /** Header buttons, with the arrow on the sorted column. */
  buildHead() {
    const columns = this.columns();
    this.shownColumns = columns.map(column => column.id).join(",");
    this.head.replaceChildren(...columns.map(column =>
      h("th", { scope: "col", class: column.id === "name" ? "" : "num", title: column.hint, "data-column": column.id },
        h("button", { type: "button", "data-sort": column.id, text: column.label, onclick: () => this.sortBy(column.id) }))));
    this.body.replaceChildren();
    this.body.keyedNodes = null;
    this.markSort();
  }

  sortBy(id) {
    const sorting = settings.processes;
    sorting.descending = sorting.sort === id ? !sorting.descending : id !== "name";
    sorting.sort = id;
    saveSettings();
    this.refresh();
  }

  markSort() {
    const { sort, descending } = settings.processes;
    for (const cell of this.head.children) {
      const active = cell.dataset.column === sort;
      const button = cell.firstChild;
      const label = COLUMNS.find(column => column.id === cell.dataset.column).label;
      setText(button, active ? `${label} ${descending ? "↓" : "↑"}` : label);
      if (active) cell.setAttribute("aria-sort", descending ? "descending" : "ascending");
      else cell.removeAttribute("aria-sort");
    }
  }

  markControls() {
    const { filter, group, columns } = settings.processes;
    for (const button of this.filters.children) button.setAttribute("aria-pressed", String(button.dataset.filter === filter));
    this.group.firstChild.checked = group;
    for (const box of this.menu?.querySelectorAll("input") ?? []) box.checked = columns.includes(box.dataset.column);
    if (this.columns().map(column => column.id).join(",") !== this.shownColumns) this.buildHead();
    else this.markSort();
  }

  visible(snapshot) {
    const { filter, group, sort, descending } = settings.processes;
    const now = snapshot.point.at_ms;
    const query = this.query;
    let list = snapshot.processes.filter(process =>
      (!query || `${process.name} ${process.pid}`.toLocaleLowerCase().includes(query))
      && (filter !== "gpu" || process.gpu.length)
      && (filter !== "active" || isActive(process)));
    if (group && !this.compact) list = grouped(list);
    const column = COLUMNS.find(entry => entry.id === sort) ?? COLUMNS[3];
    const direction = descending ? -1 : 1;
    return list.sort((a, b) => {
      const x = column.sort(a, now);
      const y = column.sort(b, now);
      const order = typeof x === "string" ? x.localeCompare(y) : x - y;
      return direction * order || a.pid - b.pid;
    });
  }

  render(snapshot) {
    this.snapshot = snapshot;
    this.refresh();
  }

  refresh() {
    const snapshot = this.snapshot;
    this.markControls();
    if (!snapshot) return;
    const { group } = settings.processes;
    const list = this.visible(snapshot);
    const shown = list.slice(0, this.limit);
    const columns = this.columns();
    const now = snapshot.point.at_ms;
    keyed(
      this.body,
      shown,
      process => (group && !this.compact ? process.name : process.pid),
      () => this.createRow(columns),
      (row, process) => this.updateRow(row, process, columns, now),
    );
    const empty = !shown.length;
    this.body.querySelector(".empty-row")?.remove();
    if (empty) {
      const message = settings.processes.filter === "gpu" ? "Keine GPU-Prozesse gemeldet" : "Keine passenden Prozesse";
      this.body.append(h("tr", { class: "empty-row" }, h("td", { class: "empty", colspan: String(columns.length), text: message })));
    }
    const scale = settings.cpuScale === "all" ? "100 % CPU = alle Kerne" : "100 % CPU = ein Kern";
    setText(this.caption, `${shown.length} von ${list.length} · ${scale}`);
    this.more.hidden = list.length <= this.limit;
    if (this.detail) this.renderDetail(snapshot);
  }

  createRow(columns) {
    const name = h("span", { class: "proc-name" });
    const sub = h("small");
    const cells = columns.map(column => (column.id === "name" ? h("td", null, name, sub) : h("td", { class: `num col-${column.id}` })));
    const row = h("tr", null, cells);
    row.parts = { name, sub, cells };
    if (!this.compact) {
      row.tabIndex = 0;
      row.addEventListener("click", () => this.select(row.process));
      row.addEventListener("keydown", event => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          this.select(row.process);
        }
      });
    }
    return row;
  }

  updateRow(row, process, columns, now) {
    row.process = process;
    const { name, sub, cells } = row.parts;
    setText(name, process.name);
    name.title = process.name;
    const count = process.count > 1 ? `${process.count} Prozesse` : `PID ${process.pid}`;
    const gpu = process.gpu.length ? ` · GPU ${[...new Set(process.gpu.map(entry => entry.index))].join(", ")}${process.gpu.some(entry => entry.compute) ? " · CUDA" : " · Grafik"}` : "";
    setText(sub, count + gpu);
    sub.classList.toggle("process-gpu", process.gpu.length > 0);
    columns.forEach((column, at) => {
      if (column.id === "name") return;
      const cell = cells[at];
      setText(cell, column.text(process, now));
      if (column.id === "cpu") setVar(cell, "--load", `${Math.min(100, cpuOf(process) / (settings.cpuScale === "all" ? 1 : 2))}%`);
    });
    row.classList.toggle("selected", this.selected === (settings.processes.group ? process.name : process.pid));
    row.classList.toggle("stuck", process.state === "D" || process.state === "Z");
  }

  select(process) {
    const key = settings.processes.group ? process.name : process.pid;
    this.selected = this.selected === key ? null : key;
    this.refresh();
    if (this.selected != null) this.detail.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }

  renderDetail(snapshot) {
    const group = settings.processes.group;
    const process = this.selected == null ? null : group ? this.visible(snapshot).find(entry => entry.name === this.selected) : snapshot.processes.find(entry => entry.pid === this.selected);
    this.detail.hidden = this.selected == null;
    if (this.selected == null) return;
    if (!process) {
      this.detail.replaceChildren(h("p", { class: "muted-note", text: "Der Prozess ist beendet." }));
      return;
    }
    const now = snapshot.point.at_ms;
    const rows = [
      ["PID", process.count > 1 ? `${process.count} Prozesse` : process.pid],
      ["Eltern-PID", process.parent ?? "—"],
      ["Status", STATE_WORDS[process.state] ?? process.state],
      ["Gestartet", process.started_secs ? `${time(process.started_secs * 1000)} (vor ${span(runtime(process, now))})` : "—"],
      ["CPU", percent(cpuOf(process))],
      ["RAM", bytes(process.ram)],
      ["Swap", process.swap == null ? "—" : bytes(process.swap)],
      ["Threads", integer(process.threads)],
      ["Offene Sockets", process.sockets == null ? "nicht lesbar" : integer(process.sockets)],
      ["Lesen", `${rate(process.read_bps)} · gesamt ${bytes(process.read_total)}`],
      ["Schreiben", `${rate(process.write_bps)} · gesamt ${bytes(process.write_total)}`],
      ...process.gpu.map(entry => [`GPU ${entry.index}`, `${percent(entry.utilization)} · VRAM ${bytes(entry.vram)} · ${entry.compute ? "CUDA" : "Grafik"}`]),
    ];
    this.detail.replaceChildren(
      h("div", { class: "panel-heading" }, h("h3", { text: process.name }), h("button", { class: "icon-button", type: "button", "aria-label": "Details schließen", onclick: () => { this.selected = null; this.refresh(); } }, icon("close"))),
      h("dl", { class: "stats" }, rows.map(([key, value]) => h("div", { class: "kv" }, h("dt", { text: key }), h("dd", { text: String(value) })))),
    );
  }
}

/** The processes with the most swap, for the swap views. */
export function topSwapUsers(snapshot, count = 5) {
  return snapshot.processes
    .filter(process => process.swap > 0)
    .sort((a, b) => b.swap - a.swap)
    .slice(0, count);
}
