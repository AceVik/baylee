"""Ordered, bottom-up neural encoder of the complete typed rule DAG.

Every node and child is retained. Compilation interns equal subtrees exactly;
this module learns their representations. Training refreshes the table once per
optimizer update with gradients intact. Deployment freezes only the resulting
public catalog table, so ordinary decisions require small gathers, not parsing.
"""
from __future__ import annotations

import json
import math
import numpy as np
import torch
from torch import nn


def structure_digest(data):
    from trajectory_store4 import _native
    return _native(['digest'],json.dumps(data,allow_nan=False).encode()).decode().strip()


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
        self._remap = None
        self._gate_w = None
        self._role_w = None
        depths = []
        for i, node in enumerate(data["nodes"]):
            if not 0 <= node["tag"] < len(data["tags"]) or any(c < 0 or c >= i for c in node["children"]):
                raise ValueError("Rule DAG must be topological and acyclic")
            depths.append(1 + max((depths[c] for c in node["children"]), default=-1))
        self._node_depths = np.array(depths, dtype=np.int32)
        self._max_pos = max(
            max((len(n["children"]) for n in data["nodes"]), default=0),
            max((len(n["bytes"]) for n in data["nodes"]), default=0),
            1,
        )
        self._closure_cache: dict[int, np.ndarray] = {}
        self._seen_mask = np.zeros(len(data["nodes"]), dtype=np.bool_)
        self._compact_lookup = np.zeros(len(data["nodes"]), dtype=np.int64)
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

    def _get_closure(self, root_id: int) -> np.ndarray:
        res = self._closure_cache.get(root_id)
        if res is None:
            nodes = self.data["nodes"]
            vis = {root_id}
            stack = [root_id]
            while stack:
                u = stack.pop()
                for c in nodes[u]["children"]:
                    if c not in vis:
                        vis.add(c)
                        stack.append(c)
            res = np.array(sorted(vis), dtype=np.int64)
            self._closure_cache[root_id] = res
        return res

    def encode_catalog(self):
        device = self.symbol.weight.device
        pos_mult = 1.0 + self.position(torch.arange(self._max_pos, device=device))
        previous = self.symbol.weight.new_zeros((0, self.width))
        for depth in range(self.levels):
            get = lambda name: getattr(self, f"l{depth}_{name}")
            tags = get("tags")
            local = self.symbol(tags)
            if get("byte_ids").numel():
                values = self.byte(get("byte_ids")) * pos_mult[get("byte_positions")]
                local = local.index_add(0, get("byte_parents"), values.to(local.dtype))
            children = torch.zeros_like(local)
            if get("child_ids").numel():
                values = self.child(previous[get("child_ids")]) * pos_mult[get("positions")]
                children = children.index_add(0, get("parents"), values.to(children.dtype))
            counts = get("counts").to(local.dtype)[:, None]
            value = self.mix(torch.cat((local, children / counts.clamp_min(1).sqrt(), counts.log1p()), -1))
            previous = torch.cat((previous, value), 0)
        result = previous[self.original_order]
        # Zero means absence/unknown and never acquires a learnable identity.
        return result * (torch.arange(len(result), device=result.device) != 0)[:, None]

    def encode_active(self, active_refs):
        """Encode only the transitive DAG closure of rule IDs referenced in this batch."""
        device = self.symbol.weight.device
        nodes = self.data["nodes"]
        n_total = len(nodes)
        seen = self._seen_mask
        seen.fill(False)
        for ref_t in active_refs:
            if ref_t is not None:
                arr = ref_t[..., 0].detach().cpu().numpy() if ref_t.device.type != "cpu" else ref_t[..., 0].numpy()
                seen[arr.ravel()] = True
        roots = np.flatnonzero(seen[1:]) + 1
        seen[0] = True
        for r in roots:
            seen[self._get_closure(int(r))] = True
        active_nodes = np.flatnonzero(seen)
        depths_act = self._node_depths[active_nodes]
        order = np.argsort(depths_act, kind="stable")
        sorted_nodes = active_nodes[order]
        sorted_depths = depths_act[order]
        m_act = len(sorted_nodes)
        compact_lookup = self._compact_lookup
        compact_lookup[sorted_nodes] = np.arange(m_act, dtype=np.int64)

        tags = np.empty(m_act, dtype=np.int64)
        counts = np.empty(m_act, dtype=np.int64)
        byte_ids, byte_parents, byte_positions = [], [], []
        unique_depths, depth_starts, depth_counts = np.unique(sorted_depths, return_index=True, return_counts=True)

        level_specs = []
        for start, cnt in zip(depth_starts.tolist(), depth_counts.tolist()):
            c_ids, c_pars, c_pos = [], [], []
            for local_par in range(cnt):
                idx = start + local_par
                node = nodes[int(sorted_nodes[idx])]
                tags[idx] = node["tag"]
                ch = node["children"]
                counts[idx] = len(ch)
                for pos, c in enumerate(ch):
                    c_ids.append(int(compact_lookup[c]))
                    c_pars.append(local_par)
                    c_pos.append(pos)
                for pos, b in enumerate(node["bytes"]):
                    byte_ids.append(b)
                    byte_parents.append(idx)
                    byte_positions.append(pos)
            level_specs.append((start, cnt, c_ids, c_pars, c_pos))

        chunks = [
            sorted_nodes,
            tags,
            counts,
            np.array(byte_ids, dtype=np.int64),
            np.array(byte_parents, dtype=np.int64),
            np.array(byte_positions, dtype=np.int64),
        ]
        for _, _, c_ids, c_pars, c_pos in level_specs:
            if c_ids:
                chunks.append(np.array(c_ids, dtype=np.int64))
                chunks.append(np.array(c_pars, dtype=np.int64))
                chunks.append(np.array(c_pos, dtype=np.int64))
        flat_cpu = torch.from_numpy(np.concatenate(chunks))
        if device.type == "cuda":
            flat_gpu = flat_cpu.pin_memory().to(device, non_blocking=True)
        else:
            flat_gpu = flat_cpu.to(device)

        offset = 0
        def take(n: int) -> torch.Tensor:
            nonlocal offset
            out = flat_gpu[offset : offset + n]
            offset += n
            return out

        t_nodes = take(m_act)
        t_tags = take(m_act)
        t_counts = take(m_act)
        n_bytes = len(byte_ids)
        t_bids = take(n_bytes)
        t_bpar = take(n_bytes)
        t_bpos = take(n_bytes)

        pos_mult = 1.0 + self.position(torch.arange(self._max_pos, device=device))
        all_local = self.symbol(t_tags)
        if n_bytes:
            bval = self.byte(t_bids) * pos_mult[t_bpos]
            all_local = all_local.index_add(0, t_bpar, bval.to(all_local.dtype))

        counts_f = t_counts.to(all_local.dtype)[:, None]
        inv_sqrt_all = counts_f.clamp_min(1).sqrt()
        log1p_all = counts_f.log1p()

        previous = self.symbol.weight.new_zeros((0, self.width))
        for start, cnt, c_ids, _, _ in level_specs:
            local = all_local[start : start + cnt]
            children = torch.zeros_like(local)
            n_c = len(c_ids)
            if n_c:
                t_cids = take(n_c)
                t_par = take(n_c)
                t_pos = take(n_c)
                cval = self.child(previous[t_cids]) * pos_mult[t_pos]
                children = children.index_add(0, t_par, cval.to(children.dtype))
            value = self.mix(
                torch.cat(
                    (
                        local,
                        children / inv_sqrt_all[start : start + cnt],
                        log1p_all[start : start + cnt],
                    ),
                    -1,
                )
            )
            previous = torch.cat((previous, value), 0)

        previous = previous * (torch.arange(m_act, device=device) != 0)[:, None]
        if self._remap is None or self._remap.device != device or self._remap.shape[0] != n_total:
            self._remap = torch.zeros(n_total, dtype=torch.long, device=device)
        self._remap[t_nodes] = torch.arange(m_act, dtype=torch.long, device=device)
        return previous

    def prepare(self, active_refs=None):
        device = self.symbol.weight.device
        role_idx = torch.arange(8, device=device)
        self._gate_w = torch.sigmoid(self.role_gate(role_idx))
        self._role_w = self.role(role_idx)
        if active_refs is not None and len(self.data["nodes"]) > 2048:
            self._table = self.encode_active(active_refs)
        else:
            self._remap = None
            self._table = self.encode_catalog()

    def clear(self):
        self._table = None
        self._remap = None
        self._gate_w = None
        self._role_w = None

    def forward(self, refs):
        table = self.encode_catalog() if self._table is None else self._table
        return self.read(table, refs)

    def read(self, table, refs):
        ids, roles = refs[..., 0], refs[..., 1]
        if self._remap is not None and table.shape[0] != len(self.original_order):
            ids = self._remap[ids]
        valid = ids != 0
        if self._gate_w is not None and self._role_w is not None and not torch.jit.is_tracing():
            values = table[ids] * self._gate_w[roles] + self._role_w[roles]
        else:
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
