"""Bind learned weights to an updated public card catalog, with fresh memory.

Rule operators transfer by their names, never by traversal-dependent numeric
IDs. New combinations of known operators need no new parameters. An unknown
operator is rejected: learning new DSL semantics requires a new training phase.
Export the resulting checkpoint and rerun regressions before using it in games.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import torch

import model4 as M
from checkpoint4 import atomic_save
from model4.structure import RuleEncoder


def refresh_encoder(old: RuleEncoder, data) -> RuleEncoder:
    """Rebuild DAG buffers and map learned symbols by exact tag names."""
    old_tags, new_tags = old.data["tags"], data["tags"]
    if len(set(old_tags)) != len(old_tags) or len(set(new_tags)) != len(new_tags):
        raise ValueError("Duplicate rule operator names")
    old_ids = {name: i for i, name in enumerate(old_tags)}
    unknown = sorted(set(new_tags) - old_ids.keys())
    if unknown:
        raise ValueError(f"New rule operators require training: {unknown}")
    new = RuleEncoder(data, old.width).to(device=old.symbol.weight.device,
                                        dtype=old.symbol.weight.dtype)
    source = dict(old.named_parameters())
    with torch.no_grad():
        for name, parameter in new.named_parameters():
            if name == "symbol.weight":
                ids = torch.tensor([old_ids[tag] for tag in new_tags], device=parameter.device)
                parameter.copy_(source[name][ids])
            else:
                parameter.copy_(source[name])
    new.train(old.training)
    return new


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--model", type=Path, required=True, help="Source checkpoint directory")
    ap.add_argument("--catalog", type=Path, required=True, help="Fresh card_structure4 export")
    ap.add_argument("--out", type=Path, required=True, help="New checkpoint directory")
    args = ap.parse_args()
    if args.out.exists():
        raise ValueError("Preserving existing output; use a new directory")
    checkpoint = args.model / "net.pt"
    old = torch.load(checkpoint, map_location="cpu", weights_only=False)
    for key, expected in (("encoder_version", M.ENCODER_VERSION), ("model_version", M.MODEL_VERSION),
                          ("trajectory_version", M.TRAJECTORY_VERSION), ("sampling_version", M.SAMPLING_VERSION)):
        if old.get(key) != expected:
            raise ValueError(f"Incompatible {key}")
    data = json.loads(args.catalog.read_text())
    net = M.PlayerNet4(M.Config4(**old["config"]))
    net.attach_structure(old["structure"])
    net.load_state_dict(old["state_dict"], strict=True)
    old_digest = net.rules.digest
    net.rules = refresh_encoder(net.rules, data)
    payload = {key: old[key] for key in ("config", "encoder_version", "model_version",
                                       "trajectory_version", "sampling_version")}
    payload.update(state_dict=net.state_dict(), structure=data,
                   provenance={"source": str(checkpoint),
                               "source_sha256": hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                               "old_catalog_sha256": old_digest,
                               "new_catalog_sha256": net.rules.digest,
                               "operator_mapping": "exact tag names",
                               "optimizer_and_memory": "fresh; replay data again"})
    atomic_save(payload, args.out / "net.pt")
    print(f"Refreshed public catalog: {old_digest} -> {net.rules.digest}; re-export and verify")


if __name__ == "__main__":
    main()
