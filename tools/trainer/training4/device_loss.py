"""Equivalent supervised loss with no device-to-host synchronization per frame.

Head-presence metadata is computed on the CPU before H2D. Entire absent heads
are still omitted: a zero gradient instead of None would change AdamW decay.
Statistics stay detached on the GPU until the caller actually logs them.
"""
import torch
import torch.nn.functional as F


def compute_device_loss(cand, pair, scalar, value, auxiliary, batch, t, plan, loss_config=None):
    if loss_config is not None and loss_config.version == 5:
        if loss_config.normalization != 'legacy_timestep':
            raise ValueError('Unroll normalization requires compute_unroll_device_loss')
        from .loss_v5 import compute_loss_v5
        sliced={key: tensor[:, t:t+1] for key,tensor in batch.items() if torch.is_tensor(tensor) and tensor.ndim>=2}
        return compute_loss_v5([(None,cand,pair,scalar,value)],auxiliary[:,None],sliced,[plan],loss_config)
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
    outcome = at("outcome")
    out_factor = 0.35 + 1.30 * outcome
    if plan.valid:
        n = valid.sum().clamp(min=1)
        value_loss = ((value - outcome).square() * valid).sum() / n
        aux_loss = ((auxiliary - at("aux_target")).square().mean(-1) * valid).sum() / n

    if plan.heads[0]:
        chosen = decision & (head == 0)
        # Non-single rows may have arbitrary targets, including out-of-range
        # indices for this frame. They must never enter gather/cross_entropy.
        target = torch.where(chosen, at("single_idx"), 0)
        choice = at("cand_mask")[:, :cand.shape[1]].sum(-1) > 1
        weights = weight * out_factor * chosen * choice
        ce = F.cross_entropy(cand, target, reduction="none")
        policy = policy + (ce * weights).sum() / weights.sum().clamp(min=1e-6)
        correct = ((cand.argmax(-1) == target) & chosen).sum()
        count = chosen.sum()

    for kind, logits, prefix in (
        (1, cand, "cand"),
        (2, pair, "pair"),
    ):
        if plan.heads[kind]:
            selected = decision & (head == kind)
            width = logits.shape[1]
            mask = at(prefix + "_mask")[:, :width]
            free = at(prefix + "_free_mask")[:, :width] * mask.float()
            target = at(prefix + "_target_mask")[:, :width]
            losses = F.binary_cross_entropy_with_logits(
                logits.masked_fill(~mask, 0.0), target, reduction="none",
            )
            weights = free * (weight * out_factor * selected).unsqueeze(-1)
            policy = policy + (losses * weights).sum() / weights.sum().clamp(min=1.0)

    if plan.heads[3]:
        selected = decision & (head == 3)
        weights = weight * selected
        policy = policy + ((scalar - at("scalar_target")).square() * weights).sum() / weights.sum().clamp(min=1e-6)

    total = policy + 0.5 * value_loss + 0.25 * aux_loss
    stats = torch.stack((policy.detach(), value_loss.detach(), aux_loss.detach(), correct.float(), count.float()))
    return total, stats


def compute_unroll_device_loss(prepared, aux_all, batch, plans, loss_config=None):
    """Vectorized loss across all U unroll steps for deferred-readout execution.

    Computes `value_loss` and `aux_loss` across the full `[B, U]` tensor in a
    single pass and visits only active decision timesteps (`any(plan.heads)`)
    for policy heads, avoiding hundreds of tiny per-timestep kernel launches on
    passive observation frames while preserving exact gradients and statistics.
    """
    if loss_config is not None and loss_config.version == 5:
        from .loss_v5 import compute_loss_v5
        return compute_loss_v5(prepared, aux_all, batch, plans, loss_config)
    device = aux_all.device
    unroll = len(plans)
    zero = torch.zeros((), device=device)
    if not any(plan.valid for plan in plans):
        return zero, torch.zeros(5, device=device)

    valid_all = batch["step_valid"].to(device, non_blocking=True)
    decision_all = batch["is_decision"].to(device, non_blocking=True) & valid_all
    head_all = batch["head"].to(device, non_blocking=True)
    weight_all = batch["loss_weight"].to(device, non_blocking=True)
    outcome_all = batch["outcome"].to(device, non_blocking=True)
    aux_target_all = batch["aux_target"].to(device, non_blocking=True)

    val_pred_all = torch.stack([prepared[t][4].float() for t in range(unroll)], dim=1)
    n_valid = valid_all.sum(dim=0).clamp(min=1)
    val_losses = ((val_pred_all - outcome_all).square() * valid_all).sum(dim=0) / n_valid
    aux_losses = ((aux_all - aux_target_all).square().mean(dim=-1) * valid_all).sum(dim=0) / n_valid

    single_idx_all = batch["single_idx"].to(device, non_blocking=True)
    cand_mask_all = batch["cand_mask"].to(device, non_blocking=True)
    cand_free_all = batch["cand_free_mask"].to(device, non_blocking=True)
    cand_target_all = batch["cand_target_mask"].to(device, non_blocking=True)
    pair_mask_all = batch["pair_mask"].to(device, non_blocking=True)
    pair_free_all = batch["pair_free_mask"].to(device, non_blocking=True)
    pair_target_all = batch["pair_target_mask"].to(device, non_blocking=True)
    scalar_target_all = batch["scalar_target"].to(device, non_blocking=True)

    traj_loss = zero
    pol_stat_sum = zero
    correct_sum = zero
    count_sum = zero

    for t, plan in enumerate(plans):
        policy = zero
        if any(plan.heads):
            _, cand_l, pair_l, scal_p, _ = prepared[t]
            cand = cand_l.float()
            pair = pair_l.float()
            scalar = scal_p.float()
            decision = decision_all[:, t]
            head = head_all[:, t]
            weight = weight_all[:, t]
            out_factor = 0.35 + 1.30 * outcome_all[:, t]

            if plan.heads[0]:
                chosen = decision & (head == 0)
                target = torch.where(chosen, single_idx_all[:, t], 0)
                choice = cand_mask_all[:, t, :cand.shape[1]].sum(-1) > 1
                weights = weight * out_factor * chosen * choice
                ce = F.cross_entropy(cand, target, reduction="none")
                policy = policy + (ce * weights).sum() / weights.sum().clamp(min=1e-6)
                correct_sum = correct_sum + ((cand.argmax(-1) == target) & chosen).sum().float()
                count_sum = count_sum + chosen.sum().float()

            if plan.heads[1]:
                selected = decision & (head == 1)
                width = cand.shape[1]
                mask = cand_mask_all[:, t, :width]
                free = cand_free_all[:, t, :width] * mask.float()
                target = cand_target_all[:, t, :width]
                losses = F.binary_cross_entropy_with_logits(
                    cand.masked_fill(~mask, 0.0), target, reduction="none",
                )
                weights = free * (weight * out_factor * selected).unsqueeze(-1)
                policy = policy + (losses * weights).sum() / weights.sum().clamp(min=1.0)

            if plan.heads[2]:
                selected = decision & (head == 2)
                width = pair.shape[1]
                mask = pair_mask_all[:, t, :width]
                free = pair_free_all[:, t, :width] * mask.float()
                target = pair_target_all[:, t, :width]
                losses = F.binary_cross_entropy_with_logits(
                    pair.masked_fill(~mask, 0.0), target, reduction="none",
                )
                weights = free * (weight * out_factor * selected).unsqueeze(-1)
                policy = policy + (losses * weights).sum() / weights.sum().clamp(min=1.0)

            if plan.heads[3]:
                selected = decision & (head == 3)
                weights = weight * selected
                policy = policy + ((scalar - scalar_target_all[:, t]).square() * weights).sum() / weights.sum().clamp(min=1e-6)

            pol_stat_sum = pol_stat_sum + policy.detach()

        loss_t = policy + 0.5 * val_losses[t] + 0.25 * aux_losses[t]
        traj_loss = traj_loss + loss_t / unroll

    val_stat_sum = val_losses.detach().sum()
    aux_stat_sum = aux_losses.detach().sum()
    stats = torch.stack((pol_stat_sum, val_stat_sum, aux_stat_sum, correct_sum, count_sum))
    return traj_loss, stats
