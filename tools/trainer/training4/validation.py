"""Held-out recurrent trajectory validation."""
import torch
from legacy_evaluation import native_metrics
from torch.utils.data import DataLoader
import model4 as M
from .loss import compute_step_loss
from .objective import choice_statistics


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
    state = dict(calibration=None, totals=None)
    choices = torch.zeros(3, dtype=torch.float64, device=device)
    use_bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported()
    h: torch.Tensor | None = None

    for batch in loader:
        B, U = batch["step_valid"].shape
        predictions, targets, statistics = [], [], []
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
            truth = batch["outcome"][:, t].to(device)
            sf = batch["seat_feat"][:, t].to(device)
            alive = (sf[:, :, M.SEAT_FEAT_PRESENT] > .5) & (sf[:, :, M.SEAT_FEAT_LOST] < .5)
            allied = (sf[:, :, M.SEAT_FEAT_SELF] > .5) | (sf[:, :, M.SEAT_FEAT_TEAMMATE] > .5)
            # Explicitly a uniform surviving-seat baseline; does not assume
            # two sides or a 0.5 prior in multiplayer games.
            prior = (alive & allied).sum(-1) / alive.sum(-1).clamp_min(1)
            baseline_error = ((prior - truth).square() * valid_t).sum()
            predictions.append(val_p.float()[valid_t].cpu())
            targets.append(truth[valid_t].cpu())
            single = valid_t & batch["is_decision"][:, t].to(device) & (batch["head"][:, t].to(device) == 0)
            target_idx = torch.where(single, batch["single_idx"][:, t].to(device), 0)
            choices += choice_statistics(cand_l, batch["cand_mask"][:, t, :cand_l.shape[1]].to(device),
                                         single, target_idx, batch["loss_weight"][:, t].to(device)).double()
            features = batch["cand_feat"][:, t].to(device)
            pass_target = features.gather(1, target_idx[:, None, None].expand(-1, 1, M.CAND_FEAT_WIDTH))[:, 0, 0] > .5
            active = single & ~pass_target
            statistics.append([st["pol_loss"], st["val_loss"], st["aux_loss"],
                int(valid_t.sum()), st["correct_single"], st["total_single"], float(baseline_error),
                int(active.sum()), int(((cand_l.argmax(-1) == target_idx) & active).sum())])
        state = native_metrics("reference-calibration", state=state["calibration"],
            totals=state["totals"], prediction=torch.cat(predictions).tolist(),
            truth=torch.cat(targets).tolist(), steps=statistics)
    # Tensor counts retain integer JSON types at the native report boundary.
    totals = state["totals"] or [0.] * 9
    choice = choices.cpu().tolist()
    return native_metrics("reference-validation", calibration=state["calibration"] or [0.] * 22,
        totals=totals, choice=choice, batches=len(loader), unroll=loader.dataset.unroll_len,
        counts=dict(frames=int(totals[3]), active=int(totals[7]), choices=int(choice[2])))
