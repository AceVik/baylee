"""Validation with CPU-derived execution plans and device-side metric reduction.

Metric definitions match validation.evaluate. Only aggregation and execution are
batched; observations, per-seat state resets and active-choice denominators stay
unchanged. Float64 accumulators keep long validation runs numerically stable.
"""
import torch
import model4 as M
from .device_loss import compute_device_loss
from .execution import plan_batch, transfer_batch, encode_boards


@torch.no_grad()
def evaluate_device(net, loader, device, thinking_steps, board_chunk=8):
    net.eval()
    totals = torch.zeros(11, dtype=torch.float64, device=device)
    calibration = torch.zeros(3, 10, dtype=torch.float64, device=device)
    h = None
    bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()
    for host in loader:
        plans = plan_batch(host, 0, thinking_steps)
        batch = transfer_batch(host, device)
        b, u = batch["step_valid"].shape
        new = batch["is_new_traj"].to(device)
        if h is None or h.shape[0] != b:
            h = torch.zeros(b, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
        else:
            h = torch.where(new[:, None, None], torch.zeros_like(h), h)
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            boards = encode_boards(net, batch, board_chunk, device)
        for t in range(u):
            at = lambda key: batch[key][:, t].to(device, non_blocking=True)
            thinking = at("is_decision").long() * thinking_steps
            with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                output = net(*[at(n) for n in M.INPUT_NAMES[:15]], h, thinking,
                             execution_hint=plans[t].hint, encoded_board=None if boards is None else boards[t],
                             policy_heads=plans[t].heads if getattr(net, "prune_unused_policy_heads", False) else None)
                auxiliary = net.aux_head(output[0][:, 0])
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
    values = totals.cpu().tolist()
    pol, val, aux, frames, correct, singles, baseline, predicted, truth, active, active_correct = values
    denominator = max(frames, 1)
    return {
        "val_pol_loss": pol / max(len(loader)*loader.dataset.unroll_len,1),
        "val_mse": val/denominator, "val_aux_mse": aux/denominator,
        "val_top1_acc": correct/max(singles,1),
        "val_active_top1_acc": active_correct/max(active,1),
        "val_active_choices": int(active), "val_uniform_seat_mse": baseline/denominator,
        "val_ece": float((calibration[1]-calibration[2]).abs().sum().cpu())/denominator,
        "val_prediction_mean": predicted/denominator, "val_outcome_mean": truth/denominator,
        "val_frames": int(frames),
    }
