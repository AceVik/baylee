"""Opt-in R2D2-style burn-in for the TBPTT learner (`--passive-grad-window W`).

A frame is differentiable when a valid decision of its own lane follows within
W frames (the decision frame itself included). Every other frame still encodes
its board and advances memory in strict order, but without gradient: its board
is encoded under no_grad and its memory update is detached, so the recurrent
chain entering the next differentiable window is cut (burn-in). Value and
auxiliary readouts of those frames still train their heads on the detached
state; their loss terms are unchanged.

With W covering a whole lane whose last valid frame is a decision, every frame
is differentiable and the step equals the default path: boards are encoded in
the same chunks, rows and order.
"""
from __future__ import annotations

import torch

import model4 as M


def window_mask(batch, window):
    """CPU [B, U] bool: valid frames with a valid decision of their lane in [t, t + window]."""
    if window < 0:
        raise ValueError("Gradient window must be nonnegative")
    valid = batch["step_valid"].bool()
    decisions = (batch["is_decision"].bool() & valid).to(torch.int32)
    unroll = decisions.shape[1]
    # ahead[:, t] counts decisions in [t, t + window]; a reversed cumulative sum.
    padded = torch.nn.functional.pad(decisions, (0, window + 1))
    suffix = padded.flip(1).cumsum(1).flip(1)
    ahead = suffix[:, :unroll] - suffix[:, window + 1:window + 1 + unroll]
    return (ahead > 0) & valid


class GradWindow:
    """Host plan plus device copy of one batch's differentiable frames."""

    def __init__(self, mask, device):
        self.host = mask
        self.device_mask = mask.to(device, non_blocking=True)
        self.any_t = mask.any(0).tolist()
        self.all_t = mask.all(0).tolist()

    @property
    def share(self):
        return float(self.host.float().mean())

    def observe(self, net, t, board, hidden, thinking, device, bf16):
        if not self.any_t[t]:
            with torch.no_grad(), torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                return net.observe_memory(board, hidden, thinking)
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            observed = net.observe_memory(board, hidden, thinking)
        if self.all_t[t]:
            return observed
        return torch.where(self.device_mask[:, t, None, None], observed, observed.detach())


def _gather_rows(batch, rows, device):
    b, unroll = batch["step_valid"].shape
    inputs = []
    for name in (*M.INPUT_NAMES[:8], "entity_rules"):
        tensor = batch[name]
        flat = tensor.reshape(b * unroll, *tensor.shape[2:])
        inputs.append(flat.index_select(0, rows.to(flat.device)).to(device, non_blocking=True))
    return inputs


def encode_boards_windowed(net, batch, chunk, device, window, nograd_scale=4):
    """Like `encode_boards`, but only `window` rows carry autograd history.

    Differentiable rows are encoded per time chunk exactly as `encode_boards`
    groups them (lane-major within the chunk). The others are encoded under
    no_grad in larger calls, since they keep no activations.
    """
    chunk = max(1, chunk)
    b, unroll = batch["step_valid"].shape
    budget = getattr(net, "board_token_budget", 0)
    tokens = M.RECURRENT_SLOTS + batch["entity_feat"].shape[2]
    if budget:
        chunk = max(1, min(chunk, budget // (b * tokens)))
    mask = window.host
    order = torch.arange(b * unroll).reshape(b, unroll)
    pieces = []
    free = order[~mask]
    if free.numel():
        rows_per_call = max(1, (budget * nograd_scale) // tokens) if budget else free.numel()
        with torch.no_grad():
            for start in range(0, free.numel(), rows_per_call):
                rows = free[start:start + rows_per_call]
                pieces.append((rows, *net.encode_board(*_gather_rows(batch, rows, device))))
    for start in range(0, unroll, chunk):
        end = min(start + chunk, unroll)
        rows = order[:, start:end][mask[:, start:end]]
        if rows.numel():
            pieces.append((rows, *net.encode_board(*_gather_rows(batch, rows, device))))
    width = pieces[0][1].shape[1:]
    board = pieces[0][1].new_zeros((b * unroll, *width))
    pad = pieces[0][2].new_zeros((b * unroll, pieces[0][2].shape[1]))
    for rows, encoded, padding in pieces:
        index = rows.to(device, non_blocking=True)
        board = board.index_copy(0, index, encoded)
        pad = pad.index_copy(0, index, padding)
    board = board.reshape(b, unroll, *width)
    pad = pad.reshape(b, unroll, -1)
    return [(board[:, t], pad[:, t]) for t in range(unroll)]
