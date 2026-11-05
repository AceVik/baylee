/* The storage tree: each physical disk with its partitions under it. */

import { h, icon, keyed, meter, meterBar, setText } from "./dom.js";
import { bytes, celsius, number, pcieLink, rate, span } from "./format.js";
import { settings } from "./store.js";

const HOUR_SECS = 3600;

/**
 * How urgent a filesystem's state is: "", "warn" or "crit". The one place that
 * decides this, for the disk rows and for the alerts alike.
 *
 * An alarm that cries wolf gets ignored, so two rules hold back. The floor in
 * GiB only counts on a disk already past the warning level: a 1 GiB /boot/efi
 * is always below it and always fine. And a forecast only counts on a disk that
 * is fairly full: a large copy onto an empty disk fills it "in four hours" only
 * if it never stops.
 */
export function usageLevel(usage, readOnly) {
  if (!usage || readOnly || !usage.total) return "";
  const { diskWarn, diskCrit, diskFreeGiB } = settings.alerts;
  const used = usage.used_pct;
  const forecast = usage.full_in_secs;
  const belowFloor = usage.available / 2 ** 30 < diskFreeGiB;
  if (used >= diskCrit || (used >= diskWarn && belowFloor)) return "crit";
  if (forecast != null && used >= 70 && forecast < 6 * HOUR_SECS) return "crit";
  if (used >= diskWarn) return "warn";
  if (forecast != null && used >= 50 && forecast < 24 * HOUR_SECS) return "warn";
  return "";
}

const LEVEL_WORDS = { crit: "Kritisch", warn: "Warnung" };

function levelChip(level) {
  return level ? h("span", { class: `chip ${level}` }, icon(level === "crit" ? "alert" : "warn"), LEVEL_WORDS[level]) : null;
}

/** Filesystem, label and partition name, in the order a person looks for. */
function partitionTitle(partition) {
  return partition.label ?? partition.part_name ?? partition.mount ?? "Ohne Namen";
}

function trendText(usage) {
  if (!usage) return "";
  if (usage.full_in_secs != null) return `Voll in ca. ${span(usage.full_in_secs * 1000)}`;
  if (usage.filling_bps != null && Math.abs(usage.filling_bps) > 1024 * 16) {
    return usage.filling_bps > 0 ? `Füllt sich mit ${bytes(usage.filling_bps * 3600)}/Std.` : `Leert sich mit ${bytes(-usage.filling_bps * 3600)}/Std.`;
  }
  return "";
}

function createPartition() {
  const row = h("div", { class: "part" });
  const bar = meterBar();
  row.parts = {
    title: h("strong"),
    device: h("code"),
    tags: h("div", { class: "part-tags" }),
    usage: h("span", { class: "part-usage" }),
    bar,
    detail: h("small", { class: "part-detail" }),
    io: h("small", { class: "part-io" }),
  };
  const { title, device, tags, usage, detail, io } = row.parts;
  row.append(
    h("div", { class: "part-head" }, h("div", { class: "part-name" }, title, device), usage),
    tags,
    bar.node,
    h("div", { class: "part-foot" }, detail, io),
  );
  return row;
}

function updatePartition(row, partition) {
  const { title, device, tags, usage: usageNode, bar, detail, io } = row.parts;
  const usage = partition.usage;
  const level = usageLevel(usage, partition.read_only);
  setText(title, partitionTitle(partition));
  title.title = [partition.label, partition.part_name].filter(Boolean).join(" · ");
  setText(device, partition.name);
  const labels = [
    partition.fs_type,
    partition.mount ?? "nicht eingehängt",
    partition.read_only ? "nur lesen" : null,
    partition.more_mounts ? `+${partition.more_mounts} Mounts` : null,
    partition.mount == null ? bytes(partition.size) : null,
  ].filter(Boolean);
  const key = labels.join("|") + level;
  if (tags.dataset.key !== key) {
    tags.dataset.key = key;
    tags.replaceChildren(...labels.map(label => h("span", { class: "tag", text: label })), ...[levelChip(level)].filter(Boolean));
  }
  row.classList.toggle("crit", level === "crit");
  row.classList.toggle("warn", level === "warn");
  const quiet = !partition.mount;
  row.classList.toggle("quiet", quiet);
  bar.node.hidden = !usage;
  if (usage) {
    meter(bar.fill, usage.used_pct, level);
    setText(usageNode, `${bytes(usage.used)} von ${bytes(usage.total)}`);
    const free = `${bytes(usage.available)} frei · ${number(usage.used_pct)} %`;
    const inodes = usage.inodes_pct != null && usage.inodes_pct >= 50 ? ` · Inodes ${number(usage.inodes_pct)} %` : "";
    const trend = trendText(usage);
    setText(detail, `${free}${inodes}${trend ? ` · ${trend}` : ""}`);
  } else {
    setText(usageNode, "");
    setText(detail, partition.mount ? "Füllstand nicht lesbar" : "");
  }
  // A partition that is not mounted only earns a rate when something touches it.
  const busy = partition.io && partition.io.read_bps + partition.io.write_bps > 0;
  setText(io, partition.io && (busy || !quiet) ? `↓ ${rate(partition.io.read_bps)} · ↑ ${rate(partition.io.write_bps)}` : "");
}

function createDisk() {
  const parts = h("div", { class: "parts" });
  const card = h("article", { class: "disk" });
  card.parts = { title: h("strong"), facts: h("small", { class: "disk-facts" }), temp: h("span", { class: "disk-temp" }), rates: h("small", { class: "disk-rates" }), list: parts };
  const { title, facts, temp, rates } = card.parts;
  card.append(
    h("header", { class: "disk-head" }, icon("disk"), h("div", { class: "disk-title" }, title, facts), temp),
    rates,
    parts,
  );
  return card;
}

function updateDisk(card, disk) {
  const { title, facts, temp, rates, list } = card.parts;
  setText(title, disk.model);
  const kind = disk.transport === "NVMe" ? "NVMe-SSD" : `${disk.transport} ${disk.rotational ? "HDD" : "SSD"}`;
  const link = disk.link ? ` · ${pcieLink(disk.link)}` : "";
  setText(facts, `${disk.name} · ${bytes(disk.size)} · ${kind}${link}${disk.removable ? " · wechselbar" : ""}`);
  setText(temp, disk.temperature == null ? "" : celsius(disk.temperature));
  temp.classList.toggle("hot", disk.temperature != null && disk.temperature >= settings.alerts.diskTempWarn);
  setText(rates, disk.io ? `Lesen ${rate(disk.io.read_bps)} · Schreiben ${rate(disk.io.write_bps)} · Auslastung ${number(disk.io.busy_pct)} %` : "");
  if (!disk.partitions.length) {
    if (!list.querySelector(".muted-note")) list.replaceChildren(h("p", { class: "muted-note", text: "Keine Partitionen" }));
    list.keyedNodes = null;
    return;
  }
  list.querySelector(".muted-note")?.remove();
  keyed(list, disk.partitions, partition => partition.name, createPartition, updatePartition);
}

function createVolume() {
  const row = createPartition();
  row.classList.add("volume");
  return row;
}

function updateVolume(row, volume) {
  updatePartition(row, {
    name: volume.fs_type,
    label: volume.mount,
    fs_type: null,
    mount: volume.mount,
    read_only: volume.read_only,
    usage: volume.usage,
    size: volume.usage?.total ?? 0,
    more_mounts: 0,
    io: null,
  });
}

export function renderDisks(host, storage) {
  if (!host.hostParts) {
    host.hostParts = { disks: h("div", { class: "disk-list" }), volumes: h("div", { class: "volumes" }), heading: h("h3", { text: "Weitere Dateisysteme" }), problem: h("p", { class: "muted-note" }) };
    host.append(host.hostParts.problem, host.hostParts.disks, host.hostParts.heading, host.hostParts.volumes);
  }
  const { disks, volumes, heading, problem } = host.hostParts;
  problem.hidden = !storage.problem;
  setText(problem, storage.problem ?? "");
  keyed(disks, storage.disks, disk => disk.name, createDisk, updateDisk);
  heading.hidden = !storage.volumes.length;
  keyed(volumes, storage.volumes, volume => volume.mount, createVolume, updateVolume);
}

/** Every filesystem worth watching, however it is mounted. */
export function allFilesystems(storage) {
  const fromDisks = storage.disks.flatMap(disk =>
    disk.partitions
      .filter(partition => partition.usage)
      .map(partition => ({ mount: partition.mount, usage: partition.usage, readOnly: partition.read_only, device: `${disk.model}, ${partition.name}` })),
  );
  const loose = storage.volumes
    .filter(volume => volume.usage)
    .map(volume => ({ mount: volume.mount, usage: volume.usage, readOnly: volume.read_only, device: volume.fs_type }));
  return [...fromDisks, ...loose];
}
