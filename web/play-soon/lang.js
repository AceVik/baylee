// Picks English or German: `?lang=de|en` in the address wins, else the
// browser's languages; the two buttons switch by hand. Nothing is stored,
// nothing is sent. Without this script the page shows both languages.
// `?theme=dark|light` forces a colour scheme (for screenshots).
(function () {
  var root = document.documentElement;
  var q = new URLSearchParams(location.search);
  var theme = q.get("theme");
  if (theme === "dark" || theme === "light") root.setAttribute("data-theme", theme);

  function pick() {
    var asked = q.get("lang");
    if (asked === "de" || asked === "en") return asked;
    var langs = navigator.languages || [navigator.language || "en"];
    for (var i = 0; i < langs.length; i++) {
      var l = String(langs[i]).toLowerCase();
      if (l.indexOf("de") === 0) return "de";
      if (l.indexOf("en") === 0) return "en";
    }
    return "en";
  }

  function apply(lang) {
    root.setAttribute("data-lang", lang);
    root.setAttribute("lang", lang);
    document.title = lang === "de" ? "Baylee · Browser-Version pausiert" : "Baylee · Browser version paused";
    var buttons = document.querySelectorAll(".lang-btn");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].getAttribute("data-lang") === lang ? "true" : "false");
    }
  }

  apply(pick());
  document.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest(".lang-btn");
    if (b) apply(b.getAttribute("data-lang"));
  });
})();
