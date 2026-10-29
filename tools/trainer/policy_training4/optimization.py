"""Decision-normalized groups and rollback of both parameters and Adam state."""
import copy
import numpy as np
import torch

POLICY_PREFIXES = ("cand_proj.", "cand_ctx_proj.", "cand_context_gate.", "cand_scorer.", "pair_blk_proj.", "pair_atk_proj.", "pair_scorer.")


def select_policy_parameters(net):
    net.eval().requires_grad_(False)
    selected = []
    for name, parameter in net.named_parameters():
        if name.startswith(POLICY_PREFIXES):
            parameter.requires_grad_(True)
            selected.append(parameter)
    if not selected:
        raise ValueError("No action-head parameters found")
    return selected


def plan_groups(actor_mask, seed, minimum=128, microbatch_size=8):
    """Randomize independent segments; cached behavior H preserves their entry."""
    counts = np.asarray(actor_mask).sum(-1)
    order = np.flatnonzero(counts > 0)
    np.random.default_rng(seed).shuffle(order)
    groups, pending, decisions = [], [], 0
    for index in order:
        pending.append(int(index))
        decisions += int(counts[index])
        if decisions >= minimum:
            groups.append((tuple(tuple(pending[start:start+microbatch_size])
                                 for start in range(0, len(pending), microbatch_size)), decisions))
            pending, decisions = [], 0
    return groups, dict(zero_actor_segments=int((counts == 0).sum()),
                         discarded_tail_segments=len(pending), discarded_tail_decisions=decisions)


def behavior_kl(logprob, recorded):
    difference = logprob.double() - recorded.double()
    if not torch.isfinite(difference).all() or (difference.abs() > 40).any():
        raise FloatingPointError("Nonfinite or extreme behavior log-ratio")
    return (torch.expm1(difference) - difference).sum()


def surrogate_sum(logprob, recorded, advantage, clip_eps):
    difference = logprob - recorded
    if not torch.isfinite(difference).all() or (difference.abs() > 40).any():
        raise FloatingPointError("Nonfinite or extreme PPO log-ratio")
    ratio = difference.exp()
    return -torch.minimum(ratio * advantage, ratio.clamp(1-clip_eps, 1+clip_eps) * advantage).sum()


def transactional_step(parameters, optimizer, microbatches, decisions, loss_sum, kl_sum, target_kl):
    """Callbacks return numerators; no per-microbatch means enter the update."""
    if decisions < 1:
        return dict(accepted=False, reason="zero_actor_decisions", decisions=0)
    weights = [p.detach().clone() for p in parameters]
    optimizer_state = copy.deepcopy(optimizer.state_dict())
    optimizer.zero_grad(set_to_none=True)
    total_loss = 0.0
    try:
        for batch in microbatches:
            numerator = loss_sum(batch)
            if not torch.isfinite(numerator):
                raise FloatingPointError("Nonfinite PPO loss")
            (numerator / decisions).backward()
            total_loss += float(numerator.detach())
        if any(p.grad is not None and not torch.isfinite(p.grad).all() for p in parameters):
            raise FloatingPointError("Nonfinite policy gradients")
        gradient_norm = torch.nn.utils.clip_grad_norm_(parameters, 1.0, error_if_nonfinite=True)
        if float(gradient_norm) == 0:
            raise FloatingPointError("zero_policy_gradient; no optimizer mutation")
        optimizer.step()
        with torch.no_grad():
            kl = sum(float(kl_sum(batch)) for batch in microbatches) / decisions
        if not np.isfinite(kl) or kl > target_kl:
            raise FloatingPointError(f"post_step_behavior_kl={kl}")
        return dict(accepted=True, decisions=decisions, loss=total_loss/decisions,
                    gradient_norm=float(gradient_norm), behavior_kl=kl)
    except (FloatingPointError, RuntimeError) as error:
        # Restore Adam counters/moments too: rejected attempts cannot affect retry.
        with torch.no_grad():
            for parameter, previous in zip(parameters, weights):
                parameter.copy_(previous)
        optimizer.load_state_dict(optimizer_state)
        optimizer.zero_grad(set_to_none=True)
        if isinstance(error, RuntimeError) and not isinstance(error, FloatingPointError):
            raise
        return dict(accepted=False, decisions=decisions, reason=str(error))


def accept_final_state(net, original, measured_kl, target_kl):
    for key, value in net.state_dict().items():
        if not key.startswith(POLICY_PREFIXES) and not torch.equal(original[key].cpu(), value.cpu()):
            raise AssertionError(f"Frozen tensor changed: {key}")
    if not np.isfinite(measured_kl) or measured_kl > target_kl:
        net.load_state_dict(original, strict=True)
        return False
    return True
