/* Training panel: train_ops pipelines, LLM game runs, the ladder and baylee-* system units (read only). */

import { h, meter, meterBar, setText } from "./dom.js";
import { clock, integer, number, percent, span, time } from "./format.js";

const UNIT_WORDS = { active: "aktiv", activating: "startet", deactivating: "stoppt", inactive: "inaktiv", failed: "fehlgeschlagen", reloading: "lädt neu", maintenance: "Wartung" };
const STAGE_WORDS = { complete: "fertig", running: "läuft", quarantined: "isoliert", failed: "fehlgeschlagen", pending: "wartet" };
const FIELD_LABELS = { step: "Schritt", loss: "Loss", epoch: "Epoche", batches_per_s: "Batches/s", gpu_peak_gb: "GPU-Spitze GB", optimizer_updates: "Updates", gpu_alloc_retries: "Alloc-Retries" };
const PHASE_WORDS = { collecting: "sammelt", stopped: "gestoppt", failed: "fehlgeschlagen", complete: "fertig", interrupted: "unterbrochen" };
const STAGES_SHOWN = 5;

const expandedStages = new Set();
let shownChecked = -1;

const ago = (ms, now) => (ms ? `vor ${span(now - ms)}` : "—");
const winRate = (won, lost) => (won + lost > 0 ? percent((100 * won) / (won + lost)) : "—");

function phaseClass(pipeline) {
  if (pipeline.problem) return "warn";
  if (pipeline.phase === "complete") return "ok";
  if (pipeline.phase === "failed" || pipeline.error) return "bad";
  return pipeline.active ? "running" : "warn";
}

function stageClass(stage) {
  if (stage.status === "running") return "running";
  if (stage.status === "pending") return "";
  if (stage.status === "complete" && (stage.returncode ?? 0) === 0) return "complete";
  return stage.status == null ? "" : "bad";
}

function isBad(stage) {
  return stageClass(stage) === "bad";
}

function evalText(result) {
  const parts = [];
  if (result.net_wins != null) parts.push(result.deals ? `Siege ${integer(result.net_wins)}/${integer(result.deals)}` : `Siege ${integer(result.net_wins)}`);
  if (result.net_losses != null) parts.push(`Niederl. ${integer(result.net_losses)}`);
  if (result.net_win_rate != null) parts.push(percent(result.net_win_rate * 100));
  return parts.join(" · ");
}

function stageText(stage, now) {
  const word = STAGE_WORDS[stage.status] ?? stage.status ?? "unbekannt";
  if (stage.status === "running" && stage.started_ms) return `${word} seit ${span(now - stage.started_ms)}`;
  const parts = [word];
  if (stage.started_ms && stage.finished_ms) parts.push(span(stage.finished_ms - stage.started_ms));
  if (stage.returncode != null && stage.returncode !== 0) parts.push(`Code ${stage.returncode}`);
  if (stage.reason) parts.push(stage.reason);
  if (stage.eval) parts.push(evalText(stage.eval));
  return parts.join(" · ");
}

function bar(fraction) {
  const { node, fill } = meterBar("run-bar");
  meter(fill, fraction * 100);
  return node;
}

function progressBox(pipeline, now) {
  const progress = pipeline.progress;
  const running = pipeline.stages.find(stage => stage.name === progress.stage)?.status === "running";
  const [, tag = "", rest = ""] = /^\[([^\]]+)\]\s*(.*)$/.exec(progress.line) ?? [];
  // Show the raw line only when it says more than its chips.
  const words = rest.split(/\s+/).filter(word => word && !progress.fields.some(([key, value]) => word === `${key}=${value}`));
  const box = h("div", { class: "progress-box" }, h("div", { class: "progress-title", text: `${progress.stage}${tag ? ` · ${tag}` : ""} · letzte Ausgabe ${ago(progress.updated_ms, now)}` }));
  if (progress.total_steps) {
    const done = Math.min(1, (progress.step ?? 0) / progress.total_steps);
    const left = progress.remaining_s != null && running ? ` · noch ca. ${span(progress.remaining_s * 1000)}` : "";
    box.append(bar(done), h("p", { class: "progress-steps", text: `Schritt ${integer(progress.step)} von ${integer(progress.total_steps)} (${percent(done * 100)})${left}` }));
  }
  if (progress.fields.length) {
    box.append(h("div", { class: "chips" }, progress.fields.map(([key, value]) => h("span", null, h("small", { text: FIELD_LABELS[key] ?? key }), value))));
  }
  if (words.length || !progress.fields.length) box.append(h("code", { class: "progress-line", text: progress.line }));
  return box;
}

function pipelineMeta(pipeline, training, now) {
  const meta = [];
  if (pipeline.stage_total) meta.push(`Stufe ${pipeline.stage_current ?? pipeline.stage_total} von ${pipeline.stage_total}`);
  if (pipeline.started_ms) {
    if (pipeline.active) meta.push(`läuft seit ${span(now - pipeline.started_ms)}`);
    else if (pipeline.updated_ms) meta.push(`Dauer ${span(pipeline.updated_ms - pipeline.started_ms)}`);
  }
  if (pipeline.controller_pid != null) {
    meta.push(pipeline.controller_alive === true ? "Controller läuft" : pipeline.controller_alive === false ? "Controller beendet" : "Controller unbekannt");
    const unit = pipeline.unit ?? training.units.find(entry => entry.gone_ms == null && entry.main_pid === pipeline.controller_pid)?.name;
    if (unit) meta.push(unit);
  }
  if (pipeline.updated_ms) meta.push(`Status von ${clock(pipeline.updated_ms, now)}`);
  if (pipeline.promoted) meta.push("übernommen");
  return meta;
}

function stageList(pipeline, now) {
  const expanded = expandedStages.has(pipeline.name);
  // A finished pipeline lists only its failed stages until opened.
  const stages = expanded ? pipeline.stages : pipeline.active ? pipeline.stages.slice(-STAGES_SHOWN) : pipeline.stages.filter(isBad).slice(-STAGES_SHOWN);
  const nodes = [];
  if (stages.length) {
    nodes.push(
      h("ol", { class: "stages" }, stages.map(stage => h("li", { class: `stage ${stageClass(stage)}` }, h("span", { class: "stage-name", text: stage.name }), h("span", { class: "stage-state", text: stageText(stage, now) }), stage.error_line ? h("span", { class: "stage-error", text: stage.error_line }) : null))),
    );
  }
  const hidden = pipeline.active ? pipeline.stages.length > STAGES_SHOWN : pipeline.stages.length > stages.length || expanded;
  if (hidden) {
    nodes.push(h("button", { class: "text-button", type: "button", "data-pipeline": pipeline.name, text: expanded ? "Weniger Stufen" : `Alle ${pipeline.stages.length} Stufen` }));
  }
  return nodes;
}

function pipelineCard(pipeline, training, now) {
  const card = h("article", { class: `pipeline${pipeline.active ? " live" : ""}` });
  card.append(h("div", { class: "pipeline-head" }, h("strong", { text: pipeline.name }), h("span", { class: `phase ${phaseClass(pipeline)}`, text: pipeline.problem ? "nicht lesbar" : pipeline.phase ?? "—" })));
  const meta = pipelineMeta(pipeline, training, now);
  if (meta.length) card.append(h("p", { class: "pipeline-meta", text: meta.join(" · ") }));
  for (const problem of [pipeline.problem, pipeline.error]) if (problem) card.append(h("p", { class: "pipeline-error", text: problem }));
  if (pipeline.progress && pipeline.active) card.append(progressBox(pipeline, now));
  card.append(...stageList(pipeline, now));
  return card;
}

function runBadge(run) {
  if (run.problem) return ["nicht lesbar", "warn"];
  if (run.active) return [run.drain ? "läuft, drain" : "läuft", "running"];
  if (run.controller === "failed" || run.phase === "failed") return ["fehlgeschlagen", "bad"];
  return [PHASE_WORDS[run.phase] ?? run.phase ?? "—", run.phase === "stopped" ? "ok" : "warn"];
}

function llmCard(run, now) {
  const [badge, badgeClass] = runBadge(run);
  const card = h("article", { class: `pipeline${run.active ? " live" : ""}` });
  card.append(h("div", { class: "pipeline-head" }, h("strong", { text: run.name }), h("span", { class: `phase ${badgeClass}`, text: badge })));
  const meta = [];
  if (run.active && run.started_ms) meta.push(`läuft seit ${span(now - run.started_ms)}`);
  else if (run.started_ms && run.ended_ms) meta.push(`Dauer ${span(run.ended_ms - run.started_ms)}`);
  if (run.unit) meta.push(run.unit);
  if (run.updated_ms) meta.push(`Stand ${ago(run.updated_ms, now)}`);
  if (run.stop) meta.push("Stopp angefordert");
  else if (run.drain && !run.active) meta.push("Drain gesetzt");
  if (meta.length) card.append(h("p", { class: "pipeline-meta", text: meta.join(" · ") }));
  if (run.problem) card.append(h("p", { class: "pipeline-error", text: run.problem }));
  if (run.assigned != null && run.max_attempts) {
    card.append(bar(Math.min(1, run.assigned / run.max_attempts)), h("p", { class: "progress-steps", text: `${integer(run.assigned)} von ${integer(run.max_attempts)} geplanten Spielen angenommen` }));
  }
  const chips = [];
  if (run.completed != null) chips.push(["fertig", integer(run.completed)]);
  if (run.in_flight != null) chips.push(["laufend", integer(run.in_flight)]);
  if (run.concurrency != null) chips.push(["parallel", integer(run.concurrency)]);
  if (run.errors) chips.push(["Fehler", integer(run.errors)]);
  if (run.incomplete) chips.push(["unvollständig", integer(run.incomplete)]);
  if (run.fallback_answers) chips.push(["Fallback-Antworten", integer(run.fallback_answers)]);
  if (run.worker_failures) chips.push(["Worker-Ausfälle", integer(run.worker_failures)]);
  if (chips.length) card.append(h("div", { class: "chips" }, chips.map(([label, value]) => h("span", null, h("small", { text: label }), value))));
  if (run.models.length) card.append(h("p", { class: "pipeline-meta", text: run.models.join(" · ") }));
  return card;
}

function ladderCard(ladder, now) {
  const card = h("article", { class: "pipeline" });
  const badge = ladder.problem ? ["nicht lesbar", "warn"] : ladder.in_flight ? [`${ladder.in_flight} laufen`, "running"] : [`${ladder.games} Spiele`, "ok"];
  card.append(h("div", { class: "pipeline-head" }, h("strong", { text: ladder.name || "Ladder" }), h("span", { class: `phase ${badge[1]}`, text: badge[0] })));
  const meta = [`${ladder.games} Spiele`];
  if (ladder.failures) meta.push(`${ladder.failures} Fehler/Zeitüberschreitungen`);
  if (ladder.updated_ms) meta.push(`Stand ${ago(ladder.updated_ms, now)}`);
  card.append(h("p", { class: "pipeline-meta", text: meta.join(" · ") }));
  if (ladder.problem) card.append(h("p", { class: "pipeline-error", text: ladder.problem }));
  if (ladder.arms.length) {
    card.append(
      h(
        "ol",
        { class: "stages" },
        ladder.arms.map(arm => {
          const parts = [`${integer(arm.net_won ?? 0)} : ${integer(arm.net_lost ?? 0)}`, winRate(arm.net_won ?? 0, arm.net_lost ?? 0)];
          if (arm.drawn) parts.push(`${integer(arm.drawn)} remis`);
          if (arm.unfinished) parts.push(`${integer(arm.unfinished)} offen`);
          return h("li", { class: "stage" }, h("span", { class: "stage-name", text: `${arm.name} · ${integer(arm.games)} Spiele` }), h("span", { class: "stage-state", text: parts.join(" · ") }));
        }),
      ),
    );
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

/** A titled list that is hidden while it has nothing to show. */
function section(title, note) {
  const list = h("div", { class: "pipeline-list" });
  const heading = h("h3", null, title, " ", h("span", { class: "count", text: note }));
  const node = h("section", { class: "training-section", hidden: true }, heading, list);
  return { node, list };
}

export function buildTraining(body, setTag) {
  const pipelines = h("div", { class: "pipeline-list" }, h("p", { class: "muted-note", text: "Warte auf Messwerte …" }));
  const runs = section("LLM-Spiele", "Sammler");
  const ladders = section("Ladder", "ladder.json");
  const problem = h("p", { class: "muted-note", hidden: true });
  const units = h("ul", { class: "unit-list" });
  const main = h("div", { class: "pipeline-column" }, pipelines, runs.node, ladders.node);
  body.append(h("div", { class: "training-grid" }, main, h("div", { class: "units-block" }, h("h3", null, "Systemdienste ", h("span", { class: "count", text: "baylee-*" })), problem, units)));
  main.addEventListener("click", event => {
    const button = event.target.closest("button[data-pipeline]");
    if (!button) return;
    const name = button.dataset.pipeline;
    if (expandedStages.has(name)) expandedStages.delete(name);
    else expandedStages.add(name);
    shownChecked = -1;
  });
  return ({ training, s, facts }) => {
    if (!training || training.checked_ms === shownChecked) return;
    shownChecked = training.checked_ms;
    const now = training.checked_ms || s.point.at_ms;
    const live = training.pipelines.filter(pipeline => pipeline.active).length + training.llm_runs.filter(run => run.active).length;
    setTag(training.checked_ms ? `${live ? `${number(live)} aktiv · ` : ""}Stand ${time(training.checked_ms)}` : "Nur Anzeige");
    const cards = training.pipelines.map(pipeline => pipelineCard(pipeline, training, now));
    if (!cards.length && facts?.panels.pipelines) cards.push(h("p", { class: "muted-note", text: "Keine Pipeline gefunden." }));
    pipelines.replaceChildren(...cards);
    runs.node.hidden = training.llm_runs.length === 0;
    runs.list.replaceChildren(...training.llm_runs.map(run => llmCard(run, now)));
    ladders.node.hidden = training.ladders.length === 0;
    ladders.list.replaceChildren(...training.ladders.map(ladder => ladderCard(ladder, now)));
    problem.hidden = !training.units_problem;
    setText(problem, training.units_problem ?? "");
    const items = training.units.map(unit => unitItem(unit, now));
    if (!items.length) items.push(h("li", { class: "unit empty", text: training.checked_ms ? "Keine baylee-* Systemdienste bekannt." : "Warte auf systemd …" }));
    units.replaceChildren(...items);
  };
}
