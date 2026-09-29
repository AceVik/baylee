"""Exports a v3 net's score tables to ONNX, and checks the export.

    uv run python export_onnx3.py --model ~/baylee-data/models/v3-a --data ~/baylee-data/datasets/d3-r003

Writes `net.onnx` beside `net.pt`: `Net.tables` at batch 1, `--entities`
rows, 8 seat rows and 128 deck rows, which the Rust player
(`baylee_train::netplay3`, feature `onnx`) runs per decision. A card's vector
is folded into one table first: the id embedding and the projected structure
summed, and for every id training never met (`seen_ids.npy`) the unseen
vector in place of its untrained id embedding. Before writing, held-out
decisions are run through torch and onnxruntime and an export that disagrees
is refused.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

import dataset3 as D3
import model3 as M

INPUTS = ("cards", "feats", "mask", "seats", "glob", "deck_cards", "deck_feats", "deck_mask", "profile")


class Folded(torch.nn.Module):
    """A card's vector from one table."""

    def __init__(self, table: torch.Tensor):
        super().__init__()
        self.table = torch.nn.Embedding.from_pretrained(table, freeze=True, padding_idx=0)

    def forward(self, ids: torch.Tensor) -> torch.Tensor:
        return self.table(ids)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--check", type=int, default=64, help="held-out decisions compared")
    args = ap.parse_args()
    out = Path(args.model).expanduser()
    ckpt = torch.load(out / "net.pt", weights_only=False)
    ds = D3.load(Path(args.data).expanduser())
    skip = ("id_space", "glob_numeric", "card_width", "seat_cols", "max_seats")
    cfg = {k: v for k, v in ckpt["config"].items() if k not in skip}
    net = M.build(ds.meta, ds.card_ids, ds.card_feats, **cfg)
    net.load_state_dict(ckpt["state"])
    net.eval()
    seen = torch.from_numpy(np.load(out / "seen_ids.npy"))
    net.trunk.cards = Folded(net.trunk.cards.folded(seen))

    class Tables(torch.nn.Module):
        def __init__(self, net):
            super().__init__()
            self.net = net

        def forward(self, *xs):
            return self.net.tables(*xs)

    wrapped = Tables(net).eval()
    E = args.entities
    _, held = D3.split_by_game(ds)
    idx = np.sort(np.random.default_rng(3).choice(held, size=args.check, replace=False))

    def feeds_of(i: int) -> dict[str, np.ndarray]:
        r = D3.gather(ds, np.array([i]), E, ds.meta["max_deck"])
        prof = np.array([max(int(ds.col("profile")[i]), 0)], dtype=np.int64)
        return {"cards": r["cards"], "feats": r["feats"], "mask": r["mask"], "seats": r["seats"],
                "glob": r["glob"], "deck_cards": r["deck_cards"], "deck_feats": r["deck_feats"],
                "deck_mask": r["deck_mask"], "profile": prof}

    example = tuple(torch.from_numpy(v) for v in feeds_of(int(idx[0])).values())
    path = out / "net.onnx"
    torch.onnx.export(wrapped, example, str(path), input_names=list(INPUTS), output_names=list(M.Net.TABLES),
                      opset_version=18, dynamo=False)

    import onnxruntime as ort

    sess = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
    worst = 0.0
    with torch.inference_mode():
        for i in idx:
            feeds = feeds_of(int(i))
            got = sess.run(None, feeds)
            want = wrapped(*[torch.from_numpy(feeds[k]) for k in INPUTS])
            for g, w in zip(got, want):
                w = w.numpy()
                finite = np.isfinite(w)
                assert (np.isfinite(g) == finite).all(), "padding differs"
                if finite.any():
                    worst = max(worst, float(np.abs(g[finite] - w[finite]).max()))
    meta = {
        "encoder_version": ds.meta["encoder_version"],
        "walk_version": ds.meta["walk_version"],
        "entities": E,
        "max_seats": ds.meta["max_seats"],
        "max_deck": ds.meta["max_deck"],
        "inputs": {
            "cards": [1, E, "int64"], "feats": [1, E, len(ds.meta["ent_cols"]), "int16"],
            "mask": [1, E, "bool, true = padding"],
            "seats": [1, ds.meta["max_seats"], len(ds.meta["seat_cols"]), "int16"],
            "glob": [1, len(ds.meta["glob_cols"]), "int16"],
            "deck_cards": [1, ds.meta["max_deck"], "int64"],
            "deck_feats": [1, ds.meta["max_deck"], len(ds.meta["deck_cols"]), "int16"],
            "deck_mask": [1, ds.meta["max_deck"], "bool, true = padding"],
            "profile": [1, "int64", ds.meta["profiles"]],
        },
        "outputs": list(M.Net.TABLES),
        "config": ckpt["config"],
        "trained_on": ckpt["dataset"],
        "verification": ckpt.get("verification"),
        "best_step": ckpt.get("step"),
        "seen_ids": int(seen.sum()),
        "parity_max_abs_diff": worst,
        "checked_decisions": int(len(idx)),
    }
    (out / "net.onnx.json").write_text(json.dumps(meta, indent=1))
    print(f"[export3] {path} ({path.stat().st_size / 1e6:.1f} MB); torch vs onnxruntime on {len(idx)} "
          f"held-out decisions: max |diff| {worst:.2e}")
    if worst > 1e-3:
        path.unlink()
        raise SystemExit("the export disagrees with torch; removed it")


if __name__ == "__main__":
    main()
