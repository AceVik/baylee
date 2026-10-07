"""The WP2 / WP3 counts §17 names that are not a dump, on a lane's own
signed-in client (dev-control) and its own gateway:

  hover    pointer moves over tiles, rows and buttons rebuild nothing
  presses  Play -> room -> Start: every press counted, the room's Start
           enabled, and the game started
  quit     a deletion waiting for its Undo reaches the gateway when the
           window is closed (the client is gone afterwards)

  BAYLEE_DEVCTL_PORT=… BAYLEE_GATEWAY_URL=http://127.0.0.1:… \
    python3 scripts/shell/measure.py hover|presses|quit USER PASSWORD PID
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import devctl  # noqa: E402
import screens  # noqa: E402

GATEWAY = os.environ.get("BAYLEE_GATEWAY_URL", "http://127.0.0.1:28931")


def rebuilds():
    return devctl.state()["ui_rebuilds"]["total"]


def hover():
    devctl.resize(1920, 1080)
    devctl.shell(text_size="l", lang="en", input="pointer")
    for screen in ("decks", "play"):
        screens.go(screen)
        devctl.settle()
        targets = [c for c in devctl.controls() if not c["press"].startswith("Header")]
        before = rebuilds()
        moves = 0
        for c in targets[:30]:
            devctl.call("/pointer", {"x": c["at_x"], "y": c["at_y"]})
            devctl.call("/pointer", {"x": c["at_x"] + 40, "y": c["at_y"] + 30})
            moves += 2
        time.sleep(0.5)
        after = rebuilds()
        print(f"hover {screen}: {moves} moves over {len(targets[:30])} controls, rebuilds {before} -> {after}")


def presses():
    devctl.resize(1920, 1080)
    screens.go("play")
    count = 0

    def tap(name):
        nonlocal count
        if not screens.press(name, wait=1.5):
            raise SystemExit(f"no {name}: {[c['press'] for c in devctl.controls()]}")
        count += 1
        print(f"  {count}: {name}")

    tap("Play(CreateTable)")
    tap("Play(Template(Duel))")
    tap("Play(Open)")
    devctl.until(lambda: devctl.state()["shell"]["table"] == "room", 15)
    devctl.settle()
    names = [c["press"] for c in devctl.controls()]
    ai = next(n for n in names if n.startswith("Shared(OpenMenu(SeatAi("))
    tap(ai)
    pick = next(c["press"] for c in devctl.controls() if c["press"].endswith('"steady"))'))
    tap(pick)
    devctl.settle()
    start = next(c["press"] for c in devctl.controls() if c["press"].startswith("Room(StartRoom("))
    tap(start)
    try:
        devctl.until(lambda: devctl.state()["shell"]["screen"] not in ("Room", "Hub", "Play"), 30)
    except TimeoutError:
        pass
    print(f"presses Play -> room -> Start: {count}; screen now {devctl.state()['shell']['screen']}")


def login(user, password):
    req = urllib.request.Request(
        GATEWAY + "/auth/login",
        data=json.dumps({"username": user, "password": password}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read())["token"]


def decks(token):
    req = urllib.request.Request(GATEWAY + "/decks", headers={"Authorization": f"Bearer {token}"})
    with urllib.request.urlopen(req) as r:
        body = json.loads(r.read())
    return body if isinstance(body, list) else body.get("decks", [])


def quit_flush(user, password, pid):
    token = login(user, password)
    before = [d["id"] for d in decks(token)]
    screens.go("decks")
    devctl.settle()
    # Duplicate the first tile, then delete the copy with its Undo standing.
    screens.press("Shared(OpenMenu(Deck(0)))")
    screens.press("Decks(Duplicate(0))", wait=2.5)
    devctl.settle()
    with_copy = [d["id"] for d in decks(token)]
    copy = [d for d in with_copy if d not in before]
    print(f"decks before {len(before)}, with the copy {len(with_copy)}")
    devctl.key("ArrowRight")
    names = [c["press"] for c in devctl.controls()]
    menus = [n for n in names if n.startswith("Shared(OpenMenu(Deck(")]
    # The copy is the newest: first under "last saved".
    screens.press(menus[0])
    deletes = [c["press"] for c in devctl.controls() if c["press"].startswith("Decks(Delete(")]
    screens.press(deletes[0])
    staged = [d["id"] for d in decks(token)]
    print(f"after Delete (Undo standing): the gateway still has {len(staged)}")
    # Close the window as a person does: the close button.
    subprocess.run(
        [
            "osascript",
            "-e",
            f'tell application "System Events" to tell (first process whose unix id is {pid}) '
            "to click button 1 of front window",
        ],
        check=False,
    )
    time.sleep(4)
    alive = subprocess.run(["kill", "-0", str(pid)], check=False).returncode == 0
    after = [d["id"] for d in decks(token)]
    print(f"after closing the window (client alive: {alive}): the gateway has {len(after)}; "
          f"copy {copy} gone: {all(c not in after for c in copy)}")


if __name__ == "__main__":
    what = sys.argv[1]
    if what == "hover":
        hover()
    elif what == "presses":
        presses()
    elif what == "quit":
        quit_flush(sys.argv[2], sys.argv[3], int(sys.argv[4]))
