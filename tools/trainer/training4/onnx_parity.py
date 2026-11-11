"""Exported ONNX graphs against their torch checkpoint on real validation frames.

    CUDA_VISIBLE_DEVICES= python -m training4.onnx_parity --model DIR --graphs DIR/fp16 \
        --data DATASET --every 8 --max-decisions 600 --out parity.json

Walks whole validation trajectories as a player does: an observation runs the
`model.k0.onnx` graph, a decision `model.k{--thinking-steps}.onnx`, the graphs
eval4 runs (`netplay4::depth_graph`). Torch (the module the exporter traced,
`export_onnx4.export_graph`) and ONNX each carry their own memory, as in play;
at every decision the ONNX graph also runs on torch's memory, which separates
the graph's own difference from the drift of the memory it carries. Reports,
at the recorded seat's single-choice decisions by distillation subtype, top-1
agreement of the candidate logits, the largest logit difference, and how often
each side's top-1 is R, N's action class and a mana ability.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import time

import numpy as np
import onnxruntime as ort
import torch

import dataset4 as D4
import model4 as M
from . import distill as D


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--model", required=True, help="Directory with net.pt")
    parser.add_argument("--graphs", default="", help="Directory with model.k*.onnx (default --model)")
    parser.add_argument("--data", required=True)
    parser.add_argument("--thinking-steps", type=int, default=2)
    parser.add_argument("--every", type=int, default=8, help="Every k-th validation trajectory")
    parser.add_argument("--max-decisions", type=int, default=600)
    parser.add_argument("--labelled-only", action="store_true",
                        help="Skip a trajectory whose first 64 decisions carry no label (an unrecorded seat), "
                             "and count labelled decisions only")
    parser.add_argument("--threads", type=int, default=8)
    parser.add_argument("--out", default="")
    args = parser.parse_args(argv)
    torch.set_num_threads(args.threads)
    from export_onnx4 import load_export_checkpoint
    model = Path(args.model).expanduser()
    graphs = Path(args.graphs).expanduser() if args.graphs else model
    checkpoint, net, _ = load_export_checkpoint(model / "net.pt")
    digest = net.rules.digest
    net.rules = net.rules.freeze()
    options = ort.SessionOptions()
    options.intra_op_num_threads = args.threads
    sessions = {depth: ort.InferenceSession(str(graphs / f"model.k{depth}.onnx"), sess_options=options,
                                            providers=["CPUExecutionProvider"])
                for depth in (0, args.thinking_steps)}
    dataset = D4.TrajectoryDataset4(Path(args.data).expanduser(), split="val", unroll_len=32,
                                    encoder_version=checkpoint["encoder_version"])
    D.check_datasets([dataset], True)
    if dataset.structure.digest != digest:
        raise ValueError("Dataset and checkpoint use different public rule catalogs")
    names = dict(enumerate(dataset.distill_subtypes))
    rows, began, frames, worst_h = [], time.perf_counter(), 0, 0.
    flags = np.asarray(dataset.store.index["flags"])
    for trajectory in dataset.trajectories[::args.every]:
        if args.labelled_only:
            stored = np.concatenate([dataset.segments[s] for s in trajectory])
            decisions = [int(r) for r in stored if flags[r] & 1][:64]
            if not any(float(dataset.encode_single_step(r)["loss_weight"]) > 0 for r in decisions):
                continue
        h_pt = torch.zeros(1, M.RECURRENT_SLOTS, net.cfg.d_model)
        h_ort = h_pt.numpy().copy()
        for segment in trajectory:
            seg = dataset[segment]
            for t in range(len(seg["step_valid"])):
                if not seg["step_valid"][t]:
                    continue
                decision = bool(seg["is_decision"][t])
                depth = args.thinking_steps if decision else 0
                inputs = tuple(torch.from_numpy(seg[name][t:t + 1]) for name in M.INPUT_NAMES[:-2])
                feed = {name: tensor.numpy() for name, tensor in zip(M.INPUT_NAMES, inputs)}
                feed["thinking_steps"] = np.array([depth], dtype=np.int64)
                with torch.inference_mode():
                    h_next, cand, *_ = net(*inputs, h_pt, torch.tensor([depth], dtype=torch.int64))
                feed["h_in"] = h_ort
                h_ort_next, cand_chained = sessions[depth].run(["h_out", "cand_logits"], feed)
                single = decision and int(seg["head"][t]) == 0
                labelled = single and float(seg["loss_weight"][t]) > 0
                mask = seg["cand_mask"][t]
                if single and mask.sum() > 1 and (labelled or not args.labelled_only):
                    feed["h_in"] = h_pt.numpy()
                    _, cand_isolated = sessions[depth].run(["h_out", "cand_logits"], feed)
                    width = cand.shape[1]
                    valid = mask[:width]
                    pick = lambda logits: int(np.where(valid, logits[0, :width], -np.inf).argmax())
                    torch_top, chained_top, isolated_top = pick(cand.numpy()), pick(cand_chained), pick(cand_isolated)
                    classes = D.action_classes(torch.from_numpy(seg["cand_feat"][t, :width])).tolist()
                    rows.append(dict(
                        subtype=(names.get(int(seg["distill_subtype"][t]), "unknown") if labelled
                                 else "unlabelled"),
                        r=int(seg["single_idx"][t]) if labelled else -1, torch_top=torch_top,
                        chained_top=chained_top,
                        isolated_top=isolated_top,
                        torch_action=D.ACTION_NAMES[classes[torch_top]],
                        chained_action=D.ACTION_NAMES[classes[chained_top]],
                        max_logit_diff_isolated=float(np.abs(cand_isolated[0, :width] - cand.numpy()[0])[valid].max()),
                        max_logit_diff_chained=float(np.abs(cand_chained[0, :width] - cand.numpy()[0])[valid].max())))
                worst_h = max(worst_h, float(np.abs(h_ort_next - h_next.numpy()).max()))
                h_pt, h_ort = h_next, h_ort_next
                frames += 1
                if frames % 200 == 0:
                    print(f"[parity] decisions={len(rows)} frames={frames} "
                          f"seconds={time.perf_counter() - began:.0f}", flush=True)
            if len(rows) >= args.max_decisions:
                break
        print(f"[parity] decisions={len(rows)} frames={frames} seconds={time.perf_counter() - began:.0f}", flush=True)
        if len(rows) >= args.max_decisions:
            break
    report = dict(model=str(model), graphs=str(graphs), frames=frames, decisions=len(rows),
                  max_memory_diff_chained=worst_h, by_subtype=summarize(rows))
    text = json.dumps(report, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
        Path(args.out).with_suffix(".rows.json").write_text(json.dumps(rows) + "\n")
    print(text, flush=True)
    return report


def summarize(rows):
    groups = {}
    for row in rows:
        for name in (row["subtype"], "all"):
            groups.setdefault(name, []).append(row)
    out = {}
    for name, items in sorted(groups.items()):
        n = len(items)
        labelled = [r for r in items if r["r"] >= 0]
        out[name] = dict(
            decisions=n,
            top1_agree_isolated=sum(r["torch_top"] == r["isolated_top"] for r in items) / n,
            top1_agree_chained=sum(r["torch_top"] == r["chained_top"] for r in items) / n,
            max_logit_diff_isolated=max(r["max_logit_diff_isolated"] for r in items),
            max_logit_diff_chained=max(r["max_logit_diff_chained"] for r in items),
            torch_top1_r=sum(r["torch_top"] == r["r"] for r in labelled) / len(labelled) if labelled else None,
            onnx_top1_r=sum(r["chained_top"] == r["r"] for r in labelled) / len(labelled) if labelled else None,
            torch_mana=sum(r["torch_action"] == "mana" for r in items) / n,
            onnx_mana=sum(r["chained_action"] == "mana" for r in items) / n)
    return out


if __name__ == "__main__":
    main()
