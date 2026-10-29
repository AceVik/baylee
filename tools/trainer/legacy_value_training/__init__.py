"""Stage 4: the value net — the win chance of the deciding seat, from its view.

    uv run python train_value.py --data ~/baylee-data/datasets/d002 --out ~/baylee-data/models/value-v1

Trained on house-AI self-play, so what it measures is "how often a house AI
in this seat, playing on, wins from here" — not a human's chances. The report
says so. Held-out games are whole games (every 10th), never single decisions.

The report is calibration first: of the positions rated p, about p should be
won. Brier score and log loss overall, a reliability table, and both split by
turn, by how far into its game a position is, and by the seat's house profile,
beside two baselines (a coin, and a logistic model on life, hand and board).
"""

from __future__ import annotations

import argparse
import json
import math
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

import dataset as D
import model as M


def log(msg: str) -> None:
    print(f"[value] {msg}", flush=True)


def predict(net: M.ValueNet, ds: D.Dataset, idx: np.ndarray, batch: int, entities: int) -> np.ndarray:
    net.eval()
    out = np.empty(len(idx), dtype=np.float32)
    at = 0
    with torch.inference_mode(), torch.autocast("cuda", dtype=torch.bfloat16):
        for b in D.Loader(ds, idx, batch, entities, shuffle=False):
            p = torch.sigmoid(net(b).float()).cpu().numpy()
            out[at : at + len(p)] = p
            at += len(p)
    net.train()
    return out


def metrics(p: np.ndarray, y: np.ndarray) -> dict:
    p = np.clip(p, 1e-6, 1 - 1e-6)
    brier = float(np.mean((p - y) ** 2))
    logloss = float(-np.mean(y * np.log(p) + (1 - y) * np.log(1 - p)))
    decided = y != 0.5
    acc = float(np.mean((p[decided] > 0.5) == (y[decided] > 0.5))) if decided.any() else float("nan")
    bins = np.minimum((p * 10).astype(int), 9)
    table, ece = [], 0.0
    for b in range(10):
        m = bins == b
        if not m.any():
            continue
        mp, my = float(p[m].mean()), float(y[m].mean())
        table.append({"bin": f"{b / 10:.1f}-{(b + 1) / 10:.1f}", "n": int(m.sum()), "predicted": mp, "won": my})
        ece += m.sum() / len(p) * abs(mp - my)
    return {"n": int(len(p)), "brier": brier, "logloss": logloss, "accuracy": acc, "ece": float(ece), "reliability": table}


def baseline_features(ds: D.Dataset, idx: np.ndarray) -> np.ndarray:
    """Life, hand and board differences, and the turn: what a person would
    guess from without reading a single card."""
    cols = ds.meta["glob_cols"]
    g = ds.glob[idx].astype(np.float32)
    life = g[:, cols.index("seat0_life")] - g[:, cols.index("seat1_life")]
    hand = g[:, cols.index("seat0_hand")] - g[:, cols.index("seat1_hand")]
    lib = g[:, cols.index("seat0_library")] - g[:, cols.index("seat1_library")]
    # Board sizes: battlefield rows by controller.
    zone = ds.meta["ent_cols"].index("zone")
    ctrl = ds.meta["ent_cols"].index("controller_rel")
    mine = np.zeros(len(idx), dtype=np.float32)
    theirs = np.zeros(len(idx), dtype=np.float32)
    for k, i in enumerate(idx):
        rows = ds.ent_feat[ds.ent_off[i] : ds.ent_off[i + 1]]
        bf = rows[rows[:, zone] == 3]
        mine[k] = (bf[:, ctrl] == 0).sum()
        theirs[k] = (bf[:, ctrl] != 0).sum()
    turn = g[:, cols.index("turn")]
    return np.stack([life / 10, hand / 3, lib / 20, (mine - theirs) / 5, turn / 10, np.ones_like(life)], axis=1)


def fit_logistic(x: np.ndarray, y: np.ndarray):
    """A logistic regression on standardised features; returns a predictor."""
    mean, std = x.mean(0), x.std(0)
    std[std == 0] = 1.0
    xt = torch.from_numpy((x - mean) / std).double()
    yt = torch.from_numpy(y).double()
    w = torch.zeros(x.shape[1], dtype=torch.float64, requires_grad=True)
    b = torch.zeros(1, dtype=torch.float64, requires_grad=True)
    opt = torch.optim.LBFGS([w, b], max_iter=100, line_search_fn="strong_wolfe")

    def closure():
        opt.zero_grad()
        loss = F.binary_cross_entropy_with_logits(xt @ w + b, yt)
        loss.backward()
        return loss

    opt.step(closure)
    w_, b_ = w.detach().numpy(), float(b.detach())
    return lambda z: 1 / (1 + np.exp(-np.clip(((z - mean) / std) @ w_ + b_, -30, 30)))


def report_by(name: str, keys: np.ndarray, p: np.ndarray, y: np.ndarray, order: list | None = None) -> dict:
    out = {}
    for k in order or sorted(set(keys.tolist())):
        m = keys == k
        if m.sum() >= 200:
            r = metrics(p[m], y[m])
            out[str(k)] = {x: r[x] for x in ("n", "brier", "logloss", "accuracy", "ece")} | {"reliability": r["reliability"]}
    return {name: out}


def markdown(rep: dict) -> str:
    lines = [f"# Value net report — {rep['model']}", ""]
    lines.append(f"**What it measures:** {rep['measures']}")
    lines.append("")
    lines.append(f"Dataset `{rep['dataset']}`: {rep['train_samples']:,} training and {rep['held_out_samples']:,} held-out decisions "
                 f"({rep['held_out_games']:,} held-out games). Working-card sets: {', '.join(rep['working_hashes'])}.")
    lines.append("")
    lines.append("| | Brier ↓ | log loss ↓ | accuracy (decided) ↑ | ECE ↓ |")
    lines.append("|---|---|---|---|---|")
    for k in ("coin", "logistic (life, hand, library, board, turn)", "value net"):
        r = rep["overall"][k]
        lines.append(f"| {k} | {r['brier']:.4f} | {r['logloss']:.4f} | {r['accuracy']:.3f} | {r['ece']:.4f} |")
    lines.append("")
    lines.append("## The extremes")
    lines.append("")
    lines.append("Where a bar's credibility lives: positions rated under 10 % and over 90 %.")
    lines.append("")
    lines.append("| | rated under 10 %: positions, mean rating, won | rated over 90 %: positions, mean rating, won |")
    lines.append("|---|---|---|")
    for name, rel in [("all", rep["overall"]["value net"]["reliability"])] + [
        (k, v["reliability"]) for k, v in rep["by_progress"].items()
    ]:
        low = next((r for r in rel if r["bin"] == "0.0-0.1"), None)
        high = next((r for r in rel if r["bin"] == "0.9-1.0"), None)
        cell = lambda r: f"{r['n']:,} · {r['predicted']:.3f} · {r['won']:.3f}" if r else "—"
        lines.append(f"| {name} | {cell(low)} | {cell(high)} |")
    lines.append("")
    if rep.get("curve"):
        lines.append(f"## Training curve (held-out slice; reported model: step {rep['best_step']:,})")
        lines.append("")
        lines.append("| step | train loss | held-out log loss | Brier | ECE |")
        lines.append("|---|---|---|---|---|")
        for c in rep["curve"]:
            lines.append(f"| {c['step']:,} | {c['train_loss']:.4f} | {c['logloss']:.4f} | {c['brier']:.4f} | {c['ece']:.4f} |")
        lines.append("")
    lines.append("## Reliability (held out)")
    lines.append("")
    lines.append("| rated | positions | mean rating | won |")
    lines.append("|---|---|---|---|")
    for row in rep["overall"]["value net"]["reliability"]:
        lines.append(f"| {row['bin']} | {row['n']:,} | {row['predicted']:.3f} | {row['won']:.3f} |")
    for split in ("by_turn", "by_progress", "by_profile"):
        lines.append("")
        lines.append(f"## {split.replace('_', ' ')}")
        lines.append("")
        lines.append("| | positions | Brier | accuracy | ECE |")
        lines.append("|---|---|---|---|---|")
        for k, r in rep[split].items():
            lines.append(f"| {k} | {r['n']:,} | {r['brier']:.4f} | {r['accuracy']:.3f} | {r['ece']:.4f} |")
    return "\n".join(lines) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=float, default=2.0)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--entities", type=int, default=160)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--d", type=int, default=256)
    ap.add_argument("--layers", type=int, default=4)
    ap.add_argument("--heads", type=int, default=8)
    ap.add_argument("--ff", type=int, default=1024)
    ap.add_argument("--dropout", type=float, default=0.1)
    ap.add_argument("--wd", type=float, default=0.05)
    ap.add_argument("--eval-every", type=int, default=1000, help="steps between looks at a held-out slice")
    ap.add_argument("--eval-slice", type=int, default=30_000, help="held-out decisions in that slice")
    ap.add_argument("--eval-max", type=int, default=400_000, help="held-out decisions scored in the report")
    ap.add_argument("--compile", action="store_true")
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    torch.backends.cuda.matmul.allow_tf32 = True
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=False)
    t0 = time.time()
    ds = D.load(Path(args.data).expanduser())
    train_idx, held_idx = D.split_by_game(ds)
    if "step" in ds.meta["meta_cols"]:
        # A multi-pick answer's later steps show the same position again.
        first = ds.col("step") == 0
        train_idx, held_idx = train_idx[first[train_idx]], held_idx[first[held_idx]]
    log(f"loaded {ds.n:,} decisions, {len(ds.ent_card):,} entities in {time.time() - t0:.0f}s; "
        f"train {len(train_idx):,}, held out {len(held_idx):,}")
    target = ds.col("result").astype(np.float32) / 2

    net = M.build(ds.meta, d=args.d, layers=args.layers, heads=args.heads, ff=args.ff, dropout=args.dropout).cuda()
    params = sum(p.numel() for p in net.parameters())
    trunk = sum(p.numel() for n, p in net.named_parameters() if not n.startswith("trunk.card."))
    log(f"{params / 1e6:.2f}M parameters ({trunk / 1e6:.2f}M besides the card embedding)")
    step_fn = torch.compile(net) if args.compile else net
    opt = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.wd, fused=True)
    # A fixed slice of held-out games, looked at during training: the model
    # the report scores is the one that did best there, not the last one.
    watch = np.sort(np.random.default_rng(2).choice(held_idx, size=min(args.eval_slice, len(held_idx)), replace=False))
    best = {"logloss": float("inf"), "step": 0}
    curve = []
    loader = D.Loader(ds, train_idx, args.batch, args.entities, shuffle=True, seed=args.seed)
    total = int(len(loader) * args.epochs)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=total, pct_start=0.03)
    log(f"{total:,} steps of {args.batch}")

    step, seen, t_start, run_loss = 0, 0, time.time(), 0.0
    epoch = 0
    while step < total:
        for b in loader:
            with torch.autocast("cuda", dtype=torch.bfloat16):
                logit = step_fn(b)
            loss = F.binary_cross_entropy_with_logits(logit.float(), b.target)
            opt.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            sched.step()
            step += 1
            seen += len(b.target)
            run_loss = 0.98 * run_loss + 0.02 * loss.item() if step > 1 else loss.item()
            if step % 500 == 0 or step == total:
                el = time.time() - t_start
                log(f"step {step:,}/{total:,} · epoch {epoch} · loss {run_loss:.4f} · {seen / el:,.0f} decisions/s · "
                    f"eta {(total - step) * el / step / 60:.0f} min")
            if step % args.eval_every == 0 or step == total:
                m = metrics(predict(net, ds, watch, 1024, args.entities), target[watch])
                curve.append({"step": step, "train_loss": run_loss} | {k: m[k] for k in ("logloss", "brier", "ece", "accuracy")})
                mark = ""
                if m["logloss"] < best["logloss"]:
                    best = {"logloss": m["logloss"], "step": step}
                    torch.save(net.state_dict(), out / "value_best.state")
                    mark = " · best"
                log(f"held out @ {step:,}: log loss {m['logloss']:.4f} · Brier {m['brier']:.4f} · ECE {m['ece']:.4f} · "
                    f"accuracy {m['accuracy']:.3f}{mark}")
            if step >= total:
                break
        epoch += 1

    net.load_state_dict(torch.load(out / "value_best.state"))
    log(f"reporting the model of step {best['step']:,} (best held-out log loss {best['logloss']:.4f})")
    torch.save({"state": net.state_dict(), "config": net.trunk.cfg.to_dict(), "dataset": ds.meta["sources"],
                "encoder_version": ds.meta["encoder_version"], "step": best["step"]}, out / "value.pt")

    # --- the report ---------------------------------------------------------------
    rng = np.random.default_rng(1)
    ev = np.sort(rng.choice(held_idx, size=min(args.eval_max, len(held_idx)), replace=False))
    y = target[ev]
    log(f"scoring {len(ev):,} held-out decisions")
    p = predict(net, ds, ev, 1024, args.entities)
    log("fitting the logistic baseline")
    tr = np.sort(rng.choice(train_idx, size=min(200_000, len(train_idx)), replace=False))
    logistic = fit_logistic(baseline_features(ds, tr), target[tr])
    p_log = logistic(baseline_features(ds, ev))
    turn = ds.col("turn")[ev]
    final = ds.col("final_turn")[ev]
    progress = np.where(turn * 3 <= final, "early third", np.where(turn * 3 <= final * 2, "middle third", "last third"))
    turn_bucket = np.select([turn <= 3, turn <= 6, turn <= 9, turn <= 14], ["turns 1-3", "turns 4-6", "turns 7-9", "turns 10-14"], "turns 15+")
    games = ds.games
    seat = ds.col("seat")[ev]
    profile = np.array([games[g]["profiles"][s] for g, s in zip(ds.col("game")[ev], seat)])
    rep = {
        "model": out.name,
        "measures": "the chance that a house AI in the deciding seat, playing on against a house AI, wins from this "
                    "position (self-play of the house profiles, not human play). Two decks only (allytifact and "
                    "victory, their partial cards swapped for basics): the net has seen about two hundred distinct "
                    "cards, and its other embedding rows are untrained, so the number does not carry to other decks yet",
        "dataset": str(args.data),
        "working_hashes": sorted({s["working"]["hash"][:12] for s in ds.meta["sources"]}),
        "builds": sorted({s["build"] for s in ds.meta["sources"]}),
        "config": net.trunk.cfg.to_dict(),
        "params": params,
        "train_samples": int(len(train_idx)),
        "held_out_samples": int(len(held_idx)),
        "held_out_games": int(len(set(ds.col("game")[held_idx].tolist()))),
        "steps": total,
        "best_step": best["step"],
        "curve": curve,
        "batch": args.batch,
        "train_seconds": time.time() - t_start,
        "overall": {
            "coin": metrics(np.full_like(y, 0.5), y),
            "logistic (life, hand, library, board, turn)": metrics(p_log, y),
            "value net": metrics(p, y),
        },
    }
    rep |= report_by("by_turn", turn_bucket, p, y, ["turns 1-3", "turns 4-6", "turns 7-9", "turns 10-14", "turns 15+"])
    rep |= report_by("by_progress", progress, p, y, ["early third", "middle third", "last third"])
    rep |= report_by("by_profile", profile, p, y, ["novice", "casual", "steady", "sharp", "expert"])
    (out / "report.json").write_text(json.dumps(rep, indent=1))
    md = markdown(rep)
    (out / "report.md").write_text(md)
    print(md, flush=True)


if __name__ == "__main__":
    main()
