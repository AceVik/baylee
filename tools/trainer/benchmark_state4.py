"""Restore benchmark sampler lanes before reusing checkpoint memory."""
from sampler4 import StatefulBatchSampler4


def checkpoint_sampler(dataset, saved, batch_size, fresh=False, seed=None):
    if not fresh:
        if saved.get("training_sampler_version", 1) != StatefulBatchSampler4.VERSION:
            raise ValueError("Checkpoint training sampler version differs; use --fresh-trajectories")
        if saved.get("sampler_seed") is None:
            raise ValueError("Checkpoint has no sampler seed; use --fresh-trajectories --sampler-seed")
        if seed is not None and seed != saved["sampler_seed"]:
            raise ValueError("Changing sampler seed requires --fresh-trajectories")
        if batch_size != saved["batch_size"]:
            raise ValueError("Changing batch lanes requires --fresh-trajectories")
    seed = saved.get("sampler_seed") if seed is None else seed
    if seed is None:
        raise ValueError("Fresh trajectories require --sampler-seed when the checkpoint has no seed")
    sampler = StatefulBatchSampler4(dataset, batch_size, shuffle=True, seed=seed)
    sampler.epoch = saved["epoch"] - 1
    sampler.start_batch = 0 if fresh else saved["next_batch"]
    return sampler
