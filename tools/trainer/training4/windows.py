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
import json
import os
from pathlib import Path
import random
import time

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


class StateStore:
    """Stored H at piece starts: memory slots 0..seats only, in RAM or in a file.

    A game with S seats keeps state any output reads only in slot 0 (global) and
    the seat slots 1..S: the recurrent cell, the norms and the thinking queries act
    per slot, thinking attends to board keys in which absent seats are masked, value
    readouts mask inactive seats and candidates gather only seats in the game. The
    unstored slots read back as zero. `bf16` stores the bf16 bits (int16), which
    rounds H; the default float32 keeps it exact.

    A file store is read and written with positional I/O, never mapped: its pages
    live in the reclaimable page cache and never count in the process's resident
    set (the train_ops guard limits the group's PSS).
    """

    def __init__(self, count, seats, width, bf16=False, path=None, mode="w+"):
        if not 1 <= seats <= M.RECURRENT_SLOTS - 1:
            raise ValueError("Stored seats must be between one and eight")
        self.count, self.slots, self.width, self.bf16 = count, seats + 1, width, bf16
        self.path = None if path is None else Path(path)
        self.dtype = np.dtype(np.int16 if bf16 else np.float32)
        self.shape = (count, self.slots, width)
        self.row_bytes = self.slots * width * self.dtype.itemsize
        self.array = self.fd = None
        if self.path is None:
            self.array = np.zeros(self.shape, dtype=self.dtype)
        elif mode == "r":
            self.fd = os.open(self.path, os.O_RDONLY)
            if os.fstat(self.fd).st_size != count * self.row_bytes:
                os.close(self.fd)
                raise ValueError(f"{self.path} does not hold {count} stored states of {self.row_bytes} bytes")
        else:
            self.fd = os.open(self.path, os.O_RDWR | os.O_CREAT | os.O_TRUNC, 0o644)
            os.ftruncate(self.fd, count * self.row_bytes)

    @staticmethod
    def bytes_for(count, seats, width, bf16=False):
        return count * (seats + 1) * width * (2 if bf16 else 4)

    def write(self, pids, values):
        """Store `values` [n, 9, d] (any device) for pieces `pids`."""
        part = values[:, :self.slots].detach().float().cpu()
        data = part.to(torch.bfloat16).view(torch.int16).numpy() if self.bf16 else part.numpy()
        pids = np.asarray(pids, dtype=np.int64)
        if self.fd is None:
            self.array[pids] = data
            return
        order = np.argsort(pids, kind="stable")
        pids = pids[order]
        # One write per run of consecutive pieces.
        breaks = np.flatnonzero(np.diff(pids) != 1) + 1
        for begin, end in zip(np.concatenate(([0], breaks)), np.concatenate((breaks, [len(pids)]))):
            block = np.ascontiguousarray(data[order[begin:end]])
            view, offset = memoryview(block).cast("B"), int(pids[begin]) * self.row_bytes
            while view:
                written = os.pwrite(self.fd, view, offset)
                view, offset = view[written:], offset + written

    def read(self, ids):
        """[n, 9, d] float32 states of pieces `ids`; slots past the stored seats are zero."""
        ids = np.asarray(ids, dtype=np.int64)
        if self.fd is None:
            rows = np.ascontiguousarray(self.array[ids])
        else:
            rows = np.empty((len(ids), self.slots, self.width), dtype=self.dtype)
            for k, pid in enumerate(ids.tolist()):
                if os.preadv(self.fd, [memoryview(rows[k]).cast("B")], pid * self.row_bytes) != self.row_bytes:
                    raise ValueError(f"{self.path} ended before stored state {pid}")
        rows = torch.from_numpy(rows)
        if self.bf16:
            rows = rows.view(torch.bfloat16).float()
        states = torch.zeros(len(rows), M.RECURRENT_SLOTS, self.width)
        states[:, :self.slots] = rows
        return states

    def flush(self):
        """Write a file store through to disk (a cache's key is written after it)."""
        if self.fd is not None:
            os.fsync(self.fd)

    def close(self, delete=False):
        if self.fd is not None:
            os.close(self.fd)
        self.array = self.fd = None
        if delete and self.path is not None and self.path.exists():
            self.path.unlink()


def piece_lookup(dataset, pieces):
    """(segment of each piece start, its frame in that segment, pieces sorted by segment)."""
    unroll = dataset.unroll_len
    flat = np.array([s for segments in dataset.trajectories for s in segments], dtype=np.int64)
    first = np.concatenate(([0], np.cumsum([len(s) for s in dataset.trajectories])[:-1])).astype(np.int64)
    start = pieces["start"].astype(np.int64)
    segment = flat[first[pieces["trajectory"]] + start // unroll]
    order = np.argsort(segment, kind="stable")
    return segment, start % unroll, order


# What the state pass reads of a batch: the board inputs and the step flags.
STATE_PASS_FIELDS = (*BOARD_INPUTS, "step_valid", "is_decision", "is_new_traj", "segment_index")


def collate_boards(items):
    """The state pass's batch: `STATE_PASS_FIELDS` only.

    A loader worker hands every tensor of a batch to the parent through a file
    in /dev/shm. Candidates, pairs and targets never go there (one card-naming
    question pads every frame of its batch to its candidate count), and the
    entity and edge axes are cut to their last used column in fresh storage,
    so the shared bytes are the bytes the pass reads.
    """
    from dataset4 import padded_stack
    out = {k: torch.from_numpy(padded_stack([item[k] for item in items])) for k in STATE_PASS_FIELDS}
    for mask, names in (("entity_mask", ("entity_feat", "entity_dsl", "entity_mask", "entity_rules")),
                        ("edge_mask", ("edge_index", "edge_type", "edge_mask"))):
        used = out[mask].any(dim=(0, 1)).nonzero()
        width = int(used[-1]) + 1 if len(used) else 1
        if width < out[mask].shape[2]:
            for name in names:
                out[name] = out[name][:, :, :width].clone()
    return out


def state_pass_loader(dataset, lanes, workers, prefetch=2, pin_memory=False, collate=collate_boards):
    """Whole trajectories in order, `lanes` segments per batch, as the state pass reads them.

    At most `workers * prefetch` batches are in flight; each sits in /dev/shm
    from the moment its worker hands it over until the parent drops it.
    """
    from torch.utils.data import DataLoader
    from sampler4 import StatefulBatchSampler4
    sampler = StatefulBatchSampler4(dataset, batch_size=lanes, shuffle=False)
    return DataLoader(dataset, batch_sampler=sampler, collate_fn=collate, num_workers=workers,
                      pin_memory=pin_memory, **({"prefetch_factor": prefetch} if workers else {}))


@torch.no_grad()
def window_states(net, dataset, pieces, device, lanes, thinking_steps, bf16, workers=2, token_budget=0,
                  nograd_scale=8, timing_out=None, store=None, prefetch=2):
    """Stored H entering every piece start, from one no-grad pass over all frames.

    Lanes stream whole trajectories in order, as validation does, and reset H at
    each trajectory start. Only passive frames are encoded: a decision keeps H.
    Writes into `store` (a `StateStore`; by default every slot in RAM) and returns
    (store, per-piece maximum entity count). A frame with a present seat beyond
    the store's seats fails: its state would be dropped.
    """
    from .burn_in import encode_trimmed, trimmed_groups
    from .memory_budget import shm_quota

    unroll = dataset.unroll_len
    d = net.cfg.d_model
    store = StateStore(len(pieces), M.RECURRENT_SLOTS - 1, d) if store is None else store
    seats = store.slots - 1
    lengths = np.array([sum(len(dataset.segments[s]) for s in segments) for segments in dataset.trajectories])
    offsets = np.concatenate(([0], np.cumsum(lengths)[:-1]))
    position = {}
    for trajectory, segments in enumerate(dataset.trajectories):
        for k, segment in enumerate(segments):
            position[segment] = (trajectory, k * unroll)
    piece_segment, piece_frame, by_segment = piece_lookup(dataset, pieces)
    sorted_segment = piece_segment[by_segment]
    entity_counts = np.zeros(int(lengths.sum()), dtype=np.int32)
    loader = state_pass_loader(dataset, lanes, workers, prefetch, pin_memory=device.type == "cuda")
    hidden = None
    # Loader workers verify the packed cache before their first read; report
    # that start apart from the steady pass.
    began = time.perf_counter()
    timing = {} if timing_out is None else timing_out
    timing.clear()
    timing.update(first_batch_s=None, frames=0, encoded=0)
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
            if seats < M.RECURRENT_SLOTS - 1 and bool(
                    ((host["seat_feat"][:, :, seats:, M.SEAT_FEAT_PRESENT] > .5).any(-1) & valid).any()):
                raise ValueError(f"A frame has a seat beyond --window-state-seats {seats}; its state would be lost")
            counts = host["entity_mask"].sum(-1).numpy()
            record = [([], []) for _ in range(unroll)]
            for lane, segment in enumerate(host["segment_index"].tolist()):
                if segment < 0:
                    continue
                trajectory, base = position[segment]
                n = int(valid[lane].sum())
                entity_counts[offsets[trajectory] + base:offsets[trajectory] + base + n] = counts[lane, :n]
                lo, hi = np.searchsorted(sorted_segment, segment, "left"), np.searchsorted(sorted_segment, segment, "right")
                for pid in by_segment[lo:hi].tolist():
                    record[int(piece_frame[pid])][0].append(lane)
                    record[int(piece_frame[pid])][1].append(pid)
            tokens = M.RECURRENT_SLOTS + host["entity_feat"].shape[2]
            rows = torch.arange(b * unroll)[passive.reshape(-1)]
            limit = token_budget * nograd_scale if token_budget else max(1, rows.numel()) * tokens
            slots = torch.zeros(b * unroll, M.RECURRENT_SLOTS, d, device=device)
            with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                # Grouped by board size and trimmed: no compute on entity padding.
                for chosen, width in trimmed_groups(rows, counts.reshape(-1), limit):
                    board, _ = encode_trimmed(net, batch, chosen, width, device)
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
                store.write(pids, values)
            timing["frames"] += int(valid.sum())
            timing["encoded"] += int(rows.numel())
            # This user's /dev/shm in use: the batches the workers hand over wait there.
            shm = shm_quota()[1]
            if shm is not None:
                timing["shm_peak_gib"] = max(timing.get("shm_peak_gib", 0.), round(shm / 2 ** 30, 3))
            if timing["first_batch_s"] is None:
                timing["first_batch_s"] = time.perf_counter() - began
                timing["first_batch_frames"] = timing["frames"]
                steady = time.perf_counter()
    finally:
        net.clear_structure()
        del loader
    store.flush()
    timing["total_s"] = time.perf_counter() - began
    if timing["first_batch_s"] is not None:
        timing["steady_s"] = time.perf_counter() - steady
    return store, piece_maxima(entity_counts, offsets, pieces)


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
        self._owned = False
        self.entities = None
        self.timing = {}
        self.lanes = WindowLanes(dataset, self.rows, self.pieces, length)
        self.sampler = WindowBatchSampler(self, lanes_per_batch, length, seed, entity_bucket, fraction)

    def store_bytes(self, width, seats=M.RECURRENT_SLOTS - 1, bf16=False):
        return StateStore.bytes_for(len(self.pieces), seats, width, bf16)

    def refresh(self, net, device, lanes, thinking_steps, bf16, workers, token_budget, cache=None, source=None,
                seats=M.RECURRENT_SLOTS - 1, store_bf16=False, run_dir=None, prefetch=2):
        """Recompute stored H with the current weights.

        `cache`, with `source` naming the exact weights, reuses an earlier pass of
        the same plan, weights and store layout; it is only for the untouched
        initial weights. The cache is a JSON key file written last, beside its
        `.states` file (raw rows [piece, slot, d]) and its `.entities.npy`.
        Without a cache a pass writes a store file in `run_dir` (removed by
        `close`), or keeps it in RAM when there is no directory.
        """
        width = net.cfg.d_model
        # The pieces' digest pins every stored start; lane length only matters through it.
        key = dict(version=2, source=source, burn_in=self.burn_in,
                   fingerprint=self.dataset.fingerprint, thinking_steps=thinking_steps, bf16=bool(bf16),
                   pieces=hashlib.sha256(self.pieces.tobytes()).hexdigest(), count=len(self.pieces),
                   seats=seats, width=width, store="raw-rows", dtype="bfloat16" if store_bf16 else "float32")
        if cache is not None:
            cache = Path(cache)
            if cache.exists():
                saved = read_cache_key(cache)
                if saved != key:
                    raise ValueError("Window state cache belongs to another plan, checkpoint or store layout")
                self._replace(StateStore(len(self.pieces), seats, width, store_bf16,
                                         Path(str(cache) + ".states"), mode="r"), owned=False)
                self.entities = np.load(Path(str(cache) + ".entities.npy"))
                return "cache"
            cache.parent.mkdir(parents=True, exist_ok=True)
            path, owned = Path(str(cache) + ".states"), False
        elif run_dir is not None:
            path, owned = Path(run_dir) / "window-states.raw", True
            path.parent.mkdir(parents=True, exist_ok=True)
        else:
            path, owned = None, True
        if self.states is not None and self.states.path == path and path is not None:
            store = self.states
        else:
            store = StateStore(len(self.pieces), seats, width, store_bf16, path)
        store, self.entities = window_states(net, self.dataset, self.pieces, device, lanes, thinking_steps, bf16,
                                             workers, token_budget, timing_out=self.timing, store=store,
                                             prefetch=prefetch)
        self._replace(store, owned)
        if cache is not None:
            np.save(Path(str(cache) + ".entities.npy"), self.entities)
            temporary = Path(str(cache) + ".tmp")
            temporary.write_text(json.dumps(dict(key=key), sort_keys=True))
            temporary.replace(cache)
        return "computed"

    def _replace(self, store, owned):
        if self.states is not None and self.states is not store:
            self.states.close(delete=self._owned)
        self.states, self._owned = store, owned

    def close(self):
        """Release the stored states; a run-local store file is deleted."""
        if self.states is not None:
            self.states.close(delete=getattr(self, "_owned", False))
            self.states = None

    def resets(self, host, device):
        """Per frame, the lanes whose memory a stored state replaces, or None."""
        if self.states is None:
            raise RuntimeError("Window states must be computed before training")
        starts = host["piece_start"]
        mask = starts >= 0
        b, unroll = mask.shape
        position = mask.nonzero()
        values = self.states.read(starts[mask].numpy())
        dense = torch.zeros(b, unroll, M.RECURRENT_SLOTS, values.shape[-1], device=device)
        dense[position[:, 0].to(device), position[:, 1].to(device)] = values.to(device, non_blocking=True)
        rows = mask.to(device, non_blocking=True)
        present = mask.any(0).tolist()
        return [(rows[:, t], dense[:, t]) if present[t] else None for t in range(unroll)]


def read_cache_key(path):
    """The key of a version 2 window-state cache. An older cache (a Torch file with
    the full float32 table inside) is refused without loading it."""
    with open(path, "rb") as handle:
        head = handle.read(2)
    if head == b"PK":
        raise ValueError(f"{path} is an older Torch window-state cache; use a new --window-state-cache path")
    try:
        return json.loads(Path(path).read_text())["key"]
    except (UnicodeDecodeError, ValueError, KeyError) as error:
        raise ValueError(f"{path} is not a version 2 window-state cache") from error


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
