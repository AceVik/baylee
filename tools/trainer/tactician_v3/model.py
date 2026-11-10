"""The tactician net (stage S1): a set transformer over one question's units that
proposes an answer, reads its own proposal back and corrects it, T times.

Input: rows [B, U, 2d + 128], the null row first (it stands for the whole
question), and a mask [B, U]. Each row is projected from its three parts
(state, unit embedding, tail) to `width`. Pass t adds a pass embedding and the
previous pass's proposal per unit (its probability, its logit, the confidence,
and a first-pass flag) and runs the same blocks (shared weights). Per pass the
net gives one logit per unit and a confidence logit: the probability that the
pass's decoded answer is the label. The window (attack, block, target) is read
from the null row's window code, so a graph takes rows and nothing else.

A unit's probability in the proposal is its sigmoid at an attack or a block and
its softmax share over the question's units at a target, as the decoders read
the logits.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass

import torch
from torch import nn
import torch.nn.functional as F

from .records import ROW_TAIL

MAX_PASSES = 8
NEG = -1.0e4


@dataclass
class Config:
    d_model: int
    width: int = 512
    layers: int = 4
    heads: int = 8
    passes: int = 3
    ffn_mult: int = 4
    dropout: float = 0.0
    detach_passes: bool = True
    # No attention across units: every unit is scored from its own row alone.
    independent: bool = False

    def to_dict(self) -> dict:
        return asdict(self)


class Block(nn.Module):
    """Pre-norm self-attention and feed-forward, attention written out so the
    graph exports as plain MatMul/Softmax ops at any number of units."""

    def __init__(self, width: int, heads: int, ffn_mult: int, dropout: float,
                 independent: bool = False):
        super().__init__()
        if width % heads:
            raise ValueError("width must divide into heads")
        self.heads = heads
        self.norm1 = nn.LayerNorm(width)
        self.qkv = nn.Linear(width, 3 * width)
        self.out = nn.Linear(width, width)
        self.norm2 = nn.LayerNorm(width)
        self.ffn = nn.Sequential(nn.Linear(width, ffn_mult * width), nn.GELU(),
                                 nn.Linear(ffn_mult * width, width))
        self.drop = nn.Dropout(dropout)
        # Each token attends to itself only: the same weights and per-unit
        # computation, no information across units (the independent head).
        self.independent = independent

    def forward(self, x, bias):
        b = x.shape[0]
        width = x.shape[-1]
        h = self.heads
        dh = width // h
        q, k, v = self.qkv(self.norm1(x)).chunk(3, dim=-1)
        q = q.reshape(b, -1, h, dh).transpose(1, 2)
        k = k.reshape(b, -1, h, dh).transpose(1, 2)
        v = v.reshape(b, -1, h, dh).transpose(1, 2)
        if self.independent:
            y = v.transpose(1, 2).reshape(b, -1, width)
            x = x + self.drop(self.out(y))
            return x + self.drop(self.ffn(self.norm2(x)))
        scores = torch.matmul(q, k.transpose(-1, -2)) * (dh ** -0.5)
        if bias is not None:
            scores = scores + bias
        attn = torch.softmax(scores, dim=-1)
        y = torch.matmul(attn, v).transpose(1, 2).reshape(b, -1, width)
        x = x + self.drop(self.out(y))
        return x + self.drop(self.ffn(self.norm2(x)))


class TacticianV3(nn.Module):
    def __init__(self, cfg: Config):
        super().__init__()
        if not 1 <= cfg.passes <= MAX_PASSES:
            raise ValueError(f"passes must lie in 1..{MAX_PASSES}")
        self.cfg = cfg
        d, w = cfg.d_model, cfg.width
        self.state_in = nn.Sequential(nn.LayerNorm(d), nn.Linear(d, w))
        self.unit_in = nn.Sequential(nn.LayerNorm(d), nn.Linear(d, w))
        self.tail_in = nn.Linear(ROW_TAIL, w)
        self.null_emb = nn.Parameter(torch.zeros(w))
        self.pass_emb = nn.Embedding(MAX_PASSES, w)
        self.proposal_in = nn.Linear(4, w)
        self.blocks = nn.ModuleList(Block(w, cfg.heads, cfg.ffn_mult, cfg.dropout, cfg.independent)
                                    for _ in range(cfg.layers))
        self.norm = nn.LayerNorm(w)
        self.unit_head = nn.Linear(w, 1)
        self.confidence_head = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 1))

    def is_target(self, rows):
        """[B] 1.0 where the question is answered by one unit: a single target
        (window code column 2) or priority (column 12)."""
        base = 2 * self.cfg.d_model
        return (rows[:, 0, base + 2] + rows[:, 0, base + 12]).clamp(max=1.0)

    def embed(self, rows):
        d = self.cfg.d_model
        state, unit, tail = rows[..., :d], rows[..., d:2 * d], rows[..., 2 * d:]
        x = self.state_in(state) + self.unit_in(unit) + self.tail_in(tail)
        null = torch.zeros_like(x[..., :1, :]) + self.null_emb
        return torch.cat([x[:, :1] + null, x[:, 1:]], dim=1)

    @staticmethod
    def proposal(logits, unit_mask, is_target):
        """Each unit's probability under a pass's logits: sigmoid, or softmax
        share at a target."""
        masked = logits.masked_fill(~unit_mask, NEG)
        share = torch.softmax(masked, dim=-1) * unit_mask
        alone = torch.sigmoid(logits) * unit_mask
        t = is_target[:, None]
        return t * share + (1.0 - t) * alone

    def forward(self, rows, mask=None, passes: int | None = None):
        """Per pass, (unit logits [B, U - 1], confidence logit [B])."""
        passes = passes or self.cfg.passes
        if not 1 <= passes <= MAX_PASSES:
            raise ValueError(f"passes must lie in 1..{MAX_PASSES}")
        bias = None
        if mask is None:
            # One question with no padding (the exported graph).
            mask = torch.ones_like(rows[..., 0], dtype=torch.bool)
        else:
            bias = torch.zeros_like(rows[..., 0]).masked_fill(~mask, NEG)[:, None, None, :]
        unit_mask = mask[:, 1:]
        base = self.embed(rows)
        is_target = self.is_target(rows)
        count = unit_mask.sum(dim=1, keepdim=True).clamp(min=1).to(rows.dtype)
        logits = torch.zeros_like(rows[:, 1:, 0])
        confidence = torch.zeros_like(rows[:, 0, 0])
        outputs = []
        for t in range(passes):
            first = 1.0 if t == 0 else 0.0
            prob = self.proposal(logits, unit_mask, is_target) if t else torch.zeros_like(logits)
            conf = torch.sigmoid(confidence) if t else torch.zeros_like(confidence)
            unit_feat = torch.stack([prob, logits.clamp(-8.0, 8.0) / 8.0,
                                     conf[:, None].expand_as(prob),
                                     torch.full_like(prob, first)], dim=-1)
            null_feat = torch.stack([torch.zeros_like(conf), torch.zeros_like(conf), conf,
                                     torch.full_like(conf, first)], dim=-1)[:, None, :]
            feats = torch.cat([null_feat, unit_feat], dim=1)
            x = base + self.pass_emb.weight[t] + self.proposal_in(feats)
            for block in self.blocks:
                x = block(x, bias)
            x = self.norm(x)
            new_logits = self.unit_head(x[:, 1:]).squeeze(-1)
            pooled = (x[:, 1:] * unit_mask[..., None]).sum(dim=1) / count
            new_conf = self.confidence_head(torch.cat([x[:, 0], pooled], dim=-1)).squeeze(-1)
            outputs.append((new_logits, new_conf))
            if self.cfg.detach_passes:
                logits, confidence = new_logits.detach(), new_conf.detach()
            else:
                logits, confidence = new_logits, new_conf
        return outputs


class Exported(nn.Module):
    """One question at T passes: rows [U, W] -> (logits [U - 1], confidence [1])."""

    def __init__(self, net: TacticianV3, passes: int):
        super().__init__()
        self.net = net
        self.passes = passes

    def forward(self, rows):
        logits, confidence = self.net(rows[None], None, self.passes)[-1]
        return logits[0], torch.sigmoid(confidence)


def decode(logits, unit_mask, groups, window):
    """The answer the Rust decoders give at temperature 0, as masks [B, U - 1]:
    at a target the best unit; at an attack or a block, per group (attacker,
    blocker) its best unit when its logit is above zero or the group's
    probabilities sum past one half with that logit above -0.65. The
    attack's required/alone constraints are not read here."""
    masked = logits.masked_fill(~unit_mask, NEG)
    b, n = logits.shape
    index = torch.arange(n, device=logits.device)[None, :].expand(b, n)
    best_target = F.one_hot(masked.argmax(dim=-1), n).to(logits.dtype) * unit_mask
    g = groups.clamp(min=0)
    size = int(g.max()) + 1 if n else 1
    gmax = torch.full((b, size), NEG, dtype=logits.dtype, device=logits.device)
    gmax = gmax.scatter_reduce(1, g, masked, reduce="amax", include_self=True)
    is_max = (masked == gmax.gather(1, g)) & unit_mask
    first = torch.full((b, size), n, dtype=torch.long, device=logits.device)
    first = first.scatter_reduce(1, g, torch.where(is_max, index, torch.full_like(index, n)),
                                 reduce="amin", include_self=True)
    best = is_max & (index == first.gather(1, g))
    psum = torch.zeros((b, size), dtype=logits.dtype, device=logits.device)
    psum = psum.scatter_add(1, g, torch.sigmoid(logits) * unit_mask)
    best_l = gmax.gather(1, g)
    take = best & ((best_l > 0) | ((psum.gather(1, g) > 0.5) & (best_l > -0.65)))
    target = ((window == 2) | (window == 3))[:, None]
    return torch.where(target, best_target, take.to(logits.dtype))


def exact(answer, labels, unit_mask):
    """[B] whether the answer equals the label on every real unit."""
    return ((answer != labels) & unit_mask).sum(dim=1) == 0
