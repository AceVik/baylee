#!/usr/bin/env python3
"""Turn a Forge Adventure deck into baylee deck rows.

Two translations happen here and nothing else.

**The set code.** Forge keeps its own codes and says what Scryfall calls the
same set in each edition file's `ScryfallCode`. Over the 679 editions on this
machine the two disagree 17 times, and our two decks use exactly one of them
(`MPS_KLD` is Scryfall's `MPS`). The map is *derived on every run* and never
committed: those files are Forge's, read the way the card scripts are read,
and a copy in this repo would be both a licence problem and a second truth.

**The row.** `CardDb.CardRequest.compose` writes
`<count> <name>[+]|<SET>|[<collector number>]`; `baylee_core::deckrow` writes
`<count> <name> (<SET>) <number> *F*`. Same three facts, different spelling.

What does **not** survive is a language, because Forge never wrote one: it
keeps `UI_CARD_DOWNLOAD_LANG` as one global preference rather than per card.
A row therefore names no language and reads as English, which is what the
save meant. Inventing a `[de]` here would be a claim the source never made.

Usage:
    to_deck_rows.py <save.sav> <deck name> [<deck name> ...] --out <dir>
"""

from __future__ import annotations

import io
import os
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from read_adventure_save import decks, parse_row  # noqa: E402

EDITIONS = os.environ.get(
    "BAYLEE_FORGE_EDITIONS", "/Users/viktor/Projects/forge/forge-gui/res/editions"
)


def scryfall_codes(root: str) -> dict[str, str]:
    """Every Forge set code and what Scryfall calls the same set.

    Read from the edition headers, which is the only place the pair is
    stated. A set with no `ScryfallCode` is one where the two agree.
    """
    out: dict[str, str] = {}
    names = [n for n in os.listdir(root) if n.endswith(".txt")]
    if len(names) < 100:
        raise SystemExit(
            f"{root} holds {len(names)} edition files, which is not a Forge "
            "editions directory -- set BAYLEE_FORGE_EDITIONS"
        )
    for name in names:
        meta = {}
        with io.open(os.path.join(root, name), encoding="utf-8") as fh:
            for line in fh:
                if line.startswith("[cards]"):
                    break
                if "=" in line:
                    k, _, v = line.strip().partition("=")
                    meta[k] = v
        if "Code" in meta:
            out[meta["Code"]] = meta.get("ScryfallCode") or meta["Code"]
    return out


def row(request: str, codes: dict[str, str]) -> str:
    """One composed Forge card request, as a `deckrow` line."""
    p = parse_row(request)
    setc = codes.get(p["set"], p["set"]).upper()
    line = f"{p['count']} {p['name']}"
    if setc:
        line += f" ({setc})"
        if p["number"]:
            line += f" {p['number']}"
    if p["foil"]:
        line += " *F*"
    return line


def main(argv: list[str]) -> int:
    if "--out" not in argv or len(argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    out_dir = argv[argv.index("--out") + 1]
    wanted = argv[2 : argv.index("--out")]
    raw = zlib.decompress(io.open(argv[1], "rb").read())
    codes = scryfall_codes(EDITIONS)
    found = {d["name"]: d for d in decks(raw)}
    missing = [w for w in wanted if w not in found]
    if missing:
        raise SystemExit(f"no such deck in this save: {', '.join(missing)}")
    os.makedirs(out_dir, exist_ok=True)
    for name in wanted:
        d = found[name]
        lines = [f"[deck:{name}]"]
        lines += [row(r, codes) for r in d["main"]]
        if d["sideboard"]:
            lines += ["", "[sideboard]"] + [row(r, codes) for r in d["sideboard"]]
        if d["commanders"]:
            lines += ["", "[commander]"] + [row(r, codes) for r in d["commanders"]]
        slug = name.lower().replace(" ", "-")
        path = os.path.join(out_dir, f"{slug}.txt")
        io.open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")
        total = sum(parse_row(r)["count"] for r in d["main"])
        print(f"{path}: {total} main, {len(d['sideboard'])} sideboard rows")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
