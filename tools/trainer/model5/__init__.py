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
