"""Profile real recurrent training batches and compare equivalent execution paths.

Use an idle GPU and a copied last.pt. This never updates a production checkpoint.
Each variant starts with identical weights, optimizer, recurrent state and data.
Spatial chunking keeps B/U and the chronological recurrence unchanged.
"""
import argparse
import copy
import gc
import json
from pathlib import Path
import time

import torch
from torch.utils.data import DataLoader

import dataset4 as D
import model4 as M
from checkpoint4 import move_optimizer, restore_rng
from benchmark_state4 import checkpoint_sampler
from torch_io4 import atomic_json
from legacy_evaluation import native_metrics
from training4.execution import plan_batch, transfer_batch, encode_boards, compile_regions
from training4.loss import compute_step_loss
from training4.device_loss import compute_device_loss
from training4.deferred import deferred_readouts
from training4.prefetch import prefetch_batches


def update(net, optimizer, host, h, step_num, reference, chunk, fused=False, capture=False, staged=None):
    if getattr(net, "_cuda_graphs", False):
        torch.compiler.cudagraph_mark_step_begin()
    plan_start = time.perf_counter()
    plans = None if reference else plan_batch(host, step_num, curriculum=True)
    plan_ms = (time.perf_counter() - plan_start) * 1000
    events = [torch.cuda.Event(enable_timing=True) for _ in range(4)]
    events[0].record()
    batch = transfer_batch(host, torch.device("cuda")) if staged is None else staged
    events[1].record()
    b, u = batch["step_valid"].shape
    h = torch.where(batch["is_new_traj"].cuda()[:, None, None], torch.zeros_like(h), h.detach())
    optimizer.zero_grad(set_to_none=True)
    with torch.autocast("cuda", dtype=torch.bfloat16):
        net.clear_structure()
        net.prepare_structure()
        boards = encode_boards(net, batch, chunk, torch.device("cuda"), getattr(net,"_entity_bucket",0))
    loss = torch.zeros((), device="cuda")
    stats = torch.zeros(5, device="cuda")
    prepared = None
    if getattr(net, "decision_chunk", 1) > 1:
        prepared, final_hidden = deferred_readouts(net, batch, boards, h, plans,
                                                  net.decision_chunk, torch.device("cuda"), True)
    for t in range(u):
        k = (1, 2, 4, 8)[(step_num + t) % 4]
        thinking = batch["is_decision"][:, t].cuda().long() * k
        with torch.autocast("cuda", dtype=torch.bfloat16):
            out = (net(*[batch[n][:, t].cuda() for n in M.INPUT_NAMES[:-2]], h, thinking,
                      execution_hint=None if reference else plans[t].hint,
                      encoded_board=None if boards is None else boards[t],
                      policy_heads=plans[t].heads if not reference and getattr(net, "prune_unused_policy_heads", False) else None)
                   if prepared is None else prepared[t])
            aux = net.aux_head(out[0][:, 0])
        args = (*[x.float() for x in out[1:]], aux.float(), batch, t)
        if reference:
            step_loss, _ = compute_step_loss(*args)
        else:
            step_loss, st = compute_device_loss(*args, plans[t])
            stats += st
        if prepared is None:
            h = torch.where(batch["step_valid"][:, t, None, None].cuda(), out[0].float(), h)
        loss = loss + step_loss / u
    if prepared is not None:
        h = final_hidden
    events[2].record()
    if not torch.isfinite(loss):
        raise FloatingPointError("Nonfinite benchmark loss")
    loss.backward()
    gradients = {name: p.grad.detach().cpu().clone() for name, p in net.named_parameters() if p.grad is not None} if capture else None
    torch.nn.utils.clip_grad_norm_(net.parameters(), 1, error_if_nonfinite=True)
    optimizer.step()
    net.clear_structure()
    events[3].record()
    torch.cuda.synchronize()
    return h, {"loss": float(loss.detach()), "plan_ms": plan_ms,
               "h2d_ms": events[0].elapsed_time(events[1]),
               "forward_loss_ms": events[1].elapsed_time(events[2]),
               "backward_optimizer_ms": events[2].elapsed_time(events[3])}, gradients


def gradient_difference(reference, actual):
    if reference.keys() != actual.keys():
        raise AssertionError("Execution changed which parameters have gradients")
    difference, magnitude = 0.0, 0.0
    maximum = 0.0
    for name in reference:
        a, b = reference[name].float(), actual[name].float()
        difference += float((a-b).square().sum())
        magnitude += float(a.square().sum())
        maximum = max(maximum, float((a-b).abs().max()))
    return {"relative_l2": (difference/max(magnitude, 1e-30))**.5, "max_absolute": maximum}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--checkpoint", required=True)
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--steps", type=int, default=8)
    ap.add_argument("--variants", default="reference,optimizer,device,chunk4,chunk8,chunk16,chunk32,chunk8-fused")
    ap.add_argument("--gradient-reference", default="reference", help="Variant whose first gradient is the parity baseline")
    ap.add_argument("--batch-size", type=int, default=0, help="Benchmark batch lanes (0=checkpoint). Changing lanes requires --fresh-trajectories")
    ap.add_argument("--fresh-trajectories", action="store_true", help="Start sampler and memory at trajectory beginnings; never resume old memory into different lanes")
    ap.add_argument("--sampler-seed", type=int, default=None, help="Defaults to the checkpoint sampler seed; changing it requires --fresh-trajectories")
    ap.add_argument("--cuda-memory-fraction", type=float, default=1.0, help="Allocator ceiling; OOM rejects this benchmark variant")
    ap.add_argument("--profile", action="store_true")
    ap.add_argument("--validation-batches", type=int, default=0, help="Also audit a held-out prefix with fully optimized validation")
    args = ap.parse_args()
    if args.batch_size < 0 or not 0 < args.cuda_memory_fraction <= 1:
        ap.error("Invalid batch size or CUDA memory fraction")
    torch.set_num_threads(4)
    torch.cuda.set_per_process_memory_fraction(args.cuda_memory_fraction)
    saved = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    batch_size = args.batch_size or saved["batch_size"]
    if batch_size != saved["batch_size"] and not args.fresh_trajectories:
        ap.error("Changing batch lanes requires --fresh-trajectories; resumed H/cursor cannot be reassigned")
    ds = D.TrajectoryDataset4(Path(args.data), split="train", unroll_len=saved["unroll"])
    if ds.fingerprint != saved["dataset_fingerprint"]:
        raise ValueError("Dataset does not match the checkpoint")
    sampler = checkpoint_sampler(ds, saved, batch_size, args.fresh_trajectories, args.sampler_seed)
    loader = DataLoader(ds, batch_sampler=sampler, collate_fn=D.collate_trajectories,
                        num_workers=4, pin_memory=True)
    batches, waits = [], []
    iterator = iter(loader)
    for _ in range(args.steps + 2):
        start = time.perf_counter()
        batches.append(next(iterator))
        waits.append(time.perf_counter() - start)
    del iterator, loader
    report = {"gpu": torch.cuda.get_device_name(), "torch": torch.__version__,
              "checkpoint": args.checkpoint, "step": saved["step_num"], "batch_size": batch_size,
              "fresh_trajectories": args.fresh_trajectories,
              "sampler_seed": sampler.seed, "training_sampler_version": sampler.VERSION,
              "sampler_epoch": saved["epoch"] - 1, "sampler_start_batch": sampler.start_batch,
              "cuda_memory_fraction": args.cuda_memory_fraction,
              "cross_batch_size_caveat": "Different lane counts change trajectory mix and loss normalization; compare frames/s, then validate learning in a separate pilot, not as equivalent resume.",
              "unroll": saved["unroll"], "scope": __doc__, "gradient_reference": args.gradient_reference,
              "loader_wait_ms": [v*1000 for v in waits],
              "batch_mib": [sum(t.numel()*t.element_size() for t in b.values())/2**20 for b in batches],
              "entity_counts": [b["entity_feat"].shape[2] for b in batches], "results": []}
    reference_gradients = None
    for variant in args.variants.split(","):
        print(json.dumps({"variant": variant, "phase": "starting", "at_unix": time.time()}), flush=True)
        reference = variant in ("reference", "optimizer")
        chunk = int(variant.split("-")[0][5:]) if variant.startswith("chunk") else 1
        fused = variant.endswith("-fused")
        net = M.PlayerNet4(M.Config4(**saved["config"])).cuda().train()
        net.attach_structure(saved["structure"])
        net = net.cuda()
        net.load_state_dict(saved["state_dict"])
        if "cached" in variant.split("-"):
            net.cache_thinking_kv = True
        if "selected" in variant.split("-"):
            net.prune_unused_policy_heads = True
        for part in variant.split("-"):
            if part.startswith("deferred"):
                net.decision_chunk = int(part[len("deferred"):])
            if part.startswith("prefetch"):
                net.gpu_prefetch = int(part[len("prefetch"):])
            if part.startswith("saved"):
                net.saved_activation_layers = int(part[len("saved"):])
        if "regional" in variant.split("-"):
            compile_regions(net, readouts="readouts" in variant.split("-"))
        elif "graphs" in variant.split("-"):
            net._entity_bucket = 0 if variant.endswith("-unpadded") else 64
            compile_regions(net, cuda_graphs=True)
        elif variant.endswith("-padded"):
            net._entity_bucket = 64
            compile_regions(net)
        value_params = [p for n,p in net.named_parameters() if "seat_value_head" in n]
        other_params = [p for n,p in net.named_parameters() if "seat_value_head" not in n]
        optimizer = torch.optim.AdamW([{"params":other_params}, {"params":value_params}], fused=True if fused else None)
        # load_state_dict may share CPU scalar step tensors with its input.
        # Each comparison must start at the same counters, not mutate saved.
        optimizer.load_state_dict(copy.deepcopy(saved["optimizer"]))
        for group in optimizer.param_groups: group["fused"] = True if fused else None
        move_optimizer(optimizer, torch.device("cuda"))
        if variant == "reference":
            # Reproduce the running pre-fix trainer's resumed optimizer layout.
            for state in optimizer.state.values():
                for key, value in state.items():
                    if isinstance(value, torch.Tensor): state[key] = value.cuda()
        restore_rng(saved["rng"])
        h = (saved["h"].cuda() if saved["h"] is not None and not args.fresh_trajectories
             else torch.zeros(batch_size, 9, net.cfg.d_model, device="cuda"))
        torch.cuda.reset_peak_memory_stats()
        retries_before = torch.cuda.memory_stats()["num_alloc_retries"]
        samples, row = [], {"variant":variant, "started_at_unix":time.time()}
        stream_batches = prefetch_batches(batches, torch.device("cuda"), getattr(net, "gpu_prefetch", 0))
        staged, batch = None, None
        try:
            for i in range(len(batches)):
                torch.cuda.synchronize()
                start, cpu_start = time.perf_counter(), time.process_time()
                # Include staging/API overhead in wall time; moving it outside
                # the measurement would unfairly favor the prefetch variant.
                batch, staged = next(stream_batches)
                h, metrics, gradients = update(net, optimizer, batch, h, saved["step_num"]+i, reference, chunk, fused, capture=i==0, staged=staged)
                metrics.update(wall_ms=(time.perf_counter()-start)*1000, cpu_ms=(time.process_time()-cpu_start)*1000)
                if i == 0:
                    row["first_loss"] = metrics["loss"]
                    if variant == args.gradient_reference: reference_gradients = gradients
                    elif reference_gradients is not None: row["gradient_difference"] = gradient_difference(reference_gradients,gradients)
                    del gradients
                if i >= 2: samples.append(metrics)
            row.update(samples=samples,
                       **native_metrics("benchmark-statistics", mode="training",
                           samples=[s["wall_ms"] for s in samples],
                           frames=torch.stack([b["step_valid"].sum() for b in batches[2:]]).sum().item(),
                           decisions=torch.stack([(b["step_valid"] & b["is_decision"]).sum() for b in batches[2:]]).sum().item()),
                       peak_allocated_gib=torch.cuda.max_memory_allocated()/2**30,
                       peak_reserved_gib=torch.cuda.max_memory_reserved()/2**30,
                       allocation_retries=torch.cuda.memory_stats()["num_alloc_retries"]-retries_before)
            if args.profile and variant in ("reference", "chunk8"):
                with torch.profiler.profile(activities=[torch.profiler.ProfilerActivity.CPU,torch.profiler.ProfilerActivity.CUDA]) as prof:
                    h, _, _ = update(net,optimizer,batches[-1],h,saved["step_num"],reference,chunk)
                averages=prof.key_averages()
                row["synchronization_counts"]={e.key:e.count for e in averages if e.key in ("aten::item","aten::_local_scalar_dense","aten::nonzero","cudaStreamSynchronize","cudaDeviceSynchronize")}
                row["profile_cpu_top"]=averages.table(sort_by="self_cpu_time_total",row_limit=12)
        except torch.cuda.OutOfMemoryError:
            row["error"]="CUDA out of memory; variant rejected"
        finally:
            stream_batches.close()
        row["finished_at_unix"] = time.time()
        report["results"].append(row)
        atomic_json(Path(args.out), report)
        print(json.dumps({k:v for k,v in row.items() if k not in ("samples","profile_cpu_top")}),flush=True)
        del net,optimizer,h,staged,batch,stream_batches
        gc.collect();torch.cuda.empty_cache()
    if args.validation_batches:
        from validation_probe4 import compare_validation
        report["validation"] = compare_validation(saved, args.data, args.validation_batches,
                                                   readouts="readouts" in args.variants)
        atomic_json(Path(args.out), report)
        print(json.dumps({"validation": report["validation"]}), flush=True)


if __name__ == "__main__":
    main()
