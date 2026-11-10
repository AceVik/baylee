"""Opt-in bundle distillation (`--distill-bundle`) on `bundle_distillation_v1` datasets.

The dataset's ordinary policy targets hold R, the tactical layer's answer with
the bundle, at every labelled decision of the recorded seats (loss weight 1;
every other decision weight 0); header flag bit 7 marks where N, the net's own
greedy answer, differs (`n_ne_r`).

The loss changes the student only where the bundle disagrees with the net, and
only in scope (`Scope`: pending kinds, priority by default, optionally narrowed
to subtypes such as cast/pass/instant_their_turn/spend):
- cross-entropy to R at in-scope decisions where N != R, optionally weighted by
  the rule groups that fired there (product of their weights, others 1);
- beta * KL(r1 || student) everywhere else: in-scope decisions where N = R, the
  same kinds' out-of-scope subtypes, other single choices, subsets and pairs
  (Bernoulli per free entry), and the squared distance to r1's scalar. Each term
  is reported apart.
Outcome weighting does not apply: the label is R whatever the game's result.
Value and auxiliary targets are not part of this loss.

The reference r1 is a frozen copy of the --init weights. When nothing before the
heads learns (stage A) it reads the student's own boards and memory, a second
readout and no trunk. With frozen layers below L (stage B) it runs only its own
layers L and above on the inputs the student's layer L received. Otherwise it
encodes its own boards. Without a shared trunk it carries its own memory.

`AUXILIARY_TARGETS` is the hook for stage B's extra auxiliary heads: each entry
`name -> fn(student_outputs, batch, plans) -> loss` joins the total. Empty today.

A student with the deliberation block (thinking revision 2, `--think-supervision`)
adds `thinking_terms` through the same hook: every step of the block gets this
loss's own target (R in scope where N != R, r1 elsewhere), a penalty for being
worse than the step before, and a confidence trained on whether its answer is
that target. The reference never deliberates: its block is switched off, so it
stays r1 whatever the student's block learns.
"""
from __future__ import annotations

from contextlib import contextmanager
import copy
from dataclasses import dataclass

import torch
import torch.nn.functional as F

import model4 as M
from .deferred import deferred_readouts
from .execution import encode_boards, plan_batch, transfer_batch

CONTRACT = "bundle_distillation_v1"
N_NE_R_FLAG = 128  # BST4 header flags bit 7
TERMS = ("ce_r", "kl_distilled_n_eq_r", "kl_kind_out_of_scope", "kl_single", "kl_subset", "kl_pair", "sq_scalar")
AUXILIARY_TARGETS = {}


def dataset_contract(meta):
    value = meta.get("bundle_distillation")
    return value.get("contract") if isinstance(value, dict) else None


def check_datasets(datasets, enabled):
    """Fail closed both ways: R labels train only under --distill-bundle.

    Also hands each dataset the subtype and rule-group vocabularies its frames index.
    """
    for dataset in datasets:
        meta = getattr(dataset, "meta", None) or {}
        contract = dataset_contract(meta)
        if enabled and contract != CONTRACT:
            raise ValueError(f"--distill-bundle needs {CONTRACT} datasets")
        if not enabled and contract is not None:
            raise ValueError(f"{contract} labels train only under --distill-bundle")
        dataset.distillation = enabled
        if enabled:
            dataset.distill_subtypes = list(meta["bundle_distillation"].get("subtypes") or [])
            dataset.distill_groups = list(meta["bundle_distillation"].get("groups") or [])


@dataclass(frozen=True)
class Scope:
    """What is distilled to R; everything else stays at r1 through the KL."""
    kinds: tuple = (0,)
    subtypes: tuple | None = None
    subtype_names: tuple = ()
    group_weights: tuple | None = None
    kl_weight: float = 1.0

    @classmethod
    def from_options(cls, kinds, kl_weight, subtypes, group_weights, datasets):
        """Names resolved against vocabularies every dataset shares; unknown names fail."""
        vocabularies = {(tuple(d.distill_subtypes), tuple(d.distill_groups)) for d in datasets}
        if len(vocabularies) != 1:
            raise ValueError("Training and validation distillation vocabularies differ")
        (subtype_names, group_names), = vocabularies
        codes = None
        if subtypes:
            missing = [name for name in subtypes if name not in subtype_names]
            if missing:
                raise ValueError(f"Dataset declares no distillation subtype {missing}")
            codes = tuple(sorted(subtype_names.index(name) for name in subtypes))
        weights = None
        if group_weights:
            missing = [name for name in group_weights if name not in group_names]
            if missing:
                raise ValueError(f"Dataset declares no rule group {missing}")
            weights = tuple(float(group_weights.get(name, 1.0)) for name in group_names)
        return cls(tuple(kinds), codes, subtype_names, weights, kl_weight)

    def masks(self, batch, device):
        """[B, U] masks: in the kinds, and in scope (kind and subtype)."""
        in_kind = torch.isin(batch["pending_kind"].to(device), torch.tensor(self.kinds, device=device))
        if self.subtypes is None:
            return in_kind, in_kind
        subtype = batch["distill_subtype"].to(device)
        return in_kind, in_kind & torch.isin(subtype, torch.tensor(self.subtypes, device=device))

    def multiplier(self, batch, device):
        """[B, U] cross-entropy weight: product of the fired groups' weights (others 1)."""
        if self.group_weights is None:
            return None
        weights = torch.tensor(self.group_weights, device=device)
        fired = batch["fired_mask"].to(device)
        bits = (fired[..., None] >> torch.arange(len(weights), device=device)) & 1
        return torch.where(bits.bool(), weights, 1.).prod(-1)


class Reference:
    """Frozen r1 and its outputs on the student's batch."""

    @staticmethod
    def snapshot(net):
        """Copy the --init weights before any freezing or adapter; rules are attached later."""
        rules = net.rules
        net.rules = None
        try:
            return copy.deepcopy(net)
        finally:
            net.rules = rules

    def __init__(self, copied, student, trainable_from_layer=-1):
        from .partial import upstream_frozen, below_layer_frozen
        rules = student.rules
        rules_frozen = not any(p.requires_grad for p in rules.parameters())
        copied.rules = rules if rules_frozen else copy.deepcopy(rules)
        self.net = copied
        self.own_rules = not rules_frozen
        self.net.eval()
        block = getattr(self.net, "deliberation", None)
        if block is not None:
            # r1 is the heads' first proposal; zero steps return it exactly.
            block.steps_override, block.collect = 0, False
        for parameter in self.net.parameters():
            parameter.requires_grad_(False)
        # Stage A: everything before the heads is frozen, so boards and memory are r1's.
        self.shared_trunk = upstream_frozen(student)
        # Stage B: what feeds layer L is frozen, so r1 differs from layer L on.
        self.shared_lower = (trainable_from_layer if not self.shared_trunk and trainable_from_layer >= 0
                             and below_layer_frozen(student, trainable_from_layer) else None)
        self.captured = []
        self.hidden = None

    @contextmanager
    def capturing(self, student):
        """Record the inputs of the student's layer L during its board encoding."""
        self.captured = []
        if self.shared_lower is None:
            yield
            return
        student.capture_layer_input = (self.shared_lower, self.captured)
        try:
            yield
        finally:
            student.capture_layer_input = None

    def _upper_boards(self, lanes):
        """r1's layers L and above on each captured chunk, split back into frames."""
        boards = []
        for inputs, pad in self.captured:
            x = inputs
            for layer in self.net.trunk_layers[self.shared_lower:]:
                x = layer(x, src_key_padding_mask=pad)
            board = self.net.trunk_norm(x)
            frames = board.shape[0] // lanes
            board = board.reshape(lanes, frames, *board.shape[1:])
            mask = pad.reshape(lanes, frames, *pad.shape[1:])
            boards.extend((board[:, t], mask[:, t]) for t in range(frames))
        return boards

    def outputs(self, host, batch, boards, hidden, plans, device, bf16, chunk, resets=None):
        with torch.no_grad():
            b, unroll = batch["step_valid"].shape
            if self.own_rules:
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    self.net.clear_structure()
                    self.net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            if self.shared_trunk:
                ref_boards, ref_hidden = boards, hidden.detach()
            else:
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    ref_boards = self._upper_boards(b) if self.captured else None
                    if ref_boards is None or len(ref_boards) != unroll:
                        ref_boards = encode_boards(self.net, batch, chunk, device)
                if resets is not None or self.hidden is None or self.hidden.shape[0] != b:
                    self.hidden = torch.zeros_like(hidden)
                new = batch["is_new_traj"].to(device, non_blocking=True)
                ref_hidden = torch.where(new[:, None, None], torch.zeros_like(self.hidden), self.hidden)
            prepared, final = deferred_readouts(self.net, batch, ref_boards, ref_hidden, plans,
                                                self.net.decision_chunk, device, bf16, resets=resets)
            if not self.shared_trunk:
                self.hidden = final
            if self.own_rules:
                self.net.clear_structure()
            self.captured = []
        return prepared


def _bernoulli_kl(student, reference):
    """KL(Bernoulli(sigmoid(reference)) || Bernoulli(sigmoid(student))) per entry."""
    p = torch.sigmoid(reference)
    return (p * (F.logsigmoid(reference) - F.logsigmoid(student))
            + (1 - p) * (F.logsigmoid(-reference) - F.logsigmoid(-student)))


def distill_loss(prepared, reference, batch, plans, scope, extra=None):
    """Returns (loss, stats [2 * len(TERMS)]): per term weighted sum, then weight."""
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads = get("head")
    in_kind, in_scope = scope.masks(batch, device)
    multiplier = scope.multiplier(batch, device)
    differs = get("n_ne_r")
    sums = torch.zeros(len(TERMS), device=device)
    dens = torch.zeros(len(TERMS), device=device)
    for t, plan in enumerate(plans):
        for kind in range(4):
            if not plan.heads[kind]:
                continue
            selected = decision[:, t] & (heads[:, t] == kind)
            w = weight[:, t] * selected
            if kind == 0:
                student = prepared[t][1].float()
                ref = reference[t][1].float()
                mask = get("cand_mask")[:, t, :student.shape[1]]
                genuine = selected & (mask.sum(-1) > 1)
                distilled = genuine & in_scope[:, t]
                target = torch.where(distilled & differs[:, t], get("single_idx")[:, t], 0)
                log_s = torch.log_softmax(torch.where(mask, student, -1e9), -1)
                log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
                ce = -log_s.gather(1, target[:, None])[:, 0]
                kl = torch.where(mask, log_r.exp() * (log_r - log_s), 0.).sum(-1)
                for index, rows, values in ((0, distilled & differs[:, t], ce),
                                            (1, distilled & ~differs[:, t], kl),
                                            (2, genuine & in_kind[:, t] & ~in_scope[:, t], kl),
                                            (3, genuine & ~in_kind[:, t], kl)):
                    rw = weight[:, t] * rows
                    if index == 0 and multiplier is not None:
                        rw = rw * multiplier[:, t]
                    sums[index] = sums[index] + (torch.where(rows, values, 0.) * rw).sum()
                    dens[index] = dens[index] + rw.sum()
            elif kind in (1, 2):
                prefix = "cand" if kind == 1 else "pair"
                student = prepared[t][kind].float()
                ref = reference[t][kind].float()
                width = student.shape[1]
                mask = get(prefix + "_mask")[:, t, :width]
                free = get(prefix + "_free_mask")[:, t, :width]
                eligible = selected[:, None] & mask & (free > 0)
                kl = torch.where(eligible, _bernoulli_kl(torch.where(eligible, student, 0.),
                                                         torch.where(eligible, ref, 0.)), 0.)
                ew = torch.where(eligible, free, 0.) * w[:, None]
                index = 4 if kind == 1 else 5
                sums[index] = sums[index] + (kl * ew).sum()
                dens[index] = dens[index] + ew.sum()
            else:
                error = torch.where(selected, prepared[t][3].float() - reference[t][3].float(), 0.)
                sums[6] = sums[6] + (error.square() * w).sum()
                dens[6] = dens[6] + w.sum()
    means = sums / dens.clamp(min=1e-6)
    total = means[0] + scope.kl_weight * means[1:].sum()
    for name, function in (extra or {}).items():
        total = total + function(prepared, batch, plans)
    return total, torch.cat((sums.detach(), dens.detach()))


def thinking_terms(frames, reference, batch, plans, scope, config):
    """Per-step terms of the deliberation block under this loss's targets.

    `frames[t]["logits"]` is `[S + 1, B, C]` (step 0: the heads' first proposal).
    A step's loss at a decision is the distillation loss it would have as the
    final answer: cross-entropy to R at in-scope single choices where N != R
    (with the rule groups' weights), `kl_weight` * KL(r1 || step) at every other
    single choice and at subsets. From these: the mean over the steps before
    the last (deep supervision; the last step is `distill_loss`'s own), a
    penalty where a step is worse than the one before, and the confidence
    against "this step's answer is its target" (R, or r1's own answer). The
    consequence term is left out: the game's outcome belongs to the answer that
    was played, not to R. Blocker pairs and scalars are not deliberated.

    Returns (loss, stats): per step the in-scope N != R decisions answered as
    R `[S + 1]`, then their count.
    """
    device = next((f["logits"].device for f in frames or () if f is not None), None)
    if device is None:
        raise ValueError("No thinking trace: the deliberation block must collect during deferred readouts")
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads = get("head")
    _, in_scope = scope.masks(batch, device)
    multiplier = scope.multiplier(batch, device)
    differs = get("n_ne_r")
    zero = torch.zeros((), device=device)
    deep = improve = confidence = weights = decisions = count = zero
    right_at_r = None
    for t, (frame, plan) in enumerate(zip(frames, plans, strict=True)):
        if frame is None or frame["logits"].shape[0] < 2 or not (plan.heads[0] or plan.heads[1]):
            continue
        logits = frame["logits"].float()
        steps, width = logits.shape[0] - 1, logits.shape[2]
        mask = get("cand_mask")[:, t, :width]
        ref = reference[t][1].float()
        parts = []
        if plan.heads[0]:
            genuine = decision[:, t] & (heads[:, t] == 0) & (mask.sum(-1) > 1)
            to_r = genuine & in_scope[:, t] & differs[:, t]
            target = torch.where(to_r, get("single_idx")[:, t], 0)
            log_s = torch.log_softmax(torch.where(mask[None], logits, -1e9), -1)
            log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
            ce = -log_s.gather(2, target[None, :, None].expand(steps + 1, -1, 1))[..., 0]
            kl = torch.where(mask[None], log_r.exp()[None] * (log_r[None] - log_s), 0.).sum(-1)
            ce_weight = multiplier[:, t] if multiplier is not None else torch.ones_like(weight[:, t])
            losses = torch.where(to_r[None], ce * ce_weight[None], scope.kl_weight * kl)
            answer = torch.where(mask[None], logits, -1e9).argmax(-1)
            wanted = torch.where(to_r, target, torch.where(mask, ref, -1e9).argmax(-1))
            right = (answer == wanted[None]) & genuine[None]
            hits = (right & to_r[None]).sum(-1).to(losses.dtype)
            right_at_r = hits if right_at_r is None else right_at_r + hits
            count = count + to_r.sum()
            parts.append((losses, genuine, right))
        if plan.heads[1]:
            selected = decision[:, t] & (heads[:, t] == 1)
            free = get("cand_free_mask")[:, t, :width]
            eligible = selected[:, None] & mask & (free > 0)
            entries = torch.where(eligible[None], _bernoulli_kl(torch.where(eligible[None], logits, 0.),
                                                                torch.where(eligible, ref, 0.)[None]), 0.)
            free = torch.where(eligible, free, 0.)
            losses = scope.kl_weight * (entries * free[None]).sum(-1) / free.sum(-1).clamp(min=1.)[None]
            genuine = eligible.any(-1)
            right = ~(eligible[None] & ((logits > 0) != (ref > 0)[None])).any(-1) & genuine[None]
            parts.append((losses, genuine, right))
        for losses, genuine, right in parts:
            w = weight[:, t] * genuine
            if steps > 1:
                deep = deep + (losses[1:steps] * w).sum() / (steps - 1)
            gain = F.relu(losses[1:] - losses[:-1].detach() + config.improve_margin)
            improve = improve + (gain * w).sum() / steps
            weights = weights + w.sum()
            known = genuine.to(losses.dtype)
            confidence = confidence + (F.binary_cross_entropy_with_logits(
                frame["confidence"].float(), right[1:].to(losses.dtype), reduction="none") * known).sum() / steps
            decisions = decisions + known.sum()
    total = (config.deep_weight * deep / weights.clamp(min=1e-6)
             + config.improve_weight * improve / weights.clamp(min=1e-6)
             + config.confidence_weight * confidence / decisions.clamp(min=1.))
    if right_at_r is None:
        return total, torch.zeros(2, device=device)
    return total, torch.cat((right_at_r.detach(), count.detach().to(right_at_r.dtype)[None]))


def term_means(stats):
    n = len(TERMS)
    return {name: float(stats[i] / stats[n + i]) if float(stats[n + i]) > 0 else None
            for i, name in enumerate(TERMS)}


GROUPS = ("distilled", "out_of_scope", "single", "subset", "pair", "scalar")


def agreement(prepared, batch, plans, scope, scalar_tolerance):
    """name -> [decisions, N != R, agree with R, agree with R at N != R] (device float64).

    `distilled` pools the scope; `out_of_scope` is the same kinds' other subtypes;
    `subtype:<name>` (and `subtype:unknown`) splits every decision of the kinds.
    Single choices agree when the argmax is R. Subsets and pairs agree when the
    threshold-0 decode equals R on every free entry (the min/max choice limits
    the actor's decoder applies are not in the frame). Scalars agree within
    `scalar_tolerance` of R's normalized value.
    """
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    decision = get("is_decision") & valid & (get("loss_weight") > 0)
    heads = get("head")
    in_kind, in_scope = scope.masks(batch, device)
    differs = get("n_ne_r")
    subtype = batch["distill_subtype"].to(device) if "distill_subtype" in batch else None
    table = {}

    def add(name, chosen, agree, t):
        row = torch.stack((chosen.sum(), (chosen & differs[:, t]).sum(),
                           (chosen & agree).sum(), (chosen & agree & differs[:, t]).sum())).double()
        table[name] = table[name] + row if name in table else row

    for t, plan in enumerate(plans):
        for kind in range(4):
            if not plan.heads[kind]:
                continue
            selected = decision[:, t] & (heads[:, t] == kind)
            if kind == 0:
                logits = prepared[t][1].float()
                mask = get("cand_mask")[:, t, :logits.shape[1]]
                genuine = selected & (mask.sum(-1) > 1)
                agree = torch.where(mask, logits, -1e9).argmax(-1) == get("single_idx")[:, t]
                add("distilled", genuine & in_scope[:, t], agree, t)
                add("out_of_scope", genuine & in_kind[:, t] & ~in_scope[:, t], agree, t)
                add("single", genuine & ~in_kind[:, t], agree, t)
                if subtype is not None:
                    for code, name in [(-1, "unknown"), *enumerate(scope.subtype_names)]:
                        add(f"subtype:{name}", genuine & in_kind[:, t] & (subtype[:, t] == code), agree, t)
            elif kind in (1, 2):
                prefix = "cand" if kind == 1 else "pair"
                logits = prepared[t][kind].float()
                width = logits.shape[1]
                eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
                target = get(prefix + "_target_mask")[:, t, :width] > .5
                agree = ~(eligible & ((logits > 0) != target)).any(-1)
                add("subset" if kind == 1 else "pair", selected & eligible.any(-1), agree, t)
            else:
                agree = (prepared[t][3].float() - get("scalar_target")[:, t]).abs() <= scalar_tolerance
                add("scalar", selected, agree, t)
    return table


@torch.no_grad()
def evaluate_distill(net, reference, loader, device, scope, board_chunk, scalar_tolerance,
                     thinking_steps=2, bf16=None, think=None):
    """Held-out agreement with R by group and subtype and every KL guard, over whole trajectories.

    With the thinking terms' configuration `think`, also the share of in-scope
    N != R decisions each deliberation step answers as R
    (`val_distill_think_top1_r_at_n_ne_r_by_step`; step 0 is the heads' first
    proposal, the last step is the net's answer).
    """
    think_steps = None
    from .prefetch import prefetch_batches
    net.eval()
    bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported() if bf16 is None else bf16
    table = {}
    stats = torch.zeros(2 * len(TERMS), device=device)
    value = torch.zeros(3, dtype=torch.float64, device=device)
    hidden = None
    reference.hidden = None
    for host, staged in prefetch_batches(loader, device, getattr(net, "gpu_prefetch", 0)):
        plans = plan_batch(host, 0, thinking_steps)
        batch = transfer_batch(host, device) if staged is None else staged
        b = batch["step_valid"].shape[0]
        new = batch["is_new_traj"].to(device, non_blocking=True)
        hidden = (torch.zeros(b, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
                  if hidden is None or hidden.shape[0] != b else
                  torch.where(new[:, None, None], torch.zeros_like(hidden), hidden))
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            with reference.capturing(net):
                boards = encode_boards(net, batch, board_chunk, device)
        prepared, final = deferred_readouts(net, batch, boards, hidden, plans, net.decision_chunk, device, bf16)
        ref = reference.outputs(host, batch, boards, hidden, plans, device, bf16, board_chunk)
        _, step_stats = distill_loss(prepared, ref, batch, plans, scope)
        stats += step_stats
        if think is not None and any(frame is not None for frame in net.thinking_frames or ()):
            _, by_step = thinking_terms(net.thinking_frames, ref, batch, plans, scope, think)
            if by_step.numel() > 2:
                think_steps = by_step if think_steps is None else think_steps + by_step
        for name, row in agreement(prepared, batch, plans, scope, scalar_tolerance).items():
            table[name] = table[name] + row if name in table else row
        valid = batch["step_valid"].to(device)
        mine = torch.stack([p[4].float() for p in prepared], 1)
        theirs = torch.stack([p[4].float() for p in ref], 1)
        value += torch.stack((((mine - batch["outcome"].to(device)).square() * valid).sum(),
                              ((mine - theirs).square() * valid).sum(), valid.sum())).double()
        net.clear_structure()
        hidden = final
    metrics = {f"val_distill_{name}": term for name, term in term_means(stats).items()}
    for group in GROUPS:
        table.setdefault(group, torch.zeros(4, dtype=torch.float64))
    for group, row in sorted(table.items()):
        count, differ, agree, agree_differ = row.cpu().tolist()
        metrics[f"val_distill_{group}_decisions"] = int(count)
        metrics[f"val_distill_{group}_n_ne_r"] = int(differ)
        metrics[f"val_distill_{group}_top1_r"] = agree / count if count else None
        metrics[f"val_distill_{group}_top1_r_at_n_ne_r"] = agree_differ / differ if differ else None
        metrics[f"val_distill_{group}_n_top1_r"] = (count - differ) / count if count else None
    if think_steps is not None:
        metrics["val_distill_think_n_ne_r"] = int(think_steps[-1])
        metrics["val_distill_think_top1_r_at_n_ne_r_by_step"] = (
            (think_steps[:-1] / think_steps[-1]).tolist() if float(think_steps[-1]) > 0 else None)
    sse, drift, frames = value.cpu().tolist()
    metrics.update(val_mse=sse / max(frames, 1), val_value_mse_to_r1=drift / max(frames, 1), val_frames=int(frames),
                   val_distill_selection=metrics["val_distill_distilled_top1_r_at_n_ne_r"])
    return metrics
