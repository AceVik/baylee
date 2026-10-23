/* Local UI: no libraries, external requests, process commands or persisted telemetry. */
"use strict";
const $ = id => document.getElementById(id);
const colors = {cpu:"#6ce2bc",gpu:"#a69afa",ram:"#75b9ed",vram:"#efb66d",gpu_watts:"#a69afa",cpu_watts:"#6ce2bc",net_rx:"#6ce2bc",net_tx:"#efb66d"};
const labels = {cpu:"CPU",gpu:"GPU 0",ram:"RAM",vram:"VRAM 0",gpu_watts:"GPU",cpu_watts:"CPU",net_rx:"Empfang",net_tx:"Versand"};
const nf = new Intl.NumberFormat("de-DE", {maximumFractionDigits:1});
let facts, snapshot, history = [], windowSecs = 300, filter = "all", sortKey = "cpu", descending = true, limit = 12;
let socket, reconnectTimer, lastArrival = 0, retry = 1000;
let allSensors = false;
const charts = [];
const text = (id, value) => { $(id).textContent = value; };
const number = value => value == null ? "—" : nf.format(value);
const percent = value => value == null ? "—" : `${number(value)} %`;
function bytes(value) {
  if (value == null) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let i = 0;
  while (value >= 1024 && i < units.length - 1) { value /= 1024; i++; }
  return `${number(value)} ${units[i]}`;
}
const rate = value => `${bytes(value)}/s`;
const time = value => new Date(value).toLocaleTimeString("de-DE", {hour:"2-digit",minute:"2-digit",second:"2-digit"});
function node(tag, className, value) {
  const e = document.createElement(tag);
  if (className) e.className = className;
  if (value != null) e.textContent = value;
  return e;
}
function meter(id, value) { $(id).style.width = `${Math.max(0, Math.min(100, value ?? 0))}%`; }
function big(id, value, unit) {
  $(id).replaceChildren(document.createTextNode(number(value)), node("small", "", value == null ? "" : unit));
}
function setConnection(live, message) {
  text("connection", message);
  $("connection-dot").classList.toggle("live", live);
  $("stale").hidden = live || !snapshot;
}
function connect() {
  clearTimeout(reconnectTimer);
  socket = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`);
  socket.onmessage = event => {
    let message;
    try { message = JSON.parse(event.data); } catch { return; }
    if (message.type === "hello") {
      facts = message.facts;
      history = message.history;
      text("hostname", facts.hostname);
      text("machine", `${facts.cpu} · ${facts.physical_cores ?? "?"} Kerne / ${facts.threads} Threads`);
      text("core-count", `${facts.threads} THREADS`);
      document.querySelector(".chart-footer span:last-child").textContent = `${number(facts.interval_ms / 1000)} Sekunde${facts.interval_ms === 1000 ? "" : "n"} · live`;
    } else if (message.type === "sample") {
      if (!snapshot || message.snapshot.point.at_ms !== snapshot.point.at_ms) lastArrival = Date.now();
      snapshot = message.snapshot;
      retry = 1000;
      const point = snapshot.point;
      // The hello history and latest snapshot can contain the same sample.
      if (!history.length || history.at(-1).at_ms < point.at_ms) history.push(point);
      while (history.length && history[0].at_ms < point.at_ms - 3600000) history.shift();
      const fresh = Date.now()-lastArrival <= Math.max(5000,(facts?.interval_ms??1000)*3);
      setConnection(fresh, fresh ? "Live verbunden" : "Messwerte veraltet");
      if (!document.hidden) render();
    }
  };
  socket.onclose = () => {
    setConnection(false, "Verbinde erneut …");
    reconnectTimer = setTimeout(connect, retry);
    retry = Math.min(15000, retry * 1.5);
  };
  socket.onerror = () => socket.close();
}

function render() {
  if (!snapshot) return;
  const s = snapshot, p = s.point, g = s.gpus[0];
  big("cpu-value", p.cpu, "%"); big("gpu-value", p.gpu, "%");
  big("ram-value", s.ram_used / 2**30, "GiB"); big("vram-value", g?.memory_used == null ? null : g.memory_used / 2**30, "GiB");
  for (const key of ["cpu","gpu","ram","vram"]) meter(`${key}-meter`, p[key]);
  text("cpu-caption", `${facts?.physical_cores ?? "?"} Kerne · ${facts?.threads ?? "?"} Threads`);
  text("gpu-caption", g?.name ?? s.gpu_status);
  text("ram-caption", `${percent(p.ram)} von ${bytes(s.ram_total)}`);
  text("vram-caption", g ? `${percent(p.vram)} von ${bytes(g.memory_total)}` : "Keine NVIDIA-GPU erkannt");
  text("power-value", p.gpu_watts == null ? "—" : `${number(p.gpu_watts)} W`);
  text("cpu-watts", p.cpu_watts == null ? "nicht verfügbar" : `${number(p.cpu_watts)} W`);
  text("power-note", `${s.cpu_power_status}. GPU-Werte enthalten nicht den Verbrauch des gesamten PCs.`);
  text("uptime", `Online seit ${Math.floor(s.uptime_secs / 86400)} T. ${Math.floor(s.uptime_secs % 86400 / 3600)} Std.`);
  text("load", s.load.map(number).join(" / "));
  text("swap", `${bytes(s.swap_used)} / ${bytes(s.swap_total)}`);
  text("rx-value", rate(p.net_rx)); text("tx-value", rate(p.net_tx));
  text("network-caption", `Physische Anschlüsse im Diagramm · Interfaces: ${s.networks.map(n => n.name).join(", ") || "keine"}`);
  text("sample-time", `${time(p.at_ms)} · Messung ${number(s.sample_ms)} ms`);
  text("history-caption", `${number(Math.min(windowSecs, (p.at_ms - (history[0]?.at_ms ?? p.at_ms)) / 1000) / 60)} Min. Verlauf vorhanden`);
  renderProcesses(); renderHardware(); charts.forEach(c => c.draw());
}

function gpuValue(p, key) {
  const values = p.gpu.map(g => g[key]);
  if (!values.length || values.some(v => v == null)) return null;
  // Sum VRAM over devices, but max utilization (percentages are per device).
  return key === "vram" ? values.reduce((a,b) => a+b, 0) : Math.max(...values);
}
function sortValue(p, key) {
  if (key === "vram") return gpuValue(p,"vram") ?? -1;
  if (key === "gpu") return gpuValue(p,"utilization") ?? -1;
  if (key === "io") return p.read_bps + p.write_bps;
  return p[key];
}
function renderProcesses() {
  if (!snapshot) return;
  const query = $("search").value.trim().toLocaleLowerCase();
  const all = snapshot.processes.filter(p => (!query || `${p.name} ${p.pid}`.toLocaleLowerCase().includes(query)) &&
    (filter !== "gpu" || p.gpu.length) && (filter !== "active" || p.cpu >= 0.5 || p.gpu.length || p.read_bps + p.write_bps > 1024));
  all.sort((a,b) => {
    const x = sortValue(a,sortKey), y = sortValue(b,sortKey);
    const delta = typeof x === "string" ? x.localeCompare(y) : x-y;
    return (descending ? -delta : delta) || a.pid-b.pid;
  });
  const rows = all.slice(0,limit).map(p => {
    const row = node("tr"), first = node("td"), name = node("span", "proc-name", p.name);
    name.title = p.name;
    first.append(name,node("small", p.gpu.length ? "process-gpu" : "", `PID ${p.pid}${p.gpu.length ? ` · GPU ${p.gpu.map(g=>g.index).join(", ")}${p.gpu.some(g=>g.compute) ? " · CUDA" : " · Grafik"}` : ""}`));
    row.append(first,node("td","cpu-cell",percent(p.cpu)),node("td","",bytes(p.ram)),node("td","process-gpu",percent(gpuValue(p,"utilization"))),node("td","",bytes(gpuValue(p,"vram"))));
    const io = node("td","",`↓ ${rate(p.read_bps)}`); io.append(node("small","",`↑ ${rate(p.write_bps)}`));row.append(io);
    return row;
  });
  if (!rows.length) { const r = node("tr"), cell = node("td","empty",filter === "gpu" ? "Keine GPU-Prozesse gemeldet" : "Keine passenden Prozesse"); cell.colSpan=6; r.append(cell); rows.push(r); }
  $("process-body").replaceChildren(...rows);
  text("process-count",`${all.length}`);
  text("process-caption",`${Math.min(limit,all.length)} / ${all.length} · 100 % CPU = ein logischer Kern`);
  $("show-more").hidden = all.length <= limit;
}

function renderHardware() {
  const s = snapshot;
  if ($("cores").children.length !== s.cores.length) $("cores").replaceChildren(...s.cores.map(()=>node("div","core")));
  s.cores.forEach((usage,i) => {
    const cell = $("cores").children[i]; cell.textContent = `${Math.round(usage)}%`;
    cell.title = `CPU ${i} · ${percent(usage)}`;
    cell.setAttribute("aria-label",cell.title);
    cell.style.backgroundColor = `rgba(108,226,188,${0.055+usage/100*0.4})`;
    cell.style.color = usage > 60 ? "#eafff7" : "#90b9ab";
  });
  const temps = [...s.temperatures, ...s.gpus.filter(g=>g.temperature != null).map(g=>({label:`GPU ${g.index}`,celsius:g.temperature}))];
  const primary = temps.filter(t=>/Tctl|Package|Motherboard|Chipset|Composite|^GPU /.test(t.label));
  const visible = allSensors ? temps : primary.length ? primary : temps.slice(0,6);
  $("show-sensors").hidden = temps.length <= (primary.length || 6);
  text("show-sensors",allSensors ? "Weniger anzeigen" : `Alle ${temps.length} Sensoren`);
  $("temperatures").replaceChildren(...visible.map(t=>{
    const label=t.label.replace(/^k10temp Tctl$/,"CPU · Tctl").replace(/^asusec Motherboard$/,"Mainboard").replace(/^asusec Chipset$/,"Chipsatz").replace(/^nvme Composite /,"SSD · ");
    const e=node("div",`sensor${t.celsius>=80 ? " hot" : ""}`);e.title=t.label;e.append(node("span","",label),node("strong","",`${number(t.celsius)}°`));return e;
  }));
  if (!temps.length) $("temperatures").append(node("p","muted-note","Keine Temperatursensoren verfügbar"));
  text("gpu-count",`${s.gpus.length}`);
  $("gpu-details").replaceChildren(...s.gpus.map(g=>{
    const e=node("div","gpu-item");e.append(node("strong","",`${g.index} · ${g.name}`),node("p","",`${percent(g.utilization)} · ${bytes(g.memory_used)} / ${bytes(g.memory_total)}`),node("p","",`${g.watts == null ? "—" : number(g.watts)+" W"} · Limit ${number(g.power_limit)} W · Lüfter ${percent(g.fan)}`),node("p","",g.process_status));return e;
  }));
  if (!s.gpus.length) $("gpu-details").append(node("p","muted-note",s.gpu_status));
  $("disks").replaceChildren(...s.disks.map(d=>{
    const e=node("div","disk-row"), heading=node("div"), bar=node("div","meter"), fill=node("i");
    const used=d.total-d.available, pct=d.total ? used/d.total*100 : 0;
    heading.append(node("strong","",d.mount),node("span","",`${bytes(used)} / ${bytes(d.total)}`));
    fill.style.width=`${pct}%`;if(pct>90)fill.style.backgroundColor=colors.vram;bar.append(fill);
    e.append(heading,bar,node("small","",`${bytes(d.available)} verfügbar · ${percent(pct)} belegt`));return e;
  }));
}

class Chart {
  constructor(id, keys, unit, fixedMax) {
    this.canvas=$(id+"-chart");this.tooltip=$(id+"-tooltip");this.keys=keys;this.unit=unit;this.fixedMax=fixedMax;this.pointer=null;
    this.canvas.addEventListener("pointermove", e=>{this.pointer=e.clientX-this.canvas.getBoundingClientRect().left;this.draw();});
    this.canvas.addEventListener("pointerleave",()=>{this.pointer=null;this.tooltip.hidden=true;this.draw();});
    new ResizeObserver(()=>this.draw()).observe(this.canvas);
    charts.push(this);
  }
  format(v){return this.unit === "rate" ? rate(v) : `${number(v)} ${this.unit}`;}
  draw() {
    const canvas=this.canvas, w=canvas.clientWidth, h=canvas.clientHeight;
    if(!w||!h)return;
    const dpr=Math.min(window.devicePixelRatio||1,2);
    if(canvas.width!==Math.round(w*dpr)||canvas.height!==Math.round(h*dpr)){canvas.width=Math.round(w*dpr);canvas.height=Math.round(h*dpr);}
    const ctx=canvas.getContext("2d");ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,w,h);
    const right=w-6, left=this.unit==="rate"?65:36, top=8, bottom=h-24, end=snapshot?.point.at_ms ?? Date.now(), start=end-windowSecs*1000;
    const points=history.filter(p=>p.at_ms>=start&&p.at_ms<=end);
    let maximum=this.fixedMax||1;
    if(!this.fixedMax)for(const p of points)for(const key of this.keys)if(p[key]!=null)maximum=Math.max(maximum,p[key]*1.15);
    const x=t=>left+(t-start)/(end-start)*(right-left), y=v=>bottom-Math.max(0,Math.min(maximum,v))/maximum*(bottom-top);
    ctx.font="9px system-ui";ctx.lineWidth=1;
    for(let i=0;i<=2;i++) {
      const value=maximum*i/2, yy=y(value);ctx.strokeStyle="#283442";ctx.beginPath();ctx.moveTo(left,yy);ctx.lineTo(right,yy);ctx.stroke();ctx.fillStyle="#8394a7";ctx.textAlign="right";
      ctx.fillText(this.unit==="rate"?bytes(value):`${Math.round(value)}${this.unit==="%"?"%":""}`,left-8,yy+3);
    }
    ctx.fillStyle="#8394a7";
    for(let i=0;i<=2;i++){ctx.textAlign=i===0?"left":i===2?"right":"center";ctx.fillText(new Date(start+i*(end-start)/2).toLocaleTimeString("de-DE",{hour:"2-digit",minute:"2-digit"}),x(start+i*(end-start)/2),h-4);}
    for(const key of this.keys) {
      ctx.strokeStyle=colors[key];ctx.lineWidth=1.65;ctx.beginPath();let connected=false,last=0;
      for(const p of points) {
        if(p[key]==null){connected=false;continue;}
        if(!connected||p.at_ms-last>Math.max(3000,(facts?.interval_ms??1000)*2.5))ctx.moveTo(x(p.at_ms),y(p[key]));else ctx.lineTo(x(p.at_ms),y(p[key]));
        connected=true;last=p.at_ms;
      }ctx.stroke();
      const final=points.at(-1);if(final?.[key]!=null){ctx.fillStyle=colors[key];ctx.beginPath();ctx.arc(x(final.at_ms),y(final[key]),2.5,0,Math.PI*2);ctx.fill();}
    }
    if(!points.length){ctx.fillStyle="#91a0b1";ctx.textAlign="center";ctx.fillText("Verlauf wird aufgebaut …",w/2,h/2);}
    if(this.pointer!=null&&points.length) {
      const target=start+(this.pointer-left)/(right-left)*(end-start);
      const point=points.reduce((a,b)=>Math.abs(a.at_ms-target)<Math.abs(b.at_ms-target)?a:b);
      const xx=x(point.at_ms);ctx.strokeStyle="#a6b4c566";ctx.beginPath();ctx.moveTo(xx,top);ctx.lineTo(xx,bottom);ctx.stroke();
      this.tooltip.textContent=[time(point.at_ms),...this.keys.map(k=>`${labels[k]}: ${point[k]==null?"—":this.format(point[k])}`)].join("\n");
      this.tooltip.hidden=false;this.tooltip.style.left=`${Math.min(Math.max(left,xx+10),Math.max(left,w-160))}px`;
    }
  }
}
new Chart("activity",["cpu","gpu","ram","vram"],"%",100);
new Chart("power",["gpu_watts","cpu_watts"],"W");
new Chart("network",["net_rx","net_tx"],"rate");
document.querySelectorAll("[data-window]").forEach(button=>button.addEventListener("click",()=>{
  windowSecs=Number(button.dataset.window);document.querySelectorAll("[data-window]").forEach(b=>b.setAttribute("aria-pressed",String(b===button)));render();
}));
document.querySelectorAll("[data-filter]").forEach(button=>button.addEventListener("click",()=>{
  filter=button.dataset.filter;limit=12;document.querySelectorAll("[data-filter]").forEach(b=>b.setAttribute("aria-pressed",String(b===button)));renderProcesses();
}));
document.querySelectorAll("[data-sort]").forEach(button=>button.addEventListener("click",()=>{
  const key=button.dataset.sort;descending=sortKey===key?!descending:key!=="name";sortKey=key;
  document.querySelectorAll("[data-sort]").forEach(b=>{b.textContent=b.textContent.replace(/ [↑↓]$/,"");b.parentElement.removeAttribute("aria-sort");});
  button.textContent+=descending?" ↓":" ↑";button.parentElement.setAttribute("aria-sort",descending?"descending":"ascending");renderProcesses();
}));
$("search").addEventListener("input",()=>{limit=12;renderProcesses();});
$("show-more").addEventListener("click",()=>{limit+=50;renderProcesses();});
$("show-sensors").addEventListener("click",()=>{allSensors=!allSensors;renderHardware();});
document.addEventListener("visibilitychange",()=>{if(!document.hidden)render();});
setInterval(()=>{
  if(snapshot&&Date.now()-lastArrival>Math.max(5000,(facts?.interval_ms??1000)*3)){
    setConnection(false,"Messwerte veraltet");if(socket?.readyState===WebSocket.OPEN)socket.close();
  }
},2000);
window.addEventListener("pagehide",()=>{socket.onclose=null;socket.close();clearTimeout(reconnectTimer);});
window.addEventListener("pageshow",e=>{if(e.persisted)connect();});
connect();
