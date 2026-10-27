"""Numerically bounded, count-explicit v5 relational processing."""
import torch
from torch import nn
from model4.config import NUM_EDGE_TYPES


class RelationalProcessorV2(nn.Module):
    def __init__(self, cfg):
        super().__init__()
        w, d = cfg.relational_width, cfg.d_model
        self.steps = cfg.relational_steps
        self.norm = nn.LayerNorm(d)
        self.input = nn.Linear(d, w)
        self.types = nn.Embedding(NUM_EDGE_TYPES * 2, w)
        self.message = nn.Linear(w * 3, w)
        self.gate = nn.Linear(w * 3, w)
        self.update = nn.Linear(w * 2 + NUM_EDGE_TYPES * 4 + 1, w)
        self.update_gate = nn.Linear(w * 2 + NUM_EDGE_TYPES * 4 + 1, w)
        self.output = nn.Linear(w, d, bias=False)
        nn.init.zeros_(self.output.weight)

    def edge_statistics(self, edges, types, mask, present, multiplicity):
        """Count incoming source groups/cards separately for each directed type."""
        n = present.shape[1]
        src, dst = edges[..., 0], edges[..., 1]
        valid = mask & (src >= 0) & (dst >= 0) & (src < n) & (dst < n)
        src, dst = src.clamp(0, n - 1), dst.clamp(0, n - 1)
        valid = valid & present.gather(1, src) & present.gather(1, dst)
        src, dst = torch.cat((src, dst), 1), torch.cat((dst, src), 1)
        kinds = torch.cat((types.clamp(0, NUM_EDGE_TYPES - 1),
                           types.clamp(0, NUM_EDGE_TYPES - 1) + NUM_EDGE_TYPES), 1)
        valid = torch.cat((valid, valid), 1).float()
        weights = multiplicity.float().clamp(min=1).gather(1, src) * valid
        bins = dst * (NUM_EDGE_TYPES * 2) + kinds
        zero = torch.zeros_like(present, dtype=torch.float32).unsqueeze(-1).expand(-1, -1, NUM_EDGE_TYPES * 2).reshape(present.shape[0], -1)
        groups = zero.scatter_add(1, bins, valid).reshape(present.shape[0], n, NUM_EDGE_TYPES * 2)
        cards = zero.scatter_add(1, bins, weights).reshape(present.shape[0], n, NUM_EDGE_TYPES * 2)
        return src, dst, kinds, weights, groups, cards

    def forward(self, tokens, edges, types, mask, present, multiplicity):
        z = self.input(self.norm(tokens))
        src, dst, kinds, weights, groups, cards = self.edge_statistics(edges, types, mask, present, multiplicity)
        counts = torch.cat((torch.log1p(groups), torch.log1p(cards),
                            torch.log1p(multiplicity.float().clamp(min=0)).unsqueeze(-1)), -1)
        width = z.shape[-1]
        si, di = src.unsqueeze(-1).expand(-1, -1, width), dst.unsqueeze(-1).expand(-1, -1, width)
        denominator = cards.sum(-1, keepdim=True).clamp(min=1)
        relation = self.types(kinds)
        delta = torch.zeros_like(z, dtype=torch.float32)
        for _ in range(self.steps):
            context = torch.cat((z.gather(1, si), z.gather(1, di), relation), -1)
            messages = torch.tanh(self.message(context)) * torch.sigmoid(self.gate(context))
            weighted = messages.float() * weights.unsqueeze(-1)
            aggregate = torch.zeros_like(z, dtype=torch.float32).scatter_add(1, di, weighted) / denominator
            update = torch.cat((z.float(), aggregate, counts), -1)
            increment = torch.sigmoid(self.update_gate(update)) * torch.tanh(self.update(update))
            delta = delta + increment.float()
            z = z + increment
        return tokens + self.output(delta) * present.unsqueeze(-1).to(tokens.dtype)
