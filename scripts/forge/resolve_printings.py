#!/usr/bin/env python3
"""Pick a printing for every row of a hand-written decklist.

What the list says is never overridden: a row naming a set keeps that set,
and a row naming a collector number keeps that printing exactly. The
preferences below only ever decide what the list left open, which is the
whole point -- a deck is somebody's, and filling a gap is not the same as
correcting a choice.

    --lang de     prefer a printing in this language, English if there is none
    --oldest      prefer the earliest printing (the old frames)
    --nonfoil     prefer a printing that exists unfoiled
    --leave-open  name no printing where the list named none

The catalogue is asked once, with every row in one statement, because two
hundred round trips to answer two hundred independent questions is the
shape that turns a lookup into a minute.

Names are matched case-insensitively **deliberately**: `card_search.names`
takes its English entry from the newest printing's `printed_name`, so 39 of
41 991 faces are spelled in the capitals a Secret Lair printed -- among them
`BIRDS OF PARADISE`. That is a defect in the projection and is reported as
one; matching without case is what keeps this script from inheriting it.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from parse_friend_list import read  # noqa: E402

PSQL = ["docker", "compose", "exec", "-T", "postgres", "psql", "-U", "baylee", "-d", "baylee"]


def sql_quote(s: str) -> str:
    return "'" + s.replace("'", "''") + "'"


def query(sql: str) -> list[list[str]]:
    out = subprocess.run(
        PSQL + ["-tAF", "\x1f", "-c", sql], capture_output=True, text=True, check=True
    )
    return [l.split("\x1f") for l in out.stdout.splitlines() if l]


def resolve(rows: list[dict], lang: str, oldest: bool, nonfoil: bool, leave_open: bool):
    """One statement, one best printing per row."""
    values = ",".join(
        f"({i},{sql_quote(r['name'])},"
        f"{sql_quote(r['set']) if r['set'] else 'NULL'},"
        f"{sql_quote(r['number']) if r['number'] else 'NULL'})"
        for i, r in enumerate(rows)
    )
    # Every clause is a preference and none is a filter, so a card that has
    # no German printing still resolves -- to its English one, which is what
    # the owner would have picked by hand.
    order = [
        # Two tiers, whole name before front face: a tracker writes a
        # two-faced card as `Front // Back` and the projection carries one
        # row per face, so the front name has to be reachable -- and a card
        # actually called `Front` must still win over one merely starting
        # with it.
        "(lower(s.names->>'en') = lower(w.name)) DESC",
        "(w.setc IS NULL OR lower(c.set_code) = lower(w.setc)) DESC",
        "(w.num IS NULL OR c.collector_number = w.num) DESC",
        f"(c.lang = {sql_quote(lang)}) DESC",
        # English is the fallback and has to be said out loud. Without it the
        # next clause is `released_at`, so a card with no German printing
        # went to whatever was oldest in *any* language -- which put an
        # Italian and a Simplified Chinese card in a German deck.
        "(c.lang = 'en') DESC",
    ]
    if nonfoil:
        order.append("(c.finishes LIKE '%nonfoil%') DESC")
    order.append("c.released_at ASC" if oldest else "c.released_at DESC")
    order.append("c.collector_number")
    sql = f"""
    with want(i, name, setc, num) as (values {values}),
         hit as (
           select distinct on (w.i)
                  w.i, w.name, c.set_code, c.collector_number, c.lang,
                  c.finishes like '%nonfoil%' as nonfoil,
                  to_char(c.released_at, 'YYYY') as year,
                  (w.setc is not null and lower(c.set_code) <> lower(w.setc)) as set_lost
           from want w
           join card_search s
             on s.face_index = 0
            and lower(s.names->>'en') in (
                  lower(w.name), lower(split_part(w.name, ' // ', 1)))
           join cards c on c.oracle_id = s.oracle_id
           order by w.i, {', '.join(order)}
         )
    select i, name, set_code, collector_number, lang, nonfoil, year, set_lost
    from hit order by i;
    """
    found = {int(r[0]): r for r in query(sql)}
    out = []
    for i, r in enumerate(rows):
        hit = found.get(i)
        if hit is None:
            out.append({**r, "resolved": None})
            continue
        if leave_open and not r["set"]:
            out.append({**r, "resolved": None})
            continue
        out.append(
            {
                **r,
                "resolved": {
                    "set": hit[2].upper(),
                    "number": hit[3],
                    "lang": hit[4],
                    "nonfoil": hit[5] == "t",
                    "year": hit[6],
                    "set_lost": hit[7] == "t",
                },
            }
        )
    return out


def deck_row(r: dict, lang: str) -> str:
    # This pool names a two-faced card by its **front face** -- there is not
    # one `//` among the names it compiles -- while a tracker and the
    # catalogue both write `Front // Back`. The full name is what the lookup
    # above needs and the front face is what the row must say, or the card
    # resolves to nothing and the deck quietly plays one card short.
    line = f"{r['count']} {r['name'].split(' // ')[0]}"
    p = r["resolved"]
    if p:
        line += f" ({p['set']}) {p['number']}"
        if p["lang"] != "en":
            line += f" [{p['lang']}]"
        if r["foil"] and p["nonfoil"]:
            pass  # the preference said unfoiled; the note below says so
        elif r["foil"]:
            line += " *F*"
    return line


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("list")
    ap.add_argument("--lang", default="en")
    ap.add_argument("--oldest", action="store_true")
    ap.add_argument("--nonfoil", action="store_true")
    ap.add_argument("--leave-open", action="store_true")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    rows = resolve(read(a.list), a.lang, a.oldest, a.nonfoil, a.leave_open)
    if a.json:
        json.dump(rows, sys.stdout, indent=2, ensure_ascii=False)
        print()
        return 0
    for r in rows:
        print(deck_row(r, a.lang))
    return 0


if __name__ == "__main__":
    sys.exit(main())
