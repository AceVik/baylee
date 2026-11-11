"""Coarse distillation's two directions on the candidate head alone, on the CPU.

    CUDA_VISIBLE_DEVICES= python -m training4.direction_probe capture --model R1 --data DATASET \
        --every 8 --offset 0 --rows 1000 --out cache-0.pt
    ... show --cache cache-0.pt --head R1 --head STUDENT --rows 20
    ... evaluate --cache cache-0.pt --head R1 --head STUDENT
    ... fit --cache cache-0.pt --cache cache-4.pt --init R1 --epochs 40 --lr 6e-5 --anchor-l2 0.15

`capture` runs a net's frozen part once over whole validation trajectories
(boards, memory, latent thinking, the candidates' rule and DSL embeddings, their
entity, seat and global tokens, the board keys of the candidate decoder and the
deliberation block) and keeps, for every labelled priority decision that
`distill_loss` teaches by `ce_coarse_r` under `--distill-coarse act` (N != R and
R's group, act or pass, differs from N's), what the candidate head reads and
the frame fields the loss reads. Under `--train-policy-head-only` nothing before
the candidate head learns, so these inputs are the same for r1 and every
student trained from it, and the head alone reproduces their logits.
It also tabulates, over every joined priority decision, the recorded N != R
flag against the sidecar's answers (R's index against N's), by direction.

`show` prints rows as the head sees them; `evaluate` reports, per head
checkpoint and direction, `ce_coarse_r`, the share whose top-1 is in R's group,
and the head's summed probability of R's group; `fit` trains a head-only
student on the cached rows with `distill_loss` (and optionally the L2 anchor)
for many epochs, as the overfit test of the act direction.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import random
import time
from types import SimpleNamespace

import torch

import model4 as M
from model4.candidate import score_candidate_hidden
from model4.config import symlog
from . import distill as D
from .coarse import CLASSES

PASS = CLASSES.index("pass")
HEAD_ONLY = SimpleNamespace(train_policy_head_only=True, adapter_on_candidate_head=0,
                            trainable_trunk_from_layer=-1, freeze_value_head=False)
FIELDS = ("single_idx", "coarse_n_idx", "n_ne_r", "pending_kind", "distill_subtype", "loss_weight", "head",
          "is_decision", "step_valid", "fired_mask")


def configure(net, args):
    net.decision_chunk = args.decision_chunk
    net.board_token_budget = args.board_token_budget
    net.prune_unused_policy_heads = True
    net.cache_thinking_kv = True
    net.gpu_prefetch = 0
    return net


def directions(batch, t, width):
    """Masks [B] at step t: labelled priority decisions joined with the sidecar, and of
    them those `distill_loss` teaches by group (coarse "act"), with R's and N's group."""
    valid = batch["step_valid"][:, t]
    weight = torch.where(valid, batch["loss_weight"][:, t], 0.)
    selected = batch["is_decision"][:, t] & valid & (weight > 0) & (batch["head"][:, t] == 0)
    mask = batch["cand_mask"][:, t, :width]
    n_idx = batch["coarse_n_idx"][:, t]
    r_idx = batch["single_idx"][:, t].clamp(min=0)
    joined = selected & (mask.sum(-1) > 1) & (batch["pending_kind"][:, t] == D.PRIORITY) & (n_idx >= 0)
    acts = batch["coarse_class"][:, t, :width] != PASS     # the loss's groups under "act"
    r_act = acts.gather(1, r_idx[:, None])[:, 0]
    n_act = acts.gather(1, n_idx.clamp(min=0)[:, None])[:, 0]
    split = joined & batch["n_ne_r"][:, t] & (r_act != n_act)
    return joined, split, r_act, n_act, r_idx != n_idx


class Recorder:
    """Wraps the net's candidate entry points and keeps what each call read, frozen parts computed."""

    def __init__(self, net):
        self.net, self.calls, self.keys = net, [], None
        self.originals = {name: getattr(net, name) for name in ("candidate_hidden", "thinking_keys",
                                                                 "candidate_decoder_keys")}

    def __enter__(self):
        net = self.net

        def keys(name):
            def wrapped(*args, **kwargs):
                out = self.originals[name](*args, **kwargs)
                if out is not None:
                    self.keys = out
                return out
            return wrapped

        def candidate_hidden(cand_feat, cand_dsl, cand_rules, cand_entity_idx, cand_seat_idx, e_ref, s_ref, g_ref,
                             decoder_keys=None):
            d = net.cfg.d_model
            has_ent = cand_entity_idx >= 0
            ent = torch.gather(e_ref, -2, cand_entity_idx.clamp(min=0).unsqueeze(-1).expand(*(-1,) * cand_entity_idx.dim(), d)
                               ) * has_ent.unsqueeze(-1).to(e_ref.dtype)
            has_seat = (cand_seat_idx >= 0) & (cand_seat_idx < M.MAX_SEATS)
            seat = torch.gather(s_ref, -2, cand_seat_idx.clamp(0, M.MAX_SEATS - 1).unsqueeze(-1).expand(
                *(-1,) * cand_seat_idx.dim(), d)) * has_seat.unsqueeze(-1).to(s_ref.dtype)
            self.calls.append(dict(c_feat=cand_feat, frozen=net.dsl_proj(symlog(cand_dsl)) + net.rule_features(cand_rules),
                                   ent=ent, seat=seat, g=g_ref,
                                   keys=decoder_keys if decoder_keys is not None else self.keys))
            return self.originals["candidate_hidden"](cand_feat, cand_dsl, cand_rules, cand_entity_idx, cand_seat_idx,
                                                      e_ref, s_ref, g_ref, decoder_keys)

        net.candidate_hidden = candidate_hidden
        net.thinking_keys = keys("thinking_keys")
        net.candidate_decoder_keys = keys("candidate_decoder_keys")
        return self

    def __exit__(self, *exc):
        for name in self.originals:
            del self.net.__dict__[name]

    def groups(self, plans, b, unroll, chunk):
        """The calls of one `deferred_readouts`, joined per readout group: [(times, call)]."""
        from .deferred import _cand_groups
        prune = getattr(self.net, "prune_unused_policy_heads", False)
        times = [t for t, p in enumerate(plans) if (not prune) or p.heads[0] or p.heads[1]]
        token_limit = 128 * 1024**2 // (b * self.net.cfg.d_model * 4)
        out, calls = [], list(self.calls)
        for group in _cand_groups(plans, times, max(chunk, unroll), token_limit):
            c_max = max(plans[t].candidates for t in group)
            parts = [calls.pop(0) for _ in range(math.ceil(c_max / 512) if c_max > 512 else 1)]
            call = dict(parts[0])
            for name in ("c_feat", "frozen", "ent", "seat"):
                call[name] = torch.cat([part[name] for part in parts], 1)
            out.append((group, call))
        if calls:
            raise RuntimeError("Candidate calls and readout groups disagree")
        self.calls, self.keys = [], None
        return out


def cached_rows(groups, batch, plans, prepared, wanted):
    """What the candidate head read at the `wanted` frames ({t: [(lane, direction)]}),
    from `Recorder.groups`, trimmed to each frame's own menu and board keys."""
    rows = []
    b = batch["step_valid"].shape[0]
    for times, call in groups:
        for i, t in enumerate(times):
            for lane, direction in wanted.get(t, ()):
                r = i * b + lane
                mask = batch["cand_mask"][lane, t, :plans[t].candidates]
                width = int(mask.nonzero().max()) + 1
                if call["keys"] is None:   # neither a candidate decoder nor a deliberation block
                    tokens, pad = call["g"].new_zeros(b * len(times), 0, call["g"].shape[-1]), mask.new_zeros(b * len(times), 0)
                else:
                    tokens, pad = call["keys"]
                length = int((~pad[r]).nonzero().max()) + 1 if pad.shape[1] else 0
                row = dict(direction=direction, width=plans[t].candidates,
                           c_feat=call["c_feat"][r, :width].float().clone(),
                           frozen=call["frozen"][r, :width].float().clone(),
                           ent=call["ent"][r, :width].float().clone(), seat=call["seat"][r, :width].float().clone(),
                           g=call["g"][r].float().clone(), tokens=tokens[r, :length].float().clone(),
                           pad=pad[r, :length].clone(), mask=mask[:width].clone(),
                           coarse_class=batch["coarse_class"][lane, t, :width].clone(),
                           logits=prepared[t][1][lane, :width].float().clone())
                row.update({name: batch[name][lane, t].clone() for name in FIELDS if name in batch})
                rows.append(row)
    return rows


DECODE_COLUMNS = ("rows", "top1_act", "top1_pass", "top1_pass_act_mass_over_half", "top1_pass_act_mass_over_best_pass",
                  "top1_act_act_mass_at_most_half")


def grouped(logits, mask, classes):
    """Per row: the act group's probability mass, the best pass candidate's probability,
    and whether the top-1 candidate acts (groups as coarse "act": pass against the rest)."""
    prob = torch.softmax(torch.where(mask, logits, -1e9), -1)
    acting, passing = mask & (classes != PASS), mask & (classes == PASS)
    top = torch.where(mask, logits, -1e9).argmax(-1)
    return (torch.where(acting, prob, 0.).sum(-1), torch.where(passing, prob, 0.).amax(-1),
            acting.gather(1, top[:, None])[:, 0])


def tally_decode(table, logits, mask, classes, sets):
    """What a grouped decode (act or pass by group mass, then the best candidate inside)
    would change against top-1, counted per row set ({name: [B] mask})."""
    act_mass, best_pass, top_act = grouped(logits, mask, classes)
    columns = (torch.ones_like(top_act), top_act, ~top_act, ~top_act & (act_mass > .5),
               ~top_act & (act_mass > best_pass), top_act & (act_mass <= .5))
    for name, chosen in sets.items():
        row = table.setdefault(name, [0] * len(DECODE_COLUMNS))
        for i, column in enumerate(columns):
            row[i] += int((chosen & column).sum())


def capture(args):
    import dataset4 as D4
    from torch.utils.data import DataLoader
    from sampler4 import StatefulBatchSampler4
    from value_calibration4.checkpoint import load_source
    from .coarse import CoarseClasses
    from .deferred import deferred_readouts
    from .execution import encode_boards, plan_batch, transfer_batch

    torch.set_num_threads(args.threads)
    _, _, net = load_source(args.model)
    configure(net, args)
    data = Path(args.data).expanduser()
    dataset = D4.TrajectoryDataset4(data, encoder_version=args.encoder_version, split=args.split,
                                    unroll_len=args.unroll)
    D.check_datasets([dataset], True)
    dataset.coarse = CoarseClasses.load(data / "bundle-distill")
    total = len(dataset.trajectories)
    dataset.trajectories = dataset.trajectories[args.offset::args.every]
    scope = D.Scope.from_options(tuple(int(k) for k in args.kinds.split(",")), 1.0,
                                 tuple(s for s in args.subtypes.split(",") if s), {}, [dataset], coarse="act")
    loader = DataLoader(dataset, batch_sampler=StatefulBatchSampler4(dataset, args.batch_size, shuffle=False),
                        collate_fn=D4.collate_trajectories, num_workers=args.workers)
    device = torch.device("cpu")
    rows, flags, kept, decode = [], {}, {"act": 0, "pass": 0}, {}
    hidden, began, batches = None, time.perf_counter(), 0
    recorder = Recorder(net)
    with torch.no_grad():
        for host in loader:
            plans = plan_batch(host, 0, args.thinking_steps)
            batch = transfer_batch(host, device)
            b, unroll = batch["step_valid"].shape
            new = batch["is_new_traj"]
            hidden = (torch.zeros(b, M.RECURRENT_SLOTS, net.cfg.d_model) if hidden is None or hidden.shape[0] != b
                      else torch.where(new[:, None, None], torch.zeros_like(hidden), hidden))
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            boards = encode_boards(net, batch, args.board_chunk, device)
            with recorder:
                prepared, final = deferred_readouts(net, batch, boards, hidden, plans, net.decision_chunk, device, False)
            wanted = {}
            for t, plan in enumerate(plans):
                if not plan.heads[0]:
                    continue
                joined, split, r_act, n_act, idx_differ = directions(batch, t, plan.candidates)
                for lane in joined.nonzero()[:, 0].tolist():
                    key = ("R act" if r_act[lane] else "R pass") + (" / N act" if n_act[lane] else " / N pass")
                    cell = flags.setdefault(key, {})
                    flag = f"flag={'N!=R' if batch['n_ne_r'][lane, t] else 'N=R'} idx={'differ' if idx_differ[lane] else 'same'}"
                    cell[flag] = cell.get(flag, 0) + 1
                for lane in split.nonzero()[:, 0].tolist():
                    direction = "act" if r_act[lane] else "pass"
                    if kept[direction] < args.rows:
                        kept[direction] += 1
                        wanted.setdefault(t, []).append((lane, direction))
                starts = batch["coarse_chain_start"][:, t] if "coarse_chain_start" in batch else torch.zeros_like(joined)
                tally_decode(decode, prepared[t][1].float(), batch["cand_mask"][:, t, :plan.candidates],
                             batch["coarse_class"][:, t, :plan.candidates],
                             dict(all=joined, chain_start=joined & starts, r_act=joined & r_act,
                                  r_pass=joined & ~r_act, split_r_act=split & r_act))
            rows.extend(cached_rows(recorder.groups(plans, b, unroll, net.decision_chunk), batch, plans, prepared, wanted))
            net.clear_structure()
            hidden = final
            batches += 1
            if batches % 25 == 0:
                print(f"[capture] batches={batches} rows={kept} seconds={time.perf_counter() - began:.0f}", flush=True)
            if min(kept.values()) >= args.rows:
                break
    meta = dict(model=str(args.model), data=str(data), split=args.split, every=args.every, offset=args.offset,
                trajectories=len(dataset.trajectories), of_trajectories=total, batches=batches,
                seconds=round(time.perf_counter() - began), rows=kept, flags=flags,
                grouped_decode={name: dict(zip(DECODE_COLUMNS, row)) for name, row in decode.items()},
                scope=dict(kinds=scope.kinds, subtypes=scope.subtypes, subtype_names=scope.subtype_names))
    torch.save(dict(meta=meta, rows=rows), args.out)
    print("[capture] " + json.dumps(meta), flush=True)


def load_rows(paths):
    rows, metas = [], []
    for path in paths:
        saved = torch.load(path, map_location="cpu", weights_only=False)
        rows.extend(saved["rows"])
        metas.append(saved["meta"])
    if len({json.dumps(m["scope"], sort_keys=True) for m in metas}) != 1:
        raise ValueError("The caches distill different scopes")
    return rows, metas


def stack(rows):
    """Pads rows to one batch: candidates masked off, board keys padded."""
    n, width = len(rows), max(len(row["mask"]) for row in rows)
    length = max(len(row["pad"]) for row in rows)
    d = rows[0]["g"].shape[-1]

    def pad_to(name, size, value=0., dim=0):
        out = []
        for row in rows:
            tensor = row[name]
            short = size - tensor.shape[dim]
            if short:
                shape = list(tensor.shape)
                shape[dim] = short
                tensor = torch.cat((tensor, torch.full(shape, value, dtype=tensor.dtype)), dim)
            out.append(tensor)
        return torch.stack(out)

    out = dict(c_feat=pad_to("c_feat", width), frozen=pad_to("frozen", width), ent=pad_to("ent", width),
               seat=pad_to("seat", width), mask=pad_to("mask", width, False),
               coarse_class=pad_to("coarse_class", width, -1), logits=pad_to("logits", width, -1e9),
               tokens=pad_to("tokens", length), pad=pad_to("pad", length, True),
               g=torch.stack([row["g"] for row in rows]))
    for name in FIELDS:
        if name in rows[0]:
            out[name] = torch.stack([row[name] for row in rows])
    assert out["g"].shape == (n, d)
    return out


def head_logits(net, rows):
    """[N, C] candidate logits from the cached frozen inputs: `candidate_hidden` after its
    frozen parts, then `score_candidate_hidden`, as `deferred_readouts` calls them."""
    c_emb = net.cand_proj(symlog(rows["c_feat"])) + rows["frozen"]
    cand_in = torch.cat([c_emb, rows["ent"], rows["seat"], rows["g"].unsqueeze(-2).expand_as(c_emb)], dim=-1)
    hidden = net.cand_scorer[2](net.cand_scorer[1](net.cand_scorer[0](cand_in)))
    keys = (rows["tokens"], rows["pad"])
    if getattr(net, "cand_decoder", None) is not None:
        hidden = net.decode_candidates(hidden, keys)
    thinking = keys if getattr(net, "deliberation", None) is not None else None
    return score_candidate_hidden(net, hidden, rows["mask"], thinking)


def loss_inputs(rows, logits, reference):
    """`distill_loss`'s arguments for a one-step unroll of the cached rows."""
    batch = {name: rows[name][:, None] for name in FIELDS if name in rows}
    batch.update(cand_mask=rows["mask"][:, None], coarse_class=rows["coarse_class"][:, None])
    zero = torch.zeros(len(logits))
    plans = [SimpleNamespace(heads=(True, False, False, False))]
    return [(None, logits, None, None, zero)], [(None, reference, None, None, zero)], batch, plans


def per_row(logits, rows):
    """Per row: ce to R's group (the loss's expression), P(R's group), top-1 in R's group,
    P(pass), the best act candidate's probability, act candidates on offer."""
    mask = rows["mask"]
    log_s = torch.log_softmax(torch.where(mask, logits, -1e9), -1)
    groups = (rows["coarse_class"] != PASS).long()
    r_group = groups.gather(1, rows["single_idx"].clamp(min=0)[:, None])
    ours = mask & (groups == r_group)
    ce = -torch.logsumexp(torch.where(ours, log_s, -1e9), -1)
    top = torch.where(mask, logits, -1e9).argmax(-1)
    passing, acting = mask & (groups == 0), mask & (groups == 1)
    prob = log_s.exp()
    return dict(ce=ce, p_group=(-ce).exp(), top1_group=ours.gather(1, top[:, None])[:, 0],
                p_pass=torch.where(passing, prob, 0.).sum(-1), best_act=torch.where(acting, prob, 0.).amax(-1),
                acts_offered=acting.sum(-1).float(), top=top)


def scope_of(meta, coarse="act"):
    scope = meta["scope"]
    return D.Scope(kinds=tuple(scope["kinds"]), subtypes=None if scope["subtypes"] is None else tuple(scope["subtypes"]),
                   subtype_names=tuple(scope["subtype_names"]), coarse=coarse)


def report(net, rows, scope, chunk=256):
    """Per direction: rows, ce_coarse_r, top-1 in R's group, mean P(R's group), P(R's group) > .5,
    mean P(pass) and best act probability; and `distill_loss`'s own ce_coarse_r over all rows."""
    sums, stats = {}, torch.zeros(2 * len(D.TERMS))
    with torch.no_grad():
        for start in range(0, len(rows), chunk):
            part = rows[start:start + chunk]
            batch = stack(part)
            logits = head_logits(net, batch).float()
            stats += D.distill_loss(*loss_inputs(batch, logits, batch["logits"]), scope)[1]
            values = per_row(logits, batch)
            for i, row in enumerate(part):
                cell = sums.setdefault(row["direction"], dict(rows=0, ce=0., top1_group=0, p_group=0., group_over_half=0,
                                                               p_pass=0., best_act=0., acts_offered=0.))
                cell["rows"] += 1
                for name in ("ce", "p_group", "p_pass", "best_act", "acts_offered"):
                    cell[name] += float(values[name][i])
                cell["top1_group"] += int(values["top1_group"][i])
                cell["group_over_half"] += int(values["p_group"][i] > .5)
    out = {}
    for direction, cell in sorted(sums.items()):
        n = cell.pop("rows")
        out[direction] = dict(rows=n, **{name: round(value / n, 4) for name, value in cell.items()})
    out["loss_ce_coarse_r"] = round(float(D.term_means(stats)["ce_coarse_r"]), 4)
    return out


def load_head(path, reference_net=None):
    from value_calibration4.checkpoint import load_source
    from .distill_audit import same_upstream
    _, _, net = load_source(path)
    if reference_net is not None and not same_upstream(net, reference_net):
        raise ValueError(f"{path}: weights before the heads differ from the capture's net")
    return net


def evaluate(args):
    torch.set_num_threads(args.threads)
    rows, metas = load_rows(args.cache)
    capture_net = load_head(metas[0]["model"])
    scope = scope_of(metas[0])
    with torch.no_grad():
        batch = stack(rows[:args.check_rows])
        drift = (head_logits(capture_net, batch).float() - batch["logits"]).where(batch["mask"], 0.).abs().max()
    out = dict(rows=len(rows), capture_flags=[m["flags"] for m in metas],
               cached_head_vs_capture_max_abs_logit_diff=float(drift), heads={})
    for path in args.head:
        out["heads"][path] = report(load_head(path, capture_net), rows, scope)
    text = json.dumps(out, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(text, flush=True)


def show(args):
    rows, metas = load_rows(args.cache)
    heads = [load_head(path) for path in args.head]
    chosen = [row for row in rows if row["direction"] == args.direction][:args.rows]
    batch = stack(chosen)
    names = D.ACTION_NAMES
    with torch.no_grad():
        logits = [head_logits(net, batch).float() for net in heads]
    subtypes = metas[0]["scope"]["subtype_names"]
    for i, row in enumerate(chosen):
        n = len(row["mask"])
        groups = (row["coarse_class"] != PASS).long()
        r_idx, n_idx = int(row["single_idx"]), int(row["coarse_n_idx"])
        actions = D.action_classes(row["c_feat"]).tolist()
        target = (row["mask"] & (groups == groups[r_idx])).nonzero()[:, 0].tolist()
        code = int(row["distill_subtype"])
        print(json.dumps(dict(
            row=i, subtype=subtypes[code] if 0 <= code < len(subtypes) else None, candidates=n,
            head_width=row["width"], offered=int(row["mask"].sum()), r_idx=r_idx, n_idx=n_idx,
            n_ne_r=bool(row["n_ne_r"]), act_target=target,
            top1={path: int(lg[i, :n].argmax()) for path, lg in zip(args.head, logits)},
            p_r_group={path: round(float(torch.softmax(lg[i, :n].where(row["mask"], -1e9), -1)[target].sum()), 4)
                       for path, lg in zip(args.head, logits)},
            menu=[[c, CLASSES[int(row["coarse_class"][c])] if row["coarse_class"][c] >= 0 else "?", names[actions[c]],
                   bool(row["mask"][c])] + [round(float(lg[i, c]), 3) for lg in logits] for c in range(n)])),
            flush=True)


def fit(args):
    from value_calibration4.checkpoint import load_source
    from .optimizer_groups import build_optimizer_groups
    from .optimizer_step import update_parameters
    from .partial import apply_partial_training
    from .rules_only import ParameterAnchor

    torch.set_num_threads(args.threads)
    torch.manual_seed(args.seed)
    rows, metas = load_rows(args.cache)
    scope = scope_of(metas[0], args.coarse)
    _, _, net = load_source(args.init)
    apply_partial_training(net, HEAD_ONLY)
    anchor = ParameterAnchor(net, args.anchor_l2) if args.anchor_l2 > 0 else None
    groups, _ = build_optimizer_groups(net, args.lr, architecture="v5", decay=anchor is None)
    optimizer = torch.optim.AdamW(groups)
    writer = SimpleNamespace(before_mutation=lambda: None)
    order, updates, began = list(range(len(rows))), 0, time.perf_counter()
    shuffle = random.Random(args.seed)
    print("[fit] epoch=0 " + json.dumps(report(net, rows, scope)), flush=True)
    for epoch in range(1, args.epochs + 1):
        shuffle.shuffle(order)
        for start in range(0, len(order), args.batch_rows):
            batch = stack([rows[i] for i in order[start:start + args.batch_rows]])
            optimizer.zero_grad(set_to_none=True)
            loss, _ = D.distill_loss(*loss_inputs(batch, head_logits(net, batch).float(), batch["logits"]), scope)
            if anchor is not None:
                loss = loss + anchor.penalty()
            updates += int(update_parameters(loss, net, optimizer, writer, None, updates + 1))
        if epoch % args.report_every == 0 or epoch == args.epochs:
            line = dict(epoch=epoch, updates=updates, seconds=round(time.perf_counter() - began),
                        anchor_distance=None if anchor is None else round(anchor.distance(), 4),
                        **report(net, rows, scope))
            print("[fit] " + json.dumps(line), flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    one = commands.add_parser("capture")
    one.add_argument("--model", required=True, help="A net whose weights before the heads are r1's (r1 itself)")
    one.add_argument("--data", required=True)
    one.add_argument("--split", default="val")
    one.add_argument("--every", type=int, default=8)
    one.add_argument("--offset", type=int, default=0)
    one.add_argument("--rows", type=int, default=1000, help="Rows to keep per direction")
    one.add_argument("--kinds", default="0,8")
    one.add_argument("--subtypes", default="cast,pass,instant_their_turn,spend,target")
    one.add_argument("--encoder-version", type=int, default=10)
    one.add_argument("--unroll", type=int, default=32)
    one.add_argument("--thinking-steps", type=int, default=2)
    one.add_argument("--batch-size", type=int, default=8)
    one.add_argument("--board-chunk", type=int, default=32)
    one.add_argument("--decision-chunk", type=int, default=4)
    one.add_argument("--board-token-budget", type=int, default=32768)
    one.add_argument("--workers", type=int, default=2)
    one.add_argument("--out", required=True)
    for name in ("evaluate", "show", "fit"):
        command = commands.add_parser(name)
        command.add_argument("--cache", action="append", required=True)
    for command in (commands.choices["evaluate"], commands.choices["show"]):
        command.add_argument("--head", action="append", required=True, help="Checkpoints whose candidate head to read")
    commands.choices["evaluate"].add_argument("--check-rows", type=int, default=64,
                                              help="Rows on which the cached head must reproduce the capture's logits")
    commands.choices["evaluate"].add_argument("--out", default="")
    commands.choices["show"].add_argument("--direction", default="act")
    commands.choices["show"].add_argument("--rows", type=int, default=20)
    fitting = commands.choices["fit"]
    fitting.add_argument("--init", required=True)
    fitting.add_argument("--epochs", type=int, default=40)
    fitting.add_argument("--lr", type=float, default=6e-5)
    fitting.add_argument("--anchor-l2", type=float, default=0.)
    fitting.add_argument("--batch-rows", type=int, default=32)
    fitting.add_argument("--report-every", type=int, default=5)
    fitting.add_argument("--seed", type=int, default=73501)
    fitting.add_argument("--coarse", default="act", choices=("act", "act-exact"), help="The coarse loss the fit trains")
    for command in commands.choices.values():
        command.add_argument("--threads", type=int, default=8)
    args = parser.parse_args(argv)
    {"capture": capture, "evaluate": evaluate, "show": show, "fit": fit}[args.command](args)


if __name__ == "__main__":
    main()
