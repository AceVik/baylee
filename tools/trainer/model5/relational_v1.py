"""Archived v5 relational revision 1; checkpoint operations remain unchanged."""
import torch
from torch import nn
from model4.config import NUM_EDGE_TYPES


class RelationalProcessor(nn.Module):
    """Shared nonlinear directed messages; the outer residual starts exactly zero."""
    def __init__(self, cfg):
        super().__init__()
        w, d = cfg.relational_width, cfg.d_model
        self.steps = cfg.relational_steps
        self.input = nn.Linear(d, w)
        self.types = nn.Embedding(NUM_EDGE_TYPES * 2, w)
        self.message = nn.Linear(w * 3, w)
        self.gate = nn.Linear(w * 3, w)
        self.update = nn.Linear(w * 2 + 3, w)
        self.update_gate = nn.Linear(w * 2 + 3, w)
        self.output = nn.Linear(w, d, bias=False)
        nn.init.zeros_(self.output.weight)

    def forward(self, tokens, edges, types, mask, present, multiplicity):
        z = self.input(tokens)
        initial = z
        width = z.shape[-1]
        n = tokens.shape[1]
        src, dst = edges[..., 0], edges[..., 1]
        valid = mask & (src >= 0) & (dst >= 0) & (src < n) & (dst < n)
        src, dst = src.clamp(0, n - 1), dst.clamp(0, n - 1)
        valid = valid & present.gather(1, src) & present.gather(1, dst)
        src, dst = torch.cat((src, dst), 1), torch.cat((dst, src), 1)
        kinds = torch.cat((types.clamp(0, NUM_EDGE_TYPES - 1),
                           types.clamp(0, NUM_EDGE_TYPES - 1) + NUM_EDGE_TYPES), 1)
        valid = torch.cat((valid, valid), 1).unsqueeze(-1).to(z.dtype)
        si, di = src.unsqueeze(-1).expand(-1, -1, width), dst.unsqueeze(-1).expand(-1, -1, width)
        # Preserve original directed in/out degrees as separate count channels.
        half = mask.shape[1]
        indegree = torch.zeros_like(z[..., :1]).scatter_add(1, dst[:, :half].unsqueeze(-1), valid[:, :half])
        outdegree = torch.zeros_like(z[..., :1]).scatter_add(1, dst[:, half:].unsqueeze(-1), valid[:, half:])
        counts = torch.cat((torch.log1p(indegree), torch.log1p(outdegree), multiplicity.unsqueeze(-1)), -1)
        relation = self.types(kinds)
        for _ in range(self.steps):
            context = torch.cat((z.gather(1, si), z.gather(1, di), relation), -1)
            messages = torch.tanh(self.message(context)) * torch.sigmoid(self.gate(context)) * valid
            aggregate = torch.zeros_like(z).scatter_add(1, di, messages)
            update = torch.cat((z, aggregate, counts), -1)
            z = z + torch.sigmoid(self.update_gate(update)) * torch.tanh(self.update(update))
        return tokens + self.output(z - initial) * present.unsqueeze(-1).to(tokens.dtype)

