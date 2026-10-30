"""Candidate revision 3: each offered answer attends to the board it acts on."""
from torch import nn
from model4.layers import DynamicMultiHeadAttention


class CandidateDecoderLayer(nn.Module):
    """Pre-norm cross-attention from candidates into `[summary slots, entities]`.

    Candidates never attend to one another, so a large menu can still be
    scored in independent slices. Both residual outputs start at zero: a
    transferred network computes its previous logits exactly, and the output
    projections receive gradients from the first update on.
    """

    def __init__(self, d_model: int, n_heads: int, d_ff: int):
        super().__init__()
        self.norm_q = nn.LayerNorm(d_model)
        self.norm_kv = nn.LayerNorm(d_model)
        self.attn = DynamicMultiHeadAttention(d_model, n_heads)
        self.norm_ff = nn.LayerNorm(d_model)
        self.ff = nn.Sequential(nn.Linear(d_model, d_ff), nn.GELU(), nn.Linear(d_ff, d_model))
        for linear in (self.attn.out_proj, self.ff[2]):
            nn.init.zeros_(linear.weight)
            nn.init.zeros_(linear.bias)

    def forward(self, hidden, tokens, key_padding_mask):
        context = self.norm_kv(tokens)
        hidden = hidden + self.attn(self.norm_q(hidden), context, context, key_padding_mask=key_padding_mask)
        return hidden + self.ff(self.norm_ff(hidden))
