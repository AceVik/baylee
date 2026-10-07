"""Quick looks while working (WP2/WP3): a screen at a size, photographed.

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/look.py OUTDIR play,decks 960x700,844x390 [touch] [de]
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devctl  # noqa: E402

GO = {
    "play": ["Header(Nav(0))"],
    "decks": ["Header(Nav(1))", "Decks(Tab(Mine))"],
    "house": ["Header(Nav(1))", "Decks(Tab(House))"],
    "create": ["Header(Nav(0))", "Play(CreateTable)"],
}


def press(name):
    for c in devctl.controls():
        if c["press"] == name:
            return devctl.click_at(c["at_x"], c["at_y"])
    return None


def go(screen):
    for step in GO[screen]:
        press(step)
        time.sleep(0.8)


def leave(screen):
    if screen == "create":
        press("Play(CloseSheet)")
        time.sleep(0.5)


if __name__ == "__main__":
    out, screens, sizes = sys.argv[1], sys.argv[2].split(","), sys.argv[3].split(",")
    touch = "touch" in sys.argv[4:]
    lang = "de" if "de" in sys.argv[4:] else "en"
    os.makedirs(out, exist_ok=True)
    devctl.shell(lang=lang, input="touch" if touch else "pointer")
    for size in sizes:
        w, h = (int(v) for v in size.split("x"))
        devctl.resize(w, h)
        time.sleep(0.8)
        for screen in screens:
            go(screen)
            time.sleep(0.6)
            path = os.path.join(out, f"{screen}-{w}x{h}-{lang}{'-touch' if touch else ''}.png")
            devctl.screenshot(path)
            print(path)
            leave(screen)
