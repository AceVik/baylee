"""Exports a Playernet v4 checkpoint (`net.pt`) to ONNX (`model.onnx`) and
verifies PyTorch vs ONNX Runtime parity across held-out steps and varying
entity/edge/candidate lengths.

Usage:
    python3 tools/trainer/export_onnx4.py \
        --model ~/baylee-data/models/v4-pilot \
        --data ~/baylee-data/datasets/v4-pilot
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch

import dataset4 as D4
import model4 as M


def make_dummy_inputs(
    d_model: int,
    B: int = 1,
    E: int = 64,
    M_edges: int = 64,
    C: int = 32,
    P: int = 32,
    K: int = 2,
) -> tuple[torch.Tensor, ...]:
    entity_feat = torch.randn(B, E, M.ENTITY_FEAT_WIDTH, dtype=torch.float32)
    entity_dsl = torch.randn(B, E, M.CARD_DSL_WIDTH, dtype=torch.float32)
    entity_mask = torch.ones(B, E, dtype=torch.bool)
    seat_feat = torch.zeros(B, M.MAX_SEATS, M.SEAT_FEAT_WIDTH, dtype=torch.float32)
    seat_feat[:, :4, M.SEAT_FEAT_PRESENT] = 1.0  # 4 present seats
    seat_feat[:, 0, M.SEAT_FEAT_SELF] = 1.0      # seat 0 is self
    seat_feat[:, 1:4, M.SEAT_FEAT_OPPONENT] = 1.0  # seats 1..3 are opponents
    seat_feat[:, :4, M.SEAT_FEAT_LOST] = 0.0     # none lost
    global_feat = torch.randn(B, M.GLOBAL_FEAT_WIDTH, dtype=torch.float32)
    edge_index = torch.zeros(B, M_edges, 2, dtype=torch.int64)
    edge_type = torch.zeros(B, M_edges, dtype=torch.int64)
    edge_mask = torch.ones(B, M_edges, dtype=torch.bool)
    cand_feat = torch.randn(B, C, M.CAND_FEAT_WIDTH, dtype=torch.float32)
    cand_entity_idx = torch.zeros(B, C, dtype=torch.int64)
    cand_seat_idx = torch.full((B, C), -1, dtype=torch.int64)
    cand_mask = torch.ones(B, C, dtype=torch.bool)
    pair_index = torch.zeros(B, P, 2, dtype=torch.int64)
    pair_mask = torch.ones(B, P, dtype=torch.bool)
    h_in = torch.zeros(B, M.RECURRENT_SLOTS, d_model, dtype=torch.float32)
    thinking_steps = torch.full((B,), K, dtype=torch.int64)
    return (
        entity_feat,
        entity_dsl,
        entity_mask,
        seat_feat,
        global_feat,
        edge_index,
        edge_type,
        edge_mask,
        cand_feat,
        cand_entity_idx,
        cand_seat_idx,
        cand_mask,
        pair_index,
        pair_mask,
        torch.zeros(B, C, M.CARD_DSL_WIDTH),
        h_in,
        thinking_steps,
    )


def export_v4_onnx(
    net: M.PlayerNet4,
    onnx_path: Path,
    thinking_steps: int = 2,
    fixed_depth: bool = False,
) -> None:
    net = net.cpu().eval()
    net.cfg.default_thinking_steps = thinking_steps
    dummy = make_dummy_inputs(net.cfg.d_model, K=thinking_steps)
    dynamic_axes = {
        "entity_feat": {0: "B", 1: "E"},
        "entity_dsl": {0: "B", 1: "E"},
        "entity_mask": {0: "B", 1: "E"},
        "seat_feat": {0: "B"},
        "global_feat": {0: "B"},
        "edge_index": {0: "B", 1: "M"},
        "edge_type": {0: "B", 1: "M"},
        "edge_mask": {0: "B", 1: "M"},
        "cand_feat": {0: "B", 1: "C"},
        "cand_entity_idx": {0: "B", 1: "C"},
        "cand_seat_idx": {0: "B", 1: "C"},
        "cand_mask": {0: "B", 1: "C"},
        "pair_index": {0: "B", 1: "P"},
        "pair_mask": {0: "B", 1: "P"},
        "cand_dsl": {0: "B", 1: "C"},
        "h_in": {0: "B"},
        "thinking_steps": {0: "B"},
        "h_out": {0: "B"},
        "cand_logits": {0: "B", 1: "C"},
        "pair_logits": {0: "B", 1: "P"},
        "scalar_pred": {0: "B"},
        "value": {0: "B"},
    }
    if fixed_depth:
        net.export_thinking_steps = thinking_steps
    torch.onnx.export(
        net,
        dummy,
        str(onnx_path),
        input_names=list(M.INPUT_NAMES),
        output_names=list(M.OUTPUT_NAMES),
        dynamic_axes=dynamic_axes,
        opset_version=18,
        dynamo=False,
    )
    if fixed_depth:
        del net.export_thinking_steps


def main() -> None:
    ap = argparse.ArgumentParser(description="Export bounded-depth Playernet v4 graphs")
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", default="")
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--thinking-steps", type=int, default=2, choices=(1, 2, 4, 8))
    ap.add_argument("--check", type=int, default=32)
    args = ap.parse_args()
    torch.set_num_threads(1)
    model_dir = Path(args.model).expanduser()
    ckpt = torch.load(model_dir / "net.pt", map_location="cpu", weights_only=False)
    if (ckpt.get("trajectory_version") != M.TRAJECTORY_VERSION or ckpt.get("model_version") != M.MODEL_VERSION
            or ckpt.get("encoder_version") != M.ENCODER_VERSION):
        raise ValueError("Checkpoint uses the old recurrent memory contract")
    cfg = M.Config4(**ckpt["config"])
    net = M.PlayerNet4(cfg).eval()
    net.load_state_dict(ckpt["state_dict"], strict=True)
    options = ort.SessionOptions()
    options.intra_op_num_threads = 1
    sessions = {}
    worst_diff = 0.0
    checked = 0

    def compare(inputs, depth, h_ort=None):
        nonlocal worst_diff, checked
        with torch.inference_mode():
            want = net(*inputs)
        feed = {name: tensor.numpy() for name, tensor in zip(M.INPUT_NAMES, inputs)}
        if h_ort is not None:
            feed["h_in"] = h_ort
        got = sessions[depth].run(list(M.OUTPUT_NAMES), feed)
        for name, actual, expected in zip(M.OUTPUT_NAMES, got, want):
            ref = expected.numpy()
            if actual.shape != ref.shape and name in ("cand_logits", "pair_logits"):
                # Eager execution trims unused padding per time step; ONNX
                # preserves dynamic input lengths and masks that padding.
                if not np.all(actual[:, ref.shape[1]:] < -1e8):
                    raise ValueError(f"Unmasked ONNX padding: {name}")
                actual = actual[:, :ref.shape[1]]
            if not np.isfinite(actual).all() or not np.isfinite(ref).all():
                raise ValueError("Non-finite ONNX parity result")
            finite = np.abs(ref) < 1e8
            if finite.any():
                worst_diff = max(worst_diff, float(np.max(np.abs(actual[finite] - ref[finite]))))
        checked += 1
        return want[0], got[0]

    for depth in (0, 1, 2, 4, 8):
        path = model_dir / f"model.k{depth}.onnx"
        export_v4_onnx(net, path, thinking_steps=depth, fixed_depth=True)
        sessions[depth] = ort.InferenceSession(str(path), sess_options=options,
                                             providers=["CPUExecutionProvider"])
        for e in (1, 17, 96):
            compare(make_dummy_inputs(cfg.d_model, E=e, M_edges=1, C=7, P=1, K=depth), depth)

    if args.data:
        ds = D4.TrajectoryDataset4(Path(args.data).expanduser(), split="val", unroll_len=16,
                                  max_shards=1)
        for trajectory in ds.trajectories:
            h_pt = torch.zeros(1, M.RECURRENT_SLOTS, cfg.d_model)
            h_ort = h_pt.numpy().copy()
            for seg_idx in trajectory:
                seg = ds[seg_idx]
                for t in range(len(seg["step_valid"])):
                    if not seg["step_valid"][t]:
                        continue
                    depth = args.thinking_steps if seg["is_decision"][t] else 0
                    inputs = tuple(torch.from_numpy(seg[name][t:t+1]) for name in M.INPUT_NAMES[:-2])
                    inputs += (h_pt, torch.tensor([depth], dtype=torch.int64))
                    h_pt, h_ort = compare(inputs, depth, h_ort)
                    if checked >= args.check + 15:
                        break
                if checked >= args.check + 15:
                    break
            if checked >= args.check + 15:
                break
    if worst_diff > 1e-3:
        raise ValueError(f"ONNX parity failed: {worst_diff:.3g}")
    shutil.copyfile(model_dir / f"model.k{args.thinking_steps}.onnx", model_dir / "model.onnx")
    meta = {
        "encoder_version": M.ENCODER_VERSION, "model_version": M.MODEL_VERSION,
        "trajectory_version": M.TRAJECTORY_VERSION,
        "sampling_version": M.SAMPLING_VERSION,
        "d_model": cfg.d_model, "n_layers": cfg.n_layers, "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps, "config": cfg.to_dict(),
        "graphs": {str(k): f"model.k{k}.onnx" for k in sessions},
        "parity_max_abs_diff": worst_diff, "checked_frames": checked,
    }
    (model_dir / "model.onnx.json").write_text(json.dumps(meta, indent=2))
    print(f"[export4] verified {checked} frames, max error={worst_diff:.3g}; exported K=0,1,2,4,8")


if __name__ == "__main__":
    main()
