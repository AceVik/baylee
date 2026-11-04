/* Small DOM helpers. Telemetry text reaches the page only through textContent. */

export const $ = id => document.getElementById(id);

/** Creates an element. `class` and `text` are shortcuts, `on…` adds a listener. */
export function h(tag, props, ...children) {
  const element = document.createElement(tag);
  for (const [key, value] of Object.entries(props ?? {})) {
    if (value == null || value === false) continue;
    if (key === "class") element.className = value;
    else if (key === "text") element.textContent = value;
    else if (key === "value" || key === "checked") element[key] = value;
    else if (key.startsWith("on")) element.addEventListener(key.slice(2), value);
    else element.setAttribute(key, value === true ? "" : value);
  }
  element.append(...children.flat().filter(child => child != null));
  return element;
}

/** An icon from the sprite in index.html; decorative, so hidden from readers. */
export function icon(name, className = "icon") {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", className);
  svg.setAttribute("aria-hidden", "true");
  const use = document.createElementNS(ns, "use");
  use.setAttribute("href", `#i-${name}`);
  svg.append(use);
  return svg;
}

/** Writes text only when it changed, which spares layout and keeps selections. */
export function setText(element, value) {
  const text = String(value ?? "");
  if (element.textContent !== text) element.textContent = text;
}

/** Sets a CSS custom property only when it changed. */
export function setVar(element, name, value) {
  const text = String(value);
  if (element.style.getPropertyValue(name) !== text) element.style.setProperty(name, text);
}

/**
 * Makes `parent`'s children match `items`, reusing the node of every key, so
 * hover, focus and scroll positions survive a refresh.
 */
export function keyed(parent, items, keyOf, create, update) {
  const previous = parent.keyedNodes ?? new Map();
  const next = new Map();
  items.forEach((item, at) => {
    const key = keyOf(item);
    const node = previous.get(key) ?? create(item);
    next.set(key, node);
    update(node, item);
    const here = parent.children[at];
    if (here !== node) parent.insertBefore(node, here ?? null);
  });
  for (const [key, node] of previous) if (!next.has(key)) node.remove();
  parent.keyedNodes = next;
}

/** A label and value pair that updates in place. */
export function statRows(parent, rows) {
  keyed(
    parent,
    rows.filter(row => row != null),
    row => row[0],
    row => h("div", { class: "kv" }, h("dt", { text: row[0] }), h("dd")),
    (node, row) => {
      setText(node.lastChild, row[1]);
      if (row[2]) node.title = row[2];
    },
  );
}

/** The value of a percent on a rounded meter, by severity. */
export function meter(fill, percent, level = "") {
  setVar(fill, "--value", `${Math.max(0, Math.min(100, percent ?? 0))}%`);
  const next = `fill ${level}`.trim();
  if (fill.className !== next) fill.className = next;
}

export function meterBar(extraClass = "") {
  const fill = h("i", { class: "fill" });
  return { node: h("div", { class: `meter ${extraClass}`.trim() }, fill), fill };
}

export const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
