"""Import unmodified, fully supported tournament lists from a pinned bulk archive.

Source: fbettega/MTG_decklistcache (GPL-3.0). Read only MTGO tournament JSON;
retain card counts and source provenance, never player names/results as labels.
The archive remains external to the code repository. No upstream scraping.
Historical format labels do not certify legality under today's ban lists.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import tarfile

import orjson

from run_guard4 import atomic_json

REPO = "https://github.com/fbettega/MTG_decklistcache"
FORMATS = ("premodern", "vintage", "legacy", "pauper", "modern", "standard", "pioneer")
BASIC_NAMES = {name.casefold() for name in ("Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes",
               "Snow-Covered Plains", "Snow-Covered Island", "Snow-Covered Swamp", "Snow-Covered Mountain", "Snow-Covered Forest")}


class SimilarityIndex:
    """Exact multiset Jaccard using an inverted index; no approximate misses."""
    def __init__(self):
        self.postings = defaultdict(list)
        self.sizes = []

    def add(self, cards):
        i = len(self.sizes)
        self.sizes.append(sum(cards.values()))
        for card, count in cards.items():
            self.postings[card].append((i, count))

    def maximum(self, cards):
        intersections = Counter()
        for card, count in cards.items():
            for i, other_count in self.postings[card]:
                intersections[i] += min(count, other_count)
        size = sum(cards.values())
        return max((n / (size + self.sizes[i] - n) for i, n in intersections.items()), default=0.)


def field(value, pascal, snake, default=None):
    return value.get(pascal, value.get(snake, default))


def read_cards(rows, by_name):
    cards, names, held = Counter(), {}, False
    for row in rows:
        name = field(row, "CardName", "card_name", "")
        card = by_name.get(name.casefold())
        if card is None:
            raise KeyError(name)
        count = field(row, "Count", "count", 0)
        if not isinstance(count, int) or isinstance(count, bool) or count < 1 or count > 100:
            raise ValueError("invalid card count")
        cards[card["id"]] += count
        names[card["id"]] = card["name"]
        held |= card["held_out"]
    return cards, names, held


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source", required=True, help="directory with source.tar.gz, revision.txt, LICENSE")
    ap.add_argument("--cards", required=True, help="deck_catalog4 with existing_decks")
    ap.add_argument("--out", required=True, help="new external corpus directory")
    ap.add_argument("--max-per-format", type=int, default=1000)
    ap.add_argument("--similarity", type=float, default=.85)
    ap.add_argument("--mainboard-only", action="store_true",
                    help="Explicit first-game projection: discard the whole sideboard, never replace a main card")
    args = ap.parse_args()
    source, root = Path(args.source), Path(args.out).resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root / "decks").mkdir()
    shutil.copyfile(source / "LICENSE", root / "LICENSE")
    registry = json.loads(Path(args.cards).read_text())
    by_name = {}
    for card in registry["cards"]:
        for name in [card["name"], *card["faces"], " // ".join(card["faces"])]:
            by_name[name.casefold()] = card
    similarity = SimilarityIndex()
    if not registry.get("existing_decks"):
        raise ValueError("An authoritative existing-corpus duplicate check is required")
    for deck in registry["existing_decks"]:
        similarity.add({int(k): v for k, v in deck["main"].items()})
    revision = (source / "revision.txt").read_text().strip()
    archive = source / "source.tar.gz"
    with archive.open("rb") as f:
        archive_sha256 = hashlib.file_digest(f, "sha256").hexdigest()
    retrieved = datetime.now(timezone.utc).isoformat()
    counts, unsupported = Counter(), Counter()
    rejected_sources = []
    accepted, seen = [], set()
    per_format = Counter()
    with tarfile.open(archive, "r|gz") as tar:
        for member in tar:
            if not member.isfile() or not member.name.endswith(".json"):
                continue
            relative = member.name.split("/", 1)[1]
            if not relative.startswith("Tournaments/MTGO/"):
                continue
            base = relative.rsplit("/", 1)[1]
            format_name = next((f for f in FORMATS if base.startswith(f + "-")), None)
            if format_name is None or per_format[format_name] >= args.max_per_format:
                continue
            # Read the member; never extract paths from an external archive.
            try:
                document = orjson.loads(tar.extractfile(member).read())
            except orjson.JSONDecodeError as error:
                counts["invalid_source_json"] += 1
                rejected_sources.append({"path": relative, "reason": str(error)})
                continue
            counts["tournaments"] += 1
            tournament = field(document, "Tournament", "tournament", {})
            for deck_number, raw in enumerate(field(document, "Decks", "decks", [])):
                counts["decks_seen"] += 1
                try:
                    main, names, held = read_cards(field(raw, "Mainboard", "mainboard", []), by_name)
                    side, side_names, side_held = read_cards(
                        [] if args.mainboard_only else field(raw, "Sideboard", "sideboard", []), by_name)
                except KeyError as missing:
                    counts["unsupported_card"] += 1
                    unsupported[str(missing.args[0])] += 1
                    continue
                except ValueError:
                    counts["invalid_count"] += 1
                    continue
                if not 60 <= sum(main.values()) <= 100 or sum(side.values()) > 15:
                    counts["invalid_size"] += 1
                    continue
                if all(name.casefold() in BASIC_NAMES for name in names.values()):
                    counts["all_basic_placeholder"] += 1
                    continue
                identity = hashlib.sha256(orjson.dumps(sorted(main.items()))).hexdigest()
                if identity in seen:
                    counts["duplicate"] += 1
                    continue
                seen.add(identity)
                overlap = similarity.maximum(main)
                if overlap >= args.similarity:
                    counts["near_duplicate_existing_or_new"] += 1
                    continue
                held |= side_held
                bucket = int(identity[:8], 16) % 10
                split = "test" if held or bucket == 0 else "val" if bucket == 1 else "train"
                title = f"MTGO {format_name} {identity[:12]}"
                filename = root / "decks" / f"{identity}.txt"
                source_ref = f"{REPO}/blob/{revision}/{relative}"
                content = (f"# source: {source_ref}, deck index {deck_number}\n"
                           "# attribution: fbettega/MTG_decklistcache; MTGO public tournament lists\n"
                           "# licence: GPL-3.0 (see ../LICENSE)\n"
                           f"# changes: mainboard unchanged; sideboard {'omitted (first-game projection)' if args.mainboard_only else 'unchanged'}; names normalized; player identity omitted\n"
                           f"[deck:{title}]\n")
                content += "".join(f"{n} {names[c]}\n" for c, n in sorted(main.items()))
                if side:
                    content += "\n[sideboard]\n"
                    content += "".join(f"{n} {side_names[c]}\n" for c, n in sorted(side.items()))
                filename.write_text(content)
                accepted.append({"id": f"mtgo/{format_name}/{identity[:20]}", "name": title,
                    "path": str(filename), "format": "constructed", "source_format": format_name,
                    "split": split, "holdout_tier": "unseen_cards" if held else f"{split}_web_decks",
                    "contains_held_out_cards": held, "is_derived": args.mainboard_only,
                    "transformation": "mainboard_only" if args.mainboard_only else "none",
                    "source_key": "mtgo_decklistcache", "source_ref": source_ref,
                    "source_deck_index": deck_number, "source_revision": revision,
                    "source_date": field(tournament, "Date", "date"),
                    "license": "GPL-3.0", "deck_hash": identity,
                    "main_count": sum(main.values()), "sideboard_count": sum(side.values()),
                    "distinct_cards": len(main), "max_preceding_jaccard": overlap,
                    "working_pool_hash": registry["working_pool_hash"],
                    "archetype": "unclassified", "strategy_confidence": "not_inferred"})
                similarity.add(main)
                per_format[format_name] += 1
                counts["accepted"] += 1
                if per_format[format_name] >= args.max_per_format:
                    break
            if counts["tournaments"] % 500 == 0:
                print(dict(counts), dict(per_format), flush=True)
    manifest = {"source": REPO, "source_revision": revision,
                "source_archive_sha256": archive_sha256, "retrieved_at": retrieved,
                "working_pool_hash": registry["working_pool_hash"],
                "existing_decks_checked": len(registry["existing_decks"]),
                "max_jaccard_threshold": args.similarity,
                "counts_by_source_format": dict(per_format),
                "counts_by_split": dict(Counter(d["split"] for d in accepted)), "decks": accepted}
    atomic_json(root / "manifest.json", manifest)
    atomic_json(root / "import-report.json", {"counts": dict(counts),
                "first_unsupported_card_counts": unsupported.most_common(),
                "rejected_sources": rejected_sources})
    print(dict(counts), manifest["counts_by_source_format"], manifest["counts_by_split"], flush=True)


if __name__ == "__main__":
    main()
