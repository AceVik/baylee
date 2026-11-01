"""Validation with CPU-derived execution plans and device-side metric reduction.

Metric definitions match validation.evaluate. Only aggregation and execution are
batched; observations, per-seat state resets and active-choice denominators stay
unchanged. Float64 accumulators keep long validation runs numerically stable.
"""
import time
import torch
import model4 as M
from .device_loss import compute_device_loss, compute_unroll_device_loss
from .execution import plan_batch, transfer_batch, encode_boards
from .deferred import deferred_readouts
from .prefetch import prefetch_batches
from .objective import choice_statistics


@torch.no_grad()
def evaluate_device(net, loader, device, thinking_steps, board_chunk=8, progress_every=0,
                    diagnostic_collector=None, use_bf16=None):
    net.eval()
    net.clear_structure()
    net.prepare_structure()
    totals = torch.zeros(11, dtype=torch.float64, device=device)
    calibration = torch.zeros(3, 10, dtype=torch.float64, device=device)
    choices = torch.zeros(3, dtype=torch.float64, device=device)
    h = None
    bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()
    if use_bf16 is not None:
        bf16 = bool(use_bf16) and device.type == "cuda" and torch.cuda.is_bf16_supported()
    started = time.perf_counter()
    for batch_index, (host, staged) in enumerate(prefetch_batches(loader, device, getattr(net, "gpu_prefetch", 0)), start=1):
        if diagnostic_collector is not None:
            diagnostic_collector.start_batch(host)
        plans = plan_batch(host, 0, thinking_steps)
        batch = transfer_batch(host, device) if staged is None else staged
        b, u = batch["step_valid"].shape
        new = batch["is_new_traj"].to(device, non_blocking=True)
        if h is None or h.shape[0] != b:
            h = torch.zeros(b, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
        else:
            h = torch.where(new[:, None, None], torch.zeros_like(h), h)
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            boards = encode_boards(net, batch, board_chunk, device)
        if getattr(net, "decision_chunk", 1) > 1:
            prepared, final_hidden = deferred_readouts(net, batch, boards, h, plans,
                                                      net.decision_chunk, device, bf16)
            with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                aux_all = net.aux_head(torch.stack([prepared[t][0][:, 0] for t in range(u)], dim=1))
            _, stats = compute_unroll_device_loss(prepared, aux_all.float(), batch, plans)
            valid_all = batch["step_valid"].to(device, non_blocking=True)
            truth_all = batch["outcome"].to(device, non_blocking=True)
            aux_t_all = batch["aux_target"].to(device, non_blocking=True)
            pred_all = torch.stack([prepared[t][4].float() for t in range(u)], dim=1)
            val_sse = ((pred_all - truth_all).square() * valid_all).sum()
            aux_sse = ((aux_all.float() - aux_t_all).square().mean(-1) * valid_all).sum()
            n_tot = valid_all.sum()
            totals[:6] += torch.stack((stats[0], val_sse, aux_sse, n_tot, stats[3], stats[4])).double()
            seats_all = batch["seat_feat"].to(device, non_blocking=True)
            alive = (seats_all[..., M.SEAT_FEAT_PRESENT] > .5) & (seats_all[..., M.SEAT_FEAT_LOST] < .5)
            allied = (seats_all[..., M.SEAT_FEAT_SELF] > .5) | (seats_all[..., M.SEAT_FEAT_TEAMMATE] > .5)
            prior = (alive & allied).sum(-1) / alive.sum(-1).clamp_min(1)
            dec_all = batch["is_decision"].to(device, non_blocking=True)
            head_all = batch["head"].to(device, non_blocking=True)
            sidx_all = batch["single_idx"].to(device, non_blocking=True)
            cmask_all = batch["cand_mask"].to(device, non_blocking=True)
            cfeat0_all = batch["cand_feat"][..., 0].to(device, non_blocking=True)
            w_all = batch["loss_weight"].to(device, non_blocking=True)
            act_sum = torch.zeros((), dtype=torch.float64, device=device)
            corr_sum = torch.zeros((), dtype=torch.float64, device=device)
            for t, plan in enumerate(plans):
                if plan.heads[0]:
                    single = valid_all[:, t] & dec_all[:, t] & (head_all[:, t] == 0)
                    target = torch.where(single, sidx_all[:, t], 0)
                    cand_t = prepared[t][1]
                    choices += choice_statistics(cand_t, cmask_all[:, t, :cand_t.shape[1]],
                                                 single, target, w_all[:, t]).double()
                    is_pass = cfeat0_all[:, t].gather(1, target[:, None])[:, 0] > .5
                    active = single & ~is_pass
                    act_sum = act_sum + active.sum().double()
                    corr_sum = corr_sum + ((cand_t.argmax(-1) == target) & active).sum().double()
            totals[6:] += torch.stack((((prior - truth_all).square() * valid_all).sum().double(),
                                       (pred_all * valid_all).sum().double(),
                                       (truth_all * valid_all).sum().double(),
                                       act_sum, corr_sum))
            bins = (pred_all * 10).long().clamp(0, 9).reshape(-1)
            v_flat = valid_all.double().reshape(-1)
            calibration[0].scatter_add_(0, bins, v_flat)
            calibration[1].scatter_add_(0, bins, (pred_all.double() * valid_all).reshape(-1))
            calibration[2].scatter_add_(0, bins, (truth_all.double() * valid_all).reshape(-1))
            if diagnostic_collector is not None:
                for t, output in enumerate(prepared):
                    diagnostic_collector.step(batch, t, output)
            h = final_hidden
        else:
            for t in range(u):
                at = lambda key: batch[key][:, t].to(device, non_blocking=True)
                thinking = at("is_decision").long() * thinking_steps
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    output = net(*[at(n) for n in M.INPUT_NAMES[:-2]], h, thinking,
                                 execution_hint=plans[t].hint, encoded_board=None if boards is None else boards[t],
                                 policy_heads=plans[t].heads if getattr(net, "prune_unused_policy_heads", False) else None)
                    auxiliary = net.aux_head(output[0][:, 0])
                if diagnostic_collector is not None:
                    diagnostic_collector.step(batch, t, output)
                _, stats = compute_device_loss(*[v.float() for v in output[1:]], auxiliary.float(), batch, t, plans[t])
                valid = at("step_valid")
                h = torch.where(valid[:, None, None], output[0].float(), h)
                n = valid.sum()
                totals[:6] += torch.stack((stats[0], stats[1]*n, stats[2]*n, n, stats[3], stats[4])).double()
                prediction, truth = output[4].float(), at("outcome")
                seats = at("seat_feat")
                alive = (seats[:, :, M.SEAT_FEAT_PRESENT] > .5) & (seats[:, :, M.SEAT_FEAT_LOST] < .5)
                allied = (seats[:, :, M.SEAT_FEAT_SELF] > .5) | (seats[:, :, M.SEAT_FEAT_TEAMMATE] > .5)
                prior = (alive & allied).sum(-1) / alive.sum(-1).clamp_min(1)
                single = valid & at("is_decision") & (at("head") == 0)
                target = torch.where(single, at("single_idx"), 0)
                choices += choice_statistics(output[1], at("cand_mask")[:, :output[1].shape[1]],
                                             single, target, at("loss_weight")).double()
                is_pass = at("cand_feat")[:, :, 0].gather(1, target[:, None])[:, 0] > .5
                active = single & ~is_pass
                correct = ((output[1].argmax(-1) == target) & active).sum()
                totals[6:] += torch.stack((((prior-truth).square()*valid).sum(),
                                         (prediction*valid).sum(), (truth*valid).sum(),
                                         active.sum(), correct)).double()
                bins = (prediction * 10).long().clamp(0, 9)
                calibration[0].scatter_add_(0, bins, valid.double())
                calibration[1].scatter_add_(0, bins, prediction.double()*valid)
                calibration[2].scatter_add_(0, bins, truth.double()*valid)
        if diagnostic_collector is not None:
            diagnostic_collector.finish_batch(host)
        if progress_every and batch_index % progress_every == 0:
            print(f"[validation4] batches={batch_index}/{len(loader)} elapsed_s={time.perf_counter()-started:.1f}", flush=True)
    values = totals.cpu().tolist()
    pol, val, aux, frames, correct, singles, baseline, predicted, truth, active, active_correct = values
    denominator = max(frames, 1)
    choice_nll, choice_correct, choice_count = choices.cpu().tolist()
    return {
        "val_choice_nll": choice_nll / max(choice_count, 1),
        "val_choice_top1_acc": choice_correct / max(choice_count, 1),
        "val_choices": int(choice_count),
        "val_pol_loss": pol / max(len(loader)*loader.dataset.unroll_len,1),
        "val_mse": val/denominator, "val_aux_mse": aux/denominator,
        "val_top1_acc": correct/max(singles,1),
        "val_active_top1_acc": active_correct/max(active,1),
        "val_active_choices": int(active), "val_uniform_seat_mse": baseline/denominator,
        "val_ece": float((calibration[1]-calibration[2]).abs().sum().cpu())/denominator,
        "val_prediction_mean": predicted/denominator, "val_outcome_mean": truth/denominator,
        "val_frames": int(frames),
    }
