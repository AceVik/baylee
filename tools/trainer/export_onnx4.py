"""Exports a versioned Playernet v4/v5 checkpoint (`net.pt`) to ONNX (`model.onnx`) and
verifies PyTorch vs ONNX Runtime parity across held-out steps and varying
entity/edge/candidate lengths.

Usage:
    python3 tools/trainer/export_onnx4.py \
        --model ~/baylee-data/models/v4-pilot \
        --data ~/baylee-data/datasets/v4-pilot
"""

from __future__ import annotations

import argparse
from pathlib import Path
from torch_io4 import atomic_copy, atomic_json, file_digest

import numpy as np
import onnxruntime as ort
import torch

import dataset4 as D4
import model4 as M


def export_critic_semantics(checkpoint):
    """Keep the value target explicit without copying the large training receipt."""
    if 'research_ppo_progress' in checkpoint and 'research_ppo_contract' not in checkpoint:
        raise ValueError('Research PPO progress requires its objective contract')
    if 'research_ppo_contract' in checkpoint:
        from research_ppo4.checkpoint import validate_resume
        validate_resume(checkpoint)
        contract = checkpoint['research_ppo_contract']
        semantics = dict(kind='failure_terminal_research_utility_v1',
                         objective=contract['objective'], target_contract=contract['target_contract'])
    else:
        semantics = dict(kind='mtg_side_outcome_utility_v1')
    if 'critic_semantics' in checkpoint and checkpoint['critic_semantics'] != semantics:
        raise ValueError('Checkpoint critic_semantics conflicts with its validated training objective')
    return semantics


def load_export_checkpoint(checkpoint_path: Path):
    """Construct only the architecture explicitly declared by the checkpoint.

    V5 keeps the E7 input ABI, but its learned relational layers are not a V4
    state-dict extension that can be silently ignored by an older exporter.
    """
    checkpoint = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
    export_critic_semantics(checkpoint)
    version = checkpoint.get("model_version")
    if version == M.MODEL_VERSION:
        module = M
        config_type, model_type = M.Config4, M.PlayerNet4
    elif version == 5:
        import model5 as module
        config_type, model_type = module.Config5, module.PlayerNet5
        if checkpoint.get("architecture") not in (*module.ARCHITECTURES.values(), *module.CANDIDATE_ARCHITECTURES.values()):
            raise ValueError("V5 checkpoint architecture is missing or incompatible")
    else:
        raise ValueError(f"Unsupported checkpoint model_version: {version}")
    for key, expected in (("trajectory_version", module.TRAJECTORY_VERSION),
                          ("sampling_version", module.SAMPLING_VERSION)):
        if checkpoint.get(key) != expected:
            raise ValueError(f"Checkpoint {key} is incompatible with this exporter")
    for key in ("INPUT_NAMES", "OUTPUT_NAMES", "ENTITY_FEAT_WIDTH", "SEAT_FEAT_WIDTH",
                "GLOBAL_FEAT_WIDTH", "CAND_FEAT_WIDTH", "CARD_DSL_WIDTH", "RECURRENT_SLOTS"):
        if getattr(module, key) != getattr(M, key):
            raise ValueError(f"Checkpoint architecture does not use the shared E7 tensor ABI: {key}")
    config_values = dict(checkpoint["config"])
    if version == 5 and "relational_revision" not in config_values:
        if checkpoint["architecture"] != module.ARCHITECTURES[1]:
            raise ValueError("V5 architecture requires an explicit matching relational revision")
        config_values["relational_revision"] = 1
    config = config_type(**config_values)
    if version == 5 and checkpoint["architecture"] != module.architecture_for_config(config):
        raise ValueError("V5 architecture and relational revision disagree")
    from encoder_contract import validate_encoder
    validate_encoder(checkpoint.get("encoder_version"), model_version=version,
                     relational_revision=getattr(config, "relational_revision", 2),
                     candidate_context_revision=getattr(config, "candidate_context_revision", 1),
                     global_feat_width=config.global_feat_width)
    net = model_type(config).eval()
    net.attach_structure(checkpoint["structure"])
    net.load_state_dict(checkpoint["state_dict"], strict=True)
    net.rules.eval()
    return checkpoint, net, module


def make_dummy_inputs(
    d_model: int,
    B: int = 1,
    E: int = 64,
    M_edges: int = 64,
    C: int = 32,
    P: int = 32,
    K: int = 2,
    global_feat_width: int = M.GLOBAL_FEAT_WIDTH,
) -> tuple[torch.Tensor, ...]:
    entity_feat = torch.randn(B, E, M.ENTITY_FEAT_WIDTH, dtype=torch.float32)
    entity_dsl = torch.randn(B, E, M.CARD_DSL_WIDTH, dtype=torch.float32)
    entity_mask = torch.ones(B, E, dtype=torch.bool)
    seat_feat = torch.zeros(B, M.MAX_SEATS, M.SEAT_FEAT_WIDTH, dtype=torch.float32)
    seat_feat[:, :4, M.SEAT_FEAT_PRESENT] = 1.0  # 4 present seats
    seat_feat[:, 0, M.SEAT_FEAT_SELF] = 1.0      # seat 0 is self
    seat_feat[:, 1:4, M.SEAT_FEAT_OPPONENT] = 1.0  # seats 1..3 are opponents
    seat_feat[:, :4, M.SEAT_FEAT_LOST] = 0.0     # none lost
    global_feat = torch.randn(B, global_feat_width, dtype=torch.float32)
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
        torch.zeros(B, E, 1, 2, dtype=torch.long),
        torch.zeros(B, C, 1, 2, dtype=torch.long),
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
    from model4.structure import RuleEncoder
    if isinstance(net.rules, RuleEncoder):
        net.rules = net.rules.freeze()
    dummy = make_dummy_inputs(net.cfg.d_model, K=thinking_steps, global_feat_width=net.cfg.global_feat_width)
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
        "entity_rules": {0: "B", 1: "E", 2: "R"},
        "cand_rules": {0: "B", 1: "C", 2: "Q"},
        "h_in": {0: "B"},
        "thinking_steps": {0: "B"},
        "h_out": {0: "B"},
        "cand_logits": {0: "B", 1: "C"},
        "pair_logits": {0: "B", 1: "P"},
        "scalar_pred": {0: "B"},
        "value": {0: "B"},
    }
    previous_depth = getattr(net, "export_thinking_steps", None)
    if fixed_depth:
        net.export_thinking_steps = thinking_steps
    try:
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
    finally:
        if fixed_depth:
            if previous_depth is None:
                del net.export_thinking_steps
            else:
                net.export_thinking_steps = previous_depth


def convert_to_float16(path: Path) -> None:
    """Rewrites a graph to FP16 weights and arithmetic in place.

    Inputs and outputs stay FP32, so the Rust ABI and the recurrent memory
    handed between steps are unchanged. Constants beyond FP16's range (the
    -1e9 masks) are clamped to its largest finite value.
    """
    import onnx
    from onnxruntime.transformers.float16 import convert_float_to_float16

    converted = convert_float_to_float16(onnx.load(str(path)), keep_io_types=True)
    partial = path.with_suffix(".fp16.partial")
    onnx.save(converted, str(partial))
    partial.replace(path)


def main() -> None:
    ap = argparse.ArgumentParser(description="Export bounded-depth Playernet v4/v5 graphs")
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", default="")
    ap.add_argument("--entities", type=int, default=192)
    ap.add_argument("--thinking-steps", type=int, default=2, choices=(1, 2, 4, 8))
    ap.add_argument("--check", type=int, default=32)
    ap.add_argument("--precision", choices=("float32", "float16"), default="float32",
                    help="float16 computes in FP16 behind FP32 inputs and outputs, in its own directory")
    ap.add_argument("--out", default="",
                    help="Export directory (default: the model directory; MODEL/fp16 for float16)")
    args = ap.parse_args()
    torch.set_num_threads(1)
    model_dir = Path(args.model).expanduser()
    half = args.precision == "float16"
    out_dir = Path(args.out).expanduser() if args.out else model_dir / "fp16" if half else model_dir
    if half and out_dir.resolve() == model_dir.resolve():
        raise ValueError("A float16 export needs its own directory; the float32 graphs stay")
    out_dir.mkdir(parents=True, exist_ok=True)
    checkpoint_path = model_dir / "net.pt"
    source_checkpoint_sha256 = file_digest(checkpoint_path)
    ckpt, net, model_module = load_export_checkpoint(checkpoint_path)
    cfg = net.cfg
    structure_sha256 = net.rules.digest
    net.rules = net.rules.freeze()
    atomic_json(out_dir / "card_structure.json", ckpt["structure"])
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
        path = out_dir / f"model.k{depth}.onnx"
        export_v4_onnx(net, path, thinking_steps=depth, fixed_depth=True)
        sessions[depth] = ort.InferenceSession(str(path), sess_options=options,
                                             providers=["CPUExecutionProvider"])
        for e in (1, 17, 96):
            compare(make_dummy_inputs(cfg.d_model, E=e, M_edges=1, C=7, P=1, K=depth, global_feat_width=cfg.global_feat_width), depth)

    if args.data:
        ds = D4.TrajectoryDataset4(Path(args.data).expanduser(), split="val", unroll_len=16,
                                  max_shards=1, encoder_version=ckpt["encoder_version"])
        if ds.structure.digest != structure_sha256:
            raise ValueError("Parity dataset and checkpoint use different public rule catalogs")
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
    if file_digest(checkpoint_path) != source_checkpoint_sha256:
        raise ValueError("Source checkpoint changed during export")
    if half:
        # The parity above holds for the FP32 graphs these are converted from;
        # FP16 against FP32 is measured where it runs, on the GPU
        # (`netplay4::batch_tests`, then a paired evaluation).
        for depth in sessions:
            convert_to_float16(out_dir / f"model.k{depth}.onnx")
    atomic_copy(out_dir / f"model.k{args.thinking_steps}.onnx", out_dir / "model.onnx")
    meta = {
        "critic_semantics": export_critic_semantics(ckpt),
        **({"encoder_warmstart": ckpt["encoder_warmstart"]} if "encoder_warmstart" in ckpt else {}),
        "encoder_version": ckpt["encoder_version"], "model_version": model_module.MODEL_VERSION,
        **({"global_feat_width": cfg.global_feat_width} if ckpt["encoder_version"] == 9 else {}),
        "trajectory_version": M.TRAJECTORY_VERSION,
        "sampling_version": M.SAMPLING_VERSION,
        "structure_version": 1, "structure_sha256": structure_sha256,
        "source_checkpoint_sha256": source_checkpoint_sha256,
        "d_model": cfg.d_model, "n_layers": cfg.n_layers, "n_heads": cfg.n_heads,
        "thinking_steps": args.thinking_steps, "config": cfg.to_dict(),
        "graphs": {str(k): f"model.k{k}.onnx" for k in sessions},
        "parity_max_abs_diff": worst_diff, "checked_frames": checked,
    }
    if model_module.MODEL_VERSION == 5:
        meta["architecture"] = model_module.architecture_for_config(cfg)
    if half:
        meta["precision"] = "float16"
        meta["parity_scope"] = "float32 graphs before float16 conversion"
        meta["float16_conversion"] = {"converter": "onnxruntime.transformers.float16",
                                      "onnxruntime": ort.__version__, "keep_io_types": True}
    atomic_json(out_dir / "model.onnx.json", meta)
    print(f"[export model_version={model_module.MODEL_VERSION} precision={args.precision}] verified {checked} frames, max error={worst_diff:.3g}; exported K=0,1,2,4,8")


if __name__ == "__main__":
    main()
