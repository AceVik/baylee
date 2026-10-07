"""WP1 (the shell design §3, §2.7, §17): the front door's faces at the seven
sizes, text steps 1, 4 and 5, English and German — shots, and the kit's
overflow, window, sibling, label-budget, 44-px hit and contrast checks over
the whole tree; plus principle 8: the colophon (the Fan Content notice,
Scryfall's credit, the AGPL source) present on every face at every size.

usage (a dev-control client at the front door, its gateway saved; launch
with BAYLEE_DEV_SCALE=1 for the contrast reading):
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/front.py OUTDIR "Gateway name"
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check  # noqa: E402
import devctl  # noqa: E402

TOUCH = ((1180, 820), (844, 390), (920, 443), (640, 360))
FACES = ("gateway", "signin", "create", "guest", "about")


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


def texts():
    return [n["t"] for n in devctl.state()["shell_nodes"] if n.get("t")]


def choose(gateway):
    """The gateway picker, then the gateway named `gateway`."""
    if "Front(LeaveGateway)" in presses():
        press_until("Front(LeaveGateway)", lambda: "Front(AddGateway)" in presses())
        time.sleep(1.0)
    st = devctl.state()
    names = [n for n in st["shell_nodes"] if n.get("t") == gateway]
    rows = [c for c in st["lobby_controls"] if c["press"].startswith("Front(SelectGateway")]
    for n in names:
        for r in rows:
            if abs(r["at_y"] - (n["y"] + n["h"] / 2)) < 30:
                press_until(r["press"], lambda: "Front(LeaveGateway)" in presses())
                time.sleep(1.2)
                return
    raise LookupError(gateway)


def to_face(face, gateway):
    if "Front(About(false))" in presses():
        press_until("Front(About(false))", lambda: "Front(About(false))" not in presses())
    if face == "gateway":
        if "Front(LeaveGateway)" in presses() or "Front(BackToSignIn)" in presses():
            if "Front(BackToSignIn)" in presses():
                press_until("Front(BackToSignIn)", lambda: "Front(LeaveGateway)" in presses())
            press_until("Front(LeaveGateway)", lambda: "Front(AddGateway)" in presses())
            time.sleep(1.0)
        return
    if face == "about":
        press_until("Front(About(true))", lambda: "Front(About(false))" in presses())
        return
    if "Front(AddGateway)" in presses():
        choose(gateway)
    if "Front(BackToSignIn)" in presses():
        press_until("Front(BackToSignIn)", lambda: "Front(GuestFace)" in presses())
        time.sleep(0.6)
    if face == "create":
        press_until("Front(ToggleRegistering)", lambda: "Shared(Focus(DisplayName))" in presses())
    elif face == "guest":
        press_until("Front(GuestFace)", lambda: "Shared(Focus(GuestName))" in presses())
    time.sleep(0.8)


def colophon(words):
    """Principle 8, in any language: the notice, Scryfall, the source."""
    joined = " ".join(words)
    missing = [
        what
        for what, found in (
            ("the Fan Content notice", "Fan Content" in joined or "Fan-Content" in joined),
            ("Scryfall's credit", "Scryfall" in joined),
            ("the AGPL source", "AGPL-3.0" in joined),
        )
        if not found
    ]
    return [f"colophon: {m} missing" for m in missing]


def main(out, gateway):
    os.makedirs(out, exist_ok=True)
    summary, failures = [], 0
    for width, height in check.SIZES:
        devctl.resize(width, height)
        time.sleep(1.0)
        touch = (width, height) in TOUCH
        for step in ("xs", "l", "xl"):
            for lang in ("en", "de"):
                devctl.shell(text_size=step, lang=lang, input="touch" if touch else "pointer")
                time.sleep(0.6)
                for face in FACES:
                    to_face(face, gateway)
                    devctl.settle(4)
                    st = devctl.state()
                    nodes = check.tree(st["shell_nodes"])
                    words = [n["t"] for n in nodes if n.get("t")]
                    faults = (
                        check.check_overflow(nodes)
                        + check.check_window(nodes, width, height)
                        + check.check_siblings(nodes)
                        + check.check_budget(nodes, lang == "de")
                        + check.check_hit(nodes, touch)
                        + (colophon(words) if face != "about" else [])
                    )
                    tag = f"{face}-{width}x{height}-{step}-{lang}"
                    contrast = []
                    if step == "l" and (lang == "en" or (width, height) in ((960, 700), (844, 390))):
                        png = os.path.join(out, f"front-{tag}.png")
                        devctl.screenshot(png)
                        if lang == "en":
                            size = devctl.health()
                            # Under a sheet only the sheet is read: the
                            # check sees rects, not what covers them.
                            ground = nodes
                            if face == "about":
                                ground = [
                                    dict(n, k=None) if n.get("k") in ("panel", "mist") else n
                                    for n in nodes
                                ]
                            more, contrast = check.check_contrast(
                                ground, png, size["width"], size["height"]
                            )
                            faults += more
                    failures += len(faults)
                    worst = min((c[1] for c in contrast), default=None)
                    worst_muted = min((c[2] for c in contrast if c[0] != "mist"), default=None)
                    summary.append(
                        f"{tag}: {len(nodes)} nodes, {len(faults)} faults"
                        + (f", ink >= {worst}, muted >= {worst_muted}" if contrast else "")
                    )
                    for fault in faults[:6]:
                        summary.append("    " + fault)
                    print("\n".join(summary[-1 - min(len(faults), 6):]), flush=True)
    to_face("signin", gateway)
    devctl.shell(text_size="l", lang="en", input="auto")
    with open(os.path.join(out, "front-check.log"), "w") as f:
        f.write("\n".join(summary) + f"\nfailures: {failures}\n")
    print(f"failures: {failures}")
    return failures


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    sys.exit(1 if main(sys.argv[1], sys.argv[2]) else 0)
