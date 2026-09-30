"""Encoder v3: imitate the house and rate the table, in one net.

    uv run python train3.py --data ~/baylee-data/datasets/d3-r003 --out ~/baylee-data/models/v3-a

Each sample is one step of an answer (see `train_policy.py`). The loss is the
policy's cross-entropy over the offered options, where the house's answer was
read, plus `--value-weight` times the value's: a cross-entropy of the
distribution over the seats against who won (the winners share it; in a
draw, the seats still in). The value is trained on the first step of each
decision only, as v2's was: later steps show the same position again.

The checkpoint kept is the one with the lowest held-out loss. The report
gives, on held-out games, the policy's agreement (by question kind and house
profile) and the value's calibration for the deciding seat's side (by seat
count and turn band), in the same measures as v2's reports.
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
from train_value import metrics

ROW_HEADS_A = (M.H_ENTITY, M.H_ABILITY, M.H_PAIR, M.H_ATTACK_PLAYER)
BANDS = [(1, 3), (4, 6), (7, 9), (10, 14), (15, 10_000)]


def log(msg: str) -> None:
    print(f"[v3] {msg}", flush=True)


def gather_options(ds: D3.Dataset, idx: np.ndarray, entities: int):
    start = ds.opt_off[idx]
    count = ds.opt_off[idx + 1] - start
    total = int(count.sum())
    sample = np.repeat(np.arange(len(idx)), count)
    first = np.concatenate([[0], np.cumsum(count)[:-1]])
    flat = np.repeat(start - first, count) + np.arange(total)
    opt = ds.opt[flat]
    live = np.ones(total, dtype=bool)
    live &= ~(np.isin(opt[:, 0], ROW_HEADS_A) & (opt[:, 1] >= entities))
    live &= ~((opt[:, 0] == M.H_PAIR) & (opt[:, 2] >= entities))
    chosen = ds.col("chosen")[idx]
    flat_chosen = np.where(chosen >= 0, first + chosen, -1)
    return opt, sample, live, flat_chosen, count, first


def run(net, ds, b, entities):
    """Policy logits padded per sample, the chosen index, which samples the
    policy learns from, value logits, and per-sample policy hits."""
    opt, sample, live, flat_chosen, count, first = gather_options(ds, b.idx, entities)
    dev = b.cards.device
    profile = ds.col("profile")[b.idx]
    profile = np.where(profile < 0, 5, profile)
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
    top = padded.argmax(1).cpu().numpy()
    hits = ok & (top == chosen)
    return padded, torch.from_numpy(np.where(ok, chosen, 0)).to(dev), torch.from_numpy(ok).to(dev), value, hits


def losses(net, ds, b, entities):
    padded, chosen, ok, value, hits = run(net, ds, b, entities)
    policy = F.cross_entropy(padded[ok], chosen[ok]) if ok.any() else padded.sum() * 0
    logp = torch.log_softmax(value, dim=1)
    t = b.value_target
    per = -torch.where(t > 0, t * logp, torch.zeros_like(logp)).sum(1)
    first = torch.from_numpy(ds.col("step")[b.idx] == 0).to(value.device)
    val = per[first].mean() if first.any() else per.sum() * 0
    return policy, val, value, hits, ok


@torch.no_grad()
def evaluate(net, ds, idx, entities, value_weight, batch=512):
    """Held-out loss, policy hits and the side's win chance per sample."""
    net.eval()
    hits = np.zeros(len(idx), dtype=bool)
    usable = np.zeros(len(idx), dtype=bool)
    side = np.zeros(len(idx), dtype=np.float64)
    total, n = 0.0, 0
    at = 0
    with torch.autocast("cuda", dtype=torch.bfloat16):
        for b in D3.Loader(ds, idx, batch, entities, shuffle=False):
            pol, val, value, h, ok = losses(net, ds, b, entities)
            k = len(b.idx)
            hits[at : at + k], usable[at : at + k] = h, ok.cpu().numpy()
            side[at : at + k] = M.side_chance(value, b.team).cpu().numpy()
            total += (pol.item() + value_weight * val.item()) * k
            n += k
            at += k
    net.train()
    return total / max(n, 1), hits, usable, side


def value_report(ds, idx, side):
    """Calibration of the side's win chance on the first step of each decision."""
    first = ds.col("step")[idx] == 0
    result = ds.col("result")[idx].astype(np.float64) / 2
    team = ds.col("team_rel")[idx]
    winners = ds.col("winners_rel")[idx]
    # The side won when a winner is on it; a draw is a half.
    y = np.where(winners == 0, 0.5, ((winners & team) != 0).astype(np.float64))
    y = np.where(ds.col("seats")[idx] == 2, result, y)
    rows = {"all": metrics(side[first], y[first])}
    seats = ds.col("seats")[idx]
    for s in sorted(set(seats[first].tolist())):
        m = first & (seats == s)
        if m.sum() >= 100:
            rows[f"{s} seats"] = metrics(side[m], y[m])
    turn = ds.col("turn")[idx]
    for lo, hi in BANDS:
        m = first & (turn >= lo) & (turn <= hi)
        if m.sum() >= 100:
            rows[f"turns {lo}-{hi if hi < 10_000 else '+'}"] = metrics(side[m], y[m])
    return rows


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
    ap.add_argument("--id-dropout", type=float, default=0.4)
    ap.add_argument("--wd", type=float, default=0.05)
    ap.add_argument("--value-weight", type=float, default=1.0)
    ap.add_argument("--eval-every", type=int, default=2000)
    ap.add_argument("--eval-slice", type=int, default=30_000)
    ap.add_argument("--eval-max", type=int, default=300_000)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--init", help="start from this net.pt's weights (same shape)")
    ap.add_argument("--resume", action="store_true", help="continue the run in --out from its last.pt")
    ap.add_argument("--ckpt-minutes", type=float, default=30.0, help="how often last.pt is written")
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    torch.backends.cuda.matmul.allow_tf32 = True
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=args.resume)
    t0 = time.time()
    ds = D3.load(Path(args.data).expanduser())
    train_idx, held_idx = D3.split_by_game(ds)
    log(f"loaded {ds.n:,} steps in {time.time() - t0:.0f}s; train {len(train_idx):,}, held out {len(held_idx):,}; "
        f"offered objects dropped {int((ds.col('offered_dropped') > 0).sum())}")

    # Ids the net saw in training: the rest get the unseen vector at export.
    tr_games = set(ds.col("game")[train_idx].tolist())
    seen = np.zeros(ds.meta["ids"]["id_space"], dtype=bool)
    ent_game = np.repeat(ds.col("game"), np.diff(ds.ent_off))
    seen[np.unique(ds.ent_card[np.isin(ent_game, list(tr_games))])] = True
    deck_game = np.repeat(ds.col("game"), np.diff(ds.deck_off))
    seen[np.unique(ds.deck_card[np.isin(deck_game, list(tr_games))])] = True
    seen[0] = False
    np.save(out / "seen_ids.npy", seen)
    log(f"{int(seen.sum()):,} card ids seen in training")

    net = M.build(ds.meta, ds.card_ids, ds.card_feats, d=args.d, layers=args.layers, heads=args.heads,
                  ff=args.ff, dropout=args.dropout, id_dropout=args.id_dropout).cuda()
    params = sum(p.numel() for p in net.parameters())
    log(f"{params / 1e6:.2f}M parameters")
    if args.init:
        net.load_state_dict(torch.load(Path(args.init).expanduser(), weights_only=False)["state"])
        log(f"weights from {args.init}")
    opt = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.wd, fused=True)
    loader = D3.Loader(ds, train_idx, args.batch, args.entities, shuffle=True, seed=args.seed)
    total = max(int(len(loader) * args.epochs), 1)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=total, pct_start=0.03)
    watch = np.sort(np.random.default_rng(2).choice(held_idx, size=min(args.eval_slice, len(held_idx)), replace=False))
    best, curve = {"loss": float("inf"), "step": 0}, []
    step, run_p, run_v = 0, None, None
    last = out / "last.pt"
    if args.resume and last.exists():
        ck = torch.load(last, weights_only=False)
        net.load_state_dict(ck["state"])
        opt.load_state_dict(ck["opt"])
        sched.load_state_dict(ck["sched"])
        step, best, curve, run_p, run_v = ck["step"], ck["best"], ck["curve"], ck["run_p"], ck["run_v"]
        loader.start = step
        log(f"resumed at step {step:,}")
    log(f"{total:,} steps of {args.batch}")

    def checkpoint() -> None:
        tmp = out / "last.pt.tmp"
        torch.save({"state": net.state_dict(), "opt": opt.state_dict(), "sched": sched.state_dict(), "step": step,
                    "best": best, "curve": curve, "run_p": run_p, "run_v": run_v}, tmp)
        tmp.replace(last)

    t_start, t_ckpt, step0 = time.time(), time.time(), step
    while step < total:
        for b in loader:
            with torch.autocast("cuda", dtype=torch.bfloat16):
                pol, val, _, _, _ = losses(net, ds, b, args.entities)
                loss = pol + args.value_weight * val
            opt.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            sched.step()
            step += 1
            run_p = pol.item() if run_p is None else 0.98 * run_p + 0.02 * pol.item()
            run_v = val.item() if run_v is None else 0.98 * run_v + 0.02 * val.item()
            if step % 500 == 0 or step == total:
                el = time.time() - t_start
                done = max(step - step0, 1)
                log(f"step {step:,}/{total:,} · policy {run_p:.4f} · value {run_v:.4f} · "
                    f"{done * args.batch / el:,.0f} steps/s · eta {(total - step) * el / done / 60:.0f} min")
            if step % args.eval_every == 0 or step == total:
                held, hits, usable, side = evaluate(net, ds, watch, args.entities, args.value_weight)
                agree = float(hits[usable].mean()) if usable.any() else float("nan")
                brier = value_report(ds, watch, side)["all"]["brier"]
                curve.append({"step": step, "held_loss": held, "agreement": agree, "brier": brier})
                mark = ""
                if held < best["loss"]:
                    best = {"loss": held, "step": step}
                    torch.save(net.state_dict(), out / "best.state")
                    mark = " · best"
                log(f"held out @ {step:,}: loss {held:.4f} · agreement {agree:.4f} · value Brier {brier:.4f}{mark}")
            if time.time() - t_ckpt > args.ckpt_minutes * 60:
                checkpoint()
                t_ckpt = time.time()
            if step >= total:
                break

    net.load_state_dict(torch.load(out / "best.state"))
    torch.save({"state": net.state_dict(), "config": net.cfg.to_dict(), "glob": D3.glob_layout(ds.meta),
                "dataset": ds.meta["sources"], "encoder_version": ds.meta["encoder_version"],
                "walk_version": ds.meta["walk_version"], "verification": ds.meta.get("verification"),
                "step": best["step"]}, out / "net.pt")
    ev = np.sort(np.random.default_rng(1).choice(held_idx, size=min(args.eval_max, len(held_idx)), replace=False))
    log(f"scoring {len(ev):,} held-out steps")
    held, hits, usable, side = evaluate(net, ds, ev, args.entities, args.value_weight)
    kinds = ds.meta["pending_kinds"]
    kind, prof, nopt = ds.col("pending_kind")[ev], ds.col("profile")[ev], ds.col("n_opts")[ev]
    by_kind = {kinds[k]: {"n": int((usable & (kind == k)).sum()), "agreement": float(hits[usable & (kind == k)].mean())}
               for k in sorted(set(kind.tolist())) if (usable & (kind == k)).sum() >= 100}
    by_profile = {ds.meta["profiles"][p]: {"n": int((usable & (prof == p)).sum()),
                                           "agreement": float(hits[usable & (prof == p)].mean())}
                  for p in sorted(set(prof.tolist())) if p >= 0 and (usable & (prof == p)).sum() >= 100}
    value = value_report(ds, ev, side)
    rep = {
        "model": out.name, "params": params, "best_step": best["step"], "held_loss": held, "curve": curve,
        "held_out_steps": int(usable.sum()), "agreement": float(hits[usable].mean()),
        "uniform_chance": float(np.mean(1.0 / np.maximum(nopt[usable], 1))),
        "by_kind": by_kind, "by_profile": by_profile, "value": value,
        "verification": ds.meta.get("verification"),
    }
    (out / "report.json").write_text(json.dumps(rep, indent=1))
    lines = [f"# v3 report — {out.name}", "",
             f"Policy: top-1 agreement with the house on {int(usable.sum()):,} held-out answer steps: "
             f"**{rep['agreement']:.3f}** (uniform pick {rep['uniform_chance']:.3f}). Model: step {best['step']:,}.", "",
             "| question | steps | agreement |", "|---|---|---|"]
    lines += [f"| {k} | {v['n']:,} | {v['agreement']:.3f} |" for k, v in by_kind.items()]
    lines += ["", "| house profile | steps | agreement |", "|---|---|---|"]
    lines += [f"| {k} | {v['n']:,} | {v['agreement']:.3f} |" for k, v in by_profile.items()]
    lines += ["", "Value (the deciding seat's side wins):", "", "| slice | n | Brier | log loss | accuracy | ECE |", "|---|---|---|---|---|---|"]
    lines += [f"| {k} | {v['n']:,} | {v['brier']:.4f} | {v['logloss']:.4f} | {v['accuracy']:.3f} | {v['ece']:.4f} |" for k, v in value.items()]
    (out / "report.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines), flush=True)


if __name__ == "__main__":
    main()
