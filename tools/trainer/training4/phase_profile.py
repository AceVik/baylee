"""Opt-in per-phase timing of the training step (`--phase-profile PATH`).

Off by default: the driver then holds a `NullProfile` whose methods do nothing,
so the default step issues the same kernels in the same order.

With `--phase-profile-sync` every phase boundary waits for the GPU, so a phase's
time is the time to finish its work; without it a phase's time is the host's
time to issue it (Python, autograd and launches), and the GPU catches up at the
step's own synchronizing checks. Steps before `warmup` are excluded. GPU busy is
NVML's utilization (fraction of time a kernel ran), sampled by `nvidia-smi`.
"""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import threading
import time

import torch


class NullProfile:
    enabled = False

    def wrap(self, iterator):
        return iterator

    def begin_step(self, step, host):
        pass

    def mark(self, name):
        pass

    def end_step(self, **extra):
        pass

    def finish(self, **extra):
        pass


class _Utilization:
    """`nvidia-smi` utilization samples (percent) while the measured window runs."""

    def __init__(self, interval_ms=100):
        self.samples = []
        self._process = None
        self._thread = None
        self._interval_ms = interval_ms

    def start(self):
        try:
            self._process = subprocess.Popen(
                ["nvidia-smi", "--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader,nounits",
                 f"-lms={self._interval_ms}"], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        except OSError:
            self._process = None
            return
        def read():
            for line in self._process.stdout:
                parts = line.strip().split(",")
                if len(parts) == 2:
                    try:
                        self.samples.append((float(parts[0]), float(parts[1])))
                    except ValueError:
                        pass
        self._thread = threading.Thread(target=read, daemon=True)
        self._thread.start()

    def stop(self):
        if self._process is not None:
            self._process.terminate()
            self._process.wait(timeout=10)
            self._thread.join(timeout=10)


class PhaseProfile:
    enabled = True

    def __init__(self, path, warmup, measured, sync, device, frames_per_game=None, decisions_per_game=None,
                 card_naming_candidates=1024):
        if warmup < 0 or measured < 1:
            raise ValueError("Phase profile needs nonnegative warmup and positive measured steps")
        self.path = Path(path)
        if self.path.exists():
            raise FileExistsError(self.path)
        self.warmup, self.measured, self.sync, self.device = warmup, measured, sync, device
        self.frames_per_game, self.decisions_per_game = frames_per_game, decisions_per_game
        self.card_naming_candidates = card_naming_candidates
        self.steps = []
        self._index = 0
        self._current = None
        self._last = None
        self._pending_wait = 0.0
        self._window_start = None
        self._window_end = None
        self._utilization = _Utilization()

    @property
    def active(self):
        return self.warmup <= self._index < self.warmup + self.measured

    def _now(self):
        if self.sync and self.device.type == "cuda":
            torch.cuda.synchronize(self.device)
        return time.perf_counter()

    def wrap(self, iterator):
        iterator = iter(iterator)
        while True:
            begin = self._now()
            try:
                item = next(iterator)
            except StopIteration:
                return
            self._pending_wait = time.perf_counter() - begin
            yield item

    def begin_step(self, step, host):
        if self._index == self.warmup:
            if self.device.type == "cuda":
                torch.cuda.synchronize(self.device)
                torch.cuda.reset_peak_memory_stats(self.device)
            if self.device.type == "cuda":
                self._utilization.start()
            self._window_start = time.perf_counter() - self._pending_wait
        valid = host["step_valid"]
        decisions = valid & host["is_decision"]
        self._current = dict(step=step, data_wait=self._pending_wait, phases={},
                             valid_frames=int(valid.sum()), decisions=int(decisions.sum()),
                             lanes=int(valid.shape[0]), unroll=int(valid.shape[1]),
                             entities=int(host["entity_feat"].shape[2]),
                             candidates=int(host["cand_mask"].shape[2]),
                             pairs=int(host["pair_mask"].shape[2]))
        self._current["card_naming"] = self._current["candidates"] >= self.card_naming_candidates
        self._last = self._now()

    def mark(self, name):
        if self._current is None:
            return
        now = self._now()
        phases = self._current["phases"]
        phases[name] = phases.get(name, 0.0) + now - self._last
        self._last = now

    def end_step(self, **extra):
        if self._current is None:
            return
        self.mark("bookkeeping")
        self._current.update(extra)
        self._current["step_time"] = self._current["data_wait"] + sum(self._current["phases"].values())
        if self.active:
            self.steps.append(self._current)
            if self._index == self.warmup + self.measured - 1:
                if self.device.type == "cuda":
                    torch.cuda.synchronize(self.device)
                self._window_end = time.perf_counter()
                self._utilization.stop()
                self.peak_allocated = torch.cuda.max_memory_allocated(self.device) if self.device.type == "cuda" else 0
                self.peak_reserved = torch.cuda.max_memory_reserved(self.device) if self.device.type == "cuda" else 0
        self._current = None
        self._index += 1

    def summary(self):
        steps = self.steps
        wall = (self._window_end or time.perf_counter()) - (self._window_start or time.perf_counter())
        names = sorted({name for step in steps for name in step["phases"]})
        totals = {name: sum(step["phases"].get(name, 0.0) for step in steps) for name in names}
        totals["data_wait"] = sum(step["data_wait"] for step in steps)
        accounted = sum(totals.values())
        frames = sum(step["valid_frames"] for step in steps)
        decisions = sum(step["decisions"] for step in steps)
        naming = [step for step in steps if step["card_naming"]]
        util = [u for u, _ in self._utilization.samples]
        result = dict(
            schema=1, sync=self.sync, warmup_steps=self.warmup, measured_steps=len(steps), wall_s=wall,
            accounted_s=accounted, steps_per_s=len(steps) / wall if wall > 0 else 0.,
            valid_frames=frames, decisions=decisions,
            frames_per_s=frames / wall if wall > 0 else 0., decisions_per_s=decisions / wall if wall > 0 else 0.,
            phase_s=totals, phase_share={k: v / accounted for k, v in totals.items()} if accounted else {},
            phase_ms_per_step={k: 1000 * v / max(len(steps), 1) for k, v in totals.items()},
            card_naming_steps=len(naming),
            card_naming_time_share=(sum(s["step_time"] for s in naming) / accounted) if accounted else 0.,
            card_naming_step_time_s=[s["step_time"] for s in naming],
            card_naming_candidates_threshold=self.card_naming_candidates,
            gpu_utilization_mean=float(sum(util) / len(util)) if util else None,
            gpu_utilization_samples=len(util),
            gpu_memory_used_mib_max=max((m for _, m in self._utilization.samples), default=None),
            peak_allocated_gib=getattr(self, "peak_allocated", 0) / 2**30,
            peak_reserved_gib=getattr(self, "peak_reserved", 0) / 2**30,
            mean_entities=sum(s["entities"] for s in steps) / max(len(steps), 1),
            mean_candidates=sum(s["candidates"] for s in steps) / max(len(steps), 1),
        )
        if self.frames_per_game:
            result["frames_per_game"] = self.frames_per_game
        if self.decisions_per_game:
            result["decisions_per_game"] = self.decisions_per_game
            result["games_per_hour"] = result["decisions_per_s"] * 3600 / self.decisions_per_game
        return result

    def finish(self, **extra):
        self._utilization.stop()
        if not self.steps:
            return
        payload = dict(self.summary(), **extra, steps=self.steps)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        temporary.write_text(json.dumps(payload, indent=1, sort_keys=True))
        temporary.replace(self.path)
        print("[phase-profile] " + json.dumps({k: v for k, v in payload.items()
                                               if k not in ("steps", "card_naming_step_time_s")}), flush=True)
