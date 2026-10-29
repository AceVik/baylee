"""Compare CPU ONNX and CUDA PyTorch inference on real v4 frames.

Measures encoded tensors through host outputs, including GPU transfers and
synchronization. Batches repeat the same frame to bound shape-matched batching
potential; this is not a measurement of complete game throughput or queue delay.
"""
import argparse
import json
from pathlib import Path
import time

import numpy as np
import onnxruntime as ort
import torch

import dataset4 as D
import model4 as M
from torch_io4 import atomic_json
from legacy_evaluation import native_metrics


def timed(fn, count):
    for _ in range(3):
        fn()
    samples = []
    for _ in range(count):
        start = time.perf_counter()
        fn()
        samples.append((time.perf_counter() - start) * 1000)
    return native_metrics("benchmark-statistics", mode="inference", samples=samples)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--model", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--repeats", type=int, default=20)
    args = ap.parse_args()
    if not torch.cuda.is_available():
        raise RuntimeError("CUDA is required for this comparison")
    torch.set_num_threads(1)
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    model = Path(args.model)
    ckpt = torch.load(model / "net.pt", map_location="cpu", weights_only=False)
    cfg = M.Config4(**ckpt["config"])
    net = M.PlayerNet4(cfg).eval()
    net.attach_structure(ckpt["structure"])
    net.load_state_dict(ckpt["state_dict"], strict=True)
    net.rules.eval()
    net.rules = net.rules.freeze()
    net = net.cuda()
    opts = ort.SessionOptions()
    opts.intra_op_num_threads = 1
    sessions = {k: ort.InferenceSession(str(model / f"model.k{k}.onnx"), sess_options=opts,
                                        providers=["CPUExecutionProvider"]) for k in (0, 1)}
    ds = D.TrajectoryDataset4(Path(args.data), split="val", unroll_len=32)
    candidates = []
    for row in native_metrics("benchmark-plan", mode="rows", length=len(ds.store.index)):
        raw = ds.store.read(int(row))
        if raw["is_decision"]:
            enc = ds.encode_single_step(raw)
            candidates.append((len(enc["entity_mask"]), int(row), enc))
    if not candidates:
        raise ValueError("No real decision frames sampled")
    selected = [candidates[i] for i in native_metrics("benchmark-plan", mode="select",
                entities=[row[0] for row in candidates])]
    report = {"model": str(model), "dataset_fingerprint": ds.fingerprint,
              "gpu": torch.cuda.get_device_name(), "precision": "FP32, TF32 disabled",
              "scope": __doc__, "results": []}
    with torch.inference_mode():
        for entities, row, enc in selected:
            for depth in (0, 1):
                base = [torch.from_numpy(enc[name]).unsqueeze(0) for name in M.INPUT_NAMES[:-2]]
                base += [torch.zeros(1, M.RECURRENT_SLOTS, cfg.d_model), torch.tensor([depth])]
                if depth == 0:
                    # The observation graph sees no active candidate set.
                    for name in ("cand_feat", "cand_entity_idx", "cand_seat_idx", "cand_mask", "cand_dsl", "pair_index", "pair_mask"):
                        i = M.INPUT_NAMES.index(name)
                        base[i] = base[i][:, :1].clone()
                    base[M.INPUT_NAMES.index("cand_mask")].zero_()
                    base[M.INPUT_NAMES.index("pair_mask")].zero_()
                feed = {name: t.numpy() for name, t in zip(M.INPUT_NAMES, base)}
                cpu = lambda: sessions[depth].run(list(M.OUTPUT_NAMES), feed)
                expected = cpu()
                cpu_time = timed(cpu, args.repeats)
                for batch_size in (1, 4, 8, 16):
                    inputs = [t.repeat(batch_size, *([1]*(t.ndim-1))) for t in base]
                    def gpu():
                        values = net(*(t.cuda() for t in inputs))
                        return [v.cpu().numpy() for v in values]
                    actual = gpu()
                    error = 0.
                    for want, got in zip(expected, actual):
                        got = got[:1]
                        if want.shape != got.shape:
                            want = want[:, :got.shape[1]]
                        keep = np.abs(want) < 1e8
                        if not np.isfinite(got).all():
                            raise ValueError("Nonfinite GPU output")
                        if keep.any():
                            error = max(error, float(np.abs(want[keep] - got[keep]).max()))
                    if error > 1e-3:
                        raise ValueError(f"GPU/CPU mismatch: {error}")
                    result = {"row": row, "entities": entities, "depth": depth,
                              "batch_size": batch_size, "cpu_single": cpu_time,
                              "gpu_batch": timed(gpu, args.repeats), "max_error": error}
                    result["throughput_speedup"] = native_metrics("benchmark-resources", mode="speedup",
                        cpu=cpu_time["p50_ms"], batch=batch_size, gpu=result["gpu_batch"]["p50_ms"])
                    report["results"].append(result)
                    atomic_json(Path(args.out), report)
                    print(json.dumps(result), flush=True)
        report["peak_allocated_bytes"] = torch.cuda.max_memory_allocated()
        atomic_json(Path(args.out), report)


if __name__ == "__main__":
    main()
