"""The shell keyboard's live acceptance on the gallery (WP0b-2; KEYBOARD.md §9),
through dev-control: the Tab walk read against the gallery's own TabOrder table
(GALLERY_ORDER in shellkit/gallery.rs), Shift+Tab, the ring after a key and not
after a click (§9.12), the ? overlay as a modal that cycles and holds the
printable keys, Esc, and Ctrl/Cmd + = and 0 stepping the text size. Keys only,
except the one click §9.12 needs.

usage:
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/keys.py OUTDIR
"""
import json, os, re, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
import devctl, check
from PIL import Image
_gallery = open(os.path.join(HERE, "..", "..", "crates", "baylee-client", "src", "shellkit", "gallery.rs")).read()
_block = _gallery[_gallery.index("stops: &["):_gallery.index("],", _gallery.index("stops: &["))]
ORDER = re.findall(r'"([a-z0-9-]+)"', _block)
CMD = "SuperLeft" if sys.platform == "darwin" else "ControlLeft"
ok = True
def say(cond, what):
    global ok
    ok &= bool(cond); print(("PASS " if cond else "FAIL ") + what, flush=True)
def shell(): return devctl.state()["shell"]
def chord(mod, name, char=None):
    devctl.key(mod, hold=True)
    body = {"name": name}
    if char: body["char"] = char
    devctl.call("/key", body); time.sleep(0.15)
    devctl.key(mod, release=True); time.sleep(0.25)
def press(name, char=None):
    body = {"name": name}
    if char: body["char"] = char
    devctl.call("/key", body); time.sleep(0.2)
def ring_pixels(png, n):
    img = Image.open(png).convert("RGB"); px = img.load(); hits = 0
    for x in range(int(n["x"]) - 4, int(n["x"] + n["w"]) + 4):
        for y in (int(n["y"]) - 3, int(n["y"] + n["h"]) + 2):
            if 0 <= x < img.size[0] and 0 <= y < img.size[1]:
                r, g, b = px[x, y]
                if g > 150 and b > 140 and r < 130: hits += 1
    return hits
for w, h in [(1920, 1080), (844, 390)]:
    devctl.resize(w, h); devctl.shell(gallery=True, text_size="l", lang="en", input="pointer"); time.sleep(1.0)
    walked = []
    for _ in ORDER:
        press("Tab"); walked.append((shell()["focus"] or {}).get("id"))
    say(walked == ORDER, f"{w}x{h} Tab walks GALLERY_ORDER ({len(walked)} stops)" + ("" if walked == ORDER else f": {walked}"))
    press("Tab", None); press("Tab"); devctl.call("/key", {"name": "Tab"}); time.sleep(0.2)
    devctl.key("ShiftLeft", hold=True); devctl.call("/key", {"name": "Tab"}); time.sleep(0.2); devctl.key("ShiftLeft", release=True); time.sleep(0.2)
    st = devctl.state(); f = st["shell"]["focus"]
    say(f and f["id"] == ORDER[1], f"{w}x{h} Shift+Tab steps back ({f})")
    say(st["shell"]["focus_visible"], f"{w}x{h} ring visible after a key")
    # the focused control's rect: the hit wrapper with that stop is not in the dump by id, so find by position via a fresh Tab to 'create'
    while (shell()["focus"] or {}).get("id") != "create": press("Tab")
    nodes = check.tree(devctl.state()["shell_nodes"], root="gallery")
    hits = [n for n in nodes if n.get("k") == "hit"]
    # 'create' is the first button hit wrapper in the buttons panel: the first hit after the header's
    btns = [n for n in hits if n["y"] > 60]
    create = btns[0]
    png = os.path.join(out, f"ring-key-{w}x{h}.png"); devctl.screenshot(png)
    by_key = ring_pixels(png, create)
    devctl.click_at(create["x"] + create["w"] / 2, create["y"] + create["h"] / 2); time.sleep(0.6)
    png2 = os.path.join(out, f"ring-click-{w}x{h}.png"); devctl.screenshot(png2)
    by_click = ring_pixels(png2, create)
    say(by_key > 20 and by_click < 5 and not shell()["focus_visible"], f"{w}x{h} ring after Tab ({by_key} accent px), none after a click ({by_click})")
    # ? overlay
    chord(CMD, "Slash", "/")
    s = shell()
    say(s["overlay"] and s["table"] == "overlay" and (s["focus"] or {}).get("id") == "search", f"{w}x{h} Ctrl/Cmd+/ opens the overlay, focus in its search ({s['focus']})")
    devctl.screenshot(os.path.join(out, f"overlay-{w}x{h}.png"))
    cyc = []
    for _ in range(4):
        press("Tab"); cyc.append((shell()["focus"] or {}).get("id"))
    say(cyc == ["list", "edit-keys", "close", "search"], f"{w}x{h} Tab cycles inside the overlay {cyc}")
    before = list(shell()["fired"])
    for ch, name in [("c", "KeyC"), ("1", "Digit1"), ("/", "Slash"), ("n", "KeyN")]:
        press(name, ch)
    say(shell()["fired"] == before, f"{w}x{h} c 1 / n typed into the overlay search fire nothing")
    press("Escape"); press("Escape")
    say(not shell()["overlay"], f"{w}x{h} Esc clears, then closes")
    step0 = shell()["text_size"]
    chord(CMD, "Equal", "=")
    step1 = shell()["text_size"]
    chord(CMD, "Digit0", "0")
    step2 = shell()["text_size"]
    say((step0, step1, step2) == (4, 5, 4), f"{w}x{h} Ctrl/Cmd+= then Ctrl/Cmd+0: steps {step0} -> {step1} -> {step2}")
devctl.shell(gallery=False, text_size="l", lang="en", input="auto")
print("RESULT", "PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
