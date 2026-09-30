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
    profile = np.where(is_learner[b.idx], EXPERT, np.where(ds.col("profile")[b.idx] < 0, 5, ds.col("profile")[b.idx]))
    logit, _ = net(b, torch.from_numpy(profile).to(dev), torch.from_numpy(sample).to(dev),
                   torch.from_numpy(opt.astype(np.int64)).to(dev))
    live_t = torch.from_numpy(live).to(dev)
    logit = logit.float().masked_fill(~live_t, float("-inf"))
    B, O = len(b.idx), max(int(count.max()) if len(count) else 0, 1)
    pos = np.arange(len(sample)) - np.repeat(first, count)
    padded = torch.full((B, O), float("-inf"), device=dev)
    padded[torch.from_numpy(sample).to(dev), torch.from_numpy(pos).to(dev)] = logit
    live_count = np.zeros(B, dtype=np.int64)
    np.add.at(live_count, sample, live.astype(np.int64))
    ok = (flat_chosen >= 0) & (live_count > 1) & (ds.col("offered_dropped")[b.idx] == 0) & is_learner[b.idx]
    return torch.log_softmax(padded, dim=1), flat_chosen - first, ok


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
    top_p = np.zeros(k)
    entropy = np.zeros(k)
    adv_sum = {True: 0.0, False: 0.0}
    adv_n = {True: 0, False: 0}
    agree = np.zeros((k, k))
    kl = np.zeros((k, k))
    hit = np.zeros(k)
    n = 0
    with torch.no_grad(), torch.autocast("cuda", dtype=torch.bfloat16):
        for b in D3.Loader(ds, held, 512, args.entities, shuffle=False):
            outs = [log_probs(net, ds, b, args.entities, is_learner) for net in nets]
            ok = torch.from_numpy(outs[0][2]).to(b.cards.device)
            if not ok.any():
                continue
            lps = [o[0][ok] for o in outs]
            chosen = torch.from_numpy(outs[0][1]).to(b.cards.device)[ok]
            tops = [lp.argmax(1) for lp in lps]
            if adv is not None:
                a = adv[b.idx][outs[0][2]]
                on_top = (tops[args.player] == chosen).cpu().numpy()
                for flag in (True, False):
                    m = (on_top == flag) & ~np.isnan(a)
                    adv_sum[flag] += float(a[m].sum())
                    adv_n[flag] += int(m.sum())
            for i in range(k):
                p_i = lps[i].exp()
                top_p[i] += float(p_i.max(1).values.sum())
                entropy[i] += float(torch.where(p_i > 0, -p_i * lps[i], torch.zeros_like(p_i)).sum())
                hit[i] += float((tops[i] == chosen).sum())
                for j in range(k):
                    agree[i, j] += float((tops[i] == tops[j]).sum())
                    p = lps[i].exp()
                    d = torch.where(p > 0, p * (lps[i] - lps[j]), torch.zeros_like(p)).sum(1)
                    kl[i, j] += float(d.clamp(min=0).sum())
            n += int(ok.sum())
    names = [m.name for m in models]
    print(f"{n:,} held-out learner decisions with a choice")
    for i in range(k):
        print(f"{names[i]}: top option is the sampled answer {hit[i] / n:.3f}; "
              f"mean top probability {top_p[i] / n:.3f}, entropy {entropy[i] / n:.3f}")
    for i in range(k):
        for j in range(i + 1, k):
            print(f"{names[i]} -> {names[j]}: top agrees {agree[i, j] / n:.3f}, KL {kl[i, j] / n:.4f}")
    if adv is not None:
        for flag, what in ((True, "its top option"), (False, "another option")):
            mean = adv_sum[flag] / max(adv_n[flag], 1)
            print(f"sampled {what} of {names[args.player]}: {adv_n[flag]:,} decisions, mean advantage {mean:+.3f}")


if __name__ == "__main__":
    main()
