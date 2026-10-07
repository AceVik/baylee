"""A small driver for the client's dev-control harness (docs/client.md
§"Driving the client without its window"), for the shell's checks.

The port is BAYLEE_DEVCTL_PORT (default 28770, the documented one). Every
call returns the parsed JSON answer; a refusal is an answer with "error".
"""
import json
import os
import time
import urllib.request

BASE = f"http://127.0.0.1:{os.environ.get('BAYLEE_DEVCTL_PORT', '28770')}"


def call(path, body=None, timeout=60):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(
        BASE + path, data=data, method="GET" if body is None else "POST"
    )
    with urllib.request.urlopen(request, timeout=timeout) as answer:
        raw = answer.read().decode()
    try:
        return json.loads(raw)
    except ValueError:
        return raw


def state():
    return call("/state")


def health():
    return call("/health")


def controls():
    return state()["lobby_controls"]


def click_at(x, y):
    return call("/pointer", {"x": x, "y": y, "button": "left", "press": True})


def press(name):
    """Clicks the lobby control whose `Press` prints as `name`."""
    for control in controls():
        if control["press"] == name:
            return click_at(control["at_x"], control["at_y"])
    raise LookupError(f"no control {name!r}")


def key(name, **mods):
    return call("/key", dict(name=name, **mods))


def resize(width, height):
    answer = call("/window", {"width": width, "height": height})
    until(lambda: (health()["width"], health()["height"]) == (width, height), 10)
    return answer


def shell(**fields):
    """Sets the shell's knobs: text_size (xs s m l xl), lang (en de),
    input (touch pointer auto), gallery (true false)."""
    return call("/shell", {k: str(v).lower() for k, v in fields.items()})


def screenshot(path):
    return call("/screenshot", {"path": os.path.abspath(path)}, timeout=120)


def pause(paused=True):
    return call("/pause", {"paused": paused})


def step(frames):
    return call("/step", {"frames": frames})


def hide(what, hidden=True):
    return call("/hide", {"what": what, "hidden": hidden})


def until(predicate, timeout=20, every=0.2):
    end = time.time() + timeout
    while time.time() < end:
        try:
            if predicate():
                return True
        except (KeyError, IndexError, ValueError):
            pass
        time.sleep(every)
    raise TimeoutError("condition not met")


def settle(timeout=8):
    """Waits until the lobby tree has not been rebuilt for a second."""
    end = time.time() + timeout
    last, quiet = state().get("ui_rebuilds"), 0
    while time.time() < end and quiet < 4:
        time.sleep(0.25)
        now = state().get("ui_rebuilds")
        quiet = quiet + 1 if now == last else 0
        last = now
