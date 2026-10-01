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
    ap.add_argument("--batch", type=int, default=0,
                    help="instead: add exports of every bucket at this fixed batch (net-e<rows>-b<B>.onnx) for "
                         "the GPU batch server, leaving the batch-1 files as they are")
    ap.add_argument("--buckets", default="32,64,96,128",
                    help="smaller row counts exported beside --entities; the player runs a decision on the "
                         "smallest that holds its rows (the tracing exporter fixes the sequence length)")
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

    def feeds_of(i: int, rows: int = E) -> dict[str, np.ndarray]:
        r = D3.gather(ds, np.array([i]), rows, ds.meta["max_deck"])
        prof = np.array([max(int(ds.col("profile")[i]), 0)], dtype=np.int64)
        return {"cards": r["cards"], "feats": r["feats"], "mask": r["mask"], "seats": r["seats"],
                "glob": r["glob"], "deck_cards": r["deck_cards"], "deck_feats": r["deck_feats"],
                "deck_mask": r["deck_mask"], "profile": prof}

    example = tuple(torch.from_numpy(v) for v in feeds_of(int(idx[0])).values())
    path = out / "net.onnx"
    buckets = sorted({int(b) for b in args.buckets.split(",") if b} | {E})
    buckets = [b for b in buckets if b <= E]
    if args.batch:
        export_batched(args.batch, out, wrapped, ds, idx, buckets)
        return
    files = {}
    for rows in buckets:
        name = "net.onnx" if rows == E else f"net-e{rows}.onnx"
        ex = tuple(torch.from_numpy(v) for v in feeds_of(int(idx[0]), rows).values())
        torch.onnx.export(wrapped, ex, str(out / name), input_names=list(INPUTS),
                          output_names=list(M.Net.TABLES), opset_version=18, dynamo=False)
        files[rows] = name
    path = out / "net.onnx"

    import onnxruntime as ort

    sessions = {rows: ort.InferenceSession(str(out / name), providers=["CPUExecutionProvider"])
                for rows, name in files.items()}
    worst = 0.0
    with torch.inference_mode():
        for i in idx:
            # Each decision on the smallest bucket that holds its rows, as
            # the Rust player runs it.
            n = int(ds.ent_off[i + 1] - ds.ent_off[i])
            rows = next((b for b in buckets if b >= n), E)
            feeds = feeds_of(int(i), rows)
            got = sessions[rows].run(None, feeds)
            want = wrapped(*[torch.from_numpy(feeds[k]) for k in INPUTS])
            for g, w in zip(got, want):
                w = w.numpy()
                if np.isnan(g).any() or np.isnan(w).any():
                    raise SystemExit("the net outputs NaN: refusing to export it")
                finite = np.isfinite(w)
                assert (np.isfinite(g) == finite).all(), "padding differs"
                if finite.any():
                    worst = max(worst, float(np.abs(g[finite] - w[finite]).max()))
    meta = {
        "encoder_version": ds.meta["encoder_version"],
        "walk_version": ds.meta["walk_version"],
        "entities": E,
        "buckets": [{"entities": rows, "file": name} for rows, name in files.items()],
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
        for name in files.values():
            (out / name).unlink()
        raise SystemExit("the export disagrees with torch; removed it")


def export_batched(B: int, out: Path, wrapped, ds, idx, buckets) -> None:
    """Every bucket at a fixed batch of `B`, checked against torch on
    held-out decisions in batches of `B`, and listed under `batched` in
    `net.onnx.json`. The server always runs exactly `B` rows, so a
    decision's result never depends on which others share its batch."""
    import onnxruntime as ort

    def feeds_many(ids: list[int], rows: int) -> dict[str, np.ndarray]:
        r = D3.gather(ds, np.asarray(ids), rows, ds.meta["max_deck"])
        prof = np.array([max(int(ds.col("profile")[i]), 0) for i in ids], dtype=np.int64)
        return {k: (prof if k == "profile" else r[k]) for k in INPUTS}

    ids = [int(i) for i in idx]
    chunks = [(ids[j : j + B] + ids[: B])[:B] for j in range(0, len(ids), B)]
    made, worst = [], 0.0
    # Traced outside inference mode, as the batch-1 files are: inside it the
    # encoder takes a fused path that has no ONNX form.
    for rows in buckets:
        name = f"net-e{rows}-b{B}.onnx"
        ex = tuple(torch.from_numpy(v) for v in feeds_many(chunks[0], rows).values())
        torch.onnx.export(wrapped, ex, str(out / name), input_names=list(INPUTS),
                          output_names=list(M.Net.TABLES), opset_version=18, dynamo=False)
        made.append({"entities": rows, "batch": B, "file": name})
    with torch.inference_mode():
        for m in made:
            rows = m["entities"]
            session = ort.InferenceSession(str(out / m["file"]), providers=["CPUExecutionProvider"])
            for chunk in chunks:
                feeds = feeds_many(chunk, rows)
                got = session.run(None, feeds)
                want = wrapped(*[torch.from_numpy(feeds[k]) for k in INPUTS])
                for g, w in zip(got, want):
                    w = w.numpy()
                    # Masked entries are -inf; NaN is never an output. The
                    # check below compares finite entries only, so a net whose
                    # weights went NaN passed it as "max |diff| 0".
                    if np.isnan(g).any() or np.isnan(w).any():
                        raise SystemExit("the net outputs NaN: refusing to export it")
                    finite = np.isfinite(w)
                    assert (np.isfinite(g) == finite).all(), "padding differs"
                    if finite.any():
                        worst = max(worst, float(np.abs(g[finite] - w[finite]).max()))
    if worst > 1e-3:
        for m in made:
            (out / m["file"]).unlink()
        raise SystemExit(f"the batched export disagrees with torch ({worst:.2e}); removed it")
    meta = json.loads((out / "net.onnx.json").read_text())
    meta["batched"] = made
    meta["batched_parity_max_abs_diff"] = worst
    (out / "net.onnx.json").write_text(json.dumps(meta, indent=1))
    print(f"[export3] {len(made)} buckets at batch {B}; torch vs onnxruntime on {len(ids)} held-out decisions: "
          f"max |diff| {worst:.2e}")


if __name__ == "__main__":
    main()
