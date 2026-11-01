"""Frozen-policy audit and complete-episode GAE before any PPO optimizer update."""
from __future__ import annotations

import numpy as np
import torch
from torch.utils.data import DataLoader

import dataset4 as D4
import model4 as M


@torch.no_grad()
def prepare_targets(net, dataset, device, batch_size, gamma, gae_lambda,
                    logprob_fn, gae_fn, tolerance=2e-3):
    from trajectory_store4 import reject_shadow_policy_gradient
    reject_shadow_policy_gradient(dataset.meta)
    if dataset.meta.get("sampling_version") != M.SAMPLING_VERSION:
        raise ValueError("Rollout uses an incompatible constrained sampler")
    net.eval()
    net.clear_structure()
    net.prepare_structure()
    n, u, d = len(dataset), dataset.unroll_len, net.cfg.d_model
    values = torch.zeros(n, u)
    outcomes = torch.zeros(n, u)
    valid = torch.zeros(n, u, dtype=torch.bool)
    terminal = torch.zeros_like(valid)
    actor = torch.zeros_like(valid)
    memory = np.zeros((n, M.RECURRENT_SLOTS, d), dtype=np.float32)
    sampler = D4.StatefulBatchSampler4(dataset, batch_size, shuffle=False)
    loader = DataLoader(dataset, batch_sampler=sampler, collate_fn=D4.collate_trajectories)
    h = None
    max_error = 0.0
    checked = 0
    for batch in loader:
        ids = batch["segment_index"].tolist()
        batch = {key: value.to(device) for key, value in batch.items()}
        b = batch["step_valid"].shape[0]
        if h is None:
            h = torch.zeros(b, M.RECURRENT_SLOTS, d, device=device)
        h = torch.where(batch["is_new_traj"].view(b, 1, 1), torch.zeros_like(h), h)
        for lane, idx in enumerate(ids):
            if idx >= 0:
                memory[idx] = h[lane].cpu().numpy()
        for t in range(u):
            k = torch.where(batch["is_decision"][:, t], batch["thinking_steps"][:, t].clamp(1, 8), 0)
            inputs = tuple(batch[name][:, t] for name in M.INPUT_NAMES[:-2]) + (h, k)
            hn, cl, pl, sp, value = net(*inputs)
            lp, _ = logprob_fn(cl, pl, sp, cl, pl, sp, batch, t)
            mask = (batch["step_valid"][:, t] & batch["is_decision"][:, t]
                    & (batch["source_kind"][:, t] == 0) & batch["has_logprob"][:, t]
                    & (batch["head"][:, t] != 3) & (batch["temperature"][:, t] > 1e-4))
            if mask.any():
                errors = (lp[mask] - batch["behavior_logprob"][:, t][mask]).abs()
                max_error = max(max_error, float(errors.max()))
                checked += int(mask.sum())
                if not torch.isfinite(errors).all() or max_error > tolerance:
                    lanes = mask.nonzero().flatten().tolist()
                    detail = [{"segment": ids[i], "t": t, "head": int(batch["head"][i, t]),
                               "replay": float(lp[i]), "recorded": float(batch["behavior_logprob"][i, t])}
                              for i in lanes]
                    raise ValueError(f"Collector/replay policy mismatch BEFORE training: max logprob error={max_error:.6g}; {detail}")
            h = torch.where(batch["step_valid"][:, t].view(b, 1, 1), hn, h)
            for lane, idx in enumerate(ids):
                if idx >= 0:
                    values[idx, t] = value[lane].cpu()
                    outcomes[idx, t] = batch["outcome"][lane, t].cpu()
                    valid[idx, t] = batch["step_valid"][lane, t].cpu()
                    terminal[idx, t] = batch["is_terminal"][lane, t].cpu()
                    actor[idx, t] = mask[lane].cpu()
    if checked == 0:
        raise ValueError("No genuine stochastic neural decisions; refusing PPO")
    advantages = torch.zeros_like(values)
    returns = torch.zeros_like(values)
    for ids in dataset.trajectories:
        ids_t = torch.tensor(ids)
        keep = valid[ids_t].flatten()
        v = values[ids_t].flatten()[keep].unsqueeze(0)
        o = outcomes[ids_t].flatten()[keep].unsqueeze(0)
        term = terminal[ids_t].flatten()[keep].unsqueeze(0)
        a, r = gae_fn(v, o, torch.ones_like(term), term, gamma, gae_lambda)
        flat_a = torch.zeros(len(ids) * u)
        flat_r = torch.zeros_like(flat_a)
        flat_a[keep], flat_r[keep] = a[0], r[0]
        advantages[ids_t] = flat_a.reshape(-1, u)
        returns[ids_t] = flat_r.reshape(-1, u)
    chosen = advantages[actor]
    advantages = (advantages - chosen.mean()) / chosen.std(unbiased=False).clamp_min(1e-5)
    dataset.ppo_cache = {
        "advantages": advantages.numpy(), "returns": returns.numpy(),
        "behavior_h": memory,
    }
    report = {"decisions_checked": checked, "max_logprob_error": max_error,
              "trajectories": len(dataset.trajectories), "gamma": gamma, "lambda": gae_lambda}
    print(f"[ppo-audit] {report}", flush=True)
    net.clear_structure()
    return report
