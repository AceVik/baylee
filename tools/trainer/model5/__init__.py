"""BayleeAI v5: gated relational processing over the provisional Encoder7 ABI."""
from dataclasses import dataclass
import torch
from torch import nn
from torch.utils.checkpoint import checkpoint
import model4 as V4
from model4.config import *

MODEL_VERSION = 5
ARCHITECTURE = "baylee_ai_v5_relational_v1"


@dataclass
class Config5(V4.Config4):
    relational_steps: int = 2
    relational_width: int = 160

    def __post_init__(self):
        if self.relational_steps not in (0, 1, 2, 3, 4):
            raise ValueError("relational_steps must be between zero and four")
        if self.relational_width < 1:
            raise ValueError("relational_width must be positive")


from .relational_v1 import RelationalProcessor


class PlayerNet5(V4.PlayerNet4):
    def __init__(self, cfg: Config5):
        super().__init__(cfg)
        with torch.random.fork_rng(devices=[]):
            self.relational = RelationalProcessor(cfg)

    def relational_tokens(self, tokens, edge_index, edge_type, edge_mask, present, entity_feat):
        if self.cfg.relational_steps == 0:
            return tokens
        # Encoder7 f122 already encodes public grouped-object multiplicity.
        count = torch.cat((torch.zeros_like(tokens[:, :RECURRENT_SLOTS, 0]), torch.log1p(entity_feat[..., 122].clamp(min=0))), 1)
        arguments = (tokens, edge_index, edge_type, edge_mask, present, count)
        if self.cfg.activation_checkpointing and self.training and torch.is_grad_enabled():
            return checkpoint(self.relational, *arguments, use_reentrant=False)
        return self.relational(*arguments)


def warmstart_from_v4(state_dict, net):
    """Explicit new-experiment transfer; never use this to repair a v5 checkpoint."""
    upgraded = V4.upgrade_state_dict_to_v7(state_dict, net)
    expected = net.state_dict()
    base = {k for k in expected if not k.startswith('relational.')}
    if set(upgraded) != base:
        raise ValueError("V4 warmstart has missing or unexpected parameters")
    return {**upgraded, **{k: v for k, v in expected.items() if k.startswith('relational.')}}

upgrade_state_dict_to_v7 = V4.upgrade_state_dict_to_v7
