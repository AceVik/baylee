"""Opt-in decision-window learner (`--window-burn-in B`); the default path never imports it.

A decision at cursor d trains on the window [d - B, d] of its own trajectory,
starting from the recurrent state H that entered frame d - B. Overlapping windows
of one trajectory merge into one run, so no frame is encoded twice. Runs longer
than the lane length split into pieces; every piece starts from a stored H, and
gradient never crosses a piece boundary (the same truncation as a TBPTT segment
boundary). Pieces pack end to end into fixed-length lanes. At a piece start the
lane's memory is replaced by that piece's stored state, so lanes need no padding
between pieces and one lane never leaks memory from one piece into the next.

Stored states come from a no-grad pass of the current network over every frame
(`window_states`, the two-pass mode) or, later, from the actor. Decision frames
never change memory (K>0 keeps H), so that pass encodes passive frames only.
"""
from __future__ import annotations

import hashlib
from pathlib import Path
import random

import numpy as np
import torch

import model4 as M

PIECE = np.dtype([("trajectory", "<i4"), ("start", "<i4"), ("length", "<i4")])


def trajectory_rows(dataset):
    """Store rows of every trajectory in cursor order, as the dataset segments them."""
    return [np.concatenate([dataset.segments[s] for s in segments]) for segments in dataset.trajectories]


def decision_flags(dataset, rows):
    """Decision flags straight from the packed index (bit 0), without decoding frames."""
    flags = np.asarray(dataset.store.index["flags"])
    return [(flags[r] & 1) != 0 for r in rows]


def covered_frames(decisions, burn_in):
    """Frames inside [d - burn_in, d] for some decision d of this trajectory."""
    n = len(decisions)
    marks = np.zeros(n + 1, dtype=np.int64)
    ends = np.flatnonzero(decisions)
    np.add.at(marks, np.maximum(ends - burn_in, 0), 1)
    np.add.at(marks, ends + 1, -1)
    return np.cumsum(marks[:n]) > 0


def plan_pieces(decisions, burn_in, max_length):
    """Every maximal covered run, split into pieces of at most `max_length` frames."""
    if burn_in < 0 or max_length < 1:
        raise ValueError("Window burn-in must be nonnegative and lanes positive")
    pieces = []
    for trajectory, flags in enumerate(decisions):
        covered = covered_frames(flags, burn_in)
        if not covered.any():
            continue
        edges = np.diff(np.concatenate(([0], covered.astype(np.int8), [0])))
        for start, stop in zip(np.flatnonzero(edges == 1), np.flatnonzero(edges == -1)):
            for begin in range(int(start), int(stop), max_length):
                pieces.append((trajectory, begin, min(max_length, int(stop) - begin)))
    return np.array(pieces, dtype=PIECE)


BOARD_INPUTS = (*M.INPUT_NAMES[:8], "entity_rules")


def same_board(left, right):
    """Two decoded frames give one board encoding: every trunk input is equal."""
    return all(left[name].shape == right[name].shape and np.array_equal(left[name], right[name])
               for name in BOARD_INPUTS)


def frame_statistics(dataset, rows, decisions, sample, seed=0):
    """Decoded-frame facts for a sample of decisions: board repeats, weights, shapes."""
    rng = random.Random(seed)
    chosen = [(t, int(c)) for t, flags in enumerate(decisions) for c in np.flatnonzero(flags)]
    chosen = rng.sample(chosen, min(sample, len(chosen)))
    repeats = weighted = previous_passive = 0
    entities, candidates, heads = [], [], {}
    for trajectory, cursor in chosen:
        frame = dataset.encode_single_step(int(rows[trajectory][cursor]))
        weighted += float(frame["loss_weight"]) > 0
        entities.append(int(frame["entity_mask"].sum()))
        candidates.append(int(frame["cand_mask"].sum()))
        heads[int(frame["head"])] = heads.get(int(frame["head"]), 0) + 1
        if cursor and not decisions[trajectory][cursor - 1]:
            previous_passive += 1
            repeats += same_board(dataset.encode_single_step(int(rows[trajectory][cursor - 1])), frame)
    return dict(sampled_decisions=len(chosen), weighted_share=weighted / max(len(chosen), 1),
                previous_frame_passive_share=previous_passive / max(len(chosen), 1),
                board_repeats_previous_passive=repeats / max(previous_passive, 1),
                entities_mean=float(np.mean(entities)) if entities else 0.,
                candidates_mean=float(np.mean(candidates)) if candidates else 0.,
                candidates_p99=float(np.quantile(candidates, .99)) if candidates else 0.,
                heads={str(k): v for k, v in sorted(heads.items())})


def piece_maxima(values, offsets, pieces):
    """Maximum of a per-frame array over each piece's frames."""
    if not len(pieces):
        return np.zeros(0, dtype=values.dtype)
    begin = offsets[pieces["trajectory"]] + pieces["start"]
    end = begin + pieces["length"]
    padded = np.concatenate((values, values[:1]))
    return np.maximum.reduceat(padded, np.stack((begin, end), 1).ravel())[::2]


@torch.no_grad()
def window_states(net, dataset, pieces, device, lanes, thinking_steps, bf16, workers=2, token_budget=0,
                  nograd_scale=8):
    """Stored H entering every piece start, from one no-grad pass over all frames.

    Lanes stream whole trajectories in order, as validation does, and reset H at
    each trajectory start. Only passive frames are encoded: a decision keeps H.
    Returns (states [pieces, 9, d] float32 on the host, per-piece maximum entity count).
    """
    from torch.utils.data import DataLoader
    from sampler4 import StatefulBatchSampler4
    import dataset4 as D4
    from .burn_in import _gather_rows

    unroll = dataset.unroll_len
    d = net.cfg.d_model
    lengths = np.array([sum(len(dataset.segments[s]) for s in segments) for segments in dataset.trajectories])
    offsets = np.concatenate(([0], np.cumsum(lengths)[:-1]))
    position = {}
    for trajectory, segments in enumerate(dataset.trajectories):
        for k, segment in enumerate(segments):
            position[segment] = (trajectory, k * unroll)
    starts = {}
    for pid, (trajectory, start, _) in enumerate(pieces.tolist()):
        segment = dataset.trajectories[trajectory][start // unroll]
        starts.setdefault(segment, []).append((start % unroll, pid))
    entity_counts = np.zeros(int(lengths.sum()), dtype=np.int32)
    states = torch.zeros(len(pieces), M.RECURRENT_SLOTS, d)
    sampler = StatefulBatchSampler4(dataset, batch_size=lanes, shuffle=False)
    loader = DataLoader(dataset, batch_sampler=sampler, collate_fn=D4.collate_trajectories,
                        num_workers=workers, pin_memory=device.type == "cuda",
                        **({"prefetch_factor": 4} if workers else {}))
    hidden = None
    net.clear_structure()
    with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
        net.prepare_structure()
    try:
        for host in loader:
            b = host["step_valid"].shape[0]
            batch = {k: v.to(device, non_blocking=True) for k, v in host.items()}
            if hidden is None or hidden.shape[0] != b:
                hidden = torch.zeros(b, M.RECURRENT_SLOTS, d, device=device)
            hidden = torch.where(batch["is_new_traj"][:, None, None], torch.zeros_like(hidden), hidden)
            valid = host["step_valid"].bool()
            passive = valid & ~host["is_decision"].bool()
            counts = host["entity_mask"].sum(-1).numpy()
            record = [([], []) for _ in range(unroll)]
            for lane, segment in enumerate(host["segment_index"].tolist()):
                if segment < 0:
                    continue
                trajectory, base = position[segment]
                n = int(valid[lane].sum())
                entity_counts[offsets[trajectory] + base:offsets[trajectory] + base + n] = counts[lane, :n]
                for t, pid in starts.get(segment, ()):
                    record[t][0].append(lane)
                    record[t][1].append(pid)
            tokens = M.RECURRENT_SLOTS + host["entity_feat"].shape[2]
            rows = torch.arange(b * unroll)[passive.reshape(-1)]
            per_call = max(1, (token_budget * nograd_scale) // tokens) if token_budget else max(1, rows.numel())
            slots = torch.zeros(b * unroll, M.RECURRENT_SLOTS, d, device=device)
            with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                for start in range(0, rows.numel(), per_call):
                    chosen = rows[start:start + per_call]
                    board, _ = net.encode_board(*_gather_rows(batch, chosen, device))
                    slots.index_copy_(0, chosen.to(device), board[:, :M.RECURRENT_SLOTS].float())
            slots = slots.reshape(b, unroll, M.RECURRENT_SLOTS, d)
            decision = batch["is_decision"].long() * thinking_steps
            captured = []
            for t in range(unroll):
                if record[t][0]:
                    captured.append((record[t][1], hidden[torch.tensor(record[t][0], device=device)]))
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    observed = net.observe_memory(slots[:, t], hidden, decision[:, t])
                hidden = torch.where(batch["step_valid"][:, t, None, None], observed.float(), hidden)
            for pids, values in captured:
                states[torch.tensor(pids)] = values.float().cpu()
    finally:
        net.clear_structure()
        del loader
    return states, piece_maxima(entity_counts, offsets, pieces)


def pack_lanes(piece_ids, lengths, capacity):
    """Best fit, in the given order, of pieces into lanes of `capacity` frames."""
    open_by_room = [[] for _ in range(capacity + 1)]
    lanes = []
    for pid in piece_ids:
        need = int(lengths[pid])
        room = next((r for r in range(need, capacity + 1) if open_by_room[r]), None)
        if room is None:
            lanes.append([pid])
            lane = len(lanes) - 1
            left = capacity - need
        else:
            lane = open_by_room[room].pop()
            lanes[lane].append(pid)
            left = room - need
        if left:
            open_by_room[left].append(lane)
    return [tuple(lane) for lane in lanes]


class WindowBatchSampler:
    """Epoch-seeded lanes of packed pieces, B lanes per batch, optional entity buckets."""

    VERSION = 1

    def __init__(self, plan, lanes_per_batch, length, seed, entity_bucket=0, fraction=1.0):
        self.plan, self.batch_size, self.length, self.seed = plan, lanes_per_batch, length, seed
        self.entity_bucket, self.fraction = entity_bucket, fraction
        self.epoch = 0
        self.start_batch = 0

    def _batches(self, epoch):
        rng = random.Random(self.seed + epoch)
        order = list(range(len(self.plan.pieces)))
        rng.shuffle(order)
        groups = {}
        entities = self.plan.entities
        for pid in order:
            key = int(entities[pid]) // self.entity_bucket if self.entity_bucket and entities is not None else 0
            groups.setdefault(key, []).append(pid)
        batches, leftovers = [], []
        for key in sorted(groups):
            lanes = pack_lanes(groups[key], self.plan.pieces["length"], self.length)
            full = len(lanes) - len(lanes) % self.batch_size
            batches.extend(lanes[i:i + self.batch_size] for i in range(0, full, self.batch_size))
            leftovers.extend(lanes[full:])
        batches.extend(leftovers[i:i + self.batch_size] for i in range(0, len(leftovers), self.batch_size))
        rng.shuffle(batches)
        if self.fraction < 1:
            batches = batches[:max(1, int(len(batches) * self.fraction))]
        return batches

    def __iter__(self):
        batches = self._batches(self.epoch)
        self.epoch += 1
        yield from batches[self.start_batch:]

    def __len__(self):
        return len(self._batches(self.epoch))


class WindowLanes(torch.utils.data.Dataset):
    """One item is one lane: its pieces' frames end to end, padded to the lane length."""

    def __init__(self, base, rows, pieces, length):
        self.base, self.rows, self.pieces, self.length = base, rows, pieces, length
        self.unroll_len = length
        self.structure = base.structure
        self.fingerprint = base.fingerprint

    def __len__(self):
        return len(self.pieces)

    def __getitem__(self, lane):
        from dataset4 import padded_stack
        encoded, starts = [], []
        for pid in lane:
            trajectory, start, length = (int(x) for x in self.pieces[pid].tolist())
            rows = self.rows[trajectory]
            for j in range(length):
                encoded.append(self.base.encode_single_step(int(rows[start + j])))
                starts.append(pid if j == 0 else -1)
        if len(encoded) > self.length:
            raise ValueError("A packed lane exceeds the lane length")
        valid = len(encoded)
        if valid < self.length:
            dummy = {k: np.zeros_like(v) for k, v in encoded[-1].items()}
            dummy["is_decision"] = np.array(False, dtype=bool)
            dummy["loss_weight"] = np.array(0.0, dtype=np.float32)
            dummy["is_terminal"] = np.array(False, dtype=bool)
            dummy["has_logprob"] = np.array(False, dtype=bool)
            dummy["cand_mask"][0] = True
            dummy["pair_mask"][0] = True
            encoded.extend([dummy] * (self.length - valid))
            starts.extend([-1] * (self.length - valid))
        out = {k: padded_stack([e[k] for e in encoded]) for k in encoded[0]}
        out["step_valid"] = np.arange(self.length) < valid
        out["is_new_traj"] = np.array(True, dtype=bool)
        out["segment_index"] = np.array(-1, dtype=np.int64)
        out["piece_start"] = np.array(starts, dtype=np.int64)
        return out


class WindowTraining:
    """Plan, stored states, lane dataset and sampler of one window run."""

    def __init__(self, dataset, burn_in, length, lanes_per_batch, seed, entity_bucket=0, fraction=1.0):
        self.dataset, self.burn_in, self.length = dataset, burn_in, length
        self.rows = trajectory_rows(dataset)
        self.decisions = decision_flags(dataset, self.rows)
        self.pieces = plan_pieces(self.decisions, burn_in, length)
        self.statistics = plan_statistics(self.decisions, self.pieces)
        self.states = None
        self.entities = None
        self.lanes = WindowLanes(dataset, self.rows, self.pieces, length)
        self.sampler = WindowBatchSampler(self, lanes_per_batch, length, seed, entity_bucket, fraction)

    def refresh(self, net, device, lanes, thinking_steps, bf16, workers, token_budget, cache=None, source=None):
        """Recompute stored H with the current weights.

        `cache`, with `source` naming the exact weights, reuses an earlier pass of
        the same plan and weights; it is only for the untouched initial weights.
        """
        # The pieces' digest pins every stored start; lane length only matters through it.
        key = dict(version=1, source=source, burn_in=self.burn_in,
                   fingerprint=self.dataset.fingerprint, thinking_steps=thinking_steps, bf16=bool(bf16),
                   pieces=hashlib.sha256(self.pieces.tobytes()).hexdigest())
        if cache is not None and Path(cache).exists():
            saved = torch.load(cache, map_location="cpu", weights_only=False)
            if saved["key"] != key:
                raise ValueError("Window state cache belongs to another plan or checkpoint")
            self.states, self.entities = saved["states"], saved["entities"]
            return "cache"
        # Pageable on purpose: with B=0 the table is about 23 KB per decision.
        self.states, self.entities = window_states(net, self.dataset, self.pieces, device, lanes,
                                                   thinking_steps, bf16, workers, token_budget)
        if cache is not None:
            Path(cache).parent.mkdir(parents=True, exist_ok=True)
            temporary = Path(str(cache) + ".tmp")
            torch.save(dict(key=key, states=self.states, entities=self.entities), temporary)
            temporary.replace(cache)
        return "computed"

    def resets(self, host, device):
        """Per frame, the lanes whose memory a stored state replaces, or None."""
        if self.states is None:
            raise RuntimeError("Window states must be computed before training")
        starts = host["piece_start"]
        mask = starts >= 0
        b, unroll = mask.shape
        position = mask.nonzero()
        values = self.states.index_select(0, starts[mask]).detach()
        dense = torch.zeros(b, unroll, M.RECURRENT_SLOTS, values.shape[-1], device=device)
        dense[position[:, 0].to(device), position[:, 1].to(device)] = values.to(device, non_blocking=True)
        rows = mask.to(device, non_blocking=True)
        present = mask.any(0).tolist()
        return [(rows[:, t], dense[:, t]) if present[t] else None for t in range(unroll)]


def plan_statistics(decisions, pieces):
    """What the window plan keeps of the data; logged once per run."""
    frames = sum(len(f) for f in decisions)
    counts = np.array([int(f.sum()) for f in decisions])
    covered = int(pieces["length"].sum()) if len(pieces) else 0
    gaps = np.concatenate([np.diff(np.flatnonzero(f)) for f in decisions if f.sum() > 1] or [np.zeros(0)])
    return dict(trajectories=len(decisions), frames=frames, decisions=int(counts.sum()),
                decision_share=float(counts.sum() / max(frames, 1)),
                frames_per_trajectory=float(frames / max(len(decisions), 1)),
                decisions_per_trajectory=float(counts.mean()) if len(counts) else 0.,
                window_frames=covered, window_share=float(covered / max(frames, 1)),
                pieces=len(pieces), mean_piece=float(covered / max(len(pieces), 1)),
                decision_gap_quantiles={str(q): float(np.quantile(gaps, q)) for q in (.1, .25, .5, .75, .9)}
                if len(gaps) else {})
