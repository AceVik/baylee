"""Whole-menu candidate pooling and versioned readout shared by execution paths."""
import torch


def score_candidate_hidden(net, hidden, mask):
    """Pool only this row's valid menu, after any candidate chunks are joined.

    Keep the legacy additive context for independent subset probabilities.
    Revision 2 adds a candidate-dependent gate; zero initialization preserves
    the legacy logits and gives categorical objectives a trainable menu context.
    Revision 3 keeps that gate on candidates that have attended to the board.
    """
    valid = mask.unsqueeze(-1).to(hidden.dtype)
    count = valid.sum(dim=1, keepdim=True).clamp(min=1.0)
    mean = (hidden * valid).sum(dim=1, keepdim=True) / count
    maximum = hidden.masked_fill(~mask.unsqueeze(-1), -1e4).amax(dim=1, keepdim=True)
    maximum = maximum * (valid.sum(dim=1, keepdim=True) > 0.0).to(hidden.dtype)
    pooled = torch.cat([mean, maximum], dim=-1)
    contextual = hidden + net.cand_ctx_proj(pooled)
    if getattr(net.cfg, "candidate_context_revision", 1) >= 2:
        contextual = contextual + hidden * torch.tanh(net.cand_context_gate(pooled))
    return net.cand_scorer[3](contextual).squeeze(-1).masked_fill(~mask, -1e9)
