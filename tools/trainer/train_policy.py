"""Stage 5, first half: imitate the house AI's answers.

    uv run python train_policy.py --data ~/baylee-data/datasets/d003 --out ~/baylee-data/models/policy-v1

Each sample is one step of an answer (encoder v2): the options its question
offered as `(head, a, b)` triples and the one the house picked. The net
scores exactly those options, so it cannot pick an illegal one. The first
number to report is agreement: how often its top option is the house's, on
held-out games, by question kind and by house profile. Imitating five
profiles through one net has a ceiling below 100 %; the profile is an input.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

import dataset as D
import model as M

ROW_HEADS_A = (M.H_ENTITY, M.H_ABILITY, M.H_PAIR, M.H_ATTACK_PLAYER)


def log(msg: str) -> None:
    print(f"[policy] {msg}", flush=True)


def gather_options(ds: D.Dataset, idx: np.ndarray, entities: int):
    """Flattened options of the samples `idx`, which sample each belongs to,
    whether each can be scored (its rows survived truncation to `entities`),
    and per sample the flat index of the chosen option."""
    start = ds.opt_off[idx]
    count = ds.opt_off[idx + 1] - start
    total = int(count.sum())
    sample = np.repeat(np.arange(len(idx)), count)
    first = np.concatenate([[0], np.cumsum(count)[:-1]])
    flat = np.repeat(start - first, count) + np.arange(total)
    opt = ds.opt[flat]
    live = np.ones(total, dtype=bool)
    rows_a = np.isin(opt[:, 0], ROW_HEADS_A)
    live &= ~(rows_a & (opt[:, 1] >= entities))
    live &= ~((opt[:, 0] == M.H_PAIR) & (opt[:, 2] >= entities))
    chosen = ds.col("chosen")[idx]
    flat_chosen = np.where(chosen >= 0, first + chosen, -1)
    return opt, sample, live, flat_chosen, count


def evaluate(net, ds, idx, entities, batch=512):
    """Top-1 agreement with the house, per sample."""
    net.eval()
    hits = np.zeros(len(idx), dtype=bool)
    at = 0
    with torch.inference_mode(), torch.autocast("cuda", dtype=torch.bfloat16):
        for b in D.Loader(ds, idx, batch, entities, shuffle=False):
            n = len(b.idx)
            hits[at : at + n] = step_scores(net, ds, b, entities)[1]
            at += n
    net.train()
    return hits


def step_scores(net, ds, b, entities):
    opt, sample, live, flat_chosen, count = gather_options(ds, b.idx, entities)
    dev = b.cards.device
    profile = ds.col("profile")[b.idx]
    profile = np.where(profile < 0, 5, profile)
    logit, value = net(
        b,
        torch.from_numpy(profile).to(dev),
        torch.from_numpy(sample).to(dev),
        torch.from_numpy(opt.astype(np.int64)).to(dev),
    )
    logit = logit.float().masked_fill(~torch.from_numpy(live).to(dev), float("-inf"))
    # Per-sample softmax over a flat list: pad to (B, max options).
    B, O = len(b.idx), int(count.max()) if len(count) else 0
    pos = np.arange(len(sample)) - np.repeat(np.concatenate([[0], np.cumsum(count)[:-1]]), count)
    padded = torch.full((B, max(O, 1)), float("-inf"), device=dev)
    padded[torch.from_numpy(sample).to(dev), torch.from_numpy(pos).to(dev)] = logit
    chosen = flat_chosen - np.concatenate([[0], np.cumsum(count)[:-1]])
    ok = (flat_chosen >= 0) & (count > 1)
    ok &= np.array([live[f] if f >= 0 else False for f in flat_chosen])
    top = padded.argmax(1).cpu().numpy()
    hits = ok & (top == chosen)
    return (padded, torch.from_numpy(np.where(ok, chosen, 0)).to(dev), torch.from_numpy(ok).to(dev), value), hits


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=float, default=1.0)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--d", type=int, default=256)
    ap.add_argument("--layers", type=int, default=4)
    ap.add_argument("--heads", type=int, default=8)
    ap.add_argument("--ff", type=int, default=1024)
    ap.add_argument("--dropout", type=float, default=0.1)
    ap.add_argument("--wd", type=float, default=0.05)
    ap.add_argument("--eval-every", type=int, default=2000)
    ap.add_argument("--eval-slice", type=int, default=30_000)
    ap.add_argument("--eval-max", type=int, default=300_000)
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    torch.backends.cuda.matmul.allow_tf32 = True
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=False)
    t0 = time.time()
    ds = D.load(Path(args.data).expanduser())
    if ds.opt is None:
        raise SystemExit("this dataset has no options: convert it with encoder v2")
    train_idx, held_idx = D.split_by_game(ds)
    # Only steps the house's answer was read for, that had a choice, and whose
    # every offered object has a row. The last fails only on boards of
    # hundreds of tokens, where a question offers more objects than a
    # decision has rows; folding identical tokens into one row is the fix.
    offered_dropped = ds.col("offered_dropped") > 0
    log(f"{offered_dropped.sum():,} steps lost offered objects to the row cap "
        f"(in {len(set(ds.col('game')[offered_dropped].tolist())):,} games); left out")
    usable = (ds.col("chosen") >= 0) & (ds.col("n_opts") > 1) & ~offered_dropped
    train_idx, held_idx = train_idx[usable[train_idx]], held_idx[usable[held_idx]]
    log(f"loaded {ds.n:,} steps in {time.time() - t0:.0f}s; train {len(train_idx):,}, held out {len(held_idx):,}")

    net = M.build_policy(ds.meta, d=args.d, layers=args.layers, heads=args.heads, ff=args.ff, dropout=args.dropout).cuda()
    params = sum(p.numel() for p in net.parameters())
    log(f"{params / 1e6:.2f}M parameters")
    opt = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.wd, fused=True)
    loader = D.Loader(ds, train_idx, args.batch, args.entities, shuffle=True, seed=args.seed)
    total = int(len(loader) * args.epochs)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=total, pct_start=0.03)
    watch = np.sort(np.random.default_rng(2).choice(held_idx, size=min(args.eval_slice, len(held_idx)), replace=False))
    best, curve = {"agreement": -1.0, "step": 0}, []
    log(f"{total:,} steps of {args.batch}")

    step, t_start, run_loss = 0, time.time(), None
    while step < total:
        for b in loader:
            with torch.autocast("cuda", dtype=torch.bfloat16):
                (padded, chosen, ok, value), _ = step_scores(net, ds, b, args.entities)
            loss = F.cross_entropy(padded[ok], chosen[ok])
            opt.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            sched.step()
            step += 1
            run_loss = loss.item() if run_loss is None else 0.98 * run_loss + 0.02 * loss.item()
            if step % 500 == 0 or step == total:
                el = time.time() - t_start
                log(f"step {step:,}/{total:,} · loss {run_loss:.4f} · {step * args.batch / el:,.0f} steps/s · "
                    f"eta {(total - step) * el / step / 60:.0f} min")
            if step % args.eval_every == 0 or step == total:
                agree = float(evaluate(net, ds, watch, args.entities).mean())
                curve.append({"step": step, "train_loss": run_loss, "agreement": agree})
                mark = ""
                if agree > best["agreement"]:
                    best = {"agreement": agree, "step": step}
                    torch.save(net.state_dict(), out / "policy_best.state")
                    mark = " · best"
                log(f"held out @ {step:,}: agreement {agree:.4f}{mark}")
            if step >= total:
                break

    net.load_state_dict(torch.load(out / "policy_best.state"))
    torch.save({"state": net.state_dict(), "config": net.trunk.cfg.to_dict(), "dataset": ds.meta["sources"],
                "encoder_version": ds.meta["encoder_version"], "step": best["step"]}, out / "policy.pt")
    ev = np.sort(np.random.default_rng(1).choice(held_idx, size=min(args.eval_max, len(held_idx)), replace=False))
    log(f"scoring {len(ev):,} held-out steps")
    hits = evaluate(net, ds, ev, args.entities)
    kinds = ds.meta["pending_kinds"]
    kind = ds.col("pending_kind")[ev]
    prof = ds.col("profile")[ev]
    nopt = ds.col("n_opts")[ev]
    by_kind = {kinds[k]: {"n": int((kind == k).sum()), "agreement": float(hits[kind == k].mean())}
               for k in sorted(set(kind.tolist())) if (kind == k).sum() >= 100}
    by_profile = {ds.meta["profiles"][p]: {"n": int((prof == p).sum()), "agreement": float(hits[prof == p].mean())}
                  for p in sorted(set(prof.tolist())) if p >= 0}
    # What a uniform pick over the offered options would agree on.
    chance = float(np.mean(1.0 / nopt))
    rep = {
        "model": out.name, "params": params, "best_step": best["step"], "curve": curve,
        "held_out_steps": int(len(ev)), "agreement": float(hits.mean()), "uniform_chance": chance,
        "by_kind": by_kind, "by_profile": by_profile,
        "unmatched_answers": ds.meta.get("unmatched_answers"),
        "offered_objects_dropped": ds.meta.get("offered_objects_dropped"),
    }
    (out / "report.json").write_text(json.dumps(rep, indent=1))
    lines = [f"# Policy report — {out.name}", "",
             f"Top-1 agreement with the house on {len(ev):,} held-out answer steps: **{rep['agreement']:.3f}** "
             f"(a uniform pick among the offered options: {chance:.3f}). Reported model: step {best['step']:,}.", "",
             "| question | steps | agreement |", "|---|---|---|"]
    lines += [f"| {k} | {v['n']:,} | {v['agreement']:.3f} |" for k, v in by_kind.items()]
    lines += ["", "| house profile | steps | agreement |", "|---|---|---|"]
    lines += [f"| {k} | {v['n']:,} | {v['agreement']:.3f} |" for k, v in by_profile.items()]
    (out / "report.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines), flush=True)


if __name__ == "__main__":
    main()
