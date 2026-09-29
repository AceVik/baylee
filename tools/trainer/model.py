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


# --- the policy ---------------------------------------------------------------------------

# Option heads, as `crates/baylee-train/src/policy.rs` numbers them.
H_FIXED, H_ENTITY, H_ABILITY, H_PAIR, H_ATTACK_PLAYER, H_PLAYER, H_COLOR, H_SUBTYPE, H_NUMBER, H_MODE = range(10)
N_FIXED, N_VERBS, N_ABILITY, N_PLAYERS, N_COLORS, N_SUBTYPES, N_NUMBERS, N_MODES, N_MODE_KINDS = 6, 5, 57, 4, 6, 350, 64, 32, 16
N_PROFILES = 6  # the five house profiles and "unknown"


class PolicyNet(nn.Module):
    """Scores the options a question offers, each a triple (head, a, b).

    Fixed answers, players, colours, creature types, numbers and ways to cast
    are read off the globals token; something done with an object off that
    object's token; a pair of objects by a bilinear form of their two tokens.
    The seat's house profile is an input: imitating five profiles is a
    mixture, and at play time the profile is the level asked for.
    """

    def __init__(self, cfg: Config, glob_cat: dict[str, int], glob_numeric_cols: list[int]):
        super().__init__()
        d = cfg.d
        self.trunk = Trunk(cfg, glob_cat)
        self.glob_numeric_cols = glob_numeric_cols
        self.profile = nn.Embedding(N_PROFILES, d)
        self.fixed = nn.Linear(d, N_FIXED)
        self.verb = nn.Linear(d, N_VERBS)
        self.ability = nn.Embedding(N_ABILITY, d)
        self.pair_a = nn.Linear(d, d)
        self.pair_b = nn.Linear(d, d)
        self.attack_player = nn.Embedding(N_PLAYERS, d)
        self.player = nn.Linear(d, N_PLAYERS)
        self.color = nn.Linear(d, N_COLORS)
        self.subtype = nn.Linear(d, N_SUBTYPES)
        self.number = nn.Linear(d, N_NUMBERS)
        self.mode = nn.Embedding(N_MODES, d)
        self.mode_kind = nn.Embedding(N_MODE_KINDS, d)
        self.value = nn.Sequential(nn.Linear(d, d), nn.GELU(), nn.Linear(d, 1))
        self.scale = d**-0.5

    # The names and order of `tables`' outputs: what the ONNX graph returns and
    # what the Rust player reads.
    TABLES = (
        "fixed", "player", "color", "subtype", "number", "mode", "mode_kind",
        "verb", "ability", "attack_player", "pair_a", "pair_b", "value",
    )

    def tables(self, cards, feats, mask, glob, profile) -> tuple[torch.Tensor, ...]:
        """Every score an option can be read from, in fixed shapes: per
        decision from the globals token, per entity from its token. This is
        the part exported to ONNX; `score` reads options off it, in Python
        for training and in Rust for play."""
        h = self.trunk_with_profile(cards, feats, mask, glob, profile)
        g, e = h[:, 0], h[:, 1:]
        s = self.scale
        return (
            self.fixed(g),  # (B, 6)
            self.player(g),  # (B, 4)
            self.color(g),  # (B, 6)
            self.subtype(g),  # (B, 350)
            self.number(g),  # (B, 64)
            g @ self.mode.weight.T * s,  # (B, 32)
            g @ self.mode_kind.weight.T * s,  # (B, 16)
            self.verb(e),  # (B, E, 5)
            e @ self.ability.weight.T * s,  # (B, E, 57)
            e @ self.attack_player.weight.T * s,  # (B, E, 4)
            self.pair_a(e),  # (B, E, d)
            self.pair_b(e) * s,  # (B, E, d)
            self.value(g).squeeze(-1),  # (B,)
        )

    @staticmethod
    def score(t: tuple[torch.Tensor, ...], opt_sample: torch.Tensor, opt: torch.Tensor) -> torch.Tensor:
        """One logit per option triple, read off `tables`' output."""
        fixed, player, color, subtype, number, mode, mode_kind, verb, ability, attack, pa, pb, _ = t
        head, a, b = opt[:, 0].long(), opt[:, 1].long(), opt[:, 2].long()
        s = opt_sample
        e_max = verb.shape[1] - 1
        ra, rb = a.clamp(0, e_max), b.clamp(0, e_max)

        def at(table: torch.Tensor, i: torch.Tensor) -> torch.Tensor:
            return table[s, i.clamp(0, table.shape[1] - 1)]

        def at2(table: torch.Tensor, j: torch.Tensor) -> torch.Tensor:
            return table[s, ra, j.clamp(0, table.shape[2] - 1)]

        cases = {
            H_FIXED: lambda: at(fixed, a),
            H_ENTITY: lambda: at2(verb, b),
            H_ABILITY: lambda: at2(ability, b),
            H_PAIR: lambda: (pa[s, ra] * pb[s, rb]).sum(-1),
            H_ATTACK_PLAYER: lambda: at2(attack, b),
            H_PLAYER: lambda: at(player, a),
            H_COLOR: lambda: at(color, a),
            H_SUBTYPE: lambda: at(subtype, a),
            H_NUMBER: lambda: at(number, a),
            H_MODE: lambda: at(mode, a) + at(mode_kind, b),
        }
        logit = torch.zeros(len(head), device=fixed.device, dtype=torch.float32)
        for hid, fn in cases.items():
            m = head == hid
            if m.any():
                logit = torch.where(m, fn().float(), logit)
        return logit

    def forward(self, batch: D.Batch, profile: torch.Tensor, opt_sample: torch.Tensor, opt: torch.Tensor):
        """Logits per option (flat, `opt_sample` naming each one's sample),
        and the value logit per sample."""
        t = self.tables(batch.cards, batch.feats, batch.mask, batch.glob, profile)
        return self.score(t, opt_sample, opt), t[-1]

    def trunk_with_profile(self, cards, feats, mask, glob, profile) -> torch.Tensor:
        t = self.trunk
        dense, zone, ctrl, own = D.expand_entities(feats)
        x = t.card_proj(t.card(cards)) + t.dense(dense) + t.zone(zone) + t.controller(ctrl) + t.owner(own)
        gnum = D.symlog(glob[:, self.glob_numeric_cols].to(torch.float32))
        gt = t.glob_numeric(gnum) + t.glob_token + self.profile(profile.clamp(0, N_PROFILES - 1))
        for name, col in t.glob_cat.items():
            n = D.GLOB_CATEGORICAL[name]
            gt = gt + t.glob_embed[name](glob[:, col].to(torch.int64).clamp(0, n - 1))
        tokens = torch.cat([gt.unsqueeze(1), x], dim=1)
        pad = torch.cat([torch.zeros_like(mask[:, :1]), mask], dim=1)
        return t.norm(t.encoder(tokens, src_key_padding_mask=pad))


def build_policy(meta: dict, **overrides) -> PolicyNet:
    numeric, cat = D.glob_layout(meta)
    cfg = Config(id_space=meta["ids"]["id_space"], glob_numeric=len(numeric), **overrides)
    return PolicyNet(cfg, cat, numeric)
