"""Reinforcement learning from league games: advantage-weighted regression.

    uv run python train_rl.py --data ~/baylee-data/datasets/d3-l001 \\
        --init ~/baylee-data/models/v3-b/net.pt --out ~/baylee-data/models/rl-001

A league run (`bin/league`) has the learner sample its answers against a
league of house profiles, frozen nets and itself; `convert3` turns it into a
v3 dataset. Here the learner's own decisions (the seat `games.jsonl` names
`learner`) are imitated, each weighted by exp(A / beta), clipped at
`--w-max` (AWR, Peng et al. 2019): answers that did better than expected are
made likelier, worse ones less likely. A is a generalised advantage (GAE,
lambda `--lam`) over the learner's own decisions in a game: how much its
value head's estimate of its side rose from one decision to its next, the
last step ending at the result. The game's result alone, over hundreds of
decisions, credits every one of them with the same luck; the value's own
steps say which decision moved it. A is standardised, so beta is in standard
deviations. The
value head learns from every decision of every seat. The learner plays as the
expert profile (4), so its RL policy is what that profile slot answers.

One run is one policy-improvement step; `rl_loop.sh` alternates league games
and these steps. `last.pt` is written every `--ckpt-minutes` and `--resume`
continues from it.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

import dataset3 as D3
import model3 as M
from train3 import gather_options, value_report

EXPERT = 4


def log(msg: str) -> None:
    print(f"[rl] {msg}", flush=True)


def learner_mask(ds: D3.Dataset) -> np.ndarray:
    """Per sample: whether its seat was the learner's."""
    names = [g.get("profiles") or [] for g in ds.games]
    game, seat = ds.col("game"), ds.col("seat")
    learner = np.array([[s in ("learner", "self") for s in (p + [""] * 8)[:8]] for p in names], dtype=bool)
    return learner[game, seat]


def advantages(net, ds, is_learner, entities, lam):
    """Per sample, the standardised GAE advantage of the learner's decision
    it is a step of (NaN for other seats), from `net`'s value."""
    from train3 import evaluate

    game, seat, n, step_ = ds.col("game"), ds.col("seat"), ds.col("n"), ds.col("step")
    first = np.nonzero(is_learner & (step_ == 0))[0]
    first = first[np.lexsort((n[first], seat[first], game[first]))]
    _, _, _, value = evaluate(net, ds, first, entities, 1.0)
    winners, team = ds.col("winners_rel")[first], ds.col("team_rel")[first]
    result = np.where(winners == 0, 0.5, ((winners & team) != 0).astype(np.float64))
    same_next = np.zeros(len(first), dtype=bool)
    same_next[:-1] = (game[first][1:] == game[first][:-1]) & (seat[first][1:] == seat[first][:-1])
    nxt = np.where(same_next, np.roll(value, -1), result)
    delta = nxt - value
    adv = np.zeros(len(first))
    run = 0.0
    for i in range(len(first) - 1, -1, -1):
        run = delta[i] + (lam * run if same_next[i] else 0.0)
        adv[i] = run
    adv = (adv - adv.mean()) / max(adv.std(), 1e-6)
    # Every step of a decision shares its advantage.
    key = (game.astype(np.int64) * 8 + seat) * (1 << 32) + n
    order = np.argsort(key[first])
    sorted_keys = key[first][order]
    out = np.full(ds.n, np.nan)
    learner = np.nonzero(is_learner)[0]
    at = np.searchsorted(sorted_keys, key[learner])
    at = np.clip(at, 0, len(sorted_keys) - 1)
    found = sorted_keys[at] == key[learner]
    out[learner[found]] = adv[order[at[found]]]
    log(f"advantages of {len(first):,} decisions (lambda {lam}): value mean {value.mean():.3f}, "
        f"raw delta std {delta.std():.3f}")
    return out


def step(net, ds, b, entities, is_learner, adv, beta, w_max):
    """Weighted policy loss on the learner's samples, value loss on all
    first steps, and the mean weight."""
    opt, sample, live, flat_chosen, count, first = gather_options(ds, b.idx, entities)
    dev = b.cards.device
    learner = torch.from_numpy(is_learner[b.idx]).to(dev)
    profile = np.where(is_learner[b.idx], EXPERT, np.where(ds.col("profile")[b.idx] < 0, 5, ds.col("profile")[b.idx]))
    logit, value = net(b, torch.from_numpy(profile).to(dev), torch.from_numpy(sample).to(dev),
                       torch.from_numpy(opt.astype(np.int64)).to(dev))
    logit = logit.float().masked_fill(~torch.from_numpy(live).to(dev), float("-inf"))
    B, O = len(b.idx), int(count.max()) if len(count) else 0
    pos = np.arange(len(sample)) - np.repeat(first, count)
    padded = torch.full((B, max(O, 1)), float("-inf"), device=dev)
    padded[torch.from_numpy(sample).to(dev), torch.from_numpy(pos).to(dev)] = logit
    chosen = flat_chosen - first
    ok = (flat_chosen >= 0) & (count > 1) & (ds.col("offered_dropped")[b.idx] == 0)
    ok &= np.array([live[f] if f >= 0 else False for f in flat_chosen])
    a = adv[b.idx]
    ok &= ~np.isnan(a)
    ok_t = torch.from_numpy(ok).to(dev) & learner
    weight = torch.clamp(torch.exp(torch.from_numpy(np.nan_to_num(a) / beta).to(dev).float()), max=w_max)
    if ok_t.any():
        ce = F.cross_entropy(padded[ok_t], torch.from_numpy(np.where(ok, chosen, 0)).to(dev)[ok_t], reduction="none")
        policy = (ce * weight[ok_t]).sum() / weight[ok_t].sum().clamp(min=1e-6)
        mean_w = float(weight[ok_t].mean())
    else:
        policy, mean_w = padded.sum() * 0, 0.0
    logp = torch.log_softmax(value, dim=1)
    t = b.value_target
    per = -torch.where(t > 0, t * logp, torch.zeros_like(logp)).sum(1)
    first_step = torch.from_numpy(ds.col("step")[b.idx] == 0).to(dev)
    val = per[first_step].mean() if first_step.any() else per.sum() * 0
    return policy, val, mean_w


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--init", required=True, help="the learner that played: its net.pt")
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=float, default=1.0)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--lr", type=float, default=5e-5)
    ap.add_argument("--wd", type=float, default=0.01)
    ap.add_argument("--beta", type=float, default=1.0, help="AWR temperature, in standard deviations of the advantage")
    ap.add_argument("--lam", type=float, default=0.9, help="GAE lambda over the learner's decisions")
    ap.add_argument("--w-max", type=float, default=20.0)
    ap.add_argument("--value-weight", type=float, default=1.0)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--ckpt-minutes", type=float, default=30.0)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    torch.backends.cuda.matmul.allow_tf32 = True
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=args.resume)
    ds = D3.load(Path(args.data).expanduser())
    is_learner = learner_mask(ds)
    train_idx, held_idx = D3.split_by_game(ds)
    log(f"{ds.n:,} decisions, {int(is_learner.sum()):,} of them the learner's; train {len(train_idx):,}, held out {len(held_idx):,}")
    base = torch.load(Path(args.init).expanduser(), weights_only=False)
    skip = ("id_space", "glob_numeric", "card_width", "seat_cols", "max_seats")
    cfg = {k: v for k, v in base["config"].items() if k not in skip}
    net = M.build(ds.meta, ds.card_ids, ds.card_feats, **cfg).cuda()
    net.load_state_dict(base["state"])
    seen_src = Path(args.init).expanduser().parent / "seen_ids.npy"
    if seen_src.exists():
        # The ids this line of nets has met: the base's and this data's.
        seen = np.load(seen_src)
        seen[np.unique(ds.ent_card)] = True
        seen[np.unique(ds.deck_card)] = True
        seen[0] = False
        np.save(out / "seen_ids.npy", seen)
    opt = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.wd, fused=True)
    loader = D3.Loader(ds, train_idx, args.batch, args.entities, shuffle=True, seed=args.seed)
    total = max(int(len(loader) * args.epochs), 1)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=total, pct_start=0.05)
    n = 0
    last = out / "last.pt"
    if args.resume and last.exists():
        ck = torch.load(last, weights_only=False)
        net.load_state_dict(ck["state"])
        opt.load_state_dict(ck["opt"])
        sched.load_state_dict(ck["sched"])
        n = ck["step"]
        loader.start = n
        log(f"resumed at step {n:,}")
    log(f"{total:,} steps of {args.batch} from {args.init}")
    adv = advantages(net, ds, is_learner, args.entities, args.lam)
    t0, t_ckpt, run_p, run_v, run_w = time.time(), time.time(), None, None, None
    while n < total:
        for b in loader:
            with torch.autocast("cuda", dtype=torch.bfloat16):
                pol, val, mean_w = step(net, ds, b, args.entities, is_learner, adv, args.beta, args.w_max)
                loss = pol + args.value_weight * val
            opt.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            sched.step()
            n += 1
            run_p = pol.item() if run_p is None else 0.98 * run_p + 0.02 * pol.item()
            run_v = val.item() if run_v is None else 0.98 * run_v + 0.02 * val.item()
            run_w = mean_w if run_w is None else 0.98 * run_w + 0.02 * mean_w
            if n % 500 == 0 or n == total:
                log(f"step {n:,}/{total:,} · weighted policy {run_p:.4f} · value {run_v:.4f} · mean weight {run_w:.2f} · "
                    f"{(time.time() - t0) / 60:.0f} min")
            if time.time() - t_ckpt > args.ckpt_minutes * 60:
                tmp = out / "last.pt.tmp"
                torch.save({"state": net.state_dict(), "opt": opt.state_dict(), "sched": sched.state_dict(), "step": n}, tmp)
                tmp.replace(last)
                t_ckpt = time.time()
            if n >= total:
                break
    torch.save({"state": net.state_dict(), "config": base["config"], "glob": D3.glob_layout(ds.meta),
                "dataset": ds.meta["sources"], "encoder_version": ds.meta["encoder_version"],
                "walk_version": ds.meta["walk_version"], "verification": ds.meta.get("verification"),
                "rl": {"from": args.init, "beta": args.beta, "lam": args.lam, "w_max": args.w_max, "lr": args.lr,
                       "steps": n, "advantage": "gae over the value head, standardised"}},
               out / "net.pt")
    # The value on held-out games, as the other trainers report it.
    from train3 import evaluate

    ev = held_idx[: min(len(held_idx), 200_000)]
    _, _, _, side = evaluate(net, ds, ev, args.entities, args.value_weight)
    value = value_report(ds, ev, side)
    (out / "report.json").write_text(json.dumps({"model": out.name, "from": args.init, "steps": n, "value": value,
                                                  "verification": ds.meta.get("verification")}, indent=1))
    log(f"done: value Brier {value['all']['brier']:.4f} on {value['all']['n']:,} held-out decisions")


if __name__ == "__main__":
    main()
