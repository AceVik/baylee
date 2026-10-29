"""Current-policy episode reconstruction and differentiable TBPTT unrolls."""
from contextlib import nullcontext
import torch
import dataset4 as D
import model4 as M
from policy_math4 import action_logprob_and_kl
from training4.execution import encode_boards


def episode_prefixes(dataset):
    prefixes = {}
    for trajectory in dataset.trajectories:
        prefix = []
        for segment in trajectory:
            if segment in prefixes:
                raise ValueError("Segment belongs to multiple recurrent histories")
            prefixes[segment] = tuple(prefix)
            prefix.append(segment)
    return prefixes


def step_inputs(batch, t, device):
    values = [batch[name][:, t].to(device) for name in M.INPUT_NAMES[:-2]]
    recorded = batch["thinking_steps"][:, t].to(device)
    decision = batch["is_decision"][:, t].to(device)
    k = torch.where(decision, recorded.clamp_min(1), 0)
    if bool((k > 8).any()):
        raise ValueError("Recorded thinking depth exceeds model contract")
    return values, k


def advance(net, batch, h, device, *, board_chunk=0, phase=None):
    scope = phase or (lambda _name: nullcontext())
    with scope("board_encoding"):
        boards = encode_boards(net, batch, board_chunk, device)
    outputs = []
    with scope("advance"):
        for t in range(batch["step_valid"].shape[1]):
            inputs, k = step_inputs(batch, t, device)
            if boards is None:
                out = net(*inputs, h, k)
            else:
                out = net(*inputs, h, k, encoded_board=boards[t])
            valid = batch["step_valid"][:, t].to(device)
            h = torch.where(valid[:, None, None], out[0].float(), h)
            outputs.append(out)
    return outputs, h


@torch.no_grad()
def reconstruct_hidden(net, dataset, indices, prefixes, device):
    """Exact prefix burn-in under current weights, detached only at U boundary.

    No stored behavior H or spatial features enter the learner's recurrence.
    Reconstructing from the episode start avoids a stale-memory approximation.
    """
    hidden = []
    net.clear_structure()
    for segment in indices:
        if segment not in prefixes:
            raise ValueError("Segment is outside admitted learner histories")
        h = torch.zeros(1, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
        for previous in prefixes[segment]:
            host = D.collate_trajectories([dataset[previous]])
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            _, h = advance(net, host, h, device)
        hidden.append(h)
    net.clear_structure()
    return torch.cat(hidden, 0).detach()


def segment_outputs(net, dataset, indices, prefixes, device, gradients=True):
    net.eval()
    with nullcontext() if gradients else torch.no_grad():
        h = reconstruct_hidden(net, dataset, indices, prefixes, device)
        batch = D.collate_trajectories([dataset[index] for index in indices])
        # Structure representations also need current-weight gradients; the
        # no-grad burn-in cache must never leak into the differentiable unroll.
        net.clear_structure()
        net.prepare_structure((batch["entity_rules"], batch["cand_rules"]))
        outputs, final_h = advance(net, batch, h, device)
        current, recorded, advantage = [], [], []
        values, returns = [], []
        for t, out in enumerate(outputs):
            mask = batch["actor_mask"][:, t].to(device).bool()
            lp, _ = action_logprob_and_kl(*out[1:4], *[x.detach() for x in out[1:4]], batch, t)
            current.append(lp[mask])
            recorded.append(batch["behavior_lp"][:, t].to(device)[mask])
            advantage.append(batch["advantages"][:, t].to(device)[mask])
            valid = batch["step_valid"][:, t].to(device).bool()
            values.append(out[4][valid])
            returns.append(batch["returns"][:, t].to(device)[valid])
        return dict(logprob=torch.cat(current), behavior=torch.cat(recorded),
                    advantage=torch.cat(advantage), value=torch.cat(values),
                    returns=torch.cat(returns), final_h=final_h, outputs=outputs,
                    batch=batch)
