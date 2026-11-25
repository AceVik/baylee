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


# A net's answer is a draw above this temperature (`netplay4::valid_raw_temperature`).
SAMPLED_TEMPERATURE = .35


def neural_decisions(batch, greedy=False):
    """The net's own decisions. By default only sampled ones (source Net, temperature
    above 0.35); `greedy` (opt-in) also counts its temperature-0 answers. A recorded
    logprob is no part of it: the action-probability contract gives one to certified
    categorical menus only, and an attack, a block or a payment is as much the net's."""
    decided = batch["step_valid"] & batch["is_decision"] & (batch["source_kind"] == 0)
    if greedy:
        return decided
    return decided & (batch["temperature"] > SAMPLED_TEMPERATURE)
