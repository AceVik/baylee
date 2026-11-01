"""Recurrent, attention and thinking layers; portable ONNX attention fallback."""
import torch
from torch import nn
import torch.nn.functional as F

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
        projected_kv: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> torch.Tensor:
        B = q.shape[0]
        qh = self.q_proj(q).reshape(B, -1, self.n_heads, self.head_dim).transpose(1, 2)
        kh, vh = self.project_kv(k, v) if projected_kv is None else projected_kv

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

    def project_kv(self, k: torch.Tensor, v: torch.Tensor):
        """Project a fixed attention source, retaining its autograd history."""
        b = k.shape[0]
        kh = self.k_proj(k).reshape(b, -1, self.n_heads, self.head_dim).transpose(1, 2)
        vh = self.v_proj(v).reshape(b, -1, self.n_heads, self.head_dim).transpose(1, 2)
        return kh, vh


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

    def prepare_board(self, board_tokens: torch.Tensor):
        """K/V depend on this observation, not the scratchpad or thinking step.

        Keep the tensors local to one decision and attached to autograd. They
        must never be reused after a new observation or an optimizer update.
        """
        kv = self.norm_kv(board_tokens)
        return self.attn.project_kv(kv, kv)

    def forward(
        self,
        h: torch.Tensor,
        board_tokens: torch.Tensor,
        board_key_padding_mask: torch.Tensor,
        step_idx: torch.Tensor,
        projected_kv: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> torch.Tensor:
        q = self.norm_q(h + self.step_emb(step_idx).unsqueeze(1))
        if projected_kv is None:
            kv = self.norm_kv(board_tokens)
            attn_out = self.attn(q, kv, kv, key_padding_mask=board_key_padding_mask)
        else:
            attn_out = self.attn(q, board_tokens, board_tokens,
                                 key_padding_mask=board_key_padding_mask, projected_kv=projected_kv)
        h = h + attn_out
        h = h + self.ff(self.norm_ff(h))
        return h
