"""One frozen pass over a dataset: the value head's inputs at every position.

Records are `[global, own seat, other seat]` of the K=0 readout, the tokens
`seat_value_head` reads in a duel, in float16. Memory advances through every
K=0 frame in float32, as the seat's own memory does.
"""
import json
import time
from pathlib import Path

import numpy as np
import torch
from torch.utils.data import DataLoader

import dataset4 as D
import model4 as M
from training4.execution import encode_boards
from .positions import FORCED, HOUSE, PRIORITY, PositionFrames, describe

RECORD = np.dtype([("game", "<u4"), ("seat", "u1"), ("turn", "<u2"), ("settled", "?"), ("active", "?"),
                   ("source", "i1"), ("pending", "i1"), ("life", "<i2"), ("opp_life", "<i2"),
                   ("hand", "i1"), ("opp_hand", "i1"), ("board", "<i2"), ("opp_board", "<i2"),
                   ("outcome", "<f4"), ("source_value", "<f4")])
TOKENS = 3


def k0_readout(net, board, memory):
    """`forward_with_seat_logits` at K=0, without the heads a position does not read."""
    k0 = torch.zeros(board.shape[0], dtype=torch.long, device=board.device)
    observed = net.observe_memory(board, memory, k0)
    return observed, net.h_norm(observed + board[:, :M.RECURRENT_SLOTS])


@torch.no_grad()
def extract(net, frames, destination, lanes=32, board_chunk=16, workers=6, max_games=0):
    """Writes `features.f16` and `positions.npy`; returns the receipt."""
    device = next(net.parameters()).device
    if any(p.dtype != torch.float32 for p in net.parameters()):
        raise ValueError("Extraction runs in float32")
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    if max_games:
        frames.trajectories = frames.trajectories[:max_games]
    net.eval().clear_structure()
    net.prepare_structure()
    loader = DataLoader(frames, batch_sampler=D.StatefulBatchSampler4(frames, lanes, shuffle=False),
                        collate_fn=D.collate_trajectories, num_workers=workers,
                        prefetch_factor=4 if workers else None)
    index = frames.store.index
    memory = None
    records, count, scanned, seen = [], 0, 0, set()
    started = time.monotonic()
    with (destination / "features.f16").open("wb") as stream:
        for batch_number, host in enumerate(loader):
            ids = host["segment_index"].tolist()
            lane_count, unroll = host["step_valid"].shape
            if memory is None:
                memory = torch.zeros(lane_count, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
            memory = torch.where(host["is_new_traj"].to(device)[:, None, None], torch.zeros_like(memory), memory)
            boards = encode_boards(net, host, board_chunk, device)
            rows = np.full((lane_count, unroll), -1, dtype=np.int64)
            for lane, segment in enumerate(ids):
                if segment >= 0:
                    span = frames.segments[segment]
                    rows[lane, :len(span)] = span
            present = rows >= 0
            source = np.where(present, frames.source[rows], FORCED)
            keep = present & (source != FORCED) & host["step_valid"].numpy()
            views = describe(host)
            keep &= ~views["lost"]
            for t in range(unroll):
                valid = host["step_valid"][:, t].to(device)
                observed, readout = k0_readout(net, boards[t][0], memory)
                memory = torch.where(valid[:, None, None], observed, memory)
                chosen = np.flatnonzero(keep[:, t])
                if not len(chosen):
                    continue
                picked = torch.as_tensor(chosen, device=device)
                tokens = readout[picked]
                seat_feat = host["seat_feat"][:, t].to(device)[picked]
                value, _ = net.seat_values(tokens[:, 1:], tokens[:, 0], seat_feat)
                stream.write(tokens[:, :TOKENS].to(torch.float16).cpu().numpy().tobytes())
                block = np.zeros(len(chosen), dtype=RECORD)
                source_rows = rows[chosen, t]
                block["game"], block["seat"] = index["game"][source_rows], index["seat"][source_rows]
                block["turn"] = host["turn"][:, t].numpy()[chosen]
                block["source"], block["pending"] = source[chosen, t], frames.pending[source_rows]
                block["active"] = views["active"][chosen, t]
                block["settled"] = views["main_quiet"][chosen, t] & (block["pending"] == PRIORITY)
                for name in ("life", "opp_life", "hand", "opp_hand", "board", "opp_board"):
                    block[name] = views[name][chosen, t]
                block["outcome"] = host["outcome"][:, t].numpy()[chosen]
                block["source_value"] = value.cpu().numpy()
                records.append(block)
                count += len(chosen)
            scanned += int(present.sum())
            seen.update(int(game) for game in index["game"][rows[present]])
            if batch_number % 200 == 0:
                seconds = time.monotonic() - started
                print(f"[value-head] batch {batch_number + 1}/{len(loader)} frames={scanned} positions={count} "
                      f"seconds={seconds:.0f} frames/s={scanned / max(seconds, 1e-3):.0f}", flush=True)
    net.clear_structure()
    positions = np.concatenate(records) if records else np.zeros(0, dtype=RECORD)
    np.save(destination / "positions.npy", positions)
    receipt = dict(positions=count, frames=scanned, games=len(seen), width=net.cfg.d_model, tokens=TOKENS,
                   seconds=time.monotonic() - started, fingerprint=frames.fingerprint,
                   house_positions=int((positions["source"] == HOUSE).sum()))
    (destination / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def open_frames(data, split, encoder_version, seats, unroll=16):
    return PositionFrames(Path(data).expanduser(), split, unroll, encoder_version, seats)


class Positions:
    """A finished extraction, read back."""

    def __init__(self, directory):
        self.path = Path(directory)
        self.receipt = json.loads((self.path / "receipt.json").read_text())
        self.rows = np.load(self.path / "positions.npy")
        shape = (len(self.rows), self.receipt["tokens"], self.receipt["width"])
        self.features = np.memmap(self.path / "features.f16", mode="r", dtype=np.float16, shape=shape)
