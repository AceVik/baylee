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
    bounds = defaultdict(list)
    cap_pairs = 0
    for deal in value["deal_details"]:
        key = (deal["table"], deal["seed"], tuple(deal["decks"]))
        groups[key].append(float(deal["net_utility"]) - float(deal["base_utility"]))
        net_lo, net_hi = (0., 1.) if deal["net_capped"] else (float(deal["net_utility"]),) * 2
        base_lo, base_hi = (0., 1.) if deal["base_capped"] else (float(deal["base_utility"]),) * 2
        bounds[key].append((net_lo - base_hi, net_hi - base_lo))
        cap_pairs += int(deal["net_capped"] or deal["base_capped"])
    if not groups:
        raise ValueError("No paired table details")
    means = np.array([np.mean(rows) for rows in groups.values()])
    sizes = np.array([len(rows) for rows in groups.values()])
    rng = np.random.default_rng(20261023)
    ids = rng.integers(len(means), size=(resamples, len(means)))
    estimate = float(np.average(means, weights=sizes))
    table_bounds = np.array([np.mean(bounds[key], axis=0) for key in groups])
    observed_bounds = np.average(table_bounds, axis=0, weights=sizes)
    bootstrap_bounds = (table_bounds[ids] * sizes[ids, None]).sum(1) / sizes[ids].sum(1)[:, None]
    # A zero empirical variance must not imply certainty from four tables.
    # Hoeffding for independent bounded table differences in [-1, 1]. This
    # still assumes representative independent tables, not arbitrary new decks.
    radius = math.sqrt(2 * math.log(40) * float(np.square(sizes / sizes.sum()).sum()))
    return {"deals": int(sizes.sum()), "independent_tables": len(means),
            "paired_utility_difference": None if cap_pairs else estimate,
            "paired_difference_with_caps_imputed_half": estimate,
            "censoring_bounds_on_observed_tables": observed_bounds.tolist(),
            "table_bootstrap_95_percent_interval": [float(np.quantile(bootstrap_bounds[:, 0], .025)), float(np.quantile(bootstrap_bounds[:, 1], .975))],
            "bounded_independent_table_95_percent_interval": [max(-1., float(observed_bounds[0]) - radius), min(1., float(observed_bounds[1]) + radius)],
            "capped_pairs": cap_pairs, "bootstrap_resamples": resamples,
            "cluster": "table/seed/decks; all focal-seat rotations stay together",
            "scope": "Conditional on these decks, opponent profiles and seeds; not a human-skill claim",
            "caps": "Capped utilities are unknown in [0,1]; interval includes worst/best completion, not a draw",
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
