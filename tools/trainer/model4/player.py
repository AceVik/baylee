"""PlayerNet4 implementation, re-exported through the stable model4 package API."""

from __future__ import annotations

import torch
from torch import nn
import torch.nn.functional as F
from .activation_checkpoint import checkpoint_activation
from .config import *
from .candidate import score_candidate_hidden
from .layers import GatedResidualUpdate, DynamicMultiHeadAttention, PreNormTransformerLayer, ThinkingStepBlock

class PlayerNet4(nn.Module):
    """Playernet v4 single-step and unrolled trajectory network."""

    def __init__(self, cfg: Config4):
        super().__init__()
        self.cfg = cfg
        from .structure import RuleEncoder
        self.rules = RuleEncoder({"version": 1, "dsl_source_sha256": "", "tags": ["Unknown"],
                                  "nodes": [{"tag": 0, "bytes": [], "children": []}], "roots": {}}, cfg.structure_width)
        self.rules_proj = nn.Linear(cfg.structure_width, cfg.d_model, bias=False)
        d = cfg.d_model

        # 1. Token Encoders
        self.global_proj = nn.Sequential(
            nn.Linear(cfg.global_feat_width, d),
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
        self.cand_ctx_proj = nn.Linear(d * 2, d)
        nn.init.zeros_(self.cand_ctx_proj.weight)
        nn.init.zeros_(self.cand_ctx_proj.bias)
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

    def attach_structure(self, data):
        from .structure import RuleEncoder
        self.rules = RuleEncoder(data, self.cfg.structure_width)

    def prepare_structure(self, active_refs=None):
        if self.rules is not None:
            self.rules.prepare(active_refs)

    def clear_structure(self):
        if self.rules is not None:
            self.rules.clear()

    def rule_features(self, refs):
        if self.rules is None:
            return self.rules_proj.weight.new_zeros((*refs.shape[:-2], self.cfg.d_model))
        return self.rules_proj(self.rules(refs))

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
        entity_rules: torch.Tensor,
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
            + self.rule_features(entity_rules)
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

        tokens = self.relational_tokens(tokens, edge_index, edge_type, edge_mask, ~pad_mask, entity_feat)
        x = tokens
        # Execution-only memory/compute tradeoff: retaining the last N layers'
        # activations avoids recomputing them on backward. Default remains full
        # checkpointing; inference and checkpoint keys are unaffected.
        saved_layers = getattr(self, "saved_activation_layers", 0)
        for index, layer in enumerate(self.trunk_layers):
            if (self.cfg.activation_checkpointing and (self.training or getattr(self, "checkpoint_during_eval", False)) and torch.is_grad_enabled()
                    and index < len(self.trunk_layers) - saved_layers):
                x = checkpoint_activation(
                    layer, x, pad_mask,
                    offload_inputs=getattr(self, "offload_checkpoint_inputs", False),
                )
            else:
                x = layer(x, src_key_padding_mask=pad_mask)
        board = self.trunk_norm(x)
        return board, pad_mask

    def relational_tokens(self, tokens, edge_index, edge_type, edge_mask, present, entity_feat):
        """Identity extension point; archived v4 weights and operations are unchanged."""
        return tokens

    def observe_memory(self, board, h_in, thinking_steps):
        """Consume K=0 observations; decision scratchpads never alter memory."""
        h_observed = self.h_norm(self.recurrent_cell(board[:, :RECURRENT_SLOTS], h_in))
        return torch.where((thinking_steps == 0).view(board.shape[0], 1, 1), h_observed, h_in)

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
        entity_rules: torch.Tensor,
        cand_rules: torch.Tensor,
        h_in: torch.Tensor,
        thinking_steps: torch.Tensor,
        execution_hint: tuple[int, int, int] | None = None,
        encoded_board: tuple[torch.Tensor, torch.Tensor] | None = None,
        policy_heads: tuple[bool, bool, bool, bool] | None = None,
        observed_memory: torch.Tensor | None = None,
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

        board, pad_mask = encoded_board if encoded_board is not None else self.encode_board(
            entity_feat,
            entity_dsl,
            entity_mask,
            seat_feat,
            global_feat,
            edge_index,
            edge_type,
            edge_mask,
            entity_rules,
        )

        if not torch.jit.is_tracing():
            # A rare name-card decision can offer thousands of candidates.
            # Trim this time step independently of the rest of its unroll.
            if execution_hint is None:
                c_used = cand_mask.any(dim=0).nonzero()
                p_used = pair_mask.any(dim=0).nonzero()
                c_len = int(c_used[-1]) + 1 if c_used.numel() else 1
                p_len = int(p_used[-1]) + 1 if p_used.numel() else 1
            else:
                # Training knows these from CPU batch masks before the transfer.
                # Do not ask CUDA to return three shape/schedule scalars per frame.
                c_len, p_len, _ = execution_hint
            cand_dsl = cand_dsl[:, :c_len]
            cand_rules = cand_rules[:, :c_len]
            cand_feat, cand_entity_idx = cand_feat[:, :c_len], cand_entity_idx[:, :c_len]
            cand_seat_idx, cand_mask = cand_seat_idx[:, :c_len], cand_mask[:, :c_len]
            pair_index, pair_mask = pair_index[:, :p_len], pair_mask[:, :p_len]

        # Persistent recurrent memory update on the 9 summary slots (1 Global + 8 Seats)
        # K=0 consumes one observation. K>=1 decides/thinks without changing
        # persistent memory. Live and replay both observe before deciding.
        h_out = (self.observe_memory(board, h_in, thinking_steps)
                 if observed_memory is None else observed_memory)

        # Latent thinking scratchpad `z`: unroll all `max_thinking_steps` during ONNX tracing
        # so runtime `thinking_steps` (`0..=8`) genuinely executes `K` steps.
        # Current seat-specific summaries also seed the scratchpad: on a fresh
        # game H is all zero, but seat queries must already be distinguishable.
        z = h_out + board[:, :RECURRENT_SLOTS]
        max_k = (
            getattr(self, "export_thinking_steps", self.cfg.max_thinking_steps)
            if torch.jit.is_tracing()
            else max(0, min(execution_hint[2], self.cfg.max_thinking_steps)) if execution_hint is not None
            else max(0, min(int(thinking_steps.max().item()), self.cfg.max_thinking_steps))
        )
        projected_kv = None
        if not torch.jit.is_tracing() and getattr(self, "cache_thinking_kv", False) and max_k > 1:
            projected_kv = self.thinking_block.prepare_board(board)
        for k in range(max_k):
            step_t = torch.full((B,), k, dtype=torch.long, device=device)
            z_next_k = self.thinking_block(z, board, pad_mask, step_t, projected_kv)
            active_k = (thinking_steps > k).view(B, 1, 1).to(z.dtype)
            z = z + active_k * (z_next_k - z)

        z_out = self.h_norm(z)
        g_ref = z_out[:, 0]  # [B, d]
        s_ref = z_out[:, 1:RECURRENT_SLOTS]  # [B, 8, d]
        e_ref = board[:, RECURRENT_SLOTS:]  # [B, E, d]

        # Training may omit heads absent from the loss for this frame. The
        # default computes every output for inference and portable ONNX export.
        if policy_heads is None or policy_heads[0] or policy_heads[1]:
            # Candidate scoring (`cand_logits: [B, C]`)
            c_emb = self.cand_proj(symlog(cand_feat)) + self.dsl_proj(symlog(cand_dsl)) + self.rule_features(cand_rules)

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
            h_c = self.cand_scorer[2](self.cand_scorer[1](self.cand_scorer[0](cand_in)))
            cand_logits = score_candidate_hidden(self, h_c, cand_mask)
        else:
            cand_logits = torch.zeros_like(cand_mask, dtype=board.dtype).masked_fill(~cand_mask, -1e9)

        if policy_heads is None or policy_heads[2]:
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
        else:
            pair_logits = torch.zeros_like(pair_mask, dtype=board.dtype).masked_fill(~pair_mask, -1e9)

        # Scalar value prediction (`scalar_pred: [B]`)
        scalar_pred = (torch.sigmoid(self.scalar_head(g_ref)).squeeze(-1)
                       if policy_heads is None or policy_heads[3]
                       else torch.zeros(B, device=device, dtype=g_ref.dtype))

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
        seat_win_logits = self.value_readout(sv_in).squeeze(-1)
        seat_win_logits = seat_win_logits.masked_fill(~s_active, -1e9)
        seat_probs = F.softmax(seat_win_logits, dim=-1)
        value = (seat_probs * s_allied.to(seat_probs.dtype)).sum(dim=-1).clamp(0.0, 1.0)

        return h_out, cand_logits, pair_logits, scalar_pred, value, seat_win_logits

    def value_readout(self, features):
        """Training-only gradient routing; defaults preserve inference and state dict."""
        return self.seat_value_head(features.detach() if getattr(self, "detach_value_input", False) else features)

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
        entity_rules: torch.Tensor,
        cand_rules: torch.Tensor,
        h_in: torch.Tensor,
        thinking_steps: torch.Tensor,
        execution_hint: tuple[int, int, int] | None = None,
        encoded_board: tuple[torch.Tensor, torch.Tensor] | None = None,
        policy_heads: tuple[bool, bool, bool, bool] | None = None,
        observed_memory: torch.Tensor | None = None,
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
                entity_rules,
                cand_rules,
                h_in,
                thinking_steps,
                execution_hint=execution_hint,
                encoded_board=encoded_board,
                policy_heads=policy_heads,
                observed_memory=observed_memory,
            )
        )
        return h_out, cand_logits, pair_logits, scalar_pred, value


def upgrade_state_dict_to_v7(
    state_dict: dict[str, torch.Tensor], net: PlayerNet4
) -> dict[str, torch.Tensor]:
    """Zero-pad Encoder 6 input projections and initialize missing candidate context weights."""
    out = dict(state_dict)
    for key in ("global_proj.0.weight", "entity_proj.0.weight"):
        if key in out:
            old_w = out[key]
            target_w = net.state_dict()[key]
            if old_w.shape[0] == target_w.shape[0] and old_w.shape[1] < target_w.shape[1]:
                padded = torch.zeros_like(target_w, dtype=old_w.dtype, device=old_w.device)
                padded[:, : old_w.shape[1]] = old_w
                out[key] = padded
    for key in ("cand_ctx_proj.weight", "cand_ctx_proj.bias"):
        if key not in out and key in net.state_dict():
            out[key] = torch.zeros_like(net.state_dict()[key])
    return out
