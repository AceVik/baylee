"""A lane's own live client (WP2/WP3): sign in at a local gateway and walk
to a screen, for the shots and the checks.

  BAYLEE_DEVCTL_PORT=… python3 scripts/shell/lane.py login USER PASSWORD
"""
import sys
import time

import devctl


def login(user, password, gateway_row=None):
    """From the gateway picker (or the sign-in face) to Play."""
    names = [c["press"] for c in devctl.controls()]
    if "Front(LeaveGateway)" not in names:
        # The picker: the row this lane added is the one that can be
        # forgotten (the pinned live gateway cannot), never the live one.
        mine = [n for n in names if n.startswith("Front(ForgetGateway(")]
        if gateway_row is not None:
            pick = gateway_row
        elif mine:
            pick = mine[0].replace("ForgetGateway", "SelectGateway")
        else:
            raise LookupError("no local gateway saved")
        # The front door moves in first, and a press on a moving panel is
        # answered by nothing: press again until the sign-in face is up.
        for _ in range(8):
            devctl.press(pick)
            try:
                devctl.until(
                    lambda: any(
                        c["press"] == "Shared(Focus(Username))" for c in devctl.controls()
                    ),
                    3,
                )
                break
            except TimeoutError:
                continue
        time.sleep(1.0)
    devctl.press("Shared(Focus(Username))")
    for _ in range(24):
        devctl.key("Backspace")
    devctl.call("/text", {"text": user})
    devctl.press("Shared(Focus(Password))")
    for _ in range(32):
        devctl.key("Backspace")
    devctl.call("/text", {"text": password})
    devctl.press("Front(Submit)")
    devctl.until(lambda: any(c["press"] == "Header(Nav(0))" for c in devctl.controls()), 20)
    # The entrance flies over the lobby first; presses wait for it.
    time.sleep(3.0)


if __name__ == "__main__":
    if len(sys.argv) >= 4 and sys.argv[1] == "login":
        login(sys.argv[2], sys.argv[3])
        print("signed in")
    else:
        print(__doc__)
