"""Pretrain public rule representations by answering exact AST path queries.

Targets come from the executable DSL, never a learned move evaluator. A query
names an ordered child path and asks for its node kind and one numeric/string
byte. Different costs, negations, branches and effect order affect the targets.
Validation holds out whole card identities, with every face in the same
split. Shared rule subtrees are expected; complete duplicate roots are removed
from validation. This is auxiliary representation training, not a playing-strength
evaluation. The resulting encoder is fine-tuned jointly with the game policy.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import random
import time

import torch
from torch import nn
from torch.nn import functional as F
from checkpoint4 import atomic_save
from torch_io4 import atomic_json, append_json
from model4.structure import RuleEncoder


class QueryModel(nn.Module):
    def __init__(self, forest, width):
        super().__init__()
        self.encoder = RuleEncoder(forest, width)
        self.query = nn.GRUCell(width, width)
        self.kind = nn.Linear(width * 2, len(forest["tags"]))
        self.byte = nn.Linear(width * 3, 256)

    def forward(self, roots, paths, lengths, byte_positions):
        values = self.encoder.encode_catalog()[roots]
        state = values
        for t in range(paths.shape[1]):
            step = self.query(self.encoder.position(paths[:, t]) + values, state)
            state = torch.where((lengths > t)[:, None], step, state)
        joint = torch.cat((values, state), -1)
        return self.kind(joint), self.byte(torch.cat((joint, self.encoder.position(byte_positions)), -1))


def root_split(forest):
    cards = [(int(k.split("/")[1]), v) for k, v in forest["roots"].items() if k.startswith("c/")]
    train = {root for cid, root in cards if cid % 10 != 0}
    validation = {root for cid, root in cards if cid % 10 == 0} - train
    return sorted(train), sorted(validation)


def queries(forest, roots, count, rng, device):
    rows, paths, targets, byte_positions, byte_targets = [], [], [], [], []
    for _ in range(count):
        root = rng.choice(roots); node = root; path = []
        while forest["nodes"][node]["children"]:
            if path and rng.random() < .15: break
            children = forest["nodes"][node]["children"]
            index = rng.randrange(len(children)); path.append(index); node = children[index]
        value = forest["nodes"][node]
        rows.append(root); paths.append(path); targets.append(value["tag"])
        byte_position = rng.randrange(len(value["bytes"])) if value["bytes"] else 0
        byte_positions.append(byte_position)
        byte_targets.append(value["bytes"][byte_position] if value["bytes"] else -100)
    width = max(1, max(map(len, paths)))
    lengths = list(map(len, paths))
    padded = [p + [0] * (width - len(p)) for p in paths]
    return [torch.tensor(v, dtype=torch.long, device=device) for v in (rows, padded, lengths, byte_positions, targets, byte_targets)]


def loss(model, batch):
    kind, byte = model(*batch[:4])
    # An all-empty-byte batch has no numeric/string target and must remain finite.
    byte_loss = F.cross_entropy(byte, batch[5], reduction="sum") / (batch[5] != -100).sum().clamp_min(1)
    return F.cross_entropy(kind, batch[4]) + .25 * byte_loss, (kind.argmax(-1) == batch[4]).float().mean()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--catalog", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--steps", type=int, default=1200)
    ap.add_argument("--batch", type=int, default=512)
    ap.add_argument("--width", type=int, default=128)
    ap.add_argument("--seed", type=int, default=31415)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--device", default="cuda")
    args = ap.parse_args()
    if args.out.exists(): raise ValueError("Output exists; keep previous experiments immutable")
    args.out.mkdir(parents=True)
    torch.set_num_threads(4); torch.manual_seed(args.seed)
    rng = random.Random(args.seed)
    forest = json.loads(args.catalog.read_text())
    train_roots, val_roots = root_split(forest)
    if not train_roots or not val_roots: raise ValueError("Need separate nonempty root splits")
    model = QueryModel(forest, args.width).to(args.device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr)
    val_rng = random.Random(args.seed + 1)
    validation = [queries(forest, val_roots, args.batch, val_rng, args.device) for _ in range(4)]
    meta = {"structure_sha256": model.encoder.digest, "train_roots": len(train_roots), "val_roots": len(val_roots),
            "seed": args.seed, "width": args.width, "steps": args.steps, "batch": args.batch,
            "objective": "AST path node-kind and value-byte reconstruction", "playing_strength": "not evaluated"}
    atomic_json(args.out / "plan.json", meta)
    best = float("inf"); started = time.monotonic()
    for step in range(args.steps + 1):
        if step:
            model.train(); optimizer.zero_grad(set_to_none=True)
            batch = queries(forest, train_roots, args.batch, rng, args.device)
            objective, accuracy = loss(model, batch)
            if not torch.isfinite(objective): raise FloatingPointError("non-finite structural loss")
            objective.backward(); nn.utils.clip_grad_norm_(model.parameters(), 1, error_if_nonfinite=True); optimizer.step()
        if step % 50 == 0 or step == args.steps:
            model.eval()
            with torch.no_grad():
                metrics = [loss(model, b) for b in validation]
                value = sum(float(v[0]) for v in metrics) / len(metrics)
                accuracy = sum(float(v[1]) for v in metrics) / len(metrics)
            record = {"step": step, "val_loss": value, "val_node_accuracy": accuracy, "elapsed_s": time.monotonic() - started}
            print(json.dumps(record), flush=True)
            append_json(args.out / "metrics.jsonl", record)
            payload = {"structure": forest, "width": args.width, "encoder": model.encoder.state_dict(), "metrics": record,
                       "model": model.state_dict(), "optimizer": optimizer.state_dict(), "rng": rng.getstate(), "torch_rng": torch.get_rng_state()}
            atomic_save(payload, args.out / "last.pt")
            if value < best: best = value; atomic_save(payload, args.out / "best.pt")
    atomic_json(args.out / "status.json", {"status": "complete", "steps": args.steps, "best_val_loss": best})


if __name__ == "__main__": main()
