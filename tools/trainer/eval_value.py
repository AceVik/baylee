"""Scores a trained value net on games it never saw, beside a baseline.

    uv run python eval_value.py --model ~/baylee-data/models/value-v2 --data ~/baylee-data/datasets/d004-mirror

The baseline knows only what needs no reading of the game: the deciding
seat's deck (unless `--no-deck`), its seat and the turn band, fit on half of
the dataset's games; the net and the baseline are both scored on the other
half. Where the net beats it, the net reads the game; where it does not,
what it knows is the decks. In mirror matches the deck says nothing, so run
mirrors with `--no-deck`.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

import dataset as D
import model as M
from train_value import metrics, predict

BANDS = ["turns 1-3", "turns 4-6", "turns 7-9", "turns 10-14", "turns 15+"]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--entities", type=int, default=160)
    ap.add_argument("--no-deck", action="store_true", help="baseline without the deck (for mirrors)")
    args = ap.parse_args()
    ckpt = torch.load(Path(args.model).expanduser() / "value.pt", weights_only=False)
    ds = D.load(Path(args.data).expanduser())
    cfg = {k: v for k, v in ckpt["config"].items() if k not in ("id_space", "glob_numeric")}
    net = M.build(ds.meta, **cfg).cuda()
    net.load_state_dict(ckpt["state"])

    first = ds.col("step") == 0 if "step" in ds.meta["meta_cols"] else np.ones(ds.n, dtype=bool)
    game = ds.col("game")
    fit = np.nonzero(first & (game % 2 == 0))[0]
    ev = np.nonzero(first & (game % 2 == 1))[0]
    y = ds.col("result").astype(np.float64) / 2
    turn = ds.col("turn")
    band = np.select([turn <= 3, turn <= 6, turn <= 9, turn <= 14], [0, 1, 2, 3], 4)
    seat = ds.col("seat")
    deck = np.zeros(ds.n, dtype=np.int64)
    if not args.no_deck:
        names = sorted({d for g in ds.games for d in g["decks"]})
        deck = np.array([names.index(ds.games[g]["decks"][s]) for g, s in zip(game, seat)])
    key = deck * 100 + seat * 10 + band
    table = {k: y[fit][key[fit] == k].mean() for k in np.unique(key[fit])}
    base = np.array([table.get(k, 0.5) for k in key[ev]])
    p = predict(net, ds, ev, 1024, args.entities).astype(np.float64)
    ye = y[ev]
    rows = []
    for b, name in enumerate(BANDS):
        m = band[ev] == b
        if m.sum() < 100:
            continue
        mb, mn = metrics(base[m], ye[m]), metrics(p[m], ye[m])
        rows.append({"band": name, "n": int(m.sum()), "baseline_brier": mb["brier"], "baseline_acc": mb["accuracy"],
                     "net_brier": mn["brier"], "net_acc": mn["accuracy"], "net_ece": mn["ece"],
                     "net_mean": float(p[m].mean()), "won": float(ye[m].mean())})
    allb, alln = metrics(base, ye), metrics(p, ye)
    rep = {"model": args.model, "data": args.data, "baseline": "seat + turn band" + ("" if args.no_deck else " + deck"),
           "bands": rows, "all": {"n": int(len(ev)), "baseline_brier": allb["brier"], "net_brier": alln["brier"],
                                   "net_ece": alln["ece"], "net_acc": alln["accuracy"]}}
    print(f"baseline: {rep['baseline']}; {len(ev):,} decisions of {len(set(game[ev].tolist())):,} games")
    print(f"{'band':12} {'n':>8} {'baseline Brier/acc':>20} {'net Brier/acc':>16} {'net ECE':>8} {'net mean':>9} {'won':>6}")
    for r in rows:
        print(f"{r['band']:12} {r['n']:8,} {r['baseline_brier']:12.4f}/{r['baseline_acc']:.3f} {r['net_brier']:9.4f}/{r['net_acc']:.3f}"
              f" {r['net_ece']:8.4f} {r['net_mean']:9.3f} {r['won']:6.3f}")
    a = rep["all"]
    print(f"{'all':12} {a['n']:8,} {a['baseline_brier']:12.4f}      {a['net_brier']:9.4f}/{a['net_acc']:.3f} {a['net_ece']:8.4f}")
    out = Path(args.model).expanduser() / f"eval-{Path(args.data).name}.json"
    out.write_text(json.dumps(rep, indent=1))


if __name__ == "__main__":
    main()
