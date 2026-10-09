// Baylee's landing page. Three jobs, each one failing on its own:
//
//   1. the language: `?lang=de|en` wins, then what this browser chose last
//      time (localStorage, in try/catch), then the browser's languages;
//   2. the latest release from GitHub's public API, matched to this machine's
//      system and architecture, and the repository's state (CI on main, the
//      last commit, stars, open issues); answers are kept in sessionStorage
//      for a while, because GitHub allows 60 requests an hour per address;
//   3. the gateway's own `/info` and `/health`, which exist only where the
//      gateway serves this page.
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
      var card = el("div", { class: "plat" + (mine && mine.id === p.id ? " mine" : "") });
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
})();
