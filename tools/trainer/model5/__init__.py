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
DECODER_ARCHITECTURE = "baylee_ai_v5_relational_v2_candidate_v3"
ARCHITECTURES = {1: ARCHITECTURE, 2: "baylee_ai_v5_relational_v2"}
CANDIDATE_ARCHITECTURES = {2: CANDIDATE_ARCHITECTURE, 3: DECODER_ARCHITECTURE}


def architecture_for_config(cfg):
    if cfg.candidate_context_revision in CANDIDATE_ARCHITECTURES:
        return CANDIDATE_ARCHITECTURES[cfg.candidate_context_revision]
    return ARCHITECTURES[cfg.relational_revision]


@dataclass
class Config5(V4.Config4):
    candidate_context_revision: int = 1
    relational_revision: int = 1
    relational_steps: int = 2
    relational_width: int = 160
    # Revision 3 only: attention layers from each candidate into the board.
    candidate_decoder_layers: int = 2

    def __post_init__(self):
        super().__post_init__()
        if self.global_feat_width == 82 and self.relational_revision != 2:
            raise ValueError('Encoder9 width82 requires relational revision2')
        if type(self.candidate_context_revision) is not int or self.candidate_context_revision not in (1, 2, 3):
            raise ValueError("Unsupported candidate context revision")
        if self.candidate_context_revision >= 2 and self.relational_revision != 2:
            raise ValueError("Candidate context revision2+ requires relational revision2")
        if self.candidate_context_revision == 3 and self.global_feat_width != 80:
            raise ValueError("Candidate decoder revision3 is an Encoder7/8 contract")
        if type(self.candidate_decoder_layers) is not int or not 1 <= self.candidate_decoder_layers <= 4:
            raise ValueError("candidate_decoder_layers must be between one and four")
        if self.relational_revision not in ARCHITECTURES:
            raise ValueError("Unsupported relational revision")
        if self.relational_steps not in (0, 1, 2, 3, 4):
            raise ValueError("relational_steps must be between zero and four")
        if self.relational_width < 1:
            raise ValueError("relational_width must be positive")


from .relational_v1 import RelationalProcessor
from .relational_v2 import RelationalProcessorV2
from .candidate_decoder import CandidateDecoderLayer


class PlayerNet5(V4.PlayerNet4):
    def __init__(self, cfg: Config5):
        cfg.__post_init__()
        super().__init__(cfg)
        with torch.random.fork_rng(devices=[]):
            processor = RelationalProcessor if cfg.relational_revision == 1 else RelationalProcessorV2
            self.relational = processor(cfg)
            if cfg.candidate_context_revision >= 2:
                self.cand_context_gate = nn.Linear(2 * cfg.d_model, cfg.d_model)
                nn.init.zeros_(self.cand_context_gate.weight)
                nn.init.zeros_(self.cand_context_gate.bias)
            if cfg.candidate_context_revision == 3:
                self.cand_decoder = nn.ModuleList(
                    CandidateDecoderLayer(cfg.d_model, cfg.n_heads, 2 * cfg.d_model)
                    for _ in range(cfg.candidate_decoder_layers))

    def candidate_decoder_keys(self, z_out, board, pad_mask):
        """Revision 3: the thought-through summary slots, then every entity."""
        if self.cfg.candidate_context_revision != 3:
            return None
        return torch.cat([z_out.to(board.dtype), board[..., RECURRENT_SLOTS:, :]], dim=-2), pad_mask

    def decode_candidates(self, hidden, keys):
        if keys is None:
            return hidden
        tokens, pad = keys
        # Candidates never attend to one another, so a huge menu (naming a
        # card offers thousands) is decoded exactly in bounded slices. Each
        # slice is its own checkpoint: its attention is recomputed alone in
        # backward instead of staying alive until the unroll's backward.
        parts = [hidden] if torch.jit.is_tracing() else hidden.split(512, dim=-2)
        if self.cfg.activation_checkpointing and (self.training or getattr(self, "checkpoint_during_eval", False)) and torch.is_grad_enabled():
            decoded = [checkpoint_activation(self._decode_candidates, part, tokens, pad,
                                             offload_inputs=getattr(self, "offload_checkpoint_inputs", False))
                       for part in parts]
        else:
            decoded = [self._decode_candidates(part, tokens, pad) for part in parts]
        return decoded[0] if len(decoded) == 1 else torch.cat(decoded, dim=-2)

    def _decode_candidates(self, hidden, tokens, pad):
        for layer in self.cand_decoder:
            hidden = layer(hidden, tokens, pad)
        return hidden

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
    if net.cfg.candidate_context_revision not in (2, 3) or net.cfg.relational_revision != 2:
        raise ValueError("Candidate context warmstart requires v5 relational2 candidate2 or 3")
    expected = net.state_dict()
    gate = {"cand_context_gate.weight", "cand_context_gate.bias"}
    decoder = {key for key in expected if key.startswith("cand_decoder.")}
    # A revision-2 source already learned its gate; a revision-1 source starts it at zero.
    added = (decoder | gate) - set(state_dict)
    if set(state_dict) != set(expected) - added or not added or not decoder <= added:
        raise ValueError("Candidate warmstart has missing or unexpected source tensors")
    for key, value in state_dict.items():
        if value.shape != expected[key].shape or value.dtype != expected[key].dtype:
            raise ValueError(f"Candidate warmstart tensor differs: {key}")
    # Fresh decoder weights keep their zero output projections: the source's
    # logits are reproduced exactly until the first update.
    return {**state_dict, **{key: torch.zeros_like(expected[key]) for key in added & gate},
            **{key: expected[key] for key in added & decoder}}


def warmstart_encoder9(state_dict, net):
    """Fresh E7/E8 transfer; new X inputs require trunk learning, not frozen head PPO.

    Zero columns preserve the old tensor function on the same first80 inputs.
    This does not promise E7-to-E9 behavioral equality: E8 context semantics differ.
    Caller must discard optimizer/scheduler/sampler/H state.
    """
    if net.cfg.global_feat_width != 82 or net.cfg.relational_revision != 2:
        raise ValueError('Encoder9 transfer requires width82 relational2')
    expected = net.state_dict()
    if set(state_dict) != set(expected):
        raise ValueError('Encoder9 transfer parameter keys differ')
    transferred = {}
    for key, target in expected.items():
        source = state_dict[key]
        if key == 'global_proj.0.weight':
            if source.shape != (target.shape[0], 80) or source.dtype != target.dtype:
                raise ValueError('Encoder9 transfer requires width80 source global projection')
            value = torch.zeros_like(target)
            value[:, :80] = source
            transferred[key] = value
        else:
            if source.shape != target.shape or source.dtype != target.dtype:
                raise ValueError(f'Encoder9 transfer tensor differs: {key}')
            transferred[key] = source
    return transferred
