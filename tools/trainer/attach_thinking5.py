"""Attaches the deliberation block (thinking revision 2) to a v5 checkpoint.

The result plays exactly as its source: the block's correction starts at zero
(`model5.deliberation.warmstart_thinking`). It is what a cost measurement or a
plumbing check exports before any training; the trainer does the same transfer
itself with `--thinking-warmstart`.

`--random-correction STD` gives the untrained block a nonzero correction so
that depths differ. That is for checking the seat's plumbing only, never a
model to play or train from, and the checkpoint says so.

Usage:
    python3 tools/trainer/attach_thinking5.py --init DIR --out DIR
"""
from __future__ import annotations

import argparse
from pathlib import Path

import torch

import model5 as M
from checkpoint4 import atomic_save
from torch_io4 import file_digest


def attach(checkpoint, steps, width, candidates, random_correction=0.0, seed=0):
    """A copy of `checkpoint` declaring thinking revision 2, with the block's weights."""
    if checkpoint.get("model_version") != 5:
        raise ValueError("The deliberation block attaches to a v5 checkpoint")
    source = M.Config5(**checkpoint["config"])
    if source.thinking_revision != 1:
        raise ValueError("The checkpoint already has a deliberation block")
    config = M.Config5(**{**source.to_dict(), "thinking_revision": 2,
                          "scratchpad_steps": source.default_thinking_steps,
                          "deliberation_steps": steps, "deliberation_width": width,
                          "deliberation_candidates": candidates})
    torch.manual_seed(seed)
    net = M.PlayerNet5(config)
    net.attach_structure(checkpoint["structure"])
    state = M.warmstart_thinking(checkpoint["state_dict"], net)
    receipt = {"kind": "weights_only", "source_config": source.to_dict(), "target_config": config.to_dict()}
    if random_correction:
        generator = torch.Generator().manual_seed(seed)
        state["deliberation.correction.weight"] = torch.randn(
            state["deliberation.correction.weight"].shape, generator=generator) * random_correction
        receipt["random_correction_std"] = random_correction
        receipt["use"] = "plumbing_measurement_only"
    kept = {key: value for key, value in checkpoint.items() if key not in ("state_dict", "config", "optimizer")}
    return {**kept, "config": config.to_dict(), "state_dict": state,
            "thinking": {"contract": "deliberation_v1"}, "thinking_warmstart": receipt}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--init", required=True, help="Source checkpoint directory (net.pt)")
    ap.add_argument("--out", required=True, help="New directory for the checkpoint with the block")
    ap.add_argument("--deliberation-steps", type=int, default=4)
    ap.add_argument("--deliberation-width", type=int, default=256)
    ap.add_argument("--deliberation-candidates", type=int, default=16)
    ap.add_argument("--random-correction", type=float, default=0.0)
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()
    source_path = Path(args.init).expanduser() / "net.pt"
    out = Path(args.out).expanduser()
    if (out / "net.pt").exists():
        raise FileExistsError("The output holds a checkpoint already")
    out.mkdir(parents=True, exist_ok=True)
    checkpoint = torch.load(source_path, map_location="cpu", weights_only=False)
    attached = attach(checkpoint, args.deliberation_steps, args.deliberation_width,
                      args.deliberation_candidates, args.random_correction, args.seed)
    attached["thinking_warmstart"]["source_checkpoint_sha256"] = file_digest(source_path)
    atomic_save(attached, out / "net.pt")
    block = sum(value.numel() for key, value in attached["state_dict"].items() if key.startswith("deliberation."))
    print(f"[attach] {out / 'net.pt'}: deliberation block {block / 1e6:.2f}M parameters, "
          f"{args.deliberation_steps} steps, width {args.deliberation_width}")


if __name__ == "__main__":
    main()
