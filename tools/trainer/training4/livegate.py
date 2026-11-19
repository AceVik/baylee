"""Gate scores of the live-search curve (`live_search_curve4`): which searched roots a model
would pick to search, from what the policy and the search heads already compute.

    python -m training4.livegate --rows rows-c2.npz --state-dir STATES_C2 --model R1_DIR \
        --b2 heads/armb/B2 --v1-gaps heads/V1/sealed/gaps.npy --v1-rows rows-c1.npz --v1-rows rows-c2.npz \
        --out gates-c2.jsonl

One JSON line per root of the rows (keyed by game, seat and root cursor, as the sidecar):

- `policy_margin`: minus r1's own logit margin among the searched (macro) candidates, the own
  answer's logit less the best other searched item's, from the captured `cand_emb` through
  r1's last linear (`cand_scorer[3]`): the less sure the policy, the higher;
- `b2_gap`: max over the searched candidates other than the own answer of |q_i - q_own| from arm
  B's B2 tower and head, held-out roots only (null elsewhere);
- `v1_gap`: the same from arm V's V1 sealed gaps, held-out roots only.

Inference only, on the CPU; nothing here is fitted.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

from . import advantage as V


def per_root_max_abs(rows, gaps):
    """[R] max over each root's searched candidates other than the own answer of |gap|."""
    searched = rows.labels["searched"].astype(bool)
    out = np.zeros(len(rows.state))
    for r in range(len(rows.state)):
        lo, n, own = int(rows.roots["offset"][r]), int(rows.roots["n"][r]), int(rows.roots["own"][r])
        items = [i for i in range(n) if searched[lo + i] and i != own]
        out[r] = max((abs(float(gaps[lo + i])) for i in items), default=0.)
    return out


def policy_margins(rows, weight, bias):
    """[R] the own answer's logit less the best other searched item's (r1's last linear on the
    captured candidate rows)."""
    logits = rows.cand.astype(np.float32) @ weight.T.astype(np.float32) + bias.astype(np.float32)
    logits = logits[:, 0]
    searched = rows.labels["searched"].astype(bool)
    out = np.zeros(len(rows.state))
    for r in range(len(rows.state)):
        lo, n, own = int(rows.roots["offset"][r]), int(rows.roots["n"][r]), int(rows.roots["own"][r])
        others = [lo + i for i in range(n) if searched[lo + i] and i != own]
        out[r] = float(logits[lo + own] - max(logits[others])) if others else np.inf
    return out


def b2_gaps(rows_path, state_dir, model, b2_dir, threads, batch=32, log=print):
    """[R] B2's per-root max |gap| on the held-out roots of one rows file (NaN elsewhere)."""
    from torch_io4 import file_digest
    from value_calibration4.checkpoint import load_source
    from .advantage_tower import ARMS, Batches, Roots, TowerPass, load_trainable, make_tower
    torch.set_num_threads(threads)
    path, _, net = load_source(model)
    source = file_digest(path)
    net.decision_chunk, net.board_token_budget = 4, 32768
    net.prune_unused_policy_heads, net.gpu_prefetch = False, 0
    saved = torch.load(Path(b2_dir) / "tower.pt", map_location="cpu", weights_only=False)
    if saved.get("arm") != "B2" or saved.get("source_checkpoint_sha256") != source:
        raise ValueError("The B2 tower is not arm B's B2 over this net")
    full = V.AdvantageRows.load(rows_path)
    keep = full.held_out.copy()
    roots = Roots.load([full], [state_dir], net, source, keeps=[keep], log=log)
    rows = full.subset(keep)
    tower = make_tower(net, "B2")
    load_trainable(tower, "B2", saved["tower"])
    head = V.SearchHead(saved["width"], saved["kind"], saved["hidden"])
    head.load_state_dict(saved["head"])
    tower.eval(), head.eval(), net.eval()
    batches = Batches(rows, roots, TowerPass(net, tower, 32, 2), head, 0.02, torch.device("cpu"))
    gaps = np.zeros(len(rows.cand))
    with torch.no_grad():
        for start in range(0, len(rows.state), batch):
            part = np.arange(start, min(start + batch, len(rows.state)))
            gaps[batches.items(part)] = batches.gaps(part, batches.scores(part)).double().numpy()
            if start // batch % 20 == 0:
                log(f"[livegate] B2 {start + len(part)}/{len(rows.state)}")
    out = np.full(len(full.state), np.nan)
    out[np.flatnonzero(keep)] = per_root_max_abs(rows, gaps)
    return out, dict(tower=str(Path(b2_dir) / "tower.pt"), arm_modules=list(ARMS["B2"]), roots=int(keep.sum()))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--rows", required=True, help="The chunk's search-advantage rows (arm B's capture)")
    parser.add_argument("--state-dir", required=True, help="Its window-state cache (read only)")
    parser.add_argument("--model", required=True, help="r1")
    parser.add_argument("--b2", required=True, help="Arm B's B2 directory (tower.pt)")
    parser.add_argument("--v1-gaps", required=True, help="Arm V's V1 sealed gaps.npy")
    parser.add_argument("--v1-rows", action="append", required=True,
                        help="The rows files V1's gaps cover, in their order; --rows must be one of them")
    parser.add_argument("--out", required=True)
    parser.add_argument("--threads", type=int, default=16)
    parser.add_argument("--runtime", default="")
    args = parser.parse_args(argv)
    if args.runtime:
        import os
        os.environ["PATH"] = f"{Path(args.runtime).expanduser()}{os.pathsep}{os.environ.get('PATH', '')}"
    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists")
    rows = V.AdvantageRows.load(args.rows)
    # V1: its gaps cover the --v1-rows in order; this file's part.
    covered = [V.AdvantageRows.load(p) for p in args.v1_rows]
    gaps = np.load(args.v1_gaps)
    if len(gaps) != sum(len(p.cand) for p in covered):
        raise ValueError("V1's gaps do not cover the --v1-rows")
    at = [str(Path(p).resolve()) for p in args.v1_rows].index(str(Path(args.rows).resolve()))
    start = sum(len(p.cand) for p in covered[:at])
    v1 = per_root_max_abs(rows, gaps[start:start + len(rows.cand)])
    v1 = np.where(rows.held_out, v1, np.nan)
    saved = torch.load(Path(args.model) / "net.pt", map_location="cpu", weights_only=False)["state_dict"]
    margin = policy_margins(rows, saved["cand_scorer.3.weight"].float().numpy(), saved["cand_scorer.3.bias"].float().numpy())
    b2, b2_meta = b2_gaps(args.rows, args.state_dir, args.model, args.b2, args.threads)
    with out.open("w") as handle:
        for r in range(len(rows.state)):
            value = lambda x: None if not np.isfinite(x) else float(x)
            handle.write(json.dumps(dict(game=int(rows.roots["game"][r]), seat=int(rows.roots["seat"][r]),
                                         root_cursor=int(rows.roots["cursor"][r]), held_out=bool(rows.held_out[r]),
                                         own_margin=value(margin[r]), policy_margin=value(-margin[r]),
                                         b2_gap=value(b2[r]), v1_gap=value(v1[r]))) + "\n")
    meta = dict(rows=args.rows, b2=b2_meta, v1_gaps=args.v1_gaps, roots=len(rows.state),
                held_out=int(rows.held_out.sum()))
    Path(str(out) + ".json").write_text(json.dumps(meta, indent=1))
    print("[livegate] " + json.dumps(meta), flush=True)


if __name__ == "__main__":
    main()
