"""Linear-time, current-weight recurrent inference for a complete PPO audit."""

import torch

import dataset4 as D
import model4 as M
from policy_math4 import action_logprob_and_kl
from .forward import advance


def stream_collection_probabilities(net, dataset, prefixes, device, batch_size):
    """Visit each segment once while weights are fixed for this entire audit.

    State starts at zero for each admitted history. It is carried only within
    that history, never taken from the behavior cache or an earlier audit. The
    optimizer's differentiable unroll/burn-in remains a separate computation.
    The caller owns no_grad and clears the rule cache on completion/failure.
    """
    if batch_size < 1:
        raise ValueError("Collection audit batch size must be positive")
    net.eval()
    hidden = None
    previous = None
    sampler = D.StatefulBatchSampler4(dataset, batch_size, shuffle=False)
    for indices in sampler:
        if hidden is None:
            hidden = torch.zeros(len(indices), M.RECURRENT_SLOTS, net.cfg.d_model,
                                 device=device)
            previous = [None] * len(indices)
        reset = []
        for lane, index in enumerate(indices):
            prefix = () if index < 0 else prefixes[index]
            if prefix and previous[lane] != prefix[-1]:
                raise ValueError("Collection audit lane is not a contiguous history")
            reset.append(not prefix)
            previous[lane] = index if index >= 0 else None
        hidden = torch.where(torch.tensor(reset, device=device)[:, None, None],
                             torch.zeros_like(hidden), hidden)
        batch = D.collate_trajectories([dataset[index] for index in indices])
        net.clear_structure()
        net.prepare_structure((batch["entity_rules"], batch["cand_rules"]))
        outputs, hidden = advance(net, batch, hidden, device)
        for t, output in enumerate(outputs):
            mask = (batch["actor_mask"][:, t] & batch["step_valid"][:, t]).to(device).bool()
            logprob, _ = action_logprob_and_kl(
                *output[1:4], *output[1:4], batch, t)
            yield logprob[mask], batch["behavior_lp"][:, t].to(device)[mask]
