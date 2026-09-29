"""Reads a dataset the `convert` binary wrote, and turns batches into tensors.

A dataset is a directory: `dataset.json` names every column, and each
`shard-NN/` holds flat little-endian column files (see
`crates/baylee-train/src/bin/convert.rs`). Everything is loaded into RAM once;
a batch is gathered on the CPU as raw integers and expanded into features on
the GPU (`expand`), so the columns stay the converter's and every choice of
how to read them lives here, in one place.
"""

from __future__ import annotations

import json
import threading
import queue
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch

ENCODER_VERSION = 1


@dataclass
class Dataset:
    meta: dict
    ent_card: np.ndarray  # (E_total,) int32
    ent_feat: np.ndarray  # (E_total, ENT_COLS) int16
    ent_off: np.ndarray  # (N + 1,) int64
    glob: np.ndarray  # (N, GLOB_WIDTH) int16
    info: np.ndarray  # (N, META_COLS) int32
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
    n_ent = len(meta["ent_cols"])
    n_glob = meta["glob_width"]
    n_meta = len(meta["meta_cols"])
    # Preallocated and filled shard by shard: concatenating would hold every
    # column twice at its peak, and a large dataset is most of the machine.
    samples = sum(s["samples"] for s in meta["shards"])
    entities = sum(s["entities"] for s in meta["shards"])
    ent_card = np.empty(entities, dtype=np.int32)
    ent_feat = np.empty((entities, n_ent), dtype=np.int16)
    ent_off = np.empty(samples + 1, dtype=np.int64)
    glob = np.empty((samples, n_glob), dtype=np.int16)
    info = np.empty((samples, n_meta), dtype=np.int32)
    e0, s0 = 0, 0
    for shard in meta["shards"]:
        d = path / shard["dir"]
        ns, ne = shard["samples"], shard["entities"]
        if ns == 0:
            continue
        ent_card[e0 : e0 + ne] = np.memmap(d / "ent_card.i32", dtype="<i4", mode="r")
        ent_feat[e0 : e0 + ne] = np.memmap(d / "ent_feat.i16", dtype="<i2", mode="r").reshape(-1, n_ent)
        ent_off[s0 : s0 + ns] = np.memmap(d / "ent_off.i64", dtype="<i8", mode="r")[:-1] + e0
        glob[s0 : s0 + ns] = np.memmap(d / "glob.i16", dtype="<i2", mode="r").reshape(-1, n_glob)
        info[s0 : s0 + ns] = np.memmap(d / "meta.i32", dtype="<i4", mode="r").reshape(-1, n_meta)
        e0 += ne
        s0 += ns
    ent_off[s0] = e0
    assert e0 == entities and s0 == samples
    games = [json.loads(l) for l in (path / "games.jsonl").read_text().splitlines()]
    return Dataset(meta=meta, ent_card=ent_card, ent_feat=ent_feat, ent_off=ent_off, glob=glob, info=info, games=games)


def split_by_game(ds: Dataset, held_out_every: int = 10) -> tuple[np.ndarray, np.ndarray]:
    """Train and held-out sample indices. A game's decisions are correlated,
    so a game is wholly on one side; every `held_out_every`-th game is held out."""
    game = ds.col("game")
    held = (game % held_out_every) == 0
    return np.nonzero(~held)[0], np.nonzero(held)[0]


@dataclass
class Batch:
    """Raw integers of a batch, on the device."""

    cards: torch.Tensor  # (B, E) int64, 0 = padding
    feats: torch.Tensor  # (B, E, ENT_COLS) int16
    mask: torch.Tensor  # (B, E) bool, True = padding
    glob: torch.Tensor  # (B, GLOB_WIDTH) int16
    target: torch.Tensor  # (B,) float, 1 win, 0.5 draw, 0 loss
    idx: np.ndarray


def gather(ds: Dataset, idx: np.ndarray, entities: int) -> dict[str, np.ndarray]:
    start = ds.ent_off[idx]
    count = np.minimum(ds.ent_off[idx + 1] - start, entities)
    ar = np.arange(entities)
    rows = start[:, None] + ar[None, :]
    live = ar[None, :] < count[:, None]
    rows = np.where(live, rows, 0)
    cards = np.where(live, ds.ent_card[rows], 0).astype(np.int64)
    feats = np.where(live[..., None], ds.ent_feat[rows], 0).astype(np.int16)
    result = ds.info[idx, ds.meta["meta_cols"].index("result")]
    return {
        "cards": cards,
        "feats": feats,
        "mask": ~live,
        "glob": ds.glob[idx],
        "target": (result.astype(np.float32) / 2.0),
    }


def to_device(raw: dict[str, np.ndarray], idx: np.ndarray, device: str) -> Batch:
    t = {k: torch.from_numpy(v).pin_memory().to(device, non_blocking=True) for k, v in raw.items()}
    return Batch(t["cards"], t["feats"], t["mask"], t["glob"], t["target"], idx)


class Loader:
    """Batches of `idx` in a shuffled order, gathered on a background thread."""

    def __init__(self, ds: Dataset, idx: np.ndarray, batch: int, entities: int, shuffle: bool, seed: int = 0, prefetch: int = 4):
        self.ds, self.idx, self.batch, self.entities = ds, idx, batch, entities
        self.shuffle, self.rng, self.prefetch = shuffle, np.random.default_rng(seed), prefetch

    def __len__(self) -> int:
        return (len(self.idx) + self.batch - 1) // self.batch

    def __iter__(self):
        order = self.rng.permutation(self.idx) if self.shuffle else self.idx
        chunks = [order[i : i + self.batch] for i in range(0, len(order), self.batch)]
        q: queue.Queue = queue.Queue(self.prefetch)

        def work():
            for c in chunks:
                q.put((gather(self.ds, c, self.entities), c))
            q.put(None)

        threading.Thread(target=work, daemon=True).start()
        while (item := q.get()) is not None:
            raw, c = item
            yield to_device(raw, c, "cuda")


# --- feature expansion (on the device) ------------------------------------------------

ENT = {
    name: i
    for i, name in enumerate(
        [
            "zone", "controller_rel", "owner_rel", "status", "types", "supertypes", "colors",
            "mana_value", "power", "toughness", "base_power", "base_toughness", "flags", "loyalty",
            "damage", "plus_counters", "minus_counters", "lore_counters", "time_counters",
            "charge_counters", "other_counters", "attached_to", "attacks_player_rel", "attacks_row",
            "blocks_row", "stack_pos", "stack_kind", "target_row", "target_player_rel", "offered",
        ]
        + [f"keywords{k}" for k in range(8)]
    )
}
# Bit fields and how many bits of each are read.
ENT_BITS = {"status": 4, "types": 16, "supertypes": 8, "colors": 8, "flags": 9, "offered": 5}
ENT_NUMERIC = [
    "mana_value", "power", "toughness", "base_power", "base_toughness", "loyalty", "damage",
    "plus_counters", "minus_counters", "lore_counters", "time_counters", "charge_counters",
    "other_counters", "stack_pos",
]
ENT_POINTERS = ["attached_to", "attacks_row", "blocks_row", "target_row"]
ENT_SEAT_REL = ["attacks_player_rel", "target_player_rel"]
ENT_DENSE = (
    sum(ENT_BITS.values()) + 128 + len(ENT_NUMERIC) + len(ENT_POINTERS) + 5 * len(ENT_SEAT_REL) + 3
)

GLOB_CATEGORICAL = {"phase": 8, "step": 16, "pending_kind": 20}


def symlog(x: torch.Tensor) -> torch.Tensor:
    return torch.sign(x) * torch.log1p(x.abs())


def bits(x: torch.Tensor, n: int) -> torch.Tensor:
    """The low `n` bits of int16 `x` as floats, last dimension."""
    x = x.to(torch.int32) & 0xFFFF
    shifts = torch.arange(n, device=x.device, dtype=torch.int32)
    return ((x.unsqueeze(-1) >> shifts) & 1).to(torch.float32)


def one_hot_rel(x: torch.Tensor, n: int = 4) -> torch.Tensor:
    """-1 (none) and 0..n-1 as n+1 one-hot slots."""
    return torch.nn.functional.one_hot((x.to(torch.int64) + 1).clamp(0, n), n + 1).to(torch.float32)


def expand_entities(f: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    """(dense features, zone, controller_rel, owner_rel) from raw entity columns."""
    parts = [bits(f[..., ENT[name]], n) for name, n in ENT_BITS.items()]
    parts.append(torch.cat([bits(f[..., ENT[f"keywords{k}"]], 16) for k in range(8)], dim=-1))
    parts.append(symlog(f[..., [ENT[c] for c in ENT_NUMERIC]].to(torch.float32)))
    parts.append((f[..., [ENT[c] for c in ENT_POINTERS]] >= 0).to(torch.float32))
    parts.extend(one_hot_rel(f[..., ENT[c]]) for c in ENT_SEAT_REL)
    parts.append(torch.nn.functional.one_hot(f[..., ENT["stack_kind"]].to(torch.int64).clamp(0, 2), 3).to(torch.float32))
    dense = torch.cat(parts, dim=-1)
    zone = f[..., ENT["zone"]].to(torch.int64).clamp(0, 15)
    ctrl = f[..., ENT["controller_rel"]].to(torch.int64).clamp(0, 7)
    own = f[..., ENT["owner_rel"]].to(torch.int64).clamp(0, 7)
    return dense, zone, ctrl, own


def glob_layout(meta: dict) -> tuple[list[int], dict[str, int]]:
    cols = meta["glob_cols"]
    cat = {name: cols.index(name) for name in GLOB_CATEGORICAL}
    numeric = [i for i, c in enumerate(cols) if c not in GLOB_CATEGORICAL]
    return numeric, cat
