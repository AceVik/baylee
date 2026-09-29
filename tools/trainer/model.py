"""The trained AI's net: a shared card-knowledge trunk and its heads.

An entity transformer. Every object the seat sees is one token: its card's
embedding (by `CardIndex` alone, never text) plus its state; the globals row
is one more token that reads the whole table. The value head scores that
token; a policy head over option tokens joins it in stage 5.

Shapes are fixed (padded entity sets), so the graph exports to ONNX for the
runtimes a player's machine has (CPU, DirectML, CoreML, an NPU).
"""

from __future__ import annotations

from dataclasses import dataclass, asdict

import torch
from torch import nn

import dataset as D


@dataclass
class Config:
    id_space: int
    glob_numeric: int
    d: int = 256
    layers: int = 4
    heads: int = 8
    ff: int = 1024
    card_dim: int = 128
    dropout: float = 0.0

    def to_dict(self) -> dict:
        return asdict(self)


class Trunk(nn.Module):
    def __init__(self, cfg: Config, glob_cat: dict[str, int]):
        super().__init__()
        self.cfg = cfg
        self.glob_cat = glob_cat
        self.card = nn.Embedding(cfg.id_space, cfg.card_dim, padding_idx=0)
        self.card_proj = nn.Linear(cfg.card_dim, cfg.d)
        self.dense = nn.Linear(D.ENT_DENSE, cfg.d)
        self.zone = nn.Embedding(16, cfg.d)
        self.controller = nn.Embedding(8, cfg.d)
        self.owner = nn.Embedding(8, cfg.d)
        self.glob_numeric = nn.Linear(cfg.glob_numeric, cfg.d)
        self.glob_embed = nn.ModuleDict({k: nn.Embedding(n, cfg.d) for k, n in D.GLOB_CATEGORICAL.items()})
        self.glob_token = nn.Parameter(torch.zeros(cfg.d))
        layer = nn.TransformerEncoderLayer(
            cfg.d, cfg.heads, cfg.ff, dropout=cfg.dropout, batch_first=True, norm_first=True, activation="gelu"
        )
        self.encoder = nn.TransformerEncoder(layer, cfg.layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(cfg.d)

    def forward(self, batch: D.Batch, glob_numeric_cols: list[int]) -> torch.Tensor:
        """(B, 1 + E, d): the globals token first, then the entities."""
        dense, zone, ctrl, own = D.expand_entities(batch.feats)
        x = (
            self.card_proj(self.card(batch.cards))
            + self.dense(dense)
            + self.zone(zone)
            + self.controller(ctrl)
            + self.owner(own)
        )
        g = batch.glob
        gnum = D.symlog(g[:, glob_numeric_cols].to(torch.float32))
        gt = self.glob_numeric(gnum) + self.glob_token
        for name, col in self.glob_cat.items():
            n = D.GLOB_CATEGORICAL[name]
            gt = gt + self.glob_embed[name](g[:, col].to(torch.int64).clamp(0, n - 1))
        tokens = torch.cat([gt.unsqueeze(1), x], dim=1)
        pad = torch.cat([torch.zeros_like(batch.mask[:, :1]), batch.mask], dim=1)
        return self.norm(self.encoder(tokens, src_key_padding_mask=pad))


class ValueNet(nn.Module):
    """Win chance for the deciding seat: a logit from the globals token."""

    def __init__(self, cfg: Config, glob_cat: dict[str, int], glob_numeric_cols: list[int]):
        super().__init__()
        self.trunk = Trunk(cfg, glob_cat)
        self.glob_numeric_cols = glob_numeric_cols
        self.value = nn.Sequential(nn.Linear(cfg.d, cfg.d), nn.GELU(), nn.Linear(cfg.d, 1))

    def forward(self, batch: D.Batch) -> torch.Tensor:
        h = self.trunk(batch, self.glob_numeric_cols)
        return self.value(h[:, 0]).squeeze(-1)


def build(meta: dict, **overrides) -> ValueNet:
    numeric, cat = D.glob_layout(meta)
    cfg = Config(id_space=meta["ids"]["id_space"], glob_numeric=len(numeric), **overrides)
    return ValueNet(cfg, cat, numeric)
