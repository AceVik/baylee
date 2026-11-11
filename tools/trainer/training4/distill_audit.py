"""Where a distilled student answers: against N (r1's own top-1) and R, per subtype,
and the offline pre-gate's verdict for an existing checkpoint.

    CUDA_VISIBLE_DEVICES= python -m training4.distill_audit \
        --student /mnt/xdata/.../round0-A-s1 --reference .../r1-4eb --data DATASET \
        --kinds 0,8 --subtypes cast,pass,instant_their_turn,spend,target --every 8 --out audit.json

Runs `evaluate_distill` over the validation split (every `--every`-th
trajectory, whole trajectories from their first frame, so memory is exact) and
reports, for the distilled kinds' single choices by group and subtype, the
student's agreement with N, with R, the share it answers neither, r1's own
agreement with R, and, where it answers neither, the actions it moved to
(R's, N's and the student's action by the candidate's priority column); R's
acting chains (`distill.ChainCounter`) at their start, mid-chain and at R's
passes, and the whole-chain completion. Its last line is the pre-gate
(`distill.pregate`, `--min-ratio`), `[pregate] {...}`, and the exit code is 3
when it fails. The coarse classes come from the dataset's sidecar when it has
one, else from the candidates' action columns. The reference shares the
student's boards when every weight before the heads is equal; otherwise it
encodes its own.

On a house_distillation_v1 dataset (R := H, the house expert) it is the house
teacher's offline probe: every kind is in scope, and the report adds per
pending kind the student's and r1's top-1 agreement with H at N != H and at
N = H, the chain-start act rates of the student, H and r1 where H's last
priority answer was a pass, whole-chain completion, the pass shares at
priority, and the gate (`house.house_gate`: `--gate-top1` over every kind at
N != H, the student's chain-start act rate at least `--gate-ratio` times H's).
It prints `[pregate]` (against r1) and then `[house-gate]`, and exits 3 when
the house gate fails.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import time

import torch
from torch.utils.data import DataLoader

import dataset4 as D4
from sampler4 import StatefulBatchSampler4
from . import distill as D
from .partial import HEAD_PREFIXES, OTHER_HEADS, VALUE_HEADS


def same_upstream(student, reference):
    """Every weight before the heads equal: r1's boards and memory are the student's."""
    theirs = dict(reference.named_parameters())
    return all(torch.equal(parameter, theirs[name]) for name, parameter in student.named_parameters()
               if not name.startswith(HEAD_PREFIXES + OTHER_HEADS + VALUE_HEADS))


def configure(net, args):
    net.decision_chunk = args.decision_chunk
    net.board_token_budget = args.board_token_budget
    net.prune_unused_policy_heads = True
    net.cache_thinking_kv = True
    net.gpu_prefetch = 0
    return net


def table_rows(metrics, groups):
    rows = []
    for group in groups:
        key = f"val_distill_{group}"
        if metrics.get(f"{key}_decisions"):
            rows.append(dict(group=group, decisions=metrics[f"{key}_decisions"], n_ne_r=metrics[f"{key}_n_ne_r"],
                             top1_n=metrics.get(f"{key}_top1_n"), top1_r=metrics.get(f"{key}_top1_r"),
                             top1_neither=metrics.get(f"{key}_top1_neither"),
                             r1_top1_r=metrics.get(f"{key}_r1_top1_r"),
                             n_flag_mismatch=metrics.get(f"{key}_n_flag_mismatch")))
    return rows


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--student", required=True, help="Checkpoint (net.pt or its directory)")
    parser.add_argument("--reference", required=True, help="r1, the distillation's --init")
    parser.add_argument("--data", required=True)
    parser.add_argument("--encoder-version", type=int, default=10)
    parser.add_argument("--unroll", type=int, default=32)
    parser.add_argument("--kinds", default="0,8")
    parser.add_argument("--subtypes", default="cast,pass,instant_their_turn,spend,target")
    parser.add_argument("--every", type=int, default=1, help="Every k-th validation trajectory (deterministic)")
    parser.add_argument("--split", default="val", help="The split to read (held-out games are val)")
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--board-chunk", type=int, default=32)
    parser.add_argument("--decision-chunk", type=int, default=4)
    parser.add_argument("--board-token-budget", type=int, default=32768)
    parser.add_argument("--threads", type=int, default=8)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--min-ratio", type=float, default=0.8, help="The pre-gate's ratio to r1")
    parser.add_argument("--gate-top1", type=float, default=0.70,
                        help="House teacher: held-out top-1 agreement with H at N != H over every kind")
    parser.add_argument("--gate-ratio", type=float, default=0.9,
                        help="House teacher: the student's chain-start act rate against H's")
    parser.add_argument("--scalar-tolerance", type=float, default=0.05)
    parser.add_argument("--out", default="")
    args = parser.parse_args(argv)
    torch.set_num_threads(args.threads)
    from value_calibration4.checkpoint import load_source

    _, _, student = load_source(args.student)
    _, _, r1 = load_source(args.reference)
    configure(student, args)
    configure(r1, args)
    data = Path(args.data).expanduser()
    dataset = D4.TrajectoryDataset4(data, encoder_version=args.encoder_version, split=args.split, unroll_len=args.unroll)
    house = D.dataset_contract(getattr(dataset, "meta", None) or {}) == D.HOUSE_CONTRACT
    D.check_datasets([dataset], True, "house" if house else "bundle")
    if (data / "bundle-distill" / "coarse-classes.json").is_file():
        from .coarse import CoarseClasses
        dataset.coarse = CoarseClasses.load(data / "bundle-distill")
    total_trajectories = len(dataset.trajectories)
    dataset.trajectories = dataset.trajectories[::args.every]
    if house:
        scope = D.Scope.from_options(tuple(range(D.KIND_SLOTS)), 1.0, (), {}, [dataset], teacher="house")
    else:
        kinds = tuple(int(k) for k in args.kinds.split(","))
        subtypes = tuple(s for s in args.subtypes.split(",") if s)
        scope = D.Scope.from_options(kinds, 1.0, subtypes, {}, [dataset])
    shared = same_upstream(student, r1)
    reference = D.Reference(r1, student)
    reference.shared_trunk, reference.shared_lower = shared, None
    loader = DataLoader(dataset, batch_sampler=StatefulBatchSampler4(dataset, args.batch_size, shuffle=False),
                        collate_fn=D4.collate_trajectories, num_workers=args.workers)
    actions, began, seen = {}, time.perf_counter(), [0]

    def progress(table):
        seen[0] += 1
        if seen[0] % 50 == 0:
            row = table.get("out_of_scope")
            done = 0 if row is None else int(row[0])
            print(f"[audit] batches={seen[0]} out_of_scope_decisions={done} "
                  f"seconds={time.perf_counter() - began:.0f}", flush=True)

    metrics = D.evaluate_distill(student, reference, loader, torch.device("cpu"), scope, args.board_chunk,
                                 args.scalar_tolerance, bf16=False, actions=actions, progress=progress)
    groups = ["distilled", "out_of_scope", *(f"subtype:{name}" for name in ("unknown", *scope.subtype_names))]
    moves = sorted(({"subtype": s, "r_action": r, "n_action": n, "student_action": a, "count": c}
                    for (s, r, n, a), c in actions.items()), key=lambda m: -m["count"])
    passed, verdict = D.pregate(metrics, args.min_ratio)
    report = dict(student=str(args.student), reference=str(args.reference), data=str(args.data), split=args.split,
                  trajectories=len(dataset.trajectories), of_trajectories=total_trajectories, every=args.every,
                  frames=metrics["val_frames"], shared_trunk=shared, coarse_sidecar=hasattr(dataset, "coarse"),
                  seconds=round(time.perf_counter() - began),
                  rows=table_rows(metrics, groups), neither_moves=moves,
                  chains={key: value for key, value in metrics.items()
                          if key.startswith(("val_distill_chain", "val_distill_pregate", "val_distill_act_"))},
                  coarse_confusion=metrics.get("val_distill_coarse_confusion"),
                  coarse_confusion_n_ne_r=metrics.get("val_distill_coarse_confusion_n_ne_r"),
                  terms={name: metrics.get(f"val_distill_{name}") for name in D.TERMS}, pregate=verdict)
    gate = None
    if house:
        from .house import house_gate
        passed, gate = house_gate(metrics, args.gate_top1, args.gate_ratio)
        report.update(house={key: value for key, value in metrics.items() if key.startswith("val_house_")},
                      kinds=metrics.get("val_distill_kinds"), house_gate=gate)
    text = json.dumps(report, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(text, flush=True)
    print("[pregate] " + json.dumps(verdict), flush=True)
    if gate is not None:
        print("[house-gate] " + json.dumps(gate), flush=True)
    return 0 if passed else 3


if __name__ == "__main__":
    sys.exit(main())
