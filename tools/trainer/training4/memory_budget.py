"""Host-memory budget of the decision-window learner, checked before the state pass.

The limit is an explicit `--memory-limit-gib`, else the smallest numeric
`memory.max` on the process's cgroup v2 path (a job inside a unit sees both),
else MemTotal. The estimate adds the holders that grow with the data. A state
store in a file is read and written with positional I/O: its pages sit in the
reclaimable page cache, outside every process's resident set (the train_ops
guard limits the group's PSS), so it is checked against free disk instead.
"""
from __future__ import annotations

import os
from pathlib import Path
import resource
import shutil

GIB = 1024 ** 3
# Calibrated on train_ops' peak PSS of the gen4 window runs: the state pass
# (6 workers, prefetch 4, 32 x 32 frames per batch, two copies) added 13 GiB
# over the same run reading its cache, about 280 KiB per frame in flight.
FRAME_BYTES = 288 * 1024
PROCESS_BASE = 4 * GIB      # interpreter, torch, CUDA context, model copies on the host side
WORKER_BASE = GIB // 2      # private pages of one forked loader worker


def cgroup_limit(cgroup_file="/proc/self/cgroup", root="/sys/fs/cgroup"):
    """Smallest numeric memory.max from the process's cgroup up to the root, or None."""
    try:
        lines = Path(cgroup_file).read_text().splitlines()
    except OSError:
        return None
    path = next((line.split("::", 1)[1] for line in lines if line.startswith("0::")), None)
    if path is None:
        return None
    limits = []
    current = Path(root) / path.lstrip("/")
    while True:
        try:
            value = (current / "memory.max").read_text().strip()
            if value != "max":
                limits.append(int(value))
        except (OSError, ValueError):
            pass
        if current == Path(root) or current.parent == current:
            break
        current = current.parent
    return min(limits) if limits else None


def mem_total(meminfo="/proc/meminfo"):
    for line in Path(meminfo).read_text().splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    raise ValueError("No MemTotal in /proc/meminfo")


def memory_limit(explicit_gib=0., cgroup_file="/proc/self/cgroup", root="/sys/fs/cgroup", meminfo="/proc/meminfo"):
    """(bytes, source) of the memory this process may use. A flag (such as the
    train_ops guard's PSS limit, which no cgroup shows) can only tighten the cgroup's."""
    limit = cgroup_limit(cgroup_file, root)
    if explicit_gib > 0 and (limit is None or int(explicit_gib * GIB) < limit):
        return int(explicit_gib * GIB), "flag"
    if limit is not None:
        return limit, "cgroup"
    return mem_total(meminfo), "MemTotal"


def window_estimate(frames, pieces, store_bytes, store_in_ram, state_workers, state_lanes, unroll,
                    train_workers, batch, length, prefetch=2):
    """Bytes of the large host holders of a window run, by holder."""
    return dict(
        state_store=store_bytes if store_in_ram else 0,
        plan_arrays=frames * (8 + 1 + 4) + pieces * (12 + 8 + 8 + 4),
        sampler_lists=pieces * 120,
        state_pass_batches=state_workers * prefetch * 2 * state_lanes * unroll * FRAME_BYTES,
        training_batches=max(train_workers, 1) * prefetch * 2 * batch * length * FRAME_BYTES,
        processes=PROCESS_BASE + (state_workers + train_workers + 2) * WORKER_BASE)


def check(estimate, limit, fraction, stores=()):
    """Refuse a run whose estimate exceeds `fraction` of the limit, or whose
    store files, `(directory, bytes)` pairs, exceed 0.9 of a
    filesystem's free space (stores on one filesystem add up)."""
    total = sum(estimate.values())
    if total > fraction * limit:
        raise MemoryError(
            f"Estimated host memory {total / GIB:.1f} GiB exceeds {fraction:g} of the {limit / GIB:.1f} GiB "
            f"limit: " + ", ".join(f"{k} {v / GIB:.2f} GiB" for k, v in estimate.items() if v))
    needs = {}
    for store_dir, size in stores:
        directory = Path(store_dir)
        while not directory.exists():
            directory = directory.parent
        device = directory.stat().st_dev
        needs[device] = (needs.get(device, (0, directory))[0] + size, directory)
    for size, directory in needs.values():
        free = shutil.disk_usage(directory).free
        if size > 0.9 * free:
            raise MemoryError(f"The window-state store needs {size / GIB:.1f} GiB; "
                              f"{directory} has {free / GIB:.1f} GiB free")
    return total


def peak_memory(status="/proc/self/status", cgroup_file="/proc/self/cgroup", root="/sys/fs/cgroup"):
    """Peak memory so far: this process's VmHWM and ru_maxrss, finished children's
    ru_maxrss, and the cgroup's memory.peak where the kernel keeps one (GiB)."""
    hwm = None
    try:
        for line in Path(status).read_text().splitlines():
            if line.startswith("VmHWM:"):
                hwm = int(line.split()[1]) * 1024 / GIB
    except OSError:
        pass
    cgroup = None
    try:
        path = next(line.split("::", 1)[1] for line in Path(cgroup_file).read_text().splitlines()
                    if line.startswith("0::"))
        cgroup = int((Path(root) / path.lstrip("/") / "memory.peak").read_text().split()[0]) / GIB
    except (OSError, StopIteration, ValueError):
        pass
    return dict(vm_hwm_gib=hwm,
                self_maxrss_gib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024 / GIB,
                children_maxrss_gib=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss * 1024 / GIB,
                cgroup_peak_gib=cgroup, pid=os.getpid())
