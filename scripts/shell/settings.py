"""WP5 (the shell design §12, §17): the settings screen's ten sections at
the seven sizes, English and German — shots, and the kit's overflow,
window, sibling, label-budget, 44-px hit and contrast checks over each
section; every section reached by its press name and the presses each one
draws listed; the language models' sheet opened and closed; and the
phone's measure (§12, M4-6): the section list and at least three rows in
view at 844 x 390.

usage (a dev-control client at the front door, before sign-in — the screen
is readable there; launch with BAYLEE_DEV_SCALE=1 for the contrast
reading):
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/settings.py OUTDIR
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check  # noqa: E402
import devctl  # noqa: E402

TOUCH = ((1180, 820), (844, 390), (920, 443), (640, 360))
# Each section's title, as its panel heads it (`Section::name`).
TITLES = {
    "Graphics": ("Graphics", "Grafik"),
    "Audio": ("Audio", "Audio"),
    "Display": ("Display & Interface", "Anzeige & Oberfl\u00e4che"),
    "Controls": ("Controls", "Steuerung"),
    "Gameplay": ("Gameplay", "Spielablauf"),
    "Account": ("Account", "Konto"),
    "Network": ("Network & Gateway", "Netzwerk & Gateway"),
    "LanguageModels": ("Language models", "Sprachmodelle"),
    "Updates": ("Updates", "Updates"),
    "Privacy": ("Privacy & Data", "Datenschutz & Daten"),
}
SECTIONS = (
    "Graphics", "Audio", "Display", "Controls", "Gameplay", "Account",
    "Network", "LanguageModels", "Updates", "Privacy",
)


def presses():
    return [c["press"] for c in devctl.controls()]


def press_until(name, done, tries=3):
    """A click right after a resize can be eaten by the window manager."""
    for _ in range(tries):
        if name in presses():
            devctl.press(name)
        try:
            devctl.until(done, 3)
            return True
        except TimeoutError:
            pass
    return False


def open_settings():
    if "Settings(CloseSettings)" in presses():
        return
    press_until("Settings(OpenSettings)", lambda: "Settings(CloseSettings)" in presses())
    time.sleep(0.6)


def rows_in_view(nodes, height):
    """Rows wholly inside the window (and the panel's scroll view)."""
    shown = []
    for n in nodes:
        if n.get("k") != "row" or n["y"] < 0 or n["y"] + n["h"] > height + 0.5:
            continue
        view = check.scrolled(nodes, n)
        if view is None or check.inside(n, view):
            shown.append(n)
    return shown


def shown(section, lang):
    """The panel heads `section`: its title outside the nav (the first
    panel drawn), so a click that missed is not measured as its section."""
    title = TITLES[section][0 if lang == "en" else 1]
    nodes = check.tree(devctl.state()["shell_nodes"])
    panels = [n for n in nodes if n.get("k") == "panel"]
    if not panels:
        return False
    in_nav = {d["index"] for d in check.descendants(nodes, panels[0]["index"])}
    return any(n.get("t") == title and n["index"] not in in_nav for n in nodes)


def into_view(name):
    """Scrolls the list a control is in (the sidebar, the rows) until it
    stands inside the window: on a phone both are taller than it."""
    for _ in range(12):
        st = devctl.state()
        hit = [c for c in st["lobby_controls"] if c["press"] == name]
        h = devctl.health()["height"]
        if not hit or 20 < hit[0]["at_y"] < h - 20:
            return
        devctl.call("/pointer", {"x": hit[0]["at_x"], "y": h / 2})
        devctl.call("/scroll", {"y": -3 if hit[0]["at_y"] > h / 2 else 3})


def section_faults(nodes, width, height, touch, german):
    return (
        check.check_overflow(nodes)
        + check.check_window(nodes, width, height)
        + check.check_siblings(nodes)
        + check.check_budget(nodes, german)
        + check.check_hit(nodes, touch)
    )


def main(out):
    os.makedirs(out, exist_ok=True)
    summary, failures, reached = [], 0, {}
    for width, height in check.SIZES:
        devctl.resize(width, height)
        time.sleep(1.0)
        touch = (width, height) in TOUCH
        for lang in ("en", "de"):
            devctl.shell(text_size="l", lang=lang, input="touch" if touch else "pointer")
            time.sleep(0.6)
            open_settings()
            for section in SECTIONS:
                name = f"Settings(Section({section}))"
                if name not in presses():
                    # Not offered on this build (Updates off a desktop).
                    summary.append(f"{section}-{width}x{height}-{lang}: not offered")
                    continue
                into_view(name)
                if not press_until(name, lambda: shown(section, lang)):
                    failures += 1
                    summary.append(f"{section}-{width}x{height}-{lang}: never shown")
                    print(summary[-1], flush=True)
                    continue
                devctl.settle(4)
                st = devctl.state()
                nodes = check.tree(st["shell_nodes"])
                drawn = sorted({c["press"] for c in st["lobby_controls"]
                                if c["press"].startswith("Settings(")
                                and "Section(" not in c["press"]})
                reached.setdefault(section, set()).update(drawn)
                faults = section_faults(nodes, width, height, touch, lang == "de")
                tag = f"{section}-{width}x{height}-{lang}"
                if (width, height) == (844, 390):
                    rows = rows_in_view(nodes, height)
                    navs = [n for n in nodes if n.get("k") == "menu_item"]
                    if len(navs) < 8:
                        faults.append(f"phone: the section list shows {len(navs)} items")
                    if section in ("Graphics", "Audio", "Display") and len(rows) < 3:
                        faults.append(f"phone: {len(rows)} rows in view, 3 wanted")
                    summary.append(f"    phone: {len(navs)} sections, {len(rows)} rows in view")
                contrast = []
                if lang == "en" or (width, height) in ((960, 700), (844, 390)):
                    png = os.path.join(out, f"settings-{tag}.png")
                    devctl.screenshot(png)
                    if lang == "en":
                        size = devctl.health()
                        more, contrast = check.check_contrast(
                            nodes, png, size["width"], size["height"]
                        )
                        faults += more
                failures += len(faults)
                worst = min((c[1] for c in contrast), default=None)
                summary.append(
                    f"{tag}: {len(nodes)} nodes, {len(drawn)} presses, {len(faults)} faults"
                    + (f", ink >= {worst}" if contrast else "")
                )
                for fault in faults[:6]:
                    summary.append("    " + fault)
                print("\n".join(summary[-1 - min(len(faults), 6):]), flush=True)
            # The language models' sheet: a new profile opens it, Esc puts
            # it away.
            if "Settings(Section(LanguageModels))" in presses():
                press_until("Settings(Section(LanguageModels))", lambda: True, tries=1)
                devctl.settle(4)
                if "Settings(Seat(Add))" in presses():
                    into_view("Settings(Seat(Add))")
                    press_until(
                        "Settings(Seat(Add))",
                        lambda: "Settings(CloseProfile)" in presses(),
                    )
                    devctl.settle(4)
                    st = devctl.state()
                    nodes = check.tree(st["shell_nodes"])
                    faults = check.check_window(nodes, width, height)
                    if "Settings(CloseProfile)" not in presses():
                        faults.append("the profile sheet did not open")
                    tag = f"llm-sheet-{width}x{height}-{lang}"
                    devctl.screenshot(os.path.join(out, f"settings-{tag}.png"))
                    devctl.key("Escape")  # leaves the name box
                    devctl.key("Escape")  # puts the sheet away
                    time.sleep(0.5)
                    if "Settings(CloseProfile)" in presses():
                        faults.append("Esc did not put the sheet away")
                    # Leave nothing behind: drop the unsaved profile.
                    if "Settings(Seat(Revert))" in presses():
                        devctl.press("Settings(Seat(Revert))")
                    failures += len(faults)
                    summary.append(f"{tag}: {len(faults)} faults")
                    for fault in faults[:6]:
                        summary.append("    " + fault)
                    print("\n".join(summary[-1 - min(len(faults), 6):]), flush=True)
    for section, drawn in sorted(reached.items()):
        summary.append(f"reached {section}: {len(drawn)} presses: " + ", ".join(sorted(drawn)))
        if not drawn:
            failures += 1
            summary.append(f"    {section} drew no control")
    devctl.shell(text_size="l", lang="en", input="auto")
    if "Settings(CloseSettings)" in presses():
        devctl.press("Settings(CloseSettings)")
    with open(os.path.join(out, "settings-check.log"), "w") as f:
        f.write("\n".join(summary) + f"\nfailures: {failures}\n")
    print(f"failures: {failures}")
    return failures


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(1 if main(sys.argv[1]) else 0)
