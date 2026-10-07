"""WP0b-3 (the shell design §2.1, §17): the header (and a seated strip when one stands) on the lobby's
signed-in screens at the seven sizes: shots, and the kit's overflow, label
budget, 44-px hit and contrast checks over the header's own subtree.

usage (a dev-control client signed in on the hub):
  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/header.py OUTDIR
"""
import json, os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check, devctl
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
SIZES = check.SIZES
fails = 0; lines = []
def subtree(nodes, kind):
    flat = check.tree(nodes)
    picked = []
    for n in flat:
        if n.get("k") == kind:
            ds = check.descendants(flat, n["index"])
            picked.append((n, ds))
    return flat, picked
def press(name):
    for c in devctl.controls():
        if c["press"] == name:
            return devctl.click_at(c["at_x"], c["at_y"])
    raise LookupError(name)
for w, h in SIZES:
    devctl.resize(w, h)
    touch = (w, h) in ((1180, 820), (844, 390), (920, 443), (640, 360))
    for lang in ("en", "de"):
        devctl.shell(lang=lang, input="touch" if touch else "pointer", text_size="l")
        for screen, how in (("play", "Header(Nav(0))"), ("decks", "Header(Nav(1))"), ("settings", "Header(Nav(2))")):
            press(how); time.sleep(1.0)
            st = devctl.state(); nodes = st["shell_nodes"]
            flat, headers = subtree(nodes, "header")
            faults = []
            for head, ds in headers:
                sub = [head] + ds
                # re-index for the checks: they read parent/index within the list
                idx = {n["index"]: i for i, n in enumerate(sub)}
                sub2 = []
                for i, n in enumerate(sub):
                    m = dict(n); m["index"] = i; m["parent"] = idx.get(n["parent"]); sub2.append(m)
                faults += check.check_overflow(sub2) + check.check_budget(sub2, lang == "de") + check.check_hit(sub2, touch)
                # the header must stay inside the window
                for n in sub2:
                    if n.get("k") == "hit" and (n["x"] + n["w"] > w + 1 or n["x"] < -1):
                        faults.append(f"outside the window: {check.box(n)}")
            tag = f"{screen}-{w}x{h}-{lang}-{'touch' if touch else 'pointer'}"
            contrast = ""
            if lang == "en" or (w, h) in ((960, 700), (844, 390)):
                png = os.path.join(out, f"{tag}.png"); devctl.screenshot(png)
                if lang == "en":
                    size = devctl.health()
                    heads = [n for n in flat if n.get("k") == "header"]
                    more, rep = check.check_contrast(check.tree(nodes), png, size["width"], size["height"])
                    hrep = [r for r in rep if r[0] == "header"]
                    faults += [f for f in more if "header" in f]
                    if hrep:
                        contrast = f", header ink >= {min(r[1] for r in hrep)} muted >= {min(r[2] for r in hrep)}"
            fails += len(faults)
            lines.append(f"{tag}: {len(headers)} header, {len(faults)} faults{contrast}")
            print(lines[-1], flush=True)
            for f in faults[:5]: print("   ", f, flush=True)
devctl.shell(lang="en", input="auto", text_size="l")
press("Header(Nav(0))")
print("failures:", fails)
sys.exit(1 if fails else 0)
