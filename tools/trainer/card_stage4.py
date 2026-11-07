"""Card stage: teach the rule encoder what each card is, costs and does.

Targets come from `card_structure4 --labels`, which reads the definitions the
catalog encodes (`cardstructure::labels`): mana value and cost symbols,
colours, types, supertypes, subtypes, printed power, toughness, loyalty and
defense, keywords, ability kinds, effect variants and effect families. They
are exact as the engine plays the card; a family's boundary is a name rule.

Only the rule encoder learns, and only as far as `--train` allows:
`new-tags` moves the symbols of operators the reference never had, so a card
built of known operators keeps its embedding bit for bit; `encoder` moves the
whole encoder, and an anchor holds every root whose embedding the refresh left
unchanged at the reference's value. Linear probes read the root embeddings; a
probe-only phase first measures what the starting encoder already says, then
the encoder learns with them. Probes are saved apart and never enter net.pt.

This is representation training. Whether a card stage plays better is decided
by paired `eval4` games, never by these metrics.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import time

import torch
from torch import nn
from torch.nn import functional as F

from checkpoint4 import atomic_save
from export_onnx4 import load_export_checkpoint
from torch_io4 import append_json, atomic_json, file_digest

LABELS_VERSION = 1
MODES = ("new-tags", "encoder")
COST_KINDS = ("white", "blue", "black", "red", "green", "colorless", "generic",
              "variable", "hybrid", "phyrexian", "snow")
MAX_MANA_VALUE = 15
PT_RANGE = (-1, 15)
SMALL_MAX = 10


def _bits(value, width):
    return [(value >> bit) & 1 for bit in range(width)]


def _multi_hot(indices, width):
    row = [0.0] * width
    for index in indices:
        row[index] = 1.0
    return row


class Targets:
    """Every face of the labels, as tensors, with its split."""

    def __init__(self, labels, roots, holdout_every):
        if labels.get("version") != LABELS_VERSION:
            raise ValueError("Unsupported card label version")
        faces = labels["faces"]
        missing = [face["root"] for face in faces if face["root"] not in roots]
        if missing:
            raise ValueError(f"Labels name roots the catalog lacks: {missing[:3]}")
        widths = dict(subtypes=len(labels["subtypes"]), kinds=len(labels["ability_kinds"]),
                      families=len(labels["families"]), effects=len(labels["effects"]))
        self.widths = widths
        self.node = torch.tensor([roots[face["root"]] for face in faces], dtype=torch.long)
        self.card = torch.tensor([face["card"] for face in faces], dtype=torch.long)
        self.validation = self.card % holdout_every == 0
        self.mana_value = torch.tensor([min(face["mana_value"], MAX_MANA_VALUE) for face in faces])
        self.cost = torch.tensor([[min(face["cost"][kind], SMALL_MAX) for kind in COST_KINDS]
                                  for face in faces], dtype=torch.float32)
        self.multi = {
            "colors": torch.tensor([_bits(face["colors"], 5) for face in faces], dtype=torch.float32),
            "types": torch.tensor([_bits(face["types"], 16) for face in faces], dtype=torch.float32),
            "supertypes": torch.tensor([_bits(face["supertypes"], 8) for face in faces], dtype=torch.float32),
            "keywords": torch.tensor([_multi_hot(face["keywords"], 128) for face in faces], dtype=torch.float32),
            "subtypes": torch.tensor([_multi_hot(face["subtypes"], widths["subtypes"]) for face in faces],
                                     dtype=torch.float32),
            "ability_kinds": torch.tensor([_multi_hot(face["ability_kinds"], widths["kinds"]) for face in faces],
                                          dtype=torch.float32),
            "families": torch.tensor([_multi_hot(face["families"], widths["families"]) for face in faces],
                                     dtype=torch.float32),
            "effects": torch.tensor([_multi_hot(face["effects"], widths["effects"]) for face in faces],
                                    dtype=torch.float32),
        }
        # A column no training face sets carries no signal; it stays out of loss and metrics.
        train = ~self.validation
        self.columns = {name: value[train].sum(0) > 0 for name, value in self.multi.items()}
        self.numbers = {}
        for name, low, high in (("power", *PT_RANGE), ("toughness", *PT_RANGE),
                                ("loyalty", 0, SMALL_MAX), ("defense", 0, SMALL_MAX)):
            present = torch.tensor([face[name] is not None for face in faces])
            value = torch.tensor([min(max(face[name], low), high) - low if face[name] is not None else 0
                                  for face in faces])
            self.numbers[name] = (present, value, high - low + 1)

    def to(self, device):
        moved = object.__new__(Targets)
        moved.__dict__.update(self.__dict__)
        moved.node, moved.card = self.node.to(device), self.card.to(device)
        moved.validation, moved.mana_value = self.validation.to(device), self.mana_value.to(device)
        moved.cost = self.cost.to(device)
        moved.multi = {k: v.to(device) for k, v in self.multi.items()}
        moved.columns = {k: v.to(device) for k, v in self.columns.items()}
        moved.numbers = {k: (p.to(device), v.to(device), n) for k, (p, v, n) in self.numbers.items()}
        return moved


class Probes(nn.Module):
    """One linear readout per target from a root embedding."""

    def __init__(self, width, targets):
        super().__init__()
        self.mana_value = nn.Linear(width, MAX_MANA_VALUE + 1)
        self.cost = nn.Linear(width, len(COST_KINDS))
        self.multi = nn.ModuleDict({name: nn.Linear(width, value.shape[1])
                                    for name, value in targets.multi.items()})
        self.present = nn.Linear(width, len(targets.numbers))
        self.numbers = nn.ModuleDict({name: nn.Linear(width, classes)
                                      for name, (_, _, classes) in targets.numbers.items()})

    def losses(self, embedding, targets, rows):
        out = {"mana_value": F.cross_entropy(self.mana_value(embedding), targets.mana_value[rows]),
               "cost": F.mse_loss(self.cost(embedding), targets.cost[rows])}
        for name, head in self.multi.items():
            keep = targets.columns[name]
            if keep.any():
                out[name] = F.binary_cross_entropy_with_logits(head(embedding)[:, keep],
                                                               targets.multi[name][rows][:, keep])
        present = torch.stack([targets.numbers[name][0][rows] for name in targets.numbers], 1).float()
        out["present"] = F.binary_cross_entropy_with_logits(self.present(embedding), present)
        for name, head in self.numbers.items():
            has, value, _ = targets.numbers[name]
            mask = has[rows]
            if mask.any():
                out[name] = F.cross_entropy(head(embedding[mask]), value[rows][mask])
        return out

    @torch.no_grad()
    def metrics(self, embedding, targets, rows):
        out = {"mana_value_accuracy": float((self.mana_value(embedding).argmax(-1)
                                             == targets.mana_value[rows]).float().mean()),
               "cost_exact": float((self.cost(embedding).round() == targets.cost[rows]).all(-1).float().mean())}
        for name, head in self.multi.items():
            keep = targets.columns[name]
            if not keep.any():
                continue
            guess = head(embedding)[:, keep] > 0
            truth = targets.multi[name][rows][:, keep] > .5
            hits = (guess & truth).sum()
            f1 = 2 * hits / (guess.sum() + truth.sum()).clamp_min(1)
            out[f"{name}_f1"] = float(f1)
            out[f"{name}_exact"] = float((guess == truth).all(-1).float().mean())
        for name, head in self.numbers.items():
            has, value, _ = targets.numbers[name]
            mask = has[rows]
            if mask.any():
                out[f"{name}_accuracy"] = float((head(embedding[mask]).argmax(-1)
                                                 == value[rows][mask]).float().mean())
        return out


def anchored_roots(new_table, new_roots, reference_table, reference_roots, tolerance=1e-6):
    """Roots both catalogs name whose embedding the refresh left unchanged."""
    shared = sorted(set(new_roots) & set(reference_roots))
    device = new_table.device
    new_ids = torch.tensor([new_roots[k] for k in shared], dtype=torch.long, device=device)
    old_ids = torch.tensor([reference_roots[k] for k in shared], dtype=torch.long, device=device)
    same = (new_table[new_ids] - reference_table[old_ids]).abs().amax(-1) <= tolerance
    return new_ids[same], reference_table[old_ids[same]], len(shared)


def restrict_to_new_tags(rules, reference_tags):
    """Freeze the encoder but the symbols of operators the reference never had."""
    known = set(reference_tags)
    new = torch.tensor([tag not in known for tag in rules.data["tags"]])
    for parameter in rules.parameters():
        parameter.requires_grad_(False)
    rules.symbol.weight.requires_grad_(True)
    keep = new.to(rules.symbol.weight.dtype)[:, None]
    rules.symbol.weight.register_hook(lambda grad: grad * keep.to(grad.device))
    return new


def run(args):
    if args.out.exists():
        raise ValueError("Output exists; keep previous stages immutable")
    torch.manual_seed(args.seed)
    device = torch.device(args.device)
    checkpoint, net, _ = load_export_checkpoint(args.model / "net.pt")
    reference, reference_net, _ = load_export_checkpoint(args.reference / "net.pt")
    labels = json.loads(args.labels.read_text())
    if labels.get("catalog_sha256") != net.rules.digest:
        raise ValueError("Labels belong to another catalog; write them with this build's card_structure4")
    sources = [args.model / "net.pt", args.reference / "net.pt", args.labels]
    hashes = {str(path): file_digest(path) for path in sources}
    # Which roots the refresh left unchanged is decided on the CPU, where the
    # same weights over the same tree give the same bits. A GPU's
    # order-dependent sums differ in the last places and would leave
    # unchanged roots unanchored (the first CUDA run anchored 23,571 of
    # 39,927).
    with torch.no_grad():
        reference_cpu = reference_net.rules.encode_catalog()
        start_cpu = net.rules.encode_catalog()
    anchor_ids, anchor_values, shared = anchored_roots(start_cpu, net.rules.data["roots"], reference_cpu,
                                                       reference_net.rules.data["roots"])
    anchor_ids, anchor_values = anchor_ids.to(device), anchor_values.to(device)
    del reference_net, reference_cpu
    rules = net.rules.to(device)
    targets = Targets(labels, rules.data["roots"], args.holdout_every).to(device)
    train_rows = (~targets.validation).nonzero().squeeze(1)
    val_rows = targets.validation.nonzero().squeeze(1)
    with torch.no_grad():
        start_table = rules.encode_catalog()
    new_tags = None
    initial_symbols = rules.symbol.weight.detach().clone()
    if args.train == "new-tags":
        new_tags = restrict_to_new_tags(rules, reference["structure"]["tags"])
    probes = Probes(rules.width, targets).to(device)
    args.out.mkdir(parents=True)
    log = args.out / "metrics.jsonl"
    started = time.monotonic()

    def measure(phase, step):
        rules.eval()
        with torch.no_grad():
            table = rules.encode_catalog()
            record = {"phase": phase, "step": step, "elapsed_s": time.monotonic() - started,
                      "validation": probes.metrics(table[targets.node[val_rows]], targets, val_rows),
                      "train": probes.metrics(table[targets.node[train_rows]], targets, train_rows),
                      "anchor_rms": float((table[anchor_ids] - anchor_values).square().mean().sqrt())
                      if len(anchor_ids) else 0.0}
        print(json.dumps(record), flush=True)
        append_json(log, record)
        return record

    def train(parameters, steps, phase, with_anchor):
        optimizer = torch.optim.AdamW(parameters, lr=args.lr, weight_decay=0.0)
        for step in range(1, steps + 1):
            rules.train()
            table = rules.encode_catalog() if phase == "joint" else start_table
            parts = probes.losses(table[targets.node[train_rows]], targets, train_rows)
            loss = sum(parts.values())
            if with_anchor and len(anchor_ids):
                loss = loss + args.anchor_weight * (table[anchor_ids] - anchor_values).square().mean()
            if not torch.isfinite(loss):
                raise FloatingPointError("non-finite card-stage loss")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            nn.utils.clip_grad_norm_([p for group in optimizer.param_groups for p in group["params"]], 1.0)
            optimizer.step()
            if step % args.measure_every == 0 or step == steps:
                measure(phase, step)

    # Probes alone first: what the starting encoder already says.
    train(list(probes.parameters()), args.probe_steps, "probe", with_anchor=False)
    before = measure("before", 0)
    encoder_parameters = [p for p in rules.parameters() if p.requires_grad]
    train(list(probes.parameters()) + encoder_parameters, args.steps, "joint",
          with_anchor=args.train == "encoder")
    after = measure("after", args.steps)
    rules.eval()
    if new_tags is not None:
        old_rows = ~new_tags.to(device)
        if not torch.equal(rules.symbol.weight[old_rows], initial_symbols[old_rows]):
            raise AssertionError("A known operator moved in new-tags mode")
    net.rules = rules.to("cpu")
    with torch.no_grad():
        final_cpu = net.rules.encode_catalog()
        root_ids = torch.tensor(sorted(set(net.rules.data["roots"].values())))
        moved = (final_cpu[root_ids] - start_cpu[root_ids]).abs().amax(-1) > 0
    lineage = dict(source=str((args.model / "net.pt").resolve()), source_sha256=hashes[str(args.model / "net.pt")],
                   reference=str((args.reference / "net.pt").resolve()),
                   reference_sha256=hashes[str(args.reference / "net.pt")],
                   labels_sha256=hashes[str(args.labels)], catalog_sha256=rules.digest,
                   train=args.train, steps=args.steps, probe_steps=args.probe_steps, lr=args.lr,
                   anchor_weight=args.anchor_weight if args.train == "encoder" else 0.0, seed=args.seed,
                   holdout="card id % {} == 0".format(args.holdout_every),
                   new_tags=[tag for tag, new in zip(rules.data["tags"], new_tags.tolist()) if new]
                   if new_tags is not None else None,
                   shared_roots=shared, anchored_roots=int(len(anchor_ids)),
                   roots_moved=int(moved.sum()), metrics_before=before, metrics_after=after,
                   playing_strength="not evaluated; decide by paired eval4")
    payload = {**checkpoint, "state_dict": net.state_dict(), "card_stage": lineage}
    if "card_stage" in checkpoint:
        lineage["previous_card_stage"] = checkpoint["card_stage"]
    metadata_path = args.model / "meta.json"
    metadata = json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
    metadata["card_stage"] = lineage
    for path, expected in hashes.items():
        if file_digest(path) != expected:
            raise ValueError("Card-stage source changed: " + path)
    atomic_save(payload, args.out / "net.pt")
    atomic_json(args.out / "meta.json", metadata)
    atomic_save({"probes": probes.state_dict(), "widths": targets.widths}, args.out / "probes.pt")
    atomic_json(args.out / "status.json", {"status": "complete", "roots_moved": lineage["roots_moved"]})
    return lineage


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--model", type=Path, required=True, help="Refreshed checkpoint directory (net.pt)")
    ap.add_argument("--reference", type=Path, required=True,
                    help="The checkpoint before the refresh: anchor values and the operators it knew")
    ap.add_argument("--labels", type=Path, required=True, help="card_structure4 --labels of this catalog")
    ap.add_argument("--out", type=Path, required=True, help="New checkpoint directory")
    ap.add_argument("--train", choices=MODES, required=True)
    ap.add_argument("--steps", type=int, default=1500)
    ap.add_argument("--probe-steps", type=int, default=1500)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--anchor-weight", type=float, default=100.0)
    ap.add_argument("--holdout-every", type=int, default=10)
    ap.add_argument("--measure-every", type=int, default=250)
    ap.add_argument("--seed", type=int, default=71001)
    ap.add_argument("--device", default="cuda")
    args = ap.parse_args()
    if args.steps < 1 or args.probe_steps < 1 or args.holdout_every < 2:
        ap.error("Steps must be positive and the holdout a proper fraction")
    lineage = run(args)
    print(json.dumps({"roots_moved": lineage["roots_moved"], "anchored_roots": lineage["anchored_roots"]}))


if __name__ == "__main__":
    main()
