"""Bounded RAM-to-CUDA lookahead on a separate copy stream.

The loader owns CPU/disk prefetch. This layer preserves its order and adds up to
three future GPU batches. Oversized or unpinned batches retain the ordinary path.
Allocator stream tracking protects retired buffers until their consumers finish;
raw VRAM addresses never need to be managed by the caller.
"""
from collections import deque
import torch


def prefetch_batches(loader, device, ahead=0, limit_bytes=128 * 1024**2):
    """Yield (host, staged_device_or_None), retaining masks for CPU planning.

    `ahead=1` provides double buffering (current plus one future batch). Retired
    allocations may remain alive until their compute stream finishes. Additional
    queued CUDA input memory is bounded by ahead * limit_bytes; active/retiring
    batches and model activations also require space. Close the iterator on an
    early stop to drain copy operations before releasing their pinned sources.
    """
    if not 0 <= ahead <= 3:
        raise ValueError("GPU lookahead must be between zero and three batches")
    if not ahead or device.type != "cuda":
        for host in loader:
            yield host, None
        return
    source = iter(loader)
    stream = torch.cuda.Stream(device=device)
    pending = deque()
    exhausted = False

    def stage():
        nonlocal exhausted
        try:
            host = next(source)
        except StopIteration:
            exhausted = True
            return
        size = sum(value.numel() * value.element_size() for value in host.values())
        if size > limit_bytes or not all(value.device.type == "cpu" and value.is_pinned()
                                        for value in host.values()):
            pending.append((host, None, None))
            return
        with torch.cuda.stream(stream):
            staged = {name: value.to(device, non_blocking=True) for name, value in host.items()}
            ready = torch.cuda.Event()
            ready.record(stream)
        pending.append((host, staged, ready))

    try:
        for _ in range(ahead + 1):
            if not exhausted:
                stage()
        while pending:
            host, staged, ready = pending.popleft()
            if ready is not None:
                consumer = torch.cuda.current_stream(device)
                consumer.wait_event(ready)
                for value in staged.values():
                    value.record_stream(consumer)
            yield host, staged
            del host, staged, ready
            if not exhausted:
                stage()
    finally:
        # Includes early break/exception. Never free a pinned source while DMA
        # still reads it. Compute lifetime is independently tracked above.
        stream.synchronize()
        pending.clear()
