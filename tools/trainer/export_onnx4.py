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
        h_in,
        thinking_steps,
    )


def export_v4_onnx(
    net: M.PlayerNet4,
    onnx_path: Path,
    thinking_steps: int = 2,
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
        "h_in": {0: "B"},
        "thinking_steps": {0: "B"},
        "h_out": {0: "B"},
        "cand_logits": {0: "B", 1: "C"},
        "pair_logits": {0: "B", 1: "P"},
        "scalar_pred": {0: "B"},
        "value": {0: "B"},
    }
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


def main() -> None:
    ap = argparse.ArgumentParser(description="Export Playernet v4 to ONNX and verify parity")
    ap.add_argument("--model", required=True, help="Model directory containing net.pt")
    ap.add_argument("--data", default="", help="Optional dataset directory for held-out parity checks")
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--thinking-steps", type=int, default=2)
    ap.add_argument("--check", type=int, default=32)
    args = ap.parse_args()

    model_dir = Path(args.model).expanduser()
    ckpt = torch.load(model_dir / "net.pt", map_location="cpu", weights_only=False)
    cfg = M.Config4(**ckpt["config"])
    cfg.default_thinking_steps = args.thinking_steps
    net = M.PlayerNet4(cfg)
    net.load_state_dict(ckpt["state_dict"])
    net.eval()

    onnx_path = model_dir / "model.onnx"
    export_v4_onnx(net, onnx_path, thinking_steps=args.thinking_steps)

    sess_opts = ort.SessionOptions()
    sess_opts.intra_op_num_threads = 1
    sess_opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    session = ort.InferenceSession(
        str(onnx_path), sess_options=sess_opts, providers=["CPUExecutionProvider"]
    )

    worst_diff = 0.0
    checked = 0

    if args.data:
        ds = D4.TrajectoryDataset4(
            Path(args.data).expanduser(), split="val", unroll_len=8, max_shards=1
        )
        if len(ds) == 0:
            ds = D4.TrajectoryDataset4(
                Path(args.data).expanduser(), split="train", unroll_len=8, max_shards=1
            )
        with torch.inference_mode():
            for seg_idx in range(min(len(ds), max(1, args.check // 4))):
                seg = ds[seg_idx]
                h_torch = torch.zeros(1, M.RECURRENT_SLOTS, cfg.d_model, dtype=torch.float32)
                h_ort = np.zeros((1, M.RECURRENT_SLOTS, cfg.d_model), dtype=np.float32)
                k_t = torch.tensor([args.thinking_steps], dtype=torch.int64)
                for t in range(seg["step_valid"].shape[0]):
                    if not seg["step_valid"][t]:
                        break
                    t_inputs = (
                        torch.from_numpy(seg["entity_feat"][t : t + 1]),
                        torch.from_numpy(seg["entity_dsl"][t : t + 1]),
                        torch.from_numpy(seg["entity_mask"][t : t + 1]),
                        torch.from_numpy(seg["seat_feat"][t : t + 1]),
                        torch.from_numpy(seg["global_feat"][t : t + 1]),
                        torch.from_numpy(seg["edge_index"][t : t + 1]),
                        torch.from_numpy(seg["edge_type"][t : t + 1]),
                        torch.from_numpy(seg["edge_mask"][t : t + 1]),
                        torch.from_numpy(seg["cand_feat"][t : t + 1]),
                        torch.from_numpy(seg["cand_entity_idx"][t : t + 1]),
                        torch.from_numpy(seg["cand_seat_idx"][t : t + 1]),
                        torch.from_numpy(seg["cand_mask"][t : t + 1]),
                        torch.from_numpy(seg["pair_index"][t : t + 1]),
                        torch.from_numpy(seg["pair_mask"][t : t + 1]),
                        h_torch,
                        k_t,
                    )
                    want = net(*t_inputs)
                    feeds = {
                        name: t_in.numpy()
                        for name, t_in in zip(M.INPUT_NAMES, t_inputs)
                    }
                    feeds["h_in"] = h_ort
                    got = session.run(None, feeds)
                    for g_arr, w_ten in zip(got, want):
                        w_arr = w_ten.numpy()
                        finite = np.isfinite(w_arr) & (np.abs(w_arr) < 1e8)
                        if finite.any():
                            diff = float(np.max(np.abs(g_arr[finite] - w_arr[finite])))
                            worst_diff = max(worst_diff, diff)
                    h_torch = want[0]
                    h_ort = got[0]
                    checked += 1
    else:
        with torch.inference_mode():
            for e_sz in (16, 48, 96):
                t_inputs = make_dummy_inputs(
                    cfg.d_model, B=1, E=e_sz, M_edges=32, C=24, P=16, K=args.thinking_steps
                )
                want = net(*t_inputs)
                feeds = {name: t_in.numpy() for name, t_in in zip(M.INPUT_NAMES, t_inputs)}
                got = session.run(None, feeds)
                for g_arr, w_ten in zip(got, want):
                    w_arr = w_ten.numpy()
                    finite = np.isfinite(w_arr) & (np.abs(w_arr) < 1e8)
                    if finite.any():
                        worst_diff = max(
                            worst_diff, float(np.max(np.abs(g_arr[finite] - w_arr[finite])))
                        )
                checked += 1

    # Also verify that K in (1, 2, 4, 8) produce distinct ONNX outputs and match PyTorch for every K
    with torch.inference_mode():
        base_inputs = list(make_dummy_inputs(cfg.d_model, B=1, E=24, M_edges=32, C=16, P=16, K=1))
        k_cand_logits: dict[int, np.ndarray] = {}
        for k_val in (1, 2, 4, 8):
            base_inputs[-1] = torch.tensor([k_val], dtype=torch.int64)
            t_in_tuple = tuple(base_inputs)
            want_k = net(*t_in_tuple)
            feeds_k = {name: t_in.numpy() for name, t_in in zip(M.INPUT_NAMES, t_in_tuple)}
            got_k = session.run(None, feeds_k)
            for g_arr, w_ten in zip(got_k, want_k):
                w_arr = w_ten.numpy()
                finite = np.isfinite(w_arr) & (np.abs(w_arr) < 1e8)
                if finite.any():
                    worst_diff = max(
                        worst_diff, float(np.max(np.abs(g_arr[finite] - w_arr[finite])))
                    )
            k_cand_logits[k_val] = got_k[1].copy()
        for k_a, k_b in ((1, 2), (2, 4), (4, 8)):
            k_delta = float(np.max(np.abs(k_cand_logits[k_a] - k_cand_logits[k_b])))
            if k_delta < 1e-6:
                raise SystemExit(
                    f"[export_onnx4] K-step loop failed: K={k_a} and K={k_b} produced identical outputs (delta={k_delta:.2e})"
                )

    if worst_diff > 1e-3:
        raise SystemExit(
            f"[export_onnx4] parity check failed: max |diff| = {worst_diff:.2e} > 1e-3"
        )

    meta = {
        "encoder_version": 4,
        "d_model": cfg.d_model,
        "n_layers": cfg.n_layers,
        "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps,
        "entities": args.entities,
        "inputs": {
            "global_feat": [1, M.GLOBAL_FEAT_WIDTH],
            "entity_feat": [1, args.entities, M.ENTITY_FEAT_WIDTH],
        },
        "outputs": list(M.OUTPUT_NAMES),
        "config": cfg.to_dict(),
        "parity_max_abs_diff": worst_diff,
        "checked_decisions": checked,
    }
    (model_dir / "meta.json").write_text(json.dumps(meta, indent=2))
    (model_dir / "model.onnx.json").write_text(json.dumps(meta, indent=2))
    print(
        f"[export_onnx4] wrote {onnx_path} ({onnx_path.stat().st_size / 1e6:.2f} MB) | "
        f"checked={checked} max_abs_diff={worst_diff:.2e}"
    )


if __name__ == "__main__":
    main()
