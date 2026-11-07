"""What PPO would see if a rollout's graphs were not the learner's FP32.

Replays a recorded stochastic rollout through the frozen FP32 learner, as
`prepare_policy_targets` does, and instead of refusing a gap it reports the gap:
the log probability of every chosen action against the one the rollout recorded,
and so the ratio exp(learner - behavior) that PPO's first update starts from.
A rollout from the FP32 graphs gives the floor; one from the FP16 graphs gives the
question. Run it on datasets `convert4` made from `eval4 --raw-policy --temperature`.
"""
import argparse
import json
from pathlib import Path

import numpy as np
import torch
from torch.utils.data import DataLoader

import dataset4 as D
import model4 as M
from policy_training4.targets import _learner_histories, actor_mask
from train_rl4 import action_logprob_and_kl
from training4.execution import encode_boards
from value_calibration4.checkpoint import load_source


@torch.no_grad()
def gaps(net, dataset, device, lanes, board_chunk, workers):
    """Per audited choice: learner minus recorded log probability, and its head."""
    dataset.trajectories = _learner_histories(dataset)
    loader = DataLoader(dataset, batch_sampler=D.StatefulBatchSampler4(dataset, lanes, shuffle=False),
                        collate_fn=D.collate_trajectories, num_workers=workers,
                        prefetch_factor=4 if workers else None)
    memory = None
    differences, heads, values = [], [], []
    net.eval()
    for number, host in enumerate(loader):
        net.clear_structure()
        net.prepare_structure((host['entity_rules'], host['cand_rules']))
        if memory is None:
            memory = torch.zeros(host['step_valid'].shape[0], M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
        memory = torch.where(host['is_new_traj'].to(device)[:, None, None], torch.zeros_like(memory), memory)
        boards = encode_boards(net, host, board_chunk, device)
        mask = actor_mask(host)
        for t in range(host['step_valid'].shape[1]):
            decision = host['is_decision'][:, t].to(device)
            recorded = host['thinking_steps'][:, t].to(device)
            source = host['source_kind'][:, t].to(device) == 0
            k = torch.where(decision, torch.where(source & (recorded > 0), recorded, 1), 0)
            chosen = tuple(bool((mask[:, t] & (host['head'][:, t] == kind)).any()) for kind in range(4))
            output = net(*[host[name][:, t].to(device) for name in M.INPUT_NAMES[:-2]], memory, k,
                         policy_heads=chosen, encoded_board=None if boards is None else boards[t])
            memory = torch.where(host['step_valid'][:, t].to(device)[:, None, None], output[0], memory)
            lp, _ = action_logprob_and_kl(output[1].float(), output[2].float(), output[3].float(),
                                          output[1].float(), output[2].float(), output[3].float(), host, t)
            keep = mask[:, t].to(device)
            if keep.any():
                differences.append((lp - host['behavior_logprob'][:, t].to(device))[keep].cpu().numpy())
                heads.append(host['head'][:, t].to(device)[keep].cpu().numpy())
                values.append(output[4].float()[keep].cpu().numpy())
        if number % 200 == 0:
            print(f"[precision-gap] batch {number + 1}/{len(loader)} choices={sum(map(len, differences))}", flush=True)
    net.clear_structure()
    return np.concatenate(differences), np.concatenate(heads), np.concatenate(values)


def summary(log_ratio):
    """The gap as PPO meets it: clipped at ratio 1 +- .2 by default."""
    absolute = np.abs(log_ratio).astype(np.float64)
    ratio = np.exp(log_ratio.astype(np.float64))
    quantile = lambda share: float(np.quantile(absolute, share))
    return dict(choices=len(log_ratio), mean_log_ratio=float(log_ratio.astype(np.float64).mean()),
                mean_ratio=float(ratio.mean()), ratio_min=float(ratio.min()), ratio_max=float(ratio.max()),
                abs_mean=float(absolute.mean()), abs_p50=quantile(.5), abs_p90=quantile(.9), abs_p99=quantile(.99),
                abs_p999=quantile(.999), abs_max=float(absolute.max()),
                above_audit_001=float((absolute > .001).mean()), above_audit_002=float((absolute > .002).mean()),
                above_01=float((absolute > .01).mean()), above_05=float((absolute > .05).mean()),
                outside_clip_02=float((np.abs(ratio - 1) > .2).mean()),
                outside_clip_01=float((np.abs(ratio - 1) > .1).mean()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True)
    parser.add_argument("--data", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--max-games", type=int, default=0)
    parser.add_argument("--lanes", type=int, default=16)
    parser.add_argument("--board-chunk", type=int, default=8)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--device", default="cuda")
    args = parser.parse_args()
    device = torch.device(args.device)
    _, saved, net = load_source(args.model)
    net.to(device)
    dataset = D.TrajectoryDataset4(Path(args.data), split="train", unroll_len=16, encoder_version=saved["encoder_version"])
    if dataset.structure.digest != net.rules.digest:
        raise ValueError("Dataset and model use different rule catalogs")
    if args.max_games:
        dataset.trajectories = dataset.trajectories[:2 * args.max_games]
    log_ratio, heads, _ = gaps(net, dataset, device, args.lanes, args.board_chunk, args.workers)
    if not np.isfinite(log_ratio).all():
        raise FloatingPointError("Nonfinite replayed log probability")
    report = dict(data=args.data, model=args.model, all=summary(log_ratio))
    for kind, name in enumerate(("single", "subset", "pair")):
        if (heads == kind).any():
            report[name] = summary(log_ratio[heads == kind])
    Path(args.out).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["all"]), flush=True)


if __name__ == "__main__":
    main()
