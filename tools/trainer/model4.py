"""Playernet v4: Recurrent Entity-Graph Set-Transformer with Latent Thinking Steps.

Implements the architecture specified in `docs/proposals/trained-ai-v4.md` and
`docs/history/trained-ai-v4-handover-2026-10-22.md`:

- Hybrid Card DSL Encoder (`160 -> d_model`) over typed `baylee_cards_dsl`
  card structure, enabling zero-shot generalization to unseen cards.
- 8-seat relative table representation (`RECURRENT_SLOTS = 9`: 1 Global + 8 Seats).
- Typed relation message-passing (`edge_index`, `edge_type`, `edge_mask`) over
  `[Global(1), Seats(8), Entities(E)]`.
- Spatial Pre-Norm Transformer trunk (`d_model` in {256, 384, 512}, `n_layers` in {4, 8}).
- Per-seat recurrent belief memory (`h_in -> h_out`, shape `[B, 9, d_model]`)
  updated via a gated GRU cell across the game trajectory.
- Latent Thinking Loop (`K` steps of shared-weight cross-attention refinement
  over the board tokens before emitting policy and value logits).
- Unified pointer/set/pair/scalar action heads + multiplayer side-utility value head.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
import torch
from torch import nn
import torch.nn.functional as F
from torch.utils.checkpoint import checkpoint

RECURRENT_SLOTS = 9
MAX_SEATS = 8
MAX_ENTITIES = 1024
MAX_EDGES = 8192
MAX_CANDIDATES = 65535
MAX_PAIRS = 65535
ENCODER_VERSION = 5
DSL_VERSION = 2
UNKNOWN_CARD = 2**32 - 1
TRAJECTORY_VERSION = 2
SAMPLING_VERSION = 2
MODEL_VERSION = 2
ENTITY_FEAT_WIDTH = 96
CARD_DSL_WIDTH = 160
SEAT_FEAT_WIDTH = 64
GLOBAL_FEAT_WIDTH = 64
CAND_FEAT_WIDTH = 48
NUM_EDGE_TYPES = 14

# Shared seat_feat column indices (must match `crates/baylee-train/src/features4.rs:encode_seats4`)
SEAT_FEAT_PRESENT = 0
SEAT_FEAT_SELF = 1
SEAT_FEAT_TEAMMATE = 2
SEAT_FEAT_OPPONENT = 3
SEAT_FEAT_LOST = 4

INPUT_NAMES = (
    "entity_feat",
    "entity_dsl",
    "entity_mask",
    "seat_feat",
    "global_feat",
    "edge_index",
    "edge_type",
    "edge_mask",
    "cand_feat",
    "cand_entity_idx",
    "cand_seat_idx",
    "cand_mask",
    "pair_index",
    "pair_mask",
    "cand_dsl",
    "h_in",
    "thinking_steps",
)

OUTPUT_NAMES = (
    "h_out",
    "cand_logits",
    "pair_logits",
    "scalar_pred",
    "value",
)


@dataclass
class Config4:
    d_model: int = 512
    n_layers: int = 8
    n_heads: int = 8
    d_ff: int = 2048
    dropout: float = 0.0
    activation_checkpointing: bool = False
    default_thinking_steps: int = 2
    max_thinking_steps: int = 8

    def to_dict(self) -> dict:
        return asdict(self)


def symlog(x: torch.Tensor) -> torch.Tensor:
    return torch.sign(x) * torch.log1p(torch.abs(x))


class GatedResidualUpdate(nn.Module):
    """GRU-style gated recurrent update for `[B, N, D]` belief tokens."""

    def __init__(self, d_model: int):
        super().__init__()
        self.norm_x = nn.LayerNorm(d_model)
        self.norm_h = nn.LayerNorm(d_model)
        self.gate_z = nn.Linear(d_model * 2, d_model)
        self.gate_r = nn.Linear(d_model * 2, d_model)
        self.cand_h = nn.Linear(d_model * 2, d_model)

    def forward(self, x: torch.Tensor, h: torch.Tensor) -> torch.Tensor:
        xn = self.norm_x(x)
        hn = self.norm_h(h)
        cat = torch.cat([xn, hn], dim=-1)
        z = torch.sigmoid(self.gate_z(cat))
        r = torch.sigmoid(self.gate_r(cat))
        h_tilde = torch.tanh(self.cand_h(torch.cat([xn, r * hn], dim=-1)))
        return (1.0 - z) * h + z * h_tilde


class DynamicMultiHeadAttention(nn.Module):
    """Multi-head attention that uses `-1` for sequence lengths in `reshape`,
    ensuring ONNX exports remain 100% dynamic across arbitrary entity counts `E`."""

    def __init__(self, d_model: int, n_heads: int, dropout: float = 0.0):
        super().__init__()
        assert d_model % n_heads == 0
        self.d_model = d_model
        self.n_heads = n_heads
        self.head_dim = d_model // n_heads
        self.scale = self.head_dim ** -0.5
        self.q_proj = nn.Linear(d_model, d_model)
        self.k_proj = nn.Linear(d_model, d_model)
        self.v_proj = nn.Linear(d_model, d_model)
        self.out_proj = nn.Linear(d_model, d_model)
        self.dropout = nn.Dropout(dropout)

    def forward(
        self,
        q: torch.Tensor,
        k: torch.Tensor,
        v: torch.Tensor,
        key_padding_mask: torch.Tensor | None = None,
    ) -> torch.Tensor:
        B = q.shape[0]
        qh = self.q_proj(q).reshape(B, -1, self.n_heads, self.head_dim).transpose(1, 2)
        kh = self.k_proj(k).reshape(B, -1, self.n_heads, self.head_dim).transpose(1, 2)
        vh = self.v_proj(v).reshape(B, -1, self.n_heads, self.head_dim).transpose(1, 2)

        if not torch.jit.is_tracing():
            allowed = None if key_padding_mask is None else ~key_padding_mask[:, None, None, :]
            out = F.scaled_dot_product_attention(
                qh, kh, vh, attn_mask=allowed,
                dropout_p=self.dropout.p if self.training else 0.0,
            )
        else:
            # Portable ONNX path; numerically checked against fused PyTorch SDPA.
            scores = torch.matmul(qh, kh.transpose(-2, -1)) * self.scale
            if key_padding_mask is not None:
                scores = scores.masked_fill(key_padding_mask[:, None, None, :], -1e9)
            out = torch.matmul(torch.softmax(scores, dim=-1), vh)
        out = out.transpose(1, 2).contiguous().reshape(B, -1, self.d_model)
        return self.out_proj(out)


class PreNormTransformerLayer(nn.Module):
    """Pre-Norm Transformer Encoder layer with dynamic sequence length."""

    def __init__(self, d_model: int, n_heads: int, d_ff: int, dropout: float = 0.0):
        super().__init__()
        self.norm1 = nn.LayerNorm(d_model)
        self.attn = DynamicMultiHeadAttention(d_model, n_heads, dropout=dropout)
        self.norm2 = nn.LayerNorm(d_model)
        self.ff = nn.Sequential(
            nn.Linear(d_model, d_ff),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(d_ff, d_model),
            nn.Dropout(dropout),
        )

    def forward(
        self, x: torch.Tensor, src_key_padding_mask: torch.Tensor | None = None
    ) -> torch.Tensor:
        xn = self.norm1(x)
        x = x + self.attn(xn, xn, xn, key_padding_mask=src_key_padding_mask)
        x = x + self.ff(self.norm2(x))
        return x


class ThinkingStepBlock(nn.Module):
    """Shared-weight latent thinking step: cross-attends from the 9 recurrent
    belief slots into the full `[Global, Seats, Entities]` board representation."""

    def __init__(self, d_model: int, n_heads: int, d_ff: int, dropout: float = 0.0):
        super().__init__()
        self.norm_q = nn.LayerNorm(d_model)
        self.norm_kv = nn.LayerNorm(d_model)
        self.attn = DynamicMultiHeadAttention(d_model, n_heads, dropout=dropout)
        self.norm_ff = nn.LayerNorm(d_model)
        self.ff = nn.Sequential(
            nn.Linear(d_model, d_ff),
            nn.GELU(),
            nn.Linear(d_ff, d_model),
        )
        self.step_emb = nn.Embedding(16, d_model)

    def forward(
        self,
        h: torch.Tensor,
        board_tokens: torch.Tensor,
        board_key_padding_mask: torch.Tensor,
        step_idx: torch.Tensor,
    ) -> torch.Tensor:
        q = self.norm_q(h + self.step_emb(step_idx).unsqueeze(1))
        kv = self.norm_kv(board_tokens)
        attn_out = self.attn(q, kv, kv, key_padding_mask=board_key_padding_mask)
        h = h + attn_out
        h = h + self.ff(self.norm_ff(h))
        return h


class PlayerNet4(nn.Module):
    """Playernet v4 single-step and unrolled trajectory network."""

    def __init__(self, cfg: Config4):
        super().__init__()
        self.cfg = cfg
        d = cfg.d_model

        # 1. Token Encoders
        self.global_proj = nn.Sequential(
            nn.Linear(GLOBAL_FEAT_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.seat_proj = nn.Sequential(
            nn.Linear(SEAT_FEAT_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.seat_rel_emb = nn.Embedding(MAX_SEATS, d)

        self.entity_proj = nn.Sequential(
            nn.Linear(ENTITY_FEAT_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.dsl_proj = nn.Sequential(
            nn.Linear(CARD_DSL_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.token_type_emb = nn.Embedding(3, d)  # 0=Global, 1=Seat, 2=Entity

        # 2. Typed Graph Edge Message Passing
        self.edge_src_emb = nn.Embedding(NUM_EDGE_TYPES, d)
        self.edge_dst_emb = nn.Embedding(NUM_EDGE_TYPES, d)
        self.edge_proj = nn.Linear(d, d)
        self.edge_norm = nn.LayerNorm(d)

        # 3. Spatial Set-Transformer Trunk
        self.trunk_layers = nn.ModuleList(
            [
                PreNormTransformerLayer(d, cfg.n_heads, cfg.d_ff, dropout=cfg.dropout)
                for _ in range(cfg.n_layers)
            ]
        )
        self.trunk_norm = nn.LayerNorm(d)

        # 4. Recurrent Belief Cell & Latent Thinking Block
        self.recurrent_cell = GatedResidualUpdate(d)
        self.thinking_block = ThinkingStepBlock(
            d, cfg.n_heads, cfg.d_ff, dropout=cfg.dropout
        )
        self.h_norm = nn.LayerNorm(d)

        # 5. Action & Value Heads
        self.cand_proj = nn.Sequential(
            nn.Linear(CAND_FEAT_WIDTH, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, d),
        )
        self.cand_scorer = nn.Sequential(
            nn.Linear(d * 4, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, 1),
        )

        self.pair_blk_proj = nn.Linear(d, d)
        self.pair_atk_proj = nn.Linear(d, d)
        self.pair_scorer = nn.Sequential(
            nn.Linear(d * 3, d),
            nn.LayerNorm(d),
            nn.GELU(),
            nn.Linear(d, 1),
        )

        self.scalar_head = nn.Sequential(
            nn.Linear(d, d // 2),
            nn.GELU(),
            nn.Linear(d // 2, 1),
        )

        self.seat_value_head = nn.Sequential(
            nn.Linear(d * 2, d),
            nn.GELU(),
            nn.Linear(d, 1),
        )
        self.aux_head = nn.Linear(d, 4)

    def encode_board(
        self,
        entity_feat: torch.Tensor,
        entity_dsl: torch.Tensor,
        entity_mask: torch.Tensor,
        seat_feat: torch.Tensor,
        global_feat: torch.Tensor,
        edge_index: torch.Tensor,
        edge_type: torch.Tensor,
        edge_mask: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor]:
        """Encodes `[Global(1), Seats(8), Entities(E)]` through the relation-aware trunk."""
        B, E, _ = entity_feat.shape
        device = entity_feat.device
        d = self.cfg.d_model

        g_tok = self.global_proj(symlog(global_feat)).unsqueeze(1) + self.token_type_emb.weight[0]
        seat_ids = torch.arange(MAX_SEATS, device=device).unsqueeze(0).expand(B, MAX_SEATS)
        s_tok = (
            self.seat_proj(symlog(seat_feat))
            + self.seat_rel_emb(seat_ids)
            + self.token_type_emb.weight[1]
        )
        e_tok = (
            self.entity_proj(symlog(entity_feat))
            + self.dsl_proj(symlog(entity_dsl))
            + self.token_type_emb.weight[2]
        )

        tokens = torch.cat([g_tok, s_tok, e_tok], dim=1)  # [B, 9 + E, d]

        # Typed relation message passing via scatter_add
        valid_e = edge_mask.unsqueeze(-1).to(tokens.dtype)
        src_idx = edge_index[:, :, 0].clamp(min=0)
        dst_idx = edge_index[:, :, 1].clamp(min=0)
        rel_id = edge_type.clamp(0, NUM_EDGE_TYPES - 1)

        src_tok = torch.gather(tokens, 1, src_idx.unsqueeze(-1).expand(-1, -1, d))
        dst_tok = torch.gather(tokens, 1, dst_idx.unsqueeze(-1).expand(-1, -1, d))

        msg_to_dst = (src_tok + self.edge_src_emb(rel_id)) * valid_e
        msg_to_src = (dst_tok + self.edge_dst_emb(rel_id)) * valid_e

        agg = torch.zeros_like(tokens)
        agg.scatter_add_(1, dst_idx.unsqueeze(-1).expand(-1, -1, d), msg_to_dst)
        agg.scatter_add_(1, src_idx.unsqueeze(-1).expand(-1, -1, d), msg_to_src)
        tokens = tokens + self.edge_proj(self.edge_norm(agg))

        # Key padding mask (`True` = ignore/pad in PyTorch MultiheadAttention)
        g_pad = torch.zeros((B, 1), dtype=torch.bool, device=device)
        seat0_mask = torch.arange(MAX_SEATS, device=device).unsqueeze(0) == 0
        # Seat is present in the game if `seat_feat[:, :, SEAT_FEAT_PRESENT] > 0.5`; always keep seat 0 present
        s_present = (seat_feat[:, :, SEAT_FEAT_PRESENT] > 0.5) | seat0_mask
        s_pad = ~s_present
        e_pad = ~entity_mask
        pad_mask = torch.cat([g_pad, s_pad, e_pad], dim=1)  # [B, 9 + E]

        x = tokens
        for layer in self.trunk_layers:
            if self.cfg.activation_checkpointing and self.training and torch.is_grad_enabled():
                x = checkpoint(layer, x, pad_mask, use_reentrant=False)
            else:
                x = layer(x, src_key_padding_mask=pad_mask)
        board = self.trunk_norm(x)
        return board, pad_mask

    def forward_with_seat_logits(
        self,
        entity_feat: torch.Tensor,
        entity_dsl: torch.Tensor,
        entity_mask: torch.Tensor,
        seat_feat: torch.Tensor,
        global_feat: torch.Tensor,
        edge_index: torch.Tensor,
        edge_type: torch.Tensor,
        edge_mask: torch.Tensor,
        cand_feat: torch.Tensor,
        cand_entity_idx: torch.Tensor,
        cand_seat_idx: torch.Tensor,
        cand_mask: torch.Tensor,
        pair_index: torch.Tensor,
        pair_mask: torch.Tensor,
        cand_dsl: torch.Tensor,
        h_in: torch.Tensor,
        thinking_steps: torch.Tensor,
    ) -> tuple[
        torch.Tensor,
        torch.Tensor,
        torch.Tensor,
        torch.Tensor,
        torch.Tensor,
        torch.Tensor,
    ]:
        B, E, _ = entity_feat.shape
        d = self.cfg.d_model
        device = entity_feat.device

        board, pad_mask = self.encode_board(
            entity_feat,
            entity_dsl,
            entity_mask,
            seat_feat,
            global_feat,
            edge_index,
            edge_type,
            edge_mask,
        )

        if not torch.jit.is_tracing():
            # A rare name-card decision can offer thousands of candidates.
            # Trim this time step independently of the rest of its unroll.
            c_used = cand_mask.any(dim=0).nonzero()
            p_used = pair_mask.any(dim=0).nonzero()
            c_len = int(c_used[-1]) + 1 if c_used.numel() else 1
            p_len = int(p_used[-1]) + 1 if p_used.numel() else 1
            cand_dsl = cand_dsl[:, :c_len]
            cand_feat, cand_entity_idx = cand_feat[:, :c_len], cand_entity_idx[:, :c_len]
            cand_seat_idx, cand_mask = cand_seat_idx[:, :c_len], cand_mask[:, :c_len]
            pair_index, pair_mask = pair_index[:, :p_len], pair_mask[:, :p_len]

        # Persistent recurrent memory update on the 9 summary slots (1 Global + 8 Seats)
        # K=0 consumes one observation. K>=1 decides/thinks without changing
        # persistent memory. Live and replay both observe before deciding.
        h_observed = self.h_norm(self.recurrent_cell(board[:, :RECURRENT_SLOTS], h_in))
        is_observation = (thinking_steps == 0).view(B, 1, 1)
        h_out = torch.where(is_observation, h_observed, h_in)

        # Latent thinking scratchpad `z`: unroll all `max_thinking_steps` during ONNX tracing
        # so runtime `thinking_steps` (`0..=8`) genuinely executes `K` steps.
        # Current seat-specific summaries also seed the scratchpad: on a fresh
        # game H is all zero, but seat queries must already be distinguishable.
        z = h_out + board[:, :RECURRENT_SLOTS]
        max_k = (
            getattr(self, "export_thinking_steps", self.cfg.max_thinking_steps)
            if torch.jit.is_tracing()
            else max(0, min(int(thinking_steps.max().item()), self.cfg.max_thinking_steps))
        )
        for k in range(max_k):
            step_t = torch.full((B,), k, dtype=torch.long, device=device)
            z_next_k = self.thinking_block(z, board, pad_mask, step_t)
            active_k = (thinking_steps > k).view(B, 1, 1).to(z.dtype)
            z = z + active_k * (z_next_k - z)

        z_out = self.h_norm(z)
        g_ref = z_out[:, 0]  # [B, d]
        s_ref = z_out[:, 1:RECURRENT_SLOTS]  # [B, 8, d]
        e_ref = board[:, RECURRENT_SLOTS:]  # [B, E, d]

        # Candidate scoring (`cand_logits: [B, C]`)
        c_emb = self.cand_proj(symlog(cand_feat)) + self.dsl_proj(symlog(cand_dsl))

        ent_idx_clamped = cand_entity_idx.clamp(min=0)
        has_ent = cand_entity_idx >= 0
        c_ent = torch.gather(
            e_ref, 1, ent_idx_clamped.unsqueeze(-1).expand(-1, -1, d)
        ) * has_ent.unsqueeze(-1).to(e_ref.dtype)

        seat_idx_clamped = cand_seat_idx.clamp(0, MAX_SEATS - 1)
        has_seat = (cand_seat_idx >= 0) & (cand_seat_idx < MAX_SEATS)
        c_seat = torch.gather(
            s_ref, 1, seat_idx_clamped.unsqueeze(-1).expand(-1, -1, d)
        ) * has_seat.unsqueeze(-1).to(s_ref.dtype)

        g_exp = g_ref.unsqueeze(1).expand_as(c_emb)
        cand_in = torch.cat([c_emb, c_ent, c_seat, g_exp], dim=-1)
        cand_logits = self.cand_scorer(cand_in).squeeze(-1)
        cand_logits = cand_logits.masked_fill(~cand_mask, -1e9)

        # Blocker-Attacker Pair scoring (`pair_logits: [B, P]`)
        blk_idx = pair_index[:, :, 0].clamp(min=0)
        atk_idx = pair_index[:, :, 1].clamp(min=0)
        blk_tok = self.pair_blk_proj(
            torch.gather(e_ref, 1, blk_idx.unsqueeze(-1).expand(-1, -1, d))
        )
        atk_tok = self.pair_atk_proj(
            torch.gather(e_ref, 1, atk_idx.unsqueeze(-1).expand(-1, -1, d))
        )
        g_pair = g_ref.unsqueeze(1).expand_as(blk_tok)
        pair_in = torch.cat([blk_tok, atk_tok, blk_tok * atk_tok + g_pair], dim=-1)
        pair_logits = self.pair_scorer(pair_in).squeeze(-1)
        pair_logits = pair_logits.masked_fill(~pair_mask, -1e9)

        # Scalar value prediction (`scalar_pred: [B]`)
        scalar_pred = torch.sigmoid(self.scalar_head(g_ref)).squeeze(-1)

        # Multiplayer seat value distribution (`seat_win_logits: [B, 8]`, `value: [B]`)
        seat0_mask = torch.arange(MAX_SEATS, device=device).unsqueeze(0) == 0
        s_present_v = seat_feat[:, :, SEAT_FEAT_PRESENT] > 0.5
        s_lost_v = seat_feat[:, :, SEAT_FEAT_LOST] > 0.5
        s_active = s_present_v & (~s_lost_v)
        any_active = s_active.to(seat_feat.dtype).sum(dim=-1, keepdim=True) > 0.5
        s_active = s_active | ((~any_active) & seat0_mask)

        s_allied = (
            (seat_feat[:, :, SEAT_FEAT_SELF] > 0.5)
            | (seat_feat[:, :, SEAT_FEAT_TEAMMATE] > 0.5)
            | seat0_mask
        )

        sv_in = torch.cat([s_ref, g_ref.unsqueeze(1).expand(-1, MAX_SEATS, d)], dim=-1)
        seat_win_logits = self.seat_value_head(sv_in).squeeze(-1)
        seat_win_logits = seat_win_logits.masked_fill(~s_active, -1e9)
        seat_probs = F.softmax(seat_win_logits, dim=-1)
        value = (seat_probs * s_allied.to(seat_probs.dtype)).sum(dim=-1).clamp(0.0, 1.0)

        return h_out, cand_logits, pair_logits, scalar_pred, value, seat_win_logits

    def forward(
        self,
        entity_feat: torch.Tensor,
        entity_dsl: torch.Tensor,
        entity_mask: torch.Tensor,
        seat_feat: torch.Tensor,
        global_feat: torch.Tensor,
        edge_index: torch.Tensor,
        edge_type: torch.Tensor,
        edge_mask: torch.Tensor,
        cand_feat: torch.Tensor,
        cand_entity_idx: torch.Tensor,
        cand_seat_idx: torch.Tensor,
        cand_mask: torch.Tensor,
        pair_index: torch.Tensor,
        pair_mask: torch.Tensor,
        cand_dsl: torch.Tensor,
        h_in: torch.Tensor,
        thinking_steps: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        h_out, cand_logits, pair_logits, scalar_pred, value, _ = (
            self.forward_with_seat_logits(
                entity_feat,
                entity_dsl,
                entity_mask,
                seat_feat,
                global_feat,
                edge_index,
                edge_type,
                edge_mask,
                cand_feat,
                cand_entity_idx,
                cand_seat_idx,
                cand_mask,
                pair_index,
                pair_mask,
                cand_dsl,
                h_in,
                thinking_steps,
            )
        )
        return h_out, cand_logits, pair_logits, scalar_pred, value
