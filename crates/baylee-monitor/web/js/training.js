/* Training panel: baylee-* system units and train_ops pipelines (read only). */

import { h, setText } from "./dom.js";
import { clock, span, time } from "./format.js";

const UNIT_WORDS = { active: "aktiv", activating: "startet", deactivating: "stoppt", inactive: "inaktiv", failed: "fehlgeschlagen", reloading: "lädt neu", maintenance: "Wartung" };
const STAGE_WORDS = { complete: "fertig", running: "läuft", quarantined: "isoliert", failed: "fehlgeschlagen" };
const FIELD_LABELS = { batches_per_s: "Batches/s", gpu_peak_gb: "GPU-Spitze GB", optimizer_updates: "Updates", gpu_alloc_retries: "Alloc-Retries" };
const STAGES_SHOWN = 5;

const expandedStages = new Set();
let shownChecked = -1;

function phaseClass(pipeline) {
  if (pipeline.problem) return "warn";
  if (pipeline.phase === "complete") return "ok";
  if (pipeline.phase === "failed" || pipeline.error) return "bad";
  return pipeline.controller_alive === false ? "warn" : "running";
}

function stageClass(stage) {
  if (stage.status === "running") return "running";
  if (stage.status === "complete" && (stage.returncode ?? 0) === 0) return "complete";
  return stage.status == null ? "" : "bad";
}

function stageText(stage, now) {
  const word = STAGE_WORDS[stage.status] ?? stage.status ?? "unbekannt";
  if (stage.status === "running" && stage.started_ms) return `${word} seit ${span(now - stage.started_ms)}`;
  const parts = [word];
  if (stage.started_ms && stage.finished_ms) parts.push(span(stage.finished_ms - stage.started_ms));
  if (stage.returncode != null && stage.returncode !== 0) parts.push(`Code ${stage.returncode}`);
  if (stage.reason) parts.push(stage.reason);
  return parts.join(" · ");
}

function progressBox(pipeline, now) {
  const progress = pipeline.progress;
  const age = progress.updated_ms ? `vor ${span(now - progress.updated_ms)}` : "—";
  const [, tag = "", rest = ""] = /^\[([^\]]+)\]\s*(.*)$/.exec(progress.line) ?? [];
  // Show the raw line only when it says more than its chips.
  const words = rest.split(/\s+/).filter(word => word && !progress.fields.some(([key, value]) => word === `${key}=${value}`));
  const box = h("div", { class: "progress-box" }, h("div", { class: "progress-title", text: `${progress.stage}${tag ? ` · ${tag}` : ""} · letzte Ausgabe ${age}` }));
  if (progress.fields.length) {
    box.append(h("div", { class: "chips" }, progress.fields.map(([key, value]) => h("span", null, h("small", { text: FIELD_LABELS[key] ?? key }), value))));
  }
  if (words.length || !progress.fields.length) box.append(h("code", { class: "progress-line", text: progress.line }));
  return box;
}

function pipelineCard(pipeline, training, now) {
  const card = h("article", { class: "pipeline" });
  card.append(h("div", { class: "pipeline-head" }, h("strong", { text: pipeline.name }), h("span", { class: `phase ${phaseClass(pipeline)}`, text: pipeline.problem ? "nicht lesbar" : pipeline.phase ?? "—" })));
  const meta = [];
  if (pipeline.controller_pid != null) {
    meta.push(pipeline.controller_alive === true ? "Controller läuft" : pipeline.controller_alive === false ? "Controller beendet" : "Controller unbekannt");
    const unit = training.units.find(entry => entry.gone_ms == null && entry.main_pid === pipeline.controller_pid);
    if (unit) meta.push(unit.name);
  }
  if (pipeline.updated_ms) meta.push(`Status von ${clock(pipeline.updated_ms, now)}`);
  if (pipeline.promoted) meta.push("übernommen");
  if (meta.length) card.append(h("p", { class: "pipeline-meta", text: meta.join(" · ") }));
  for (const problem of [pipeline.problem, pipeline.error]) if (problem) card.append(h("p", { class: "pipeline-error", text: problem }));
  if (pipeline.progress) card.append(progressBox(pipeline, now));
  if (pipeline.stages.length) {
    const expanded = expandedStages.has(pipeline.name);
    const stages = expanded ? pipeline.stages : pipeline.stages.slice(-STAGES_SHOWN);
    card.append(h("ol", { class: "stages" }, stages.map(stage => h("li", { class: `stage ${stageClass(stage)}` }, h("span", { class: "stage-name", text: stage.name }), h("span", { class: "stage-state", text: stageText(stage, now) })))));
    if (pipeline.stages.length > STAGES_SHOWN) {
      card.append(h("button", { class: "text-button", type: "button", "data-pipeline": pipeline.name, text: expanded ? "Weniger Stufen" : `Alle ${pipeline.stages.length} Stufen` }));
    }
  }
  return card;
}

function unitItem(unit, now) {
  const gone = unit.gone_ms != null;
  const state = gone ? "gone" : unit.active_state === "failed" ? "failed" : unit.active_state === "active" ? "active" : "";
  const details = [gone ? "beendet" : `${UNIT_WORDS[unit.active_state] ?? unit.active_state} · ${unit.sub_state}`];
  if (unit.result && unit.result !== "success") details.push(`Ergebnis ${unit.result}`);
  if (!gone && unit.active_state === "active" && unit.main_pid) details.push(`PID ${unit.main_pid}`);
  const when = h("div", { class: "when" });
  if (gone) {
    const ended = unit.inactive_since_ms != null && unit.inactive_since_ms >= (unit.active_since_ms ?? 0) ? unit.inactive_since_ms : unit.gone_ms;
    when.append(h("span", { text: `bis ${clock(ended, now)}` }));
    if (unit.active_since_ms) when.append(h("small", { text: span(ended - unit.active_since_ms) }));
  } else if (unit.active_state === "active" && unit.active_since_ms) {
    when.append(h("span", { text: `seit ${clock(unit.active_since_ms, now)}` }), h("small", { text: span(now - unit.active_since_ms) }));
  } else if (unit.inactive_since_ms) {
    when.append(h("span", { text: `seit ${clock(unit.inactive_since_ms, now)}` }));
  }
  return h("li", { class: `unit ${state}`.trim() }, h("div", null, h("strong", { text: unit.name }), h("small", { text: details.join(" · ") })), when);
}

export function buildTraining(body, setTag) {
  const pipelines = h("div", { class: "pipeline-list" }, h("p", { class: "muted-note", text: "Warte auf Messwerte …" }));
  const problem = h("p", { class: "muted-note", hidden: true });
  const units = h("ul", { class: "unit-list" });
  body.append(h("div", { class: "training-grid" }, pipelines, h("div", { class: "units-block" }, h("h3", null, "Systemdienste ", h("span", { class: "count", text: "baylee-*" })), problem, units)));
  pipelines.addEventListener("click", event => {
    const button = event.target.closest("button[data-pipeline]");
    if (!button) return;
    const name = button.dataset.pipeline;
    if (expandedStages.has(name)) expandedStages.delete(name);
    else expandedStages.add(name);
    shownChecked = -1;
  });
  return ({ training, s }) => {
    if (!training || training.checked_ms === shownChecked) return;
    shownChecked = training.checked_ms;
    const now = training.checked_ms || s.point.at_ms;
    setTag(training.checked_ms ? `Stand ${time(training.checked_ms)}` : "Nur Anzeige");
    const cards = training.pipelines.map(pipeline => pipelineCard(pipeline, training, now));
    if (!cards.length) cards.push(h("p", { class: "muted-note", text: "Keine Pipeline konfiguriert. Mit --pipeline <Verzeichnis> zeigt Pulse eine train_ops-Pipeline an." }));
    pipelines.replaceChildren(...cards);
    problem.hidden = !training.units_problem;
    setText(problem, training.units_problem ?? "");
    const items = training.units.map(unit => unitItem(unit, now));
    if (!items.length) items.push(h("li", { class: "unit empty", text: training.checked_ms ? "Keine baylee-* Systemdienste bekannt." : "Warte auf systemd …" }));
    units.replaceChildren(...items);
  };
}
