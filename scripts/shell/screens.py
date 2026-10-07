"""The WP2 / WP3 acceptance dumps (.claude/ux-b6/DESIGN-v5.md §17), through
dev-control on a lane's own signed-in client (`lane.py`), launched with
BAYLEE_DEV_SCALE=1 so a logical pixel is a pixel of the PNG.

For each of the seven sizes, English and German, text steps 1 and 5
(`xs`, `xl`), Pointer (and Touch where the size is a touch device's), on
Play, Decks, House decks, the Create-table sheet and the room: check.py's
overflow / window / siblings / budget / 44 checks over every lobby node,
and at step 5 a shot with the contrast check. Beside them the counts §17
names:

  tiles      deck tiles per row on Decks (3 at 1180, 4 at 1920)
  phone      Decks at 844 x 390: two tiles with their action rows inside
             the window; Play: >= 3 table rows at 844 x 390, >= 2 at 640 x 360
  credit     the art credit's muted ink >= 4.5 : 1 on the ground around it

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/screens.py OUTDIR [room|lobby|all]

`room` measures the room the client is seated in (start in one); `lobby`
the rest (start unseated). Shots are OUTDIR/<screen>-<w>x<h>-<lang>-<input>.png.
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check  # noqa: E402
import devctl  # noqa: E402

TOUCH_SIZES = {(1180, 820), (844, 390), (920, 443), (640, 360)}
STEPS = ("xs", "xl")


def press(name, wait=0.7):
    for c in devctl.controls():
        if c["press"] == name:
            devctl.click_at(c["at_x"], c["at_y"])
            time.sleep(wait)
            return True
    return False


def names():
    return [c["press"] for c in devctl.controls()]


def go(screen):
    if screen == "play":
        press("Header(Nav(0))")
    elif screen == "decks":
        press("Header(Nav(1))")
        press("Decks(Tab(Mine))")
    elif screen == "house":
        press("Header(Nav(1))")
        press("Decks(Tab(House))")
    elif screen == "create":
        press("Header(Nav(0))")
        press("Play(CreateTable)")
    devctl.settle()


def leave(screen):
    if screen == "create":
        # Esc closes the sheet wherever its Cancel stands (a phone's footer
        # may be under the fold).
        devctl.key("Escape")
        time.sleep(0.5)


def rows_of(nodes, kind):
    return [n for n in nodes if n.get("k") == kind]


def tiles_per_row(nodes):
    tiles = rows_of(nodes, "tile")
    if not tiles:
        return 0
    top = min(t["y"] for t in tiles)
    return sum(1 for t in tiles if abs(t["y"] - top) < 2)


def visible(n, height):
    return n["y"] >= 0 and n["y"] + n["h"] <= height + 1


def phone_counts(screen, nodes, height):
    """What the phone needs to show without scrolling."""
    if screen == "decks":
        # A tile counts when its last hit (the action row's ⋯) is inside.
        whole = 0
        for t in rows_of(nodes, "tile"):
            hits = [d for d in check.descendants(nodes, t["index"]) if d.get("k") == "hit"]
            if hits and visible(max(hits, key=lambda d: d["y"] + d["h"]), height):
                whole += 1
        return {"tiles_whole": whole}
    if screen == "play":
        rows = [
            r
            for r in rows_of(nodes, "row")
            if any(d.get("t") for d in check.descendants(nodes, r["index"]))
            and r["w"] > 200
        ]
        return {"rows_visible": sum(1 for r in rows if visible(r, height))}
    return {}


def credit_contrast(nodes, png):
    """Muted ink against the brightest pixel of the ground each credit line
    stands on: its plate (the line's parent), outside the line's own box
    (a glyph is not ground)."""
    from PIL import Image

    image = Image.open(png).convert("RGB")
    px = image.load()
    worst = None
    for n in rows_of(nodes, "credit"):
        if n["w"] < 4 or n["parent"] is None:
            continue
        plate = nodes[n["parent"]]
        # A credit scrolled out of its panel is clipped, not shown: the
        # pixels there are whatever lies under the panel.
        panel = next(
            (a for a in check.ancestors(nodes, n["index"], stop=None) if a.get("k") == "panel"),
            None,
        )
        if panel is not None and not check.inside(plate, panel, slack=-8.0):
            continue
        brightest = None
        for x in range(int(plate["x"]) + 1, int(plate["x"] + plate["w"]) - 1):
            if n["x"] - 3 <= x <= n["x"] + n["w"] + 3:
                continue
            for y in range(int(plate["y"]) + 2, int(plate["y"] + plate["h"]) - 2):
                if not (0 <= x < image.size[0] and 0 <= y < image.size[1]):
                    continue
                rgb = tuple(c / 255 for c in px[x, y])
                if brightest is None or check.luminance(rgb) > check.luminance(brightest):
                    brightest = rgb
        if brightest is None:
            continue
        r = check.ratio(check.MUTED, brightest)
        worst = r if worst is None else min(worst, r)
    return worst


def measure(screen, out, width, height, lang, step, touch, summary):
    nodes = check.tree(devctl.state()["shell_nodes"])
    faults = (
        check.check_overflow(nodes)
        + check.check_window(nodes, width, height)
        + check.check_siblings(nodes)
        + check.check_budget(nodes, lang == "de")
        + check.check_hit(nodes, touch)
    )
    tag = f"{screen}-{width}x{height}-{lang}-{'touch' if touch else 'pointer'}-{step}"
    extra = {}
    if screen in ("decks", "house"):
        extra["tiles_per_row"] = tiles_per_row(nodes)
    if (width, height) in ((844, 390), (640, 360)):
        extra.update(phone_counts(screen, nodes, height))
    if step == "xl":
        png = os.path.join(out, f"{screen}-{width}x{height}-{lang}-{'touch' if touch else 'pointer'}.png")
        devctl.screenshot(png)
        more, contrast = check.check_contrast(nodes, png, width, height)
        if screen == "create":
            # The sheet's scrim (and on a phone the sheet) stands over the
            # header, which is not read while it is up.
            more = [f for f in more if " on header " not in f]
            contrast = [c for c in contrast if c[0] != "header"]
        faults += more
        if contrast:
            extra["ink_min"] = min(c[1] for c in contrast)
            muted = [c[2] for c in contrast if c[0] != "mist"]
            if muted:
                extra["muted_min"] = min(muted)
        credit = credit_contrast(nodes, png)
        if credit is not None:
            extra["credit_min"] = round(credit, 2)
            if credit < 4.5:
                faults.append(f"credit: {credit:.2f} < 4.5")
    line = f"{tag}: {len(nodes)} nodes, {len(faults)} faults {json.dumps(extra)}"
    summary.append(line)
    for f in faults[:6]:
        summary.append("    " + f)
    print("\n".join(summary[-1 - min(len(faults), 6):]), flush=True)
    return len(faults)


def sweep(out, screens):
    os.makedirs(out, exist_ok=True)
    summary, failures = [], 0
    for width, height in check.SIZES:
        devctl.resize(width, height)
        time.sleep(0.8)
        inputs = (False, True) if (width, height) in TOUCH_SIZES else (False,)
        for lang in ("en", "de"):
            for touch in inputs:
                for step in STEPS:
                    devctl.shell(text_size=step, lang=lang, input="touch" if touch else "pointer")
                    for screen in screens:
                        if screen != "room":
                            go(screen)
                        devctl.settle()
                        failures += measure(screen, out, width, height, lang, step, touch, summary)
                        leave(screen)
    devctl.shell(text_size="l", lang="en", input="auto")
    with open(os.path.join(out, "screens-summary.txt"), "a") as f:
        f.write("\n".join(summary) + f"\nfailures: {failures}\n")
    print(f"failures: {failures}")
    return failures


if __name__ == "__main__":
    out = sys.argv[1]
    which = sys.argv[2] if len(sys.argv) > 2 else "lobby"
    screens = {"room": ["room"], "lobby": ["play", "decks", "house", "create"]}.get(
        which, ["play", "decks", "house", "create"]
    )
    sys.exit(1 if sweep(out, screens) else 0)
