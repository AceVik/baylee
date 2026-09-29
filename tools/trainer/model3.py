"""The trained AI's net for encoder v3.

The same entity transformer as v2 (`model.py`), with what v3 adds:

- A card's vector is its id embedding plus its structure: the card table's
  `cardwalk` features, projected. In training the id part is replaced by a
  learned "unseen card" vector for a share of the tokens (`id_dropout`), so
  the net learns to read a card from its structure, and a card it never saw
  is not a random vector. At export both parts are folded into one table.
- Seats are tokens of their own (up to 8, in turn order from the deciding
  seat), and the deck list is one more token, attention-pooled.
- The value head is a distribution over the seats: one logit per seat
  token, a softmax over the seats present. The deciding seat's win chance is
  the sum over its side.

Token layout: 0 the globals, 1..S the seats, S+1 the deck, then entities.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass

import numpy as np
import torch
from torch import nn

import dataset as D
import dataset3 as D3

UNKNOWN_ID_OFFSET = 2  # UNKNOWN_ID = ledger rows + 2


@dataclass
class Config:
    id_space: int
    glob_numeric: int
    card_width: int
    seat_cols: list
    max_seats: int = 8
    d: int = 256
    layers: int = 4
    heads: int = 8
    ff: int = 1024
    card_dim: int = 128
    dropout: float = 0.0
    id_dropout: float = 0.4

    def to_dict(self) -> dict:
        return asdict(self)


class CardVectors(nn.Module):
    """A card's vector: id embedding + projected structure."""

    def __init__(self, cfg: Config, card_ids: np.ndarray, card_feats: np.ndarray):
        super().__init__()
        self.cfg = cfg
        self.emb = nn.Embedding(cfg.id_space, cfg.card_dim, padding_idx=0)
        self.unseen = nn.Parameter(torch.zeros(cfg.card_dim))
        table = torch.zeros(cfg.id_space, cfg.card_width)
        ids = torch.from_numpy(card_ids.astype(np.int64))
        table[ids] = D.symlog(torch.from_numpy(card_feats))
        # Saved with the model: the export folds it into the embedding.
        self.register_buffer("feat_table", table)
        self.proj = nn.Sequential(nn.Linear(cfg.card_width, cfg.card_dim), nn.GELU(), nn.Linear(cfg.card_dim, cfg.card_dim))

    def forward(self, ids: torch.Tensor) -> torch.Tensor:
        emb = self.emb(ids)
        if self.training and self.cfg.id_dropout > 0:
            drop = (torch.rand(ids.shape, device=ids.device) < self.cfg.id_dropout) & (ids > 0)
            emb = torch.where(drop.unsqueeze(-1), self.unseen.to(emb.dtype).expand_as(emb), emb)
        structure = self.proj(self.feat_table[ids])
        return emb + structure * (ids > 0).unsqueeze(-1).to(structure.dtype)

    @torch.no_grad()
    def folded(self, seen: torch.Tensor | None = None, chunk: int = 4096) -> torch.Tensor:
        """Every id's vector in one table: the id part where the id was seen
        in training (`seen`, bool per id), the unseen vector where not."""
        out = torch.empty(self.cfg.id_space, self.cfg.card_dim, device=self.emb.weight.device)
        for i in range(0, self.cfg.id_space, chunk):
            ids = torch.arange(i, min(i + chunk, self.cfg.id_space), device=out.device)
            emb = self.emb(ids)
            if seen is not None:
                emb = torch.where(seen[ids].unsqueeze(-1), emb, self.unseen.expand_as(emb))
            out[ids] = emb + self.proj(self.feat_table[ids]) * (ids > 0).unsqueeze(-1).float()
        out[0] = 0
        return out


class Trunk(nn.Module):
    def __init__(self, cfg: Config, glob_cat: dict[str, int], glob_numeric_cols: list[int], card_ids, card_feats):
        super().__init__()
        self.cfg = cfg
        d = cfg.d
        self.glob_cat, self.glob_numeric_cols = glob_cat, glob_numeric_cols
        self.cards = CardVectors(cfg, card_ids, card_feats)
        self.card_proj = nn.Linear(cfg.card_dim, d)
        self.dense = nn.Linear(D3.ENT_DENSE3, d)
        self.zone = nn.Embedding(16, d)
        self.controller = nn.Embedding(8, d)
        self.owner = nn.Embedding(8, d)
        self.seat_dense = nn.Linear(D3.SEAT_DENSE, d)
        self.seat_rel = nn.Embedding(8, d)
        self.seat_token = nn.Parameter(torch.zeros(d))
        self.deck_card = nn.Linear(cfg.card_dim, d)
        self.deck_dense = nn.Linear(2, d)
        self.deck_query = nn.Parameter(torch.randn(d) * d**-0.5)
        self.deck_token = nn.Parameter(torch.zeros(d))
        self.glob_numeric = nn.Linear(cfg.glob_numeric, d)
        self.glob_embed = nn.ModuleDict({k: nn.Embedding(n, d) for k, n in D3.GLOB3_CATEGORICAL.items()})
        self.glob_token = nn.Parameter(torch.zeros(d))
        layer = nn.TransformerEncoderLayer(d, cfg.heads, cfg.ff, dropout=cfg.dropout, batch_first=True, norm_first=True, activation="gelu")
        self.encoder = nn.TransformerEncoder(layer, cfg.layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(d)

    def forward(self, cards, feats, mask, seats, glob, deck_cards, deck_feats, deck_mask, glob_extra=None):
        """(tokens (B, 1 + S + 1 + E, d), seat padding mask (B, S))."""
        dense, zone, ctrl, own = D3.expand_entities(feats)
        x = self.card_proj(self.cards(cards)) + self.dense(dense) + self.zone(zone) + self.controller(ctrl) + self.owner(own)
        sd, rel, spad = D3.expand_seats(seats, self.cfg.seat_cols)
        st = self.seat_dense(sd) + self.seat_rel(rel) + self.seat_token
        dv = self.deck_card(self.cards(deck_cards)) + self.deck_dense(D.symlog(deck_feats.to(torch.float32)))
        score = (dv @ self.deck_query).masked_fill(deck_mask, float("-inf"))
        w = torch.nan_to_num(torch.softmax(score.float(), dim=1), nan=0.0).to(dv.dtype)
        dt = (w.unsqueeze(-1) * dv).sum(1) + self.deck_token
        gnum = D.symlog(glob[:, self.glob_numeric_cols].to(torch.float32))
        gt = self.glob_numeric(gnum) + self.glob_token
        for name, col in self.glob_cat.items():
            n = D3.GLOB3_CATEGORICAL[name]
            gt = gt + self.glob_embed[name](glob[:, col].to(torch.int64).clamp(0, n - 1))
        if glob_extra is not None:
            gt = gt + glob_extra
        tokens = torch.cat([gt.unsqueeze(1), st, dt.unsqueeze(1), x], dim=1)
        zero = torch.zeros_like(mask[:, :1])
        pad = torch.cat([zero, spad, zero, mask], dim=1)
        return self.norm(self.encoder(tokens, src_key_padding_mask=pad)), spad


# Option heads, as `crates/baylee-train/src/policy.rs` numbers them.
H_FIXED, H_ENTITY, H_ABILITY, H_PAIR, H_ATTACK_PLAYER, H_PLAYER, H_COLOR, H_SUBTYPE, H_NUMBER, H_MODE = range(10)
N_FIXED, N_VERBS, N_ABILITY, N_COLORS, N_SUBTYPES, N_NUMBERS, N_MODES, N_MODE_KINDS = 6, 5, 57, 6, 350, 64, 32, 16
N_PROFILES = 6  # the five house profiles and "unknown"


class Net(nn.Module):
    """Policy over the options a question offers, and the value as a
    distribution over the seats. Players (who to target, whom to attack) are
    read off the seat tokens."""

    TABLES = (
        "fixed", "player", "color", "subtype", "number", "mode", "mode_kind",
        "verb", "ability", "attack_player", "pair_a", "pair_b", "value",
    )

    def __init__(self, cfg: Config, glob_cat, glob_numeric_cols, card_ids, card_feats):
        super().__init__()
        d = cfg.d
        self.cfg = cfg
        self.trunk = Trunk(cfg, glob_cat, glob_numeric_cols, card_ids, card_feats)
        self.profile = nn.Embedding(N_PROFILES, d)
        self.fixed = nn.Linear(d, N_FIXED)
        self.verb = nn.Linear(d, N_VERBS)
        self.ability = nn.Embedding(N_ABILITY, d)
        self.pair_a = nn.Linear(d, d)
        self.pair_b = nn.Linear(d, d)
        self.player = nn.Linear(d, 1)
        self.attack_e = nn.Linear(d, d)
        self.attack_s = nn.Linear(d, d)
        self.color = nn.Linear(d, N_COLORS)
        self.subtype = nn.Linear(d, N_SUBTYPES)
        self.number = nn.Linear(d, N_NUMBERS)
        self.mode = nn.Embedding(N_MODES, d)
        self.mode_kind = nn.Embedding(N_MODE_KINDS, d)
        self.value = nn.Sequential(nn.Linear(d, d), nn.GELU(), nn.Linear(d, 1))
        self.scale = d**-0.5

    def tables(self, cards, feats, mask, seats, glob, deck_cards, deck_feats, deck_mask, profile):
        """Every score an option can be read from, in fixed shapes (the part
        exported to ONNX)."""
        S = self.cfg.max_seats
        h, spad = self.trunk(cards, feats, mask, seats, glob, deck_cards, deck_feats, deck_mask,
                             glob_extra=self.profile(profile.clamp(0, N_PROFILES - 1)))
        g, s_tok, e = h[:, 0], h[:, 1 : 1 + S], h[:, S + 2 :]
        sc = self.scale
        value = self.value(s_tok + g.unsqueeze(1)).squeeze(-1).float().masked_fill(spad, float("-inf"))
        return (
            self.fixed(g),  # (B, 6)
            self.player(s_tok).squeeze(-1),  # (B, S)
            self.color(g),  # (B, 6)
            self.subtype(g),  # (B, 350)
            self.number(g),  # (B, 64)
            g @ self.mode.weight.T * sc,  # (B, 32)
            g @ self.mode_kind.weight.T * sc,  # (B, 16)
            self.verb(e),  # (B, E, 5)
            e @ self.ability.weight.T * sc,  # (B, E, 57)
            self.attack_e(e) @ self.attack_s(s_tok).transpose(1, 2) * sc,  # (B, E, S)
            self.pair_a(e),  # (B, E, d)
            self.pair_b(e) * sc,  # (B, E, d)
            value,  # (B, S), -inf where no seat
        )

    @staticmethod
    def score(t, opt_sample: torch.Tensor, opt: torch.Tensor) -> torch.Tensor:
        """One logit per option triple, read off `tables`' output."""
        fixed, player, color, subtype, number, mode, mode_kind, verb, ability, attack, pa, pb, _ = t
        head, a, b = opt[:, 0].long(), opt[:, 1].long(), opt[:, 2].long()
        s = opt_sample
        e_max = verb.shape[1] - 1
        ra, rb = a.clamp(0, e_max), b.clamp(0, e_max)

        def at(table, i):
            return table[s, i.clamp(0, table.shape[1] - 1)]

        def at2(table, j):
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

    def forward(self, batch: D3.Batch, profile, opt_sample, opt):
        t = self.tables(batch.cards, batch.feats, batch.mask, batch.seats, batch.glob,
                        batch.deck_cards, batch.deck_feats, batch.deck_mask, profile)
        return self.score(t, opt_sample, opt), t[-1]


def build(meta: dict, card_ids: np.ndarray, card_feats: np.ndarray, **overrides) -> Net:
    numeric, cat = D3.glob_layout(meta)
    cfg = Config(
        id_space=meta["ids"]["id_space"],
        glob_numeric=len(numeric),
        card_width=card_feats.shape[1],
        seat_cols=list(meta["seat_cols"]),
        max_seats=meta["max_seats"],
        **overrides,
    )
    return Net(cfg, cat, numeric, card_ids, card_feats)


def side_chance(value_logits: torch.Tensor, team: torch.Tensor) -> torch.Tensor:
    """The deciding seat's side's win chance from the per-seat logits."""
    p = torch.softmax(value_logits.float(), dim=1)
    return (p * team.float()).sum(1)
