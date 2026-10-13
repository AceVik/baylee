// Baylee's Impressum (web/site/impressum.html). Two jobs:
//
//   1. the language, as on the landing page: `?lang=de|en`, then what this
//      browser chose last time (the same localStorage key as site.js), then
//      the browser's languages;
//   2. the provider's details. They are in no file of the repository and in
//      nothing a crawler reads as text: this script fetches a blob from a
//      directory only the server has (/private/impressum.bin, Caddy answers
//      it same-origin only, scripts/server/site.caddy), decodes it and writes
//      the text into the page as real text. The e-mail address and the
//      telephone number become links only when a visitor asks for them.
//
// The blob is written by scripts/site/impressum_blob.py, which says how it is
// built; the key is split between SCRIPT_KEY here and a nonce in the blob.
// That keeps harvesters that grep for addresses away and nobody else; it is
// not meant to. Without JavaScript, or when the blob does not come, the page
// says to turn JavaScript on or use the contact form.
//
// Nothing here identifies the visitor to anyone. The page's CSP allows no
// inline script, so everything lives in this file.
(function () {
  "use strict";

  var BLOB = "/private/impressum.bin";
  var MAGIC = "BYIM";
  var VERSION = 1;
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
  // A button that, once pressed, gives way to the link it stood for; the
  // link takes the focus so a keyboard or screen reader stays on it.
  function reveal(slot, label, make) {
    var button = el("button", { type: "button", class: "button button-ghost reveal-btn" }, [label]);
    button.addEventListener("click", function () {
      var link = make();
      slot.replaceChild(link, button);
      link.focus();
    });
    slot.appendChild(button);
  }

  function render(f) {
    var box = document.getElementById("imp-provider");
    var contact = document.getElementById("imp-contact");
    if (!box || !contact) return;

    var address = el("address", { class: "imp-address" }, [
      el("span", { text: f.name }), el("br"),
      el("span", { text: f.street }), el("br"),
      el("span", { text: f.postcode + " " + f.city }), el("br"),
      el("span", {}, [t(f.country_en, f.country_de)])
    ]);
    box.appendChild(address);

    var mailSlot = el("dd", { id: "imp-mail" });
    var phoneSlot = el("dd", { id: "imp-phone" });
    reveal(mailSlot, t("Show e-mail address", "E-Mail-Adresse anzeigen"), function () {
      return el("a", { href: "mailto:" + f.email, text: f.email });
    });
    reveal(phoneSlot, t("Show telephone number", "Telefonnummer anzeigen"), function () {
      return el("a", { href: "tel:" + f.phone.replace(/[^+\d]/g, ""), text: f.phone });
    });
    contact.appendChild(el("dl", { class: "imp-dl" }, [
      el("dt", {}, [t("E-mail", "E-Mail")]), mailSlot,
      el("dt", {}, [t("Telephone", "Telefon")]), phoneSlot
    ]));
    root.classList.add("imp-loaded");
  }

  function failed() {
    var notes = document.querySelectorAll(".imp-failed");
    for (var i = 0; i < notes.length; i++) notes[i].hidden = false;
  }

  if (!window.crypto || !crypto.subtle || !window.fetch || !window.TextDecoder) { failed(); return; }
  fetch(BLOB, { credentials: "omit", referrerPolicy: "no-referrer", cache: "no-store", headers: { Accept: "application/octet-stream" } })
    .then(function (r) { if (!r.ok) throw new Error("HTTP " + r.status); return r.text(); })
    .then(decode)
    .then(render)
    .catch(failed);
})();
