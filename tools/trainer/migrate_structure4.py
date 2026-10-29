"""Explicit encoder-5 -> encoder-6 warm start; never resume old hidden lanes.

Matching policy weights transfer exactly. The new semantic adapter starts at
zero, preserving the initial legacy policy, and learns with the pretrained
structure encoder on newly replayed observations. Optimizer/RNG/H are fresh.
"""
import argparse
from pathlib import Path
import torch
import model4 as M
from checkpoint4 import atomic_save
from torch_io4 import file_digest


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--policy", type=Path, required=True)
    ap.add_argument("--structure", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    if args.out.exists(): raise ValueError("Preserving existing output")
    old = torch.load(args.policy, map_location="cpu", weights_only=False)
    structure = torch.load(args.structure, map_location="cpu", weights_only=False)
    if (old.get("encoder_version"), old.get("model_version")) != (5, 2):
        raise ValueError("This migration only accepts the frozen encoder-5/model-2 contract")
    config = M.Config4(**old["config"]); config.structure_width = structure["width"]
    net = M.PlayerNet4(config); net.attach_structure(structure["structure"])
    net.rules.load_state_dict(structure["encoder"], strict=True)
    missing, extra = net.load_state_dict(old["state_dict"], strict=False)
    if extra or any(not k.startswith(("rules.", "rules_proj.")) for k in missing):
        raise ValueError(f"Unexpected policy architecture difference: missing={missing}, extra={extra}")
    with torch.no_grad(): net.rules_proj.weight.zero_()
    payload = {"config": config.to_dict(), "state_dict": net.state_dict(), "structure": structure["structure"],
               "encoder_version": M.ENCODER_VERSION, "model_version": M.MODEL_VERSION,
               "trajectory_version": M.TRAJECTORY_VERSION, "sampling_version": M.SAMPLING_VERSION,
               "provenance": {"policy": str(args.policy), "structure": str(args.structure),
                              "policy_sha256": file_digest(args.policy),
                              "structure_sha256": file_digest(args.structure),
                              "optimizer_and_memory": "fresh", "adapter": "zero initialized"}}
    atomic_save(payload, args.out / "net.pt")
    print(f"Migrated {sum(p.numel() for p in net.parameters()):,} parameters; fresh optimizer and trajectories required")

if __name__ == "__main__": main()
