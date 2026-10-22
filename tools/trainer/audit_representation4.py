"""Measure exact card-summary aliases in a converted dataset.

Aliases are a diagnostic, not proof of a bug: distinct names may have identical
rules. Known semantically different pairs still require a richer encoder.
This reads only public card metadata and never changes training data.
"""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path

import numpy as np

from run_guard4 import atomic_json


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--data", required=True)
    ap.add_argument("--catalog", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    data = Path(args.data)
    meta = json.loads((data / "meta.json").read_text())
    catalog = json.loads(Path(args.catalog).read_text())
    names = {c["id"]: c["name"] for c in catalog["cards"]}
    rows = np.fromfile(data / "card_dsl.bin", dtype=np.float32).reshape(len(meta["cards"]), 160)
    abilities = np.fromfile(data / "card_abilities.bin", dtype=np.float32).reshape(len(rows), -1)
    if not np.isfinite(rows).all() or not np.isfinite(abilities).all():
        raise ValueError("Nonfinite structural representation")
    summaries, combined = defaultdict(list), defaultdict(list)
    for card, row, ability in zip(meta["cards"], rows, abilities, strict=True):
        if card in names:
            label = {"id": card, "name": names[card]}
            summaries[row.tobytes()].append(label)
            combined[row.tobytes() + ability.tobytes()].append(label)
    aliases = sorted((g for g in summaries.values() if len(g) > 1), key=lambda g: (-len(g), g[0]["id"]))
    combined_aliases = [g for g in combined.values() if len(g) > 1]
    result = {
        "encoder_version": meta["encoder_version"], "dsl_version": meta["dsl_version"],
        "card_dsl_sha256": hashlib.sha256((data / "card_dsl.bin").read_bytes()).hexdigest(),
        "working_cards": sum(map(len, summaries.values())),
        "unique_front_summaries": len(summaries), "collision_groups": len(aliases),
        "cards_in_collision_groups": sum(map(len, aliases)),
        "summary_plus_ability_collision_groups": len(combined_aliases),
        "summary_plus_ability_colliding_cards": sum(map(len, combined_aliases)),
        "scope": "Front face only. Combined ability rows are diagnostic and are not all actor inputs.",
        "interpretation": "Check rule differences; exact aliases include legitimate functional reprints.",
        "groups": aliases,
    }
    atomic_json(Path(args.out), result)
    print(json.dumps({k: v for k, v in result.items() if k != "groups"}, indent=2))


if __name__ == "__main__":
    main()
