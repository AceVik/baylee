"""Collect held-out multiplayer/web games and reconvert an immutable broad corpus.

The old dataset and running experiment stay untouched. Capped games are
preserved as findings; only normally completed games enter conversion.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path

from run_guard4 import run_job, atomic_json


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--base-data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--web-manifest", required=True)
    ap.add_argument("--extra-run", action="append", default=[])
    args = ap.parse_args()
    root = Path(args.out).resolve()
    root.mkdir(parents=True, exist_ok=True)
    meta = json.loads((Path(args.base_data) / "meta.json").read_text())
    train, val = list(meta["source_runs"]) + args.extra_run, list(meta["validation_runs"])
    for number, (label, seats, games, shape, split, manifest) in enumerate([
        ("web-train", 2, 48, "constructed", "train", args.web_manifest),
        ("three-val", 3, 12, "constructed", "val", args.manifest),
        ("six-val", 6, 12, "commander", "val", args.manifest),
        ("eight-val", 8, 16, "commander", "val", args.manifest),
    ]):
        run = root / "runs" / label
        command = ["target/release/selfplay", "--decks-manifest", manifest,
                   "--deck-split", split, "--shapes", shape, "--seats", str(seats),
                   "--games", str(games), "--first-seed", str(895001 + number * 1000),
                   "--threads", "2", "--profiles", "steady,sharp,expert", "--fair-scouting",
                   "--max-secs", "60", "--out", str(run)]
        if run_job(root / "jobs" / label, command, 1800, inputs=[manifest]):
            raise RuntimeError(f"Collection quarantined: {label}")
        (train if split == "train" else val).append(str(run))
    admission = {}
    for run in train + val:
        run_meta = json.loads((Path(run) / "run.json").read_text())
        if not run_meta.get("finished") or not run_meta.get("fair_scouting"):
            raise ValueError(f"Unfinished or privileged teacher run: {run}")
        admission[run] = run_meta.get("totals", {}).get("outcomes", {})
    atomic_json(root / "admission.json", {"outcomes_by_run": admission,
        "rule": "Only won/team_won/draw records enter conversion; caps and failures remain findings"})
    dataset = root / "dataset"
    command = ["target/release/convert4", "--runs", ",".join(train),
               "--val-runs", ",".join(val), "--out", str(dataset),
               "--include-passive-obs", "--threads", "4"]
    if run_job(root / "jobs" / "convert", command, 3600,
               inputs=[Path(run) / "run.json" for run in train + val]):
        raise RuntimeError("Conversion quarantined")
    converted = json.loads((dataset / "meta.json").read_text())
    if converted["games_rejected"] or converted["unencoded_decisions"]:
        raise ValueError("Review rejected/unencoded games before admitting the broad dataset")
    atomic_json(root / "result.json", {"status": "complete", "dataset": str(dataset),
        "train_steps": converted["train_steps"], "val_steps": converted["val_steps"]})


if __name__ == "__main__":
    main()
