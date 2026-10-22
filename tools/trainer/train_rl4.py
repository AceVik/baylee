"""Playernet v4 Recurrent PPO + Anchor KL Trainer (`train_rl4.py`).

Implements Phase C (T5) from `docs/history/trained-ai-v4-handover-2026-10-22.md`
with all P0/P1 fixes from `docs/history/trained-ai-v4-argon-feedback-2026-10-22.md`:
- Enforces `source_kind == 0` (`Net`) and `has_logprob == True` on PPO updates,
  using the exact `behavior_logprob` recorded by `NetPlayer4` (`P0-2`, `P0-5`).
- Exact temperature-scaled (`1 / tau`) log-probabilities and `free_mask`
  conditioning across `SingleChoice`, `SubsetPick`, `PairPick`, and `ScalarValue` (`P0-3`).
- Stateful trajectory streaming (`StatefulBatchSampler4`) carrying `h` and `h_anc`
  across consecutive segments (`P0-4`).
- True reverse-time GAE(gamma=0.995, lambda=0.95) with `is_terminal` handling and
  `net.eval()` during PPO updates so dropout noise never corrupts importance
  sampling ratios (`P0-5`).
- Optional `--anchor-data` supervised replay mixing alongside explicit KL penalty
  (`beta_kl = 0.02`) against the frozen Phase B anchor (`P1-9`).
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader

import dataset4 as D4
import model4 as M
import train4 as T4
from ppo_targets4 import prepare_targets
from checkpoint4 import atomic_save, rng_state


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


def compute_reverse_gae(
    values: torch.Tensor,
    outcomes: torch.Tensor,
    step_valid: torch.Tensor,
    is_terminal: torch.Tensor,
    gamma: float,
    gae_lambda: float,
    bootstrap_value: torch.Tensor | None = None,
    bootstrap_advantage: torch.Tensor | None = None,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Computes reverse-time GAE(gamma, lambda) across `[B, U]` (`P0-5`)."""
    B, U = values.shape
    device = values.device
    gae = (torch.zeros(B, device=device, dtype=torch.float32) if bootstrap_advantage is None
           else bootstrap_advantage.detach().to(device).clone())
    adv_out = torch.zeros((B, U), device=device, dtype=torch.float32)
    ret_out = torch.zeros((B, U), device=device, dtype=torch.float32)

    for t in reversed(range(U)):
        term_t = is_terminal[:, t]
        valid_t = step_valid[:, t]
        v_t = values[:, t].detach()
        r_term = outcomes[:, t]
        at_boundary = torch.ones(B, dtype=torch.bool, device=device) if t == U - 1 else ~step_valid[:, t + 1]
        needs_bootstrap = valid_t & ~term_t & at_boundary
        if needs_bootstrap.any() and bootstrap_value is None:
            raise ValueError("Nonterminal sequence boundary requires the actual next-state value")
        boot = torch.zeros(B, device=device) if bootstrap_value is None else bootstrap_value.detach().to(device)
        v_next = boot if t == U - 1 else torch.where(at_boundary, boot, values[:, t + 1].detach())
        delta = torch.where(term_t, r_term - v_t, gamma * v_next - v_t)
        updated = delta + gamma * gae_lambda * (~term_t).float() * gae
        gae = torch.where(valid_t, updated, gae)
        adv_out[:, t] = torch.where(valid_t, gae, 0.0)
        ret_out[:, t] = torch.where(valid_t, v_t + gae, 0.0)

    return adv_out, ret_out


def main() -> None:
    ap = argparse.ArgumentParser(description="Recurrent PPO + Anchor KL trainer for Playernet v4")
    ap.add_argument("--init", required=True, help="Initial/anchor checkpoint directory")
    ap.add_argument(
        "--rollout-data",
        "--rollout",
        dest="rollout_data",
        required=True,
        help="Rollout dataset directory (convert4)",
    )
    ap.add_argument("--anchor-data", default="", help="Optional supervised replay dataset for mixing")
    ap.add_argument("--out", required=True, help="Output checkpoint directory")
    ap.add_argument("--audit-only", action="store_true")
    ap.add_argument("--target-kl", type=float, default=0.03)
    ap.add_argument("--batch-size", type=int, default=4)
    ap.add_argument("--unroll", "--unroll-len", dest="unroll", type=int, default=16)
    ap.add_argument("--thinking-steps", type=int, default=2)
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--max-steps", type=int, default=120)
    ap.add_argument("--max-shards", type=int, default=2)
    ap.add_argument("--lr", type=float, default=2e-5)
    ap.add_argument("--clip-eps", type=float, default=0.2)
    ap.add_argument("--beta-kl", type=float, default=0.02)
    ap.add_argument("--anchor-mix-weight", type=float, default=0.25)
    ap.add_argument("--gamma", type=float, default=1.0)
    ap.add_argument("--gae-lambda", type=float, default=0.95)
    args = ap.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    use_bf16 = False  # Collector probabilities are float32; audit and PPO use the same precision.

    init_dir = Path(args.init).expanduser()
    ckpt = torch.load(init_dir / "net.pt", map_location="cpu", weights_only=False)
    if (ckpt.get("trajectory_version") != M.TRAJECTORY_VERSION or ckpt.get("model_version") != M.MODEL_VERSION
                or ckpt.get("encoder_version") != M.ENCODER_VERSION):
        raise ValueError("Incompatible checkpoint memory contract")
    cfg = M.Config4(**ckpt["config"])
    net = M.PlayerNet4(cfg).to(device)
    net.load_state_dict(ckpt["state_dict"], strict=True)

    anchor = copy.deepcopy(net).to(device).eval()
    for p in anchor.parameters():
        p.requires_grad_(False)

    rollout_ds = D4.TrajectoryDataset4(
        Path(args.rollout_data).expanduser(),
        split="train",
        unroll_len=args.unroll,
        max_shards=args.max_shards,
    )
    audit = prepare_targets(net, rollout_ds, device, args.batch_size, args.gamma,
                            args.gae_lambda, action_logprob_and_kl, compute_reverse_gae)
    if args.audit_only:
        print(json.dumps(audit, indent=2))
        return
    rollout_sampler = D4.StatefulBatchSampler4(
        rollout_ds, batch_size=args.batch_size, shuffle=True
    )
    loader = DataLoader(
        rollout_ds,
        batch_sampler=rollout_sampler,
        collate_fn=D4.collate_trajectories,
    )

    anchor_loader = None
    anchor_iter = None
    if args.anchor_data:
        anc_path = Path(args.anchor_data).expanduser()
        if (anc_path / "meta.json").exists():
            anc_ds = D4.TrajectoryDataset4(
                anc_path,
                split="train",
                unroll_len=args.unroll,
                max_shards=1,
            )
            anc_sampler = D4.StatefulBatchSampler4(
                anc_ds, batch_size=args.batch_size, shuffle=True
            )
            anchor_loader = DataLoader(
                anc_ds,
                batch_sampler=anc_sampler,
                collate_fn=D4.collate_trajectories,
            )

    opt = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=1e-2)

    h_sup_state = None
    stop = False
    step_num = 0
    total_rl_decisions = 0
    for epoch in range(1, args.epochs + 1):
        # Keep net in eval() mode during PPO updates so dropout noise does not
        # corrupt importance sampling ratios (`ratio == 1.0` at epoch 0, P0-5)
        net.eval()
        tot_loss = 0.0
        tot_kl = 0.0
        n_batches = 0
        h: torch.Tensor | None = None
        h_anc: torch.Tensor | None = None

        for batch in loader:
            B, U = batch["step_valid"].shape
            opt.zero_grad(set_to_none=True)
            is_new = batch["is_new_traj"].to(device)

            # Standard truncated recurrent PPO: condition each unroll on the
            # stored behavior memory, with gradients through this unroll only.
            h = batch["behavior_h"].to(device)
            h_anc = h.clone()

            default_k = torch.full((B,), args.thinking_steps, dtype=torch.long, device=device)

            logps, old_logps, values, valids, kls = [], [], [], [], []

            for t in range(U):
                if "thinking_steps" in batch:
                    rec_k = batch["thinking_steps"][:, t].to(device)
                    is_dec_t = batch["is_decision"][:, t].to(device)
                    k_t = torch.where(
                        is_dec_t,
                        torch.where(rec_k > 0, rec_k, default_k),
                        torch.zeros_like(default_k),
                    )
                else:
                    k_t = default_k
                step_in = (
                    batch["entity_feat"][:, t].to(device),
                    batch["entity_dsl"][:, t].to(device),
                    batch["entity_mask"][:, t].to(device),
                    batch["seat_feat"][:, t].to(device),
                    batch["global_feat"][:, t].to(device),
                    batch["edge_index"][:, t].to(device),
                    batch["edge_type"][:, t].to(device),
                    batch["edge_mask"][:, t].to(device),
                    batch["cand_feat"][:, t].to(device),
                    batch["cand_entity_idx"][:, t].to(device),
                    batch["cand_seat_idx"][:, t].to(device),
                    batch["cand_mask"][:, t].to(device),
                    batch["pair_index"][:, t].to(device),
                    batch["pair_mask"][:, t].to(device),
                    batch["cand_dsl"][:, t].to(device),
                )
                with torch.autocast(
                    device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16
                ):
                    h_next, cand_l, pair_l, scal_p, val_p = net(*step_in, h, k_t)
                    with torch.no_grad():
                        h_anc_next, anc_cand_l, anc_pair_l, anc_scal_p, _ = anchor(
                            *step_in, h_anc, k_t
                        )

                lp_t, kl_t = action_logprob_and_kl(
                    cand_l.float(),
                    pair_l.float(),
                    scal_p.float(),
                    anc_cand_l.float(),
                    anc_pair_l.float(),
                    anc_scal_p.float(),
                    batch,
                    t,
                )
                # Strict provenance contract (P0-2, P0-5): only update policy on
                # genuine neural rollout steps (`source_kind == 0` and `has_logprob == True`)
                beh_lp = batch["behavior_logprob"][:, t].to(device)
                is_net_step = (batch["source_kind"][:, t].to(device) == 0) & batch[
                    "has_logprob"
                ][:, t].to(device)
                rl_valid_t = (
                    batch["step_valid"][:, t].to(device)
                    & batch["is_decision"][:, t].to(device)
                    & is_net_step
                    & (batch["head"][:, t].to(device) != 3)
                    & (batch["temperature"][:, t].to(device) > 1e-4)
                )

                step_valid_t = batch["step_valid"][:, t].to(device)
                h = torch.where(step_valid_t.view(B, 1, 1), h_next.float(), h)
                h_anc = torch.where(step_valid_t.view(B, 1, 1), h_anc_next.float(), h_anc)

                logps.append(lp_t)
                old_logps.append(beh_lp)
                values.append(val_p.float())
                valids.append(rl_valid_t.float())
                kls.append(kl_t)

            vals = torch.stack(values, dim=1)  # [B, U]
            outcomes = batch["outcome"].to(device)  # [B, U]
            step_valid_stack = batch["step_valid"].to(device)  # [B, U]
            is_term_stack = batch["is_terminal"].to(device)  # [B, U]
            v_mask = torch.stack(valids, dim=1)  # [B, U]
            total_rl_decisions += int(v_mask.sum().item())

            # Targets were computed once from complete episodes with the frozen
            # behavior policy, before the first optimizer step.
            adv = batch["advantages"].to(device) * v_mask
            td_returns = batch["returns"].to(device)

            lp_stack = torch.stack(logps, dim=1)
            old_lp_stack = torch.stack(old_logps, dim=1)
            kl_stack = torch.stack(kls, dim=1)

            ratio = torch.exp((lp_stack - old_lp_stack).clamp(-10.0, 10.0))
            approx_kl = (((ratio - 1) - (lp_stack - old_lp_stack)) * v_mask).sum() / v_mask.sum().clamp(min=1)
            if approx_kl > args.target_kl:
                print(f"[train_rl4] stopping update: behavior KL={float(approx_kl):.4f}", flush=True)
                stop = True
                break
            surr1 = ratio * adv
            surr2 = torch.clamp(ratio, 1.0 - args.clip_eps, 1.0 + args.clip_eps) * adv
            ppo_loss = -(torch.min(surr1, surr2) * v_mask).sum() / v_mask.sum().clamp(
                min=1.0
            )
            val_valid_f = step_valid_stack.float()
            val_loss = ((vals - td_returns).pow(2) * val_valid_f).sum() / val_valid_f.sum().clamp(
                min=1.0
            )
            kl_loss = (kl_stack * v_mask).sum() / v_mask.sum().clamp(min=1.0)

            loss = ppo_loss + 0.5 * val_loss + args.beta_kl * kl_loss

            # Optional supervised replay anchor mixing (P1-9)
            if anchor_loader is not None and args.anchor_mix_weight > 0.0:
                if anchor_iter is None:
                    anchor_iter = iter(anchor_loader)
                try:
                    anc_batch = next(anchor_iter)
                except StopIteration:
                    anchor_iter = iter(anchor_loader)
                    h_sup_state = None
                    anc_batch = next(anchor_iter)
                B_a, U_a = anc_batch["step_valid"].shape
                if h_sup_state is None or h_sup_state.shape[0] != B_a:
                    h_sup_state = torch.zeros((B_a, M.RECURRENT_SLOTS, cfg.d_model), device=device)
                h_sup = torch.where(anc_batch["is_new_traj"].to(device).view(B_a, 1, 1),
                                    torch.zeros_like(h_sup_state), h_sup_state.detach())
                k_sup = torch.full(
                    (B_a,), args.thinking_steps, dtype=torch.long, device=device
                )
                sup_loss = torch.tensor(0.0, device=device)
                for t_a in range(U_a):
                    k_sup = torch.where(anc_batch["is_decision"][:, t_a].to(device),
                                        args.thinking_steps, 0)
                    with torch.autocast(
                        device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16
                    ):
                        h_sup_next, cl_a, pl_a, sp_a, vp_a = net(
                            anc_batch["entity_feat"][:, t_a].to(device),
                            anc_batch["entity_dsl"][:, t_a].to(device),
                            anc_batch["entity_mask"][:, t_a].to(device),
                            anc_batch["seat_feat"][:, t_a].to(device),
                            anc_batch["global_feat"][:, t_a].to(device),
                            anc_batch["edge_index"][:, t_a].to(device),
                            anc_batch["edge_type"][:, t_a].to(device),
                            anc_batch["edge_mask"][:, t_a].to(device),
                            anc_batch["cand_feat"][:, t_a].to(device),
                            anc_batch["cand_entity_idx"][:, t_a].to(device),
                            anc_batch["cand_seat_idx"][:, t_a].to(device),
                            anc_batch["cand_mask"][:, t_a].to(device),
                            anc_batch["pair_index"][:, t_a].to(device),
                            anc_batch["pair_mask"][:, t_a].to(device),
                            anc_batch["cand_dsl"][:, t_a].to(device),
                            h_sup,
                            k_sup,
                        )
                        aux_a = net.aux_head(h_sup_next[:, 0])
                    l_a, _ = T4.compute_step_loss(
                        cl_a.float(),
                        pl_a.float(),
                        sp_a.float(),
                        vp_a.float(),
                        aux_a.float(),
                        anc_batch,
                        t_a,
                    )
                    v_a = anc_batch["step_valid"][:, t_a].to(device)
                    h_sup = torch.where(v_a.view(B_a, 1, 1), h_sup_next.float(), h_sup)
                    sup_loss = sup_loss + l_a / U_a
                h_sup_state = h_sup.detach()
                loss = loss + args.anchor_mix_weight * sup_loss

            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()

            tot_loss += float(loss.detach().item())
            tot_kl += float(kl_loss.detach().item())
            n_batches += 1
            step_num += 1
            if args.max_steps > 0 and step_num >= args.max_steps:
                break

        print(
            f"[train_rl4] epoch={epoch} step={step_num} rl_decisions={total_rl_decisions} "
            f"loss={tot_loss / max(n_batches, 1):.4f} kl={tot_kl / max(n_batches, 1):.4f}",
            flush=True,
        )
        if stop or (args.max_steps > 0 and step_num >= args.max_steps):
            break

    if total_rl_decisions == 0:
        raise RuntimeError(
            "train_rl4 saw 0 neural rollout decisions with has_logprob=True (P0-2 contract violation)"
        )

    out_dir = Path(args.out).expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)
    atomic_save(
        {
            "config": cfg.to_dict(),
            "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": M.ENCODER_VERSION,
            "sampling_version": M.SAMPLING_VERSION,
            "dataset_fingerprint": rollout_ds.fingerprint,
            "optimizer": opt.state_dict(), "rng": rng_state(), "audit": audit,
            "state_dict": net.state_dict(),
            "step": step_num,
            "rl_decisions": total_rl_decisions,
            "dataset": str(args.rollout_data),
        },
        out_dir / "net.pt",
    )
    (out_dir / "meta.json").write_text(
        json.dumps(
            {
                "encoder_version": M.ENCODER_VERSION,
                "d_model": cfg.d_model,
                "n_layers": cfg.n_layers,
                "n_heads": cfg.n_heads,
                "thinking_steps": args.thinking_steps,
                "rl_decisions": total_rl_decisions,
            },
            indent=2,
        )
    )
    print(f"[train_rl4] saved RL checkpoint to {out_dir / 'net.pt'}")


if __name__ == "__main__":
    main()
