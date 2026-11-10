"""Training the tactician on the layer's answers, with a loss at every pass.

Per pass t (deep supervision, the passes weighted alike):
- attack and block: per-unit binary cross-entropy against the label;
- block (and attack with `--group-softmax attack,block`): per group (blocker,
  attacker) a softmax over its units and "none" (logit 0), the labelled unit or
  none as the class, which is how the decoder reads a group;
- target: softmax cross-entropy over the units;
- confidence: binary cross-entropy of the pass's confidence against whether the
  pass's decoded answer equals the label.

Held out: whole tables, by seed (`seed % held_out_mod == 0`). Reported per window
and per pass: exact-mask agreement with the label and per-unit accuracy, beside
the net's own answer `N`; the confidence's calibration; and, per pass, the
agreement of "the built answer when the confidence reaches tau, else N" for each
tau, whose best tau becomes the threshold the seat plays with.
"""
from __future__ import annotations

import hashlib
import json
import math
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

from .model import NEG, Config, TacticianV3, decode, exact
from .records import ATTACK, BLOCK, RECORD_CONTRACT, ROW_TAIL, TARGET, WINDOWS, read_records

CONTRACT = "tactician_head_v2"
TAUS = tuple(round(0.05 * i, 2) for i in range(20))


def group_softmax_loss(logits, labels, unit_mask, groups):
    """[B] mean over a decision's groups of -log p(labelled unit or none), the
    group's softmax taken over its units and a "none" logit of 0. A group with
    more than one labelled unit is left out."""
    b, n = logits.shape
    g = groups.clamp(min=0)
    size = int(g.max()) + 1 if n else 1
    masked = logits.masked_fill(~unit_mask, NEG)
    gmax = torch.full((b, size), NEG, dtype=logits.dtype, device=logits.device)
    gmax = gmax.scatter_reduce(1, g, masked, reduce="amax", include_self=True)
    m = gmax.clamp(min=0.0)
    sumexp = torch.zeros((b, size), dtype=logits.dtype, device=logits.device)
    sumexp = sumexp.scatter_add(1, g, torch.exp(masked - m.gather(1, g)) * unit_mask)
    lse = m + torch.log(torch.exp(-m) + sumexp)
    chosen = torch.zeros((b, size), dtype=logits.dtype, device=logits.device)
    chosen = chosen.scatter_add(1, g, logits * labels * unit_mask)
    count = torch.zeros((b, size), dtype=logits.dtype, device=logits.device)
    count = count.scatter_add(1, g, labels * unit_mask)
    present = torch.zeros((b, size), dtype=logits.dtype, device=logits.device)
    present = present.scatter_add(1, g, unit_mask.to(logits.dtype)) > 0
    valid = (present & (count <= 1)).to(logits.dtype)
    per_group = (lse - chosen) * valid
    return per_group.sum(dim=1) / valid.sum(dim=1).clamp(min=1.0)


def pass_losses(logits, confidence, batch, group_windows):
    """(unit loss [B], confidence loss [B], hit [B]) of one pass."""
    um = batch.unit_mask
    labels = batch.labels
    window = batch.window
    umf = um.to(logits.dtype)
    bce = F.binary_cross_entropy_with_logits(logits, labels, reduction="none") * umf
    per_set = bce.sum(dim=1) / umf.sum(dim=1).clamp(min=1.0)
    masked = logits.masked_fill(~um, NEG)
    target_ce = F.cross_entropy(masked, labels.argmax(dim=1), reduction="none")
    is_target = (window == TARGET).to(logits.dtype)
    unit_loss = (1.0 - is_target) * per_set + is_target * target_ce
    if group_windows:
        use = torch.zeros_like(is_target)
        for w in group_windows:
            use = use + (window == w).to(logits.dtype)
        unit_loss = unit_loss + use * group_softmax_loss(logits, labels, um, batch.groups)
    answer = decode(logits.detach(), um, batch.groups, window)
    hit = exact(answer, labels, um).to(logits.dtype)
    conf_loss = F.binary_cross_entropy_with_logits(confidence, hit, reduction="none")
    return unit_loss, conf_loss, hit


def loss_of(outputs, batch, group_windows, confidence_weight):
    """The mean over passes of each pass's unit and confidence losses."""
    total = 0.0
    for logits, confidence in outputs:
        unit_loss, conf_loss, _ = pass_losses(logits, confidence, batch, group_windows)
        total = total + unit_loss.mean() + confidence_weight * conf_loss.mean()
    return total / len(outputs)


@torch.no_grad()
def evaluate(net, store, index, batch_size, device, passes, group_windows):
    """Held-out readings per pass and window, calibration, threshold curves."""
    net.eval()
    per_pass = [dict(loss=0.0, hit=[], conf=[], unit_right=0.0, units=0.0) for _ in range(passes)]
    windows, net_hits, decisions = [], [], 0
    for start in range(0, len(index), batch_size):
        batch = store.batch(index[start:start + batch_size], device)
        outputs = net(batch.rows, batch.mask, passes)
        um = batch.unit_mask
        net_hits.append(exact(batch.nets, batch.labels, um).cpu())
        windows.append(batch.window.cpu())
        decisions += len(batch.window)
        for t, (logits, confidence) in enumerate(outputs):
            unit_loss, conf_loss, hit = pass_losses(logits.float(), confidence.float(), batch,
                                                    group_windows)
            answer = decode(logits.float(), um, batch.groups, batch.window)
            right = ((answer == batch.labels) & um).sum(dim=1).float()
            reading = per_pass[t]
            reading["loss"] += float((unit_loss + conf_loss).sum())
            reading["hit"].append(hit.cpu())
            reading["conf"].append(torch.sigmoid(confidence.float()).cpu())
            reading.setdefault("right", []).append(right.cpu())
            reading.setdefault("count", []).append(um.sum(dim=1).float().cpu())
    net.train()
    window = torch.cat(windows).numpy()
    n_hit = torch.cat(net_hits).float().numpy()
    report = dict(decisions=decisions, passes=[], net_answer={})
    for w, name in enumerate(WINDOWS):
        sel = window == w
        report["net_answer"][name] = dict(decisions=int(sel.sum()),
                                          exact=float(n_hit[sel].mean()) if sel.any() else None)
    report["net_answer"]["all"] = dict(decisions=decisions, exact=float(n_hit.mean()))
    for t, reading in enumerate(per_pass):
        hit = torch.cat(reading["hit"]).numpy()
        conf = torch.cat(reading["conf"]).numpy()
        right = torch.cat(reading["right"]).numpy()
        count = torch.cat(reading["count"]).numpy()
        by_window = {}
        for w, name in enumerate(WINDOWS):
            sel = window == w
            if sel.any():
                by_window[name] = dict(decisions=int(sel.sum()), exact=float(hit[sel].mean()),
                                       unit_accuracy=float(right[sel].sum() / count[sel].sum()))
        by_window["all"] = dict(decisions=decisions, exact=float(hit.mean()),
                                unit_accuracy=float(right.sum() / count.sum()))
        curve = threshold_curve(conf, hit, n_hit)
        report["passes"].append(dict(
            t=t + 1, loss=reading["loss"] / max(decisions, 1), by_window=by_window,
            calibration=calibration(conf, hit), thresholds=curve,
            threshold=best_threshold(curve)))
    return report


def calibration(conf, hit, bins: int = 10):
    """Reliability bins, expected calibration error and Brier score."""
    edges = np.linspace(0.0, 1.0, bins + 1)
    rows, ece = [], 0.0
    for lo, hi in zip(edges[:-1], edges[1:]):
        sel = (conf >= lo) & ((conf < hi) | (hi == 1.0))
        if sel.any():
            gap = abs(float(conf[sel].mean()) - float(hit[sel].mean()))
            ece += gap * sel.sum() / len(conf)
            rows.append(dict(lo=float(lo), hi=float(hi), n=int(sel.sum()),
                             confidence=float(conf[sel].mean()), accuracy=float(hit[sel].mean())))
    return dict(bins=rows, ece=float(ece), brier=float(np.mean((conf - hit) ** 2)),
                mean_confidence=float(conf.mean()), accuracy=float(hit.mean()))


def threshold_curve(conf, hit, net_hit):
    """Per tau: how often the built answer plays, and the agreement with the
    label of "built when confident, else N"."""
    out = []
    for tau in TAUS:
        use = conf >= tau
        out.append(dict(tau=tau, built=float(use.mean()),
                        agreement=float(np.where(use, hit, net_hit).mean())))
    return out


def best_threshold(curve):
    best = max(row["agreement"] for row in curve)
    return min(row["tau"] for row in curve if row["agreement"] == best)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_groups(text: str):
    names = [w for w in text.split(",") if w]
    unknown = [w for w in names if w not in ("attack", "block")]
    if unknown:
        raise ValueError(f"group softmax applies to attack and block, not {unknown}")
    return tuple(WINDOWS.index(w) for w in names)


def train(args):
    from .export import export_dir

    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a tactician gets its own directory")
    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    device = torch.device(args.device)
    started = time.time()
    store = read_records(args.records)
    train_idx, held_idx = store.split(args.held_out_mod)
    if len(train_idx) == 0 or len(held_idx) == 0:
        raise ValueError("both a training and a held-out split are needed")
    store = store.to(device if args.data_on_device else "cpu",
                     torch.float16 if args.store_fp16 else None)
    cfg = Config(d_model=store.d_model, width=args.width, layers=args.layers, heads=args.heads,
                 passes=args.passes, ffn_mult=args.ffn_mult, dropout=args.dropout,
                 detach_passes=not args.through_passes)
    net = TacticianV3(cfg).to(device)
    group_windows = parse_groups(args.group_softmax)
    optimizer = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.weight_decay)
    steps_per_epoch = math.ceil(len(train_idx) / args.batch)
    total_steps = steps_per_epoch * args.epochs
    if args.max_steps:
        total_steps = min(total_steps, args.max_steps)

    def lr_at(step):
        if step < args.warmup:
            return (step + 1) / args.warmup
        progress = (step - args.warmup) / max(1, total_steps - args.warmup)
        return 0.1 + 0.9 * 0.5 * (1.0 + math.cos(math.pi * min(progress, 1.0)))

    scheduler = torch.optim.lr_scheduler.LambdaLR(optimizer, lr_at)
    amp = args.bf16 and device.type == "cuda"
    history, best, best_state, stale, step = [], -1.0, None, 0, 0
    for epoch in range(1, args.epochs + 1):
        order = rng.permutation(train_idx)
        running, seen = 0.0, 0
        for start in range(0, len(order), args.batch):
            if args.max_steps and step >= args.max_steps:
                break
            batch = store.batch(order[start:start + args.batch], device)
            with torch.autocast(device.type, dtype=torch.bfloat16, enabled=amp):
                outputs = net(batch.rows, batch.mask)
            outputs = [(l.float(), c.float()) for l, c in outputs]
            loss = loss_of(outputs, batch, group_windows, args.confidence_weight)
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), args.clip)
            optimizer.step()
            scheduler.step()
            step += 1
            running += float(loss) * len(batch.window)
            seen += len(batch.window)
        held = evaluate(net, store, held_idx, args.eval_batch, device, cfg.passes, group_windows)
        final = held["passes"][-1]
        row = dict(epoch=epoch, step=step, train_loss=running / max(seen, 1),
                   held_out_loss=final["loss"], held_out_exact=final["by_window"]["all"]["exact"],
                   held_out_exact_by_pass=[p["by_window"]["all"]["exact"] for p in held["passes"]],
                   seconds=time.time() - started)
        history.append(row)
        print(json.dumps(row), flush=True)
        if row["held_out_exact"] > best:
            best, stale = row["held_out_exact"], 0
            best_state = {k: v.detach().cpu().clone() for k, v in net.state_dict().items()}
        else:
            stale += 1
            if stale >= args.patience:
                break
        if args.max_steps and step >= args.max_steps:
            break
    net.load_state_dict(best_state)
    held = evaluate(net, store, held_idx, args.eval_batch, device, cfg.passes, group_windows)
    trained = evaluate(net, store, train_idx[: args.train_report], args.eval_batch, device,
                       cfg.passes, group_windows)
    out.mkdir(parents=True)
    thresholds = [p["threshold"] for p in held["passes"]]
    meta = dict(
        contract=CONTRACT, record_contract=RECORD_CONTRACT, d_model=cfg.d_model,
        row_tail=ROW_TAIL, rows_read_rules=False, config=cfg.to_dict(),
        parameters=sum(p.numel() for p in net.parameters()),
        thresholds_by_pass=thresholds,
        seed=args.seed, lr=args.lr, weight_decay=args.weight_decay, batch=args.batch,
        epochs_run=len(history), steps=step, group_softmax=[WINDOWS[w] for w in group_windows],
        confidence_weight=args.confidence_weight, held_out_rule=f"seed % {args.held_out_mod} == 0",
        decisions=dict(train=len(train_idx), held_out=len(held_idx)),
        records=[dict(dir=str(Path(p).resolve()),
                      files=sorted(f.name for f in Path(p).glob("w*")),
                      run_sha256=digest(Path(p) / "run.json")) for p in args.records],
        label="the tactical layer's answer (imitation; trains the tactician only)",
        torch=torch.__version__,
    )
    torch.save(dict(state_dict=net.state_dict(), config=cfg.to_dict(), meta=meta), out / "net.pt")
    report = dict(history=history, held_out=held, train_sample=trained,
                  seconds=time.time() - started)
    (out / "report.json").write_text(json.dumps(report, indent=2))
    check = [store.batch([i], "cpu").rows[0] for i in held_idx[: args.parity_rows]]
    for passes in sorted({cfg.passes, *args.export_passes}):
        target = out if passes == cfg.passes else out / "passes" / f"T{passes}"
        export_dir(net, meta, passes, target, check)
    summary = dict(held_out_exact_by_pass=[p["by_window"] for p in held["passes"]],
                   net_answer=held["net_answer"], threshold=thresholds[-1],
                   calibration=held["passes"][-1]["calibration"])
    print(json.dumps(summary), flush=True)
    return meta, report
