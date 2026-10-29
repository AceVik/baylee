"""How far RL steps moved the policy: nets compared on the same decisions.

    uv run python eval_shift.py --data ~/baylee-data/datasets/d3-rl-l03 \\
        --models ~/baylee-data/models/v3-a-dyn ~/baylee-data/models/rl-0{1,2,3}

On held-out learner decisions with a real choice (two or more live options),
prints per pair of nets the share of decisions whose top option agrees and
the mean KL divergence of the second from the first, and per net how often
its top option is the answer the learner sampled, its mean top probability
and its mean entropy (rising top probability: the policy sharpens).

With `--player N` (the index in `--models` of the net that played the data),
also the mean standardised advantage `train_rl` gives the decisions where the
learner sampled that net's top option and where it sampled another: whether
the data says deviating did better or worse than expected.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import torch

import dataset3 as D3
import model3 as M
from train3 import gather_options
from train_rl import EXPERT, advantages, learner_mask
from legacy_evaluation import native_metrics


def load(model: Path, ds) -> torch.nn.Module:
    base = torch.load(model / "net.pt", weights_only=False)
    skip = ("id_space", "glob_numeric", "card_width", "seat_cols", "max_seats")
    cfg = {k: v for k, v in base["config"].items() if k not in skip}
    net = M.build(ds.meta, ds.card_ids, ds.card_feats, **cfg).cuda()
    net.load_state_dict(base["state"])
    net.eval()
    return net


def log_probs(net, ds, b, entities, is_learner):
    """Per sample of `b`, log-probabilities over its options (padded with
    -inf), the chosen option, and whether the sample is a usable choice."""
    opt, sample, live, flat_chosen, count, first = gather_options(ds, b.idx, entities)
    dev = b.cards.device
    learner = torch.from_numpy(is_learner[b.idx]).to(dev)
    profiles = torch.from_numpy(ds.col('profile')[b.idx]).to(dev)
    profile = torch.where(learner, EXPERT, torch.where(profiles < 0, 5, profiles))
    sample_t = torch.from_numpy(sample).to(dev)
    first_t = torch.from_numpy(first).to(dev)
    count_t = torch.from_numpy(count).to(dev)
    logit, _ = net(b, profile, sample_t, torch.from_numpy(opt.astype(np.int64)).to(dev))
    live_t = torch.from_numpy(live).to(dev)
    logit = logit.float().masked_fill(~live_t, float('-inf'))
    B = len(b.idx)
    O = max(int(count_t.max()) if len(count) else 0, 1)
    pos = torch.arange(len(sample), device=dev) - torch.repeat_interleave(first_t, count_t)
    padded = torch.full((B, O), float('-inf'), device=dev)
    padded[sample_t, pos] = logit
    live_count = torch.zeros(B, dtype=torch.long, device=dev)
    live_count.scatter_add_(0, sample_t, live_t.long())
    chosen = torch.from_numpy(flat_chosen).to(dev)
    dropped = torch.from_numpy(ds.col('offered_dropped')[b.idx]).to(dev)
    ok = (chosen >= 0) & (live_count > 1) & (dropped == 0) & learner
    return torch.log_softmax(padded, dim=1), (chosen - first_t).cpu().numpy(), ok.cpu().numpy()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--models", nargs="+", required=True)
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--limit", type=int, default=60_000)
    ap.add_argument("--player", type=int, default=None)
    ap.add_argument("--lam", type=float, default=0.9)
    args = ap.parse_args()
    ds = D3.load(Path(args.data).expanduser())
    is_learner = learner_mask(ds)
    _, held = D3.split_by_game(ds)
    held = held[is_learner[held]][: args.limit]
    models = [Path(m).expanduser() for m in args.models]
    nets = [load(m, ds) for m in models]
    k = len(nets)
    adv = None
    if args.player is not None:
        adv = advantages(nets[args.player], ds, is_learner, args.entities, args.lam)
        nets[args.player].eval()
    measurements = []
    with torch.no_grad(), torch.autocast("cuda", dtype=torch.bfloat16):
        for b in D3.Loader(ds, held, 512, args.entities, shuffle=False):
            outs = [log_probs(net, ds, b, args.entities, is_learner) for net in nets]
            ok = torch.from_numpy(outs[0][2]).to(b.cards.device)
            if not ok.any():
                continue
            lps = [o[0][ok] for o in outs]
            chosen = torch.from_numpy(outs[0][1]).to(b.cards.device)[ok]
            tops = [lp.argmax(1) for lp in lps]
            row = dict(n=int(ok.sum()), top_p=[], entropy=[], hit=[], agree=[], kl=[])
            if adv is not None:
                advantage = torch.from_numpy(adv[b.idx][outs[0][2]])
                row.update(advantage=torch.nan_to_num(advantage).tolist(),
                           advantage_finite=torch.isfinite(advantage).tolist(),
                           advantage_on_top=(tops[args.player] == chosen).cpu().tolist())
            for i in range(k):
                probability = lps[i].exp()
                row['top_p'].append(float(probability.max(1).values.sum()))
                row['entropy'].append(float(torch.where(probability > 0, -probability * lps[i],
                                                        torch.zeros_like(probability)).sum()))
                row['hit'].append(int((tops[i] == chosen).sum()))
                row['agree'].append([int((tops[i] == tops[j]).sum()) for j in range(k)])
                row['kl'].append([float(torch.where(probability > 0,
                    probability * (lps[i] - lps[j]), torch.zeros_like(probability)).sum(1).clamp(min=0).sum())
                    for j in range(k)])
            measurements.append(row)
    report = native_metrics('shift-report', names=[m.name for m in models],
                            player=args.player, batches=measurements)
    print(report['text'], end='')


if __name__ == "__main__":
    main()
