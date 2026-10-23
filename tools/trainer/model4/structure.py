"""Ordered, bottom-up neural encoder of the complete typed rule DAG.

Every node and child is retained. Compilation interns equal subtrees exactly;
this module learns their representations. Training refreshes the table once per
optimizer update with gradients intact. Deployment freezes only the resulting
public catalog table, so ordinary decisions require small gathers, not parsing.
"""
from __future__ import annotations

import hashlib
import json
import math
import torch
from torch import nn


def structure_digest(data):
    return hashlib.sha256(json.dumps(data, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


class RuleEncoder(nn.Module):
    def __init__(self, data, width=128):
        super().__init__()
        if data["version"] != 1 or not data["nodes"] or data["tags"][data["nodes"][0]["tag"]] != "Unknown":
            raise ValueError("Invalid rule structure contract")
        self.data = data
        self.width = width
        self.digest = structure_digest(data)
        self.symbol = nn.Embedding(len(data["tags"]), width)
        self.byte = nn.Embedding(256, width)
        self.child = nn.Linear(width, width, bias=False)
        self.mix = nn.Sequential(nn.Linear(width * 2 + 1, width * 2), nn.GELU(), nn.Linear(width * 2, width), nn.LayerNorm(width))
        self.role = nn.Embedding(8, width, padding_idx=0)
        self.role_gate = nn.Embedding(8, width, padding_idx=0)
        self._table = None
        depths = []
        for i, node in enumerate(data["nodes"]):
            if not 0 <= node["tag"] < len(data["tags"]) or any(c < 0 or c >= i for c in node["children"]):
                raise ValueError("Rule DAG must be topological and acyclic")
            depths.append(1 + max((depths[c] for c in node["children"]), default=-1))
        order = sorted(range(len(depths)), key=lambda i: (depths[i], i))
        inverse = [0] * len(order)
        for i, original in enumerate(order): inverse[original] = i
        self.register_buffer("original_order", torch.tensor(inverse, dtype=torch.long))
        self.levels = max(depths) + 1
        for depth in range(self.levels):
            rows = [i for i in order if depths[i] == depth]
            tags, child_ids, parents, positions, byte_ids, byte_parents, byte_positions, counts = [], [], [], [], [], [], [], []
            for parent, row in enumerate(rows):
                node = data["nodes"][row]
                tags.append(node["tag"])
                counts.append(len(node["children"]))
                for position, child in enumerate(node["children"]):
                    child_ids.append(inverse[child]); parents.append(parent); positions.append(position)
                for position, byte in enumerate(node["bytes"]):
                    byte_ids.append(byte); byte_parents.append(parent); byte_positions.append(position)
            for name, values in dict(tags=tags, child_ids=child_ids, parents=parents, positions=positions,
                                     byte_ids=byte_ids, byte_parents=byte_parents, byte_positions=byte_positions,
                                     counts=counts).items():
                self.register_buffer(f"l{depth}_{name}", torch.tensor(values, dtype=torch.long))

    def position(self, positions):
        frequency = torch.exp(torch.arange(self.width, device=positions.device, dtype=torch.float32) * (-math.log(10000) / self.width))
        return torch.sin((positions.float()[:, None] + 1) * frequency[None, :])

    def encode_catalog(self):
        previous = self.symbol.weight.new_zeros((0, self.width))
        for depth in range(self.levels):
            get = lambda name: getattr(self, f"l{depth}_{name}")
            tags = get("tags")
            local = self.symbol(tags)
            if get("byte_ids").numel():
                values = self.byte(get("byte_ids")) * (1 + self.position(get("byte_positions")))
                local = local.index_add(0, get("byte_parents"), values.to(local.dtype))
            children = torch.zeros_like(local)
            if get("child_ids").numel():
                values = self.child(previous[get("child_ids")]) * (1 + self.position(get("positions")))
                children = children.index_add(0, get("parents"), values.to(children.dtype))
            counts = get("counts").to(local.dtype)[:, None]
            value = self.mix(torch.cat((local, children / counts.clamp_min(1).sqrt(), counts.log1p()), -1))
            previous = torch.cat((previous, value), 0)
        result = previous[self.original_order]
        # Zero means absence/unknown and never acquires a learnable identity.
        return result * (torch.arange(len(result), device=result.device) != 0)[:, None]

    def prepare(self):
        self._table = self.encode_catalog()

    def clear(self):
        self._table = None

    def forward(self, refs):
        table = self.encode_catalog() if self._table is None else self._table
        return self.read(table, refs)

    def read(self, table, refs):
        ids, roles = refs[..., 0], refs[..., 1]
        valid = ids != 0
        values = (table[ids] * torch.sigmoid(self.role_gate(roles)) + self.role(roles))
        return (values * valid[..., None]).sum(-2) / valid.sum(-1, keepdim=True).clamp_min(1).sqrt()

    def freeze(self):
        """Return the identical portable lookup for this exact public catalog."""
        with torch.no_grad():
            return FrozenRules(self.encode_catalog(), self.role, self.role_gate)


class FrozenRules(nn.Module):
    def __init__(self, table, role, role_gate):
        super().__init__()
        self.register_buffer("table", table)
        self.role = role
        self.role_gate = role_gate

    def forward(self, refs):
        ids, roles = refs[..., 0], refs[..., 1]
        valid = ids != 0
        value = self.table[ids] * torch.sigmoid(self.role_gate(roles)) + self.role(roles)
        return (value * valid[..., None]).sum(-2) / valid.sum(-1, keepdim=True).clamp_min(1).sqrt()
