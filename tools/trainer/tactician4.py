"""Trains the tactician head on game outcomes (stage 1: attacks, blocks, targets).

Reads the window decisions an exploring collection kept (`eval4
--tactician-explore`, `tactician_records_v1`: `wNN.jsonl` beside `wNN.f32`)
and fits one MLP over rows `[state | unit embedding | window code and look]`,
the null row first, as `netplay4::tactician::head::assemble` builds them. The
advantage of the played answer is the null row's score plus the played units'
scores; its target is the deal's utility for the seat minus the net's value at
the decision. Outcomes are the only signal: no rule, house or teacher answer is
a label. Exports `tactician.onnx` (`rows` [N, 2d + 128] -> `score` [N]) and
`tactician.json`, the margin included (docs/trained-ai.md, "The tactician").
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import torch
from torch import nn

CONTRACT = "tactician_head_v1"
RECORD_CONTRACT = "tactician_records_v1"
ROW_TAIL = 128
WEIGHT_CAP = 10.0
MARGINS = (0.0, 0.005, 0.01, 0.02, 0.04, 0.08)


class TacticianHead(nn.Module):
    """width -> width -> width -> 1 with GELU: four times the plain head."""

    def __init__(self, width: int):
        super().__init__()
        self.net = nn.Sequential(nn.Linear(width, width), nn.GELU(), nn.Linear(width, width),
                                 nn.GELU(), nn.Linear(width, 1))

    def forward(self, rows):
        return self.net(rows).squeeze(-1)


def read_records(dirs):
    """Every decision of the record directories, with its arrays."""
    decisions = []
    for directory in map(Path, dirs):
        blobs = {}
        for lines in sorted(directory.glob("w*.jsonl")):
            for line in lines.read_text().splitlines():
                if not line.strip():
                    continue
                record = json.loads(line)
                if record.get("contract") != RECORD_CONTRACT:
                    raise ValueError(f"{lines}: not a {RECORD_CONTRACT} record")
                blob = record["blob"]
                if blob["file"] not in blobs:
                    blobs[blob["file"]] = np.memmap(directory / blob["file"], dtype="<f4", mode="r")
                data = blobs[blob["file"]]
                start = blob["offset"] // 4
                state = np.array(data[start:start + blob["state"]])
                start += blob["state"]
                units = np.array(data[start:start + blob["units"]])
                start += blob["units"]
                rows = np.array(data[start:start + blob["rows"]])
                n = len(record["net"])
                d = len(state)
                if units.size != n * d or rows.size != (n + 1) * ROW_TAIL:
                    raise ValueError(f"{lines}: arrays do not match the decision's units")
                record["state"] = state
                record["units"] = units.reshape(n, d)
                record["tails"] = rows.reshape(n + 1, ROW_TAIL)
                record["source"] = str(directory)
                decisions.append(record)
    if not decisions:
        raise ValueError("no decisions")
    return decisions


def held_out(record) -> bool:
    """Both seats of a deal on the same side: every tenth table by its seed."""
    return record["seed"] % 10 == 0


def advantage_target(record) -> float:
    return float(record["utility"]) - float(record["value"])


def weight(record) -> float:
    return min(1.0 / max(float(record["probability"]), 1e-9), WEIGHT_CAP)


class Batch:
    """Decisions flattened to rows, with what sums a decision's prediction."""

    def __init__(self, records, device):
        d = len(records[0]["state"])
        rows, owner, takes, targets, weights = [], [], [], [], []
        for i, record in enumerate(records):
            n = len(record["net"])
            state = np.broadcast_to(record["state"], (n + 1, d))
            units = np.concatenate([np.zeros((1, d), np.float32), record["units"]])
            rows.append(np.concatenate([state, units, record["tails"]], axis=1))
            owner.extend([i] * (n + 1))
            # The null row always counts; a unit counts when the answer takes it.
            takes.extend([1.0] + [float(bit != 0) for bit in record["played"]])
            targets.append(advantage_target(record))
            weights.append(weight(record))
        self.rows = torch.from_numpy(np.concatenate(rows).astype(np.float32)).to(device)
        self.owner = torch.tensor(owner, device=device)
        self.takes = torch.tensor(takes, device=device)
        self.targets = torch.tensor(targets, device=device)
        self.weights = torch.tensor(weights, device=device)
        self.size = len(records)

    def predict(self, head):
        scores = head(self.rows)
        return torch.zeros(self.size, device=scores.device, dtype=scores.dtype).index_add(
            0, self.owner, scores * self.takes)


def weighted_mse(pred, batch, scale):
    return float(((pred - batch.targets) ** 2 * batch.weights).sum() / scale)


def evaluate(head, batches, scale):
    head.eval()
    total, zero = 0.0, 0.0
    with torch.no_grad():
        for batch in batches:
            total += weighted_mse(batch.predict(head), batch, scale)
            zero += weighted_mse(torch.zeros_like(batch.targets), batch, scale)
    head.train()
    return total, zero


def batches_of(records, size, device):
    return [Batch(records[i:i + size], device) for i in range(0, len(records), size)]


def supported(record):
    """`N` first, then the rules' answer and the neighbors: what a head may play."""
    out = [record["net"]]
    for mask in [record["rules"], *record["offered"]]:
        if mask not in out:
            out.append(mask)
    return out


def choice(scores, record, margin):
    """The answer `TacticianMode4::Play` plays from these row scores."""
    def value(mask):
        return scores[0] + sum(scores[1 + i] for i, bit in enumerate(mask) if bit)
    net = record["net"]
    base, best, gain = value(net), net, None
    for mask in supported(record)[1:]:
        g = value(mask) - base
        if g > margin and (gain is None or g > gain):
            best, gain = mask, g
    return best


def margin_estimates(head, records, device):
    """One-step importance estimates of the mean advantage per margin."""
    head.eval()
    sums = {m: 0.0 for m in MARGINS}
    with torch.no_grad():
        for record in records:
            batch = Batch([record], device)
            scores = head(batch.rows).cpu().numpy().tolist()
            for margin in MARGINS:
                if choice(scores, record, margin) == record["played"]:
                    sums[margin] += advantage_target(record) / float(record["probability"])
    head.train()
    return {m: s / max(len(records), 1) for m, s in sums.items()}


def pick_margin(estimates):
    best = max(estimates.values())
    return max(m for m, e in estimates.items() if e == best)


def export(head, width, path):
    import onnx  # noqa: F401  (the exporter needs it)
    head = head.cpu().eval()
    dummy = torch.zeros(3, width)
    torch.onnx.export(head, (dummy,), str(path), input_names=["rows"], output_names=["score"],
                      dynamic_axes={"rows": {0: "N"}, "score": {0: "N"}}, opset_version=18,
                      dynamo=False)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def train(args):
    if Path(args.out).exists():
        raise FileExistsError(f"{args.out} exists; a head gets its own directory")
    torch.manual_seed(args.seed)
    np.random.seed(args.seed)
    device = torch.device(args.device)
    records = read_records(args.records)
    d = len(records[0]["state"])
    width = 2 * d + ROW_TAIL
    train_set = [r for r in records if not held_out(r)]
    val_set = [r for r in records if held_out(r)]
    if not train_set or not val_set:
        raise ValueError("both a training and a held-out split are needed")
    mean_weight = float(np.mean([weight(r) for r in train_set]))
    rng = np.random.default_rng(args.seed)
    val_batches = batches_of(val_set, args.batch, device)
    train_eval = batches_of(train_set, args.batch, device)
    train_scale = mean_weight * len(train_set)
    val_scale = mean_weight * len(val_set)
    head = TacticianHead(width).to(device)
    optimizer = torch.optim.AdamW(head.parameters(), lr=args.lr, weight_decay=args.weight_decay)
    history, best, best_state, stale = [], math.inf, None, 0
    train_loss, train_zero = evaluate(head, train_eval, train_scale)
    val_loss, val_zero = evaluate(head, val_batches, val_scale)
    history.append(dict(epoch=0, train=train_loss, held_out=val_loss,
                        train_zero=train_zero, held_out_zero=val_zero))
    print(json.dumps(history[-1]), flush=True)
    for epoch in range(1, args.epochs + 1):
        order = rng.permutation(len(train_set))
        for start in range(0, len(order), args.batch):
            batch = Batch([train_set[i] for i in order[start:start + args.batch]], device)
            pred = batch.predict(head)
            loss = ((pred - batch.targets) ** 2 * batch.weights / mean_weight).mean()
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
        train_loss, _ = evaluate(head, train_eval, train_scale)
        val_loss, _ = evaluate(head, val_batches, val_scale)
        history.append(dict(epoch=epoch, train=train_loss, held_out=val_loss,
                            train_zero=train_zero, held_out_zero=val_zero))
        print(json.dumps(history[-1]), flush=True)
        if val_loss < best:
            best, stale = val_loss, 0
            best_state = {k: v.detach().clone() for k, v in head.state_dict().items()}
        else:
            stale += 1
            if stale >= args.patience:
                break
    head.load_state_dict(best_state)
    estimates = margin_estimates(head, val_set, device)
    margin = pick_margin(estimates)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=False)
    export(head, width, out / "tactician.onnx")
    torch.save({"state_dict": head.state_dict(), "width": width}, out / "head.pt")
    selected = min(history[1:], key=lambda h: h["held_out"])
    meta = dict(
        contract=CONTRACT, d_model=d, row_tail=ROW_TAIL, width=width,
        parameters=sum(p.numel() for p in head.parameters()),
        margin=margin, margin_estimates={str(k): v for k, v in estimates.items()},
        seed=args.seed, lr=args.lr, weight_decay=args.weight_decay, batch=args.batch,
        weight_cap=WEIGHT_CAP, held_out_rule="seed % 10 == 0",
        decisions=dict(train=len(train_set), held_out=len(val_set)),
        explored=dict(train=sum(r["played"] != r["net"] for r in train_set),
                      held_out=sum(r["played"] != r["net"] for r in val_set)),
        records=[dict(dir=str(p), files=sorted(f.name for f in Path(p).glob("w*")))
                 for p in args.records],
        history=history, selected_epoch=selected["epoch"],
        onnx_sha256=digest(out / "tactician.onnx"),
        target="utility - value (the net's own value at the decision); outcomes only",
    )
    (out / "tactician.json").write_text(json.dumps(meta, indent=2))
    print(json.dumps(dict(selected=selected, margin=margin, estimates=estimates)), flush=True)
    return meta


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--records", nargs="+", required=True, help="tactician_records_v1 directories")
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--weight-decay", type=float, default=1e-2)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--epochs", type=int, default=40)
    ap.add_argument("--patience", type=int, default=4)
    ap.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    return train(ap.parse_args(argv))


if __name__ == "__main__":
    main()
