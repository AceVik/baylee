"""Executed-action probabilities and anchor KL for recurrent PPO."""

import torch
import torch.nn.functional as F


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
        sc1 = scale[m1]
        fm1 = cfmask[m1]
        t1 = ctgt[m1]
        l1 = (cand_logits[m1] * sc1).masked_fill(~cmask[m1], 0.0)
        al1 = (anc_cand_logits[m1] * sc1).masked_fill(~cmask[m1], 0.0)
        p1 = torch.sigmoid(l1).clamp(1e-7, 1.0 - 1e-7)
        ap1 = torch.sigmoid(al1).clamp(1e-7, 1.0 - 1e-7)
        lp1 = t1 * torch.log(p1) + (1.0 - t1) * torch.log(1.0 - p1)
        logp[m1] = (lp1 * fm1).sum(dim=-1)
        kl1 = (
            ap1 * (torch.log(ap1) - torch.log(p1))
            + (1.0 - ap1) * (torch.log(1.0 - ap1) - torch.log(1.0 - p1))
        )
        kl[m1] = (kl1 * fm1).sum(dim=-1).clamp(min=0.0)

    # 2: PairPick (temperature-scaled conditional Bernoulli over pair_free_mask == 1)
    m2 = head == 2
    if m2.any():
        sc2 = scale[m2]
        fm2 = pfmask[m2]
        t2 = ptgt[m2]
        l2 = (pair_logits[m2] * sc2).masked_fill(~pmask[m2], 0.0)
        al2 = (anc_pair_logits[m2] * sc2).masked_fill(~pmask[m2], 0.0)
        p2 = torch.sigmoid(l2).clamp(1e-7, 1.0 - 1e-7)
        ap2 = torch.sigmoid(al2).clamp(1e-7, 1.0 - 1e-7)
        lp2 = t2 * torch.log(p2) + (1.0 - t2) * torch.log(1.0 - p2)
        logp[m2] = (lp2 * fm2).sum(dim=-1)
        kl2 = (
            ap2 * (torch.log(ap2) - torch.log(p2))
            + (1.0 - ap2) * (torch.log(1.0 - ap2) - torch.log(1.0 - p2))
        )
        kl[m2] = (kl2 * fm2).sum(dim=-1).clamp(min=0.0)

    # ScalarValue is deterministic regression, not a stochastic behavior policy.
    # It remains eligible for supervised replay, never the PPO ratio.
    return logp, kl
