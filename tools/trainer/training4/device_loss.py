"""Equivalent supervised loss with no device-to-host synchronization per frame.

Head-presence metadata is computed on the CPU before H2D. Entire absent heads
are still omitted: a zero gradient instead of None would change AdamW decay.
Statistics stay detached on the GPU until the caller actually logs them.
"""
import torch
import torch.nn.functional as F


def compute_device_loss(cand, pair, scalar, value, auxiliary, batch, t, plan):
    device = cand.device
    at = lambda key: batch[key][:, t].to(device, non_blocking=True)
    valid = at("step_valid")
    decision = at("is_decision") & valid
    head = at("head")
    weight = at("loss_weight")
    zero = torch.zeros((), device=device)
    policy = zero
    value_loss = zero
    aux_loss = zero
    correct = zero
    count = zero
    if plan.valid:
        n = valid.sum().clamp(min=1)
        value_loss = ((value - at("outcome")).square() * valid).sum() / n
        aux_loss = ((auxiliary - at("aux_target")).square().mean(-1) * valid).sum() / n

    if plan.heads[0]:
        chosen = decision & (head == 0)
        # Non-single rows may have arbitrary targets, including out-of-range
        # indices for this frame. They must never enter gather/cross_entropy.
        target = torch.where(chosen, at("single_idx"), 0)
        is_pass = at("cand_feat")[:, :, 0].gather(1, target[:, None])[:, 0] > 0.5
        weights = torch.where(is_pass, 1.0, 3.0) * weight * chosen
        ce = F.cross_entropy(cand, target, reduction="none")
        policy = policy + (ce * weights).sum() / weights.sum().clamp(min=1e-6)
        correct = ((cand.argmax(-1) == target) & chosen).sum()
        count = chosen.sum()

    for kind, logits, prefix in ((1, cand, "cand"), (2, pair, "pair")):
        if plan.heads[kind]:
            selected = decision & (head == kind)
            width = logits.shape[1]
            mask = at(prefix + "_mask")[:, :width]
            free = at(prefix + "_free_mask")[:, :width] * mask.float()
            target = at(prefix + "_target_mask")[:, :width]
            losses = F.binary_cross_entropy_with_logits(
                logits.masked_fill(~mask, 0.0), target,
                pos_weight=torch.tensor(2.0, device=device), reduction="none",
            )
            weights = free * (weight * selected).unsqueeze(-1)
            policy = policy + 2.5 * (losses * weights).sum() / weights.sum().clamp(min=1.0)

    if plan.heads[3]:
        selected = decision & (head == 3)
        weights = weight * selected
        policy = policy + ((scalar - at("scalar_target")).square() * weights).sum() / weights.sum().clamp(min=1e-6)

    total = policy + 0.5 * value_loss + 0.25 * aux_loss
    stats = torch.stack((policy.detach(), value_loss.detach(), aux_loss.detach(), correct.float(), count.float()))
    return total, stats
