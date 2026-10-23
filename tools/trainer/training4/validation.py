"""Held-out recurrent trajectory validation."""
import torch
import numpy as np
from torch.utils.data import DataLoader
import model4 as M
from .loss import compute_step_loss


@torch.no_grad()
def evaluate(
    net: M.PlayerNet4,
    loader: DataLoader,
    device: torch.device,
    thinking_steps: int,
) -> dict[str, float]:
    net.eval()
    net.clear_structure()
    net.prepare_structure()
    tot_pol = 0.0
    tot_val = 0.0
    tot_aux = 0.0
    tot_steps = 0
    corr = 0
    tot_single = 0
    active_correct = active_count = 0
    baseline_squared_error = prediction_sum = outcome_sum = 0.0
    calibration_count = np.zeros(10)
    calibration_prediction = np.zeros(10)
    calibration_outcome = np.zeros(10)
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
            k_tensor = torch.where(
                batch["is_decision"][:, t].to(device),
                torch.full((B,), thinking_steps, dtype=torch.long, device=device),
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
                    batch["cand_dsl"][:, t].to(device),
                    batch["entity_rules"][:, t].to(device),
                    batch["cand_rules"][:, t].to(device),
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
            count = int(valid_t.sum())
            tot_val += st["val_loss"] * count
            tot_aux += st["aux_loss"] * count
            tot_steps += count
            corr += st["correct_single"]
            tot_single += st["total_single"]
            truth = batch["outcome"][:, t].to(device)
            sf = batch["seat_feat"][:, t].to(device)
            alive = (sf[:, :, M.SEAT_FEAT_PRESENT] > .5) & (sf[:, :, M.SEAT_FEAT_LOST] < .5)
            allied = (sf[:, :, M.SEAT_FEAT_SELF] > .5) | (sf[:, :, M.SEAT_FEAT_TEAMMATE] > .5)
            # Explicitly a uniform surviving-seat baseline; does not assume
            # two sides or a 0.5 prior in multiplayer games.
            prior = (alive & allied).sum(-1) / alive.sum(-1).clamp_min(1)
            baseline_squared_error += float(((prior - truth).square() * valid_t).sum())
            predictions = val_p.float()[valid_t].cpu().numpy()
            targets = truth[valid_t].cpu().numpy()
            prediction_sum += float(predictions.sum())
            outcome_sum += float(targets.sum())
            bins = np.minimum((predictions * 10).astype(int), 9)
            np.add.at(calibration_count, bins, 1)
            np.add.at(calibration_prediction, bins, predictions)
            np.add.at(calibration_outcome, bins, targets)
            single = valid_t & batch["is_decision"][:, t].to(device) & (batch["head"][:, t].to(device) == 0)
            target_idx = batch["single_idx"][:, t].to(device)
            features = batch["cand_feat"][:, t].to(device)
            pass_target = features.gather(1, target_idx[:, None, None].expand(-1, 1, M.CAND_FEAT_WIDTH))[:, 0, 0] > .5
            active = single & ~pass_target
            active_count += int(active.sum())
            active_correct += int(((cand_l.argmax(-1) == target_idx) & active).sum())

    denom = max(tot_steps, 1)
    return {
        "val_pol_loss": tot_pol / max(len(loader) * loader.dataset.unroll_len, 1),
        "val_mse": tot_val / denom,
        "val_aux_mse": tot_aux / denom,
        "val_top1_acc": corr / max(tot_single, 1),
        "val_active_top1_acc": active_correct / max(active_count, 1),
        "val_active_choices": active_count,
        "val_uniform_seat_mse": baseline_squared_error / denom,
        "val_ece": float(np.abs(calibration_prediction - calibration_outcome).sum() / denom),
        "val_prediction_mean": prediction_sum / denom,
        "val_outcome_mean": outcome_sum / denom,
        "val_frames": tot_steps,
    }
