#!/usr/bin/env python3
"""Read a hand-written decklist of the `1x Name (SET:NUM)` shape.

The form friends export from paper collection trackers, and it is not the
one `baylee_core::deckrow` reads: the count carries an `x`, the printing is
`(SET)` or `(SET:NUMBER)` rather than two groups, and a trailing bare `f`
means foil where deckrow writes `*F*`. `(000)` is what those trackers put
when they have no set at all, so it is read as "no printing named" rather
than as a set called `000`.

Everything the list does not say is left unsaid here and decided later,
against the catalogue, by whoever knows what the deck's owner likes.
"""

from __future__ import annotations

import re
import sys

LINE = re.compile(
    r"^(?P<count>\d+)x\s+"
    r"(?P<name>.+?)"
    r"(?:\s+\((?P<set>[^):]+)(?::(?P<number>[^)]+))?\))?"
    r"(?:\s+(?P<foil>f))?$"
)

# What a tracker writes when it knows no set. Not a set code: the catalogue
# has no such set, and reading it as one would send every such card down the
# "printing not found" path instead of the "pick one" path.
NO_SET = {"000"}


def parse(line: str) -> dict | None:
    line = line.strip()
    if not line or line.startswith("#"):
        return None
    m = LINE.match(line)
    if not m:
        raise SystemExit(f"not a decklist line: {line!r}")
    g = m.groupdict()
    setc = g["set"]
    if setc in NO_SET:
        setc = None
    # A two-faced card is `Front // Back` everywhere Scryfall is involved,
    # and trackers write it with one slash. Normalised here rather than at
    # the lookup, so every reader downstream sees one spelling.
    name = g["name"].strip().replace(" / ", " // ")
    return {
        "count": int(g["count"]),
        "name": name,
        "set": setc.upper() if setc else None,
        "number": g["number"],
        "foil": bool(g["foil"]),
    }


def read(path: str) -> list[dict]:
    out = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            row = parse(line)
            if row:
                out.append(row)
    return out


if __name__ == "__main__":
    rows = read(sys.argv[1])
    named = sum(1 for r in rows if r["set"])
    print(f"{len(rows)} rows, {sum(r['count'] for r in rows)} cards, "
          f"{named} name a set, {sum(1 for r in rows if r['number'])} a number, "
          f"{sum(1 for r in rows if r['foil'])} foil")
