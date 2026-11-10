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

import dataclasses
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch

RECORD_CONTRACT = "tactician_imitation_v1"
# Stage-1 outcome records, read only by the offline probe (`read_records(...,
# stage1_probe=True)`), their rule columns zeroed.
STAGE1_CONTRACT = "tactician_records_v1"
ROW_TAIL = 128
WINDOW_WIDTH = 16
WINDOWS = ("attack", "block", "target", "priority")
ATTACK, BLOCK, TARGET, PRIORITY = 0, 1, 2, 3
# Windows answered by one unit (softmax over the units), and their window-code
# columns.
SINGLE_WINDOWS = (TARGET, PRIORITY)
SINGLE_CODE_COLUMNS = (2, 12)
# Look columns (after the window code), for ablations: the castable instants
# under the train-deck prior (0-24), every look column but the facts (0-99),
# and the trivial facts of stage S1 rows (100-111).
PRIOR_COLUMNS = (0, 25)
LOOK_COLUMNS = (0, 100)
FACT_COLUMNS = (100, 112)
ZERO_PARTS = ("state", "unit", "look", "prior", "facts")
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
    # Per decision, the rule groups that changed the layer's answer there
    # (`fired`; empty for stage-1 records, whose layer ran no group).
    fired: list
    seat: np.ndarray
    sources: list
    # Per decision, what kind of decision it is (`decision_kind`: the window,
    # or at priority what the layer did, e.g. pass, cast, spend).
    kinds: list
    # Parts of every row read as zero (ZERO_PARTS; the window code
    # always stays): an ablation of
    # the probe, which part carries what the label needs.
    zero_parts: tuple = ()

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
        floats = ("states", "units", "tails")
        tensors = floats + ("labels", "nets", "groups", "n", "unit_offset", "tail_offset", "window")
        return dataclasses.replace(
            self, **{name: move(getattr(self, name), name in floats) for name in tensors})

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
        if "state" in self.zero_parts:
            state = torch.zeros_like(state)
        if "unit" in self.zero_parts:
            units = torch.zeros_like(units)
        for part, (lo, hi) in (("look", LOOK_COLUMNS), ("prior", PRIOR_COLUMNS),
                               ("facts", FACT_COLUMNS)):
            if part in self.zero_parts:
                keep = torch.ones(ROW_TAIL, dtype=tails.dtype, device=tails.device)
                keep[WINDOW_WIDTH + lo:WINDOW_WIDTH + hi] = 0.0
                tails = tails * keep
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


def action_kind(key: str) -> str:
    """The action a priority unit's key names: its variant, e.g. CastSpell."""
    try:
        action = json.loads(key)
    except (TypeError, ValueError):
        return "other"
    if isinstance(action, str):
        return action
    if isinstance(action, dict) and action:
        return next(iter(action))
    return "other"


def decision_kind(record: dict, tail: np.ndarray) -> str:
    """What kind of decision a record is: the window, or at priority what the
    layer's answer R did, read with the net's N and the facts of the position:
    `pay` (a cost is being paid), `pass` (R passes), `spend` (N passes, R
    acts in the seat's own turn), `instant_their_turn` (R casts or
    activates in the other side's turn), `cast`, `activate`, `mana`,
    `land`, `other`."""
    if record["window"] != "priority":
        return record["window"]
    facts = tail[0, WINDOW_WIDTH + FACT_COLUMNS[0]:WINDOW_WIDTH + FACT_COLUMNS[1]]
    my_turn, paying = facts[0] > 0.5, facts[10] > 0.5
    keys = record["keys"]
    rules = next((action_kind(k) for k, bit in zip(keys, record["rules"]) if bit), "other")
    net = next((action_kind(k) for k, bit in zip(keys, record["net"]) if bit), "other")
    if paying:
        return "pay"
    if rules == "PassPriority":
        return "pass"
    acts = rules in ("CastSpell", "ActivateAbility")
    if acts and not my_turn:
        return "instant_their_turn"
    if acts and net == "PassPriority":
        return "spend"
    return {"CastSpell": "cast", "ActivateAbility": "activate",
            "ActivateManaAbility": "mana", "PlayLand": "land"}.get(rules, "other")


def _check_stage1_line(path: Path, record: dict) -> None:
    if record.get("contract") != STAGE1_CONTRACT:
        raise ValueError(f"{path}: not a {STAGE1_CONTRACT} record")
    if record["window"] not in WINDOWS:
        raise ValueError(f"{path}: unknown window {record['window']!r}")


def groups_from_keys(window: str, keys) -> list:
    """Each unit's group from a stage-1 line's keys: the attacker of
    `attacker@defender`, the blocker of `blocker>attacker`, one group at a
    target; numbered in order of first appearance."""
    if window in ("target", "priority"):
        return [0] * len(keys)
    sep = "@" if window == "attack" else ">"
    seen = {}
    return [seen.setdefault(key.split(sep, 1)[0], len(seen)) for key in keys]


def read_records(dirs, stage1_probe: bool = False) -> Store:
    """Every decision of the record directories, checked, in one [`Store`].

    `stage1_probe` reads stage-1 outcome records (`tactician_records_v1`) for
    the offline probe instead: their `rules` (the layer with no rule group) is
    the label, the window-code columns that carry it are zeroed on reading, and
    the groups come from the keys. A measurement; nothing it trains is played.
    """
    states, units, tails = [], [], []
    labels, nets, groups = [], [], []
    ns, windows, seeds, games, seats, sources, fired, kinds = [], [], [], [], [], [], [], []
    d = None
    for directory in map(Path, dirs):
        if not stage1_probe:
            read_run(directory)
        blobs = {}
        for lines in sorted(directory.glob("w*.jsonl")):
            for text in lines.read_text().splitlines():
                if not text.strip():
                    continue
                record = json.loads(text)
                if stage1_probe:
                    _check_stage1_line(lines, record)
                    record["groups"] = groups_from_keys(record["window"], record["keys"])
                else:
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
                if stage1_probe:
                    tail[:, list(RULE_COLUMNS)] = 0.0
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
                fired.append(tuple(record.get("fired", ())))
                kinds.append(decision_kind(record, tail))
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
        fired=fired,
        kinds=kinds,
        seat=np.asarray(seats, np.int64),
        sources=sources,
    )
