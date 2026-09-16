#!/usr/bin/env python3
"""Remember every card file this session wrote.

A `PostToolUse` hook, and deliberately a silent one: the card batches run
one agent per card under a prompt that says "touch exactly one file", so a
hook that answered them with "now go and write a test" would be arguing with
the instructions they were given. The driver is who owes the test, and
`require-card-tests.py` is where it is asked for.

It records rather than recomputes because a card written and committed inside
one session would otherwise fall out of `git status` and be forgotten. What
it writes is only a list of paths; whether the debt is still owed is decided
fresh at the other end.
"""

import json
import os
import pathlib
import sys

CARDS = "crates/baylee-cards/src/cards/"


def main() -> None:
    event = json.load(sys.stdin)
    if event.get("tool_name") not in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
        return
    path = (event.get("tool_input") or {}).get("file_path") or ""
    if CARDS not in path or not path.endswith(".rs") or path.endswith("mod.rs"):
        return

    root = pathlib.Path(
        os.environ.get("CLAUDE_PROJECT_DIR") or event.get("cwd") or "."
    )
    rel = path.split(CARDS, 1)[1]
    debt = root / ".claude" / "card-test-debt"
    debt.parent.mkdir(parents=True, exist_ok=True)
    seen = set(debt.read_text().split()) if debt.exists() else set()
    if rel not in seen:
        with debt.open("a") as f:
            f.write(rel + "\n")


if __name__ == "__main__":
    try:
        main()
    except Exception:
        # A bookkeeping hook must never be the reason an edit reports failure.
        pass
    sys.exit(0)
