"""Thinking revision 2: deliberation over the offered answers.

After the heads have scored a decision once, a small block with shared weights
runs `S` steps over one decision token and the `M` best candidates by those
first logits. Each step reads its previous proposal back (probabilities, the
gap to the best answer, how it moved, the predicted consequence of each answer
and its own last confidence), compares the answers with one another, looks at
the board again, and emits a correction to every logit, a consequence estimate
per answer and a confidence that the current top answer is right.

The correction's output starts at zero: a net with the block attached computes
its source's logits exactly until the block is trained. Every shape inside the
block is fixed (`1 + M` tokens, a constant step count per exported graph); the
only dynamic axis is the board's key count, which the trunk has already.

Design and measurements: `docs/proposals/baylee-ai-thinking.md`.
"""
from __future__ import annotations

import math

import torch
from torch import nn

from model4.layers import DynamicMultiHeadAttention

MAX_STEPS = 8
PROPOSAL_FEATURES = 7
# FP16-safe stand-in for "not offered" inside the block's own arithmetic.
NEG = -1.0e4
CONTRACT = "deliberation_v1"


class Deliberation(nn.Module):
    def __init__(self, d_model: int, width: int, heads: int, steps: int, candidates: int,
                 feedback: bool = True):
        super().__init__()
        if not 1 <= steps <= MAX_STEPS:
            raise ValueError(f"deliberation steps must lie in 1..{MAX_STEPS}")
        if candidates < 2 or width % heads:
            raise ValueError("deliberation needs two or more candidates and a width its heads divide")
        self.steps, self.candidates, self.feedback = steps, candidates, feedback
        self.state_in = nn.Sequential(nn.LayerNorm(d_model), nn.Linear(d_model, width))
        self.cand_in = nn.Sequential(nn.LayerNorm(d_model), nn.Linear(d_model, width))
        self.board_in = nn.Sequential(nn.LayerNorm(d_model), nn.Linear(d_model, width))
        self.decision_emb = nn.Parameter(torch.zeros(width))
        self.step_emb = nn.Embedding(MAX_STEPS, width)
        self.proposal_in = nn.Linear(PROPOSAL_FEATURES, width)
        self.decision_in = nn.Linear(PROPOSAL_FEATURES, width)
        self.norm_self = nn.LayerNorm(width)
        self.self_attn = DynamicMultiHeadAttention(width, heads)
        self.norm_cross = nn.LayerNorm(width)
        self.cross_attn = DynamicMultiHeadAttention(width, heads)
        self.norm_ff = nn.LayerNorm(width)
        self.ff = nn.Sequential(nn.Linear(width, 4 * width), nn.GELU(), nn.Linear(4 * width, width))
        self.norm_out = nn.LayerNorm(width)
        self.correction = nn.Linear(width, 1)
        self.consequence = nn.Linear(width, 1)
        self.confidence = nn.Linear(width, 1)
        nn.init.zeros_(self.correction.weight)
        nn.init.zeros_(self.correction.bias)
        # Training reads the per-step trace of the latest call when asked to.
        self.collect = False
        self.trace = None
        # Probability that the returned top answer is right, of the latest call.
        self.last_confidence = None
        # A measurement may fix the eager depth (`thinking_probe5.py`).
        self.steps_override = None

    def _features(self, current, previous_share, consequence, confidence, chosen, first):
        """What a step reads of the proposal so far; detached by the caller."""
        offered = current.masked_fill(~chosen, NEG)
        share = torch.softmax(offered, dim=-1) * chosen
        best = offered.amax(dim=-1, keepdim=True)
        flag = torch.full_like(share, 1.0 if first else 0.0)
        per_candidate = torch.stack((
            share,
            torch.sigmoid(current) * chosen,
            (offered - best).clamp(min=-10.0) / 10.0,
            ((offered == best) & chosen).to(share.dtype),
            share - previous_share,
            torch.sigmoid(consequence) * chosen * (0.0 if first else 1.0),
            flag,
        ), dim=-1)
        top = share.topk(2, dim=-1).values
        count = chosen.to(share.dtype).sum(dim=-1)
        entropy = -(share * torch.log(share.clamp(min=1e-6))).sum(dim=-1) / math.log(self.candidates)
        decision = torch.stack((
            top[:, 0],
            top[:, 0] - top[:, 1],
            entropy,
            confidence * (0.0 if first else 1.0),
            flag[:, 0],
            count / self.candidates,
            torch.zeros_like(count),
        ), dim=-1)
        return share, per_candidate, decision

    def forward(self, hidden, logits, mask, tokens, pad, depth=None, limit=None):
        """Corrected logits `[B, C]`.

        `hidden` `[B, C, d]` are the candidates as their last linear reads
        them, `logits` their first scores, `tokens` `[B, 9 + E, d]` the
        thought-through summary slots then every entity, `pad` its padding.
        `depth` `[B]` gives each row's step count (a row at zero is returned
        unchanged); `None` runs the configured count on every row, as training
        does. `limit` fixes the loop length when a graph is traced.
        """
        b = logits.shape[0]
        m = self.candidates
        if limit is None:
            if self.steps_override is not None:
                limit, depth = self.steps_override, None
            elif depth is None or self.training:
                limit = self.steps
                depth = None if depth is None else depth.clamp(max=1) * self.steps
            else:
                limit = max(0, min(int(depth.max()), MAX_STEPS))
        # The M best offered answers; a short menu leaves masked slots that
        # point at candidate zero and add nothing there.
        offered = torch.where(mask, logits.clamp(min=NEG), torch.full_like(logits, 2.0 * NEG))
        padded = torch.cat((offered, torch.full((b, m), 2.0 * NEG, dtype=offered.dtype, device=offered.device)), dim=1)
        first_scores, index = padded.topk(m, dim=1)
        chosen = first_scores > 1.5 * NEG
        index = torch.where(chosen, index, torch.zeros_like(index))
        gather = index.unsqueeze(-1).expand(-1, -1, hidden.shape[-1])
        first_scores = torch.where(chosen, first_scores, torch.zeros_like(first_scores))

        x = torch.cat(((self.state_in(tokens[:, 0]) + self.decision_emb).unsqueeze(1),
                       self.cand_in(torch.gather(hidden, 1, gather))), dim=1)
        board = self.board_in(tokens)
        board_kv = self.cross_attn.project_kv(board, board)
        token_pad = torch.cat((torch.zeros_like(chosen[:, :1]), ~chosen), dim=1)

        current = first_scores
        share = torch.zeros_like(current)
        consequence = torch.zeros_like(current)
        confidence = torch.ones_like(current[:, 0])
        confidence_logit = torch.zeros_like(confidence)
        steps_logits, steps_confidence, steps_consequence = [], [], []
        for s in range(limit):
            share_now, per_candidate, decision = self._features(
                current.detach(), share, consequence.detach(), confidence.detach(), chosen, s == 0)
            if not self.feedback:
                per_candidate, decision = torch.zeros_like(per_candidate), torch.zeros_like(decision)
            y = x + self.step_emb.weight[s] + torch.cat(
                (self.decision_in(decision).unsqueeze(1), self.proposal_in(per_candidate)), dim=1)
            normed = self.norm_self(y)
            y = y + self.self_attn(normed, normed, normed, key_padding_mask=token_pad)
            y = y + self.cross_attn(self.norm_cross(y), board, board, key_padding_mask=pad,
                                    projected_kv=board_kv)
            y = y + self.ff(self.norm_ff(y))
            out = self.norm_out(y)
            step_correction = self.correction(out[:, 1:]).squeeze(-1) * chosen
            step_consequence = self.consequence(out[:, 1:]).squeeze(-1)
            step_confidence = self.confidence(out[:, 0]).squeeze(-1)
            if depth is None:
                x, current = y, current + step_correction
                consequence, confidence_logit = step_consequence, step_confidence
            else:
                active = (depth > s).to(current.dtype)
                x = x + active.view(b, 1, 1) * (y - x)
                current = current + active.view(b, 1) * step_correction
                consequence = consequence + active.view(b, 1) * (step_consequence - consequence)
                confidence_logit = confidence_logit + active * (step_confidence - confidence_logit)
            confidence = torch.sigmoid(confidence_logit)
            if depth is not None:
                # A row that has not thought states no doubt.
                confidence = torch.where(depth > 0, confidence, torch.ones_like(confidence))
            share = share_now
            if self.collect:
                steps_logits.append(self._spread(logits, index, current - first_scores, mask))
                steps_confidence.append(confidence_logit)
                steps_consequence.append(consequence)
        self.last_confidence = confidence
        result = self._spread(logits, index, current - first_scores, mask)
        if self.collect:
            self.trace = {
                "logits": torch.stack([logits, *steps_logits]) if steps_logits else logits.unsqueeze(0),
                "confidence": torch.stack(steps_confidence) if steps_confidence else logits.new_zeros((0, b)),
                "consequence": torch.stack(steps_consequence) if steps_consequence else logits.new_zeros((0, b, m)),
                "index": index, "chosen": chosen,
            }
        return result

    @staticmethod
    def _spread(logits, index, correction, mask):
        """The first logits plus each chosen candidate's summed correction."""
        added = torch.zeros_like(logits).scatter_add(1, index, correction.to(logits.dtype))
        return (logits + added).masked_fill(~mask, -1e9)


def warmstart_thinking(state_dict, net):
    """Explicit weights-only transfer onto a net that adds the deliberation block.

    The block's fresh weights keep their zero correction output, so the
    source's logits are reproduced exactly until the first update. The caller
    starts a fresh optimizer; never use this to repair or resume a checkpoint.
    """
    if getattr(net, "deliberation", None) is None:
        raise ValueError("Thinking warmstart requires thinking revision 2")
    expected = net.state_dict()
    added = {key for key in expected if key.startswith("deliberation.")}
    if set(state_dict) != set(expected) - added:
        raise ValueError("Thinking warmstart has missing or unexpected source tensors")
    for key, value in state_dict.items():
        if value.shape != expected[key].shape or value.dtype != expected[key].dtype:
            raise ValueError(f"Thinking warmstart tensor differs: {key}")
    return {**state_dict, **{key: expected[key] for key in added}}
