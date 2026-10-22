"""Bounded, non-destructive BC -> export -> validation -> audited PPO pipeline.

Run in the repository root. Every stage has its own durable job and logs;
existing successful stages are reused, failures are preserved for diagnosis.
No model is promoted to a live server by this offline pipeline.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import sys

from run_guard4 import run_job, atomic_json


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True, help="new experiment root")
    ap.add_argument("--init", default="", help="Compatible weights from a completed preliminary run")
    ap.add_argument("--manifest", default="data/decks/manifest.v4.json")
    ap.add_argument("--hours", type=float, default=5)
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--d-model", type=int, default=640)
    ap.add_argument("--heads", type=int, default=10)
    ap.add_argument("--layers", type=int, default=8)
    ap.add_argument("--batch-size", type=int, default=8)
    ap.add_argument("--unroll", type=int, default=32)
    ap.add_argument("--supervised-only", action="store_true")
    args = ap.parse_args()
    root, data = Path(args.out).resolve(), Path(args.data).resolve()
    root.mkdir(parents=True, exist_ok=True)
    if not (data / "meta.json").exists():
        raise ValueError("Conversion must finish before this pipeline starts")
    metadata = json.loads((data / "meta.json").read_text())
    if metadata.get("games_rejected", 0):
        raise ValueError("Review rejected-games.json before using this dataset")
    bc, rl = root / "bc", root / "ppo-1"
    py = sys.executable

    def stage(name, command, timeout, inputs=()):
        print(f"[pipeline4] {name}", flush=True)
        status = run_job(root / "jobs" / name, command, timeout, inputs=inputs)
        if status:
            raise RuntimeError(f"Stage {name} quarantined; inspect its job.json and stderr.log")

    stage("supervised", [py, "tools/trainer/train4.py", "--data", str(data), "--out", str(bc),
          "--d-model", str(args.d_model), "--layers", str(args.layers), "--heads", str(args.heads),
          "--batch-size", str(args.batch_size), "--unroll", str(args.unroll),
          "--activation-checkpointing", "--thinking-steps", "1", "--thinking-curriculum", "--lr", "0.00015",
          "--epochs", str(args.epochs), "--time-budget-seconds", str(args.hours * 3600),
          "--num-workers", "4", "--checkpoint-every", "100", "--validation-every", "1000",
          *(["--init", args.init] if args.init else [])],
          args.hours * 3600 + 5400,
          [data / "meta.json", args.manifest, *([Path(args.init) / "net.pt"] if args.init else [])])

    def export(name, model):
        stage(name, [py, "tools/trainer/export_onnx4.py", "--model", str(model),
                     "--data", str(data), "--thinking-steps", "1", "--check", "128"], 1800,
              [model / "net.pt"])

    def evaluate(name, model, seats=2, shape="constructed", depth=1):
        path = root / "evaluation" / f"{name}.json"
        stage(name, ["target/release/eval4", "--model", str(model), "--deals", "32",
                     "--seats", str(seats), "--shape", shape, "--thinking-steps", str(depth),
                     "--against", "sharp", "--baseline", "steady", "--generated", "0",
                     "--decks-manifest", args.manifest, "--deck-split", "val",
                     "--threads", "4", "--first-seed", "870001", "--out", str(path)], 5400,
              [model / "model.onnx.json", args.manifest])
        return path

    export("export-bc", bc)
    evaluate("bc-duels", bc)
    evaluate("bc-commander", bc, 4, "commander")
    if args.supervised_only:
        atomic_json(root / "result.json", {"status": "supervised_complete", "model": str(bc)})
        return
    runs = []
    for name, seats, shape, deals, teams in [
        ("duels", 2, "constructed", 32, []), ("commander", 4, "commander", 16, []),
        ("teams", 4, "commander", 16, ["--teams", "1,2,1,2"])]:
        run = root / "rollouts" / name
        runs.append(str(run))
        stage("collect-" + name, ["target/release/eval4", "--model", str(bc), "--deals", str(deals),
              "--seats", str(seats), "--shape", shape, "--thinking-steps", "1", "--temperature", "0.6",
              "--against", "steady", "--generated", "0", "--decks-manifest", args.manifest,
              "--deck-split", "train", "--threads", "4", "--first-seed", "880001", "--no-baseline",
              "--record-out", str(run), "--out", str(root / "evaluation" / f"collect-{name}.json"), *teams],
              7200, [bc / "model.onnx.json", args.manifest])
    rollout = root / "rollout-data"
    stage("convert-rollout", ["target/release/convert4", "--runs", ",".join(runs),
          "--out", str(rollout), "--include-passive-obs", "--val-every", "0", "--threads", "4"], 3600)
    # train_rl4 always audits the frozen behavior probabilities before updating.
    stage("ppo", [py, "tools/trainer/train_rl4.py", "--init", str(bc), "--rollout-data", str(rollout),
          "--anchor-data", str(data), "--out", str(rl), "--batch-size", "4", "--unroll", "16",
          "--epochs", "1", "--max-steps", "80", "--max-shards", "0", "--thinking-steps", "1",
          "--gamma", "1", "--gae-lambda", "1", "--target-kl", "0.02", "--beta-kl", "0.04"],
          7200, [bc / "net.pt", rollout / "meta.json"])
    export("export-ppo", rl)
    evaluate("ppo-duels", rl)
    evaluate("ppo-commander", rl, 4, "commander")
    atomic_json(root / "result.json", {"status": "experiment_complete", "bc": str(bc), "ppo": str(rl),
        "selection": "Both checkpoints retained; use paired validation results, no automatic live promotion"})


if __name__ == "__main__":
    main()
