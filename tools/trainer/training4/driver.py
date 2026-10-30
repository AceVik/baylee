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

import json
import os
import time
import random
import numpy as np
from pathlib import Path

import torch
from torch.utils.data import DataLoader

import dataset4 as D4
import model4 as M
from checkpoint4 import atomic_save, rng_state, restore_rng, move_optimizer
from torch_io4 import atomic_json


from .loss import compute_step_loss
from .validation import evaluate
from .validation_device import evaluate_device
from .device_loss import compute_device_loss, compute_unroll_device_loss
from .execution import plan_batch, transfer_batch, encode_boards, compile_regions
from .deferred import deferred_readouts
from .prefetch import prefetch_batches
from .checkpoint_async import AsyncCheckpointWriter
from .depth_schedule import DepthSchedule, evaluate_depths
from .objective import selection_score, check_resume_objective
from .optimizer_groups import build_optimizer_groups, check_resume_schedule
from .optimizer_step import update_parameters


def main(default_architecture="v4") -> None:
    global M
    from .cli import parse_args
    args, M, identity = parse_args(default_architecture)
    loss_config = args.supervised_loss
    from .supervised_options import auxiliary_readout

    # Deterministic CUDA scatter/reduction kernels are required for exact resume.
    os.environ.setdefault("CUBLAS_WORKSPACE_CONFIG", ":4096:8")
    torch.use_deterministic_algorithms(True)
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
        encoder_version=args.encoder_version,
        split="train",
        unroll_len=args.unroll,
        max_entities=args.max_entities,
        max_shards=args.max_shards,
    )
    val_ds = D4.TrajectoryDataset4(
        Path(args.validation_data).expanduser() if args.validation_data else data_dir,
        encoder_version=args.encoder_version,
        split="val",
        unroll_len=args.unroll,
        max_entities=args.max_entities,
        max_shards=1 if args.max_shards > 0 else 0,
    )
    if len(train_ds) == 0 or len(val_ds) == 0:
        raise ValueError("Nonempty separate training and validation trajectories are required")
    if train_ds.structure.digest != val_ds.structure.digest:
        raise ValueError("Training and validation use different public rule catalogs")

    train_sampler = D4.StatefulBatchSampler4(
        train_ds, batch_size=args.batch_size, shuffle=True, seed=args.seed
    )
    if args.bc_replay_data:
        from .bc_mixture import BCMixture, BCMixtureSampler
        replay_path = Path(args.bc_replay_data).expanduser()
        replay = train_ds if replay_path.resolve() == data_dir.resolve() else D4.TrajectoryDataset4(
            replay_path, encoder_version=args.encoder_version, split="train", unroll_len=args.unroll, max_entities=args.max_entities)
        train_ds = BCMixture(train_ds, replay, val_ds, args.batch_size)
        train_sampler = BCMixtureSampler(train_ds, args.batch_size, args.seed, args.bc_epoch_batches)
        identity["bc_mixture"] = train_ds.contract
    val_sampler = D4.StatefulBatchSampler4(
        val_ds, batch_size=args.validation_batch_size, shuffle=False, seed=args.seed
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

    config_class = M.Config5 if args.architecture == "v5" else M.Config4
    cfg = config_class(
        global_feat_width=82 if args.encoder_version == 9 else 80,
        **(dict(relational_revision=args.relational_revision,
                 candidate_context_revision=args.candidate_context_revision,
                 relational_steps=args.relational_steps) if args.architecture == 'v5' else {}),
        d_model=args.d_model,
        n_layers=args.layers,
        n_heads=args.heads,
        d_ff=args.d_ff if args.d_ff > 0 else args.d_model * 4,
        default_thinking_steps=args.thinking_steps,
        dropout=args.dropout,
        activation_checkpointing=args.activation_checkpointing,
    )
    if args.architecture == "v5":
        cfg.relational_steps = args.relational_steps
        cfg.relational_revision = args.relational_revision
        cfg.candidate_context_revision = args.candidate_context_revision
        identity["architecture"] = M.architecture_for_config(cfg)
    # Extra architecture initialization must not perturb training RNG in paired runs.
    net = M.PlayerNet5(cfg) if args.architecture == "v5" else M.PlayerNet4(cfg)
    net.compact_thinking_rows = args.compact_thinking_rows
    net.detach_value_input = loss_config.detach_value_input
    net.attach_structure(train_ds.structure.data)
    net = net.to(device)
    net.saved_activation_layers = args.saved_activation_layers
    net.offload_checkpoint_inputs = args.offload_checkpoint_inputs
    net.cache_thinking_kv = args.cache_thinking_kv
    net.decision_chunk = args.decision_chunk
    net.gpu_prefetch = args.gpu_prefetch
    net.prune_unused_policy_heads = args.prune_unused_policy_heads
    if args.init:
        init_p = Path(args.init).expanduser()
        init_pt = init_p / "net.pt" if init_p.is_dir() else init_p
        ckpt_init = torch.load(init_pt, map_location="cpu", weights_only=False)
        if (ckpt_init.get("trajectory_version") != M.TRAJECTORY_VERSION or ckpt_init.get("model_version") not in ((3, 5) if args.architecture == "v5" else (3,))):
            raise ValueError("Initial checkpoint uses an incompatible memory contract")
        from encoder_contract import check_initial_encoder
        provenance = check_initial_encoder(ckpt_init.get("encoder_version"), args.encoder_version,
                                           args.encoder_warmstart_from7, args.encoder_warmstart_to9)
        if provenance:
            provenance["source_checkpoint"] = str(init_pt.resolve())
            from torch_io4 import file_digest
            provenance["source_checkpoint_sha256"] = file_digest(init_pt)
            identity["encoder_warmstart"] = provenance
        from model4.structure import structure_digest
        if structure_digest(ckpt_init["structure"]) != train_ds.structure.digest:
            raise ValueError("Initial checkpoint uses a different public rules catalog")
        if args.encoder_warmstart_to9:
            from value_calibration4.model_identity import source_model
            source_net = source_model(ckpt_init)
            source_config = source_net.cfg.to_dict()
            source_config['global_feat_width'] = 82
            if source_config != cfg.to_dict():
                raise ValueError('Encoder9 transfer must preserve all other model configuration')
            upgraded = M.warmstart_encoder9(ckpt_init['state_dict'], net)
        elif args.candidate_context_warmstart:
            source_cfg = M.Config5(**ckpt_init["config"])
            expected = cfg.to_dict()
            expected["candidate_context_revision"] = source_cfg.candidate_context_revision
            if (ckpt_init.get("model_version") != 5 or source_cfg.relational_revision != 2
                    or source_cfg.candidate_context_revision >= cfg.candidate_context_revision
                    or source_cfg.to_dict() != expected
                    or ckpt_init.get("architecture") != M.architecture_for_config(source_cfg)
                    or ckpt_init.get("encoder_version") != args.encoder_version):
                raise ValueError("Candidate context transfer source architecture/config differs")
            upgraded = M.warmstart_candidate_context(ckpt_init["state_dict"], net)
            from torch_io4 import file_digest
            source_sha = file_digest(init_pt)
            identity["candidate_context_warmstart"] = {
                "kind": "weights_only", "source_checkpoint_sha256": source_sha,
                "source_architecture": ckpt_init["architecture"], "target_architecture": identity["architecture"],
                "source_config": source_cfg.to_dict(), "target_config": cfg.to_dict()}
        elif args.architecture == "v5" and ckpt_init["model_version"] == 3:
            upgraded = M.warmstart_from_v4(ckpt_init["state_dict"], net)
        else:
            if args.architecture == "v5" and (ckpt_init.get("architecture") != M.architecture_for_config(cfg)
                    or M.Config5(**ckpt_init["config"]).to_dict() != cfg.to_dict()):
                raise ValueError("V5 initialization architecture/config differs")
            upgraded = (dict(ckpt_init["state_dict"]) if args.architecture == "v5"
                        else M.upgrade_state_dict_to_v7(ckpt_init["state_dict"], net))
        net.load_state_dict(upgraded, strict=True)
        print(f"[train4] loaded initial weights from {init_pt}")
    if args.freeze_trunk:
        head_prefixes = (
            "cand_proj.",
            "cand_ctx_proj.",
            "cand_context_gate.",
            "cand_scorer.",
            "pair_blk_proj.",
            "pair_atk_proj.",
            "pair_scorer.",
            "scalar_head.",
        )
        for n, p in net.named_parameters():
            p.requires_grad_(n.startswith(head_prefixes))
    opt_groups, optimizer_schedule = build_optimizer_groups(
        net, args.lr, architecture=args.architecture, freeze_trunk=args.freeze_trunk,
        relational_lr=args.relational_lr, output_lr=args.relational_output_lr,
        warmup_steps=args.relational_warmup_steps)
    opt = torch.optim.AdamW(
        opt_groups,
        fused=True if args.fused_optimizer and device.type == "cuda" else None,
    )
    package_parent = None
    if args.init_state and args.architecture == "v5":
        raise ValueError("V5 package continuation requires explicit verified support; use --init or --resume")
    if args.init_state:
        from .package_state import load_package_state
        package_parent = load_package_state(args.init_state, net, opt, device)
        print("[train4] package continuation " + json.dumps(package_parent), flush=True)

    n_params = sum(p.numel() for p in net.parameters())
    print(
        f"[train4] device={device} bf16={use_bf16} params={n_params / 1e6:.2f}M "
        f"train_segments={len(train_ds)} val_segments={len(val_ds)} "
        f"B={args.batch_size} U={args.unroll} K={args.thinking_steps} validation_B={args.validation_batch_size}"
    )

    depth_schedule = DepthSchedule.from_training(args.thinking_steps, args.thinking_curriculum)
    depth_counts = {}
    step_num = 0
    optimizer_updates = 0
    best_optimizer_updates = 0
    best_score = float("inf")
    best_step = 0
    last_validation_step = -1
    last_validation_metrics = {}
    best_state: dict[str, torch.Tensor] | None = None
    best_val_metrics: dict[str, float] = {}
    resume_epoch, resume_batch, resume_h = 1, 0, None
    if args.resume:
        saved = torch.load(Path(args.resume).expanduser(), map_location="cpu", weights_only=False)
        if saved.get("model_version") != M.MODEL_VERSION or any(saved.get(k) != v for k, v in identity.items()):
            raise ValueError("Resume model architecture differs")
        from .bc_mixture import check_resume
        check_resume(saved, getattr(train_ds, "contract", None))
        check_resume_objective(saved, args.checkpoint_selection, loss_config)
        check_resume_schedule(saved, optimizer_schedule)
        depth_schedule.check_resume(saved)
        depth_counts = saved.get("depth_counts", {})
        if (saved.get("training_sampler_version", 1) != D4.StatefulBatchSampler4.VERSION
                or saved.get("sampler_seed") != args.seed):
            raise ValueError("Resume training sampler version/seed differs; use --init-state for fresh lanes")
        if saved.get("dataset_fingerprint") != train_ds.fingerprint:
            raise ValueError("Resume dataset differs from the checkpoint")
        if saved.get("validation_fingerprint", val_ds.fingerprint) != val_ds.fingerprint:
            raise ValueError("Resume validation dataset differs from the checkpoint")
        if (config_class(**saved["config"]).to_dict() != cfg.to_dict()) or saved["batch_size"] != args.batch_size or saved["unroll"] != args.unroll:
            raise ValueError("Resume configuration/batch lanes must match exactly")
        if saved.get("validation_batch_size", saved["batch_size"]) != args.validation_batch_size:
            raise ValueError("Resume validation batch lanes must match the saved scoring protocol")
        for receipt in ("encoder_warmstart", "candidate_context_warmstart"):
            if receipt in saved:
                identity[receipt] = saved[receipt]
        net.load_state_dict(saved["state_dict"], strict=True)
        opt.load_state_dict(saved["optimizer"])
        for group in opt.param_groups:
            group["fused"] = True if args.fused_optimizer and device.type == "cuda" else None
        move_optimizer(opt, device)
        step_num = saved["step_num"]
        optimizer_updates = saved.get("optimizer_updates", step_num)
        if not 0 <= optimizer_updates <= step_num:
            raise ValueError("Resume optimizer update count exceeds consumed batches")
        resume_epoch, resume_batch = saved["epoch"], saved["next_batch"]
        resume_h = saved["h"].to(device) if saved["h"] is not None else None
        best_score = saved.get("best_score", best_score)
        restore_rng(saved["rng"])
        if (out_dir / "net.pt").exists():
            previous_best = torch.load(out_dir / "net.pt", map_location="cpu", weights_only=False)
            if previous_best.get("encoder_version") != args.encoder_version:
                raise ValueError("Best checkpoint encoder differs from resumed run")
            best_state = previous_best["state_dict"]
            best_step = previous_best["step_num"]
            best_optimizer_updates = previous_best.get("optimizer_updates", best_step)
            best_val_metrics = previous_best.get("val_metrics", {})
        print(f"[train4] resumed epoch={resume_epoch} batch={resume_batch} step={step_num}", flush=True)
    elif (out_dir / "last.pt").exists() or (out_dir / "net.pt").exists():
        raise FileExistsError("Output contains a run already; use --resume or a new output directory")

    checkpoint_writer = AsyncCheckpointWriter(device, enabled=args.async_checkpoint)

    def save_last(epoch, next_batch, hidden):
        checkpoint_writer.save({**identity, "optimizer_schedule": optimizer_schedule, "depth_schedule": depth_schedule.to_dict(), "depth_counts": depth_counts, "config": cfg.to_dict(), "structure": train_ds.structure.data, "state_dict": net.state_dict(),
                     "optimizer": opt.state_dict(), "rng": rng_state(),
                     "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": args.encoder_version, "sampling_version": M.SAMPLING_VERSION,
                     "step_num": step_num, "epoch": epoch, "next_batch": next_batch,
                     "optimizer_updates": optimizer_updates,
                     "h": None if hidden is None else (hidden.detach() if args.async_checkpoint else hidden.detach().cpu()),
                     "dataset_fingerprint": train_ds.fingerprint,
                     "validation_fingerprint": val_ds.fingerprint,
                     "package_parent": package_parent,
                     "training_sampler_version": D4.StatefulBatchSampler4.VERSION,
                     "training_loss_version": loss_config.version, "supervised_loss_config": loss_config.to_dict(), "checkpoint_selection": args.checkpoint_selection,
                     "sampler_seed": args.seed,
                     "batch_size": args.batch_size, "unroll": args.unroll,
                     "validation_batch_size": args.validation_batch_size,
                     "last_validation_step": last_validation_step,
                     "last_validation_metrics": dict(last_validation_metrics),
                     "best_score": best_score}, out_dir / "last.pt")

    if args.compile_regions:
        if device.type != "cuda":
            raise ValueError("Regional compilation is currently verified on CUDA only")
        compile_regions(net, readouts=args.compile_readouts)

    def validate_and_save():
        nonlocal best_score, best_state, best_val_metrics, best_step
        nonlocal best_optimizer_updates
        nonlocal last_validation_step, last_validation_metrics
        if last_validation_step == step_num:
            return last_validation_metrics
        print(f"[validation4] starting full held-out validation at step={step_num}", flush=True)
        was_training = net.training
        metrics = evaluate_depths(net, val_loader, device, depth_schedule,
            lambda n, loader, dev, k: evaluate(n, loader, dev, k) if args.reference_execution
            else evaluate_device(n, loader, dev, k, args.board_chunk, progress_every=500))
        net.train(was_training)
        score = selection_score(metrics, args.checkpoint_selection)
        if score < best_score:
            best_score, best_step = score, step_num
            best_optimizer_updates = optimizer_updates
            best_state = {k: v.detach().cpu().clone() for k, v in net.state_dict().items()}
            best_val_metrics = dict(metrics)
            atomic_save({**identity, "optimizer_schedule": optimizer_schedule, "depth_schedule": depth_schedule.to_dict(), "depth_counts": depth_counts, "config": cfg.to_dict(), "structure": train_ds.structure.data, "state_dict": best_state,
                         "training_loss_version": loss_config.version, "supervised_loss_config": loss_config.to_dict(), "checkpoint_selection": args.checkpoint_selection,
                         "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": args.encoder_version,
                         "sampling_version": M.SAMPLING_VERSION, "step_num": best_step,
                         "optimizer_updates": best_optimizer_updates,
                         "validation_batch_size": args.validation_batch_size,
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
    with checkpoint_writer:
        if args.validate_initial and not args.resume:
            validate_and_save()
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
                plans = None if args.reference_execution else plan_batch(host, step_num, schedule=depth_schedule)
                depth_schedule.count_batch(host, step_num, depth_counts)
                batch = transfer_batch(host, device) if staged is None else staged
                B, U = batch["step_valid"].shape
                opt.zero_grad(set_to_none=True)
                is_new = batch["is_new_traj"].to(device, non_blocking=True)
                if h is None or h.shape[0] != B:
                    h = torch.zeros(
                        (B, M.RECURRENT_SLOTS, cfg.d_model), dtype=torch.float32, device=device
                    )
                else:
                    h = h.detach()
                    h = torch.where(is_new.view(B, 1, 1), torch.zeros_like(h), h)

                traj_loss = torch.zeros((), device=device)
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                    net.clear_structure()
                    net.prepare_structure(
                        None if args.reference_execution else (host["entity_rules"], host["cand_rules"])
                    )
                    boards = encode_boards(net, batch, args.board_chunk, device)

                prepared = None
                if args.decision_chunk > 1:
                    prepared, final_hidden = deferred_readouts(net, batch, boards, h, plans,
                                                              args.decision_chunk, device, use_bf16)
                    with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                        aux_all = auxiliary_readout(net, torch.stack([prepared[t][0][:, 0] for t in range(U)], dim=1), loss_config)
                    traj_loss, st = compute_unroll_device_loss(prepared, aux_all.float(), batch, plans, loss_config)
                    epoch_stats[:3] += st[:3] / U
                    epoch_stats[3:] += st[3:]
                    h = final_hidden
                else:
                    full_outputs, full_aux = [], []
                    for t in range(U):
                        k_val = depth_schedule.depth(step_num, t)
                        k_tensor = batch["is_decision"][:, t].to(device).long() * k_val
                        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                            output = net(*[batch[name][:, t].to(device) for name in M.INPUT_NAMES[:-2]], h, k_tensor,
                                         execution_hint=None if plans is None else plans[t].hint,
                                         encoded_board=None if boards is None else boards[t],
                                         policy_heads=plans[t].heads if plans is not None and args.prune_unused_policy_heads else None)
                        h_next, cand_l, pair_l, scal_p, val_p = output
                        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=use_bf16):
                            aux_p = auxiliary_readout(net, h_next[:, 0], loss_config)
                        valid_t = batch["step_valid"][:, t].to(device)
                        h = torch.where(valid_t.view(B, 1, 1), h_next.float(), h)
                        if loss_config.version == 5:
                            full_outputs.append((h_next, cand_l.float(), pair_l.float(), scal_p.float(), val_p.float()))
                            full_aux.append(aux_p.float())
                        else:
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
                    if loss_config.version == 5:
                        traj_loss, st = compute_unroll_device_loss(full_outputs, torch.stack(full_aux, dim=1), batch, plans, loss_config)
                        epoch_stats[:3] += st[:3] / U
                        epoch_stats[3:] += st[3:]
                updated = update_parameters(traj_loss, net, opt, checkpoint_writer,
                                            optimizer_schedule, optimizer_updates + 1)
                optimizer_updates += int(updated)
                net.clear_structure()

                step_num += 1
                ep_batches += 1
                next_batch = batch_idx + 1
                if step_num % 25 == 0:
                    elapsed = time.perf_counter() - t0
                    print(f"[train4] step={step_num} epoch={epoch} loss={float(traj_loss.detach()):.4f} "
                          f"optimizer_updates={optimizer_updates} "
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
        **identity, "optimizer_schedule": optimizer_schedule, "depth_schedule": depth_schedule.to_dict(), "depth_counts": depth_counts,
        "training_loss_version": loss_config.version, "supervised_loss_config": loss_config.to_dict(), "checkpoint_selection": args.checkpoint_selection,
        "validation_batch_size": args.validation_batch_size,
        "config": cfg.__dict__,
        "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": args.encoder_version,
        "sampling_version": M.SAMPLING_VERSION,
        "dataset_fingerprint": train_ds.fingerprint,
        "structure": train_ds.structure.data, "state_dict": final_state,
        "step_num": best_step if best_state is not None else step_num,
        "optimizer_updates": best_optimizer_updates if best_state is not None else optimizer_updates,
        "val_metrics": final_metrics,
        "validation_fingerprint": val_ds.fingerprint,
        "package_parent": package_parent,
    }
    atomic_save(ckpt, out_dir / "net.pt")
    meta_out = {
        **identity, "optimizer_schedule": optimizer_schedule, "depth_schedule": depth_schedule.to_dict(), "depth_counts": depth_counts,
        "training_loss_version": loss_config.version, "supervised_loss_config": loss_config.to_dict(), "checkpoint_selection": args.checkpoint_selection,
        "validation_batch_size": args.validation_batch_size,
        "trajectory_version": M.TRAJECTORY_VERSION, "model_version": M.MODEL_VERSION, "encoder_version": args.encoder_version,
        "sampling_version": M.SAMPLING_VERSION,
        "dataset_fingerprint": train_ds.fingerprint,
        "dsl_version": M.DSL_VERSION,
        "structure_version": 1, "structure_sha256": train_ds.structure.digest,
        "d_model": cfg.d_model,
        "n_layers": cfg.n_layers,
        "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps,
        "step_num": best_step if best_state is not None else step_num,
        "optimizer_updates": best_optimizer_updates if best_state is not None else optimizer_updates,
        "consumed_batches": step_num,
        "total_optimizer_updates": optimizer_updates,
        "batches_without_update": step_num - optimizer_updates,
        "val_metrics": final_metrics,
        "validation_fingerprint": val_ds.fingerprint,
        "package_parent": package_parent,
    }
    atomic_json(out_dir / "meta.json", meta_out)
    print(f"[train4] saved checkpoint to {out_dir}")


if __name__ == "__main__":
    main()
