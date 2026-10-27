"""Reference supervised loss; shared with PPO anchor training."""
import torch
import torch.nn.functional as F
import model4 as M


def compute_step_loss(
    cand_logits: torch.Tensor,
    pair_logits: torch.Tensor,
    scalar_pred: torch.Tensor,
    value_pred: torch.Tensor,
    aux_pred: torch.Tensor,
    step_batch: dict[str, torch.Tensor],
    t: int,
    smooth_value: bool = True,
) -> tuple[torch.Tensor, dict[str, float]]:
    device = cand_logits.device
    valid = step_batch["step_valid"][:, t].to(device)
    is_dec = step_batch["is_decision"][:, t].to(device) & valid
    loss_w = step_batch["loss_weight"][:, t].to(device)
    head = step_batch["head"][:, t].to(device)
    outcome = step_batch["outcome"][:, t].to(device)
    aux_tgt = step_batch["aux_target"][:, t].to(device)

    # Value and Auxiliary self-supervised loss on all valid steps
    if valid.any():
        # Terminal side utility is a proper probability target in duels, teams
        # and FFA. Shrinking every early position toward 0.5 biases 3–8 seat FFA.
        val_loss = F.mse_loss(value_pred[valid], outcome[valid])
        aux_loss = F.mse_loss(aux_pred[valid], aux_tgt[valid])
    else:
        val_loss = torch.tensor(0.0, device=device)
        aux_loss = torch.tensor(0.0, device=device)

    pol_loss = torch.tensor(0.0, device=device)
    correct_single = 0
    total_single = 0

    # 0: SingleChoice (weighted by step loss_weight, and outcome weight)
    m0 = is_dec & (head == 0)
    if m0.any():
        logits0 = cand_logits[m0]
        tgt0 = step_batch["single_idx"][:, t].to(device)[m0]
        out0 = outcome[m0]
        w0 = loss_w[m0] * (0.35 + 1.30 * out0)
        ce_per_sample = F.cross_entropy(logits0, tgt0, reduction="none")
        choice = step_batch["cand_mask"][:, t, :cand_logits.shape[1]].to(device)[m0].sum(-1) > 1
        sample_w = w0 * choice.to(w0.dtype)
        pol_loss = pol_loss + (ce_per_sample * sample_w).sum() / sample_w.sum().clamp(min=1e-6)
        correct_single = int((logits0.argmax(dim=-1) == tgt0).sum().item())
        total_single = int(m0.sum().item())

    # 1: SubsetPick (ChooseAttackers, ChooseTargets, ChooseCards) on free choices (P0-3)
    m1 = is_dec & (head == 1)
    if m1.any():
        logits1 = cand_logits[m1]
        cmask1 = step_batch["cand_mask"][:, t, :cand_logits.shape[1]].to(device)[m1]
        fmask1 = step_batch["cand_free_mask"][:, t, :cand_logits.shape[1]].to(device)[m1] * cmask1.float()
        tgt1 = step_batch["cand_target_mask"][:, t, :cand_logits.shape[1]].to(device)[m1]
        w1 = (loss_w[m1] * (0.35 + 1.30 * outcome[m1])).unsqueeze(-1)
        bce1 = F.binary_cross_entropy_with_logits(
            logits1.masked_fill(~cmask1, 0.0), tgt1, reduction="none"
        )
        eff_mask1 = fmask1 * w1
        denom1 = eff_mask1.sum().clamp(min=1.0)
        pol_loss = pol_loss + (bce1 * eff_mask1).sum() / denom1

    # 2: PairPick (ChooseBlockers) on free choices (P0-3)
    m2 = is_dec & (head == 2)
    if m2.any():
        logits2 = pair_logits[m2]
        pmask2 = step_batch["pair_mask"][:, t, :pair_logits.shape[1]].to(device)[m2]
        fmask2 = step_batch["pair_free_mask"][:, t, :pair_logits.shape[1]].to(device)[m2] * pmask2.float()
        tgt2 = step_batch["pair_target_mask"][:, t, :pair_logits.shape[1]].to(device)[m2]
        w2 = (loss_w[m2] * (0.35 + 1.30 * outcome[m2])).unsqueeze(-1)
        bce2 = F.binary_cross_entropy_with_logits(
            logits2.masked_fill(~pmask2, 0.0), tgt2, reduction="none"
        )
        eff_mask2 = fmask2 * w2
        denom2 = eff_mask2.sum().clamp(min=1.0)
        pol_loss = pol_loss + (bce2 * eff_mask2).sum() / denom2

    # 3: ScalarValue
    m3 = is_dec & (head == 3)
    if m3.any():
        pred3 = scalar_pred[m3]
        tgt3 = step_batch["scalar_target"][:, t].to(device)[m3]
        w3 = loss_w[m3]
        pol_loss = pol_loss + ((pred3 - tgt3).pow(2) * w3).sum() / w3.sum().clamp(min=1e-6)

    total_loss = pol_loss + 0.5 * val_loss + 0.25 * aux_loss
    stats = {
        "pol_loss": float(pol_loss.detach().item()),
        "val_loss": float(val_loss.detach().item()),
        "aux_loss": float(aux_loss.detach().item()),
        "correct_single": correct_single,
        "total_single": total_single,
    }
    return total_loss, stats
