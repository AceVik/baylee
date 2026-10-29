"""BayleeAI v5: gated relational processing over the provisional Encoder7 ABI."""
from dataclasses import dataclass
import torch
from torch import nn
from model4.activation_checkpoint import checkpoint_activation
import model4 as V4
from model4.config import *

MODEL_VERSION = 5
ARCHITECTURE = "baylee_ai_v5_relational_v1"
CANDIDATE_ARCHITECTURE = "baylee_ai_v5_relational_v2_candidate_v2"
ARCHITECTURES = {1: ARCHITECTURE, 2: "baylee_ai_v5_relational_v2"}


def architecture_for_config(cfg):
    if cfg.candidate_context_revision == 2:
        return CANDIDATE_ARCHITECTURE
    return ARCHITECTURES[cfg.relational_revision]


@dataclass
class Config5(V4.Config4):
    candidate_context_revision: int = 1
    relational_revision: int = 1
    relational_steps: int = 2
    relational_width: int = 160

    def __post_init__(self):
        if type(self.candidate_context_revision) is not int or self.candidate_context_revision not in (1, 2):
            raise ValueError("Unsupported candidate context revision")
        if self.candidate_context_revision == 2 and self.relational_revision != 2:
            raise ValueError("Candidate context revision2 requires relational revision2")
        if self.relational_revision not in ARCHITECTURES:
            raise ValueError("Unsupported relational revision")
        if self.relational_steps not in (0, 1, 2, 3, 4):
            raise ValueError("relational_steps must be between zero and four")
        if self.relational_width < 1:
            raise ValueError("relational_width must be positive")


from .relational_v1 import RelationalProcessor
from .relational_v2 import RelationalProcessorV2


class PlayerNet5(V4.PlayerNet4):
    def __init__(self, cfg: Config5):
        cfg.__post_init__()
        super().__init__(cfg)
        with torch.random.fork_rng(devices=[]):
            processor = RelationalProcessor if cfg.relational_revision == 1 else RelationalProcessorV2
            self.relational = processor(cfg)
            if cfg.candidate_context_revision == 2:
                self.cand_context_gate = nn.Linear(2 * cfg.d_model, cfg.d_model)
                nn.init.zeros_(self.cand_context_gate.weight)
                nn.init.zeros_(self.cand_context_gate.bias)

    def relational_tokens(self, tokens, edge_index, edge_type, edge_mask, present, entity_feat):
        if self.cfg.relational_steps == 0:
            return tokens
        # Encoder7 f122 already encodes public grouped-object multiplicity.
        raw_count = entity_feat[..., 122].clamp(min=0)
        count = (torch.cat((torch.zeros_like(tokens[:, :RECURRENT_SLOTS, 0]), torch.log1p(raw_count)), 1)
                 if self.cfg.relational_revision == 1 else
                 torch.cat((torch.ones_like(tokens[:, :RECURRENT_SLOTS, 0]), raw_count), 1))
        arguments = (tokens, edge_index, edge_type, edge_mask, present, count)
        if self.cfg.activation_checkpointing and (self.training or getattr(self, "checkpoint_during_eval", False)) and torch.is_grad_enabled():
            return checkpoint_activation(
                self.relational, *arguments,
                offload_inputs=getattr(self, "offload_checkpoint_inputs", False),
            )
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


def warmstart_candidate_context(state_dict, net):
    """Explicit weights-only transfer after validating the source config/identity.

    The caller must start a fresh optimizer, scheduler and recurrent lanes.
    Never use this to repair or resume a checkpoint with a different identity.
    """
    if net.cfg.candidate_context_revision != 2 or net.cfg.relational_revision != 2:
        raise ValueError("Candidate context warmstart requires v5 relational2 candidate2")
    expected = net.state_dict()
    new_keys = {"cand_context_gate.weight", "cand_context_gate.bias"}
    if set(state_dict) != set(expected) - new_keys:
        raise ValueError("Candidate warmstart has missing or unexpected source tensors")
    for key, value in state_dict.items():
        if value.shape != expected[key].shape or value.dtype != expected[key].dtype:
            raise ValueError(f"Candidate warmstart tensor differs: {key}")
    return {**state_dict, **{key: torch.zeros_like(expected[key]) for key in new_keys}}
