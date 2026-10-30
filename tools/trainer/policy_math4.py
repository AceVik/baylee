"""Executed-action probabilities and anchor KL for recurrent PPO."""

import torch
import torch.nn.functional as F


def _bernoulli_logprob_and_kl(
    logits: torch.Tensor,
    anchor_logits: torch.Tensor,
    target: torch.Tensor,
    free_mask: torch.Tensor,
    scale: torch.Tensor,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Conditional free-entry likelihood and KL(anchor || policy).

    Mask before arithmetic so forced/padded entries contribute neither values
    nor gradients, even when their unused logits are nonfinite. Log-sigmoid
    retains rare-action likelihoods and gradients after sigmoid saturates.
    """
    free = free_mask != 0
    z = logits.masked_fill(~free, 0.0) * scale
    az = anchor_logits.masked_fill(~free, 0.0) * scale
    chosen, absent = F.logsigmoid(z), F.logsigmoid(-z)
    anchor_chosen, anchor_absent = F.logsigmoid(az), F.logsigmoid(-az)
    logprob = torch.where(target.bool(), chosen, absent).masked_fill(~free, 0.0)
    kl = (
        anchor_chosen.exp() * (anchor_chosen - chosen)
        + anchor_absent.exp() * (anchor_absent - absent)
    ).masked_fill(~free, 0.0)
    return logprob.sum(dim=-1), kl.sum(dim=-1).clamp(min=0.0)


def action_logprob_and_kl(
    cand_logits: torch.Tensor,
    pair_logits: torch.Tensor,
    scalar_pred: torch.Tensor,
    anc_cand_logits: torch.Tensor,
    anc_pair_logits: torch.Tensor,
    anc_scalar_pred: torch.Tensor,
    batch: dict[str, torch.Tensor],
    t: int,
) -> tuple[torch.Tensor, torch.Tensor]:
    device = cand_logits.device
    B = cand_logits.shape[0]
    head = batch["head"][:, t].to(device)
    single_idx = batch["single_idx"][:, t].to(device)
    cmask = batch["cand_mask"][:, t, :cand_logits.shape[1]].to(device)
    pmask = batch["pair_mask"][:, t, :pair_logits.shape[1]].to(device)
    ctgt = batch["cand_target_mask"][:, t, :cand_logits.shape[1]].to(device)
    ptgt = batch["pair_target_mask"][:, t, :pair_logits.shape[1]].to(device)
    cfmask = batch["cand_free_mask"][:, t, :cand_logits.shape[1]].to(device) * cmask.float()
    pfmask = batch["pair_free_mask"][:, t, :pair_logits.shape[1]].to(device) * pmask.float()
    temp = batch["temperature"][:, t].to(device)
    scale = torch.where(temp <= 1e-4, torch.ones_like(temp), 1.0 / temp).unsqueeze(-1)

    logp = torch.zeros(B, device=device, dtype=torch.float32)
    kl = torch.zeros(B, device=device, dtype=torch.float32)

    # 0: SingleChoice (temperature-scaled categorical log-softmax)
    m0 = head == 0
    if m0.any():
        sc0 = scale[m0]
        cm0 = cmask[m0]
        l0 = (cand_logits[m0] * sc0).masked_fill(~cm0, -1e4)
        al0 = (anc_cand_logits[m0] * sc0).masked_fill(~cm0, -1e4)
        lp0 = F.log_softmax(l0, dim=-1)
        anc_lp0 = F.log_softmax(al0, dim=-1)
        anc_p0 = anc_lp0.exp()
        chosen_lp = lp0.gather(-1, single_idx[m0].unsqueeze(-1)).squeeze(-1)
        logp[m0] = chosen_lp
        kl0 = (anc_p0 * (anc_lp0 - lp0)).masked_fill(~cm0, 0.0).sum(dim=-1)
        kl[m0] = kl0.clamp(min=0.0)

    # 1: SubsetPick (temperature-scaled conditional Bernoulli over free_mask == 1)
    m1 = head == 1
    if m1.any():
        logp[m1], kl[m1] = _bernoulli_logprob_and_kl(
            cand_logits[m1], anc_cand_logits[m1], ctgt[m1], cfmask[m1], scale[m1]
        )

    # 2: PairPick (temperature-scaled conditional Bernoulli over pair_free_mask == 1)
    m2 = head == 2
    if m2.any():
        logp[m2], kl[m2] = _bernoulli_logprob_and_kl(
            pair_logits[m2], anc_pair_logits[m2], ptgt[m2], pfmask[m2], scale[m2]
        )

    # ScalarValue is deterministic regression, not a stochastic behavior policy.
    # It remains eligible for supervised replay, never the PPO ratio.
    return logp, kl
