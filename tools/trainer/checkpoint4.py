"""Atomic local checkpoints including optimizer, data cursor and RNG state."""
from __future__ import annotations

from pathlib import Path
import random

import numpy as np
import torch
from tensor_io4 import TensorPublication


def atomic_save(payload: dict, path: Path) -> None:
    with TensorPublication('file',path) as publication:
        torch.save(payload,publication.path)
        publication.commit()


def rng_state() -> dict:
    return {"torch": torch.get_rng_state(), "numpy": np.random.get_state(),
            "python": random.getstate(),
            "cuda": torch.cuda.get_rng_state_all() if torch.cuda.is_available() else []}


def restore_rng(state: dict) -> None:
    torch.set_rng_state(state["torch"].cpu())
    np.random.set_state(state["numpy"])
    random.setstate(state["python"])
    if state["cuda"] and torch.cuda.is_available():
        torch.cuda.set_rng_state_all([s.cpu() for s in state["cuda"]])


def move_optimizer(optimizer, device) -> None:
    # Ordinary AdamW reads its scalar step counters on the CPU. Moving those
    # to CUDA on resume adds a device synchronization per parameter; fused or
    # capturable optimizers, conversely, need the counters on the device.
    for group in optimizer.param_groups:
        device_steps = group.get("fused", False) or group.get("capturable", False)
        for parameter in group["params"]:
            for key, value in optimizer.state.get(parameter, {}).items():
                if isinstance(value, torch.Tensor):
                    destination = device if key != "step" or device_steps else "cpu"
                    optimizer.state[parameter][key] = value.to(destination)
