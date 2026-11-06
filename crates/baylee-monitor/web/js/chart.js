/*
 * Time charts on canvas, drawn only when a sample arrives. One y-axis per chart
 * by rule; gaps in the data stay gaps. Long ranges keep each pixel column's
 * minimum and maximum, so a one-second spike survives an hour on 400 pixels.
 */

import { h } from "./dom.js";
import { token, onThemeChange } from "./theme.js";
import { number, shortTime, time, rate, niceCeiling, niceRateCeiling } from "./format.js";

const charts = new Set();

onThemeChange(() => charts.forEach(chart => chart.draw()));
// A tap shows a chart's values; the next tap elsewhere puts them away.
document.addEventListener("pointerdown", event => {
  for (const chart of charts) if (!chart.host.contains(event.target)) chart.clearInspection();
});

function fitCanvas(canvas) {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (!width || !height) return null;
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  const pixelWidth = Math.round(width * scale);
  const pixelHeight = Math.round(height * scale);
  if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
    canvas.width = pixelWidth;
    canvas.height = pixelHeight;
  }
  const context = canvas.getContext("2d");
  context.setTransform(scale, 0, 0, scale, 0, 0);
  context.clearRect(0, 0, width, height);
  return { context, width, height };
}

/**
 * Splits `points` into connected runs of [x, y, x, y, …]. A column that holds
 * several points contributes its lowest and its highest.
 */
function trace(points, value, x, y, gap) {
  const runs = [];
  let run = null;
  let column = null;
  let low = 0;
  let high = 0;
  let lastTime = 0;
  const flush = () => {
    if (column == null || !run) return;
    run.push(column, high);
    if (low !== high) run.push(column, low);
    column = null;
  };
  for (const point of points) {
    const reading = value(point);
    if (reading == null) {
      flush();
      run = null;
      continue;
    }
    if (run && point.at_ms - lastTime > gap) {
      flush();
      run = null;
    }
    if (!run) {
      run = [];
      runs.push(run);
    }
    const px = Math.round(x(point.at_ms));
    const py = y(reading);
    if (px === column) {
      low = Math.max(low, py);
      high = Math.min(high, py);
    } else {
      flush();
      column = px;
      low = py;
      high = py;
    }
    lastTime = point.at_ms;
  }
  flush();
  return runs;
}

function strokeRun(context, run) {
  context.beginPath();
  context.moveTo(run[0], run[1]);
  for (let at = 2; at < run.length; at += 2) context.lineTo(run[at], run[at + 1]);
  context.stroke();
}

function fillRun(context, run, base) {
  context.beginPath();
  context.moveTo(run[0], base);
  for (let at = 0; at < run.length; at += 2) context.lineTo(run[at], run[at + 1]);
  context.lineTo(run.at(-2), base);
  context.closePath();
  context.fill();
}

export const formats = {
  percent: { axis: value => `${Math.round(value)} %`, value: value => `${number(value)} %`, ceiling: () => 100 },
  perSecond: { axis: value => `${number(value)}/s`, value: value => `${number(value)}/s`, ceiling: niceCeiling },
  rate: { axis: rate, value: rate, ceiling: niceRateCeiling },
  watts: { axis: value => `${Math.round(value)} W`, value: value => `${number(value)} W`, ceiling: niceCeiling },
  celsius: { axis: value => `${Math.round(value)} °`, value: value => `${number(value)} °C`, ceiling: niceCeiling },
};

export class Chart {
  /**
   * `series` is [{ label, color, value(point) }]. `format` is a key of
   * `formats`; `max` fixes the top of the scale.
   */
  constructor({ series, format, max, label, area = series.length <= 2, compact = false }) {
    this.series = series;
    this.format = formats[format];
    this.max = max;
    this.area = area;
    this.range = null;
    this.inspected = null;
    this.canvas = h("canvas", { role: "img", tabindex: "0", "aria-label": label });
    this.tooltip = h("output", { class: "tooltip", hidden: true });
    this.host = h("div", { class: `chart${compact ? " compact" : ""}` }, this.canvas, this.tooltip);
    this.resizer = new ResizeObserver(() => this.draw());
    this.resizer.observe(this.canvas);
    this.listen();
    charts.add(this);
  }

  /** Stops drawing; the caller removes the element. */
  dispose() {
    this.resizer.disconnect();
    charts.delete(this);
  }

  listen() {
    const inspect = event => {
      this.inspected = { x: event.clientX - this.canvas.getBoundingClientRect().left };
      this.draw();
    };
    this.canvas.addEventListener("pointermove", inspect);
    this.canvas.addEventListener("pointerdown", inspect);
    this.canvas.addEventListener("pointerleave", event => {
      if (event.pointerType === "mouse") this.clearInspection();
    });
    this.canvas.addEventListener("blur", () => this.clearInspection());
    this.canvas.addEventListener("keydown", event => this.step(event));
  }

  /** Arrow keys walk the crosshair over the samples, as a pointer would. */
  step(event) {
    const points = this.range?.points ?? [];
    if (!points.length) return;
    if (event.key === "Escape") return this.clearInspection();
    const moves = { ArrowLeft: -1, ArrowRight: 1, PageUp: -10, PageDown: 10 };
    if (event.key === "Home") this.inspected = { index: 0 };
    else if (event.key === "End") this.inspected = { index: points.length - 1 };
    else if (moves[event.key]) {
      const from = this.inspected?.index ?? points.length - 1;
      this.inspected = { index: Math.max(0, Math.min(points.length - 1, from + moves[event.key])) };
    } else return;
    event.preventDefault();
    this.draw();
  }

  clearInspection() {
    if (!this.inspected) return;
    this.inspected = null;
    this.tooltip.hidden = true;
    this.draw();
  }

  /** The top of the scale: fixed, or the visible maximum, rounded up. */
  scaleTop(points) {
    if (this.max != null) return this.max;
    let top = 0;
    for (const point of points) {
      for (const series of this.series) {
        const value = series.value(point);
        if (value != null && value > top) top = value;
      }
    }
    return this.format.ceiling(top * 1.05);
  }

  draw(range = this.range) {
    this.range = range;
    const frame = fitCanvas(this.canvas);
    if (!frame || !range) return;
    const { context, width, height } = frame;
    const { points, start, end, gap } = range;
    const top = this.scaleTop(points);
    const font = `11px ${token("--font")}`;
    context.font = font;
    const axisLabels = [top, top / 2, 0].map(value => this.format.axis(value));
    const left = Math.ceil(Math.max(...axisLabels.map(label => context.measureText(label).width))) + 10;
    const right = width - 6;
    const upper = 8;
    const lower = height - 20;
    const x = at => left + ((at - start) / (end - start)) * (right - left);
    const y = value => lower - (Math.max(0, Math.min(top, value)) / top) * (lower - upper);

    this.drawGrid(context, { axisLabels, left, right, upper, lower, start, end, width, height });
    this.drawSeries(context, points, x, y, lower, gap);
    if (!points.length) {
      context.fillStyle = token("--muted");
      context.textAlign = "center";
      context.fillText("Verlauf wird aufgebaut …", width / 2, height / 2);
    }
    if (this.inspected && points.length) this.drawInspection(context, points, { x, left, right, upper, lower, start, end, width });
  }

  drawGrid(context, { axisLabels, left, right, upper, lower, start, end, width, height }) {
    context.lineWidth = 1;
    context.textBaseline = "middle";
    axisLabels.forEach((label, at) => {
      const level = Math.round(upper + ((lower - upper) * at) / 2) + 0.5;
      context.strokeStyle = at === 2 ? token("--axis") : token("--grid");
      context.beginPath();
      context.moveTo(left, level);
      context.lineTo(right, level);
      context.stroke();
      context.fillStyle = token("--muted");
      context.textAlign = "right";
      context.fillText(label, left - 8, level);
    });
    context.textBaseline = "alphabetic";
    const ticks = width < 260 ? [0, 1] : [0, 0.5, 1];
    for (const share of ticks) {
      const at = start + (end - start) * share;
      context.textAlign = share === 0 ? "left" : share === 1 ? "right" : "center";
      context.fillText(shortTime(at), left + (right - left) * share, height - 4);
    }
  }

  drawSeries(context, points, x, y, lower, gap) {
    context.lineWidth = 2;
    context.lineJoin = "round";
    context.lineCap = "round";
    for (const series of this.series) {
      const color = token(series.color);
      const runs = trace(points, series.value, x, y, gap);
      context.strokeStyle = color;
      context.fillStyle = color;
      for (const run of runs) {
        if (this.area && run.length > 2) {
          context.globalAlpha = 0.1;
          fillRun(context, run, lower);
          context.globalAlpha = 1;
        }
        strokeRun(context, run);
      }
      const last = points.at(-1);
      const reading = last && series.value(last);
      if (reading != null) {
        context.fillStyle = token("--surface");
        context.beginPath();
        context.arc(x(last.at_ms), y(reading), 6, 0, Math.PI * 2);
        context.fill();
        context.fillStyle = color;
        context.beginPath();
        context.arc(x(last.at_ms), y(reading), 4, 0, Math.PI * 2);
        context.fill();
      }
    }
  }

  drawInspection(context, points, { x, left, right, upper, lower, start, end, width }) {
    let point;
    if (this.inspected.index != null) {
      point = points[Math.min(this.inspected.index, points.length - 1)];
    } else {
      const target = start + ((this.inspected.x - left) / (right - left)) * (end - start);
      point = points.reduce((best, next) => (Math.abs(next.at_ms - target) < Math.abs(best.at_ms - target) ? next : best));
    }
    const at = Math.round(x(point.at_ms)) + 0.5;
    context.strokeStyle = token("--muted");
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(at, upper);
    context.lineTo(at, lower);
    context.stroke();
    this.tooltip.replaceChildren(
      h("time", { text: time(point.at_ms) }),
      ...this.series.map(series => {
        const reading = series.value(point);
        const key = h("i");
        key.style.background = token(series.color);
        return h("div", { class: "tooltip-row" }, key, h("b", { text: reading == null ? "—" : this.format.value(reading) }), h("span", { text: series.label }));
      }),
    );
    this.tooltip.hidden = false;
    const box = this.tooltip.offsetWidth;
    const onLeft = at + 12 + box > width;
    this.tooltip.style.left = `${Math.max(left, onLeft ? at - box - 12 : at + 12)}px`;
  }
}

/** Forgets every chart inside `element`, before it is emptied. */
export function disposeChartsIn(element) {
  for (const chart of charts) if (element.contains(chart.host)) chart.dispose();
}

/** A small line without axes, for tiles and the resource list. */
export function drawSparkline(canvas, points, value, { color, max, gap = 3000 }) {
  const frame = fitCanvas(canvas);
  if (!frame || points.length < 2) return;
  const { context, width, height } = frame;
  const start = points[0].at_ms;
  const end = points.at(-1).at_ms;
  let top = max ?? 0;
  if (max == null) {
    for (const point of points) top = Math.max(top, value(point) ?? 0);
    if (top <= 0) top = 1;
  }
  const x = at => ((at - start) / (end - start || 1)) * width;
  const y = reading => height - 2 - (Math.max(0, Math.min(top, reading)) / top) * (height - 5);
  const stroke = token(color);
  context.lineWidth = 2;
  context.lineJoin = "round";
  context.lineCap = "round";
  context.strokeStyle = stroke;
  context.fillStyle = stroke;
  for (const run of trace(points, value, x, y, gap)) {
    if (run.length > 2) {
      context.globalAlpha = 0.12;
      fillRun(context, run, height);
      context.globalAlpha = 1;
    }
    strokeRun(context, run);
  }
}

/**
 * Each logical CPU as a row and time as columns, one hue from quiet to busy.
 * Only samples since this page opened: per-core history is not kept on the
 * service.
 */
export function drawCoreStrip(canvas, history) {
  const frame = fitCanvas(canvas);
  if (!frame || !history.length) return;
  const { context, width, height } = frame;
  const cores = history.at(-1).cores.length;
  const rowHeight = height / cores;
  const columnWidth = Math.max(1, width / 300);
  context.fillStyle = token("--heat");
  history.forEach((entry, column) => {
    const left = width - (history.length - column) * columnWidth;
    if (left + columnWidth < 0) return;
    entry.cores.forEach((usage, core) => {
      context.globalAlpha = 0.06 + (Math.min(100, usage) / 100) * 0.94;
      context.fillRect(left, core * rowHeight, columnWidth + 0.5, Math.max(1, rowHeight - 1));
    });
  });
  context.globalAlpha = 1;
}
