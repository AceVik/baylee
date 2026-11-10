"""Losses that make a deliberation step matter (thinking revision 2).

The ordinary policy loss reads the logits after the last step. These terms
read every step of the trace `deferred_readouts` leaves in
`net.thinking_frames` (step 0 is the heads' first proposal):

- deep supervision: the policy loss at the steps before the last;
- improvement: a step is penalised when it is worse than the step before it;
- confidence: each step's confidence against "this step's answer is the target";
- consequence: the predicted outcome of the target answer against the game's.

Targets, masks and weights are those of the supervised loss (`loss_v5`), so
with distillation targets the same terms teach corrections toward the teacher.
Single choices and subsets are covered; blocker pairs are not deliberated.
"""
from dataclasses import asdict, dataclass
import math

import torch
import torch.nn.functional as F


@dataclass(frozen=True)
class ThinkLossConfig:
    deep_weight: float = 1.0
    improve_weight: float = 0.5
    improve_margin: float = 0.0
    confidence_weight: float = 0.25
    consequence_weight: float = 0.1

    def __post_init__(self):
        if any(not math.isfinite(v) or v < 0 for v in asdict(self).values()):
            raise ValueError("Thinking loss weights must be finite and nonnegative")

    def to_dict(self):
        return {"version": 1, **asdict(self)}


def _step_losses(frame, batch, t, plan, outcome_weighting, get):
    """Per head kind: step losses `[S + 1, B]`, decision weights `[B]`, and
    whether each step's decoded answer is the target `[S + 1, B]`."""
    logits = frame["logits"].float()
    width = logits.shape[-1]
    valid = get("step_valid")[:, t]
    decision = get("is_decision")[:, t] & valid
    head = get("head")[:, t]
    weight = torch.where(valid, get("loss_weight")[:, t], 0.)
    outcome = torch.where(valid, get("outcome")[:, t], 0.)
    factor = (.35 + 1.30 * outcome) if outcome_weighting else torch.ones_like(outcome)
    mask = get("cand_mask")[:, t, :width]
    parts = []
    if plan.heads[0]:
        genuine = decision & (head == 0) & (mask.sum(-1) > 1) & (weight > 0)
        target = torch.where(genuine, get("single_idx")[:, t], 0)
        safe = torch.where(genuine[None, :, None], logits, 0.)
        losses = F.cross_entropy(safe.flatten(0, 1), target.repeat(logits.shape[0]),
                                 reduction="none").view(logits.shape[0], -1)
        right = (logits.argmax(-1) == target[None]) & genuine[None]
        parts.append((0, losses, weight * factor * genuine, right, genuine, target))
    if plan.heads[1]:
        selected = decision & (head == 1)
        free = get("cand_free_mask")[:, t, :width] * mask.float()
        eligible = selected[:, None] & mask & (free > 0) & (weight[:, None] > 0)
        target = torch.where(eligible, get("cand_target_mask")[:, t, :width], 0.)
        safe = torch.where(eligible[None], logits, 0.)
        entries = F.binary_cross_entropy_with_logits(safe, target[None].expand_as(safe), reduction="none")
        free = torch.where(eligible, free, 0.)
        losses = (entries * free[None]).sum(-1) / free.sum(-1).clamp(min=1.)[None]
        genuine = eligible.any(-1)
        wrong = ((logits > 0) != (target[None] > .5)) & eligible[None]
        right = ~wrong.any(-1) & genuine[None]
        parts.append((1, losses, weight * factor * genuine, right, genuine, None))
    return parts, outcome


def thinking_loss(frames, batch, plans, config, outcome_weighting=True):
    """`(loss, stats)`; stats = right answers per step `[S + 1]`, then the decision count."""
    device = next((f["logits"].device for f in frames or () if f is not None), None)
    if device is None:
        raise ValueError("No thinking trace: the deliberation block must collect during deferred readouts")
    moved = {}

    def get(key):
        if key not in moved:
            moved[key] = batch[key].to(device, non_blocking=True)
        return moved[key]

    zero = torch.zeros((), device=device)
    deep = improve = confidence = consequence = zero
    weights = decisions = consequence_count = zero
    right_total = None
    for t, (frame, plan) in enumerate(zip(frames, plans, strict=True)):
        if frame is None or not (plan.heads[0] or plan.heads[1]) or frame["logits"].shape[0] < 2:
            continue
        steps = frame["logits"].shape[0] - 1
        parts, outcome = _step_losses(frame, batch, t, plan, outcome_weighting, get)
        for kind, losses, weight, right, genuine, target in parts:
            if steps > 1:
                deep = deep + (losses[1:steps] * weight).sum() / (steps - 1)
            gain = F.relu(losses[1:] - losses[:-1].detach() + config.improve_margin)
            improve = improve + (gain * weight).sum() / steps
            weights = weights + weight.sum()
            known = genuine.to(losses.dtype)
            confidence = confidence + (F.binary_cross_entropy_with_logits(
                frame["confidence"].float(), right[1:].to(losses.dtype), reduction="none") * known).sum() / steps
            decisions = decisions + known.sum()
            counted = right.sum(-1).to(losses.dtype)
            right_total = counted if right_total is None else right_total + counted
            if kind == 0 and config.consequence_weight:
                match = (frame["index"] == target[:, None]) & frame["chosen"] & genuine[:, None]
                held = match.any(-1).to(losses.dtype)
                predicted = (frame["consequence"].float() * match).sum(-1)
                consequence = consequence + (F.binary_cross_entropy_with_logits(
                    predicted, outcome[None].expand_as(predicted), reduction="none") * held).sum() / steps
                consequence_count = consequence_count + held.sum()
    total = (config.deep_weight * deep / weights.clamp(min=1e-6)
             + config.improve_weight * improve / weights.clamp(min=1e-6)
             + config.confidence_weight * confidence / decisions.clamp(min=1.)
             + config.consequence_weight * consequence / consequence_count.clamp(min=1.))
    if right_total is None:
        return total, torch.zeros(2, device=device)
    return total, torch.cat((right_total.detach(), decisions.detach()[None]))
