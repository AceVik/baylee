"""Replay full unrolls from immutable behavior memory; gather only actor rows."""
import torch
import dataset4 as D
import model4 as M
from train_rl4 import action_logprob_and_kl
from training4.execution import encode_boards, transfer_batch
from .drift import outside


def batch_rows(dataset, indices, device):
    host = D.collate_trajectories([dataset[index] for index in indices])
    return transfer_batch(host, device)


def replay(net, batch, board_chunk, wanted):
    """Every frame's (t, outputs) from the stored behavior memory, with the policy heads that the
    rows of `wanted(t)` ([B], on the model's device) read."""
    device = next(net.parameters()).device
    h = batch["behavior_h"].to(device).detach()
    u = batch["step_valid"].shape[1]
    # Spatial encoding is entirely frozen in this entry point. Grouping only
    # that work retains independent boards and the same chronological H updates.
    with torch.no_grad():
        net.clear_structure()
        net.prepare_structure((batch['entity_rules'], batch['cand_rules']))
        boards = encode_boards(net, batch, board_chunk, device)
    for t in range(u):
        at = lambda name: batch[name][:, t].to(device)
        decision = at("is_decision")
        stored = at("thinking_steps")
        source_net = at("source_kind") == 0
        k = torch.where(decision, torch.where(source_net & (stored > 0), stored, 1), 0)
        if (k > 8).any():
            raise ValueError("Recorded thinking depth exceeds model contract")
        mask, heads = wanted(t), at("head")
        enabled = tuple(bool(((heads == kind) & mask).any()) for kind in range(4))
        # One rare NameCard question must not inflate every other frame's graph.
        inputs = []
        extents = {}
        for prefix in ("cand", "pair"):
            present = at(prefix + "_mask").any(0).nonzero().flatten()
            extents[prefix] = int(present[-1]) + 1 if len(present) else 1
        for name in M.INPUT_NAMES[:-2]:
            value = batch[name][:, t]
            if name.startswith("cand_"):
                value = value[:, :extents["cand"]]
            elif name.startswith("pair_"):
                value = value[:, :extents["pair"]]
            inputs.append(value.to(device))
        outputs = net(*inputs, h, k, policy_heads=enabled,
                      encoded_board=None if boards is None else boards[t])
        h = torch.where(at("step_valid")[:, None, None], outputs[0], h)
        yield t, outputs


def actor_outputs(net, batch, board_chunk=0):
    device = next(net.parameters()).device
    actor = lambda t: batch["actor_mask"][:, t].to(device).bool()
    current, recorded, advantage = [], [], []
    for t, outputs in replay(net, batch, board_chunk, actor):
        mask = actor(t)
        if mask.any():
            at = lambda name: batch[name][:, t].to(device)
            cl, pl, scalar = outputs[1:4]
            lp, _ = action_logprob_and_kl(cl, pl, scalar, cl.detach(), pl.detach(), scalar.detach(), batch, t)
            current.append(lp[mask])
            rec_lp = at("behavior_lp") if "behavior_lp" in batch else at("behavior_logprob")
            recorded.append(rec_lp[mask])
            advantage.append(at("advantages")[mask])
    if not current:
        raise ValueError("Optimizer microbatch contains no actor choices")
    return tuple(torch.cat(values) for values in (current, recorded, advantage))


@torch.no_grad()
def observe_outside(net, dataset, indices, device, board_chunk, observe):
    """Show `observe` (`policy_training4.drift`) the current readouts of these segments'
    decisions outside the action-probability contract."""
    batch = batch_rows(dataset, indices, device)
    found = [outside(batch, t) for t in range(batch["step_valid"].shape[1])]
    if not any(found):
        return
    nothing = torch.zeros(len(indices), dtype=torch.bool)
    wanted = [torch.stack([*rows.values(), nothing]).any(0).to(device) for rows in found]
    for t, outputs in replay(net, batch, board_chunk, lambda t: wanted[t]):
        observe(indices, batch, t, outputs)
