"""The report sheet (window B, .claude/ux-b6/windows-b6/DESIGN.md §B.3)
through dev-control: the W9b walk and the dumps at the two sizes the design
measures, in English and German.

  walk   W9b away from a table: F8 opens the sheet with the keyboard on the
         text; `#Lightning B` offers the pool, `↓ ↑` move, Enter writes
         `[Lightning Bolt] `, the span previews its printing under the
         pointer, the reference rides in `refs`; `Esc` puts suggestions away
         before it closes the sheet, the draft kept.
  dump   1708 x 1032 (Pointer) and 844 x 390 (Touch), en and de: the
         sheet's nodes (`"r":"report"`) held to check.py's overflow,
         window, budget and 44-px checks, a screenshot and the node dump per
         case; on the phone the field's top and the chip row's bottom above
         the keyboard band (y 195).

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/report.py walk
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/report.py table   # starts a house game
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/report.py dump OUTDIR

Start a client of your own with dev-control (never the owner's): its lobby's
front door, signed in nowhere, is enough.
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devctl
from check import (
    check_budget,
    check_hit,
    check_overflow,
    check_window,
    tree,
)

KEYBOARD_BAND = 195
ok = True


def say(cond, what):
    global ok
    ok &= bool(cond)
    print(("PASS " if cond else "FAIL ") + what, flush=True)


def report():
    return devctl.state()["report"]


def key(name, wait=0.4, **mods):
    answer = devctl.key(name, **mods)
    if "error" in answer:
        raise SystemExit(f"/key {name}: {answer}")
    time.sleep(wait)


def typed(words):
    devctl.call("/text", {"text": words})
    time.sleep(0.6)


def open_sheet():
    if not report()["open"]:
        key("F8", wait=0.2)
    devctl.until(lambda: any(n.get("r") == "report" for n in devctl.state()["shell_nodes"]))
    time.sleep(0.6)


def close_sheet():
    for _ in range(3):
        if not report()["open"]:
            return
        key("Escape")


def clear_text():
    cmd = "SuperLeft" if sys.platform == "darwin" else "ControlLeft"
    key(cmd, hold=True, wait=0.05)
    key("A", wait=0.15)
    key(cmd, release=True, wait=0.2)
    key("Backspace")


def nodes():
    return tree(devctl.state()["shell_nodes"], root="report")


def field():
    return next(n for n in nodes() if n.get("k") == "field")


def check_inside_the_sheet(sheet):
    """Every control of the sheet stands inside the sheet (the footer's
    row ran past its right edge once, inside the window)."""
    surface = next(n for n in sheet if n.get("k") == "opaque" and n["d"] == 1)
    right, bottom = surface["x"] + surface["w"], surface["y"] + surface["h"]
    return [
        f"sheet: {n['k']} [{n['x']:.0f},{n['y']:.0f} {n['w']:.0f}x{n['h']:.0f}] leaves the sheet"
        for n in sheet
        if n.get("k") in ("hit", "button")
        and n["y"] < bottom
        and (n["x"] < surface["x"] - 1 or n["x"] + n["w"] > right + 1)
    ]


def walk():
    devctl.shell(lang="en", input="pointer")
    open_sheet()
    clear_text()
    focus = devctl.state()["shell"]["focus"]
    say(focus and focus["table"] == "report" and focus["id"] == "text", f"F8 puts the keyboard on the text: {focus}")
    typed("#Lightning B")
    names = [s["name"] for s in report()["suggestions"]]
    say(names[:1] == ["Lightning Bolt"], f"#Lightning B offers Lightning Bolt first: {names}")
    key("ArrowDown")
    say(report()["chosen"] == min(1, len(names) - 1), "↓ moves in the list")
    key("ArrowUp")
    say(report()["chosen"] == 0, "↑ moves back")
    key("Enter")
    text = report()["text"]
    say(text == "[Lightning Bolt] ", f"Enter writes the name in brackets: {text!r}")
    say(report()["suggestions"] == [], "and the list is gone")
    refs = report()["refs"]
    say(refs and refs["cards"][0]["text"] == "Lightning Bolt" and refs["cards"][0]["at"] == [0, 16],
        f"the reference rides in refs: {refs}")
    box = field()
    # The span is the line's first word: the pointer on it previews it.
    for _ in range(3):
        devctl.call("/pointer", {"x": box["x"] + 40, "y": box["y"] + 20})
        if devctl.until(lambda: report()["preview"] is not None, 3):
            break
    preview = report()["preview"]
    say(preview is not None and preview.endswith("#0"), f"hovering the span previews its printing: {preview}")
    devctl.call("/pointer", {"x": 5, "y": 5})
    say(devctl.until(lambda: report()["preview"] is None, 3), "and leaving it takes the preview down")
    typed("at #Wrath")
    say(len(report()["suggestions"]) > 0, "a second reference opens the list again")
    key("Escape")
    r = report()
    say(r["open"] and r["suggestions"] == [] and r["text"].endswith("#Wrath"),
        "the first Esc puts the list away, the typing stays")
    key("Escape")
    say(not report()["open"], "the second Esc closes the sheet")
    open_sheet()
    say(report()["text"].endswith("at #Wrath"), "reopened, the draft is there")
    close_sheet()
    print("walk: " + ("ok" if ok else "FAILED"))
    return ok


def table():
    """W9b at a table (the house, offline): `#` offers the seat's own view
    and nothing else. Every name `#` offers is a card the view shows face up
    in the seat's hand or a public zone; no card of a library (the view has
    counts there) or of the house's hand is ever among them, whatever is
    typed; `@` names the house's seat."""
    devctl.shell(lang="en", input="pointer")
    presses = lambda: [c["press"] for c in devctl.controls()]
    if devctl.state()["view"] is None:
        if "Front(PlayOffline)" in presses():
            devctl.press("Front(PlayOffline)")
        devctl.until(lambda: "Play(PlayHouse)" in presses(), 30)
        devctl.press("Play(PlayHouse)")
        devctl.until(lambda: devctl.state()["view"] is not None, 90)
        time.sleep(3)
    view = devctl.state()["view"]
    say(view is not None, "a table is up")
    shown = {c["name"] for c in view["hand"]}
    for zone in ("battlefield", "stack", "looking_at", "library_tops"):
        shown |= {o["name"] for o in view[zone] if o.get("card")}
    for zones in ("graveyards", "exile", "command"):
        for per_seat in view[zones]:
            shown |= {o["name"] for o in per_seat if o.get("card")}
    open_sheet()
    clear_text()
    say(report()["at_table"], "the sheet knows it is over a table")
    offered = set()
    for letter in "abcdefghijklmnopqrstuvwxyz":
        typed(f"#{letter}")
        offered |= {s["name"] for s in report()["suggestions"]}
        clear_text()
    say(offered and offered <= shown, f"every name offered is one the view shows: {sorted(offered)} of {sorted(shown)}")
    hand = view["seats"][1]["hand_count"] if len(view["seats"]) > 1 else None
    say(report()["candidates"] <= len(view["hand"]) + sum(
        len(view[z]) for z in ("battlefield", "stack", "looking_at", "library_tops")
    ) + sum(len(p) for z in ("graveyards", "exile", "command") for p in view[z]),
        f"no more candidates than the view's own objects (the house holds {hand} unseen)")
    first = min(offered) if offered else None
    if first:
        typed(f"#{first[:3]}")
        key("Enter")
        say(report()["text"].startswith("["), f"a card taken at the table: {report()['text']!r}")
        refs = report()["refs"]
        say(refs and refs["cards"][0].get("zone") is not None and refs["cards"][0].get("object") is not None,
            f"a table's reference names its zone and object: {refs}")
        clear_text()
    players = report()["players"]
    say(len(players) >= 1, f"@ offers the other seats: {players}")
    if players:
        typed(f"@{players[0][:2]}")
        key("Enter")
        say(report()["text"] == f"[@{players[0]}] ", f"@ writes the seat as shown: {report()['text']!r}")
        clear_text()
    close_sheet()
    print("table: " + ("ok" if ok else "FAILED"))
    return ok


def dump(outdir):
    os.makedirs(outdir, exist_ok=True)
    failures = 0
    for (width, height, touch) in ((1708, 1032, False), (844, 390, True)):
        devctl.resize(width, height)
        for lang in ("en", "de"):
            devctl.shell(lang=lang, input="touch" if touch else "pointer")
            open_sheet()
            clear_text()
            typed("When I cast #Li")
            time.sleep(0.6)
            st = devctl.state()
            sheet = tree(st["shell_nodes"], root="report")
            faults = (
                check_overflow(sheet)
                + check_window(sheet, width, height)
                + check_budget(sheet, lang == "de")
                + check_hit(sheet, touch)
            )
            tag = f"{width}x{height}-{lang}-{'touch' if touch else 'pointer'}"
            faults += check_inside_the_sheet(sheet)
            if touch:
                top = next(n for n in sheet if n.get("k") == "field")["y"]
                # The chip row: the one scroller under the field (the body is the other).
                chips = [n for n in sheet if n.get("k") == "scroll" and n["y"] > top and n["h"] < 80]
                bottom = max((n["y"] + n["h"] for n in chips), default=None)
                if top >= KEYBOARD_BAND:
                    faults.append(f"phone: the field's top {top:.0f} is not above y {KEYBOARD_BAND}")
                if bottom is None or bottom >= KEYBOARD_BAND:
                    faults.append(f"phone: the chip row ends at {bottom}, not above y {KEYBOARD_BAND}")
                print(f"{tag}: field top {top:.0f}, chip row bottom {bottom}", flush=True)
            devctl.screenshot(os.path.join(outdir, f"report-{tag}.png"))
            with open(os.path.join(outdir, f"report-{tag}.json"), "w") as f:
                json.dump({"report": st["report"], "nodes": sheet}, f, indent=1)
            failures += len(faults)
            print(f"{tag}: {len(sheet)} nodes, {len(faults)} faults", flush=True)
            for fault in faults[:12]:
                print("    " + fault, flush=True)
            clear_text()
            close_sheet()
    devctl.shell(lang="en", input="auto")
    print(f"failures: {failures}")
    return failures == 0


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "walk":
        sys.exit(0 if walk() else 1)
    if len(sys.argv) >= 2 and sys.argv[1] == "table":
        sys.exit(0 if table() else 1)
    if len(sys.argv) >= 3 and sys.argv[1] == "dump":
        sys.exit(0 if dump(sys.argv[2]) else 1)
    print(__doc__)
    sys.exit(2)
