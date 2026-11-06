/*
 * The dashboard's grid: cards can be moved, resized, hidden and restored.
 *
 * Moving and resizing use pointer events, so a finger works like a mouse; only
 * a card's header takes over touch, so the page still scrolls everywhere else.
 * The same actions have keys on each card's grip: arrows move, Shift+arrows
 * resize, Delete hides.
 *
 * Widths are stored on a 12-column scale. A wide monitor keeps those twelve
 * columns, only wider, so the layout never gains holes. A narrower grid scales
 * widths down and, below tablet width, stacks cards at their natural height.
 */

import { $, h, icon, reducedMotion, setText } from "./dom.js";
import { saveSettings, settings } from "./store.js";

const WIDE = 12;
const FLIP_MS = 180;
/** Time after a swap in which the card under the pointer may not swap back. */
const SWAP_COOLDOWN_MS = 170;
const EDGE_PX = 70;

const announce = text => setText($("live"), text);

export class Dashboard {
  /** `cards` are { id, title, kind, w, h, node } in their default order. */
  constructor(grid, cards) {
    this.grid = grid;
    this.cards = cards;
    this.byId = new Map(cards.map(card => [card.id, card]));
    this.columns = WIDE;
    this.editing = false;
    this.undo = null;
    /** Cards this machine has nothing for, until a sample says otherwise. */
    this.missing = new Set(cards.filter(card => card.available).map(card => card.id));
    for (const card of cards) this.attach(card);
    new ResizeObserver(() => this.fit()).observe(grid);
    this.apply();
  }

  /** The stored order, with unknown ids dropped and new cards in their place. */
  order() {
    const known = settings.layout.order.filter(id => this.byId.has(id));
    const missing = this.cards.map(card => card.id).filter(id => !known.includes(id));
    for (const id of missing) {
      const before = this.cards.findIndex(card => card.id === id);
      const anchor = known.findIndex(other => this.cards.findIndex(card => card.id === other) > before);
      known.splice(anchor < 0 ? known.length : anchor, 0, id);
    }
    return known;
  }

  isHidden(id) {
    return settings.layout.hidden.includes(id);
  }

  /** Shows what this machine has. A card without a source is no card, not an empty one. */
  sync(context) {
    const missing = this.cards.filter(card => card.available && !card.available(context)).map(card => card.id);
    if (missing.length === this.missing.size && missing.every(id => this.missing.has(id))) return;
    this.missing = new Set(missing);
    this.apply();
  }

  size(card) {
    return settings.layout.sizes[card.id] ?? { w: card.w, h: card.h };
  }

  /** Columns a card spans on the current grid. */
  span(card) {
    const { w } = this.size(card);
    if (this.columns === WIDE) return w;
    if (this.columns <= 4) return card.kind === "tile" ? Math.ceil(this.columns / 2) : this.columns;
    const scaled = Math.round((w * this.columns) / WIDE);
    return Math.max(card.kind === "tile" ? 2 : Math.ceil(this.columns / 2), Math.min(this.columns, scaled));
  }

  fit() {
    const width = this.grid.clientWidth;
    const columns = width >= 1080 ? WIDE : width >= 740 ? 8 : width >= 460 ? 4 : 2;
    if (columns === this.columns && this.grid.style.getPropertyValue("--cols")) return;
    this.columns = columns;
    this.apply();
  }

  /** Puts the DOM in the stored order and sets every card's span. */
  apply() {
    this.grid.style.setProperty("--cols", this.columns);
    this.grid.classList.toggle("natural", this.columns <= 4);
    this.order().forEach((id, at) => {
      const card = this.byId.get(id);
      // During a drag only CSS moves cards: moving the dragged node in the DOM
      // would end its pointer capture, and the drop would never arrive.
      if (this.drag) {
        card.node.style.order = at;
      } else {
        card.node.style.order = "";
        this.grid.append(card.node);
      }
      card.node.hidden = this.isHidden(id) || this.missing.has(id);
      this.place(card);
    });
    this.grid.dispatchEvent(new CustomEvent("layoutchange"));
  }

  place(card) {
    const span = this.span(card);
    card.node.style.gridColumn = `span ${span}`;
    card.node.style.gridRow = this.columns <= 4 ? "auto" : `span ${this.size(card).h}`;
  }

  store() {
    settings.layout.order = this.order();
    saveSettings();
  }

  setEditing(on) {
    this.editing = on;
    this.grid.classList.toggle("editing", on);
    document.body.classList.toggle("is-editing", on);
    for (const card of this.cards) {
      card.tools.hidden = !on;
      card.grip.hidden = !on;
    }
    if (on) announce("Anordnen aktiv. Karten am Kopf ziehen oder den Griff fokussieren und die Pfeiltasten nutzen.");
  }

  // Moving

  /** Runs `change`, then lets every card glide from its old place. */
  animated(change, except) {
    const visible = this.cards.filter(card => !card.node.hidden && card !== except);
    const before = new Map(visible.map(card => [card, card.node.getBoundingClientRect()]));
    change();
    if (reducedMotion()) return;
    for (const card of visible) {
      const from = before.get(card);
      const to = card.node.getBoundingClientRect();
      const dx = from.left - to.left;
      const dy = from.top - to.top;
      if (dx || dy) card.node.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: FLIP_MS, easing: "cubic-bezier(.2,.8,.2,1)" });
    }
  }

  moveTo(id, targetId, after) {
    const order = this.order().filter(other => other !== id);
    order.splice(order.indexOf(targetId) + (after ? 1 : 0), 0, id);
    settings.layout.order = order;
  }

  /** One step along the visible cards, for the keyboard. */
  moveBy(card, step) {
    const order = this.order().filter(id => !this.isHidden(id) && !this.missing.has(id));
    const at = order.indexOf(card.id);
    const target = order[at + step];
    if (!target) return;
    this.animated(() => {
      this.moveTo(card.id, target, step > 0);
      this.apply();
    });
    this.store();
    card.grip.focus();
    announce(`${card.title}: Position ${at + step + 1} von ${order.length}.`);
  }

  startDrag(card, event) {
    const node = card.node;
    const rect = node.getBoundingClientRect();
    const drag = { card, id: event.pointerId, grabX: event.clientX - rect.left, grabY: event.clientY - rect.top, x: event.clientX, y: event.clientY, startOrder: this.order(), swapped: 0 };
    this.drag = drag;
    node.setPointerCapture(event.pointerId);
    node.classList.add("dragging");
    this.grid.classList.add("drag-active");
    this.follow();
    this.scrollLoop();
  }

  /** Keeps the card under the pointer wherever its slot currently is. */
  follow() {
    const { card, x, y, grabX, grabY } = this.drag;
    const origin = this.grid.getBoundingClientRect();
    const left = x - origin.left - grabX - card.node.offsetLeft;
    const top = y - origin.top - grabY - card.node.offsetTop;
    card.node.style.transform = `translate(${left}px, ${top}px) rotate(.6deg)`;
  }

  dragMove(event) {
    const drag = this.drag;
    if (!drag || event.pointerId !== drag.id) return;
    drag.x = event.clientX;
    drag.y = event.clientY;
    this.follow();
    const now = performance.now();
    if (now - drag.swapped < SWAP_COOLDOWN_MS) return;
    const target = document.elementFromPoint(drag.x, drag.y)?.closest(".card");
    if (!target || target === drag.card.node) return;
    const targetId = target.dataset.card;
    const order = this.order();
    const after = order.indexOf(drag.card.id) < order.indexOf(targetId);
    drag.swapped = now;
    this.animated(() => {
      this.moveTo(drag.card.id, targetId, after);
      this.apply();
      this.follow();
    }, drag.card);
  }

  /** Scrolls while the pointer rests near the top or bottom of the window. */
  scrollLoop() {
    const step = () => {
      const drag = this.drag;
      if (!drag) return;
      const edge = drag.y < EDGE_PX ? -1 : drag.y > window.innerHeight - EDGE_PX ? 1 : 0;
      if (edge) {
        window.scrollBy(0, edge * 12);
        this.follow();
      }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  endDrag(cancelled) {
    const drag = this.drag;
    if (!drag) return;
    this.drag = null;
    const node = drag.card.node;
    node.classList.remove("dragging");
    node.style.transform = "";
    this.grid.classList.remove("drag-active");
    if (cancelled) {
      settings.layout.order = drag.startOrder;
      announce("Verschieben abgebrochen.");
    }
    this.apply();
    if (!cancelled) this.store();
  }

  // Resizing

  resizeBy(card, dw, dh) {
    const size = { ...this.size(card) };
    size.w = Math.max(card.minW ?? 2, Math.min(WIDE, size.w + dw));
    size.h = Math.max(card.minH ?? 2, Math.min(14, size.h + dh));
    settings.layout.sizes[card.id] = size;
    this.place(card);
    saveSettings();
    this.grid.dispatchEvent(new CustomEvent("layoutchange"));
    announce(`${card.title}: ${size.w} Spalten breit, ${size.h} Zeilen hoch.`);
  }

  startResize(card, event) {
    event.stopPropagation();
    event.preventDefault();
    const style = getComputedStyle(this.grid);
    const gap = parseFloat(style.columnGap) || 0;
    const row = parseFloat(style.gridAutoRows) || 80;
    const column = (this.grid.clientWidth - gap * (this.columns - 1)) / this.columns;
    const rect = card.node.getBoundingClientRect();
    const scale = WIDE / this.columns;
    this.resizing = { card, id: event.pointerId, gap, row, column, left: rect.left, top: rect.top, scale };
    card.node.setPointerCapture(event.pointerId);
    card.node.classList.add("resizing");
  }

  resizeMove(event) {
    const state = this.resizing;
    if (!state || event.pointerId !== state.id) return;
    const { card, gap, row, column, left, top, scale } = state;
    const spanWide = Math.round((event.clientX - left + gap) / (column + gap));
    const spanHigh = Math.round((event.clientY - top + gap) / (row + gap));
    const size = {
      w: Math.max(card.minW ?? 2, Math.min(WIDE, Math.round(spanWide * scale))),
      h: Math.max(card.minH ?? 2, Math.min(14, spanHigh)),
    };
    const old = this.size(card);
    if (old.w === size.w && old.h === size.h) return;
    settings.layout.sizes[card.id] = size;
    this.place(card);
    this.grid.dispatchEvent(new CustomEvent("layoutchange"));
  }

  endResize() {
    if (!this.resizing) return;
    this.resizing.card.node.classList.remove("resizing");
    this.resizing = null;
    saveSettings();
  }

  // Hiding

  setHidden(id, hidden) {
    const list = settings.layout.hidden.filter(other => other !== id);
    if (hidden) list.push(id);
    settings.layout.hidden = list;
    saveSettings();
    this.animated(() => this.apply());
  }

  hide(card) {
    this.setHidden(card.id, true);
    announce(`${card.title} ausgeblendet.`);
    this.offerUndo(`${card.title} ausgeblendet`, () => this.setHidden(card.id, false));
  }

  /** Replaces the layout, keeping the previous one for an undo. */
  replaceLayout(layout, message) {
    const previous = JSON.parse(JSON.stringify(settings.layout));
    settings.layout = layout;
    saveSettings();
    this.animated(() => this.apply());
    this.offerUndo(message, () => {
      settings.layout = previous;
      saveSettings();
      this.animated(() => this.apply());
    });
  }

  offerUndo(message, action) {
    const toast = $("toast");
    toast.replaceChildren(
      h("span", { text: message }),
      h("button", { class: "pill-button", type: "button", text: "Rückgängig", onclick: () => { action(); toast.hidden = true; } }),
    );
    toast.hidden = false;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => { toast.hidden = true; }, 8000);
  }

  // Wiring

  attach(card) {
    const { node, head, grip, tools, handle } = card;
    grip.addEventListener("keydown", event => {
      if (!this.editing) return;
      const steps = { ArrowLeft: [-1, -1, 0], ArrowUp: [-1, 0, -1], ArrowRight: [1, 1, 0], ArrowDown: [1, 0, 1] };
      if (event.key === "Delete") {
        event.preventDefault();
        this.hide(card);
        return;
      }
      const step = steps[event.key];
      if (!step) return;
      event.preventDefault();
      if (event.shiftKey) this.resizeBy(card, step[1], step[2]);
      else this.moveBy(card, step[0]);
    });
    head.addEventListener("pointerdown", event => {
      if (!this.editing || event.button !== 0 || event.target.closest(".card-tools")) return;
      event.preventDefault();
      this.startDrag(card, event);
    });
    handle.addEventListener("pointerdown", event => {
      if (this.editing) this.startResize(card, event);
    });
    node.addEventListener("pointermove", event => {
      this.dragMove(event);
      this.resizeMove(event);
    });
    node.addEventListener("pointerup", () => {
      this.endDrag(false);
      this.endResize();
    });
    node.addEventListener("pointercancel", () => {
      this.endDrag(true);
      this.endResize();
    });
    // Whatever takes the pointer away, a card must never stay stuck to it.
    node.addEventListener("lostpointercapture", () => {
      this.endDrag(false);
      this.endResize();
    });
    tools.querySelector("[data-action=hide]").addEventListener("click", () => this.hide(card));
    tools.querySelector("[data-action=wider]").addEventListener("click", () => this.resizeBy(card, 1, 0));
    tools.querySelector("[data-action=narrower]").addEventListener("click", () => this.resizeBy(card, -1, 0));
    tools.querySelector("[data-action=taller]").addEventListener("click", () => this.resizeBy(card, 0, 1));
    tools.querySelector("[data-action=shorter]").addEventListener("click", () => this.resizeBy(card, 0, -1));
  }
}

/** Escape cancels a drag in progress. */
export function cancelDragOnEscape(dashboard) {
  document.addEventListener("keydown", event => {
    if (event.key === "Escape" && dashboard.drag) dashboard.endDrag(true);
  });
}

/** The tool buttons a card shows while arranging. */
export function cardTools(title) {
  const button = (action, name, label) => h("button", { class: "icon-button", type: "button", "data-action": action, title: label, "aria-label": `${title}: ${label}` }, icon(name));
  return h(
    "div",
    { class: "card-tools", hidden: true },
    button("narrower", "narrower", "Schmaler"),
    button("wider", "wider", "Breiter"),
    button("shorter", "shorter", "Niedriger"),
    button("taller", "taller", "Höher"),
    button("hide", "hide", "Ausblenden"),
  );
}
