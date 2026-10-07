"""The shell's failable checks, run against a live client through dev-control
(the shell design, .claude/ux-b6/DESIGN-v5.md §2.2, §2.7, §8, §17).

Every check reads `/state.shell_nodes` — each node's depth, rect, text,
`"k"` role and `"r"` root — and, for contrast, the client's own screenshot
at scale 1 (launch with BAYLEE_DEV_SCALE=1, so a logical pixel is one pixel
of the PNG). Nothing is composited: the pixels are the ones rendered.

  overflow   no text node leaves its parent's rect (1 px tolerance)
  siblings   no two sibling rows, tiles or list rows intersect
  budget     a button/chip/tab/nav label within its budget (§2.3), en or de
  hit        under Touch every hit wrapper is at least 44 x 44
  contrast   ink >= 7 : 1 and muted >= 4.5 : 1 against the brightest ground
             pixel under every panel and the header; ink >= 7 on a mist plate

usage:
  python3 scripts/shell/check.py gallery OUTDIR   # the WP0b-1 acceptance
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devctl  # noqa: E402

INK = (0.90, 0.93, 0.94)
MUTED = (0.58, 0.64, 0.68)
BUDGETS = {"button": (18, 20), "chip": (14, 14), "tab": (16, 16), "nav": (12, 13)}
SIZES = [(1920, 1080), (960, 700), (1180, 820), (844, 390), (920, 443), (640, 360), (2560, 1440)]


def tree(nodes, root=None):
    """Nodes with parent indices; only the subtree of roots named `root`."""
    out, stack, keep = [], [], False
    for node in nodes:
        depth = node["d"]
        if depth == 0:
            keep = root is None or node.get("r") == root
        if not keep:
            continue
        while stack and stack[-1][0] >= depth:
            stack.pop()
        parent = stack[-1][1] if stack else None
        node = dict(node, parent=parent, index=len(out))
        out.append(node)
        stack.append((depth, node["index"]))
    return out


def children(nodes, i):
    return [n for n in nodes if n["parent"] == i]


def descendants(nodes, i):
    found, frontier = [], [i]
    while frontier:
        here = frontier.pop()
        for n in nodes:
            if n["parent"] == here:
                found.append(n)
                frontier.append(n["index"])
    return found


def inside(child, parent, slack=1.0):
    return (
        child["x"] >= parent["x"] - slack
        and child["y"] >= parent["y"] - slack
        and child["x"] + child["w"] <= parent["x"] + parent["w"] + slack
        and child["y"] + child["h"] <= parent["y"] + parent["h"] + slack
    )


def overlap(a, b):
    w = min(a["x"] + a["w"], b["x"] + b["w"]) - max(a["x"], b["x"])
    h = min(a["y"] + a["h"], b["y"] + b["h"]) - max(a["y"], b["y"])
    return max(w, 0) * max(h, 0)


def check_overflow(nodes):
    faults = []
    for n in nodes:
        if "t" not in n or not n["t"].strip() or n["parent"] is None:
            continue
        if n["w"] == 0 and n["h"] == 0:
            continue
        parent = nodes[n["parent"]]
        if not inside(n, parent):
            faults.append(f"overflow: {n['t'][:40]!r} {box(n)} leaves {box(parent)}")
    return faults


def check_window(nodes, width, height):
    """No control (a hit wrapper) leaves the window: a pill pushed off the
    header's right end overflows nothing the overflow check can see."""
    return [
        f"window: {box(n)} leaves the {width:.0f} x {height:.0f} window"
        for n in nodes
        if n.get("k") == "hit"
        and (n["x"] < -1 or n["x"] + n["w"] > width + 1)
    ]


def check_siblings(nodes):
    faults = []
    for n in nodes:
        rows = [c for c in children(nodes, n["index"]) if c.get("k") in ("row", "tile")]
        for i, a in enumerate(rows):
            for b in rows[i + 1:]:
                if overlap(a, b) > 1.0:
                    faults.append(f"siblings: {box(a)} and {box(b)} intersect")
    return faults


NOT_LABEL = ("keycap", "count")


def label_of(nodes, i):
    """The words of a control: its text without key caps and count badges
    (the budget is the label's, §2.3; a count stands after it)."""
    words = []
    for d in descendants(nodes, i):
        if d.get("k") in NOT_LABEL:
            continue
        if any(a.get("k") in NOT_LABEL for a in ancestors(nodes, d["index"], stop=i)):
            continue
        if "t" in d and d["t"].strip():
            words.append(d["t"])
    return " ".join(words)


def ancestors(nodes, i, stop):
    found, here = [], nodes[i]["parent"]
    while here is not None and here != stop:
        found.append(nodes[here])
        here = nodes[here]["parent"]
    return found


def check_budget(nodes, german):
    faults = []
    for n in nodes:
        budget = BUDGETS.get(n.get("k"))
        if not budget:
            continue
        text = label_of(nodes, n["index"])
        limit = budget[1] if german else budget[0]
        if len(text) > limit:
            faults.append(f"budget: {n['k']} {text!r} is {len(text)} > {limit}")
    return faults


def check_hit(nodes, touch):
    if not touch:
        return []
    return [
        f"hit: {box(n)} under 44 x 44"
        for n in nodes
        if n.get("k") == "hit" and (n["w"] < 43.5 or n["h"] < 43.5)
    ]


def luminance(rgb):
    def channel(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    r, g, b = (channel(c) for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def check_contrast(nodes, png, width, height):
    from PIL import Image

    image = Image.open(png).convert("RGB")
    # A sleeping or locked display hands back a black frame, on which every
    # ratio passes: that is no measurement, so it fails.
    if max(image.convert("L").getextrema()) < 24:
        return [f"contrast: {os.path.basename(png)} is black - is the display asleep?"], []
    sx, sy = image.size[0] / width, image.size[1] / height
    pixels = image.load()
    faults, report = [], []
    for n in nodes:
        kind = n.get("k")
        if kind not in ("panel", "header", "mist", "opaque"):
            continue
        # Everything drawn on the panel is cover, not ground: text, a node
        # with a role, and every leaf (a knob, a disc, a glyph, a rule),
        # which carries neither. What is left is the panel over the painting.
        below = descendants(nodes, n["index"])
        parents = {d["parent"] for d in below}
        cover = [d for d in below if "t" in d or "k" in d or d["index"] not in parents]
        # A container's own hairline (a row's rule, a box's border, which a
        # rounded corner pulls up to 3 px in) lies on its edge: drawn, not
        # ground.
        edges = [d for d in below if d not in cover and d["w"] > 0 and d["h"] > 0]
        x0, y0 = max(n["x"], 0), max(n["y"], 0)
        x1, y1 = min(n["x"] + n["w"], width), min(n["y"] + n["h"], height)
        if x1 - x0 < 4 or y1 - y0 < 4:
            continue
        brightest, seen = None, 0
        # A panel's corners are rounded (radius 14, `px_fixed`): the square
        # corner of its rect shows the painting, not the panel.
        corner = 16
        y = y0 + 3
        while y < y1 - 3:
            x = x0 + 3
            while x < x1 - 3:
                in_corner = (x - n["x"] < corner or n["x"] + n["w"] - x < corner) and (
                    y - n["y"] < corner or n["y"] + n["h"] - y < corner
                )
                if in_corner:
                    x += 3
                    continue
                covered = any(
                    d["x"] - 2 <= x <= d["x"] + d["w"] + 2 and d["y"] - 2 <= y <= d["y"] + d["h"] + 2
                    for d in cover
                ) or any(
                    d["x"] - 2 <= x <= d["x"] + d["w"] + 2
                    and d["y"] - 2 <= y <= d["y"] + d["h"] + 2
                    and (
                        min(abs(x - d["x"]), abs(x - d["x"] - d["w"])) <= 4
                        or min(abs(y - d["y"]), abs(y - d["y"] - d["h"])) <= 4
                    )
                    for d in edges
                )
                if not covered:
                    px = pixels[int(x * sx), int(y * sy)]
                    rgb = tuple(c / 255 for c in px)
                    seen += 1
                    if brightest is None or luminance(rgb) > luminance(brightest):
                        brightest = rgb
                x += 3
            y += 3
        if brightest is None:
            continue
        ink, muted = ratio(INK, brightest), ratio(MUTED, brightest)
        report.append((kind, round(ink, 2), round(muted, 2), seen))
        if ink < 7.0:
            faults.append(f"contrast: ink {ink:.2f} on {kind} {box(n)}")
        if kind != "mist" and muted < 4.5:
            faults.append(f"contrast: muted {muted:.2f} on {kind} {box(n)}")
    return faults, report


def box(n):
    return f"[{n['x']:.0f},{n['y']:.0f} {n['w']:.0f}x{n['h']:.0f}]"


def gallery(outdir):
    """The WP0b-1 acceptance on the gallery: seven sizes, steps 1 and 5 (and 4),
    English and German, Pointer and Touch."""
    os.makedirs(outdir, exist_ok=True)
    devctl.shell(gallery=True)
    summary, failures = [], 0
    for width, height in SIZES:
        devctl.resize(width, height)
        for step in ("xs", "l", "xl"):
            for lang in ("en", "de"):
                for touch in (False, True):
                    devctl.shell(text_size=step, lang=lang, input="touch" if touch else "pointer")
                    devctl.step(4) if devctl.health().get("paused") else None
                    devctl.until(lambda: any(n.get("r") == "gallery" for n in devctl.state()["shell_nodes"]))
                    import time

                    time.sleep(0.6)
                    st = devctl.state()
                    nodes = tree(st["shell_nodes"], root="gallery")
                    faults = (
                        check_overflow(nodes)
                        + check_window(nodes, width, height)
                        + check_siblings(nodes)
                        + check_budget(nodes, lang == "de")
                        + check_hit(nodes, touch)
                    )
                    tag = f"{width}x{height}-{step}-{lang}-{'touch' if touch else 'pointer'}"
                    contrast = []
                    if step == "l" and not touch:
                        png = os.path.join(outdir, f"gallery-{tag}.png")
                        devctl.screenshot(png)
                        size = devctl.health()
                        more, contrast = check_contrast(nodes, png, size["width"], size["height"])
                        faults += more
                    with open(os.path.join(outdir, f"gallery-{tag}.json"), "w") as f:
                        json.dump(st["shell_nodes"], f)
                    failures += len(faults)
                    worst = min((c[1] for c in contrast), default=None)
                    worst_muted = min((c[2] for c in contrast if c[0] != "mist"), default=None)
                    actual = devctl.health()
                    summary.append(
                        f"{tag} (client {actual['width']:.0f}x{actual['height']:.0f}): "
                        f"{len(nodes)} nodes, {len(faults)} faults"
                        + (f", ink >= {worst}, muted >= {worst_muted}" if contrast else "")
                    )
                    for fault in faults[:8]:
                        summary.append("    " + fault)
                    print(summary[-1 - min(len(faults), 8)], flush=True)
    devctl.shell(text_size="l", lang="en", input="auto", gallery=False)
    print("\n".join(summary))
    print(f"failures: {failures}")
    return failures


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "gallery":
        sys.exit(1 if gallery(sys.argv[2]) else 0)
    print(__doc__)
    sys.exit(2)
