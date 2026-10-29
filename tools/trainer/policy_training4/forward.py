"""Replay full unrolls from immutable behavior memory; gather only actor rows."""
import torch
import dataset4 as D
import model4 as M
from train_rl4 import action_logprob_and_kl
from training4.execution import encode_boards, transfer_batch


def batch_rows(dataset, indices, device):
    host = D.collate_trajectories([dataset[index] for index in indices])
    return transfer_batch(host, device)


def actor_outputs(net, batch, board_chunk=0):
    device = next(net.parameters()).device
    h = batch["behavior_h"].to(device).detach()
    b, u = batch["step_valid"].shape
    # Spatial encoding is entirely frozen in this entry point. Grouping only
    # that work retains independent boards and the same chronological H updates.
    with torch.no_grad():
        net.clear_structure()
        net.prepare_structure((batch['entity_rules'], batch['cand_rules']))
        boards = encode_boards(net, batch, board_chunk, device)
    current, recorded, advantage = [], [], []
    for t in range(u):
        at = lambda name: batch[name][:, t].to(device)
        decision = at("is_decision")
        stored = at("thinking_steps")
        source_net = at("source_kind") == 0
        k = torch.where(decision, torch.where(source_net & (stored > 0), stored, 1), 0)
        if (k > 8).any():
            raise ValueError("Recorded thinking depth exceeds model contract")
        mask = at("actor_mask").bool()
        heads = at("head")
        enabled = tuple(bool(((heads == kind) & mask).any()) for kind in range(3)) + (False,)
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
        if mask.any():
            cl, pl, scalar = outputs[1:4]
            lp, _ = action_logprob_and_kl(cl, pl, scalar, cl.detach(), pl.detach(), scalar.detach(), batch, t)
            current.append(lp[mask])
            rec_lp = at("behavior_lp") if "behavior_lp" in batch else at("behavior_logprob")
            recorded.append(rec_lp[mask])
            advantage.append(at("advantages")[mask])
    if not current:
        raise ValueError("Optimizer microbatch contains no actor choices")
    return tuple(torch.cat(values) for values in (current, recorded, advantage))
