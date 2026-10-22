"""Import diverse, unmodified 40+ card decks from 17Lands' CC BY 4.0 dumps.

Only the public bulk links published in its dataset catalog are read. No deck
pages, private APIs, player profiles, card assets or gameplay labels are used.
Unknown/unimplemented cards reject the entire deck. Decks containing held-out
cards stay in test. Near duplicates are rejected across the entire new corpus.
"""
from __future__ import annotations

import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
import gzip
import hashlib
import io
import json
from pathlib import Path
import urllib.parse
import urllib.request

LICENSE = "https://creativecommons.org/licenses/by/4.0/"
CATALOG = "https://www.17lands.com/public_datasets"
SOURCE_PREFIX = "https://17lands-public.s3.amazonaws.com/analysis_data/game_data/"


def request(url):
    return urllib.request.urlopen(urllib.request.Request(
        url, headers={"User-Agent": "baylee-training/4 (non-commercial deck research)"}), timeout=45)


def fetch_catalog():
    base = "https://17lands.cdn.prismic.io/api/v2"
    with request(base) as response:
        api = json.load(response)
    ref = next(item["ref"] for item in api["refs"] if item["isMasterRef"])
    url = base + "/documents/search?" + urllib.parse.urlencode({
        "ref": ref, "q": '[[at(document.type,"public-data")]]'})
    with request(url) as response:
        return json.load(response)


def catalog_sources(catalog):
    sources = {}
    for entry in catalog["results"][0]["data"]["datasets"]:
        expansion = entry["expansion"][0]["text"]
        mode = entry["format"][0]["text"]
        for paragraph in entry.get("game_data", []):
            for span in paragraph.get("spans", []):
                url = span.get("data", {}).get("url", "")
                if url.startswith(SOURCE_PREFIX) and url.endswith(".csv.gz"):
                    # One mode per expansion keeps requests and repeated deck
                    # families bounded. Prefer traditional draft's smaller dump.
                    rank = {"TradDraft": 0, "TradSealed": 1, "Sealed": 2}.get(mode, 3)
                    if expansion not in sources or rank < sources[expansion][0]:
                        sources[expansion] = (rank, mode, url)
    return [(expansion, mode, url) for expansion, (_, mode, url) in sorted(sources.items())]


def similarity(a, b):
    intersection = sum(min(value, b.get(key, 0)) for key, value in a.items())
    return intersection / (sum(a.values()) + sum(b.values()) - intersection)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cards", required=True, help="deck_catalog4 output")
    parser.add_argument("--out", required=True, help="new corpus directory")
    parser.add_argument("--rows-per-source", type=int, default=10000)
    parser.add_argument("--decks-per-source", type=int, default=80)
    parser.add_argument("--sets", default="", help="comma-separated expansion names; empty means all published sets")
    args = parser.parse_args()
    root = Path(args.out).resolve()
    root.mkdir(parents=True, exist_ok=False)
    (root / "decks").mkdir()
    registry = json.loads(Path(args.cards).read_text())
    by_name = {}
    for card in registry["cards"]:
        for name in [card["name"], *card["faces"], " // ".join(card["faces"])]:
            by_name[name.casefold()] = card
    catalog = fetch_catalog()
    (root / "source-catalog.json").write_text(json.dumps(catalog, indent=2))
    selected = set(args.sets.split(",")) if args.sets else None
    accepted, multisets, source_reports = [], [], []
    exact_hashes = set()
    fetched = datetime.now(timezone.utc).isoformat()
    for expansion, mode, url in catalog_sources(catalog):
        if selected is not None and expansion not in selected:
            continue
        counts = Counter()
        report = {"expansion": expansion, "mode": mode, "url": url,
                  "retrieved_at": fetched, "counts": counts}
        try:
            with request(url) as response:
                report["etag"] = response.headers.get("ETag")
                with io.TextIOWrapper(gzip.GzipFile(fileobj=response), encoding="utf-8-sig") as stream:
                    reader = csv.reader(stream)
                    header = next(reader)
                    columns = [(i, by_name.get(name[5:].casefold())) for i, name in enumerate(header)
                               if name.startswith("deck_")]
                    if not columns:
                        raise ValueError("No deck_ columns in published schema")
                    known = sum(card is not None for _, card in columns)
                    report["known_card_columns"] = known
                    report["card_columns"] = len(columns)
                    # A nearly absent set cannot yield a representative legal
                    # corpus; avoid scanning a huge file for an all-basic oddity.
                    if known / len(columns) < .4:
                        counts["skipped_low_coverage_source"] += 1
                    else:
                        draft_column = header.index("draft_id") if "draft_id" in header else None
                        for row_number, row in enumerate(reader, start=2):
                            if counts["rows"] >= args.rows_per_source or counts["accepted"] >= args.decks_per_source:
                                break
                            counts["rows"] += 1
                            cards, names, held, unsupported = Counter(), {}, False, False
                            for column, card in columns:
                                count = int(row[column] or 0)
                                if count == 0:
                                    continue
                                if card is None:
                                    unsupported = True
                                    break
                                cards[card["id"]] += count
                                names[card["id"]] = card["name"]
                                held |= card["held_out"]
                            if unsupported:
                                counts["unsupported_card"] += 1
                                continue
                            size = sum(cards.values())
                            if size < 40 or size > 100 or min(cards.values(), default=0) < 1:
                                counts["invalid_size"] += 1
                                continue
                            identity = hashlib.sha256(json.dumps(sorted(cards.items())).encode()).hexdigest()
                            if identity in exact_hashes:
                                counts["duplicate"] += 1
                                continue
                            exact_hashes.add(identity)
                            if any(similarity(cards, previous) >= .85 for previous in multisets):
                                counts["near_duplicate"] += 1
                                continue
                            # All variants of one anonymized draft belong to one
                            # split; retain only its hash, no user identity fields.
                            family = row[draft_column] if draft_column is not None else identity
                            family_hash = hashlib.sha256((expansion + ":" + family).encode()).hexdigest()
                            bucket = int(family_hash[:8], 16) % 10
                            split = "test" if held or bucket == 0 else "val" if bucket == 1 else "train"
                            deck_id = f"17lands/{expansion}/{identity[:20]}"
                            path = root / "decks" / f"{identity}.txt"
                            text = (f"# baylee deck export v1\n# name: {expansion} {mode} {identity[:12]}\n"
                                    f"# format: freeform\n# source: 17Lands, {url}, CSV row {row_number}\n"
                                    f"# licence: CC BY 4.0, {LICENSE}\n# changes: none (card names normalized)\n")
                            text += "".join(f"{count} {names[card]}\n" for card, count in sorted(cards.items()))
                            path.write_text(text)
                            accepted.append({"id": deck_id, "name": f"{expansion} {mode} {identity[:12]}",
                                             "path": str(path), "format": "limited", "split": split,
                                             "holdout_tier": "unseen_cards" if held else f"{split}_limited",
                                             "contains_held_out_cards": held, "is_derived": False,
                                             "source_key": "17lands_public_bulk", "source_ref": url,
                                             "source_row": row_number, "source_date": fetched,
                                             "license": "CC-BY-4.0", "license_url": LICENSE,
                                             "deck_hash": identity, "family_hash": family_hash,
                                             "main_count": size, "distinct_cards": len(cards),
                                             "working_pool_hash": registry["working_pool_hash"]})
                            multisets.append(cards)
                            counts["accepted"] += 1
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
        source_reports.append(report)
        (root / "manifest.json").write_text(json.dumps({"decks": accepted}, indent=2))
        (root / "import-report.json").write_text(json.dumps(source_reports, indent=2))
        print(expansion, mode, dict(counts), report.get("error", ""), flush=True)
    held_families = {deck["family_hash"] for deck in accepted if deck["contains_held_out_cards"]}
    for deck in accepted:
        if deck["family_hash"] in held_families:
            deck["split"] = "test"
            deck["holdout_tier"] = "unseen_cards_family"
    (root / "manifest.json").write_text(json.dumps({"decks": accepted}, indent=2))
    print(f"Accepted {len(accepted)} unmodified decks; splits {dict(Counter(d['split'] for d in accepted))}")


if __name__ == "__main__":
    main()
