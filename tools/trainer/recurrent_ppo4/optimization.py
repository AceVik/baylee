"""Full-model PPO transactions checked against all admitted behavior decisions."""
import copy
import math
import torch
from checkpoint4 import rng_state, restore_rng
from policy_training4.drift import Drift, SourceReadouts
from .forward import segment_outputs
from .collection import stream_collection_batches, stream_collection_probabilities


def behavior_divergence(current, behavior):
    delta = current.double() - behavior.double()
    estimate = delta.exp() - 1 - delta
    if not bool(torch.isfinite(estimate).all()):
        raise FloatingPointError("Nonfinite behavior divergence")
    return estimate.sum()


@torch.no_grad()
def source_readouts(net, dataset, prefixes, device, batch_size):
    """The frozen source's readouts of the decisions outside the probability contract, from one
    pass over the collection before any update (and before a resume restores one)."""
    source = SourceReadouts()
    try:
        for indices, batch, outputs in stream_collection_batches(net, dataset, prefixes, device, batch_size):
            for t, output in enumerate(outputs):
                source.observe(indices, batch, t, output)
        return source
    finally:
        net.clear_structure()


@torch.no_grad()
def collection_divergence(net, dataset, prefixes, device, batch_size, drift=None):
    """The mean behavior divergence of the certified decisions. `drift` (a fresh `Drift`)
    collects, from the same pass, that of the decisions outside the contract."""
    total, count = 0.0, 0
    try:
        for current, behavior in stream_collection_probabilities(
                net, dataset, prefixes, device, batch_size,
                observe=None if drift is None else drift.observe):
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
                         target_kl=.02, clip_eps=.2, audit_batch_size=4, beta_kl=.02, extra_loss=None,
                         source=None):
    """One update, kept only if the whole collection stays within `target_kl` of behavior. With
    the frozen source's readouts (`source_readouts`) that holds, head by head, for the decisions
    outside the probability contract too; they enter no loss."""
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
        drift = None if source is None else Drift(source)
        divergence = collection_divergence(net, dataset, prefixes, device, audit_batch_size, drift)
        record.update(loss=float(loss.detach()), grad_norm=float(norm),
                      collection_behavior_kl=divergence)
        drifted = []
        if drift is not None:
            drifted = drift.refused(target_kl)
            record.update(uncertified_drift=drift.report())
        if not math.isfinite(divergence) or divergence > target_kl:
            record["reason"] = "collection_behavior_kl_rejected"
        elif drifted:
            record["reason"] = "uncertified_drift_rejected: " + ", ".join(drifted)
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
