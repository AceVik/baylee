// Starts the browser client (#327). Trunk writes the tag that loads this file
// in place of `<link data-trunk rel="rust">` (`Trunk.toml`, `pattern_script`),
// with the hashed names of the wasm-bindgen module and the wasm on it. It is a
// file and not trunk's inline loader so that the page carries no inline
// script at all, and `scripts/server/play.caddy`'s Content-Security-Policy can
// say `script-src 'self'` (docs/client.md §"Serving it at /play/").

// Sound waits for the player (#296). A browser lets a page make sound only
// after someone has pressed, tapped or typed on it: an AudioContext made
// before that starts suspended, and cpal makes its one at startup and asks it
// to resume only then, so without this nothing the client plays is ever heard
// in a browser. This keeps every context the page makes and resumes it on
// each press until it runs. It must run before the wasm is loaded, so the
// context cpal makes is one of these.
(() => {
  const Made = window.AudioContext || window.webkitAudioContext;
  if (!Made) return;
  const made = [];
  window.AudioContext = class extends Made {
    constructor(...args) {
      super(...args);
      made.push(this);
    }
  };
  const wake = () => {
    for (const context of made) {
      if (context.state === "suspended") context.resume();
    }
  };
  for (const press of ["pointerdown", "keydown", "touchend"]) {
    window.addEventListener(press, wake, { capture: true, passive: true });
  }
})();

// Why this browser cannot draw the table, or null when it can. The client
// renders through WebGPU only (bevy's `webgpu` feature); without an adapter
// wgpu finds nothing to draw with and the page stays black, so it is asked
// here first, before twenty megabytes are fetched for nothing.
async function missingWebGpu() {
  if (!("gpu" in navigator)) {
    // `navigator.gpu` exists only in a secure context: https, or localhost.
    return window.isSecureContext ? "browser" : "insecure";
  }
  try {
    return (await navigator.gpu.requestAdapter()) ? null : "adapter";
  } catch {
    return "adapter";
  }
}

const WORDS = {
  en: {
    title: "Baylee needs WebGPU",
    browser:
      "This browser does not offer WebGPU, which Baylee draws the table with.",
    insecure:
      "WebGPU is offered only to pages served over https (or from localhost).",
    adapter:
      "This browser offers WebGPU but found no graphics adapter it may use; it may be switched off, or the graphics driver not supported.",
    which:
      "It runs in Chrome or Edge 113 and later, Safari 26 and later (macOS, iOS, iPadOS), and Firefox 141 and later on Windows or 145 and later on macOS. Firefox on Linux and Android does not offer WebGPU yet.",
  },
  de: {
    title: "Baylee braucht WebGPU",
    browser:
      "Dieser Browser bietet kein WebGPU an, womit Baylee den Tisch zeichnet.",
    insecure:
      "WebGPU gibt es nur für Seiten, die über https (oder von localhost) kommen.",
    adapter:
      "Dieser Browser bietet WebGPU an, fand aber keinen Grafikadapter, den er nutzen darf; es kann abgeschaltet sein, oder der Grafiktreiber wird nicht unterstützt.",
    which:
      "Es läuft in Chrome oder Edge ab 113, Safari ab 26 (macOS, iOS, iPadOS) und Firefox ab 141 unter Windows bzw. ab 145 unter macOS. Firefox unter Linux und Android bietet WebGPU noch nicht an.",
  },
};

// The page's answer when the table cannot be drawn: text, never a blank
// canvas. Built from elements and `textContent`, styled by `page.css`.
function explain(reason) {
  const words = (navigator.language || "en").toLowerCase().startsWith("de")
    ? WORDS.de
    : WORDS.en;
  const box = document.createElement("main");
  box.id = "baylee-unsupported";
  const title = document.createElement("h1");
  title.textContent = words.title;
  box.append(title);
  for (const line of [words[reason], words.which]) {
    const p = document.createElement("p");
    p.textContent = line;
    box.append(p);
  }
  document.body.append(box);
}

const tag = document.querySelector("script[data-wasm]");
const reason = await missingWebGpu();
if (reason) {
  explain(reason);
} else {
  const bindings = await import(tag.dataset.js);
  const wasm = await bindings.default({ module_or_path: tag.dataset.wasm });
  window.wasmBindings = bindings;
  dispatchEvent(new CustomEvent("TrunkApplicationStarted", { detail: { wasm } }));
}
