"""Command-line orchestration; caches are separate from immutable checkpoints."""
import argparse
import json
from pathlib import Path
import torch
import dataset4 as D
from .checkpoint import collection_plan, file_digest, load_source, merged_checkpoint, publish
from .extract import cached_features
from .fit import fit
from .metrics import assert_disjoint_games
from .model_identity import source_identity


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True)
    parser.add_argument("--data", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--batch-size", type=int, default=8, help="Chronological feature-extraction lanes")
    parser.add_argument("--unroll", type=int, default=16)
    parser.add_argument("--sample-every", type=int, default=64)
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--lr", type=float, default=1e-4)
    parser.add_argument("--seed", type=int, default=31431)
    parser.add_argument("--head-batch-size", type=int, default=256)
    parser.add_argument("--board-chunk", type=int, default=4)
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    parser.add_argument("--cache-dir", default="~/baylee-data/v4/value-calibration-cache")
    parser.add_argument("--max-samples", type=int, default=200000)
    parser.add_argument("--collection-plan", help="JSON whose model_sha256 must match the source checkpoint")
    args = parser.parse_args()
    if min(args.batch_size, args.unroll, args.sample_every, args.head_batch_size, args.max_samples) < 1 or args.epochs < 0 or args.lr <= 0 or args.board_chunk < 0:
        parser.error("Invalid calibration size, epoch count or learning rate")
    output = Path(args.out).expanduser().resolve()
    if output.exists() or output.is_symlink():
        raise FileExistsError("Preserving existing output; use a new directory")
    cache_root = Path(args.cache_dir).expanduser().resolve()
    repo = Path(__file__).resolve().parents[3]
    if cache_root.is_relative_to(repo):
        raise ValueError("Calibration caches must live outside the repository")
    torch.set_num_threads(4)
    torch.manual_seed(args.seed)
    source_path, saved, net = load_source(args.model)
    source_hash = file_digest(source_path)
    plan = collection_plan(args.collection_plan, source_hash)
    device = torch.device(args.device)
    net.to(device)
    datasets = [D.TrajectoryDataset4(Path(args.data).expanduser(), split=split, unroll_len=args.unroll,
                                    encoder_version=saved['encoder_version'])
                for split in ("train", "val")]
    if not all(len(ds) for ds in datasets):
        raise ValueError("Separate nonempty training and validation games are required")
    assert_disjoint_games(*datasets)
    for dataset in datasets:
        if dataset.structure.digest != net.rules.digest:
            raise ValueError("Dataset and source use different rule catalogs")
    caches = [cached_features(net, ds, cache_root, source_hash, args.sample_every,
                              args.batch_size, args.board_chunk, args.max_samples) for ds in datasets]
    head, report = fit(net.seat_value_head, *caches, device, args.epochs, args.lr,
                       args.seed, args.head_batch_size)
    if file_digest(source_path) != source_hash:
        raise RuntimeError("Source checkpoint changed during calibration")
    provenance = dict(kind="isolated_value_calibration", source=str(source_path.resolve()),
                      source_sha256=source_hash, source_identity=source_identity(saved), source_catalog_sha256=net.rules.digest, seed=args.seed, promoted=False,
                      train_fingerprint=datasets[0].fingerprint, validation_fingerprint=datasets[1].fingerprint,
                      caches=[str(cache.path) for cache in caches], selected_epoch=report["selected_epoch"],
                      collection_plan=plan,
                      frozen="Every tensor except seat_value_head.* bit-identical; no policy, trunk, memory or rule updates",
                      optimizer_and_memory="fresh experiment; optimizer/H/old validation metrics omitted")
    report.update(provenance=provenance, settings=vars(args),
                  cache_metadata=[cache.meta for cache in caches],
                  caveat="Metrics describe stride-sampled learner-trajectory states, not every original frame; turn>=20 is clipped; no automatic promotion")
    publish(output, merged_checkpoint(saved, head, provenance), report)
    print(json.dumps(dict(output=str(output), selected_epoch=report["selected_epoch"],
                          validation=report["validation"]["all"]), indent=2))
