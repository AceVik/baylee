// Runs before the first paint: in <head>, not deferred, a few hundred bytes.
// It says that the script runs (data-js), applies ?theme= and picks the
// language as site.js does, so the page never paints both languages and a
// hidden language switch first and then reflows under the visitor's eyes
// (layout shift). Everything else is site.js's.
(function () {
  "use strict";
  var root = document.documentElement, q = null, l = null, i;
  try { q = new URLSearchParams(location.search); } catch (e) { /* very old browser: defaults */ }
  root.setAttribute("data-js", "");
  var theme = q && q.get("theme");
  if (theme === "dark" || theme === "light") root.setAttribute("data-theme", theme);
  l = q && q.get("lang");
  if (l !== "de" && l !== "en") { try { l = localStorage.getItem("baylee-site-lang"); } catch (e) { l = null; } }
  if (l !== "de" && l !== "en") {
    l = "en";
    var langs = navigator.languages || [navigator.language || "en"];
    for (i = 0; i < langs.length; i++) {
      var x = String(langs[i]).toLowerCase();
      if (x.indexOf("de") === 0) { l = "de"; break; }
      if (x.indexOf("en") === 0) break;
    }
  }
  root.setAttribute("data-lang", l);
  root.setAttribute("lang", l);
})();
