"""One screen at one size, its faults with the words of each offender:

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/probe.py play 844x390 de touch xl [shot.png]
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check  # noqa: E402
import devctl  # noqa: E402
import screens  # noqa: E402

screen, size, lang, inp, step = sys.argv[1:6]
w, h = (int(v) for v in size.split("x"))
devctl.resize(w, h)
devctl.shell(text_size=step, lang=lang, input=inp)
time.sleep(0.5)
if screen != "room":
    screens.go(screen)
devctl.settle()
nodes = check.tree(devctl.state()["shell_nodes"])
faults = (
    check.check_overflow(nodes)
    + check.check_window(nodes, w, h)
    + check.check_siblings(nodes)
    + check.check_budget(nodes, lang == "de")
    + check.check_hit(nodes, inp == "touch")
)
for n in nodes:
    if n.get("k") == "hit" and (n["x"] < -1 or n["x"] + n["w"] > w + 1):
        chain, here = [], n["parent"]
        while here is not None:
            p = nodes[here]
            chain.append(f"{p.get('k') or '-'}[{p['x']:.0f},{p['y']:.0f} {p['w']:.0f}x{p['h']:.0f}]")
            here = p["parent"]
        print("window:", repr(check.label_of(nodes, n["index"])), check.box(n), " <- ", " ".join(chain[:6]))
for f in faults:
    if not f.startswith("window"):
        print(f)
if len(sys.argv) > 6:
    devctl.screenshot(sys.argv[6])
screens.leave(screen)
