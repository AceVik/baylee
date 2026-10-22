"""Playernet v4 Supervised Trajectory Distillation Trainer (`train4.py`).

Trains `PlayerNet4` over unrolled `(game_id, seat)` trajectory segments `[B, U]`
using bf16 mixed precision on CUDA:
- Streams consecutive segments per batch lane via `StatefulBatchSampler4` and
  carries `h = h.detach()` across segment boundaries, resetting `h[b]` only when
  `is_new_traj[b]` is True (`P0-4`).
- Computes multi-head policy loss weighted by `loss_weight` (`SingleChoice`
  cross-entropy, `SubsetPick` free-mask binary cross-entropy, `PairPick`
  free-mask binary cross-entropy, `ScalarValue` MSE) + multiplayer side-utility
  value MSE + self-supervised `AuxTargets4` MSE (`P0-3`, `P0-4`, `P1-9`).
- Saves `net.pt` and `meta.json` in `--out`.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader

import dataset4 as D4
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
        if smooth_value:
            # Early turns have high outcome variance around 0.5; shrink early-turn
            # Monte-Carlo targets toward 0.5 so value_head learns board evaluation
            # rather than memorizing deck/opening-hand IDs.
            turn_norm = step_batch["global_feat"][:, t, 0].to(device)[valid]
            conf = (turn_norm * 2.5).clamp(min=0.25, max=1.0)
            val_tgt = 0.5 + (outcome[valid] - 0.5) * conf
            val_w = loss_w[valid].clamp(min=0.15)
            val_loss = ((value_pred[valid] - val_tgt).pow(2) * val_w).sum() / val_w.sum().clamp(min=1e-6)
        else:
            val_loss = F.mse_loss(value_pred[valid], outcome[valid])
        aux_loss = F.mse_loss(aux_pred[valid], aux_tgt[valid])
    else:
        val_loss = torch.tensor(0.0, device=device)
        aux_loss = torch.tensor(0.0, device=device)

    pol_loss = torch.tensor(0.0, device=device)
    correct_single = 0
    total_single = 0

    # 0: SingleChoice (weighted by step loss_weight and active-play boost)
    m0 = is_dec & (head == 0)
    if m0.any():
        logits0 = cand_logits[m0]
        tgt0 = step_batch["single_idx"][:, t].to(device)[m0]
        w0 = loss_w[m0]
        ce_per_sample = F.cross_entropy(logits0, tgt0, reduction="none")
        sample_w = torch.where(tgt0 > 0, 3.0, 1.0) * w0
        pol_loss = pol_loss + (ce_per_sample * sample_w).sum() / sample_w.sum().clamp(min=1e-6)
        correct_single = int((logits0.argmax(dim=-1) == tgt0).sum().item())
        total_single = int(m0.sum().item())

    # 1: SubsetPick (ChooseAttackers, ChooseTargets, ChooseCards) on free choices (P0-3)
    m1 = is_dec & (head == 1)
    if m1.any():
        logits1 = cand_logits[m1]
        cmask1 = step_batch["cand_mask"][:, t].to(device)[m1]
        fmask1 = step_batch["cand_free_mask"][:, t].to(device)[m1] * cmask1.float()
        tgt1 = step_batch["cand_target_mask"][:, t].to(device)[m1]
        w1 = loss_w[m1].unsqueeze(-1)
        pos_w1 = torch.tensor(2.0, device=device)
        bce1 = F.binary_cross_entropy_with_logits(
            logits1.masked_fill(~cmask1, 0.0), tgt1, pos_weight=pos_w1, reduction="none"
        )
        eff_mask1 = fmask1 * w1
        denom1 = eff_mask1.sum().clamp(min=1.0)
        pol_loss = pol_loss + 2.5 * (bce1 * eff_mask1).sum() / denom1

    # 2: PairPick (ChooseBlockers) on free choices (P0-3)
    m2 = is_dec & (head == 2)
    if m2.any():
        logits2 = pair_logits[m2]
        pmask2 = step_batch["pair_mask"][:, t].to(device)[m2]
        fmask2 = step_batch["pair_free_mask"][:, t].to(device)[m2] * pmask2.float()
        tgt2 = step_batch["pair_target_mask"][:, t].to(device)[m2]
        w2 = loss_w[m2].unsqueeze(-1)
        pos_w2 = torch.tensor(2.0, device=device)
        bce2 = F.binary_cross_entropy_with_logits(
            logits2.masked_fill(~pmask2, 0.0), tgt2, pos_weight=pos_w2, reduction="none"
        )
        eff_mask2 = fmask2 * w2
        denom2 = eff_mask2.sum().clamp(min=1.0)
        pol_loss = pol_loss + 2.5 * (bce2 * eff_mask2).sum() / denom2

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


@torch.no_grad()
def evaluate(
    net: M.PlayerNet4,
    loader: DataLoader,
    device: torch.device,
    thinking_steps: int,
) -> dict[str, float]:
    net.eval()
    tot_pol = 0.0
    tot_val = 0.0
    tot_aux = 0.0
    tot_steps = 0
    corr = 0
    tot_single = 0
    use_bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()
    h: torch.Tensor | None = None

    for batch in loader:
        B, U = batch["step_valid"].shape
        is_new = batch["is_new_traj"].to(device)
        if h is None or h.shape[0] != B:
            h = torch.zeros(
                (B, M.RECURRENT_SLOTS, net.cfg.d_model), dtype=torch.float32, device=device
            )
        else:
            h = torch.where(is_new.view(B, 1, 1), torch.zeros_like(h), h)

        k_tensor = torch.full((B,), thinking_steps, dtype=torch.long, device=device)
        for t in range(U):
            with torch.autocast(
                device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16
            ):
                h_next, cand_l, pair_l, scal_p, val_p = net(
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
                    h,
                    k_tensor,
                )
                aux_p = net.aux_head(h_next[:, 0])
            _, st = compute_step_loss(
                cand_l.float(),
                pair_l.float(),
                scal_p.float(),
                val_p.float(),
                aux_p.float(),
                batch,
                t,
                smooth_value=False,
            )
            valid_t = batch["step_valid"][:, t].to(device)
            h = torch.where(valid_t.view(B, 1, 1), h_next.float(), h)
            tot_pol += st["pol_loss"]
            tot_val += st["val_loss"]
            tot_aux += st["aux_loss"]
            tot_steps += 1
            corr += st["correct_single"]
            tot_single += st["total_single"]

    denom = max(tot_steps, 1)
    return {
        "val_pol_loss": tot_pol / denom,
        "val_mse": tot_val / denom,
        "val_aux_mse": tot_aux / denom,
        "val_top1_acc": corr / max(tot_single, 1),
    }


def main() -> None:
    ap = argparse.ArgumentParser(description="Train Playernet v4 via stateful TBPTT distillation")
    ap.add_argument("--data", required=True, help="Dataset directory produced by convert4")
    ap.add_argument("--out", required=True, help="Output checkpoint directory")
    ap.add_argument("--init", default="", help="Optional initial checkpoint directory (e.g. Phase A)")
    ap.add_argument("--d-model", type=int, default=512)
    ap.add_argument("--layers", "--n-layers", dest="layers", type=int, default=8)
    ap.add_argument("--heads", "--n-heads", dest="heads", type=int, default=8)
    ap.add_argument("--d-ff", type=int, default=0, help="FFN width (0 = 4 * d_model)")
    ap.add_argument("--batch-size", type=int, default=4)
    ap.add_argument("--unroll", "--unroll-len", dest="unroll", type=int, default=16)
    ap.add_argument("--thinking-steps", type=int, default=2)
    ap.add_argument(
        "--thinking-curriculum",
        action="store_true",
        help="Cycle thinking_steps K across (1, 2, 4, 8) during training (Phase B)",
    )
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--max-steps", type=int, default=0, help="Stop after N optimizer steps if > 0")
    ap.add_argument("--max-shards", type=int, default=0, help="Limit shards loaded (0 = all)")
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--max-entities", type=int, default=M.MAX_ENTITIES)
    ap.add_argument("--num-workers", type=int, default=0)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    out_dir = Path(args.out).expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)
    data_dir = Path(args.data).expanduser()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    use_bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()

    train_ds = D4.TrajectoryDataset4(
        data_dir,
        split="train",
        unroll_len=args.unroll,
        max_entities=args.max_entities,
        max_shards=args.max_shards,
    )
    val_ds = D4.TrajectoryDataset4(
        data_dir,
        split="val",
        unroll_len=args.unroll,
        max_entities=args.max_entities,
        max_shards=1 if args.max_shards > 0 else 0,
    )
    if len(val_ds) == 0:
        val_ds = train_ds

    train_sampler = D4.StatefulBatchSampler4(
        train_ds, batch_size=args.batch_size, shuffle=True, seed=args.seed
    )
    val_sampler = D4.StatefulBatchSampler4(
        val_ds, batch_size=args.batch_size, shuffle=False, seed=args.seed
    )

    nw = args.num_workers if len(train_ds) >= 32 else 0
    train_loader = DataLoader(
        train_ds,
        batch_sampler=train_sampler,
        collate_fn=D4.collate_trajectories,
        num_workers=nw,
        pin_memory=(device.type == "cuda"),
        persistent_workers=(nw > 0),
    )
    val_loader = DataLoader(
        val_ds,
        batch_sampler=val_sampler,
        collate_fn=D4.collate_trajectories,
        num_workers=min(nw, 2),
        pin_memory=(device.type == "cuda"),
    )

    cfg = M.Config4(
        d_model=args.d_model,
        n_layers=args.layers,
        n_heads=args.heads,
        d_ff=args.d_ff if args.d_ff > 0 else args.d_model * 4,
        default_thinking_steps=args.thinking_steps,
    )
    net = M.PlayerNet4(cfg).to(device)
    if args.init:
        init_p = Path(args.init).expanduser()
        init_pt = init_p / "net.pt" if init_p.is_dir() else init_p
        ckpt_init = torch.load(init_pt, map_location="cpu", weights_only=False)
        net.load_state_dict(ckpt_init["state_dict"], strict=False)
        print(f"[train4] loaded initial weights from {init_pt}")
    val_head_params = [p for n, p in net.named_parameters() if "seat_value_head" in n]
    other_params = [p for n, p in net.named_parameters() if "seat_value_head" not in n]
    opt = torch.optim.AdamW(
        [
            {"params": other_params, "lr": args.lr, "weight_decay": 1e-2},
            {"params": val_head_params, "lr": args.lr * 0.2, "weight_decay": 5e-2},
        ]
    )

    n_params = sum(p.numel() for p in net.parameters())
    print(
        f"[train4] device={device} bf16={use_bf16} params={n_params / 1e6:.2f}M "
        f"train_segments={len(train_ds)} val_segments={len(val_ds)} "
        f"B={args.batch_size} U={args.unroll} K={args.thinking_steps}"
    )

    step_num = 0
    best_score = float("inf")
    best_state: dict[str, torch.Tensor] | None = None
    best_val_metrics: dict[str, float] = {}
    t0 = time.perf_counter()
    for epoch in range(1, args.epochs + 1):
        net.train()
        ep_pol = 0.0
        ep_val = 0.0
        ep_aux = 0.0
        ep_corr = 0
        ep_tot = 0
        ep_batches = 0
        h: torch.Tensor | None = None

        for batch in train_loader:
            B, U = batch["step_valid"].shape
            opt.zero_grad(set_to_none=True)
            is_new = batch["is_new_traj"].to(device)
            if h is None or h.shape[0] != B:
                h = torch.zeros(
                    (B, M.RECURRENT_SLOTS, cfg.d_model), dtype=torch.float32, device=device
                )
            else:
                h = h.detach()
                h = torch.where(is_new.view(B, 1, 1), torch.zeros_like(h), h)

            k_choices = (1, 2, 4, 8)
            traj_loss = torch.tensor(0.0, device=device)

            for t in range(U):
                k_val = (
                    k_choices[(step_num + t) % len(k_choices)]
                    if args.thinking_curriculum
                    else args.thinking_steps
                )
                is_dec_t = batch["is_decision"][:, t].to(device)
                k_tensor = torch.where(
                    is_dec_t,
                    torch.full((B,), k_val, dtype=torch.long, device=device),
                    torch.zeros((B,), dtype=torch.long, device=device),
                )
                with torch.autocast(
                    device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16
                ):
                    h_next, cand_l, pair_l, scal_p, val_p = net(
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
                        h,
                        k_tensor,
                    )
                    aux_p = net.aux_head(h_next[:, 0])
                loss_t, st = compute_step_loss(
                    cand_l.float(),
                    pair_l.float(),
                    scal_p.float(),
                    val_p.float(),
                    aux_p.float(),
                    batch,
                    t,
                )
                valid_t = batch["step_valid"][:, t].to(device)
                h = torch.where(valid_t.view(B, 1, 1), h_next.float(), h)
                traj_loss = traj_loss + loss_t / U
                ep_pol += st["pol_loss"] / U
                ep_val += st["val_loss"] / U
                ep_aux += st["aux_loss"] / U
                ep_corr += st["correct_single"]
                ep_tot += st["total_single"]

            traj_loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()

            step_num += 1
            ep_batches += 1
            if args.max_steps > 0 and step_num >= args.max_steps:
                break

        val_metrics = evaluate(net, val_loader, device, args.thinking_steps)
        score = val_metrics["val_pol_loss"] + 2.0 * val_metrics["val_mse"]
        if score < best_score:
            best_score = score
            best_state = {k: v.detach().cpu().clone() for k, v in net.state_dict().items()}
            best_val_metrics = dict(val_metrics)
        elapsed = time.perf_counter() - t0
        print(
            f"[train4] epoch={epoch} step={step_num} "
            f"train_pol={ep_pol / max(ep_batches, 1):.4f} "
            f"train_val={ep_val / max(ep_batches, 1):.4f} "
            f"train_aux={ep_aux / max(ep_batches, 1):.4f} "
            f"train_acc={ep_corr / max(ep_tot, 1):.3f} | "
            f"val_pol={val_metrics['val_pol_loss']:.4f} "
            f"val_mse={val_metrics['val_mse']:.4f} "
            f"val_aux={val_metrics['val_aux_mse']:.4f} "
            f"val_acc={val_metrics['val_top1_acc']:.3f} ({elapsed:.1f}s)",
            flush=True,
        )
        if args.max_steps > 0 and step_num >= args.max_steps:
            break

    final_state = best_state if best_state is not None else net.state_dict()
    final_metrics = best_val_metrics if best_val_metrics else val_metrics
    ckpt = {
        "config": cfg.__dict__,
        "state_dict": final_state,
        "step_num": step_num,
        "val_metrics": final_metrics,
    }
    torch.save(ckpt, out_dir / "net.pt")
    meta_out = {
        "encoder_version": M.ENCODER_VERSION,
        "dsl_version": M.DSL_VERSION,
        "d_model": cfg.d_model,
        "n_layers": cfg.n_layers,
        "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps,
        "step_num": step_num,
        "val_metrics": final_metrics,
    }
    (out_dir / "meta.json").write_text(json.dumps(meta_out, indent=2))
    print(f"[train4] saved checkpoint to {out_dir}")


if __name__ == "__main__":
    main()
