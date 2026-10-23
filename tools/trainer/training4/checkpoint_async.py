"""One immutable checkpoint in flight, with atomic publication and error checks.

The v4 forward/backward reads parameters and does not update model buffers.
Call before_mutation() before the next optimizer step (or any other mutation
of CUDA tensors supplied to save). Disk I/O may then overlap later updates;
the pinned CPU snapshot no longer shares their storage. This is not a generic
wrapper for models with BatchNorm or other forward-mutated buffers.
"""
from concurrent.futures import ThreadPoolExecutor
import copy
from pathlib import Path

import torch

from checkpoint4 import atomic_save


class AsyncCheckpointWriter:
    def __init__(self, device, enabled=True):
        self.device = torch.device(device)
        self.enabled = enabled
        self._executor = None
        self._future = None
        self._stream = None
        self._copied = None
        self._closed = False

    def _freeze(self, value):
        if isinstance(value, torch.Tensor):
            if value.device.type == "cpu":
                return value.detach().clone()
            if value.device != self.device:
                raise ValueError("Checkpoint tensors must be on the writer's CUDA device or CPU")
            frozen = torch.empty_like(value, device="cpu", pin_memory=True)
            frozen.copy_(value.detach(), non_blocking=True)
            value.record_stream(self._stream)
            return frozen
        if isinstance(value, dict):
            frozen = copy.copy(value)
            for key, item in value.items():
                frozen[key] = self._freeze(item)
            if hasattr(value, "_metadata"):
                frozen._metadata = copy.deepcopy(value._metadata)
            return frozen
        if isinstance(value, list):
            return [self._freeze(item) for item in value]
        if isinstance(value, tuple):
            items = [self._freeze(item) for item in value]
            return type(value)(*items) if hasattr(value, "_fields") else tuple(items)
        return copy.deepcopy(value)

    def save(self, payload, path: Path):
        if self._closed:
            raise RuntimeError("Checkpoint writer is closed")
        if not self.enabled:
            atomic_save(payload, path)
            return
        # Backpressure bounds pinned memory to one snapshot and propagates a
        # failed previous write before another checkpoint can replace it.
        self.flush()
        copied = None
        if self.device.type == "cuda":
            if self._stream is None:
                self._stream = torch.cuda.Stream(device=self.device)
                self.device = self._stream.device
            self._stream.wait_stream(torch.cuda.current_stream(self.device))
            with torch.cuda.stream(self._stream):
                try:
                    frozen = self._freeze(payload)
                    copied = torch.cuda.Event()
                    copied.record(self._stream)
                except BaseException:
                    # Do not release partially staged pinned tensors while
                    # outstanding DMA copies still write them.
                    self._stream.synchronize()
                    raise
        else:
            frozen = self._freeze(payload)
        self._copied = copied

        def write():
            if copied is not None:
                copied.synchronize()
            atomic_save(frozen, path)

        if self._executor is None:
            self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="checkpoint4")
        self._future = self._executor.submit(write)

    def before_mutation(self):
        if self._future is not None and self._future.done():
            self._future.result()
        if self._copied is not None:
            torch.cuda.current_stream(self.device).wait_event(self._copied)
            self._copied = None

    def flush(self):
        if self._future is not None:
            self._future.result()
            self._future = None
        self._copied = None

    def close(self):
        try:
            self.flush()
        finally:
            self._closed = True
            if self._executor is not None:
                self._executor.shutdown(wait=True)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
