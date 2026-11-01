"""Continue learned parameters and Adam state at a new package boundary.

Trajectory lanes, sampler position and validation-best selection are deliberately
fresh. This is not --resume: that path still requires the identical dataset.
"""
from pathlib import Path
import torch
import model4 as M
from model4.structure import structure_digest
from checkpoint4 import move_optimizer


def load_package_state(path, net, optimizer, device):
    saved = torch.load(Path(path), map_location="cpu", weights_only=False)
    for key, expected in (("encoder_version", M.ENCODER_VERSION), ("model_version", M.MODEL_VERSION),
                          ("trajectory_version", M.TRAJECTORY_VERSION), ("sampling_version", M.SAMPLING_VERSION)):
        if saved.get(key) != expected:
            raise ValueError(f"Package checkpoint has incompatible {key}")
    if M.Config4(**saved["config"]).to_dict() != net.cfg.to_dict() or structure_digest(saved["structure"]) != net.rules.digest:
        raise ValueError("Package checkpoint architecture/rule catalog differs")
    options = [{k: group[k] for k in ("lr", "weight_decay", "fused")} for group in optimizer.param_groups]
    net.load_state_dict(saved["state_dict"], strict=True)
    optimizer.load_state_dict(saved["optimizer"])
    for group, values in zip(optimizer.param_groups, options):
        group.update(values)
    move_optimizer(optimizer, device)
    return {"checkpoint": str(path), "parent_step_num": saved["step_num"],
            "parent_dataset_fingerprint": saved["dataset_fingerprint"],
            "memory_and_sampler": "reset for new complete-game package", "optimizer": "preserved"}
