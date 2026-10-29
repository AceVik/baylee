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
from pathlib import Path

import numpy as np
import torch

import dataset as D
import model as M
from train_value import predict
from legacy_evaluation import native_metrics




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

    rows = {name: ds.col(name).tolist() for name in ('game', 'seat', 'turn', 'result')}
    if 'step' in ds.meta['meta_cols']:
        rows['step'] = ds.col('step').tolist()
    plan = native_metrics('value-plan', rows=rows, games=ds.games, no_deck=args.no_deck)
    indices = np.asarray(plan['indices'], dtype=np.int64)
    prediction = predict(net, ds, indices, 1024, args.entities)
    report = native_metrics('value-report', plan=plan, prediction=prediction.tolist(),
                            model=args.model, data=args.data)
    print(report['text'], end='')


if __name__ == "__main__":
    main()
