"""Key-only walkthroughs on Play, Decks and the room (KEYBOARD.md §8: W2 from
step 2, W3, W6, W7), through dev-control: `/key` and `/text` only — the
runner has no pointer. Each step prints where focus stands; each walk ends
on a `/state` assertion (its *End:* line). Start signed in on Play,
unseated, with a deck and at least one open table named "Thursday …" on
the list (the lane's seeded gateway).

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/walks.py
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devctl  # noqa: E402

CMD = "SuperLeft" if sys.platform == "darwin" else "ControlLeft"
ok = True


def say(cond, what):
    global ok
    ok &= bool(cond)
    print(("PASS " if cond else "FAIL ") + what, flush=True)


def key(name, wait=0.35, **mods):
    body = dict(name=name, **mods)
    # A key that types a character says which (dev-control sends no logical
    # character for a bare physical name).
    if name == "Slash":
        body["char"] = "/"
    answer = devctl.call("/key", body)
    if "error" in answer:
        raise SystemExit(f"/key {name}: {answer}")
    time.sleep(wait)


def chord(mod, name):
    key(mod, hold=True, wait=0.05)
    key(name, wait=0.15)
    key(mod, release=True, wait=0.4)


def text(words):
    devctl.call("/text", {"text": words})
    time.sleep(0.4)


def shell():
    return devctl.state()["shell"]


def focus():
    f = shell()["focus"]
    return None if f is None else (f["table"], f["id"], f["item"])


def where(label):
    s = shell()
    print(f"    {label}: screen={s['screen']} table={s['table']} focus={s['focus']} typing={s['typing']}")


def texts():
    return [n["t"] for n in devctl.state()["shell_nodes"] if n.get("t")]


def presses():
    return [c["press"] for c in devctl.controls()]


def w6():
    print("W6 · Use a deck for the next game")
    key("Digit2", wait=0.8)
    where("Decks")
    say(focus() and focus()[:2] == ("decks", "tiles"), "Decks opens on a tile")
    key("ArrowRight")
    where("→")
    uses = {p for p in presses() if p.startswith("Decks(Use(")}
    key("Enter", wait=0.8)
    where("Enter")
    now = {p for p in presses() if p.startswith("Decks(Use(")}
    say(uses != now and len(now) == len(uses), f"the next game moved ({sorted(uses - now)} is it now)")
    key("Digit1", wait=0.8)
    where("Play")
    say(shell()["table"] == "play", "End: Play, the hero shows it")


def w7():
    print("W7 · Delete a deck and undo")
    key("Digit2", wait=0.8)
    before = sum(1 for p in presses() if p.startswith("Decks(Use("))
    key("ArrowRight")
    key("Backspace" if sys.platform == "darwin" else "Delete", wait=0.8)
    where("Delete")
    after = sum(1 for p in presses() if p.startswith("Decks(Use("))
    say(after == before - 1, f"the tile is gone ({before} -> {after})")
    say("Decks(Undo)" in presses(), "the Undo toast stands")
    chord(CMD, "KeyZ")
    where("Undo")
    back = sum(1 for p in presses() if p.startswith("Decks(Use("))
    say(back == before, f"restored ({back})")


def w3():
    print("W3 · Join a table from the list")
    key("Digit2", wait=0.8)
    key("Digit1", wait=0.8)
    where("Play")
    say(focus() and focus()[:2] == ("play", "tables"), "Play opens on the tables")
    key("Slash")
    where("/")
    say(shell()["typing"], "the search has the caret")
    text("thurs")
    time.sleep(1.5)
    key("ArrowDown")
    where("↓")
    say(focus() and focus()[:2] == ("play", "tables"), "↓ to the first match")
    key("Enter", wait=2.5)
    where("Enter")
    s = shell()
    say(s["screen"] == "Room" or s["table"] == "room", "in the room")


def w2():
    print("W2 · Create table → set clocks → Start (from step 2)")
    key("Digit1", wait=0.8)
    key("KeyC", wait=0.8)
    where("c")
    say(shell()["table"] == "create-table", "the Create-table sheet")
    say(shell()["typing"], "the name has the caret")
    for _ in range(20):
        key("Backspace", wait=0.03)
    text("Thursday walk")
    key("Tab")
    where("Tab: players")
    key("Tab")
    where("Tab: templates")
    key("ArrowRight")
    where("→ Duel")
    for i in range(6):
        key("Tab")
        f = focus()
        if f and f[1] == "clock":
            break
    where("Tab… clock")
    say(focus() and focus()[1] == "clock", "the clock")
    key("ArrowRight")
    where("→ standard")
    key("Enter", wait=2.5)
    where("Enter")
    s = shell()
    say(s["table"] == "room", "the room")
    return s


if __name__ == "__main__":
    which = sys.argv[1:] or ["w6", "w7", "w3"]
    for w in which:
        globals()[w]()
    print("ok" if ok else "FAILED")
    sys.exit(0 if ok else 1)
