"""Bind learned weights to an updated public card catalog, with fresh memory.

Rule operators transfer by their names, never by traversal-dependent numeric
IDs. New combinations of known operators need no new parameters. An unknown
operator is rejected: learning new DSL semantics requires a new training phase.
Export the resulting checkpoint and rerun regressions before using it in games.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch

import model4 as M
from checkpoint4 import atomic_save
from torch_io4 import atomic_json, file_digest
from export_onnx4 import load_export_checkpoint
from model4.structure import RuleEncoder


def _donor_ids_for_tag(tag: str, old_ids: dict[str, int]) -> list[int]:
    if tag in old_ids:
        return [old_ids[tag]]
    base = tag.split(".")[0]
    if base in old_ids:
        return [old_ids[base]]
    prefix = base.split("::")[0] + "::"
    candidates = [idx for name, idx in old_ids.items() if name.startswith(prefix)]
    if candidates:
        return candidates
    return list(old_ids.values())


def refresh_encoder(old: RuleEncoder, data, *, allow_new_operators: bool = False) -> RuleEncoder:
    """Rebuild DAG buffers and map learned symbols by exact tag names."""
    old_tags, new_tags = old.data["tags"], data["tags"]
    if len(set(old_tags)) != len(old_tags) or len(set(new_tags)) != len(new_tags):
        raise ValueError("Duplicate rule operator names")
    old_ids = {name: i for i, name in enumerate(old_tags)}
    unknown = sorted(set(new_tags) - old_ids.keys())
    if unknown and not allow_new_operators:
        raise ValueError(f"New rule operators require training: {unknown}")
    new = RuleEncoder(data, old.width).to(device=old.symbol.weight.device,
                                        dtype=old.symbol.weight.dtype)
    source = dict(old.named_parameters())
    with torch.no_grad():
        for name, parameter in new.named_parameters():
            if name == "symbol.weight":
                src_w = source[name]
                for i, tag in enumerate(new_tags):
                    if tag in old_ids:
                        parameter[i].copy_(src_w[old_ids[tag]])
                    else:
                        donors = _donor_ids_for_tag(tag, old_ids)
                        parameter[i].copy_(src_w[donors].mean(dim=0))
            else:
                parameter.copy_(source[name])
    new.train(old.training)
    return new


def refresh_checkpoint(model, catalog, out, *, allow_new_operators=False):
    """Create a fresh catalog binding; never overwrite or mutate the source."""
    model, catalog, out = map(Path, (model, catalog, out))
    if out.exists() or out.is_symlink():
        raise ValueError("Preserving existing output; use a new directory")
    checkpoint = model / "net.pt"
    metadata_path = model / "meta.json"
    sources = [checkpoint, catalog] + ([metadata_path] if metadata_path.exists() else [])
    source_hashes = {str(path): file_digest(path) for path in sources}
    # The shared loader validates explicit v5 architecture/revision and strict
    # parameters. Keep the pre-existing v4 encoder-6 upgrade as an explicit case.
    header = torch.load(checkpoint, map_location="cpu", weights_only=False)
    if header.get("encoder_version") == 6 and header.get("model_version") == M.MODEL_VERSION:
        for key, expected in (("trajectory_version", M.TRAJECTORY_VERSION),
                              ("sampling_version", M.SAMPLING_VERSION)):
            if header.get(key) != expected:
                raise ValueError(f"Incompatible {key}")
        old = header
        net = M.PlayerNet4(M.Config4(**old["config"])).eval()
        net.attach_structure(old["structure"])
        net.load_state_dict(M.upgrade_state_dict_to_v7(old["state_dict"], net), strict=True)
    else:
        del header
        old, net, _module = load_export_checkpoint(checkpoint)
    data = json.loads(catalog.read_text())
    old_digest = net.rules.digest
    old_ids = {tag: i for i, tag in enumerate(net.rules.data["tags"])}
    unknown = sorted(set(data["tags"]) - old_ids.keys())
    donors = {tag: [net.rules.data["tags"][i] for i in _donor_ids_for_tag(tag, old_ids)]
              for tag in unknown}
    net.rules = refresh_encoder(net.rules, data, allow_new_operators=allow_new_operators)
    lineage = dict(source=str(checkpoint.resolve()), source_sha256=source_hashes[str(checkpoint)],
        catalog_file_sha256=source_hashes[str(catalog)],
        source_encoder_version=old["encoder_version"], old_catalog_sha256=old_digest,
        new_catalog_sha256=net.rules.digest,
        operator_mapping=("exact shared tag names; donor-initialized new tags" if unknown else "exact tag names"),
        initialized_new_tags=unknown, new_tag_donors=donors,
        requires_new_operator_training=bool(unknown),
        optimizer_and_memory="fresh; replay data again",
        inherited_validation_metrics_scope="source catalog; refreshed catalog requires new validation")
    # Preserve training selection, config, architecture and pre-existing provenance.
    # A catalog refresh is a new lineage event, not a replacement training history.
    payload = {**old, "encoder_version": M.ENCODER_VERSION, "structure": data,
               "state_dict": net.state_dict(), "structure_sha256": net.rules.digest,
               "structure_version": 1, "catalog_refresh": lineage}
    lineage["discarded_runtime_state"] = [key for key in ("optimizer", "rng", "h") if key in payload]
    for key in lineage["discarded_runtime_state"]:
        payload.pop(key)
    if "catalog_refresh" in old:
        lineage["previous_refresh"] = old["catalog_refresh"]
    metadata = json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
    for key in ("config", "architecture", "model_family", "model_version", "encoder_version",
                "trajectory_version", "sampling_version", "training_loss_version", "checkpoint_selection",
                "step_num", "val_metrics", "validation_fingerprint", "dataset_fingerprint",
                "optimizer_schedule", "provenance"):
        if key in payload:
            metadata[key] = payload[key]
    metadata.update(structure_version=1, structure_sha256=net.rules.digest, catalog_refresh=lineage)
    for path, expected in source_hashes.items():
        if file_digest(path) != expected:
            raise ValueError("Refresh source changed: " + path)
    out.mkdir(parents=True, exist_ok=False)
    atomic_save(payload, out / "net.pt")
    atomic_json(out / "meta.json", metadata)
    return lineage


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--model", type=Path, required=True, help="Source checkpoint directory")
    ap.add_argument("--catalog", type=Path, required=True, help="Fresh card_structure4 export")
    ap.add_argument("--out", type=Path, required=True, help="New checkpoint directory")
    ap.add_argument("--allow-new-operators", action="store_true",
                    help="Warm-start new rule operators from namespace donors for subsequent fine-tuning")
    args = ap.parse_args()
    lineage = refresh_checkpoint(args.model, args.catalog, args.out,
                                 allow_new_operators=args.allow_new_operators)
    print(f"Refreshed public catalog: {lineage['old_catalog_sha256']} -> "
          f"{lineage['new_catalog_sha256']}; re-export and verify")


if __name__ == "__main__":
    main()
