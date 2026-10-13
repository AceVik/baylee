// The language switch of a page with nothing else to do (datenschutz.html):
// boot.js has picked the language before the first paint; this marks the
// two buttons, sets the title, and switches and remembers on a click (the
// same localStorage key as site.js and impressum.js). The page's CSP allows
// no inline script, so it lives in this file.
(function () {
  "use strict";
  var root = document.documentElement;
  var titles = {
    en: root.getAttribute("data-title-en") || document.title,
    de: root.getAttribute("data-title-de") || document.title
  };
  function apply(l) {
    root.setAttribute("data-lang", l);
    root.setAttribute("lang", l);
    document.title = titles[l];
    var buttons = document.querySelectorAll(".lang-btn");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].getAttribute("data-lang") === l ? "true" : "false");
    }
  }
  apply(root.getAttribute("data-lang") === "de" ? "de" : "en");
  document.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest(".lang-btn");
    if (!b) return;
    var l = b.getAttribute("data-lang");
    apply(l);
    try { localStorage.setItem("baylee-site-lang", l); } catch (err) { /* private mode: fine */ }
  });
})();
