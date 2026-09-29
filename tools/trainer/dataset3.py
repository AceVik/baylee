"""Reads a v3 dataset (`convert3`), and turns batches into tensors.

v3 adds to v2's columns (see `crates/baylee-train/src/bin/convert3.rs`):
seat rows (`seat.i16`, `MAX_SEATS` per sample), the deciding seat's deck
list (`deck_*`), every seat's list for the critic (`omni_deck_*`), masks of
who won, who was still in and who is on the deciding seat's side (bit `r` is
the seat `r` places on in turn order), and the card table at the root. As in
`dataset.py`, everything is loaded into RAM once and a batch is gathered on
the CPU as raw integers; the model expands them on the GPU.
"""

from __future__ import annotations

import json
import queue
import threading
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch

import dataset as D

ENCODER_VERSION = 3


@dataclass
class Dataset:
    meta: dict
    ent_card: np.ndarray  # (E_total,) int32
    ent_feat: np.ndarray  # (E_total, ENT_COLS) int16
    ent_off: np.ndarray  # (N + 1,) int64
    seat: np.ndarray  # (N, MAX_SEATS, SEAT_COLS) int16
    glob: np.ndarray  # (N, GLOB_COLS) int16
    info: np.ndarray  # (N, META_COLS) int32
    deck_card: np.ndarray  # (K_total,) int32
    deck_feat: np.ndarray  # (K_total, DECK_COLS) int16
    deck_off: np.ndarray  # (N + 1,) int64
    opt: np.ndarray  # (O_total, 3) int16
    opt_off: np.ndarray  # (N + 1,) int64
    card_ids: np.ndarray  # (C,) int32
    card_feats: np.ndarray  # (C, WIDTH) float32
    games: list[dict]

    @property
    def n(self) -> int:
        return len(self.glob)

    def col(self, name: str) -> np.ndarray:
        return self.info[:, self.meta["meta_cols"].index(name)]


def load(path: str | Path) -> Dataset:
    path = Path(path)
    meta = json.loads((path / "dataset.json").read_text())
    if meta["encoder_version"] != ENCODER_VERSION:
        raise ValueError(f"encoder v{meta['encoder_version']}, this reader knows v{ENCODER_VERSION}")
    n_ent, n_seat, n_glob = len(meta["ent_cols"]), len(meta["seat_cols"]), len(meta["glob_cols"])
    n_deck, n_meta, max_seats = len(meta["deck_cols"]), len(meta["meta_cols"]), meta["max_seats"]
    shards = meta["shards"]
    samples = sum(s["samples"] for s in shards)
    entities = sum(s["entities"] for s in shards)
    decks = sum(s["decks"] for s in shards)
    opts = sum(s["opts"] for s in shards)
    ds = Dataset(
        meta=meta,
        ent_card=np.empty(entities, dtype=np.int32),
        ent_feat=np.empty((entities, n_ent), dtype=np.int16),
        ent_off=np.empty(samples + 1, dtype=np.int64),
        seat=np.empty((samples, max_seats, n_seat), dtype=np.int16),
        glob=np.empty((samples, n_glob), dtype=np.int16),
        info=np.empty((samples, n_meta), dtype=np.int32),
        deck_card=np.empty(decks, dtype=np.int32),
        deck_feat=np.empty((decks, n_deck), dtype=np.int16),
        deck_off=np.empty(samples + 1, dtype=np.int64),
        opt=np.empty((opts, 3), dtype=np.int16),
        opt_off=np.empty(samples + 1, dtype=np.int64),
        card_ids=np.fromfile(path / "card_ids.i32", dtype="<i4"),
        card_feats=np.empty(0, dtype=np.float32),
        games=[json.loads(l) for l in (path / "games.jsonl").read_text().splitlines()],
    )
    ds.card_feats = np.fromfile(path / "card_feats.f32", dtype="<f4").reshape(len(ds.card_ids), -1)
    s0 = e0 = k0 = o0 = 0
    for shard in shards:
        d = path / shard["dir"]
        ns, ne, nk, no = shard["samples"], shard["entities"], shard["decks"], shard["opts"]
        if ns == 0:
            continue

        def mm(name, dtype):
            return np.memmap(d / name, dtype=dtype, mode="r")

        ds.ent_card[e0 : e0 + ne] = mm("ent_card.i32", "<i4")
        ds.ent_feat[e0 : e0 + ne] = mm("ent_feat.i16", "<i2").reshape(-1, n_ent)
        ds.ent_off[s0 : s0 + ns] = mm("ent_off.i64", "<i8")[:-1] + e0
        ds.seat[s0 : s0 + ns] = mm("seat.i16", "<i2").reshape(-1, max_seats, n_seat)
        ds.glob[s0 : s0 + ns] = mm("glob.i16", "<i2").reshape(-1, n_glob)
        ds.info[s0 : s0 + ns] = mm("meta.i32", "<i4").reshape(-1, n_meta)
        ds.deck_card[k0 : k0 + nk] = mm("deck_card.i32", "<i4")
        ds.deck_feat[k0 : k0 + nk] = mm("deck_feat.i16", "<i2").reshape(-1, n_deck)
        ds.deck_off[s0 : s0 + ns] = mm("deck_off.i64", "<i8")[:-1] + k0
        ds.opt[o0 : o0 + no] = mm("opt.i16", "<i2").reshape(-1, 3)
        ds.opt_off[s0 : s0 + ns] = mm("opt_off.i64", "<i8")[:-1] + o0
        s0, e0, k0, o0 = s0 + ns, e0 + ne, k0 + nk, o0 + no
    ds.ent_off[s0], ds.deck_off[s0], ds.opt_off[s0] = e0, k0, o0
    assert (s0, e0, k0, o0) == (samples, entities, decks, opts)
    return ds


def split_by_game(ds: Dataset, held_out_every: int = 10) -> tuple[np.ndarray, np.ndarray]:
    """Train and held-out sample indices, a game wholly on one side."""
    held = (ds.col("game") % held_out_every) == 0
    return np.nonzero(~held)[0], np.nonzero(held)[0]


def value_target(winners: np.ndarray, alive: np.ndarray, seats: int) -> np.ndarray:
    """Per sample, a distribution over the `seats` seat rows: the winners
    share it (a team's seats alike); in a draw the seats still in do."""
    bits = (winners[:, None] >> np.arange(seats)[None, :]) & 1
    alive_bits = (alive[:, None] >> np.arange(seats)[None, :]) & 1
    t = np.where((bits.sum(1) > 0)[:, None], bits, alive_bits).astype(np.float32)
    return t / np.maximum(t.sum(1, keepdims=True), 1.0)


@dataclass
class Batch:
    cards: torch.Tensor  # (B, E) int64
    feats: torch.Tensor  # (B, E, ENT_COLS) int16
    mask: torch.Tensor  # (B, E) bool, True = padding
    seats: torch.Tensor  # (B, S, SEAT_COLS) int16
    glob: torch.Tensor  # (B, GLOB_COLS) int16
    deck_cards: torch.Tensor  # (B, K) int64
    deck_feats: torch.Tensor  # (B, K, DECK_COLS) int16
    deck_mask: torch.Tensor  # (B, K) bool, True = padding
    value_target: torch.Tensor  # (B, S) float
    team: torch.Tensor  # (B, S) bool: the deciding seat's side
    idx: np.ndarray


def gather(ds: Dataset, idx: np.ndarray, entities: int, deck: int) -> dict[str, np.ndarray]:
    def ragged(off, cards, feats, width):
        start = off[idx]
        count = np.minimum(off[idx + 1] - start, width)
        ar = np.arange(width)
        rows = start[:, None] + ar[None, :]
        live = ar[None, :] < count[:, None]
        rows = np.where(live, rows, 0)
        return (
            np.where(live, cards[rows], 0).astype(np.int64),
            np.where(live[..., None], feats[rows], 0).astype(np.int16),
            ~live,
        )

    cards, feats, mask = ragged(ds.ent_off, ds.ent_card, ds.ent_feat, entities)
    dcards, dfeats, dmask = ragged(ds.deck_off, ds.deck_card, ds.deck_feat, deck)
    seats = ds.seat.shape[1]
    team = ((ds.col("team_rel")[idx][:, None] >> np.arange(seats)[None, :]) & 1).astype(bool)
    return {
        "cards": cards,
        "feats": feats,
        "mask": mask,
        "seats": ds.seat[idx],
        "glob": ds.glob[idx],
        "deck_cards": dcards,
        "deck_feats": dfeats,
        "deck_mask": dmask,
        "value_target": value_target(ds.col("winners_rel")[idx], ds.col("alive_rel")[idx], seats),
        "team": team,
    }


def to_device(raw: dict[str, np.ndarray], idx: np.ndarray, device: str) -> Batch:
    t = {k: torch.from_numpy(v).pin_memory().to(device, non_blocking=True) for k, v in raw.items()}
    return Batch(idx=idx, **t)


class Loader:
    """Batches of `idx` in a shuffled order, gathered on a background thread."""

    def __init__(self, ds, idx, batch, entities, deck=128, shuffle=True, seed=0, prefetch=4):
        self.ds, self.idx, self.batch, self.entities, self.deck = ds, idx, batch, entities, deck
        self.shuffle, self.rng, self.prefetch = shuffle, np.random.default_rng(seed), prefetch

    def __len__(self) -> int:
        return (len(self.idx) + self.batch - 1) // self.batch

    def __iter__(self):
        order = self.rng.permutation(self.idx) if self.shuffle else self.idx
        chunks = [order[i : i + self.batch] for i in range(0, len(order), self.batch)]
        q: queue.Queue = queue.Queue(self.prefetch)

        def work():
            for c in chunks:
                q.put((gather(self.ds, c, self.entities, self.deck), c))
            q.put(None)

        threading.Thread(target=work, daemon=True).start()
        while (item := q.get()) is not None:
            raw, c = item
            yield to_device(raw, c, "cuda")


# --- feature expansion (on the device) ------------------------------------------------

# v3's entity columns are v2's and then these.
ENT3_EXTRA = ["count", "picked"]
ENT_DENSE3 = D.ENT_DENSE + len(ENT3_EXTRA)


def expand_entities(f: torch.Tensor):
    """(dense, zone, controller_rel, owner_rel): v2's expansion of the first
    38 columns, and the pile's size and picked count, symlog'd."""
    dense, zone, ctrl, own = D.expand_entities(f[..., : len(D.ENT)])
    extra = D.symlog(f[..., len(D.ENT) :].to(torch.float32))
    return torch.cat([dense, extra], dim=-1), zone, ctrl, own


SEAT_NUMERIC = [
    "life", "poison", "energy", "hand", "library", "graveyard", "commander_damage",
    "pool_w", "pool_u", "pool_b", "pool_r", "pool_g", "pool_c",
]
SEAT_FLAGS = ["present", "same_team", "eliminated", "active", "asked", "monarch"]
SEAT_DENSE = len(SEAT_NUMERIC) + len(SEAT_FLAGS)


def expand_seats(s: torch.Tensor, cols: list[str]) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
    """(dense, rel, padding mask) from raw seat rows."""
    at = {c: i for i, c in enumerate(cols)}
    num = D.symlog(s[..., [at[c] for c in SEAT_NUMERIC]].to(torch.float32))
    flags = s[..., [at[c] for c in SEAT_FLAGS]].to(torch.float32).clamp(0, 1)
    rel = s[..., at["rel"]].to(torch.int64).clamp(0, 7)
    return torch.cat([num, flags], dim=-1), rel, s[..., at["present"]] == 0


GLOB3_CATEGORICAL = {"phase": 8, "step": 16, "pending_kind": 20}


def glob_layout(meta: dict) -> tuple[list[int], dict[str, int]]:
    cols = meta["glob_cols"]
    cat = {name: cols.index(name) for name in GLOB3_CATEGORICAL}
    numeric = [i for i, c in enumerate(cols) if c not in GLOB3_CATEGORICAL]
    return numeric, cat
