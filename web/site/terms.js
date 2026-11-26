// The gateway's terms of use (nutzungsbedingungen.html). It asks the gateway
// that serves this page for the terms in force, in the page's language
// (GET /terms?lang=, same origin; docs/protocol.md §"Terms of use (WG-1)"),
// and draws them with the Markdown subset the client draws
// (crates/baylee-client-core/src/terms.rs): `#`/`##` headings, paragraphs,
// **bold**, *italic*/_italic_, `-`/`*`/`+` and `1.` lists, `---` rules,
// links as [text](url) and <url>, and here also a bare https:// address.
// Every word becomes a text node, never markup: nothing in the text becomes
// HTML, and an address becomes a link only when it is https: or mailto:.
// A switch of language (legal.js sets data-lang) asks again; an answer for
// a language no longer shown is kept for later and not drawn. The page's
// CSP allows no inline script, so it lives in this file.
(function () {
  "use strict";
  var root = document.documentElement;
  var box = document.getElementById("terms-text");
  var loading = document.getElementById("terms-loading");
  var failed = document.getElementById("terms-failed");
  if (!box || !window.fetch) return;

  var MAX = 1 << 20; // a text larger than this is not a gateway's terms
  var cache = {};
  var drawn = null;
  var scrolled = false;

  function lang() { return root.getAttribute("data-lang") === "de" ? "de" : "en"; }

  // ---------------------------------------------------------------- inline
  // A span: {text, bold, italic, href}. `href` only on a link.
  var ALNUM = /[\p{L}\p{N}]/u;
  function inline(text) {
    var spans = [], bold = false, italic = false, word = "", i = 0, c;
    function push() {
      if (!word) return;
      var last = spans[spans.length - 1];
      if (last && !last.href && last.bold === bold && last.italic === italic) last.text += word;
      else spans.push({ text: word, bold: bold, italic: italic, href: null });
      word = "";
    }
    function link(label, href) {
      push();
      spans.push({ text: label, bold: bold, italic: italic, href: href });
    }
    var chars = Array.from(text);
    while (i < chars.length) {
      c = chars[i];
      if (c === "\\" && i + 1 < chars.length && /^[!-\/:-@\[-`{-~]$/.test(chars[i + 1])) {
        word += chars[i + 1];
        i += 2;
      } else if (c === "*" && chars[i + 1] === "*") {
        push(); bold = !bold; i += 2;
      } else if ((c === "*" || c === "_") &&
                 (c === "*" || !(i > 0 && ALNUM.test(chars[i - 1]) && i + 1 < chars.length && ALNUM.test(chars[i + 1])))) {
        push(); italic = !italic; i += 1;
      } else if (c === "[") {
        var l = linkAt(chars, i);
        if (l) {
          if (safe(l.url)) link(l.label, l.url);
          else word += l.url && l.url !== l.label ? l.label + " (" + l.url + ")" : l.label;
          i = l.next;
        } else { word += c; i += 1; }
      } else if (c === "<") {
        var end = chars.indexOf(">", i + 1);
        var inner = end > i ? chars.slice(i + 1, end).join("") : "";
        if (/^(https?:\/\/|mailto:)/.test(inner)) {
          if (safe(inner)) link(inner, inner); else word += inner;
          i = end + 1;
        } else { word += c; i += 1; }
      } else { word += c; i += 1; }
    }
    push();
    return spans;
  }

  function linkAt(chars, at) {
    var close = chars.indexOf("]", at + 1);
    if (close < 0 || chars[close + 1] !== "(") return null;
    var end = chars.indexOf(")", close + 2);
    if (end < 0) return null;
    return {
      label: chars.slice(at + 1, close).join(""),
      url: chars.slice(close + 2, end).join("").trim(),
      next: end + 1
    };
  }

  // https: and mailto: only, and nothing that could leave the attribute.
  function safe(url) {
    return /^(https:\/\/[^\s<>"'`\\]+|mailto:[^\s<>"'`\\:\/]+@[^\s<>"'`\\\/]+)$/.test(url);
  }

  // ---------------------------------------------------------------- blocks
  function heading(line) {
    var m = /^(#{1,6}) (.*)$/.exec(line);
    if (!m) return null;
    return { level: m[1].length === 1 ? 1 : 2, text: m[2].trim().replace(/#+$/, "").trim() };
  }
  function isRule(line) {
    var compact = line.replace(/\s+/g, "");
    return compact.length >= 3 && /^(-+|\*+|_+)$/.test(compact);
  }
  function parse(markdown) {
    var blocks = [], paragraph = [];
    function flush() {
      if (paragraph.length) { blocks.push({ kind: "p", spans: inline(paragraph.join(" ")) }); paragraph = []; }
    }
    var lines = markdown.split(/\r?\n/);
    for (var n = 0; n < lines.length; n++) {
      var raw = lines[n], line = raw.trim(), h, m, last;
      if (line.indexOf("<!--") === 0 && line.slice(-3) === "-->") { flush(); continue; }
      if (!line) { flush(); continue; }
      if (isRule(line)) { flush(); blocks.push({ kind: "hr" }); continue; }
      if ((h = heading(line))) { flush(); blocks.push({ kind: "h", level: h.level, spans: inline(h.text) }); continue; }
      if ((m = /^[-*+] (.*)$/.exec(line))) { flush(); blocks.push({ kind: "li", number: null, spans: inline(m[1].trim()) }); continue; }
      if ((m = /^(\d{1,9})[.)] (.*)$/.exec(line))) { flush(); blocks.push({ kind: "li", number: +m[1], spans: inline(m[2].trim()) }); continue; }
      last = blocks[blocks.length - 1];
      if (/^[ \t]/.test(raw) && !paragraph.length && last && last.kind === "li") {
        last.spans = last.spans.concat([{ text: " ", bold: false, italic: false, href: null }], inline(line));
        continue;
      }
      paragraph.push(line);
    }
    flush();
    return blocks;
  }

  // ---------------------------------------------------------------- drawing
  var BARE = /https:\/\/[^\s<>"'`\\]+/g;
  function words(parent, text) {
    // A bare https:// address in the words becomes a link; trailing
    // punctuation stays a word.
    var at = 0, m;
    BARE.lastIndex = 0;
    while ((m = BARE.exec(text))) {
      var url = m[0].replace(/[.,;:!?)\]»“”"']+$/, "");
      if (url.length <= "https://".length) continue;
      if (m.index > at) parent.appendChild(document.createTextNode(text.slice(at, m.index)));
      parent.appendChild(anchor(url, url));
      at = m.index + url.length;
      BARE.lastIndex = at;
    }
    if (at < text.length) parent.appendChild(document.createTextNode(text.slice(at)));
  }
  function anchor(href, label) {
    var a = document.createElement("a");
    var origin = null;
    try { origin = new URL(href).origin; } catch (err) { /* not an address: words */ }
    if (origin === null && href.indexOf("mailto:") !== 0) {
      a = document.createElement("span");
    } else {
      a.href = href;
      if (origin && origin !== location.origin) a.rel = "noopener";
    }
    a.textContent = label;
    return a;
  }
  function spans(parent, list) {
    for (var i = 0; i < list.length; i++) {
      var s = list[i], into = parent;
      if (s.bold) { var b = document.createElement("strong"); into.appendChild(b); into = b; }
      if (s.italic) { var e = document.createElement("em"); into.appendChild(e); into = e; }
      if (s.href) into.appendChild(anchor(s.href, s.text));
      else words(into, s.text);
    }
  }
  // Anchors to link a section: "## 16. …" is #s16, "**16.3 …**" #s16-3.
  function sectionId(list) {
    var first = list[0] && list[0].text;
    var m = first && /^(\d{1,2}[a-z]?)\.(\d{1,2})?(\s|$)/.exec(first);
    return m ? "s" + m[1] + (m[2] ? "-" + m[2] : "") : null;
  }
  function draw(doc, l) {
    var frag = document.createDocumentFragment(), list = null, listKind = null;
    var blocks = parse(doc.markdown), used = {};
    for (var i = 0; i < blocks.length; i++) {
      var b = blocks[i], el, id;
      if (b.kind === "li") {
        var kind = b.number === null ? "ul" : "ol";
        if (!list || listKind !== kind) {
          list = document.createElement(kind);
          listKind = kind;
          if (kind === "ol" && b.number !== 1) list.start = b.number;
          frag.appendChild(list);
        }
        el = document.createElement("li");
        spans(el, b.spans);
        list.appendChild(el);
        continue;
      }
      list = null;
      if (b.kind === "hr") { frag.appendChild(document.createElement("hr")); continue; }
      el = document.createElement(b.kind === "h" ? (b.level === 1 ? "h2" : "h3") : "p");
      spans(el, b.spans);
      id = (b.kind === "h" && b.level === 2) || (b.kind === "p" && b.spans[0] && b.spans[0].bold) ? sectionId(b.spans) : null;
      if (id && !used[id]) { el.id = id; used[id] = true; }
      frag.appendChild(el);
    }
    box.textContent = "";
    box.setAttribute("lang", doc.lang || l);
    box.appendChild(frag);
    drawn = l;
    if (!scrolled && location.hash.length > 1) {
      scrolled = true;
      var target = document.getElementById(decodeURIComponent(location.hash.slice(1)));
      // instant: a smooth scroll over the whole text would only be in the way
      if (target && box.contains(target)) target.scrollIntoView({ block: "start", behavior: "instant" });
    }
  }

  // ---------------------------------------------------------------- loading
  function state(which) {
    if (loading) loading.hidden = which !== "loading";
    if (failed) failed.hidden = which !== "failed";
    box.setAttribute("aria-busy", which === "loading" ? "true" : "false");
  }
  function load() {
    var l = lang();
    if (drawn === l) return;
    if (cache[l]) { draw(cache[l], l); state("done"); return; }
    state("loading");
    fetch("/terms?lang=" + l, { credentials: "omit", cache: "no-cache", headers: { Accept: "application/json" } })
      .then(function (r) {
        if (!r.ok) throw new Error("HTTP " + r.status);
        return r.json();
      })
      .then(function (doc) {
        if (!doc || typeof doc.markdown !== "string" || doc.markdown.length > MAX) throw new Error("not terms");
        cache[l] = doc;
        if (lang() !== l) return;
        draw(doc, l);
        state("done");
      })
      .catch(function () {
        if (lang() === l) state("failed");
      });
  }

  if (window.MutationObserver) {
    new MutationObserver(load).observe(root, { attributes: true, attributeFilter: ["data-lang"] });
  }
  load();
})();
