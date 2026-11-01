"""Warm and audit regional compilation alongside a running trainer.

Uses only four frames of a copied real batch to bound additional VRAM. Timings
under concurrent training are NOT a throughput comparison. This is a compile/
correctness probe; measure complete updates on an idle GPU before deployment.
"""
import argparse
import copy
import gc
import json
from pathlib import Path
import time

import torch
import torch._inductor.config
import dataset4 as D
import model4 as M
from bench_training4 import update, gradient_difference
from checkpoint4 import move_optimizer, restore_rng
from torch_io4 import atomic_json
from training4.execution import compile_regions


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--checkpoint", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--cuda-graphs", action="store_true")
    ap.add_argument("--cache-thinking-kv", action="store_true")
    ap.add_argument("--compile-readouts", action="store_true")
    args = ap.parse_args()
    torch.set_num_threads(2)
    torch.cuda.set_per_process_memory_fraction(0.18)
    torch._inductor.config.compile_threads = 4
    saved = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    ds = D.TrajectoryDataset4(Path(args.data), split="train", unroll_len=saved["unroll"])
    sampler = D.StatefulBatchSampler4(ds, saved["batch_size"], seed=42)
    sampler.epoch = saved["epoch"] - 1
    sampler.start_batch = saved["next_batch"]
    indices = next(iter(sampler))
    host = D.collate_trajectories([ds[i] for i in indices])
    host = {key: value[:, :4].pin_memory() if value.ndim >= 2 else value.pin_memory() for key,value in host.items()}
    decisions = host["is_decision"].sum(0).tolist()
    print(json.dumps({"decisions_per_frame": decisions, "step": saved["step_num"]}), flush=True)
    del ds,sampler
    rows, gradients, weights = [], None, None
    for compiled in (False, True):
        net = M.PlayerNet4(M.Config4(**saved["config"])).cuda().train()
        net.attach_structure(saved["structure"])
        net = net.cuda()
        net.load_state_dict(saved["state_dict"])
        if compiled:
            net.cache_thinking_kv = args.cache_thinking_kv
            net._entity_bucket = 64 if args.cuda_graphs else 0
            compile_regions(net, cuda_graphs=args.cuda_graphs, readouts=args.compile_readouts)
        optimizer = torch.optim.AdamW([
            {"params":[p for n,p in net.named_parameters() if "seat_value_head" not in n]},
            {"params":[p for n,p in net.named_parameters() if "seat_value_head" in n]},
        ])
        optimizer.load_state_dict(copy.deepcopy(saved["optimizer"]))
        move_optimizer(optimizer,"cuda")
        restore_rng(saved["rng"])
        h = saved["h"].cuda()
        start = time.perf_counter()
        h, metrics, grad = update(net,optimizer,host,h,saved["step_num"],False,4,capture=True)
        metrics.update(compiled=compiled,cold_seconds=time.perf_counter()-start,peak_gib=torch.cuda.max_memory_allocated()/2**30)
        if not compiled:
            gradients=grad
            weights={n:p.detach().cpu().clone() for n,p in net.named_parameters()}
        else:
            metrics["gradient_difference"] = gradient_difference(gradients,grad)
            metrics["updated_weight_difference"] = gradient_difference(weights,{n:p.detach().cpu() for n,p in net.named_parameters()})
        rows.append(metrics)
        atomic_json(Path(args.out),{"scope":__doc__,"results":rows})
        print(json.dumps(metrics),flush=True)
        del net,optimizer,h,grad
        gc.collect();torch.cuda.empty_cache()


if __name__ == "__main__": main()
