// Baylee's Impressum (web/site/impressum.html). Two jobs:
//
//   1. the language, as on the landing page: `?lang=de|en`, then what this
//      browser chose last time (the same localStorage key as site.js), then
//      the browser's languages;
//   2. the provider's details. They are in no file of the repository and in
//      nothing a crawler reads as text. On the visitor's click ("show the
//      details") the browser solves a small proof of work in a worker
//      (impressum-worker.js) and the feedback service hands out the blob
//      for it (/site/impressum, proxied by the site; crates/baylee-feedback/
//      src/impressum.rs); this script decodes it and puts the characters
//      into the page one by one, readable on screen and scrambled for
//      anything that reads the DOM as text ("the page", below); copying and
//      the mailto:/tel: links happen only on a click.
//
// The blob is written by scripts/site/impressum_blob.py, which says how it is
// built; the key is split between SCRIPT_KEY here and a nonce in the blob.
// All of it keeps away harvesters that grep for addresses, read the DOM or
// walk many pages, and nobody who renders the page and reads the pixels; it
// is not meant to. Without JavaScript, or when the details do not come, the
// page says to turn JavaScript on or use the contact form.
//
// Nothing here identifies the visitor to anyone. The page's CSP allows no
// inline script, so everything lives in this file.
(function () {
  "use strict";

  var MAGIC = "BYIM";
  var VERSION = 2;
  var NONCE_LEN = 16;
  // the script's half of the key; scripts/site/impressum_blob.py carries the
  // same 32 hex digits (a test holds the two equal)
  var SCRIPT_KEY = "6b1f0c9e52d84a37b0e1f6a2c9d35e48";

  var root = document.documentElement;
  var query = new URLSearchParams(location.search);
  var lang = "en";
  root.setAttribute("data-js", "");

  // ------------------------------------------------------------ language
  var theme = query.get("theme");
  if (theme === "dark" || theme === "light") root.setAttribute("data-theme", theme);

  function storedLang() {
    try { return localStorage.getItem("baylee-site-lang"); } catch (e) { return null; }
  }
  function rememberLang(l) {
    try { localStorage.setItem("baylee-site-lang", l); } catch (e) { /* private mode, blocked storage: fine */ }
  }
  function pickLang() {
    var asked = query.get("lang");
    if (asked === "de" || asked === "en") return asked;
    var kept = storedLang();
    if (kept === "de" || kept === "en") return kept;
    var langs = navigator.languages || [navigator.language || "en"];
    for (var i = 0; i < langs.length; i++) {
      var l = String(langs[i]).toLowerCase();
      if (l.indexOf("de") === 0) return "de";
      if (l.indexOf("en") === 0) return "en";
    }
    return "en";
  }
  var TITLES = { en: "Legal notice – Baylee", de: "Impressum – Baylee" };
  function applyLang(l) {
    lang = l;
    root.setAttribute("data-lang", l);
    root.setAttribute("lang", l);
    document.title = TITLES[l];
    var buttons = document.querySelectorAll(".lang-btn");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].getAttribute("data-lang") === l ? "true" : "false");
    }
  }
  applyLang(pickLang());
  document.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest(".lang-btn");
    if (b) { applyLang(b.getAttribute("data-lang")); rememberLang(lang); }
  });

  // ------------------------------------------------------------ helpers
  function el(tag, attrs, children) {
    var n = document.createElement(tag);
    if (attrs) for (var k in attrs) {
      if (k === "text") n.textContent = attrs[k];
      else if (k === "class") n.className = attrs[k];
      else n.setAttribute(k, attrs[k]);
    }
    if (children) for (var i = 0; i < children.length; i++) {
      if (children[i] == null) continue;
      n.appendChild(typeof children[i] === "string" ? document.createTextNode(children[i]) : children[i]);
    }
    return n;
  }
  // a bilingual run: two spans the CSS shows one of
  function t(en, de) {
    var s = document.createDocumentFragment();
    s.appendChild(el("span", { class: "en", text: en }));
    var d = el("span", { class: "de", text: de });
    d.setAttribute("lang", "de");
    s.appendChild(d);
    return s;
  }
  function hexBytes(hex) {
    var out = new Uint8Array(hex.length / 2);
    for (var i = 0; i < out.length; i++) out[i] = parseInt(hex.substr(i * 2, 2), 16);
    return out;
  }
  function b64urlBytes(text) {
    var s = text.trim().replace(/-/g, "+").replace(/_/g, "/");
    while (s.length % 4) s += "=";
    var bin = atob(s);
    var out = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  // ------------------------------------------------------------ the blob
  // keystream block i = SHA-256(SCRIPT_KEY | nonce | i as 4 bytes big-endian)
  function keystream(nonce, length) {
    var key = hexBytes(SCRIPT_KEY);
    var blocks = [];
    for (var i = 0; i * 32 < length; i++) {
      var input = new Uint8Array(key.length + nonce.length + 4);
      input.set(key, 0);
      input.set(nonce, key.length);
      new DataView(input.buffer).setUint32(key.length + nonce.length, i, false);
      blocks.push(crypto.subtle.digest("SHA-256", input));
    }
    return Promise.all(blocks).then(function (parts) {
      var out = new Uint8Array(parts.length * 32);
      for (var j = 0; j < parts.length; j++) out.set(new Uint8Array(parts[j]), j * 32);
      return out.subarray(0, length);
    });
  }
  function decode(text) {
    var raw = b64urlBytes(text);
    var head = MAGIC.length + 1 + NONCE_LEN;
    if (raw.length <= head || String.fromCharCode.apply(null, raw.subarray(0, MAGIC.length)) !== MAGIC || raw[MAGIC.length] !== VERSION) {
      return Promise.reject(new Error("not an Impressum blob"));
    }
    var nonce = raw.subarray(MAGIC.length + 1, head), body = raw.subarray(head);
    return keystream(nonce, body.length).then(function (ks) {
      var plain = new Uint8Array(body.length);
      for (var i = 0; i < body.length; i++) plain[i] = body[i] ^ ks[i];
      var doc = JSON.parse(new TextDecoder("utf-8").decode(plain));
      if (!doc || doc.v !== VERSION || !doc.fields) throw new Error("unexpected document");
      return doc.fields;
    });
  }

  // ------------------------------------------------------------ the page
  // Nothing below ever holds a value as one string, and the page never does
  // either: each character is a span of its own, carrying its position as an
  // order class (.o0 … .o63, site.css), and the spans of a field go into the
  // DOM shuffled, among decoys (.scr-x) that take no room on screen but are
  // in textContent and innerText like the rest. The flexbox puts the real
  // characters back in their order for the eye; a crawler that reads the DOM
  // as text, the accessibility tree included (the block is aria-hidden),
  // gets a jumble. The characters are put back together only in a click
  // handler, for the clipboard or a mailto:/tel: navigation, and dropped
  // with it: no href keeps an address.
  var DECOYS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ@.-+";
  for (var dg = 0; dg < 10; dg++) DECOYS += dg; // (no digit run in this file)
  var MAX_LEN = 64;

  function rand(n) {
    var a = new Uint32Array(1);
    crypto.getRandomValues(a);
    return a[0] % n;
  }
  function shuffle(a) {
    for (var i = a.length - 1; i > 0; i--) {
      var j = rand(i + 1), x = a[i];
      a[i] = a[j];
      a[j] = x;
    }
    return a;
  }
  // one field, [[position, code point], …], as a scrambled run
  function scramble(pairs) {
    var run = el("span", { class: "scr" });
    var nodes = [];
    for (var i = 0; i < pairs.length; i++) {
      var pos = pairs[i][0], cp = pairs[i][1];
      if (typeof pos !== "number" || typeof cp !== "number" || pos < 0 || pos >= MAX_LEN) throw new Error("bad field");
      nodes.push(el("span", { class: "o" + pos, text: String.fromCodePoint(cp) }));
    }
    var decoys = 2 + Math.ceil(pairs.length / 2) + rand(3);
    for (var d = 0; d < decoys; d++) {
      nodes.push(el("span", { class: "scr-x o" + rand(MAX_LEN), text: DECOYS.charAt(rand(DECOYS.length)) }));
    }
    shuffle(nodes);
    for (var k = 0; k < nodes.length; k++) run.appendChild(nodes[k]);
    return run;
  }
  // a run read back in its visual order, for a click handler only
  function readRun(run) {
    var chars = [];
    for (var i = 0; i < run.children.length; i++) {
      var c = run.children[i];
      if (c.classList.contains("scr-x")) continue;
      var m = /(?:^|\s)o(\d+)(?:\s|$)/.exec(c.className);
      if (m) chars[+m[1]] = c.textContent;
    }
    return chars.join("");
  }
  // a block's lines as they show now: the runs of one line joined by a space,
  // the other language's runs left out
  function readLines(block) {
    var out = [], lines = block.querySelectorAll(".imp-line");
    for (var i = 0; i < lines.length; i++) {
      var runs = lines[i].querySelectorAll(".scr"), words = [];
      for (var j = 0; j < runs.length; j++) {
        var other = runs[j].closest(lang === "de" ? ".en" : ".de");
        if (!other || !lines[i].contains(other)) words.push(readRun(runs[j]));
      }
      out.push(words.join(" "));
    }
    return out.join("\n");
  }
  function line(runs) {
    var div = el("div", { class: "imp-line" });
    for (var i = 0; i < runs.length; i++) {
      if (i) div.appendChild(document.createTextNode(" "));
      div.appendChild(runs[i]);
    }
    return div;
  }
  function langRuns(en, de) {
    var s = el("span", { class: "en" }, [en]);
    var d = el("span", { class: "de", lang: "de" }, [de]);
    return [s, d];
  }
  function button(label, onClick) {
    var b = el("button", { type: "button", class: "button button-ghost imp-btn" }, [label]);
    b.addEventListener("click", onClick);
    return b;
  }
  function copier(status, read) {
    return function () {
      var say = function (en, de) { clear(status); status.appendChild(t(en, de)); };
      if (!navigator.clipboard || !navigator.clipboard.writeText) {
        say("Copying is not possible in this browser; please use the contact form.", "Kopieren geht in diesem Browser nicht; bitte nutze das Kontaktformular.");
        return;
      }
      navigator.clipboard.writeText(read()).then(
        function () { say("Copied.", "Kopiert."); },
        function () { say("Copying did not work; please use the contact form.", "Kopieren hat nicht geklappt; bitte nutze das Kontaktformular."); }
      );
    };
  }
  function clear(n) { while (n.firstChild) n.removeChild(n.firstChild); }

  function render(f) {
    var box = document.getElementById("imp-provider");
    var contact = document.getElementById("imp-contact");
    if (!box || !contact) return;

    var address = el("div", { class: "imp-address scr-block", "aria-hidden": "true" }, [
      line([scramble(f.name)]),
      line([scramble(f.street)]),
      line([scramble(f.postcode), scramble(f.city)]),
      el("div", { class: "imp-line" }, langRuns(scramble(f.country_en), scramble(f.country_de)))
    ]);
    var boxStatus = el("p", { class: "imp-status", role: "status", "aria-live": "polite" });
    box.appendChild(el("p", { class: "vh" }, [t(
      "The address is drawn so that address harvesters cannot read it, and a screen reader cannot either. The button below copies it to the clipboard; or write to me with the contact form on the start page.",
      "Die Anschrift ist so gezeichnet, dass Adresssammler sie nicht lesen können, und deshalb auch kein Screenreader. Der Knopf darunter kopiert sie in die Zwischenablage; oder schreib mir über das Kontaktformular auf der Startseite."
    )]));
    box.appendChild(address);
    box.appendChild(el("div", { class: "imp-actions" }, [
      button(t("Copy address", "Anschrift kopieren"), copier(boxStatus, function () { return readLines(address); }))
    ]));
    box.appendChild(boxStatus);

    var mail = el("span", { class: "scr-block", "aria-hidden": "true" }, [scramble(f.email)]);
    var phone = el("span", { class: "scr-block", "aria-hidden": "true" }, [scramble(f.phone)]);
    var contactStatus = el("p", { class: "imp-status", role: "status", "aria-live": "polite" });
    contact.appendChild(el("dl", { class: "imp-dl" }, [
      el("dt", {}, [t("E-mail", "E-Mail")]),
      el("dd", {}, [mail, el("span", { class: "imp-actions" }, [
        button(t("Write an e-mail", "E-Mail schreiben"), function () { location.href = "mailto:" + readRun(mail.firstChild); }),
        button(t("Copy e-mail address", "E-Mail-Adresse kopieren"), copier(contactStatus, function () { return readRun(mail.firstChild); }))
      ])]),
      el("dt", {}, [t("Telephone", "Telefon")]),
      el("dd", {}, [phone, el("span", { class: "imp-actions" }, [
        button(t("Call", "Anrufen"), function () { location.href = "tel:" + readRun(phone.firstChild).replace(/[^+\d]/g, ""); }),
        button(t("Copy number", "Nummer kopieren"), copier(contactStatus, function () { return readRun(phone.firstChild); }))
      ])])
    ]));
    contact.appendChild(contactStatus);
    root.classList.add("imp-loaded");
  }

  function failed() {
    var notes = document.querySelectorAll(".imp-failed");
    for (var i = 0; i < notes.length; i++) notes[i].hidden = false;
  }

  // ------------------------------------------------------------ the puzzle
  // The details come from the feedback service, for a solved proof of work
  // (crates/baylee-feedback/src/impressum.rs): a second of the browser's
  // time costs a visitor nothing and a harvester of many pages a lot. It
  // starts only on the visitor's click. What the page sees of its own
  // browser goes along (`s`): a browser driven by a program, or one without
  // languages or with a software renderer, gets a harder puzzle, never a
  // refusal.
  function browserBits() {
    var bits = 0;
    if (navigator.webdriver) bits |= 1;
    if (!navigator.languages || navigator.languages.length === 0) bits |= 2;
    try {
      var gl = document.createElement("canvas").getContext("webgl");
      var info = gl && gl.getExtension("WEBGL_debug_renderer_info");
      var renderer = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
      if (/swiftshader|llvmpipe|software/i.test(renderer)) bits |= 4;
    } catch (e) { /* no WebGL: nothing to say */ }
    return bits;
  }

  function endpoint(box) {
    var local = /^(localhost|127\.0\.0\.1|\[::1\])$/.test(location.hostname);
    return box.getAttribute(local ? "data-endpoint-local" : "data-endpoint");
  }

  function solve(challenge, onProgress) {
    return new Promise(function (resolve, reject) {
      var worker = new Worker("/site/impressum-worker.js");
      worker.onmessage = function (e) {
        if (e.data && typeof e.data.progress === "number") { onProgress(e.data.progress); return; }
        worker.terminate();
        if (e.data && e.data.counters) resolve(e.data.counters);
        else reject(new Error("no answer"));
      };
      worker.onerror = function () { worker.terminate(); reject(new Error("worker")); };
      worker.postMessage({ nonce: challenge.nonce, parts: challenge.parts, difficulty: challenge.difficulty });
    });
  }

  // one round: a challenge, solved, answered; `retry` once more when the
  // answer came too late or twice
  function fetchBlob(base, onProgress, retry) {
    var opts = { credentials: "omit", referrerPolicy: "no-referrer", cache: "no-store" };
    return fetch(base + "/challenge?s=" + browserBits(), opts)
      .then(function (r) { if (!r.ok) throw new Error("HTTP " + r.status); return r.json(); })
      .then(function (c) {
        return solve(c, onProgress).then(function (counters) {
          return fetch(base, {
            method: "POST", credentials: "omit", referrerPolicy: "no-referrer", cache: "no-store",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ nonce: c.nonce, parts: c.parts, difficulty: c.difficulty, expires: c.expires, sig: c.sig, counters: counters })
          });
        });
      })
      .then(function (r) {
        if ((r.status === 409 || r.status === 410) && retry) return fetchBlob(base, onProgress, false);
        if (!r.ok) throw new Error("HTTP " + r.status);
        return r.text();
      });
  }

  var box = document.getElementById("imp-provider");
  var show = document.getElementById("imp-show");
  var showStatus = document.getElementById("imp-show-status");
  var ready = window.crypto && crypto.subtle && crypto.getRandomValues && window.fetch && window.TextDecoder && window.Worker && window.Promise;
  if (!box || !show) return;
  if (!ready) { failed(); return; }
  show.hidden = false;
  show.addEventListener("click", function () {
    if (show.getAttribute("aria-busy") === "true") return;
    show.setAttribute("aria-busy", "true");
    clear(showStatus);
    showStatus.appendChild(t("One moment, your browser is working out a small puzzle …", "Einen Moment, dein Browser löst ein kleines Rätsel …"));
    var bar = document.getElementById("imp-progress");
    if (bar) { bar.value = 0; bar.hidden = false; }
    fetchBlob(endpoint(box), function (p) { if (bar) bar.value = p; }, true)
      .then(decode)
      .then(function (fields) {
        show.hidden = true;
        if (bar) bar.hidden = true;
        clear(showStatus);
        render(fields);
        var first = box.querySelector(".imp-btn");
        if (first) first.focus();
      })
      .catch(function () {
        show.removeAttribute("aria-busy");
        if (bar) bar.hidden = true;
        clear(showStatus);
        failed();
      });
  });
})();
