"""Opt-in partial training: which parameters learn, and a mergeable candidate adapter.

Stage A (`--train-policy-head-only`): the trunk and everything before the
candidate head stay frozen; the candidate head learns (`cand_proj`,
`cand_scorer`, `cand_ctx_proj`, and `cand_context_gate`/`cand_decoder` where the
architecture has them). `dsl_proj` and the rule encoder also feed board tokens,
so they stay frozen.

Stage A with an adapter (`--adapter-on-candidate-head R`): every parameter is
frozen and a rank-R update `B A` (B starts at zero) is added to the candidate
head's input layer `cand_scorer[0]`, which reads `[candidate, its entity, its
seat, global]`. A saved state dict merges it (`W + B A`), so checkpoints, the
exporter and the Rust actor see the unchanged architecture and keys.

Stage B (`--trainable-trunk-from-layer L`): trunk layers L and above,
`trunk_norm`, the thinking block and every head learn; the token encoders, rule
encoder, relational block, lower layers and the memory (`recurrent_cell`,
`h_norm`) stay frozen. With frozen lower layers autograd records nothing below
layer L, so the backward pass stops there by itself.

`--freeze-value-head` freezes `seat_value_head` and `aux_head` in any mode.

A net with the deliberation block (thinking revision 2) trains it wherever the
candidate head trains: with the head in stage A ("A + block") and with the
upper trunk in stage B ("B + block"). The block sits after the heads, so the
boards and memory of stage A stay r1's and the reference shares them as before.
The adapter mode leaves it frozen. `--train-thinking-only` (the block alone)
is its own mode and combines with none of these.
"""
from __future__ import annotations

import torch
from torch import nn

THINKING_PREFIX = "deliberation."
HEAD_PREFIXES = ("cand_proj.", "cand_scorer.", "cand_ctx_proj.", "cand_context_gate.", "cand_decoder.",
                 THINKING_PREFIX)
OTHER_HEADS = ("pair_blk_proj.", "pair_atk_proj.", "pair_scorer.", "scalar_head.")
VALUE_HEADS = ("seat_value_head.", "aux_head.")


class LowRankAdapter(nn.Module):
    """`base(x) + up(down(x))`; `up` starts at zero, so outputs start unchanged."""

    def __init__(self, base: nn.Linear, rank: int):
        super().__init__()
        if rank < 1:
            raise ValueError("Adapter rank must be positive")
        self.base = base
        self.down = nn.Linear(base.in_features, rank, bias=False)
        self.up = nn.Linear(rank, base.out_features, bias=False)
        nn.init.zeros_(self.up.weight)
        for parameter in self.base.parameters():
            parameter.requires_grad_(False)

    def forward(self, x):
        return self.base(x) + self.up(self.down(x))

    def merged_weight(self):
        with torch.no_grad():
            return self.base.weight + self.up.weight.float() @ self.down.weight.float()


def _set_trainable(net, predicate):
    names = []
    for name, parameter in net.named_parameters():
        parameter.requires_grad_(bool(predicate(name)))
        if parameter.requires_grad:
            names.append(name)
    return names


def apply_partial_training(net, args):
    """Freeze per the opt-in flags; returns a description, or None when all are off."""
    head_only = getattr(args, "train_policy_head_only", False)
    rank = getattr(args, "adapter_on_candidate_head", 0)
    from_layer = getattr(args, "trainable_trunk_from_layer", -1)
    freeze_value = getattr(args, "freeze_value_head", False)
    if not (head_only or rank or from_layer >= 0 or freeze_value):
        return None
    if head_only:
        mode = "policy_head"
        _set_trainable(net, lambda n: n.startswith(HEAD_PREFIXES))
    elif rank:
        mode = "candidate_adapter"
        _set_trainable(net, lambda n: False)
        base = net.cand_scorer[0]
        net.cand_scorer[0] = LowRankAdapter(base, rank).to(base.weight.device)
    elif from_layer >= 0:
        if from_layer > len(net.trunk_layers):
            raise ValueError("--trainable-trunk-from-layer exceeds the trunk depth")
        mode = "upper_trunk"
        upper = tuple(f"trunk_layers.{i}." for i in range(from_layer, len(net.trunk_layers)))
        _set_trainable(net, lambda n: n.startswith(upper + ("trunk_norm.", "thinking_block.")
                                                   + HEAD_PREFIXES + OTHER_HEADS + VALUE_HEADS))
    else:
        mode = "full"
    if freeze_value:
        for name, parameter in net.named_parameters():
            if name.startswith(VALUE_HEADS):
                parameter.requires_grad_(False)
    trainable = [n for n, p in net.named_parameters() if p.requires_grad]
    if not trainable:
        raise ValueError("Partial training leaves no trainable parameter")
    description = dict(version=1, mode=mode, adapter_rank=rank, trainable_trunk_from_layer=from_layer,
                       freeze_value_head=freeze_value,
                       trainable_parameters=sum(p.numel() for p in net.parameters() if p.requires_grad))
    # Named only for a net that has the block, so every other run's identity stays as it was.
    if any(n.startswith(THINKING_PREFIX) for n in trainable):
        description["deliberation"] = "trains"
    return description


def below_layer_frozen(net, layer):
    """True when everything that feeds trunk layer `layer` is frozen (stage B's lower part)."""
    downstream = (tuple(f"trunk_layers.{i}." for i in range(layer, len(net.trunk_layers)))
                  + ("trunk_norm.", "thinking_block.", "recurrent_cell.", "h_norm.")
                  + HEAD_PREFIXES + OTHER_HEADS + VALUE_HEADS)
    return not any(p.requires_grad for n, p in net.named_parameters() if not n.startswith(downstream))


def upstream_frozen(net):
    """True when nothing before the policy heads learns: boards and memory are fixed."""
    return not any(p.requires_grad for n, p in net.named_parameters()
                   if not n.startswith(HEAD_PREFIXES + OTHER_HEADS + VALUE_HEADS))


def state_dict_for_save(net):
    """The architecture's own keys; adapters are merged. Unchanged without adapters."""
    state = net.state_dict()
    adapters = [(name, module) for name, module in net.named_modules() if isinstance(module, LowRankAdapter)]
    if not adapters:
        return state
    state = dict(state)
    for name, module in adapters:
        for key in ("base.weight", "base.bias", "down.weight", "up.weight"):
            state.pop(f"{name}.{key}", None)
        state[f"{name}.weight"] = module.merged_weight().to(module.base.weight.dtype)
        if module.base.bias is not None:
            state[f"{name}.bias"] = module.base.bias.detach()
    return state
