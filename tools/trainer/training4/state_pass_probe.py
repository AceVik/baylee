"""CPU probe of the window state pass's loader: shared bytes per batch and the
user's /dev/shm use, through the pass's own loader, without a network or GPU.

    CUDA_VISIBLE_DEVICES= python -m training4.state_pass_probe --data DIR \
        --lanes 32 --workers 6 --prefetch 2 --batches 300 --consume-seconds 0.5

`--consume-seconds` holds every batch as long as the GPU would, so the
workers run ahead and fill every prefetch slot. A thread samples this user's
/dev/shm quota usage; the probe stops by itself above `--shm-stop-gib`,
because the quota is shared with every other process of the same user.
`--all-fields` uses the training collate (every field) instead.
"""
from __future__ import annotations

import argparse
import json
import threading
import time
from pathlib import Path

import numpy as np
import torch

import dataset4 as D4
import model4 as M
from .memory_budget import GIB, shm_quota
from .windows import collate_boards, state_pass_loader


def padded_bytes(items):
    """Bytes `collate_trajectories` would share for `items`, every field padded
    to the widest item, and the widest candidate axis; nothing is allocated."""
    total = 0
    for name, first in items[0].items():
        shape = np.max([item[name].shape for item in items], axis=0) if first.ndim else ()
        total += len(items) * int(np.prod(shape, dtype=np.int64)) * first.dtype.itemsize
    return total, max(int(item["cand_mask"].shape[1]) for item in items)


def measured_collate(collate):
    def run(items):
        return collate(items), *padded_bytes(items)
    return run


def shared_bytes(batch):
    """Bytes of the distinct storages a worker shares for `batch`, by field."""
    seen, fields = set(), {}
    for name, tensor in batch.items():
        storage = tensor.untyped_storage()
        if storage.data_ptr() not in seen:
            seen.add(storage.data_ptr())
            fields[name] = storage.nbytes()
    return fields


class ShmWatch(threading.Thread):
    """Peak of this user's /dev/shm usage above the start, sampled every `period` seconds."""

    def __init__(self, stop_bytes, period=0.01):
        super().__init__(daemon=True)
        self.base = shm_quota()[1] or 0
        self.peak, self.stop_bytes, self.period = 0, stop_bytes, period
        self.over = threading.Event()
        self.done = threading.Event()

    def run(self):
        while not self.done.is_set():
            used = (shm_quota()[1] or 0) - self.base
            self.peak = max(self.peak, used)
            if used > self.stop_bytes:
                self.over.set()
            time.sleep(self.period)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--data", required=True)
    parser.add_argument("--encoder-version", type=int, default=10)
    parser.add_argument("--unroll", type=int, default=32)
    parser.add_argument("--max-entities", type=int, default=M.MAX_ENTITIES)
    parser.add_argument("--lanes", type=int, default=32)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--prefetch", type=int, default=2)
    parser.add_argument("--batches", type=int, default=300)
    parser.add_argument("--start-batch", type=int, default=0, help="Begin this many batches into the pass")
    parser.add_argument("--consume-seconds", type=float, default=0.,
                        help="Hold each batch this long, as the GPU would")
    parser.add_argument("--all-fields", action="store_true", help="The training collate: every field")
    parser.add_argument("--shm-stop-gib", type=float, default=8.)
    parser.add_argument("--out", default="", help="Also write the summary JSON here")
    args = parser.parse_args(argv)
    torch.set_num_threads(1)

    dataset = D4.TrajectoryDataset4(Path(args.data).expanduser(), encoder_version=args.encoder_version, split="train",
                                    unroll_len=args.unroll, max_entities=args.max_entities)
    loader = state_pass_loader(dataset, args.lanes, args.workers, args.prefetch,
                               collate=measured_collate(D4.collate_trajectories if args.all_fields else collate_boards))
    loader.batch_sampler.start_batch = args.start_batch
    watch = ShmWatch(args.shm_stop_gib * GIB)
    limit = shm_quota()[0]
    watch.start()
    sizes, full, widths, largest, stopped = [], [], [], {}, None
    began = steady = time.perf_counter()
    frames = 0
    try:
        for index, (batch, every_field, candidates) in enumerate(loader):
            if index == 0:
                steady = time.perf_counter()
            fields = shared_bytes(batch)
            total = sum(fields.values())
            sizes.append(total)
            full.append(every_field)
            if total >= max(sizes):
                largest = fields
            widths.append((batch["entity_feat"].shape[2], batch["edge_mask"].shape[2], candidates))
            frames += int(batch["step_valid"].sum())
            if args.consume_seconds:
                time.sleep(args.consume_seconds)
            del batch
            if watch.over.is_set():
                stopped = f"user /dev/shm above {args.shm_stop_gib} GiB"
                break
            if index + 1 >= args.batches:
                break
            if (index + 1) % 50 == 0:
                print(f"[probe] batches={index + 1} shm_peak_gib={watch.peak / GIB:.2f} "
                      f"batch_max_gib={max(sizes) / GIB:.3f}", flush=True)
    finally:
        seconds = time.perf_counter() - began
        del loader
        watch.done.set()
        watch.join()
    sizes = np.array(sizes, dtype=np.float64)
    full = np.array(full, dtype=np.float64)
    widths = np.array(widths)
    wide = np.argsort(-full)[:5]
    summary = dict(
        settings=dict(lanes=args.lanes, workers=args.workers, prefetch=args.prefetch, unroll=args.unroll,
                      consume_seconds=args.consume_seconds, all_fields=args.all_fields, start_batch=args.start_batch),
        batches=len(sizes), frames=frames, seconds=round(seconds, 1),
        steady_frames_per_s=round(frames / max(time.perf_counter() - steady, 1e-9), 1) if len(sizes) else 0,
        batch_gib=dict(mean=round(sizes.mean() / GIB, 4), p50=round(float(np.median(sizes)) / GIB, 4),
                       p99=round(float(np.quantile(sizes, .99)) / GIB, 4), max=round(sizes.max() / GIB, 4)),
        bytes_per_frame_max=int(sizes.max() / (args.lanes * args.unroll)),
        widths_max=dict(entities=int(widths[:, 0].max()), edges=int(widths[:, 1].max()),
                        candidates=int(widths[:, 2].max())),
        # What the training collate (every field) would have shared for the same batches.
        every_field_batch_gib=dict(p50=round(float(np.median(full)) / GIB, 4), max=round(full.max() / GIB, 3),
                                   over_1_gib=int((full > GIB).sum())),
        widest_batches=[dict(batch=args.start_batch + int(i), every_field_gib=round(full[i] / GIB, 3),
                             candidates=int(widths[i, 2]), board_gib=round(sizes[i] / GIB, 4)) for i in wide],
        largest_batch_fields_mib={k: round(v / 2 ** 20, 1) for k, v in sorted(largest.items(), key=lambda kv: -kv[1])
                                  if v >= 2 ** 20},
        shm_peak_gib=round(watch.peak / GIB, 3), shm_quota_gib=None if limit is None else round(limit / GIB, 2),
        batches_in_shm_at_peak=round(watch.peak / sizes.max(), 1), stopped=stopped)
    text = json.dumps(summary)
    print("[probe] " + text, flush=True)
    if args.out:
        Path(args.out).write_text(text + "\n")
    return summary


if __name__ == "__main__":
    main()
