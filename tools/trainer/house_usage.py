"""Which cards the house is offered and does not use.

    uv run python house_usage.py --data ~/baylee-data/datasets/d003 --out ~/baylee-data/reports/house-usage-d003

Reads a v2 or v3 dataset's options and the house's choices: per card and way
of using it (play it as a land, cast it, activate ability slot k, …) how
often a question offered it and how often the house took it, when offered.
A card offered often and almost never taken is where the house's heuristics
may not know what the card is for, most useful for newly implemented cards;
it complements `docs/ai-coverage-todo.md`. Only priority questions are
counted (the ones where using a card is one choice among many), and neither
mana abilities (the house activates those while paying, not as a choice at
priority), granted abilities (slots 48 and up: mostly granted mana) nor tokens
and unknown objects. Never taken is not wrong by itself (Homeward Path waits
for a stolen creature): the list is what to look at, not what is broken.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np

import dataset as D

H_ENTITY, H_ABILITY = 1, 2
VERBS = ["land", "cast", "suspend", "mana", "pick"]


def load(path: Path):
    meta = json.loads((path / "dataset.json").read_text())
    if meta["encoder_version"] == 3:
        import dataset3 as D3

        return D3.load(path)
    return D.load(path)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--min-offered", type=int, default=200)
    ap.add_argument("--names", default="~/baylee-data/verify/ability-log/pool-inventory.json",
                    help="a pool inventory (card, name), from a BAYLEE_ABILITY_LOG run")
    args = ap.parse_args()
    ds = load(Path(args.data).expanduser())
    kinds = ds.meta["pending_kinds"]
    priority = kinds.index("priority")
    idx = np.nonzero((ds.col("pending_kind") == priority) & (ds.col("n_opts") > 1))[0]
    start, end = ds.opt_off[idx], ds.opt_off[idx + 1]
    count = end - start
    sample = np.repeat(idx, count)
    first = np.repeat(start, count)
    offsets = np.concatenate([[0], np.cumsum(count)[:-1]])
    flat = first + np.arange(int(count.sum())) - np.repeat(offsets, count)
    opt = ds.opt[flat].astype(np.int64)
    chosen_flat = np.repeat(ds.col("chosen")[idx], count)
    was_chosen = (flat - first) == chosen_flat
    row_heads = np.isin(opt[:, 0], [H_ENTITY, H_ABILITY])
    opt, sample, was_chosen = opt[row_heads], sample[row_heads], was_chosen[row_heads]
    # The option's object: its row in its sample, then that row's card id.
    row = ds.ent_off[sample] + opt[:, 1]
    ok = row < ds.ent_off[sample + 1]
    card = np.where(ok, ds.ent_card[np.minimum(row, len(ds.ent_card) - 1)], 0)
    use = np.where(opt[:, 0] == H_ENTITY, opt[:, 2], 100 + opt[:, 2])  # verb, or 100 + ability slot
    key = card.astype(np.int64) * 1000 + use
    keys, inv = np.unique(key[ok], return_inverse=True)
    offered = np.bincount(inv)
    taken = np.bincount(inv, weights=was_chosen[ok].astype(np.float64))
    names, mana = {}, set()
    inventory = Path(args.names).expanduser()
    if inventory.exists():
        for r in json.loads(inventory.read_text()):
            names[r["card"]] = r["name"]
            for a in r["abilities"]:
                if a["kind"] == "mana" and a["position"] < 48:
                    mana.add((r["card"], a["position"]))
    first_non_card = ds.meta["ids"]["ledger_rows"] + 1
    rows = []
    for k, o, t in zip(keys, offered, taken):
        c, u = int(k // 1000), int(k % 1000)
        if c == 0 or c >= first_non_card or VERBS[u if u < 100 else 0] == "mana" and u < 100:
            continue
        if u >= 100 and ((c - 1, u - 100) in mana or u - 100 >= 48):
            continue
        how = VERBS[u] if u < 100 else f"ability {u - 100}"
        rows.append({"card_id": c, "card_index": c - 1, "name": names.get(c - 1, "?"), "use": how,
                     "offered": int(o), "taken": int(t), "rate": float(t / o)})
    rows.sort(key=lambda r: (r["rate"], -r["offered"]))
    rare = [r for r in rows if r["offered"] >= args.min_offered]
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "usage.json").write_text(json.dumps({"data": args.data, "priority_decisions": int(len(idx)),
                                                 "rows": rows}, indent=1))
    lines = [f"# House usage — {Path(args.data).name}", "",
             f"{len(idx):,} priority decisions with a choice. Offered at least {args.min_offered} times, "
             "rarest taken first:", "",
             "| card | use | offered | taken | rate |", "|---|---|---|---|---|"]
    lines += [f"| {r['name']} ({r['card_index']}) | {r['use']} | {r['offered']:,} | {r['taken']:,} | {r['rate']:.4f} |"
              for r in rare[:60]]
    (out / "usage.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:40]))


if __name__ == "__main__":
    main()
