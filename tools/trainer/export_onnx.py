"""Exports a trained policy's score tables to ONNX, and checks the export.

    uv run python export_onnx.py --model ~/baylee-data/models/policy-v1 --data ~/baylee-data/datasets/d003

Writes `policy.onnx` beside `policy.pt`: the fixed-shape part of the net
(`PolicyNet.tables`), batch 1 and `--entities` rows, which is what a player's
machine runs per decision — CPU, GPU or NPU. The options are scored from its
outputs by the Rust player (`baylee-train`, feature `onnx`), which mirrors
`PolicyNet.score`. Before writing, it runs real held-out decisions through
both torch and onnxruntime and refuses an export that disagrees.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

import dataset as D
import model as M


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--check", type=int, default=64, help="held-out decisions compared")
    args = ap.parse_args()
    out = Path(args.model).expanduser()
    ckpt = torch.load(out / "policy.pt", weights_only=False)
    ds = D.load(Path(args.data).expanduser())
    cfg = {k: v for k, v in ckpt["config"].items() if k not in ("id_space", "glob_numeric")}
    net = M.build_policy(ds.meta, **cfg)
    net.load_state_dict(ckpt["state"])
    net.eval()

    class Tables(torch.nn.Module):
        def __init__(self, net):
            super().__init__()
            self.net = net

        def forward(self, cards, feats, mask, glob, profile):
            return self.net.tables(cards, feats, mask, glob, profile)

    wrapped = Tables(net).eval()
    E = args.entities
    _, held = D.split_by_game(ds)
    idx = np.sort(np.random.default_rng(3).choice(held, size=args.check, replace=False))
    raw = D.gather(ds, idx[:1], E)
    profile = torch.tensor([4])
    example = (
        torch.from_numpy(raw["cards"]),
        torch.from_numpy(raw["feats"]),
        torch.from_numpy(raw["mask"]),
        torch.from_numpy(raw["glob"]),
        profile,
    )
    path = out / "policy.onnx"
    torch.onnx.export(
        wrapped,
        example,
        str(path),
        input_names=["cards", "feats", "mask", "glob", "profile"],
        output_names=list(M.PolicyNet.TABLES),
        opset_version=18,
        dynamo=False,
    )

    import onnxruntime as ort

    sess = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
    worst = 0.0
    with torch.inference_mode():
        for i in idx:
            r = D.gather(ds, np.array([i]), E)
            prof = np.array([max(int(ds.col("profile")[i]), 0)], dtype=np.int64)
            feeds = {"cards": r["cards"], "feats": r["feats"], "mask": r["mask"], "glob": r["glob"], "profile": prof}
            got = sess.run(None, feeds)
            want = wrapped(*[torch.from_numpy(feeds[k]) for k in ("cards", "feats", "mask", "glob", "profile")])
            for g, w in zip(got, want):
                worst = max(worst, float(np.abs(g - w.numpy()).max()))
    meta = {
        "entities": E,
        "inputs": {"cards": [1, E, "int64"], "feats": [1, E, len(ds.meta["ent_cols"]), "int16"],
                   "mask": [1, E, "bool, true = padding"], "glob": [1, ds.meta["glob_width"], "int16"],
                   "profile": [1, "int64", ds.meta["profiles"]]},
        "outputs": list(M.PolicyNet.TABLES),
        "encoder_version": ds.meta["encoder_version"],
        "config": ckpt["config"],
        "trained_on": ckpt["dataset"],
        "best_step": ckpt.get("step"),
        "parity_max_abs_diff": worst,
        "checked_decisions": int(len(idx)),
    }
    (out / "policy.onnx.json").write_text(json.dumps(meta, indent=1))
    print(f"[export] {path} ({path.stat().st_size / 1e6:.1f} MB); torch vs onnxruntime on {len(idx)} "
          f"held-out decisions: max |diff| {worst:.2e}")
    if worst > 1e-3:
        path.unlink()
        raise SystemExit("the export disagrees with torch; removed it")


if __name__ == "__main__":
    main()
