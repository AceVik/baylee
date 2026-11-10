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
from .records import (ATTACK, BLOCK, RECORD_CONTRACT, ROW_TAIL, STAGE1_CONTRACT, TARGET,
                      WINDOWS, read_records)

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
    """Held-out readings per pass and window, on every decision and on those
    where the net's answer N differs from the label R (`differ`, the decisions
    that say whether the tactician learnt what the layer does beyond the
    trunk); calibration and threshold curves."""
    net.eval()
    # By size, so a batch pads little; every reading is a sum over decisions.
    index = np.asarray(index)
    index = index[np.argsort(store.n.cpu().numpy()[index], kind="stable")]
    keys = ("hit", "conf", "right", "count", "changed_right", "changed")
    per_pass = [dict(loss=0.0, **{k: [] for k in keys}) for _ in range(passes)]
    windows, net_hits, decisions = [], [], 0
    for start in range(0, len(index), batch_size):
        batch = store.batch(index[start:start + batch_size], device)
        outputs = net(batch.rows, batch.mask, passes)
        um = batch.unit_mask
        net_hits.append(exact(batch.nets, batch.labels, um).cpu())
        windows.append(batch.window.cpu())
        decisions += len(batch.window)
        changed = (batch.nets != batch.labels) & um
        for t, (logits, confidence) in enumerate(outputs):
            unit_loss, conf_loss, hit = pass_losses(logits.float(), confidence.float(), batch,
                                                    group_windows)
            answer = decode(logits.float(), um, batch.groups, batch.window)
            right = (answer == batch.labels) & um
            reading = per_pass[t]
            reading["loss"] += float((unit_loss + conf_loss).sum())
            reading["hit"].append(hit.cpu())
            reading["conf"].append(torch.sigmoid(confidence.float()).cpu())
            reading["right"].append(right.sum(dim=1).float().cpu())
            reading["count"].append(um.sum(dim=1).float().cpu())
            reading["changed_right"].append((right & changed).sum(dim=1).float().cpu())
            reading["changed"].append(changed.sum(dim=1).float().cpu())
    net.train()
    window = torch.cat(windows).numpy()
    n_hit = torch.cat(net_hits).float().numpy()
    differ = n_hit < 0.5
    report = dict(decisions=decisions, differ=int(differ.sum()), passes=[], net_answer={})
    for w, name in enumerate(WINDOWS):
        sel = window == w
        report["net_answer"][name] = dict(
            decisions=int(sel.sum()), differ=int((sel & differ).sum()),
            exact=float(n_hit[sel].mean()) if sel.any() else None)
    report["net_answer"]["all"] = dict(decisions=decisions, differ=int(differ.sum()),
                                       exact=float(n_hit.mean()))

    def reading_of(sel, hit, right, count, changed_right, changed):
        if not sel.any():
            return None
        return dict(decisions=int(sel.sum()), exact=float(hit[sel].mean()),
                    unit_accuracy=float(right[sel].sum() / max(count[sel].sum(), 1.0)),
                    changed_unit_accuracy=(float(changed_right[sel].sum() / changed[sel].sum())
                                           if changed[sel].sum() > 0 else None))

    for t, reading in enumerate(per_pass):
        arrays = {k: torch.cat(reading[k]).numpy() for k in keys}
        hit, conf = arrays["hit"], arrays["conf"]
        parts = (arrays["hit"], arrays["right"], arrays["count"], arrays["changed_right"],
                 arrays["changed"])
        by_window, by_window_differ = {}, {}
        for w, name in enumerate(WINDOWS):
            sel = window == w
            if sel.any():
                by_window[name] = reading_of(sel, *parts)
                by_window_differ[name] = reading_of(sel & differ, *parts)
        everything = np.ones_like(differ)
        by_window["all"] = reading_of(everything, *parts)
        by_window_differ["all"] = reading_of(differ, *parts)
        curve = threshold_curve(conf, hit, n_hit)
        # Per rule group that changed the layer's answer at the decision (a
        # decision counts for each group that fired; "none": the bare layer).
        fired = [store.fired[i] for i in index]
        names = sorted({g for groups in fired for g in groups})
        by_group_differ = {}
        for name in [*names, "none"]:
            sel = np.array([(name in groups) if name != "none" else not groups
                            for groups in fired], dtype=bool)
            by_group_differ[name] = reading_of(sel & differ, *parts)
        report["passes"].append(dict(
            t=t + 1, loss=reading["loss"] / max(decisions, 1), by_window=by_window,
            by_window_differ=by_window_differ, by_group_differ=by_group_differ,
            calibration=calibration(conf, hit), thresholds=curve,
            threshold=best_threshold(curve)))
    return report


def differ_exact(reading) -> float:
    """Exact-mask agreement with R where N differs: what selection reads."""
    value = reading["by_window_differ"]["all"]
    return value["exact"] if value else 0.0


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


def bucketed(rng, index, sizes, batch, span: int = 32):
    """An epoch's batches: shuffled, then within each run of `span` batches
    sorted by units so a batch pads little; the batches' order shuffled again."""
    order = rng.permutation(index)
    chunk = batch * span
    batches = []
    for start in range(0, len(order), chunk):
        part = order[start:start + chunk]
        part = part[np.argsort(sizes[part], kind="stable")]
        batches.extend(part[i:i + batch] for i in range(0, len(part), batch))
    return [batches[i] for i in rng.permutation(len(batches))]


def split_held_out(store, held_idx, held_out_mod):
    """The held-out tables in two halves by seed: one selects the checkpoint,
    the other is reported, so the reported numbers were not chosen on."""
    tables = store.seed[held_idx] // np.uint64(held_out_mod)
    select = (tables % np.uint64(2)) == 0
    return held_idx[select], held_idx[~select]


def train(args):
    from .export import export_dir

    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a tactician gets its own directory")
    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    device = torch.device(args.device)
    if args.threads:
        torch.set_num_threads(args.threads)
    started = time.time()
    store = read_records(args.records, stage1_probe=args.stage1_probe)
    train_idx, held_idx = store.split(args.held_out_mod)
    select_idx, report_idx = split_held_out(store, held_idx, args.held_out_mod)
    if len(train_idx) == 0 or len(select_idx) == 0 or len(report_idx) == 0:
        raise ValueError("a training split and both held-out halves are needed")
    print(json.dumps(dict(decisions=len(store), train=len(train_idx), select=len(select_idx),
                          report=len(report_idx), read_seconds=time.time() - started)),
          flush=True)
    store = store.to(device if args.data_on_device else "cpu",
                     torch.float16 if args.store_fp16 else None)
    parts = tuple(p for p in getattr(args, "zero_parts", "").split(",") if p)
    if set(parts) - {"state", "unit", "look"}:
        raise ValueError(f"--zero-parts names state, unit or look, not {parts}")
    store.zero_parts = parts
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
        batches = bucketed(rng, train_idx, store.n.cpu().numpy(), args.batch)
        running, seen = 0.0, 0
        for chosen in batches:
            if args.max_steps and step >= args.max_steps:
                break
            batch = store.batch(chosen, device)
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
        held = evaluate(net, store, select_idx, args.eval_batch, device, cfg.passes,
                        group_windows)
        final = held["passes"][-1]
        row = dict(epoch=epoch, step=step, train_loss=running / max(seen, 1),
                   select_loss=final["loss"], select_exact=final["by_window"]["all"]["exact"],
                   select_differ_exact=differ_exact(final),
                   select_differ_exact_by_pass=[differ_exact(p) for p in held["passes"]],
                   seconds=time.time() - started)
        history.append(row)
        print(json.dumps(row), flush=True)
        # Checkpoints are chosen on agreement where N and R differ, not on the
        # loss: a lower loss can come with fewer right answers there.
        if row["select_differ_exact"] > best:
            best, stale = row["select_differ_exact"], 0
            best_state = {k: v.detach().cpu().clone() for k, v in net.state_dict().items()}
        else:
            stale += 1
            if stale >= args.patience:
                break
        if args.max_steps and step >= args.max_steps:
            break
    net.load_state_dict(best_state)
    held = evaluate(net, store, report_idx, args.eval_batch, device, cfg.passes, group_windows)
    chosen_on = evaluate(net, store, select_idx, args.eval_batch, device, cfg.passes,
                         group_windows)
    trained = evaluate(net, store, train_idx[: args.train_report], args.eval_batch, device,
                       cfg.passes, group_windows)
    out.mkdir(parents=True)
    thresholds = [p["threshold"] for p in held["passes"]]

    def run_sha(path: Path):
        return digest(path / "run.json") if (path / "run.json").exists() else None

    meta = dict(
        contract=CONTRACT, record_contract=STAGE1_CONTRACT if args.stage1_probe else RECORD_CONTRACT,
        d_model=cfg.d_model, row_tail=ROW_TAIL, rows_read_rules=False, config=cfg.to_dict(),
        parameters=sum(p.numel() for p in net.parameters()),
        thresholds_by_pass=thresholds, threshold_use="reported only; play ignores it by default",
        seed=args.seed, lr=args.lr, weight_decay=args.weight_decay, batch=args.batch,
        epochs_run=len(history), steps=step, group_softmax=[WINDOWS[w] for w in group_windows],
        confidence_weight=args.confidence_weight, held_out_rule=f"seed % {args.held_out_mod} == 0",
        selection="held-out tables with (seed // mod) even: exact agreement with R where N != R",
        decisions=dict(train=len(train_idx), select=len(select_idx), report=len(report_idx)),
        records=[dict(dir=str(Path(p).resolve()),
                      files=sorted(f.name for f in Path(p).glob("w*")),
                      run_sha256=run_sha(Path(p))) for p in args.records],
        label="the tactical layer's answer (imitation; trains the tactician only)",
        probe=bool(args.stage1_probe), zero_parts=list(parts),
        torch=torch.__version__,
    )
    torch.save(dict(state_dict=net.state_dict(), config=cfg.to_dict(), meta=meta), out / "net.pt")
    report = dict(meta=meta, history=history, held_out=held, selection_half=chosen_on,
                  train_sample=trained, seconds=time.time() - started)
    (out / "report.json").write_text(json.dumps(report, indent=2))
    if not args.stage1_probe:
        # A probe on stage-1 records is a measurement: nothing of it is played.
        check = [store.batch([i], "cpu").rows[0] for i in report_idx[: args.parity_rows]]
        for passes in sorted({cfg.passes, *args.export_passes}):
            target = out if passes == cfg.passes else out / "passes" / f"T{passes}"
            export_dir(net, meta, passes, target, check)
    summary = dict(differ_by_pass=[p["by_window_differ"] for p in held["passes"]],
                   all_by_pass=[p["by_window"] for p in held["passes"]],
                   net_answer=held["net_answer"], threshold=thresholds[-1],
                   calibration=held["passes"][-1]["calibration"])
    print(json.dumps(summary), flush=True)
    return meta, report
