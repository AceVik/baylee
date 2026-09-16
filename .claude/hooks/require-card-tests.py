#!/usr/bin/env python3
"""A card this session added, fixed or refactored is played once in a test.

A `Stop` hook. It reads the list `card-test-debt.py` kept, throws away every
card that is still a stub (a refusal owes nothing — the file is byte-identical
to the one codegen wrote), and asks of each of the rest one question: does any
engine test name its `oracle_id`?

**Why the oracle id and not the card's name.** `card_tests.rs` addresses a
card as `card_index("<oracle_id>")`, so the id is the handle a test actually
holds; a name would match a doc comment and count a card as played because
somebody mentioned it. 148 ids appear across the engine's test modules today,
which is what makes the question answerable at all.

**Why not a test module in the card file.** CLAUDE.md is explicit that a card
file carries none: "a card file is data, and a test that reads the literal it
sits under proves nothing". The thing worth proving is that the rules engine
does what the printed sentence says, and that can only be shown by playing it
— `crates/baylee-engine/src/engine/card_tests.rs` is the home, on the shared
`testkit`.

**And a fix owes a second test.** A card that was wrong was wrong because a
rule was wrong, and the rule is where hundreds of other cards live. The
regression test belongs beside the rule — `keyword_tests`, `layers_tests`,
`abilities`, wherever the defect was — and has to be the test that fails
against the old code. Writing only the card's own scenario fixes one card and
leaves the next fifty to be found by a player.

It blocks rather than warns, because a reminder that costs nothing is one
that gets read and stepped over. `stop_hook_active` keeps it from looping: a
stop that a hook already blocked is allowed through, so the debt is stated
once per turn and never traps the session.
"""

import json
import os
import pathlib
import re
import sys

STUB = "// GENERATED STUB"
ORACLE = re.compile(r'^//! .*Oracle ID: ([0-9a-f-]{36})', re.M)
FALLBACK = re.compile(r'oracle_id\s*=\s*"([0-9a-f-]{36})"')


def main() -> int:
    event = json.load(sys.stdin)
    if event.get("stop_hook_active"):
        return 0

    root = pathlib.Path(os.environ.get("CLAUDE_PROJECT_DIR") or ".")
    debt = root / ".claude" / "card-test-debt"
    if not debt.exists():
        return 0
    touched = [line for line in debt.read_text().split("\n") if line.strip()]
    if not touched:
        return 0

    tests = ""
    for p in (root / "crates/baylee-engine/src/engine").glob("*_tests.rs"):
        tests += p.read_text()

    owed = []
    for rel in touched:
        path = root / "crates/baylee-cards/src/cards" / rel
        if not path.exists():
            continue
        text = path.read_text()
        if STUB in text:
            continue  # a refusal restored its stub and owes nothing
        m = ORACLE.search(text) or FALLBACK.search(text)
        if not m:
            continue
        if m.group(1) not in tests:
            name = text.split("—")[0].removeprefix("//! ").strip()
            owed.append((name or rel, rel, m.group(1)))

    if not owed:
        debt.unlink(missing_ok=True)
        return 0

    lines = [
        f"{len(owed)} card(s) written this session are not played by any engine test.",
        "",
        "A card file carries no test module (CLAUDE.md: it is data, and a test that",
        "reads the literal it sits under proves nothing). What is owed is a scenario in",
        "crates/baylee-engine/src/engine/card_tests.rs on the shared testkit, addressing",
        'the card as card_index("<oracle id>") — small, and carrying only the card\'s own',
        "printed text as the situation:",
        "",
    ]
    for name, rel, oid in owed[:25]:
        lines.append(f'  {name}\n      {rel}\n      card_index("{oid}")')
    if len(owed) > 25:
        lines.append(f"  … and {len(owed) - 25} more (full list: .claude/card-test-debt)")
    lines += [
        "",
        "If any of these was a FIX rather than a new card, it owes a second test, and",
        "that one does not go in card_tests.rs: the card was wrong because a rule was",
        "wrong, so the regression test belongs beside the rule it broke and must be a",
        "test that fails against the old code. Strike both halves.",
        "",
        "Clear the list with `rm .claude/card-test-debt` once the tests are written, or",
        "deliberately, if a card genuinely plays nothing an engine can observe — say so.",
    ]
    print("\n".join(lines), file=sys.stderr)
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # never wedge a session on a bookkeeping hook
        print(f"require-card-tests hook failed: {exc}", file=sys.stderr)
        sys.exit(0)
