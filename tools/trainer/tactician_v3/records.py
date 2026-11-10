"""Imitation records (`tactician_imitation_v1`) and padded batches of their rows.

A record directory holds `run.json` (contract `tactician_imitation_v1`) and, per
worker, `wNN.jsonl` beside `wNN.f32`. Each line is one window decision: the
units' keys and groups, the net's answer `net`, the layer's answer `rules` (the
label), and where its floats lie in the blob: the thought-through state `[d]`,
each unit's embedding `[n, d]`, and each row's tail `[(n + 1), 128]` (window code
and look), the null row first. A row is `[state | unit (zero for the null row) |
tail]`, as `netplay4::tactician::head::assemble` lays it out. The tails never
carry `R` (`rows_read_rules: false`): columns 5 and 11 of the window code are
zero.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch

RECORD_CONTRACT = "tactician_imitation_v1"
ROW_TAIL = 128
WINDOW_WIDTH = 16
WINDOWS = ("attack", "block", "target")
ATTACK, BLOCK, TARGET = 0, 1, 2
# Window-code columns that would read the layer's answer: never set here.
RULE_COLUMNS = (5, 11)
# Window-code column holding the net's own answer for the unit.
NET_COLUMN = 4


def read_run(directory: Path) -> dict:
    """The directory's `run.json`, which must name the imitation contract."""
    path = directory / "run.json"
    if not path.exists():
        raise ValueError(f"{directory}: no run.json; not an imitating collection")
    run = json.loads(path.read_text())
    if run.get("contract") != RECORD_CONTRACT:
        raise ValueError(f"{directory}: run.json is not a {RECORD_CONTRACT} run")
    return run


@dataclass
class Store:
    """Every decision of some record directories, flat, with offsets.

    `states` [D, d]; `units` [sum n, d]; `tails` [sum (n + 1), 128]; per unit
    `labels`, `nets` and `groups`; per decision `n`, `unit_offset`, `tail_offset`,
    `window`, `seed`, `game` and `seat`.
    """

    states: torch.Tensor
    units: torch.Tensor
    tails: torch.Tensor
    labels: torch.Tensor
    nets: torch.Tensor
    groups: torch.Tensor
    n: torch.Tensor
    unit_offset: torch.Tensor
    tail_offset: torch.Tensor
    window: torch.Tensor
    seed: np.ndarray
    game: list
    seat: np.ndarray
    sources: list

    @property
    def d_model(self) -> int:
        return int(self.states.shape[1])

    @property
    def width(self) -> int:
        return 2 * self.d_model + ROW_TAIL

    def __len__(self) -> int:
        return int(self.n.shape[0])

    def to(self, device, dtype=None) -> "Store":
        """The arrays on `device`; floats in `dtype` when given (e.g. float16)."""
        def move(t, floating):
            t = t.to(device)
            return t.to(dtype) if floating and dtype is not None else t
        return Store(move(self.states, True), move(self.units, True), move(self.tails, True),
                     move(self.labels, False), move(self.nets, False), move(self.groups, False),
                     move(self.n, False), move(self.unit_offset, False),
                     move(self.tail_offset, False), move(self.window, False),
                     self.seed, self.game, self.seat, self.sources)

    def split(self, held_out_mod: int):
        """Decision indices of the training and held-out games: a table is held
        out when its seed is divisible by `held_out_mod`, both seats together."""
        held = (self.seed % held_out_mod) == 0
        return np.flatnonzero(~held), np.flatnonzero(held)

    def batch(self, index, device=None) -> "Batch":
        """The decisions `index`, padded to the largest: rows [B, U, W] (null row
        first), mask [B, U], and per unit labels, nets and groups [B, U - 1]."""
        device = device or self.states.device
        idx = torch.as_tensor(np.asarray(index), dtype=torch.long, device=self.n.device)
        n = self.n[idx]
        rows_per = n + 1
        width = int(rows_per.max())
        pos = torch.arange(width, device=n.device)[None, :]
        valid = pos < rows_per[:, None]
        tail_at = (self.tail_offset[idx][:, None] + pos).clamp(max=self.tails.shape[0] - 1)
        unit_pos = pos[:, 1:] - 1
        unit_valid = valid[:, 1:]
        unit_at = (self.unit_offset[idx][:, None] + unit_pos).clamp(0, self.units.shape[0] - 1)
        d = self.d_model
        b = len(idx)
        state = self.states[idx].float()[:, None, :].expand(b, width, d)
        units = torch.zeros(b, width, d, dtype=torch.float32, device=n.device)
        units[:, 1:] = self.units[unit_at].float() * unit_valid[..., None]
        tails = self.tails[tail_at].float() * valid[..., None]
        rows = torch.cat([state * valid[..., None], units, tails], dim=-1)
        labels = (self.labels[unit_at].float() * unit_valid)
        nets = (self.nets[unit_at].float() * unit_valid)
        groups = torch.where(unit_valid, self.groups[unit_at].long(),
                             torch.full_like(unit_at, -1))
        return Batch(rows.to(device), valid.to(device), labels.to(device), nets.to(device),
                     groups.to(device), self.window[idx].to(device))


@dataclass
class Batch:
    rows: torch.Tensor     # [B, U, W]
    mask: torch.Tensor     # [B, U] bool, the null row always true
    labels: torch.Tensor   # [B, U - 1] float {0, 1}
    nets: torch.Tensor     # [B, U - 1] float {0, 1}
    groups: torch.Tensor   # [B, U - 1] long, -1 for padding
    window: torch.Tensor   # [B] long

    @property
    def unit_mask(self) -> torch.Tensor:
        return self.mask[:, 1:]


def _check_line(path: Path, record: dict) -> None:
    if record.get("contract") != RECORD_CONTRACT:
        raise ValueError(f"{path}: not a {RECORD_CONTRACT} record "
                         f"({record.get('contract')!r}); this trainer reads imitation records only")
    if record.get("label") != "rules" or record.get("rows_read_rules") is not False:
        raise ValueError(f"{path}: the label must be the layer's answer and the rows must not "
                         "carry it")
    if record["window"] not in WINDOWS:
        raise ValueError(f"{path}: unknown window {record['window']!r}")


def read_records(dirs) -> Store:
    """Every decision of the record directories, checked, in one [`Store`]."""
    states, units, tails = [], [], []
    labels, nets, groups = [], [], []
    ns, windows, seeds, games, seats, sources = [], [], [], [], [], []
    d = None
    for directory in map(Path, dirs):
        read_run(directory)
        blobs = {}
        for lines in sorted(directory.glob("w*.jsonl")):
            for text in lines.read_text().splitlines():
                if not text.strip():
                    continue
                record = json.loads(text)
                _check_line(lines, record)
                blob = record["blob"]
                if blob["file"] not in blobs:
                    blobs[blob["file"]] = np.memmap(directory / blob["file"], dtype="<f4", mode="r")
                data = blobs[blob["file"]]
                start = blob["offset"] // 4
                n = len(record["rules"])
                state = np.array(data[start:start + blob["state"]])
                start += blob["state"]
                unit = np.array(data[start:start + blob["units"]])
                start += blob["units"]
                tail = np.array(data[start:start + blob["rows"]])
                if d is None:
                    d = len(state)
                if (len(state) != d or unit.size != n * d or tail.size != (n + 1) * ROW_TAIL
                        or n == 0 or len(record["net"]) != n or len(record["groups"]) != n):
                    raise ValueError(f"{lines}: arrays do not match the decision's units")
                tail = tail.reshape(n + 1, ROW_TAIL)
                if np.any(tail[:, list(RULE_COLUMNS)] != 0):
                    raise ValueError(f"{lines}: rows carry the layer's answer")
                states.append(state)
                units.append(unit.reshape(n, d))
                tails.append(tail)
                labels.append(np.asarray(record["rules"], np.uint8))
                nets.append(np.asarray(record["net"], np.uint8))
                groups.append(np.asarray(record["groups"], np.int32))
                ns.append(n)
                windows.append(WINDOWS.index(record["window"]))
                seeds.append(int(record["seed"]))
                games.append(record["game"])
                seats.append(int(record["seat"]))
                sources.append(str(directory))
    if not ns:
        raise ValueError("no decisions")
    n = np.asarray(ns, np.int64)
    unit_offset = np.concatenate([[0], np.cumsum(n)[:-1]])
    tail_offset = np.concatenate([[0], np.cumsum(n + 1)[:-1]])
    return Store(
        states=torch.from_numpy(np.stack(states).astype(np.float32)),
        units=torch.from_numpy(np.concatenate(units).astype(np.float32)),
        tails=torch.from_numpy(np.concatenate(tails).astype(np.float32)),
        labels=torch.from_numpy(np.concatenate(labels)),
        nets=torch.from_numpy(np.concatenate(nets)),
        groups=torch.from_numpy(np.concatenate(groups)),
        n=torch.from_numpy(n),
        unit_offset=torch.from_numpy(unit_offset),
        tail_offset=torch.from_numpy(tail_offset),
        window=torch.tensor(windows, dtype=torch.long),
        seed=np.asarray(seeds, np.uint64),
        game=games,
        seat=np.asarray(seats, np.int64),
        sources=sources,
    )
