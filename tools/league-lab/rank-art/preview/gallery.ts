/// <reference lib="dom" />
/// <reference lib="dom.iterable" />
type Family = "player" | "deck";
type Badge = { label: string; division: number; place: number; image: string };
type Material = {
  name: string;
  description: string;
  period: number;
  player: Badge[];
  deck: Badge[];
};
type Catalog = { vertex: string; fragment: string; materials: Material[] };
const el = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const data: Catalog = JSON.parse(el("catalog").textContent!);
const canvas = el<HTMLCanvasElement>("canvas");
const fallback = el<HTMLImageElement>("fallback");
const original = el<HTMLImageElement>("original");
const variant = el<HTMLSelectElement>("variant");
const play = el<HTMLButtonElement>("play");
const slider = el<HTMLInputElement>("phase");
const reduced = matchMedia("(prefers-reduced-motion: reduce)");
let family: Family = "player";
let tier = data.materials.length - 1;
let index = data.materials[tier].player.findIndex((v) => v.place === 1);
let running = !reduced.matches;
let phase = 0.34;
let frame = 0;
let previous = 0;
let imageVersion = 0;
const material = () => data.materials[tier];
const badge = () => material()[family][index];

function renderer() {
  const gl = canvas.getContext("webgl2", {
    alpha: true,
    premultipliedAlpha: false,
    antialias: false,
    depth: false,
  });
  if (!gl) throw new Error("WebGL 2 ist hier nicht verfügbar.");
  const program = gl.createProgram()!;
  for (
    const [kind, source] of [[gl.VERTEX_SHADER, data.vertex], [
      gl.FRAGMENT_SHADER,
      data.fragment,
    ]] as const
  ) {
    const shader = gl.createShader(kind)!;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      throw new Error(gl.getShaderInfoLog(shader) ?? "Shaderfehler");
    }
    gl.attachShader(program, shader);
    gl.deleteShader(shader);
  }
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) ?? "Linkfehler");
  }
  gl.useProgram(program);
  gl.bindVertexArray(gl.createVertexArray());
  gl.disable(gl.DITHER);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.UNIFORM_BUFFER, buffer);
  const bytes = new ArrayBuffer(32);
  const ints = new Uint32Array(bytes);
  const floats = new Float32Array(bytes);
  gl.bufferData(gl.UNIFORM_BUFFER, bytes.byteLength, gl.DYNAMIC_DRAW);
  gl.uniformBlockBinding(program, 0, 0);
  gl.bindBufferBase(gl.UNIFORM_BUFFER, 0, buffer);
  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) {
    gl.texParameteri(gl.TEXTURE_2D, axis, gl.CLAMP_TO_EDGE);
  }
  for (const filter of [gl.TEXTURE_MAG_FILTER, gl.TEXTURE_MIN_FILTER]) {
    gl.texParameteri(gl.TEXTURE_2D, filter, gl.LINEAR);
  }
  gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
  return {
    upload(image: HTMLImageElement) {
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA8,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        image,
      );
    },
    draw() {
      floats[0] = phase;
      ints.set([
        tier,
        Number(family === "deck"),
        badge().division,
        badge().place,
        1,
      ], 1);
      gl.bufferSubData(gl.UNIFORM_BUFFER, 0, new Uint8Array(bytes));
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    },
  };
}
let gpu: ReturnType<typeof renderer> | undefined;
let ready = false;
function unavailable(reason: string) {
  ready = false;
  gpu = undefined;
  canvas.hidden = true;
  fallback.hidden = false;
  el("finishLabel").textContent = "Standbild";
  el("status").textContent = reason +
    " Die Originale sind weiterhin auswählbar.";
  el("status").hidden = false;
  schedule();
}
function tick(now: number) {
  if (previous) {
    phase = (phase + (now - previous) / (1000 * material().period)) % 1;
  }
  previous = now;
  slider.value = String(Math.round(phase * 1000));
  gpu?.draw();
  frame = requestAnimationFrame(tick);
}
function schedule() {
  cancelAnimationFrame(frame);
  previous = 0;
  const moving = material().period > 0 && !!gpu && ready;
  play.disabled = slider.disabled = !moving;
  play.textContent = running && moving ? "Anhalten" : "Abspielen";
  el("cycle").textContent = material().period
    ? `${material().period} s pro Umlauf`
    : "Ruhendes Material";
  slider.value = String(Math.round(phase * 1000));
  if (!ready) return;
  gpu?.draw();
  if (running && moving && !document.hidden) {
    frame = requestAnimationFrame(tick);
  }
}
const accents = [
  "",
  " Im großen Brillanten bricht sich das Licht in Farben. Danach erreicht es den Kranz kleiner Diamanten und das goldene Diadem.",
  " Die Stufen des blauen Diamanten fangen den Reflex zuerst auf, dann die beiden Seitensteine und die silbernen Federn.",
  " Ein warmer Lichtstrich trifft den roséfarbenen Diamanten und die Blattspitzen des Lorbeerreifs.",
];
function present() {
  const v = badge();
  el("title").textContent = [material().name, v.label === "Liga" ? "" : v.label]
    .filter(Boolean).join(" · ");
  el("description").textContent = material().description + accents[v.place];
  original.src = fallback.src = v.image;
  original.alt = `${el("title").textContent}, ${
    family === "player" ? "Spielerhelm" : "Deckabzeichen"
  }`;
  ready = false;
  canvas.hidden = true;
  fallback.hidden = false;
  schedule();
  const ticket = ++imageVersion;
  original.decode().then(() => {
    if (ticket !== imageVersion || !gpu) return;
    gpu.upload(original);
    ready = true;
    canvas.hidden = false;
    fallback.hidden = true;
    schedule();
  }).catch(() => {
    if (ticket === imageVersion) {
      unavailable("Das Abzeichen konnte nicht geladen werden.");
    }
  });
}
function showSelectedLeague() {
  const nav = el("leagues");
  const active = nav.children[tier] as HTMLElement | undefined;
  if (active && nav.scrollWidth > nav.clientWidth) {
    nav.scrollLeft += active.getBoundingClientRect().left -
      nav.getBoundingClientRect().left +
      (active.offsetWidth - nav.clientWidth) / 2;
  }
}
function select() {
  variant.replaceChildren(
    ...material()[family].map((v, i) => new Option(v.label, String(i))),
  );
  variant.selectedIndex = index;
  variant.disabled = variant.length === 1;
  for (const f of ["player", "deck"]) {
    el(f).setAttribute("aria-pressed", String(family === f));
  }
  for (const [i, b] of [...el("leagues").children].entries()) {
    b.setAttribute("aria-pressed", String(i === tier));
    b.querySelector("img")!.src = data.materials[i][family][0].image;
  }
  showSelectedLeague();
  present();
}
window.addEventListener("resize", showSelectedLeague);
for (const [i, m] of data.materials.entries()) {
  const button = document.createElement("button");
  button.className = "league";
  const icon = new Image();
  icon.alt = "";
  button.append(icon, document.createTextNode(m.name));
  button.onclick = () => {
    tier = i;
    index = 0;
    select();
  };
  el("leagues").append(button);
}
for (const f of ["player", "deck"] as const) {
  el(f).onclick = () => {
    const label = badge().label;
    family = f;
    index = Math.max(0, material()[f].findIndex((v) => v.label === label));
    select();
  };
}
variant.onchange = () => {
  index = variant.selectedIndex;
  present();
};
slider.oninput = () => {
  running = false;
  phase = Number(slider.value) / 1000;
  schedule();
};
play.onclick = () => {
  running = !running;
  schedule();
};
reduced.addEventListener("change", () => {
  running = !reduced.matches;
  schedule();
});
document.addEventListener("visibilitychange", schedule);
canvas.addEventListener("webglcontextlost", (e) => {
  e.preventDefault();
  unavailable("Der Grafikkontext wurde unterbrochen.");
});
function init() {
  try {
    gpu = renderer();
    el("status").hidden = true;
    el("finishLabel").textContent = "Mit Shader";
  } catch (e) {
    console.error(e);
    unavailable("Der Materialshader ist in diesem Browser nicht verfügbar.");
  }
  select();
}
canvas.addEventListener("webglcontextrestored", init);
init();
