"""The deployed model's exact seat-mask and side-probability calculation."""
import torch
import model4 as M


def seat_masks(seats):
    seat0 = torch.arange(M.MAX_SEATS, device=seats.device)[None, :] == 0
    active = (seats[..., M.SEAT_FEAT_PRESENT] > .5) & ~(seats[..., M.SEAT_FEAT_LOST] > .5)
    active = active | (~active.any(-1, keepdim=True) & seat0)
    allied = ((seats[..., M.SEAT_FEAT_SELF] > .5)
              | (seats[..., M.SEAT_FEAT_TEAMMATE] > .5) | seat0)
    return active, allied


def predict(head, features, active, allied):
    logits = head(features).squeeze(-1).masked_fill(~active, -1e9)
    return (torch.softmax(logits, -1) * allied).sum(-1).clamp(0, 1)


def neural_decisions(batch):
    return (batch["step_valid"] & batch["is_decision"] & (batch["source_kind"] == 0)
            & batch["has_logprob"] & (batch["temperature"] > 1e-4))
