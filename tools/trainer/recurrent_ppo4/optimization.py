"""Full-model PPO transactions checked against all admitted behavior decisions."""
import copy
import math
import torch
from checkpoint4 import rng_state, restore_rng
from .forward import segment_outputs
from .collection import stream_collection_probabilities


def behavior_divergence(current, behavior):
    delta = current.double() - behavior.double()
    estimate = delta.exp() - 1 - delta
    if not bool(torch.isfinite(estimate).all()):
        raise FloatingPointError("Nonfinite behavior divergence")
    return estimate.sum()


@torch.no_grad()
def collection_divergence(net, dataset, prefixes, device, batch_size):
    total, count = 0.0, 0
    try:
        for current, behavior in stream_collection_probabilities(
                net, dataset, prefixes, device, batch_size):
            total += float(behavior_divergence(current, behavior))
            count += current.numel()
        if not count:
            raise ValueError("No admitted policy decisions")
        return total / count
    finally:
        net.clear_structure()


def ppo_loss(output, clip_eps=.2, beta_kl=.02):
    lp, old, advantage = (output[name] for name in ("logprob", "behavior", "advantage"))
    if not lp.numel():
        # Passive-only segments still train the critic and recurrence.
        policy = output["value"].sum() * 0
    else:
        ratio = (lp - old).exp()
        policy = -torch.minimum(ratio * advantage, ratio.clamp(1-clip_eps, 1+clip_eps) * advantage).mean()
    value = (output["value"] - output["returns"]).square().mean()
    regularizer = behavior_divergence(lp, old) / max(lp.numel(), 1)
    return policy + .5 * value + beta_kl * regularizer


def transactional_update(net, optimizer, dataset, indices, prefixes, device,
                         target_kl=.02, clip_eps=.2, audit_batch_size=4, beta_kl=.02, extra_loss=None):
    before = {key: value.detach().cpu().clone() for key, value in net.state_dict().items()}
    optimizer_before = copy.deepcopy(optimizer.state_dict())
    random_before = rng_state()
    record = dict(accepted=False, reason="unknown", loss=None, collection_behavior_kl=None)
    try:
        optimizer.zero_grad(set_to_none=True)
        output = segment_outputs(net, dataset, indices, prefixes, device)
        record.update(decisions=output["logprob"].numel(),frames=output["value"].numel())
        loss = ppo_loss(output, clip_eps, beta_kl)
        if extra_loss is not None:
            loss = loss + extra_loss()
        if not bool(torch.isfinite(loss)):
            raise FloatingPointError("Nonfinite PPO loss")
        loss.backward()
        norm = torch.nn.utils.clip_grad_norm_(net.parameters(), 1, error_if_nonfinite=True)
        optimizer.step()
        net.clear_structure()
        divergence = collection_divergence(net, dataset, prefixes, device, audit_batch_size)
        record.update(loss=float(loss.detach()), grad_norm=float(norm),
                      collection_behavior_kl=divergence)
        if not math.isfinite(divergence) or divergence > target_kl:
            record["reason"] = "collection_behavior_kl_rejected"
        else:
            record.update(accepted=True, reason="accepted")
    except FloatingPointError as error:
        record["reason"] = str(error)
    except BaseException:
        net.load_state_dict(before, strict=True)
        optimizer.load_state_dict(optimizer_before)
        restore_rng(random_before)
        net.clear_structure()
        raise
    if not record["accepted"]:
        net.load_state_dict(before, strict=True)
        optimizer.load_state_dict(optimizer_before)
        restore_rng(random_before)
        net.clear_structure()
    return record
