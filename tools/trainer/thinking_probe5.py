"""How much do a checkpoint's thinking steps change its answers? (CPU measurement.)

Replays held-out trajectories with the memory a seat would have, and at every
real decision reads the candidate logits at each depth K from the same memory
and board. Reports, against the reference depth: how often the top answer
differs, the size of the logit change, how far the scratchpad moves, and the
top-1 agreement with the recorded target per depth. A depth whose answers never
differ from the reference cannot change play.

A checkpoint with the deliberation block (thinking revision 2) is read step by
step instead (`probe_steps`): what each step corrects and breaks of the first
proposal, and what its confidence would make a seat think through again.

Usage:
    python3 tools/trainer/thinking_probe5.py --model DIR --data DATASET [--decisions 2000]
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch

import dataset4 as D4
import model4 as M
from export_onnx4 import load_export_checkpoint


def probe(net, dataset, encoder_depths, reference, limit):
    stats = {k: dict(flips=0, target_top1=0, logit_delta=0.0, margin_ratio=0.0) for k in encoder_depths}
    decisions = single = 0
    scratch_move = {k: 0.0 for k in encoder_depths}
    captured = {}
    hook = net.h_norm.register_forward_hook(lambda _m, _a, out: captured.__setitem__("z", out))
    try:
        for trajectory in dataset.trajectories:
            h = torch.zeros(1, M.RECURRENT_SLOTS, net.cfg.d_model)
            for seg_idx in trajectory:
                seg = dataset[seg_idx]
                for t in range(len(seg["step_valid"])):
                    if not seg["step_valid"][t]:
                        continue
                    inputs = tuple(torch.from_numpy(seg[name][t:t + 1]) for name in M.INPUT_NAMES[:-2])
                    board = net.encode_board(*inputs[:8], inputs[15])
                    h = net(*inputs, h, torch.zeros(1, dtype=torch.long), encoded_board=board)[0]
                    if not seg["is_decision"][t] or int(seg["cand_mask"][t].sum()) < 2:
                        continue
                    logits, scratch = {}, {}
                    for k in sorted({*encoder_depths, reference}):
                        logits[k] = net(*inputs, h, torch.full((1,), k, dtype=torch.long),
                                        encoded_board=board)[1][0]
                        scratch[k] = captured["z"][0]
                    decisions += 1
                    is_single = int(seg["head"][t]) == 0 and float(seg["loss_weight"][t]) > 0
                    single += is_single
                    ref = logits[reference]
                    valid = ref > -1e8
                    ref_c = ref[valid] - ref[valid].mean()
                    top2 = ref[valid].topk(2).values
                    margin = float(top2[0] - top2[1])
                    for k in encoder_depths:
                        cur = logits[k][valid] - logits[k][valid].mean()
                        delta = float((cur - ref_c).abs().max())
                        stats[k]["flips"] += int(logits[k].argmax() != ref.argmax())
                        stats[k]["logit_delta"] += delta
                        stats[k]["margin_ratio"] += delta / max(margin, 1e-6) if margin > 1e-6 else 0.0
                        if is_single:
                            stats[k]["target_top1"] += int(int(logits[k].argmax()) == int(seg["single_idx"][t]))
                        scratch_move[k] += float((scratch[k] - scratch[reference]).norm() / scratch[reference].norm())
                    if decisions >= limit:
                        raise StopIteration
    except StopIteration:
        pass
    finally:
        hook.remove()
    n = max(decisions, 1)
    return {"decisions": decisions, "single_choice_with_target": single, "reference_depth": reference,
            "per_depth": {str(k): {"top_answer_differs": s["flips"] / n,
                                   "mean_max_centered_logit_change": s["logit_delta"] / n,
                                   "mean_change_over_top2_margin": s["margin_ratio"] / n,
                                   "scratchpad_relative_move": scratch_move[k] / n,
                                   "target_top1": s["target_top1"] / max(single, 1)}
                          for k, s in stats.items()}}


def probe_steps(net, dataset, limit):
    """Thinking revision 2: what each deliberation step does to the answer.

    Step 0 is the heads' first proposal (the source net's own answer). Per
    step: top-1 against the recorded target, how many first-proposal mistakes
    it has corrected and how many right answers it has broken. Then how well
    the final confidence is calibrated, and for a list of thresholds how many
    decisions a seat would think through again and how right both groups are.
    """
    block = net.deliberation
    block.collect, block.steps_override = True, block.steps
    steps = block.steps
    right = torch.zeros(steps + 1)
    corrected = torch.zeros(steps + 1)
    broken = torch.zeros(steps + 1)
    confidences, finals = [], []
    decisions = first_wrong = 0
    try:
        for trajectory in dataset.trajectories:
            h = torch.zeros(1, M.RECURRENT_SLOTS, net.cfg.d_model)
            for seg_idx in trajectory:
                seg = dataset[seg_idx]
                for t in range(len(seg["step_valid"])):
                    if not seg["step_valid"][t]:
                        continue
                    inputs = tuple(torch.from_numpy(seg[name][t:t + 1]) for name in M.INPUT_NAMES[:-2])
                    board = net.encode_board(*inputs[:8], inputs[15])
                    h = net(*inputs, h, torch.zeros(1, dtype=torch.long), encoded_board=board)[0]
                    if (not seg["is_decision"][t] or int(seg["head"][t]) != 0
                            or float(seg["loss_weight"][t]) <= 0 or int(seg["cand_mask"][t].sum()) < 2):
                        continue
                    block.trace = None
                    net(*inputs, h, torch.full((1,), net.cfg.scratchpad_steps, dtype=torch.long),
                        encoded_board=board)
                    answers = block.trace["logits"][:, 0].argmax(-1)
                    target = int(seg["single_idx"][t])
                    hit = (answers == target).float()
                    right += hit
                    if hit[0]:
                        broken += 1 - hit
                    else:
                        first_wrong += 1
                        corrected += hit
                    confidences.append(float(block.last_confidence[0]))
                    finals.append(bool(hit[-1]))
                    decisions += 1
                    if decisions >= limit:
                        raise StopIteration
    except StopIteration:
        pass
    n = max(decisions, 1)
    confidence, final = torch.tensor(confidences), torch.tensor(finals, dtype=torch.float32)
    ece = 0.0
    for low in torch.arange(0, 1, 0.1):
        inside = (confidence >= low) & (confidence < low + 0.1 + (1e-6 if low > 0.85 else 0))
        if inside.any():
            ece += float(inside.float().mean() * (confidence[inside].mean() - final[inside].mean()).abs())
    thresholds = {}
    for tau in (0.5, 0.6, 0.7, 0.8, 0.9):
        again = confidence < tau
        thresholds[str(tau)] = {
            "thought_again_share": float(again.float().mean()) if decisions else 0.0,
            "right_among_kept": float(final[~again].mean()) if (~again).any() else None,
            "right_among_thought_again": float(final[again].mean()) if again.any() else None}
    return {"decisions": decisions, "first_proposal_wrong": first_wrong, "steps": steps,
            "right_by_step": (right / n).tolist(),
            "first_mistakes_corrected_by_step": (corrected / max(first_wrong, 1)).tolist(),
            "right_answers_broken_by_step": (broken / max(decisions - first_wrong, 1)).tolist(),
            "final_confidence_ece": ece, "thresholds": thresholds}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--decisions", type=int, default=2000)
    ap.add_argument("--depths", default="0,1,2,4,8")
    ap.add_argument("--reference", type=int, default=2)
    ap.add_argument("--shards", type=int, default=1)
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    torch.set_num_threads(4)
    checkpoint, net, _ = load_export_checkpoint(Path(args.model).expanduser() / "net.pt")
    net.rules = net.rules.freeze()
    dataset = D4.TrajectoryDataset4(Path(args.data).expanduser(), split="val", unroll_len=16,
                                    max_shards=args.shards, encoder_version=checkpoint["encoder_version"])
    with torch.inference_mode():
        if getattr(net, "deliberation", None) is not None:
            report = probe_steps(net, dataset, args.decisions)
        else:
            report = probe(net, dataset, [int(k) for k in args.depths.split(",")], args.reference, args.decisions)
    text = json.dumps(report, indent=2)
    print(text)
    if args.out:
        Path(args.out).expanduser().write_text(text + "\n")


if __name__ == "__main__":
    main()
