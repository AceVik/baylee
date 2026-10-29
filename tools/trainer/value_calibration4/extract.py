"""Frozen chronological inference; sampling never skips a memory update."""
from pathlib import Path
import time

import numpy as np
import torch
from torch.utils.data import DataLoader
import dataset4 as D
import model4 as M
from training4.execution import encode_boards
from .cache import FeatureCache, identity, key_for, record_dtype
from .features import neural_decisions, seat_masks
from .source_identity import file_digest
from tensor_io4 import TensorPublication


@torch.no_grad()
def extract(net, dataset, destination, stride=64, batch_size=8, board_chunk=4,
            max_samples=200000):
    """Write detached records and return complete learner eligibility metadata."""
    device = next(net.parameters()).device
    if any(p.dtype != torch.float32 for p in net.parameters()):
        raise ValueError("Feature extraction requires float32 parameters")
    net.eval()
    sampler = D.StatefulBatchSampler4(dataset, batch_size, shuffle=False)
    loader = DataLoader(dataset, batch_sampler=sampler, collate_fn=D.collate_trajectories)
    segment_to_traj = {segment: i for i, trajectory in enumerate(dataset.trajectories)
                       for segment in trajectory}
    eligible = set()
    counts, scanned = {}, 0
    scanned_trajectories = set()
    captured = []
    hook = net.seat_value_head.register_forward_pre_hook(lambda _module, inputs: captured.append(inputs[0].detach()))
    h = None
    count = 0
    started = time.monotonic()
    try:
        net.clear_structure()
        net.prepare_structure()
        with TensorPublication('file',Path(destination)/'records.bin') as stream:
            for batch_index, host in enumerate(loader):
                record_blocks=[]
                ids = host["segment_index"].tolist()
                b, u = host["step_valid"].shape
                if h is None:
                    h = torch.zeros(b, 9, net.cfg.d_model, device=device)
                h = torch.where(host["is_new_traj"].to(device)[:, None, None], torch.zeros_like(h), h)
                # Board chunks only group independent spatial encodings.
                boards = encode_boards(net, host, board_chunk, device)
                neural = neural_decisions(host)
                for lane, idx in enumerate(ids):
                    if idx >= 0 and neural[lane].any():
                        eligible.add(segment_to_traj[idx])
                for t in range(u):
                    decision = host["is_decision"][:, t].to(device)
                    recorded = host["thinking_steps"][:, t].to(device)
                    source_net = host["source_kind"][:, t].to(device) == 0
                    if ((recorded > 8) & decision & source_net).any():
                        raise ValueError("Recorded thinking depth exceeds deployed K contract")
                    k = torch.where(decision, torch.where(source_net & (recorded > 0), recorded, 1), 0)
                    captured.clear()
                    args = [host[name][:, t].to(device) for name in M.INPUT_NAMES[:-2]]
                    output = net(*args, h, k, policy_heads=(False, False, False, False),
                                 encoded_board=None if boards is None else boards[t])
                    valid = host["step_valid"][:, t].to(device)
                    h = torch.where(valid[:, None, None], output[0], h)
                    if len(captured) != 1:
                        raise RuntimeError("Expected exactly one deployed seat-value readout")
                    active, allied = seat_masks(args[3])
                    for lane, idx in enumerate(ids):
                        if idx < 0 or not bool(host["step_valid"][lane, t]):
                            continue
                        rows = dataset.segments[idx]
                        if t >= len(rows):
                            continue
                        scanned += 1
                        trajectory = segment_to_traj[idx]
                        first = trajectory not in scanned_trajectories
                        scanned_trajectories.add(trajectory)
                        index = dataset.store.index[int(rows[t])]
                        cursor = int(index["cursor"])
                        terminal = bool(host["is_terminal"][lane, t])
                        if cursor % stride and not terminal and not first:
                            continue
                        if count >= max_samples:
                            raise ValueError("Calibration sample limit exceeded; increase --max-samples explicitly")
                        record = np.zeros(1, dtype=record_dtype(net.cfg.d_model))
                        for name, tensor in (("features", captured[0]), ("active", active),
                                             ("allied", allied), ("source_value", output[4])):
                            record[name][0] = tensor[lane].cpu().numpy()
                        g = host["global_feat"][lane, t]
                        record["trajectory"] = trajectory
                        record["game"], record["seat"], record["cursor"] = index["game"], index["seat"], cursor
                        record["players"] = int((host["seat_feat"][lane, t, :, M.SEAT_FEAT_PRESENT] > .5).sum())
                        record["turn_norm"], record["commander"], record["team"] = float(g[0]), bool(g[21] > .5), bool(g[20] > .5)
                        record["outcome"] = float(host["outcome"][lane, t])
                        record["source"] = int(host["source_kind"][lane, t])
                        record["decision"], record["neural_decision"], record["terminal"] = bool(decision[lane]), bool(neural[lane, t]), terminal
                        if (not np.isfinite(record["features"]).all()
                                or not np.isfinite(record["source_value"]).all()
                                or not 0 <= float(record["outcome"][0]) <= 1):
                            raise FloatingPointError("Nonfinite feature/value or invalid terminal outcome")
                        record_blocks.append(record.tobytes())
                        counts[trajectory] = counts.get(trajectory, 0) + 1
                        count += 1
                if record_blocks:
                    stream.write(b''.join(record_blocks))
                if batch_index % 100 == 0:
                    print(f"[value-cache] batches={batch_index + 1}/{len(loader)} frames={scanned} samples={count} seconds={time.monotonic()-started:.1f}", flush=True)
            if not eligible:
                raise ValueError("No genuine stochastic neural trajectory; refusing teacher-only calibration")
            stream.commit()
        return dict(records=count, records_sha256=file_digest(Path(destination) / "records.bin"),
                    scanned_frames=scanned, eligible_trajectories=sorted(eligible),
                    sampled_trajectories=len(counts), learner_trajectories=len(eligible),
                    turn_metadata="global_feat[0]: turn/20 clipped at 1; 1 means turn>=20",
                    format_metadata="commander/team flags only; no invented named format")
    finally:
        hook.remove()
        net.clear_structure()


def cached_features(net, dataset, cache_root, source_hash, stride, batch_size, board_chunk, max_samples):
    settings = identity(source_hash, dataset, stride, board_chunk, next(net.parameters()).device.type)
    settings["batch_size"] = batch_size
    destination=Path(cache_root).expanduser()/key_for(settings)
    options=dict(width=net.cfg.d_model,settings=settings,max_samples=max_samples)
    with TensorPublication('cache',destination,**options) as publication:
        if not publication.receipt['reused']:
            meta=extract(net,dataset,publication.path,stride,batch_size,board_chunk,max_samples)
            meta['identity']=settings
            publication.commit(metadata=meta)
    cache = FeatureCache(destination, net.cfg.d_model)
    return cache
