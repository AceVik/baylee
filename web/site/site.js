// Baylee's landing page. Three jobs, each one failing on its own:
//
//   1. the language: `?lang=de|en` wins, then what this browser chose last
//      time (localStorage, in try/catch), then the browser's languages;
//   2. the latest release from GitHub's public API, matched to this machine's
//      system and architecture, and the repository's state (CI on main, the
//      last commit, stars, open issues); answers are kept in sessionStorage
//      for a while, because GitHub allows 60 requests an hour per address;
//   3. the gateway's own `/info` and `/health`, which exist only where the
//      gateway serves this page;
//   4. the motion: the hero's layers at their own pace (parallax), sections
//      that come up as they are read, the bar that frosts once the page
//      moves, the menu on narrow screens, the light that follows the
//      pointer over a panel, the ring a click leaves, the screenshots large;
//      all of it transform and opacity, and none under prefers-reduced-motion;
//   5. the contact form, sent with fetch to the feedback service
//      (docs/feedback.md §"The landing page's contact form").
//
// Nothing here identifies the visitor to anyone. No token, no cookie. The
// page's CSP allows no inline script, so everything lives in this file.
(function () {
  "use strict";

  var REPO = "AceVik/baylee";
  var API = "https://api.github.com/repos/" + REPO;
  var RELEASES_PAGE = "https://github.com/" + REPO + "/releases";
  var CACHE_TTL_MS = 10 * 60 * 1000;
  var root = document.documentElement;
  var query = new URLSearchParams(location.search);
  var lang = "en";
  root.setAttribute("data-js", "");
  var still = window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : { matches: false };

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
  var TITLES = { en: "Baylee – a table that knows the rules", de: "Baylee – ein Tisch, der die Regeln kennt" };
  function applyLang(l) {
    lang = l;
    root.setAttribute("data-lang", l);
    root.setAttribute("lang", l);
    document.title = TITLES[l];
    var buttons = document.querySelectorAll(".lang-btn");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", buttons[i].getAttribute("data-lang") === l ? "true" : "false");
    }
    // what this script rendered carries both languages where it can (.en/.de
    // spans); dates and numbers are formatted for one, so they are drawn again
    for (var j = 0; j < redraw.length; j++) redraw[j]();
  }
  var redraw = []; // functions that draw fetched data again in the current language
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
  function tt(en, de) { return lang === "de" ? de : en; }
  function clear(n) { while (n.firstChild) n.removeChild(n.firstChild); }
  function mb(bytes) {
    var m = bytes / 1048576;
    var s = m >= 100 ? Math.round(m).toString() : m.toFixed(1);
    return (lang === "de" ? s.replace(".", ",") : s) + " MB";
  }
  function dateText(iso) {
    var d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    try {
      return d.toLocaleDateString(lang === "de" ? "de-DE" : "en-GB", { year: "numeric", month: "long", day: "numeric" });
    } catch (e) { return iso.slice(0, 10); }
  }
  function relTime(iso) {
    var ms = Date.now() - new Date(iso).getTime();
    if (isNaN(ms)) return "";
    var mins = Math.round(ms / 60000);
    if (mins < 60) return tt(mins + " min ago", "vor " + mins + " Min.");
    var h = Math.round(mins / 60);
    if (h < 48) return tt(h + " h ago", "vor " + h + " Std.");
    var d = Math.round(h / 24);
    return tt(d + " days ago", "vor " + d + " Tagen");
  }

  // GitHub with a cache in this tab. A miss returns null; a failed request
  // too, and the page then keeps its static text.
  function cached(key) {
    try {
      var raw = sessionStorage.getItem("baylee-site:" + key);
      if (!raw) return null;
      var v = JSON.parse(raw);
      if (!v || typeof v.at !== "number" || Date.now() - v.at > CACHE_TTL_MS) return null;
      return v.data;
    } catch (e) { return null; }
  }
  function keep(key, data) {
    try { sessionStorage.setItem("baylee-site:" + key, JSON.stringify({ at: Date.now(), data: data })); } catch (e) { /* fine */ }
  }
  function getJSON(url, key) {
    var hit = cached(key);
    if (hit) return Promise.resolve(hit);
    if (!window.fetch) return Promise.resolve(null);
    return fetch(url, { headers: { Accept: "application/vnd.github+json" }, credentials: "omit", referrerPolicy: "no-referrer" })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (j) { if (j) keep(key, j); return j; })
      .catch(function () { return null; });
  }

  // ------------------------------------------------------------ the machine
  // What we can tell about the visitor's system, honestly. `sure` says
  // whether the architecture was read (UA-CH) or only guessed from the UA.
  function detect() {
    var ua = navigator.userAgent || "";
    var guess = { os: null, arch: null, sure: false, mobile: false };
    if (/Android/i.test(ua)) { guess.os = "android"; guess.mobile = true; }
    else if (/iPhone|iPad|iPod/i.test(ua) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)) { guess.os = "ios"; guess.mobile = true; }
    else if (/Windows/i.test(ua)) { guess.os = "windows"; guess.arch = /ARM64|aarch64/i.test(ua) ? "arm64" : "x64"; }
    else if (/Mac OS X|Macintosh/i.test(ua)) { guess.os = "mac"; guess.arch = null; }
    else if (/CrOS/i.test(ua)) { guess.os = "chromeos"; guess.mobile = true; }
    else if (/Linux|X11/i.test(ua)) { guess.os = "linux"; guess.arch = /aarch64|arm64|armv8/i.test(ua) ? "arm64" : (/x86_64|x64|amd64/i.test(ua) ? "x64" : null); }

    var uad = navigator.userAgentData;
    if (!uad || !uad.getHighEntropyValues) return Promise.resolve(guess);
    return uad.getHighEntropyValues(["architecture", "bitness", "platform"]).then(function (v) {
      var p = String(v.platform || "").toLowerCase();
      if (p === "windows") guess.os = "windows";
      else if (p === "macos") guess.os = "mac";
      else if (p === "linux") guess.os = "linux";
      else if (p === "android") { guess.os = "android"; guess.mobile = true; }
      else if (p === "chrome os" || p === "chromeos") { guess.os = "chromeos"; guess.mobile = true; }
      if (v.architecture === "arm") { guess.arch = "arm64"; guess.sure = true; }
      else if (v.architecture === "x86") { guess.arch = v.bitness === "64" ? "x64" : "x86"; guess.sure = v.bitness === "64"; }
      return guess;
    }).catch(function () { return guess; });
  }

  // ------------------------------------------------------------ the release
  // The six platforms the project builds for, and the one it does not.
  var PLATFORMS = [
    { id: "windows-x64", os: "windows", arch: "x64", name: ["Windows", "Windows"], sub: ["most PCs: 64-bit Intel or AMD", "die meisten PCs: 64-Bit Intel oder AMD"] },
    { id: "windows-arm64", os: "windows", arch: "arm64", name: ["Windows on ARM", "Windows auf ARM"], sub: ["Snapdragon laptops and the like", "Snapdragon-Laptops und ähnliche"] },
    { id: "mac-arm64", os: "mac", arch: "arm64", name: ["macOS", "macOS"], sub: ["Apple silicon: M1 and later", "Apple Silicon: M1 und neuer"] },
    { id: "mac-x64", os: "mac", arch: "x64", name: ["macOS on Intel", "macOS auf Intel"], sub: ["", ""] },
    { id: "linux-x64", os: "linux", arch: "x64", name: ["Linux", "Linux"], sub: ["x86-64", "x86-64"] },
    { id: "linux-arm64", os: "linux", arch: "arm64", name: ["Linux on ARM", "Linux auf ARM"], sub: ["ARM64 / aarch64", "ARM64 / aarch64"] }
  ];
  // name → platform id and kind. Two families: the installers (beta.6 on)
  // and the archives the updater uses (every release).
  var FILE_RULES = [
    [/^Baylee-Setup-.*-x64\.exe$/, "windows-x64", "installer", ["Setup", "Setup"]],
    [/^Baylee-Setup-.*-arm64\.exe$/, "windows-arm64", "installer", ["Setup", "Setup"]],
    [/^Baylee-.*-aarch64\.dmg$/, "mac-arm64", "installer", ["Disk image", "Disk-Image"]],
    [/^Baylee-.*-x86_64\.dmg$/, "mac-x64", "installer", ["Disk image", "Disk-Image"]],
    [/^Baylee-.*-x86_64\.AppImage$/, "linux-x64", "installer", ["AppImage", "AppImage"]],
    [/^Baylee-.*-aarch64\.AppImage$/, "linux-arm64", "installer", ["AppImage", "AppImage"]],
    [/^baylee_.*_amd64\.deb$/, "linux-x64", "deb", ["Debian / Ubuntu package", "Debian-/Ubuntu-Paket"]],
    [/^baylee_.*_arm64\.deb$/, "linux-arm64", "deb", ["Debian / Ubuntu package", "Debian-/Ubuntu-Paket"]],
    [/^baylee-client-.*-x86_64-pc-windows-msvc\.zip$/, "windows-x64", "archive", ["Archive (zip)", "Archiv (zip)"]],
    [/^baylee-client-.*-aarch64-pc-windows-msvc\.zip$/, "windows-arm64", "archive", ["Archive (zip)", "Archiv (zip)"]],
    [/^baylee-client-.*-aarch64-apple-darwin\.zip$/, "mac-arm64", "archive", ["Archive (zip)", "Archiv (zip)"]],
    [/^baylee-client-.*-x86_64-apple-darwin\.zip$/, "mac-x64", "archive", ["Archive (zip)", "Archiv (zip)"]],
    [/^baylee-client-.*-x86_64-unknown-linux-gnu\.tar\.gz$/, "linux-x64", "archive", ["Archive (tar.gz)", "Archiv (tar.gz)"]],
    [/^baylee-client-.*-aarch64-unknown-linux-gnu\.tar\.gz$/, "linux-arm64", "archive", ["Archive (tar.gz)", "Archiv (tar.gz)"]]
  ];
  var KIND_ORDER = { installer: 0, deb: 1, archive: 2 };

  function sortFiles(release) {
    var byName = {};
    var i;
    for (i = 0; i < release.assets.length; i++) byName[release.assets[i].name] = release.assets[i];
    var files = {}; // platform id → [{asset, kind, label, sha, sig}]
    for (i = 0; i < release.assets.length; i++) {
      var a = release.assets[i];
      for (var r = 0; r < FILE_RULES.length; r++) {
        if (FILE_RULES[r][0].test(a.name)) {
          var pid = FILE_RULES[r][1];
          (files[pid] = files[pid] || []).push({
            asset: a, kind: FILE_RULES[r][2], label: FILE_RULES[r][3],
            sha: byName[a.name + ".sha256"] || null, sig: byName[a.name + ".sig"] || null
          });
          break;
        }
      }
    }
    for (var k in files) files[k].sort(function (x, y) { return KIND_ORDER[x.kind] - KIND_ORDER[y.kind]; });
    return files;
  }

  function pickRelease(list) {
    if (!list || !list.length) return null;
    for (var i = 0; i < list.length; i++) if (!list[i].draft && list[i].assets && list[i].assets.length) return list[i];
    return null;
  }

  function fileRow(f) {
    var li = el("li");
    li.appendChild(el("a", { class: "file", href: f.asset.browser_download_url, rel: "noopener", text: f.asset.name }));
    var kind = el("span", { class: "kind" }); kind.appendChild(t(f.label[0], f.label[1])); li.appendChild(kind);
    li.appendChild(el("span", { class: "size", text: mb(f.asset.size) }));
    if (f.sha || f.sig) {
      var aux = el("span", { class: "aux" });
      if (f.sha) aux.appendChild(el("a", { href: f.sha.browser_download_url, rel: "noopener", text: ".sha256" }));
      if (f.sha && f.sig) aux.appendChild(document.createTextNode(" · "));
      if (f.sig) aux.appendChild(el("a", { href: f.sig.browser_download_url, rel: "noopener", text: ".sig" }));
      li.appendChild(aux);
    }
    return li;
  }

  function renderPlatforms(release, files, mine) {
    var box = document.getElementById("platforms");
    clear(box);
    for (var i = 0; i < PLATFORMS.length; i++) {
      var p = PLATFORMS[i];
      var list = files[p.id] || [];
      if (!list.length && p.id !== "mac-x64") continue; // a platform the release lacks is said below the rows instead
      var card = el("div", { class: "plat glass reveal in" + (mine && mine.id === p.id ? " mine" : "") });
      var name = el("div", { class: "plat-name" });
      name.appendChild(t(p.name[0], p.name[1]));
      if (mine && mine.id === p.id) { var tag = el("span", { class: "mine-tag" }); tag.appendChild(t("your system", "dein System")); name.appendChild(tag); }
      card.appendChild(name);
      if (p.sub[0]) { var sub = el("p", { class: "plat-sub" }); sub.appendChild(t(p.sub[0], p.sub[1])); card.appendChild(sub); }
      if (!list.length) {
        var none = el("p", { class: "plat-none" });
        none.appendChild(t("No build for Intel Macs yet. Baylee needs a Mac with an Apple chip (M1 or later).", "Noch kein Build für Intel-Macs. Baylee braucht einen Mac mit Apple-Chip (M1 oder neuer)."));
        card.appendChild(none);
      } else {
        var ul = el("ul", { class: "plat-files" });
        for (var j = 0; j < list.length; j++) ul.appendChild(fileRow(list[j]));
        card.appendChild(ul);
      }
      box.appendChild(card);
    }
    var missing = [];
    for (i = 0; i < PLATFORMS.length; i++) if (PLATFORMS[i].id !== "mac-x64" && !(files[PLATFORMS[i].id] || []).length) missing.push(PLATFORMS[i]);
    if (missing.length) {
      var note = el("p", { class: "dl-fallback" });
      var names = missing.map(function (p) { return tt(p.name[0], p.name[1]); }).join(", ");
      note.appendChild(t("This release has no file for: " + names + ". Older releases are on the releases page.", "Diese Version hat keine Datei für: " + names + ". Ältere Versionen liegen auf der Releases-Seite."));
      box.appendChild(note);
    }
  }

  function renderHead(release) {
    var head = document.getElementById("release-head");
    clear(head);
    head.appendChild(el("span", { class: "ver", text: release.tag_name }));
    if (release.prerelease) { var tag = el("span", { class: "tag" }); tag.appendChild(t("pre-release", "Vorabversion")); head.appendChild(tag); }
    head.appendChild(el("span", { text: dateText(release.published_at) }));
    var notes = el("a", { href: release.html_url, rel: "noopener" });
    notes.appendChild(t("Release notes and checksums", "Release-Notizen und Prüfsummen"));
    head.appendChild(notes);
    var all = el("a", { href: RELEASES_PAGE, rel: "noopener" });
    all.appendChild(t("All releases", "Alle Versionen"));
    head.appendChild(all);
  }

  function renderPrimary(release, files, machine) {
    var box = document.getElementById("dl-primary");
    var mine = null, i;
    if (machine && machine.os && !machine.mobile) {
      for (i = 0; i < PLATFORMS.length; i++) {
        var p = PLATFORMS[i];
        if (p.os !== machine.os) continue;
        if (machine.arch ? p.arch === machine.arch : p.id === "mac-arm64" || p.id === "linux-x64" || p.id === "windows-x64") { mine = p; break; }
      }
    }
    var mineFiles = mine ? (files[mine.id] || []) : [];
    var best = mineFiles.length ? mineFiles[0] : null;
    clear(box);

    var a = el("a", { class: "button button-big", href: best ? best.asset.browser_download_url : release.html_url, rel: "noopener" });
    var icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("class", "button-icon"); icon.setAttribute("viewBox", "0 0 24 24"); icon.setAttribute("width", "22"); icon.setAttribute("height", "22"); icon.setAttribute("aria-hidden", "true");
    var path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", "M12 3v11.2l3.6-3.6 1.4 1.4-6 6-6-6 1.4-1.4 3.6 3.6V3h2zM4 19h16v2H4z"); path.setAttribute("fill", "currentColor");
    icon.appendChild(path); a.appendChild(icon);
    if (best) {
      a.appendChild(t("Download for " + mine.name[0], "Download für " + mine.name[1]));
    } else {
      a.appendChild(t("Download Baylee", "Baylee herunterladen"));
    }
    box.appendChild(a);

    var meta = el("p", { class: "dl-meta" });
    if (best) {
      meta.appendChild(document.createTextNode(release.tag_name));
      meta.appendChild(el("span", { class: "sep", text: "·" }));
      meta.appendChild(t(best.label[0], best.label[1]));
      meta.appendChild(el("span", { class: "sep", text: "·" }));
      meta.appendChild(document.createTextNode(mb(best.asset.size)));
      meta.appendChild(el("span", { class: "sep", text: "·" }));
      meta.appendChild(document.createTextNode(dateText(release.published_at)));
    } else {
      meta.appendChild(document.createTextNode(release.tag_name + " · " + dateText(release.published_at)));
    }
    box.appendChild(meta);

    var sub = el("p", { class: "dl-sub" });
    if (best) {
      if (best.sha) sub.appendChild(el("a", { href: best.sha.browser_download_url, rel: "noopener", text: "SHA-256" }));
      if (best.sha && best.sig) sub.appendChild(document.createTextNode(" · "));
      if (best.sig) sub.appendChild(el("a", { href: best.sig.browser_download_url, rel: "noopener", text: tt("signature", "Signatur") }));
      if (best.sha || best.sig) sub.appendChild(document.createTextNode(" · "));
    }
    sub.appendChild(el("a", { href: release.html_url, rel: "noopener", text: tt("release notes", "Release-Notizen") }));
    sub.appendChild(document.createTextNode(" · "));
    sub.appendChild(el("a", { href: "#download", text: tt("other systems", "andere Systeme") }));
    box.appendChild(sub);

    // what we could not tell, said plainly
    var arch = el("p", { class: "dl-arch" });
    if (!machine || !machine.os) {
      arch.appendChild(t("We could not tell your system from this browser; every file is listed under Download.", "Dein System ließ sich aus diesem Browser nicht erkennen; unter Download sind alle Dateien aufgeführt."));
    } else if (machine.mobile) {
      arch.appendChild(t("There are no builds for phones and tablets yet. The button takes you to the release for your computer.", "Für Handy und Tablet gibt es noch keine Builds. Der Knopf führt zum Release für deinen Rechner."));
    } else if (machine.os === "mac" && !machine.sure) {
      arch.appendChild(t("This browser does not say whether your Mac has an Apple chip or an Intel one. The file above is for Apple silicon (M1 and later); there is no Intel build yet.", "Dieser Browser verrät nicht, ob dein Mac einen Apple- oder einen Intel-Chip hat. Die Datei oben ist für Apple Silicon (M1 und neuer); einen Intel-Build gibt es noch nicht."));
    } else if (machine.os === "mac" && machine.arch === "x64") {
      arch.appendChild(t("Your Mac reports an Intel chip, and there is no Intel build yet. The button takes you to the release page.", "Dein Mac meldet einen Intel-Chip, und einen Intel-Build gibt es noch nicht. Der Knopf führt zur Release-Seite."));
    } else if (machine.os === "windows" && !machine.sure) {
      arch.appendChild(t("If this is a Windows on ARM PC (a Snapdragon laptop, say), take the ARM file under Download instead.", "Wenn das ein Windows-auf-ARM-PC ist (etwa ein Snapdragon-Laptop), nimm stattdessen die ARM-Datei unter Download."));
    } else if (machine.os === "linux" && !machine.arch) {
      arch.appendChild(t("Your browser does not say which CPU this is; x86-64 is assumed. The ARM file is under Download.", "Dein Browser sagt nicht, welche CPU das ist; x86-64 ist angenommen. Die ARM-Datei steht unter Download."));
    }
    if (arch.firstChild) box.appendChild(arch);

    if (best && best.kind === "archive") {
      var how = el("p", { class: "dl-arch" });
      how.appendChild(t("This release ships as an archive: unpack it and start Baylee. Installers come with the next release.", "Diese Version kommt als Archiv: entpacken und Baylee starten. Installer kommen mit der nächsten Version."));
      box.appendChild(how);
    }
    return mine;
  }

  function releaseFailed() {
    var head = document.getElementById("release-head");
    clear(head);
    head.appendChild(t("GitHub did not answer just now; it lets a page like this ask 60 times an hour. ", "GitHub hat gerade nicht geantwortet; einer Seite wie dieser erlaubt es 60 Anfragen pro Stunde. "));
    var a = el("a", { href: RELEASES_PAGE, rel: "noopener" });
    a.appendChild(t("The releases page has every file.", "Die Releases-Seite hat jede Datei."));
    head.appendChild(a);
    // the static text under it speaks of a page without JavaScript, which this is not
    var box = document.getElementById("platforms");
    clear(box);
    var p = el("p", { class: "dl-fallback" });
    p.appendChild(t("An installer and an archive for Windows (x64 and ARM64), macOS (Apple silicon) and Linux (x86-64 and ARM64), each with a .sha256 beside it, on the ", "Ein Installer und ein Archiv für Windows (x64 und ARM64), macOS (Apple Silicon) und Linux (x86-64 und ARM64), jeweils mit einer .sha256 daneben, auf der "));
    var b = el("a", { href: RELEASES_PAGE, rel: "noopener" });
    b.appendChild(t("releases page", "Releases-Seite"));
    p.appendChild(b);
    p.appendChild(document.createTextNode("."));
    box.appendChild(p);
  }

  function stat(id, children) {
    var v = document.querySelector("#" + id + " .stat-value");
    if (!v) return;
    clear(v);
    for (var i = 0; i < children.length; i++) v.appendChild(children[i]);
  }

  Promise.all([getJSON(API + "/releases?per_page=6", "releases"), detect()]).then(function (r) {
    var release = pickRelease(r[0]);
    if (!release) { releaseFailed(); return; }
    var files = sortFiles(release);
    var draw = function () {
      renderHead(release);
      var mine = renderPrimary(release, files, r[1]);
      renderPlatforms(release, files, mine);
      var link = el("a", { href: release.html_url, rel: "noopener", text: release.tag_name });
      stat("stat-release", [link, el("span", { class: "small", text: " · " + dateText(release.published_at) })]);
    };
    draw();
    redraw.push(draw);
  });

  // ------------------------------------------------------------ the repository
  function setKV(scope, key, children) {
    var dd = document.querySelector("#" + scope + " dd[data-k='" + key + "']");
    if (!dd) return;
    clear(dd);
    for (var i = 0; i < children.length; i++) dd.appendChild(children[i]);
  }

  getJSON(API + "/actions/workflows/ci.yml/runs?branch=main&event=push&status=completed&per_page=1", "ci").then(function (d) {
    var run = d && d.workflow_runs && d.workflow_runs[0];
    if (!run) return;
    var ok = run.conclusion === "success";
    var cls = ok ? "ok" : (run.conclusion === "failure" ? "bad" : "warn");
    var word = ok ? t("passing", "grün") : (run.conclusion === "failure" ? t("failing", "rot") : t(run.conclusion || "unknown", run.conclusion || "unbekannt"));
    var mk = function () {
      var a = el("a", { href: run.html_url, rel: "noopener" });
      a.appendChild(el("span", { class: "dot " + cls, "aria-hidden": "true" }));
      a.appendChild(word.cloneNode(true));
      return a;
    };
    var draw = function () {
      stat("stat-ci", [mk()]);
      setKV("repo-card", "ci", [mk(), el("span", { class: "small", text: " · " + relTime(run.updated_at) + " · " + run.head_sha.slice(0, 7) })]);
    };
    draw();
    redraw.push(draw);
  });

  getJSON(API + "/commits?sha=main&per_page=1", "commit").then(function (d) {
    var c = d && d[0];
    if (!c) return;
    var subject = (c.commit.message || "").split("\n")[0];
    if (subject.length > 72) subject = subject.slice(0, 70) + "…";
    var when = c.commit.committer && c.commit.committer.date || c.commit.author.date;
    var mk = function () { return el("a", { href: c.html_url, rel: "noopener", text: c.sha.slice(0, 7) }); };
    var draw = function () {
      stat("stat-commit", [mk(), el("span", { class: "small", text: " · " + relTime(when) })]);
      setKV("repo-card", "commit", [mk(), document.createTextNode(" · " + relTime(when) + " · "), el("span", { class: "small", text: subject })]);
    };
    draw();
    redraw.push(draw);
  });

  getJSON(API, "repo").then(function (d) {
    if (!d) return;
    setKV("repo-card", "stars", [el("a", { href: "https://github.com/" + REPO + "/stargazers", rel: "noopener", text: String(d.stargazers_count) })]);
    setKV("repo-card", "issues", [el("a", { href: "https://github.com/" + REPO + "/issues", rel: "noopener", text: String(d.open_issues_count) })]);
  });

  // ------------------------------------------------------------ the gateway
  // Same origin, no credentials. Where this page is not served by the
  // gateway (a local preview), these are 404 and the card keeps its text.
  function gw(path) {
    if (!window.fetch) return Promise.resolve(null);
    return fetch(path, { credentials: "omit", referrerPolicy: "no-referrer", headers: { Accept: "application/json" } })
      .then(function (r) { return r.ok ? r.json() : null; })
      .catch(function () { return null; });
  }
  gw("/info").then(function (info) {
    if (!info || typeof info.name !== "string") return;
    setKV("gateway-card", "name", [document.createTextNode(info.name)]);
    if (info.version) setKV("gateway-card", "version", [el("code", { text: String(info.version).split(" ")[0] })]);
    var reg = info.registration;
    var words = reg === "invite" ? t("closed beta: a new account or guest needs a key", "geschlossene Beta: neues Konto oder Gast braucht einen Schlüssel")
      : reg === "off" ? t("no new accounts", "keine neuen Konten")
      : t("open", "offen");
    setKV("gateway-card", "registration", [words]);
    stat("stat-gateway", [el("span", { class: "dot ok", "aria-hidden": "true" }), reg === "invite" ? t("up, closed beta", "erreichbar, geschlossene Beta") : t("up", "erreichbar")]);
  });
  gw("/health").then(function (h) {
    if (!h || !h.games) return;
    var running = h.games.running | 0, waiting = h.games.waiting | 0;
    setKV("gateway-card", "games", [t(running + " running, " + waiting + " waiting for players", running + " laufen, " + waiting + " warten auf Spieler")]);
  });

  // ------------------------------------------------------------ motion
  // Everything here moves with transform and opacity only, from one
  // requestAnimationFrame per frame at most, and does nothing at all when
  // the visitor asks for reduced motion.
  var motion = (function () {
    var calm = function () { return still.matches; };
    var io = window.IntersectionObserver;

    // the bar frosts once the page has moved; the section in view is marked
    var top = document.querySelector(".top");
    var hero = document.querySelector(".hero");
    // the hero's box, measured once and again on a resize, never per frame:
    // a read after the last frame's writes would force a style pass
    var heroTop = 0, heroH = 1, heroW = 1;
    function measure() {
      if (!hero) return;
      var r = hero.getBoundingClientRect();
      heroTop = r.top + (window.scrollY || window.pageYOffset);
      heroH = r.height || 1; heroW = r.width || 1;
    }
    measure();
    function frost() {
      var y = window.scrollY || window.pageYOffset;
      if (top) top.classList.toggle("solid", y > (hero ? Math.min(heroH * 0.55, 420) : 40));
    }
    frost();

    // the hero's layers, each at its own pace (data-speed: how far it follows
    // the scroll; data-tilt: how many pixels it leans with the pointer)
    var layers = [].slice.call(document.querySelectorAll(".sky .layer")).map(function (n) {
      return { n: n, speed: parseFloat(n.getAttribute("data-speed")) || 0, tilt: parseFloat(n.getAttribute("data-tilt")) || 0 };
    });
    var inner = document.getElementById("hero-inner");
    var heroSeen = true, queued = false, px = 0, py = 0, tx = 0, ty = 0;
    function paint() {
      queued = false;
      frost();
      if (calm() || !hero || !heroSeen) return;
      var y = Math.max(0, window.scrollY || window.pageYOffset);
      var h = heroH;
      // the pointer's lean eases towards where the pointer is
      tx += (px - tx) * 0.12; ty += (py - ty) * 0.12;
      for (var i = 0; i < layers.length; i++) {
        var L = layers[i];
        L.n.style.transform = "translate3d(" + (-tx * L.tilt).toFixed(2) + "px," + (y * L.speed - ty * L.tilt).toFixed(2) + "px,0)";
      }
      if (inner) {
        var k = Math.min(1, y / (h * 0.7));
        // on a phone the text stays put and only fades: the status cards
        // below it would otherwise slide over it
        inner.style.transform = window.innerWidth > 700 ? "translate3d(0," + (y * 0.18).toFixed(2) + "px,0)" : "";
        inner.style.opacity = String(1 - k * 0.85);
      }
      if (Math.abs(px - tx) > 0.001 || Math.abs(py - ty) > 0.001) queue();
    }
    function queue() { if (!queued) { queued = true; requestAnimationFrame(paint); } }
    window.addEventListener("scroll", queue, { passive: true });
    window.addEventListener("resize", function () { measure(); queue(); }, { passive: true });
    window.addEventListener("load", measure);
    if (hero && io) new io(function (e) { heroSeen = e[0].isIntersecting; if (heroSeen) queue(); }).observe(hero);
    var fine = window.matchMedia && window.matchMedia("(hover: hover) and (pointer: fine)").matches;
    if (hero && fine) {
      hero.addEventListener("pointermove", function (e) {
        var y = window.scrollY || window.pageYOffset;
        px = e.clientX / heroW - 0.5;
        py = (e.clientY + y - heroTop) / heroH - 0.5;
        queue();
      }, { passive: true });
      hero.addEventListener("pointerleave", function () { px = 0; py = 0; queue(); });
    }
    function settle() {
      // reduced motion switched on while the page is open: put everything back
      if (!calm()) { queue(); return; }
      for (var i = 0; i < layers.length; i++) layers[i].n.style.transform = "";
      if (inner) { inner.style.transform = ""; inner.style.opacity = ""; }
    }
    if (still.addEventListener) still.addEventListener("change", settle);
    queue();

    // sections come up as they are read: only what is below the fold when
    // the page opens, so nothing on screen ever blinks away and back
    var REVEAL = ".section .eyebrow, .section h2, .section-lead, .two-col > div, .features > li, .shots > li, .numbers > div, .sets-block > h3, .legend, .install-notes > div, .channels > li";
    function reveal() {
      if (!io || calm()) return;
      var seen = new io(function (entries) {
        for (var i = 0; i < entries.length; i++) {
          if (!entries[i].isIntersecting) continue;
          entries[i].target.classList.add("in");
          seen.unobserve(entries[i].target);
        }
      }, { rootMargin: "0px 0px -8% 0px", threshold: 0.08 });
      var all = document.querySelectorAll(REVEAL);
      var fold = window.innerHeight;
      for (var i = 0; i < all.length; i++) {
        var n = all[i];
        if (n.getBoundingClientRect().top < fold) continue;
        // siblings in one grid follow each other a moment apart
        var idx = [].indexOf.call(n.parentNode.children, n);
        n.style.setProperty("--d", String(Math.min(idx, 8)));
        n.classList.add("reveal");
        seen.observe(n);
      }
    }
    reveal();

    // a set's progress bar grows when its row comes into view
    var grower = io && new io(function (entries) {
      for (var i = 0; i < entries.length; i++) {
        if (!entries[i].isIntersecting) continue;
        entries[i].target.parentNode.classList.add("shown");
        grower.unobserve(entries[i].target);
      }
    }, { threshold: 0.5 });
    // the summary line is watched, not the row: an open set is far taller
    // than the screen
    function grow(row) {
      if (!grower || calm()) { row.classList.add("shown"); return; }
      grower.observe(row.firstChild);
    }

    // a number counts up to itself the first time it is seen
    function countTo(dt, value) {
      if (!dt) return;
      var draw = function (v) { clear(dt); dt.appendChild(numT(Math.round(v))); };
      if (!io || calm()) { draw(value); return; }
      var r = dt.getBoundingClientRect();
      if (r.top < window.innerHeight && r.bottom > 0) { draw(value); return; }
      draw(0);
      var o = new io(function (e) {
        if (!e[0].isIntersecting) return;
        o.disconnect();
        var start = performance.now(), span = 1300;
        (function step(now) {
          var t = Math.min(1, (now - start) / span);
          draw(value * (1 - Math.pow(1 - t, 3)));
          if (t < 1) requestAnimationFrame(step);
        })(start);
      }, { threshold: 0.6 });
      o.observe(dt);
    }

    // the light that follows the pointer over a panel
    // (at most once a frame: the last move of the frame wins)
    var lastMove = null, moveQueued = false;
    function light() {
      moveQueued = false;
      var e = lastMove, g = e && e.target.closest && e.target.closest(".glass");
      if (!g) return;
      var r = g.getBoundingClientRect();
      g.style.setProperty("--mx", (e.clientX - r.left).toFixed(0) + "px");
      g.style.setProperty("--my", (e.clientY - r.top).toFixed(0) + "px");
    }
    document.addEventListener("pointermove", function (e) {
      if (e.pointerType !== "mouse" || calm()) return;
      lastMove = e;
      if (!moveQueued) { moveQueued = true; requestAnimationFrame(light); }
    }, { passive: true });

    // the menu on narrow screens
    var menu = document.querySelector(".menu-btn"), nav = document.getElementById("nav");
    function setMenu(open) {
      if (!menu || !nav) return;
      menu.setAttribute("aria-expanded", open ? "true" : "false");
      nav.classList.toggle("open", open);
      if (open && top) top.classList.add("solid");
      else frost();
    }
    if (menu && nav) {
      menu.addEventListener("click", function (e) {
        var open = menu.getAttribute("aria-expanded") !== "true";
        setMenu(open);
        // opened from the keyboard (a click with no pointer), focus goes in
        if (open && e.detail === 0) { var first = nav.querySelector("a"); if (first) first.focus(); }
      });
      // while it is open, Tab goes round the button and the menu's links
      document.addEventListener("keydown", function (e) {
        if (e.key !== "Tab" || menu.getAttribute("aria-expanded") !== "true") return;
        var items = nav.querySelectorAll("a"), last = items[items.length - 1];
        if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); menu.focus(); }
        else if (e.shiftKey && document.activeElement === menu) { e.preventDefault(); last.focus(); }
      });
      nav.addEventListener("click", function (e) { if (e.target.closest && e.target.closest("a")) setMenu(false); });
      document.addEventListener("keydown", function (e) {
        if (e.key === "Escape" && menu.getAttribute("aria-expanded") === "true") { setMenu(false); menu.focus(); }
      });
      document.addEventListener("click", function (e) {
        if (menu.getAttribute("aria-expanded") === "true" && !(e.target.closest && e.target.closest(".top"))) setMenu(false);
      });
    }

    // the section in view, marked in the bar
    var links = nav ? nav.querySelectorAll("a[href^='#']") : [];
    if (io && links.length) {
      var byId = {};
      for (var li = 0; li < links.length; li++) byId[links[li].getAttribute("href").slice(1)] = links[li];
      var spy = new io(function (entries) {
        for (var i = 0; i < entries.length; i++) {
          var a = byId[entries[i].target.id];
          if (!a || !entries[i].isIntersecting) continue;
          for (var j = 0; j < links.length; j++) { links[j].classList.remove("here"); links[j].removeAttribute("aria-current"); }
          a.classList.add("here");
          a.setAttribute("aria-current", "location");
        }
      }, { rootMargin: "-45% 0px -50% 0px" });
      for (var id in byId) { var sec = document.getElementById(id); if (sec) spy.observe(sec); }
    }

    // the screenshots, large
    var box = document.getElementById("lightbox");
    var shots = [].slice.call(document.querySelectorAll(".shot-link"));
    var at = 0;
    function show(i) {
      if (!box) return;
      at = (i + shots.length) % shots.length;
      var a = shots[at], img = a.querySelector("img");
      var big = document.getElementById("lightbox-img");
      big.src = a.getAttribute("href");
      big.alt = img ? img.alt : "";
      var cap = document.getElementById("lightbox-cap");
      clear(cap);
      var fc = a.parentNode.querySelector("figcaption");
      if (fc) for (var c = 0; c < fc.childNodes.length; c++) cap.appendChild(fc.childNodes[c].cloneNode(true));
    }
    if (box && box.showModal && shots.length) {
      shots.forEach(function (a, i) {
        a.addEventListener("click", function (e) {
          if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
          e.preventDefault();
          show(i);
          box.showModal();
          document.getElementById("lightbox-close").focus();
        });
      });
      document.getElementById("lightbox-close").addEventListener("click", function () { box.close(); });
      document.getElementById("lightbox-prev").addEventListener("click", function () { show(at - 1); });
      document.getElementById("lightbox-next").addEventListener("click", function () { show(at + 1); });
      box.addEventListener("click", function (e) { if (e.target === box) box.close(); });
      box.addEventListener("keydown", function (e) {
        if (e.key === "ArrowLeft") show(at - 1);
        else if (e.key === "ArrowRight") show(at + 1);
      });
      box.addEventListener("close", function () { shots[at].focus(); });
    }

    return { grow: grow, countTo: countTo };
  })();

  // ------------------------------------------------------------ the contact form
  // Sent with fetch as JSON to the feedback service (docs/feedback.md
  // §"The landing page's contact form"), whose address is the form's
  // data-endpoint; a page served from this machine for a try uses the
  // local service instead. Nothing is kept in the browser.
  (function contact() {
    var form = document.getElementById("contact-form");
    if (!form || !window.fetch) return;
    var card = document.getElementById("contact-card");
    var msg = document.getElementById("c-message"), email = document.getElementById("c-email");
    var count = document.getElementById("c-count"), status = document.getElementById("c-status");
    var send = document.getElementById("c-send");
    var MAX = 5000;
    var loopback = /^(localhost|127\.0\.0\.1|\[::1\])$/.test(location.hostname);
    var endpoint = (loopback && form.getAttribute("data-endpoint-local")) || form.getAttribute("data-endpoint");

    function counted() {
      var n = msg.value.length;
      count.textContent = num(n) + " / " + num(MAX);
      count.classList.toggle("near", n > MAX * 0.9);
    }
    msg.addEventListener("input", function () { counted(); msg.removeAttribute("aria-invalid"); msg.setAttribute("aria-describedby", "c-count"); });
    email.addEventListener("input", function () { email.removeAttribute("aria-invalid"); email.removeAttribute("aria-describedby"); });
    redraw.push(counted);
    counted();

    function say(cls, en, de) {
      status.className = "form-status" + (cls ? " " + cls : "");
      clear(status);
      if (en) status.appendChild(t(en, de));
    }

    // the button says where the message is: idle, sending, sent, error. It
    // is never `disabled` while sending, which would drop the keyboard's
    // focus to the page; a second press is ignored instead.
    function state(s) {
      send.setAttribute("data-state", s);
      if (s === "sending") { send.setAttribute("aria-disabled", "true"); send.setAttribute("aria-busy", "true"); }
      else { send.removeAttribute("aria-disabled"); send.removeAttribute("aria-busy"); }
    }
    form.addEventListener("submit", function (e) {
      e.preventDefault();
      if (send.getAttribute("data-state") === "sending") return;
      var text = msg.value.trim();
      var mail = email.value.trim();
      if (!text) {
        msg.setAttribute("aria-invalid", "true");
        msg.setAttribute("aria-describedby", "c-status");
        say("bad", "Please write a message first.", "Bitte schreib zuerst eine Nachricht.");
        msg.focus();
        return;
      }
      if (mail && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(mail)) {
        email.setAttribute("aria-invalid", "true");
        email.setAttribute("aria-describedby", "c-status");
        say("bad", "That e-mail address does not look right. Leave it empty if you want no answer.", "Diese E-Mail-Adresse sieht nicht richtig aus. Lass sie leer, wenn du keine Antwort willst.");
        email.focus();
        return;
      }
      var body = {
        name: document.getElementById("c-name").value.trim() || null,
        email: mail || null,
        message: text,
        language: lang,
        website: document.getElementById("c-website").value
      };
      state("sending");
      say("", "Sending…", "Wird gesendet …");
      fetch(endpoint, {
        method: "POST", credentials: "omit", referrerPolicy: "no-referrer", mode: "cors",
        headers: { "Content-Type": "application/json" }, body: JSON.stringify(body)
      }).then(function (r) {
        if (r.status === 201) {
          state("sent");
          say("ok", "Sent.", "Gesendet.");
          // a moment for the button's tick, then the thank-you
          setTimeout(function () {
            form.reset();
            counted();
            say("", "", "");
            state("idle");
            card.classList.add("sent");
            var done = document.getElementById("contact-done");
            if (done) done.focus();
          }, still.matches ? 0 : 650);
          return;
        }
        state("error");
        if (r.status === 429) say("bad", "Too many messages from here in the last hour. Please try again later.", "Zu viele Nachrichten von hier in der letzten Stunde. Bitte versuch es später noch einmal.");
        else if (r.status === 400 || r.status === 413) say("bad", "The message was not accepted. Is it shorter than 5,000 characters, and the address a real one?", "Die Nachricht wurde nicht angenommen. Ist sie kürzer als 5.000 Zeichen und die Adresse eine echte?");
        else say("bad", "The form could not reach us just now. Please try again later, or write on Discord or GitHub.", "Das Formular hat uns gerade nicht erreicht. Bitte versuch es später noch einmal, oder schreib auf Discord oder GitHub.");
      }).catch(function () {
        state("error");
        say("bad", "The form could not reach us just now. Please try again later, or write on Discord or GitHub.", "Das Formular hat uns gerade nicht erreicht. Bitte versuch es später noch einmal, oder schreib auf Discord oder GitHub.");
      });
    });
    var again = document.getElementById("c-again");
    if (again) again.addEventListener("click", function () { card.classList.remove("sent"); msg.focus(); });
  })();

  // ------------------------------------------------------------ sets and card status
  // data/sets.json (scripts/site/card_status.py) lists every set in release
  // order with its counts; a set's cards are in data/sets/set-<code>.json and
  // are fetched only when that set is opened. Card pictures come straight
  // from Scryfall's image host (docs/legal.md §3), unaltered, lazily, and
  // only for an opened set; nothing is drawn on a picture, the status sits
  // under it.
  var DATA = "/site/data/";
  var IMG_HOST = "https://cards.scryfall.io";
  var SCRYFALL_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
  var STATUS = {
    implemented: { group: "implemented", en: "Full", de: "Voll" },
    partial: { group: "partial", en: "Partial", de: "Teilweise" },
    unimplemented: { group: "none", en: "Not yet", de: "Noch nicht" },
    absent: { group: "none", en: "Not yet", de: "Noch nicht" },
    never: { group: "never", en: "Never", de: "Nie" }
  };
  var NEVER = {
    ante: ["it plays for ante (CR 407), and no table here plays for ante", "sie spielt um Ante (CR 407), und kein Tisch hier spielt um Ante"],
    dexterity: ["it asks a physical act of the player, which no game state describes", "sie verlangt eine körperliche Handlung, die kein Spielzustand beschreibt"],
    subgame: ["it starts a game within the game (CR 728), which the engine does not host", "sie startet ein Spiel im Spiel (CR 728), das die Engine nicht ausrichtet"]
  };
  function num(n) { return Number(n).toLocaleString(lang === "de" ? "de-DE" : "en-GB"); }
  function numT(n) { return t(Number(n).toLocaleString("en-GB"), Number(n).toLocaleString("de-DE")); }
  function pct(done, of) {
    if (!of) return 100;
    var p = done * 100 / of;
    return p > 0 && p < 1 ? 1 : (p > 99 && p < 100 ? 99 : Math.round(p));
  }
  function why(c) {
    switch (c.status) {
      case "implemented": return [tt("Plays as printed.", "Spielt wie gedruckt.")];
      case "partial": return [tt("Partly implemented. Missing: ", "Teilweise umgesetzt. Es fehlt (Notiz auf Englisch): ") + (c.note || "")];
      case "unimplemented": return [tt("In the card pool, but its abilities are not written yet: not playable.", "Im Kartenpool, aber ihre Fähigkeiten sind noch nicht geschrieben: nicht spielbar.")];
      case "absent": return [tt("Not in the card pool yet.", "Noch nicht im Kartenpool.")];
      case "never":
        var r = NEVER[c.note];
        return [tt("Never built: ", "Wird nie gebaut: ") + (r ? tt(r[0], r[1]) : (c.note || ""))];
    }
    return [""];
  }

  function bar(s) {
    var of = s.total - s.never;
    var b = el("div", { class: "pbar", "aria-hidden": "true" }, [
      el("span", { class: "pbar-full" }), el("span", { class: "pbar-partial" })
    ]);
    // CSSOM, not a style attribute: the page's CSP allows no inline style.
    b.firstChild.style.width = (of ? s.implemented * 100 / of : 100) + "%";
    b.lastChild.style.width = (of ? s.partial * 100 / of : 0) + "%";
    return b;
  }
  function counts(s) {
    var none = s.unimplemented + s.absent;
    var parts = [
      el("span", { class: "c c-implemented" }, [numT(s.implemented), " ", t("full", "voll")]),
      el("span", { class: "c c-partial" }, [numT(s.partial), " ", t("partial", "teilweise")]),
      el("span", { class: "c c-none" }, [numT(none), " ", t("not yet", "noch nicht")])
    ];
    if (s.never) parts.push(el("span", { class: "c c-never" }, [numT(s.never), " ", t("never", "nie")]));
    return el("span", { class: "set-counts" }, parts);
  }

  // Appends make(item) for every item to parent in slices of about 8 ms,
  // yielding between them, so a set list of hundreds of rows or a set of
  // hundreds of cards never blocks the page for long; done() after the last.
  function inChunks(items, make, parent, done) {
    var i = 0;
    (function slice() {
      var start = Date.now(), frag = document.createDocumentFragment();
      while (i < items.length && (Date.now() - start < 8 || i === 0)) frag.appendChild(make(items[i++]));
      parent.appendChild(frag);
      if (i < items.length) setTimeout(slice, 0);
      else if (done) done();
    })();
  }

  // ------------------------------------------------------------ card tiles
  // A set's grid is one stop for Tab; the arrow keys, Home and End move in
  // it (a roving tabindex), Enter or Space opens the card large. With a
  // mouse, resting on a card shows it large beside the pointer after a
  // moment (and focus does the same beside the card); a click or a tap
  // opens it in a dialog. The large picture is Scryfall's "normal" image,
  // fetched only then, unaltered.
  //
  // The line under a card shown large. Keep it here, in both languages: the
  // legal review may ask for more (docs/legal.md §3).
  var CARD_CREDIT = {
    en: "Card image provided by Scryfall, unaltered. Baylee is not affiliated with Scryfall or Wizards of the Coast.",
    de: "Kartenbild bereitgestellt von Scryfall, unverändert. Baylee ist weder mit Scryfall noch mit Wizards of the Coast verbunden."
  };
  var cardOf = window.WeakMap ? new WeakMap() : null;
  // A picture only for a card in the pool, and only where the data names
  // one: card_status.py writes none for a card outside the pool or a
  // Universes Beyond printing (docs/legal.md §3), and this holds it here too.
  var POOL = { implemented: 1, partial: 1, unimplemented: 1, never: 1 };
  function imgOf(c, size) {
    return POOL[c.status] && c.img && SCRYFALL_ID.test(c.img) ? IMG_HOST + "/" + size + "/front/" + c.img[0] + "/" + c.img[1] + "/" + c.img + ".jpg" : null;
  }
  // the card's page on Scryfall, where its artist and printing are: the
  // printing's own page where the data has it, else Scryfall's search for
  // the card, which opens the card's page
  function pageOf(c) {
    return c.uri && /^https:\/\/scryfall\.com\/card\//.test(c.uri) ? c.uri : "https://scryfall.com/search?q=oracleid%3A" + encodeURIComponent(c.oracle_id);
  }

  function tile(c) {
    var st = STATUS[c.status];
    var src = imgOf(c, "small");
    var pic = src
      ? el("img", { src: src, alt: "", loading: "lazy", decoding: "async", width: "146", height: "204", referrerpolicy: "no-referrer" })
      : el("span", { class: "ctile-blank", "aria-hidden": "true" }, [c.name]);
    var badge = el("span", { class: "badge b-" + st.group }, [t(st.en, st.de)]);
    // the reason, in both languages like every other run, for assistive technology
    var reason = el("span", { class: "vh" }, [t(" " + whyIn(c, "en"), " " + whyIn(c, "de"))]);
    // the tile is a link to the card on Scryfall (artist and printing)
    var link = el("a", { class: "ctile-link", href: pageOf(c), target: "_blank", rel: "noopener", tabindex: "-1" }, [
      el("span", { class: "ctile-pic" }, [pic]),
      el("span", { class: "ctile-name", text: c.name })
    ]);
    var n = el("li", { class: "ctile", "data-group": st.group }, [link, el("div", { class: "ctile-cap" }, [badge, reason])]);
    if (cardOf) cardOf.set(n, c);
    return n;
  }
  function whyIn(c, l) { var keep = lang; lang = l; var w = why(c)[0]; lang = keep; return w; }

  // the tiles a filter leaves showing, in order
  function shown(grid) {
    return [].filter.call(grid.children, function (n) { return n.offsetParent !== null; });
  }
  // exactly one showing tile's link is the grid's stop for Tab; returns it
  function rove(grid, to) {
    var vis = shown(grid);
    if (!vis.length) return null;
    var cur = to || grid.querySelector(".ctile-link[tabindex='0']");
    cur = cur && cur.closest(".ctile");
    if (!cur || vis.indexOf(cur) < 0) cur = vis[0];
    for (var i = 0; i < grid.children.length; i++) grid.children[i].firstChild.setAttribute("tabindex", grid.children[i] === cur ? "0" : "-1");
    return cur.firstChild;
  }
  function gridKeys(grid) {
    grid.addEventListener("keydown", function (e) {
      var tileEl = e.target.closest && e.target.closest(".ctile");
      if (!tileEl) return;
      // Enter follows the link to Scryfall; Space shows the card large here
      if (e.key === " ") { e.preventDefault(); openCard(tileEl); return; }
      var vis = shown(grid), at = vis.indexOf(tileEl);
      if (at < 0) return;
      var cols = 1;
      while (cols < vis.length && vis[cols].offsetTop === vis[0].offsetTop) cols++;
      var to = { ArrowRight: at + 1, ArrowLeft: at - 1, ArrowDown: at + cols, ArrowUp: at - cols, Home: 0, End: vis.length - 1 }[e.key];
      if (to === undefined) return;
      e.preventDefault();
      to = Math.max(0, Math.min(vis.length - 1, to));
      rove(grid, vis[to].firstChild).focus();
    });
    grid.addEventListener("focusin", function (e) {
      var tileEl = e.target.closest && e.target.closest(".ctile");
      if (tileEl) rove(grid, tileEl);
    });
    // without a hover (a touch screen) the first tap shows the card large,
    // with its link to Scryfall; with a mouse the click follows the link
    grid.addEventListener("click", function (e) {
      var tileEl = e.target.closest && e.target.closest(".ctile");
      if (!tileEl || hoverable) return;
      e.preventDefault();
      openCard(tileEl);
    });
  }

  // the card large beside the pointer (or the focused card)
  var preview = null, previewTimer = 0, previewFor = null;
  var hoverable = window.matchMedia && window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  function cardText(c) {
    var st = STATUS[c.status];
    var out = [el("strong", { class: "cp-name", text: c.name }), el("span", { class: "badge b-" + st.group }, [t(st.en, st.de)]),
      el("span", { class: "cp-why" }, [t(whyIn(c, "en"), whyIn(c, "de"))])];
    if (c.artist) out.push(el("span", { class: "cp-artist" }, [t("Illustrated by ", "Illustriert von "), c.artist]));
    return out;
  }
  function credit() { return el("p", { class: "cp-credit" }, [t(CARD_CREDIT.en, CARD_CREDIT.de)]); }
  // the large picture, or nothing for a card shown by name only
  function bigPic(c) {
    var src = imgOf(c, "normal");
    return src ? el("div", { class: "cp-pic" }, [el("img", { src: src, alt: "", width: "488", height: "680", decoding: "async", referrerpolicy: "no-referrer" })]) : null;
  }
  function showPreview(tileEl, x, y) {
    var c = cardOf && cardOf.get(tileEl);
    if (!c) return;
    if (!preview) {
      preview = el("div", { class: "cpreview", "aria-hidden": "true" });
      document.body.appendChild(preview);
    }
    if (previewFor !== tileEl) {
      clear(preview);
      var pic = bigPic(c);
      if (pic) preview.appendChild(pic);
      preview.appendChild(el("div", { class: "cp-cap" }, cardText(c)));
      preview.appendChild(el("p", { class: "cp-go" }, [t("Click for the artist and printing on Scryfall.", "Klick: Künstler und Druck bei Scryfall.")]));
      preview.appendChild(credit());
      previewFor = tileEl;
    }
    preview.classList.add("on");
    place(x, y, tileEl);
  }
  function place(x, y, tileEl) {
    if (!preview) return;
    var w = preview.offsetWidth, h = preview.offsetHeight, W = window.innerWidth, H = window.innerHeight, gap = 18;
    if (x == null) { var r = tileEl.getBoundingClientRect(); x = r.right; y = r.top + r.height / 2; gap = 12; }
    var left = x + gap + w > W - 8 ? x - gap - w : x + gap;
    if (left < 8) left = 8;
    var topY = Math.max(8, Math.min(H - h - 8, y - h / 2));
    preview.style.transform = "translate3d(" + Math.round(left) + "px," + Math.round(topY) + "px,0)";
  }
  function hidePreview() {
    clearTimeout(previewTimer);
    if (preview) preview.classList.remove("on");
  }
  if (hoverable) {
    document.addEventListener("pointerover", function (e) {
      var tileEl = e.target.closest && e.target.closest(".ctile");
      if (!tileEl || e.pointerType !== "mouse") return;
      clearTimeout(previewTimer);
      var x = e.clientX, y = e.clientY;
      previewTimer = setTimeout(function () { showPreview(tileEl, x, y); }, preview && preview.classList.contains("on") ? 0 : 220);
    });
    document.addEventListener("pointermove", function (e) {
      if (preview && preview.classList.contains("on") && previewFor && previewFor.contains(e.target)) place(e.clientX, e.clientY);
    }, { passive: true });
    document.addEventListener("pointerout", function (e) {
      var tileEl = e.target.closest && e.target.closest(".ctile");
      if (tileEl && !(e.relatedTarget && tileEl.contains(e.relatedTarget))) hidePreview();
    });
  }
  // focus by keyboard shows it too, beside the card
  document.addEventListener("focusin", function (e) {
    var tileEl = e.target.closest && e.target.closest(".ctile");
    if (!tileEl) { hidePreview(); return; }
    if (e.target.matches && e.target.matches(":focus-visible")) { clearTimeout(previewTimer); showPreview(tileEl); }
  });
  document.addEventListener("focusout", function (e) { if (e.target.closest && e.target.closest(".ctile")) hidePreview(); });
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") hidePreview(); });
  window.addEventListener("scroll", hidePreview, { passive: true });

  // the card in a dialog: a tap, a click, Enter or Space
  var cardview = document.getElementById("cardview"), cardFrom = null;
  function openCard(tileEl) {
    var c = cardOf && cardOf.get(tileEl);
    if (!c || !cardview || !cardview.showModal) return;
    hidePreview();
    var body = document.getElementById("cardview-body");
    clear(body);
    var pic = bigPic(c);
    if (pic) body.appendChild(el("a", { href: pageOf(c), target: "_blank", rel: "noopener", tabindex: "-1", class: "cp-piclink" }, [pic]));
    var cap = el("div", { class: "cp-cap" }, cardText(c));
    cap.firstChild.id = "cardview-name";
    body.appendChild(cap);
    body.appendChild(el("p", { class: "cp-go" }, [el("a", { href: pageOf(c), target: "_blank", rel: "noopener", class: "button button-ghost" }, [t("Artist and printing on Scryfall", "Künstler und Druck bei Scryfall")])]));
    body.appendChild(credit());
    cardFrom = tileEl;
    cardview.showModal();
    document.getElementById("cardview-close").focus();
  }
  if (cardview) {
    document.getElementById("cardview-close").addEventListener("click", function () { cardview.close(); });
    cardview.addEventListener("click", function (e) { if (e.target === cardview) cardview.close(); });
    cardview.addEventListener("close", function () { if (cardFrom && document.contains(cardFrom)) cardFrom.firstChild.focus(); });
  }

  function filterBar(grid, s) {
    var groups = [
      ["all", "All", "Alle", s.total],
      ["implemented", "Full", "Voll", s.implemented],
      ["partial", "Partial", "Teilweise", s.partial],
      ["none", "Not yet", "Noch nicht", s.unimplemented + s.absent],
      ["never", "Never", "Nie", s.never]
    ];
    var fbar = el("div", { class: "cfilter", role: "group" });
    fbar.setAttribute("aria-label", "Filter");
    groups.forEach(function (g) {
      if (g[0] !== "all" && !g[3]) return;
      var b = el("button", { type: "button", class: "cfilter-btn", "aria-pressed": g[0] === "all" ? "true" : "false", "data-filter": g[0] },
        [t(g[1], g[2]), " ", el("span", { class: "cfilter-n" }, [numT(g[3])])]);
      fbar.appendChild(b);
    });
    fbar.addEventListener("click", function (e) {
      var b = e.target.closest && e.target.closest(".cfilter-btn");
      if (!b) return;
      grid.setAttribute("data-filter", b.getAttribute("data-filter"));
      var all = fbar.querySelectorAll(".cfilter-btn");
      for (var i = 0; i < all.length; i++) all[i].setAttribute("aria-pressed", all[i] === b ? "true" : "false");
      rove(grid);
    });
    return fbar;
  }

  function openSet(d, s) {
    if (d.getAttribute("data-loaded")) return;
    d.setAttribute("data-loaded", "1");
    var body = d.querySelector(".set-body");
    body.appendChild(el("p", { class: "small" }, [t("Loading…", "Lädt …")]));
    fetch(DATA + "sets/set-" + encodeURIComponent(s.code) + ".json", { credentials: "omit", headers: { Accept: "application/json" } })
      .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
      .then(function (doc) {
        clear(body);
        var grid = el("ul", { class: "cgrid", "data-filter": "all" });
        grid.setAttribute("aria-label", tt("Cards of ", "Karten aus ") + s.name);
        gridKeys(grid);
        body.appendChild(filterBar(grid, s));
        body.appendChild(el("p", { class: "small cgrid-hint" }, [t("Arrow keys move between cards; Space shows one large, Enter opens it on Scryfall.", "Pfeiltasten wechseln die Karte; die Leertaste zeigt sie groß, Enter öffnet sie bei Scryfall.")]));
        body.appendChild(grid);
        body.appendChild(el("p", { class: "small set-foot" }, [
          el("a", { href: "https://scryfall.com/sets/" + encodeURIComponent(s.code), rel: "noopener" }, [t("This set on Scryfall", "Dieses Set bei Scryfall")]),
          " · ", t("Images: Scryfall · © Wizards of the Coast", "Bilder: Scryfall · © Wizards of the Coast")
        ]));
        inChunks(doc.cards, tile, grid, function () { rove(grid); });
      })
      .catch(function () {
        d.removeAttribute("data-loaded");
        clear(body);
        body.appendChild(el("p", { class: "small" }, [t("The card list did not load. Close and open the set to try again.", "Die Kartenliste wurde nicht geladen. Schließe das Set und öffne es erneut.")]));
      });
  }

  function setRow(s) {
    var of = s.total - s.never;
    var p = pct(s.implemented, of);
    var year = s.released ? s.released.slice(0, 4) : "";
    var label = el("span", { class: "vh" }, [t(p + " percent full", p + " Prozent voll")]);
    var sum = el("summary", { class: "set-sum" }, [
      el("span", { class: "set-title" }, [
        el("span", { class: "set-name", text: s.name }),
        el("span", { class: "set-meta" }, [s.code.toUpperCase() + (year ? " · " + year : "") + " · ", numT(s.total), " ", t(s.total === 1 ? "card" : "cards", s.total === 1 ? "Karte" : "Karten")])
      ]),
      el("span", { class: "set-progress" }, [
        el("span", { class: "set-pct" + (p === 100 ? " done" : ""), "aria-hidden": "true", text: p + " %" }), label, bar(s)
      ]),
      counts(s)
    ]);
    var d = el("details", { class: "set", "data-code": s.code }, [sum, el("div", { class: "set-body" })]);
    d.addEventListener("toggle", function () { if (d.open) openSet(d, s); });
    motion.grow(d);
    return d;
  }

  // ------------------------------------------------------------ the sets
  // A summary first: the whole corpus, the finished sets, and the next ones
  // in release order. The full list waits behind "Show all sets", in eras of
  // four years whose rows exist only while the era is open; a search lists
  // its matches flat. data/sets.json is fetched only when the cards section
  // comes near (or a link names a set), a set's cards only when it opens.
  var ERA = 4, FIRST_YEAR = 1993, NEXT_UP = 3, FOUND_MAX = 40;
  var setsDoc = null, setsLoading = null, built = false;
  function yearOf(s) { return parseInt((s.released || "").slice(0, 4), 10) || FIRST_YEAR; }
  function eraOf(s) { return FIRST_YEAR + Math.floor((yearOf(s) - FIRST_YEAR) / ERA) * ERA; }
  function begun(s) { return s.implemented + s.partial > 0; }
  function finished(s) { var of = s.total - s.never; return of > 0 && s.implemented === of; }
  function sum(list) {
    var a = { total: 0, implemented: 0, partial: 0, unimplemented: 0, absent: 0, never: 0 };
    list.forEach(function (s) { for (var k in a) a[k] += s[k]; });
    return a;
  }
  var ui = {
    summary: document.getElementById("sets-summary"),
    toggle: document.getElementById("sets-toggle"),
    all: document.getElementById("sets-all"),
    list: document.getElementById("sets-list"),
    found: document.getElementById("sets-found"),
    q: document.getElementById("sets-q"),
    started: document.getElementById("sets-started")
  };

  function loadSets() {
    if (setsLoading) return setsLoading;
    if (!window.fetch) return (setsLoading = Promise.reject(new Error("no fetch")));
    setsLoading = fetch(DATA + "sets.json", { credentials: "omit", headers: { Accept: "application/json" } })
      .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
      .then(function (doc) { setsDoc = doc; drawSummary(doc); return doc; });
    setsLoading.catch(function () { /* the static text stays: it says where the list is */ });
    return setsLoading;
  }

  function drawSummary(doc) {
    var tot = doc.totals, all = doc.corpus - tot.never;
    motion.countTo(document.getElementById("num-full"), tot.implemented);
    motion.countTo(document.getElementById("num-partial"), tot.partial);
    motion.countTo(document.getElementById("num-corpus"), all);
    var box = ui.summary;
    if (!box) return;
    clear(box);
    var p = pct(tot.implemented, all);
    var overall = el("div", { class: "sets-overall glass" }, [
      el("p", { class: "sets-total" }, [t(
        doc.sets.length + " sets, " + Number(doc.corpus).toLocaleString("en-GB") + " cards: " + Number(tot.implemented).toLocaleString("en-GB") + " full (" + p + " %), " + tot.partial + " partial, " + Number(tot.unimplemented + tot.absent).toLocaleString("en-GB") + " not yet, " + tot.never + " never. Measured at commit " + String(doc.rev).slice(0, 9) + ".",
        doc.sets.length + " Sets, " + Number(doc.corpus).toLocaleString("de-DE") + " Karten: " + Number(tot.implemented).toLocaleString("de-DE") + " voll (" + p + " %), " + tot.partial + " teilweise, " + Number(tot.unimplemented + tot.absent).toLocaleString("de-DE") + " noch nicht, " + tot.never + " nie. Gemessen an Commit " + String(doc.rev).slice(0, 9) + "."
      )]),
      bar({ total: doc.corpus, never: tot.never, implemented: tot.implemented, partial: tot.partial })
    ]);
    motion.grow(overall);
    box.appendChild(overall);
    var done = doc.sets.filter(finished);
    if (done.length) {
      var chips = el("ul", { class: "set-chips" });
      done.forEach(function (s) { chips.appendChild(el("li", null, [el("a", { href: "#set-" + s.code, class: "chip" }, [s.name])])); });
      box.appendChild(el("h4", { class: "sets-sub" }, [t("Finished", "Fertig")]));
      box.appendChild(chips);
    }
    var next = doc.sets.filter(function (s) { return s.total - s.never > 0 && !finished(s); }).slice(0, NEXT_UP);
    if (next.length) {
      box.appendChild(el("h4", { class: "sets-sub" }, [t("Next, in release order", "Als Nächstes, in Erscheinungsreihenfolge")]));
      var rows = el("div", { class: "sets" });
      next.forEach(function (s) { rows.appendChild(setRow(s)); });
      box.appendChild(rows);
    }
    if (ui.toggle) {
      clear(ui.toggle);
      ui.toggle.appendChild(t("Show all " + doc.sets.length + " sets", "Alle " + doc.sets.length + " Sets anzeigen"));
      ui.toggle.hidden = false;
    }
  }

  // the full list: eras, each rendered when opened and emptied when closed
  function era(from, list) {
    var a = sum(list), to = Math.min(from + ERA - 1, Math.max.apply(null, list.map(yearOf)));
    var of = a.total - a.never, p = pct(a.implemented, of);
    var d = el("details", { class: "set set-era", "data-era": String(from) }, [
      el("summary", { class: "set-sum" }, [
        el("span", { class: "set-title" }, [
          el("span", { class: "set-name", text: from === to ? String(from) : from + "–" + to }),
          el("span", { class: "set-meta" }, [numT(list.length), " ", t(list.length === 1 ? "set" : "sets", "Sets"), " · ", numT(a.total), " ", t("cards", "Karten")])
        ]),
        el("span", { class: "set-progress" }, [el("span", { class: "set-pct" + (p === 100 ? " done" : ""), "aria-hidden": "true", text: p + " %" }),
          el("span", { class: "vh" }, [t(p + " percent full", p + " Prozent voll")]), bar(a)]),
        counts(a)
      ]),
      el("div", { class: "era-body sets" })
    ]);
    d.addEventListener("toggle", function () { if (d.open) fillEra(d, list); else { clear(d.lastChild); d.removeAttribute("data-filled"); } });
    motion.grow(d);
    return d;
  }
  function fillEra(d, list, done) {
    if (d.getAttribute("data-filled")) { if (done) done(); return; }
    d.setAttribute("data-filled", "1");
    inChunks(list, setRow, d.lastChild, done);
  }
  function chosen() {
    var only = ui.started && ui.started.checked;
    return setsDoc.sets.filter(function (s) { return !only || begun(s); });
  }
  function drawEras() {
    clear(ui.list);
    var byEra = {}, order = [];
    chosen().forEach(function (s) { var e = eraOf(s); if (!byEra[e]) { byEra[e] = []; order.push(e); } byEra[e].push(s); });
    order.forEach(function (e) { ui.list.appendChild(era(e, byEra[e])); });
    say(ui.found, "", "");
  }
  function say(node, en, de) { if (!node) return; clear(node); if (en) node.appendChild(t(en, de)); }
  var searchTimer = 0;
  function search() {
    var needle = (ui.q && ui.q.value || "").trim().toLowerCase();
    if (!needle) { drawEras(); return; }
    var hits = chosen().filter(function (s) { return (s.name + " " + s.code).toLowerCase().indexOf(needle) >= 0; });
    clear(ui.list);
    hits.slice(0, FOUND_MAX).forEach(function (s) { ui.list.appendChild(setRow(s)); });
    var n = hits.length;
    if (!n) say(ui.found, "No set matches.", "Kein Set passt.");
    else if (n > FOUND_MAX) say(ui.found, n + " sets match; the first " + FOUND_MAX + " are shown. Type more to narrow it.", n + " Sets passen; die ersten " + FOUND_MAX + " stehen hier. Tippe mehr, um einzugrenzen.");
    else say(ui.found, n === 1 ? "1 set matches." : n + " sets match.", n === 1 ? "1 Set passt." : n + " Sets passen.");
  }
  function expand(open) {
    if (!ui.all || !ui.toggle) return;
    ui.all.hidden = !open;
    ui.toggle.setAttribute("aria-expanded", open ? "true" : "false");
    if (open && !built && setsDoc) { built = true; drawEras(); }
  }
  if (ui.toggle) ui.toggle.addEventListener("click", function () {
    loadSets().then(function () { expand(ui.toggle.getAttribute("aria-expanded") !== "true"); });
  });
  if (ui.q) ui.q.addEventListener("input", function () { clearTimeout(searchTimer); searchTimer = setTimeout(search, 150); });
  if (ui.started) ui.started.addEventListener("change", search);

  // #set-<code> opens that set wherever it is: its era, then the row
  function openDeep() {
    var m = /^#set-([a-z0-9]+)$/.exec(location.hash);
    if (!m) return;
    var code = m[1];
    loadSets().then(function (doc) {
      var s = doc.sets.filter(function (x) { return x.code === code; })[0];
      if (!s) return;
      if (ui.q && ui.q.value) { ui.q.value = ""; }
      if (ui.started && ui.started.checked && !begun(s)) ui.started.checked = false;
      if (!built) { built = true; drawEras(); } else if (!ui.list.querySelector(".set-era")) drawEras();
      expand(true);
      var d = ui.list.querySelector('.set-era[data-era="' + eraOf(s) + '"]');
      if (!d) return;
      fillEra(d, chosen().filter(function (x) { return eraOf(x) === eraOf(s); }), function () {
        d.open = true;
        var row = d.querySelector('details[data-code="' + code + '"]');
        if (!row) return;
        row.open = true;
        row.scrollIntoView({ block: "start", behavior: "instant" });
        row.firstChild.focus({ preventScroll: true });
      });
    });
  }
  window.addEventListener("hashchange", openDeep);
  if (/^#set-/.test(location.hash)) openDeep();
  else {
    var near = document.getElementById("cards");
    if (near && window.IntersectionObserver) {
      var watch = new IntersectionObserver(function (e) {
        if (!e[0].isIntersecting) return;
        watch.disconnect();
        loadSets();
      }, { rootMargin: "900px 0px" });
      watch.observe(near);
    } else loadSets();
  }
})();
