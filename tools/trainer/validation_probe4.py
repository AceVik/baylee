"""Compare validation paths on the same bounded prefix of held-out trajectories."""
import gc
from itertools import islice
from pathlib import Path
import time
from types import SimpleNamespace

import torch
from torch.utils.data import DataLoader
import dataset4 as D
import model4 as M
from training4.execution import compile_regions
from training4.validation import evaluate
from training4.validation_device import evaluate_device


def compare_validation(saved, data, count, readouts=False, decision_chunk=1, production_reference=False):
    ds = D.TrajectoryDataset4(Path(data), split="val", unroll_len=saved["unroll"])
    sampler = D.StatefulBatchSampler4(ds, saved["batch_size"], shuffle=False, seed=42)
    loader = DataLoader(ds, batch_sampler=sampler, collate_fn=D.collate_trajectories,
                        num_workers=2, pin_memory=True)
    batches = list(islice(loader, count))
    del loader, sampler, ds

    class Prefix:
        dataset = SimpleNamespace(unroll_len=saved["unroll"])
        def __iter__(self): return iter(batches)
        def __len__(self): return len(batches)

    rows = []
    for optimized in (False, True):
        net = M.PlayerNet4(M.Config4(**saved["config"])).cuda().eval()
        net.attach_structure(saved["structure"])
        net = net.cuda()
        net.load_state_dict(saved["state_dict"])
        if optimized or production_reference:
            net.cache_thinking_kv = True
            net.prune_unused_policy_heads = True
            net.decision_chunk = decision_chunk if optimized else 1
            compile_regions(net, readouts=readouts if optimized else False)
        torch.cuda.synchronize()
        start = time.perf_counter()
        function = evaluate_device if optimized or production_reference else evaluate
        metrics = function(net, Prefix(), torch.device("cuda"), 1)
        torch.cuda.synchronize()
        rows.append({"optimized": optimized, "seconds_including_compile": time.perf_counter()-start,
                     "compile_readouts": optimized and readouts,
                     "decision_chunk": decision_chunk if optimized else 1,
                     "batches": len(batches), "metrics": metrics})
        del net
        gc.collect(); torch.cuda.empty_cache()
    expected, actual = [row["metrics"] for row in rows]
    difference = {name: actual[name]-value for name, value in expected.items()}
    return {"rows": rows, "difference": difference,
            "production_reference": production_reference,
            "scope": "Bounded validation prefix, not a full strength evaluation. Same weights and trajectory lanes."}
