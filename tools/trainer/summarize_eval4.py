"""Paired eval uncertainty, resampling whole tables rather than correlated seats."""
import argparse
from collections import defaultdict
import json
import math
from pathlib import Path

import numpy as np

from run_guard4 import atomic_json


def summarize(value, resamples=20000):
    if value.get("no_baseline"):
        raise ValueError("A paired house baseline is required")
    groups = defaultdict(list)
    cap_pairs = 0
    for deal in value["deal_details"]:
        key = (deal["table"], deal["seed"], tuple(deal["decks"]))
        groups[key].append(float(deal["net_utility"]) - float(deal["base_utility"]))
        cap_pairs += int(deal["net_capped"] or deal["base_capped"])
    if not groups:
        raise ValueError("No paired table details")
    means = np.array([np.mean(rows) for rows in groups.values()])
    sizes = np.array([len(rows) for rows in groups.values()])
    rng = np.random.default_rng(20261023)
    ids = rng.integers(len(means), size=(resamples, len(means)))
    bootstrap = (means[ids] * sizes[ids]).sum(1) / sizes[ids].sum(1)
    estimate = float(np.average(means, weights=sizes))
    # A zero empirical variance must not imply certainty from four tables.
    # Hoeffding for independent bounded table differences in [-1, 1]. This
    # still assumes representative independent tables, not arbitrary new decks.
    radius = math.sqrt(2 * math.log(40) * float(np.square(sizes / sizes.sum()).sum()))
    return {"deals": int(sizes.sum()), "independent_tables": len(means),
            "paired_utility_difference": estimate,
            "table_bootstrap_95_percent_interval": np.quantile(bootstrap, [.025, .975]).tolist(),
            "bounded_independent_table_95_percent_interval": [max(-1., estimate - radius), min(1., estimate + radius)],
            "capped_pairs": cap_pairs, "bootstrap_resamples": resamples,
            "cluster": "table/seed/decks; all focal-seat rotations stay together",
            "scope": "Conditional on these decks, opponent profiles and seeds; not a human-skill claim",
            "small_sample": len(means) < 20,
            "warning": "Few tables can yield deceptively narrow bootstrap intervals; expand independent tables" if len(means) < 20 else None}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("evaluation")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    value = json.loads(Path(args.evaluation).read_text())
    report = summarize(value)
    atomic_json(Path(args.out), report)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
