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
import random
import numpy as np
from pathlib import Path

import torch
from torch.utils.data import DataLoader

import dataset4 as D4
import model4 as M
from checkpoint4 import atomic_save, rng_state, restore_rng, move_optimizer


from .loss import compute_step_loss
from .validation import evaluate
from .validation_device import evaluate_device
from .device_loss import compute_device_loss
from .execution import plan_batch, transfer_batch, encode_boards, compile_regions
from .deferred import deferred_readouts
from .prefetch import prefetch_batches


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
    ap.add_argument("--resume", default="", help="Resume an atomic last.pt checkpoint")
    ap.add_argument("--checkpoint-every", type=int, default=100)
    ap.add_argument("--validation-every", type=int, default=1000)
    ap.add_argument("--activation-checkpointing", action="store_true")
    ap.add_argument("--time-budget-seconds", type=float, default=0)
    ap.add_argument("--dropout", type=float, default=0.0)
    ap.add_argument("--reference-execution", action="store_true", help="Use the original synchronized execution for A/B checks")
    ap.add_argument("--fused-optimizer", action="store_true", help="Fuse CUDA AdamW kernels; benchmark before enabling")
    ap.add_argument("--board-chunk", type=int, default=1, help="Independent observation encodings per trunk call; recurrence remains sequential")
    ap.add_argument("--compile-regions", action="store_true", help="Compile repeated transformer/thinking blocks with TorchInductor")
    ap.add_argument("--compile-readouts", action="store_true", help="Also compile recurrent updates and action/value readouts; requires --compile-regions")
    ap.add_argument("--cache-thinking-kv", action="store_true", help="Reuse this decision's board attention projections across thinking steps")
    ap.add_argument("--prune-unused-policy-heads", action="store_true", help="Skip only policy heads that contribute nothing to this frame's loss")
    ap.add_argument("--decision-chunk", type=int, default=1, help="Group independent readouts after causal memory updates; requires grouped boards and dropout=0")
    ap.add_argument("--gpu-prefetch", type=int, choices=range(4), default=0, help="Future CUDA input batches on a separate copy stream (0=off, 1=double buffer)")
    args = ap.parse_args()
    if args.compile_readouts and not args.compile_regions:
        ap.error("--compile-readouts requires --compile-regions")

    if args.decision_chunk > 1 and (args.reference_execution or args.board_chunk < 2 or args.dropout != 0):
        ap.error("--decision-chunk > 1 requires --board-chunk >= 2, dropout=0 and optimized execution")
    if args.decision_chunk < 1:
        ap.error("--decision-chunk must be positive")

    torch.set_num_threads(4)
    torch.manual_seed(args.seed)
    np.random.seed(args.seed)
    random.seed(args.seed)
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
    if len(train_ds) == 0 or len(val_ds) == 0:
        raise ValueError("Nonempty separate training and validation trajectories are required")

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
        generator=torch.Generator().manual_seed(args.seed + 1000),
        collate_fn=D4.collate_trajectories,
        num_workers=nw,
        pin_memory=(device.type == "cuda"),
        persistent_workers=(nw > 0),
    )
    val_loader = DataLoader(
        val_ds,
        batch_sampler=val_sampler,
        generator=torch.Generator().manual_seed(args.seed + 2000),
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
        dropout=args.dropout,
        activation_checkpointing=args.activation_checkpointing,
    )
    net = M.PlayerNet4(cfg).to(device)
    net.cache_thinking_kv = args.cache_thinking_kv
    net.decision_chunk = args.decision_chunk
    net.gpu_prefetch = args.gpu_prefetch
    net.prune_unused_policy_heads = args.prune_unused_policy_heads
    if args.init:
        init_p = Path(args.init).expanduser()
        init_pt = init_p / "net.pt" if init_p.is_dir() else init_p
        ckpt_init = torch.load(init_pt, map_location="cpu", weights_only=False)
        if (ckpt_init.get("trajectory_version") != M.TRAJECTORY_VERSION or ckpt_init.get("model_version") != M.MODEL_VERSION
                or ckpt_init.get("encoder_version") != M.ENCODER_VERSION):
            raise ValueError("Initial checkpoint uses an incompatible memory contract")
        net.load_state_dict(ckpt_init["state_dict"], strict=True)
        print(f"[train4] loaded initial weights from {init_pt}")
    val_head_params = [p for n, p in net.named_parameters() if "seat_value_head" in n]
    other_params = [p for n, p in net.named_parameters() if "seat_value_head" not in n]
    opt = torch.optim.AdamW(
        [
            {"params": other_params, "lr": args.lr, "weight_decay": 1e-2},
            {"params": val_head_params, "lr": args.lr * 0.2, "weight_decay": 5e-2},
        ],
        fused=True if args.fused_optimizer and device.type == "cuda" else None,
    )

    n_params = sum(p.numel() for p in net.parameters())
    print(
        f"[train4] device={device} bf16={use_bf16} params={n_params / 1e6:.2f}M "
        f"train_segments={len(train_ds)} val_segments={len(val_ds)} "
        f"B={args.batch_size} U={args.unroll} K={args.thinking_steps}"
    )

    step_num = 0
    best_score = float("inf")
    best_step = 0
    last_validation_step = -1
    last_validation_metrics = {}
    best_state: dict[str, torch.Tensor] | None = None
    best_val_metrics: dict[str, float] = {}
    resume_epoch, resume_batch, resume_h = 1, 0, None
    if args.resume:
        saved = torch.load(Path(args.resume).expanduser(), map_location="cpu", weights_only=False)
        if saved.get("dataset_fingerprint") != train_ds.fingerprint:
            raise ValueError("Resume dataset differs from the checkpoint")
        if saved["config"] != cfg.to_dict() or saved["batch_size"] != args.batch_size or saved["unroll"] != args.unroll:
            raise ValueError("Resume configuration/batch lanes must match exactly")
        net.load_state_dict(saved["state_dict"], strict=True)
        opt.load_state_dict(saved["optimizer"])
        for group in opt.param_groups:
            group["fused"] = True if args.fused_optimizer and device.type == "cuda" else None
        move_optimizer(opt, device)
        step_num = saved["step_num"]
        resume_epoch, resume_batch = saved["epoch"], saved["next_batch"]
        resume_h = saved["h"].to(device) if saved["h"] is not None else None
        best_score = saved.get("best_score", best_score)
        restore_rng(saved["rng"])
        if (out_dir / "net.pt").exists():
            previous_best = torch.load(out_dir / "net.pt", map_location="cpu", weights_only=False)
            best_state = previous_best["state_dict"]
            best_step = previous_best["step_num"]
            best_val_metrics = previous_best.get("val_metrics", {})
        print(f"[train4] resumed epoch={resume_epoch} batch={resume_batch} step={step_num}", flush=True)
    elif (out_dir / "last.pt").exists() or (out_dir / "net.pt").exists():
        raise FileExistsError("Output contains a run already; use --resume or a new output directory")

    def save_last(epoch, next_batch, hidden):
        atomic_save({"config": cfg.to_dict(), "state_dict": net.state_dict(),
                     "optimizer": opt.state_dict(), "rng": rng_state(),
                     "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": M.ENCODER_VERSION, "sampling_version": M.SAMPLING_VERSION,
                     "step_num": step_num, "epoch": epoch, "next_batch": next_batch,
                     "h": None if hidden is None else hidden.detach().cpu(),
                     "dataset_fingerprint": train_ds.fingerprint,
                     "batch_size": args.batch_size, "unroll": args.unroll,
                     "best_score": best_score}, out_dir / "last.pt")

    if args.compile_regions:
        if device.type != "cuda":
            raise ValueError("Regional compilation is currently verified on CUDA only")
        compile_regions(net, readouts=args.compile_readouts)

    def validate_and_save():
        nonlocal best_score, best_state, best_val_metrics, best_step
        nonlocal last_validation_step, last_validation_metrics
        if last_validation_step == step_num:
            return last_validation_metrics
        was_training = net.training
        metrics = (evaluate(net, val_loader, device, args.thinking_steps) if args.reference_execution
                   else evaluate_device(net, val_loader, device, args.thinking_steps, args.board_chunk))
        net.train(was_training)
        score = metrics["val_pol_loss"] + 2.0 * metrics["val_mse"]
        if score < best_score:
            best_score, best_step = score, step_num
            best_state = {k: v.detach().cpu().clone() for k, v in net.state_dict().items()}
            best_val_metrics = dict(metrics)
            atomic_save({"config": cfg.to_dict(), "state_dict": best_state,
                         "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": M.ENCODER_VERSION,
                         "sampling_version": M.SAMPLING_VERSION, "step_num": best_step,
                         "val_metrics": metrics, "dataset_fingerprint": train_ds.fingerprint}, out_dir / "net.pt")
        last_validation_step, last_validation_metrics = step_num, metrics
        print(f"[validation4] step={step_num} " + json.dumps(metrics, sort_keys=True), flush=True)
        return metrics

    t0 = time.perf_counter()
    stop = False
    val_metrics = dict(best_val_metrics)
    if resume_epoch > args.epochs:
        print("[train4] Requested epochs already complete; checkpoint preserved", flush=True)
        return
    for epoch in range(resume_epoch, args.epochs + 1):
        net.train()
        ep_pol = 0.0
        ep_val = 0.0
        ep_aux = 0.0
        ep_corr = 0
        ep_tot = 0
        epoch_stats = torch.zeros(5, device=device)
        ep_batches = 0
        h = resume_h if epoch == resume_epoch else None
        train_sampler.epoch = epoch - 1
        train_sampler.start_batch = resume_batch if epoch == resume_epoch else 0
        next_batch = train_sampler.start_batch

        for batch_idx, (host, staged) in enumerate(prefetch_batches(train_loader, device, args.gpu_prefetch),
                                                  start=train_sampler.start_batch):
            # A rare name-card question can pad many ordinary frames to a very
            # wide C. Keep large batches on the host and transfer one time step.
            plans = None if args.reference_execution else plan_batch(host, step_num, args.thinking_steps, args.thinking_curriculum)
            batch = transfer_batch(host, device) if staged is None else staged
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
            with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                boards = encode_boards(net, batch, args.board_chunk, device)

            prepared = None
            if args.decision_chunk > 1:
                prepared, final_hidden = deferred_readouts(net, batch, boards, h, plans,
                                                          args.decision_chunk, device, use_bf16)
            for t in range(U):
                if prepared is None:
                    k_val = (k_choices[(step_num + t) % len(k_choices)]
                             if args.thinking_curriculum else args.thinking_steps)
                    k_tensor = batch["is_decision"][:, t].to(device).long() * k_val
                    with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                        output = net(*[batch[name][:, t].to(device) for name in M.INPUT_NAMES[:15]], h, k_tensor,
                                     execution_hint=None if plans is None else plans[t].hint,
                                     encoded_board=None if boards is None else boards[t],
                                     policy_heads=plans[t].heads if plans is not None and args.prune_unused_policy_heads else None)
                else:
                    output = prepared[t]
                h_next, cand_l, pair_l, scal_p, val_p = output
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                    aux_p = net.aux_head(h_next[:, 0])
                loss_function = compute_step_loss if plans is None else compute_device_loss
                loss_t, st = loss_function(
                    cand_l.float(),
                    pair_l.float(),
                    scal_p.float(),
                    val_p.float(),
                    aux_p.float(),
                    batch,
                    t,
                    *([] if plans is None else [plans[t]]),
                )
                if prepared is None:
                    valid_t = batch["step_valid"][:, t].to(device)
                    h = torch.where(valid_t.view(B, 1, 1), h_next.float(), h)
                traj_loss = traj_loss + loss_t / U
                if plans is None:
                    ep_pol += st["pol_loss"] / U
                    ep_val += st["val_loss"] / U
                    ep_aux += st["aux_loss"] / U
                    ep_corr += st["correct_single"]
                    ep_tot += st["total_single"]
                else:
                    epoch_stats[:3] += st[:3] / U
                    epoch_stats[3:] += st[3:]

            if prepared is not None:
                h = final_hidden
            if not torch.isfinite(traj_loss):
                raise FloatingPointError(f"Non-finite loss at step {step_num}")
            traj_loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0, error_if_nonfinite=True)
            opt.step()

            step_num += 1
            ep_batches += 1
            next_batch = batch_idx + 1
            if step_num % 25 == 0:
                elapsed = time.perf_counter() - t0
                print(f"[train4] step={step_num} epoch={epoch} loss={float(traj_loss.detach()):.4f} "
                      f"batches_per_s={ep_batches / max(elapsed, 1):.2f} "
                      f"gpu_peak_gb={torch.cuda.max_memory_allocated() / 1e9 if device.type == 'cuda' else 0:.2f} "
                      f"gpu_alloc_retries={torch.cuda.memory_stats()['num_alloc_retries'] if device.type == 'cuda' else 0}", flush=True)
            stop = ((args.max_steps > 0 and step_num >= args.max_steps)
                    or (args.time_budget_seconds > 0 and time.perf_counter() - t0 >= args.time_budget_seconds))
            if step_num % args.checkpoint_every == 0 or stop:
                save_last(epoch, next_batch, h)
            if args.validation_every > 0 and step_num % args.validation_every == 0:
                validate_and_save()
                save_last(epoch, next_batch, h)
                # Validation can itself exhaust the wall-time budget. Stop at
                # this already-validated checkpoint instead of taking one more
                # update and triggering another full validation immediately.
                stop = stop or (args.time_budget_seconds > 0 and time.perf_counter() - t0 >= args.time_budget_seconds)
            if stop:
                break

        val_metrics = validate_and_save()
        if not args.reference_execution:
            ep_pol, ep_val, ep_aux, ep_corr, ep_tot = epoch_stats.cpu().tolist()
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
        save_last(epoch if stop else epoch + 1, next_batch if stop else 0, h if stop else None)
        if stop:
            break

    final_state = best_state if best_state is not None else net.state_dict()
    final_metrics = best_val_metrics if best_val_metrics else val_metrics
    ckpt = {
        "config": cfg.__dict__,
        "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": M.ENCODER_VERSION,
        "sampling_version": M.SAMPLING_VERSION,
        "dataset_fingerprint": train_ds.fingerprint,
        "state_dict": final_state,
        "step_num": best_step if best_state is not None else step_num,
        "val_metrics": final_metrics,
    }
    atomic_save(ckpt, out_dir / "net.pt")
    meta_out = {
        "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": M.ENCODER_VERSION,
        "sampling_version": M.SAMPLING_VERSION,
        "dataset_fingerprint": train_ds.fingerprint,
        "dsl_version": M.DSL_VERSION,
        "d_model": cfg.d_model,
        "n_layers": cfg.n_layers,
        "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps,
        "step_num": best_step if best_state is not None else step_num,
        "val_metrics": final_metrics,
    }
    (out_dir / "meta.json").write_text(json.dumps(meta_out, indent=2))
    print(f"[train4] saved checkpoint to {out_dir}")


if __name__ == "__main__":
    main()
