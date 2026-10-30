"""Playernet v4: Recurrent Entity-Graph Set-Transformer with Latent Thinking Steps.

Implements the architecture specified in `docs/proposals/trained-ai-v4.md` and
`docs/history/trained-ai-v4-handover-2026-10-22.md`:

- Hybrid Card DSL Encoder (`160 -> d_model`) over typed `baylee_cards_dsl`
  card structure, enabling zero-shot generalization to unseen cards.
- 8-seat relative table representation (`RECURRENT_SLOTS = 9`: 1 Global + 8 Seats).
- Typed relation message-passing (`edge_index`, `edge_type`, `edge_mask`) over
  `[Global(1), Seats(8), Entities(E)]`.
- Configurable spatial Pre-Norm Transformer trunk; depth pilots use d=640 and 8/12 layers.
- Per-seat recurrent belief memory (`h_in -> h_out`, shape `[B, 9, d_model]`)
  updated via a gated GRU cell across the game trajectory.
- Latent Thinking Loop (`K` steps of shared-weight cross-attention refinement
  over the board tokens before emitting policy and value logits).
- Unified pointer/set/pair/scalar action heads + multiplayer side-utility value head.
"""

from __future__ import annotations

import torch
from torch import nn
import torch.nn.functional as F
from .activation_checkpoint import checkpoint_activation

from .config import *
from .layers import GatedResidualUpdate, DynamicMultiHeadAttention, PreNormTransformerLayer, ThinkingStepBlock

from .player import PlayerNet4, upgrade_state_dict_to_v7
