"""Playernet v4 T1 Hardware Pilot Benchmark (`bench_t1.py`).

Executes the empirical hardware pilot required by Section 8 & Section 14 (T1) of
`docs/history/trained-ai-v4-handover-2026-10-22.md`:

1. GPU Training Benchmark (RTX 4070 Ti 12 GB VRAM, bf16 mixed precision):
   Sweeps:
   - Backbone sizes: `(256, 4)`, `(384, 8)`, `(512, 8)`
   - Trajectory batch size `B in {2, 4, 8}`
   - Unroll length `U in {16, 32}`
   - Thinking steps `K in {1, 2, 4}`
   - Token buckets `N_tok in {64, 128, 320}` (`E = N_tok - 9`)
   Measures peak allocated VRAM (MB), step wall time (ms), and decision steps/sec.

2. CPU ONNX Runtime Single-Step Latency Benchmark:
   Exports each backbone `(256/4, 384/8, 512/8)` at `K in {1, 2, 4, 8}` and
   measures single-thread (`intra_op_num_threads=1`) and 4-thread CPU inference
   latency (`p50`, `p95`, `p99` ms) across token buckets `N_tok in {64, 128, 320}`.
"""

from __future__ import annotations

import argparse
import json
import tempfile
import time
from pathlib import Path

import onnxruntime as ort
import torch
from legacy_evaluation import native_metrics

import export_onnx4 as E4
import model4 as M
from torch_io4 import atomic_json


def bench_gpu_config(
    d_model: int,
    n_layers: int,
    B: int,
    U: int,
    K: int,
    n_tok: int,
    warmup: int = 2,
    iters: int = 5,
) -> dict:
    device = torch.device("cuda")
    torch.cuda.empty_cache()
    torch.cuda.reset_peak_memory_stats(device)

    E = max(n_tok - M.RECURRENT_SLOTS, 8)
    M_edges = min(E * 2, 256)
    C = 32
    P = 32

    cfg = M.Config4(
        d_model=d_model,
        n_layers=n_layers,
        n_heads=8,
        d_ff=d_model * 4,
        default_thinking_steps=K,
    )
    net = M.PlayerNet4(cfg).to(device).train()
    opt = torch.optim.AdamW(net.parameters(), lr=3e-4)
    n_params = sum(p.numel() for p in net.parameters())

    # Pre-allocate one synthetic unrolled batch on GPU
    inputs = [
        t.to(device)
        for t in E4.make_dummy_inputs(d_model, B=B, E=E, M_edges=M_edges, C=C, P=P, K=K)
    ]
    # Synthetic backbone benchmark only; use bench_training4.py with real
    # trajectories to include the full public rule catalog and candidate mix.
    observations, k_t = inputs[:-2], inputs[-1]

    def run_step() -> None:
        opt.zero_grad(set_to_none=True)
        h = torch.zeros((B, M.RECURRENT_SLOTS, d_model), dtype=torch.float32, device=device)
        loss_acc = torch.tensor(0.0, device=device)
        with torch.autocast(device_type="cuda", dtype=torch.bfloat16):
            net.prepare_structure()
        for _ in range(U):
            with torch.autocast(device_type="cuda", dtype=torch.bfloat16):
                h_next, cand_l, pair_l, scal_p, val_p = net(
                    *observations,
                    h,
                    k_t,
                )
                loss_t = (
                    cand_l[:, 0].mean()
                    + pair_l[:, 0].mean() * 0.1
                    + scal_p.mean() * 0.1
                    + val_p.mean()
                )
            h = h_next.float()
            loss_acc = loss_acc + loss_t / U
        loss_acc.backward()
        opt.step()
        net.clear_structure()

    try:
        for _ in range(warmup):
            run_step()
        torch.cuda.synchronize(device)
        t0 = time.perf_counter()
        for _ in range(iters):
            run_step()
        torch.cuda.synchronize(device)
        elapsed = time.perf_counter() - t0
        return {
            "status": "ok",
            "d_model": d_model,
            "n_layers": n_layers,
            "B": B,
            "U": U,
            "K": K,
            "n_tok": n_tok,
            **native_metrics("benchmark-resources", mode="t1-training",
                elapsed=elapsed, iterations=iters, parameters=n_params, batch=B, unroll=U,
                peak_bytes=torch.cuda.max_memory_allocated(device)),
        }
    except RuntimeError as e:
        if "out of memory" in str(e).lower():
            torch.cuda.empty_cache()
            return {
                "status": "oom",
                "d_model": d_model,
                "n_layers": n_layers,
                "B": B,
                "U": U,
                "K": K,
                "n_tok": n_tok,
            }
        raise


def bench_cpu_onnx(
    d_model: int,
    n_layers: int,
    K: int,
    n_tok: int,
    intra_threads: int = 1,
    iters: int = 40,
) -> dict:
    E = max(n_tok - M.RECURRENT_SLOTS, 8)
    M_edges = min(E * 2, 256)
    cfg = M.Config4(
        d_model=d_model,
        n_layers=n_layers,
        n_heads=8,
        d_ff=d_model * 4,
        default_thinking_steps=K,
    )
    net = M.PlayerNet4(cfg).eval()
    with tempfile.TemporaryDirectory() as tmp:
        onnx_path = Path(tmp) / "model.onnx"
        E4.export_v4_onnx(net, onnx_path, thinking_steps=K)
        opts = ort.SessionOptions()
        opts.intra_op_num_threads = intra_threads
        opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        sess = ort.InferenceSession(
            str(onnx_path), sess_options=opts, providers=["CPUExecutionProvider"]
        )
        t_inputs = E4.make_dummy_inputs(
            d_model, B=1, E=E, M_edges=M_edges, C=32, P=32, K=K
        )
        feeds = {name: t_in.numpy() for name, t_in in zip(M.INPUT_NAMES, t_inputs)}
        for _ in range(5):
            sess.run(None, feeds)
        latencies = []
        for _ in range(iters):
            t0 = time.perf_counter()
            sess.run(None, feeds)
            latencies.append((time.perf_counter() - t0) * 1000.0)
        return {
            "d_model": d_model,
            "n_layers": n_layers,
            "K": K,
            "n_tok": n_tok,
            "intra_threads": intra_threads,
            **native_metrics("benchmark-statistics", mode="t1", samples=latencies),
        }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="", help="Optional path to write JSON benchmark report")
    args = ap.parse_args()

    gpu_results = []
    print("=== T1.1: GPU Training VRAM & Throughput Sweep (RTX 4070 Ti bf16) ===")
    for d_model, n_layers in [(256, 4), (384, 8), (512, 8)]:
        for B, U, K, n_tok in [
            (4, 16, 2, 64),
            (4, 16, 2, 128),
            (4, 16, 2, 320),
            (8, 16, 2, 128),
            (4, 32, 2, 128),
            (4, 16, 4, 128),
            (2, 32, 4, 320),
        ]:
            res = bench_gpu_config(d_model, n_layers, B, U, K, n_tok)
            gpu_results.append(res)
            print(
                f"  [{d_model}/{n_layers}L] B={B} U={U} K={K} N_tok={n_tok:3d} -> "
                f"VRAM={res.get('peak_vram_mb')} MB | "
                f"step={res.get('step_ms')} ms | "
                f"steps/s={res.get('decision_steps_per_s')}"
            )

    cpu_results = []
    print("\n=== T1.2: CPU ONNX Runtime Single-Step Latency Sweep ===")
    for d_model, n_layers in [(256, 4), (384, 8), (512, 8)]:
        for K in (1, 2, 4, 8):
            for n_tok in (64, 128, 320):
                res = bench_cpu_onnx(d_model, n_layers, K, n_tok, intra_threads=1)
                cpu_results.append(res)
                if n_tok == 128 or K in (2, 8):
                    print(
                        f"  [{d_model}/{n_layers}L] K={K} N_tok={n_tok:3d} (1T) -> "
                        f"p50={res['p50_ms']:.2f} ms | p95={res['p95_ms']:.2f} ms | p99={res['p99_ms']:.2f} ms"
                    )

    report = {"gpu_training": gpu_results, "cpu_onnx_inference": cpu_results}
    if args.out:
        out_p = Path(args.out).expanduser()
        out_p.parent.mkdir(parents=True, exist_ok=True)
        atomic_json(out_p, report)
        print(f"\n[bench_t1] saved full report to {out_p}")


if __name__ == "__main__":
    main()
