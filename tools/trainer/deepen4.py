"""Grow a trained v4 trunk with initially identity residual blocks.

This creates a new experiment, not an exact optimizer/trajectory resume.
Existing blocks retain their order. New attention/FFN output projections start
at zero, so the added pre-norm blocks initially compute x -> x and can learn.
"""
from __future__ import annotations

import argparse
from dataclasses import replace
import json
from pathlib import Path

import torch

from checkpoint4 import atomic_save
from torch_io4 import atomic_json, file_digest
import model4 as M


def deepen_network(source: M.PlayerNet4, layers: int, seed: int = 31421):
    """Copy a CPU float32 network into a deeper independent network."""
    old_depth = source.cfg.n_layers
    if old_depth < 1 or layers <= old_depth:
        raise ValueError("Target depth must exceed the positive source depth")
    if source.cfg.dropout != 0:
        raise ValueError("Function-preserving training transfer requires dropout=0")
    if any(p.device.type != "cpu" or p.dtype != torch.float32 for p in source.parameters()):
        raise ValueError("Load source weights on CPU in float32 before deepening")
    mapping = {i: i * layers // old_depth for i in range(old_depth)}
    inserted = sorted(set(range(layers)) - set(mapping.values()))
    # Initializing new blocks must not advance the caller's RNG state.
    with torch.random.fork_rng(devices=[]):
        # torch.manual_seed also reseeds CUDA/MPS/XPU. Construction is CPU-only;
        # leave those generators (including deferred seeds) untouched.
        torch.random.default_generator.manual_seed(seed)
        deeper = M.PlayerNet4(replace(source.cfg, n_layers=layers))
        deeper.attach_structure(source.rules.data)
    state = deeper.state_dict()
    for key, value in source.state_dict().items():
        target = key
        if key.startswith("trunk_layers."):
            _, index, suffix = key.split(".", 2)
            target = f"trunk_layers.{mapping[int(index)]}.{suffix}"
        state[target] = value
    deeper.load_state_dict(state, strict=True)
    with torch.no_grad():
        for index in inserted:
            layer = deeper.trunk_layers[index]
            for projection in (layer.attn.out_proj, layer.ff[3]):
                projection.weight.zero_()
                projection.bias.zero_()
    deeper.train(source.training)
    return deeper, {"source_layers": old_depth, "target_layers": layers,
                    "copied_layer_map": mapping, "inserted_layers": inserted,
                    "initialization": "zero attention and FFN output projections; identity residual blocks",
                    "seed": seed,
                    "source_parameters": sum(p.numel() for p in source.parameters()),
                    "target_parameters": sum(p.numel() for p in deeper.parameters())}


def deepen_checkpoint(checkpoint: Path, output: Path, layers: int, seed: int = 31421):
    if output.exists():
        raise FileExistsError("Preserving existing output; use a new directory")
    checkpoint = checkpoint / "net.pt" if checkpoint.is_dir() else checkpoint
    saved = torch.load(checkpoint, map_location="cpu", weights_only=False)
    contracts = {"encoder_version": M.ENCODER_VERSION, "model_version": M.MODEL_VERSION,
                 "trajectory_version": M.TRAJECTORY_VERSION, "sampling_version": M.SAMPLING_VERSION}
    for name, expected in contracts.items():
        if saved.get(name) != expected:
            raise ValueError(f"Incompatible {name}")
    source = M.PlayerNet4(M.Config4(**saved["config"]))
    source.attach_structure(saved["structure"])
    source.load_state_dict(saved["state_dict"], strict=True)
    if not all(torch.isfinite(p).all().item() for p in source.parameters()):
        raise ValueError("Source contains nonfinite parameters")
    deeper, report = deepen_network(source, layers, seed)
    source_digest = file_digest(checkpoint)
    report.update(source=str(checkpoint.resolve()), source_sha256=source_digest,
                  structure_sha256=deeper.rules.digest,
                  optimizer_and_memory="fresh; use --init, not --resume or --init-state",
                  source_provenance=saved.get("provenance"))
    output.mkdir(parents=True, exist_ok=False)
    atomic_save({**contracts, "config": deeper.cfg.to_dict(), "structure": saved["structure"],
                 "state_dict": deeper.state_dict(), "provenance": report}, output / "net.pt")
    atomic_json(output / "depth-transfer.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--layers", type=int, default=12)
    parser.add_argument("--seed", type=int, default=31421)
    args = parser.parse_args()
    torch.set_num_threads(4)
    print(json.dumps(deepen_checkpoint(args.model.expanduser(), args.out.expanduser(), args.layers, args.seed), indent=2))


if __name__ == "__main__":
    main()
