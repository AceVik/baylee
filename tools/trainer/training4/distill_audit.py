"""Where a distilled student answers: against N (r1's own top-1) and R, per subtype.

    CUDA_VISIBLE_DEVICES= python -m training4.distill_audit \
        --student /mnt/xdata/.../round0-A-s1 --reference .../r1-4eb --data DATASET \
        --kinds 0,8 --subtypes cast,pass,instant_their_turn,spend,target --every 8 --out audit.json

Runs `evaluate_distill` over the validation split (every `--every`-th
trajectory, whole trajectories from their first frame, so memory is exact) and
reports, for the distilled kinds' single choices by group and subtype, the
student's agreement with N, with R, the share it answers neither, r1's own
agreement with R, and, where it answers neither, the actions it moved to
(R's, N's and the student's action by the candidate's priority column).
The reference shares the student's boards when every weight before the heads
is equal; otherwise it encodes its own.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
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


class Chains:
    """Mana chains under teacher forcing (validation's recorded states).

    A chain is a run of consecutive labelled single-choice decisions of the
    distilled kinds in one trajectory where R taps a mana ability; its first
    step is a tap whose previous labelled decision was not one (a casting
    sequence's start, in a main phase or at instant speed). Per position it
    counts the student's and r1's top-1 actions; per chain, whether each would
    have tapped at every step. In play one missed tap ends the sequence, so the
    chain rate bounds play from above.
    """

    def __init__(self, scope):
        self.scope = scope
        self.lanes = {}
        self.steps = {position: dict(decisions=0, student_mana=0, r1_mana=0, student_pass=0, student_cast=0)
                      for position in ("first", "later")}
        self.chains = []

    def close(self, lane):
        chain = self.lanes.get(lane, {}).get("chain")
        if chain:
            self.chains.append((len(chain), all(s for s, _ in chain), all(n for _, n in chain)))
        self.lanes[lane] = dict(prev=None, chain=[])

    def __call__(self, prepared, reference, batch, plans):
        get = lambda key: batch[key].cpu()
        valid, weight, heads = get("step_valid"), get("loss_weight"), get("head")
        decision = get("is_decision") & valid & (weight > 0) & (heads == 0)
        in_kind, _ = self.scope.masks(batch, torch.device("cpu"))
        for lane in torch.nonzero(get("is_new_traj"))[:, 0].tolist():
            self.close(lane)
        for t, plan in enumerate(plans):
            if not plan.heads[0]:
                continue
            student, r1 = prepared[t][1].float().cpu(), reference[t][1].float().cpu()
            mask = get("cand_mask")[:, t, :student.shape[1]]
            classes = D.action_classes(get("cand_feat")[:, t, :student.shape[1]])
            top = lambda logits: torch.where(mask, logits, -1e9).argmax(-1)
            r_actions = classes.gather(1, get("single_idx")[:, t].clamp(min=0)[:, None])[:, 0]
            s_actions = classes.gather(1, top(student)[:, None])[:, 0]
            n_actions = classes.gather(1, top(r1)[:, None])[:, 0]
            chosen = decision[:, t] & (mask.sum(-1) > 1) & in_kind[:, t]
            for lane in torch.nonzero(chosen)[:, 0].tolist():
                state = self.lanes.setdefault(lane, dict(prev=None, chain=[]))
                r_act, s_act, n_act = (D.ACTION_NAMES[int(x[lane])] for x in (r_actions, s_actions, n_actions))
                if r_act != "mana":
                    self.close(lane)
                    self.lanes[lane]["prev"] = r_act
                    continue
                counts = self.steps["later" if state["prev"] == "mana" else "first"]
                counts["decisions"] += 1
                counts["student_mana"] += s_act == "mana"
                counts["r1_mana"] += n_act == "mana"
                counts["student_pass"] += s_act == "pass"
                counts["student_cast"] += s_act == "cast"
                state["chain"].append((s_act == "mana", n_act == "mana"))
                state["prev"] = r_act

    def report(self):
        for lane in list(self.lanes):
            self.close(lane)
        steps = {position: {k: (v / c["decisions"] if k != "decisions" and c["decisions"] else v)
                            for k, v in c.items()} for position, c in self.steps.items()}
        n = len(self.chains)
        return dict(steps=steps, chains=n,
                    mean_length=sum(length for length, _, _ in self.chains) / n if n else None,
                    student_taps_through=sum(s for _, s, _ in self.chains) / n if n else None,
                    r1_taps_through=sum(r for _, _, r in self.chains) / n if n else None)


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
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--board-chunk", type=int, default=32)
    parser.add_argument("--decision-chunk", type=int, default=4)
    parser.add_argument("--board-token-budget", type=int, default=32768)
    parser.add_argument("--threads", type=int, default=8)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--out", default="")
    args = parser.parse_args(argv)
    torch.set_num_threads(args.threads)
    from value_calibration4.checkpoint import load_source

    _, _, student = load_source(args.student)
    _, _, r1 = load_source(args.reference)
    configure(student, args)
    configure(r1, args)
    dataset = D4.TrajectoryDataset4(Path(args.data).expanduser(), encoder_version=args.encoder_version, split="val",
                                    unroll_len=args.unroll)
    D.check_datasets([dataset], True)
    total_trajectories = len(dataset.trajectories)
    dataset.trajectories = dataset.trajectories[::args.every]
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

    chains = Chains(scope)
    metrics = D.evaluate_distill(student, reference, loader, torch.device("cpu"), scope, args.board_chunk, .05,
                                 bf16=False, actions=actions, progress=progress, observe=chains)
    groups = ["distilled", "out_of_scope", *(f"subtype:{name}" for name in ("unknown", *scope.subtype_names))]
    moves = sorted(({"subtype": s, "r_action": r, "n_action": n, "student_action": a, "count": c}
                    for (s, r, n, a), c in actions.items()), key=lambda m: -m["count"])
    report = dict(student=str(args.student), reference=str(args.reference), data=str(args.data),
                  trajectories=len(dataset.trajectories), of_trajectories=total_trajectories, every=args.every,
                  frames=metrics["val_frames"], shared_trunk=shared, seconds=round(time.perf_counter() - began),
                  rows=table_rows(metrics, groups), neither_moves=moves, mana_chains=chains.report(),
                  terms={name: metrics.get(f"val_distill_{name}") for name in D.TERMS})
    text = json.dumps(report, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(text, flush=True)
    return report


if __name__ == "__main__":
    main()
