"""Synchronized PyTorch measurements of the production history scheduler."""

from contextlib import contextmanager
from dataclasses import dataclass
import time

import torch

from policy_math4 import action_logprob_and_kl
from recurrent_ppo4.collection import stream_collection_batches
from recurrent_ppo4.optimization import behavior_divergence


class PhaseTimer:
    def __init__(self, device):
        self.device = torch.device(device)
        self.seconds = {}

    def synchronize(self):
        if self.device.type == "cuda":
            torch.cuda.synchronize(self.device)

    @contextmanager
    def scope(self, name):
        self.synchronize()
        started = time.perf_counter()
        try:
            yield
        finally:
            self.synchronize()
            self.seconds[name] = self.seconds.get(name, 0.) + time.perf_counter() - started


@dataclass
class Snapshot:
    actions: dict
    frames: dict
    report: dict


def frame_identities(dataset):
    """Bind tensor rows to native immutable (game, seat, cursor) identities."""
    identities = {}
    seen = set()
    for history in dataset.trajectories:
        for segment in history:
            for t, index in enumerate(dataset.segments[segment]):
                row = dataset.store.index[int(index)]
                key = (int(row['game']), int(row['seat']), int(row['cursor']))
                if key in seen:
                    raise ValueError("Duplicate admitted history frame identity")
                seen.add(key)
                identities[segment, t] = key
    return identities


@torch.no_grad()
def measure_pass(net, dataset, prefixes, identities, device, lanes, board_chunk):
    """Capture every valid H/value and actor probability without changing weights.

    Whole-pass time includes instrumentation, CPU parity copies and synchronization;
    it is not a production-throughput estimate. Phase sums exclude scheduler gaps.
    """
    timer = PhaseTimer(device)
    actions, frames = {}, {}
    total, slots = 0., 0
    if timer.device.type == "cuda":
        torch.cuda.reset_peak_memory_stats(timer.device)
    timer.synchronize()
    started = time.perf_counter()
    try:
        for indices, batch, outputs in stream_collection_batches(
                net, dataset, prefixes, device, lanes,
                board_chunk=board_chunk, phase=timer.scope):
            slots += batch['step_valid'].numel()
            for t, output in enumerate(outputs):
                with timer.scope("likelihood_and_divergence"):
                    actor = batch['actor_mask'][:, t] & batch['step_valid'][:, t]
                    mask = actor.to(device).bool()
                    lp, _ = action_logprob_and_kl(*output[1:4], *output[1:4], batch, t)
                    current = lp[mask]
                    behavior = batch['behavior_lp'][:, t].to(device)[mask]
                    # Same FP64 estimate, per-timestep sum and Python accumulation
                    # as production collection_divergence; no KL subsampling.
                    total += float(behavior_divergence(current, behavior))
                with timer.scope("parity_capture"):
                    logprob = current.cpu().tolist()
                    recorded = behavior.cpu().tolist()
                    hidden = output[0].float().cpu()
                    value = output[4].float().cpu()
                    if not bool(torch.isfinite(hidden).all() & torch.isfinite(value).all()):
                        raise FloatingPointError("Nonfinite recurrent audit output")
                    chosen = 0
                    for lane, segment in enumerate(indices):
                        if not bool(batch['step_valid'][lane, t]):
                            continue
                        key = identities[segment, t]
                        if key in frames:
                            raise ValueError("Repeated complete-history frame")
                        frames[key] = (hidden[lane].clone(), float(value[lane]))
                        if bool(actor[lane]):
                            actions[key] = (logprob[chosen], recorded[chosen])
                            chosen += 1
                    if chosen != len(logprob):
                        raise ValueError("Actor identity and tensor counts disagree")
    finally:
        net.clear_structure()
        timer.synchronize()
    if not actions:
        raise ValueError("No admitted policy decisions")
    report = dict(wall_seconds=time.perf_counter() - started,
                  phase_seconds=timer.seconds, choices=len(actions), frames=len(frames),
                  padded_frames=slots-len(frames), allocated_slots=slots,
                  collection_behavior_kl=total/len(actions))
    if timer.device.type == "cuda":
        stats = torch.cuda.memory_stats(timer.device)
        report.update(cuda_peak_allocated_bytes=torch.cuda.max_memory_allocated(timer.device),
                      cuda_peak_reserved_bytes=torch.cuda.max_memory_reserved(timer.device),
                      cuda_allocator_retries=stats.get('num_alloc_retries', 0))
        if report['cuda_allocator_retries']:
            raise RuntimeError("CUDA allocator retry; probe does not change shape automatically")
    return Snapshot(actions, frames, report)
