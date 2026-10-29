"""Playernet v4 trajectory dataset loader, stateful TBPTT batch sampler, and
dynamic-padded minibatch collator.

Reads datasets produced by `cargo run --release -p baylee-train --bin convert4`:
- `meta.json`: schema constants, card vocabulary (`cards`), coverage matrix.
- `card_dsl.bin`: `[len(cards), 160]` float32 front-face DSL summary table.
- `card_dsl_back.bin`: `[len(cards), 160]` float32 back-face DSL summary table.
- `token_dsl.bin`: `[token_count + 1, 160]` float32 token DSL summary table.
- `train/steps-*.jsonl.gz` and `val/steps-*.jsonl.gz`: `Step4` records.

Sequence and encoding contracts:
- P0-2: Front/back face + token DSL parity with Rust `entity_dsl_summary`;
  versioned caps with dynamic per-batch trimming;
  pair target/free mask remapping; no silent relabeling of out-of-range targets.
- P0-3: Exports `cand_free_mask`, `pair_free_mask`, `temperature`, `source_kind`,
  and `has_logprob` for exact conditional log-probabilities.
- P0-4: `StatefulBatchSampler4` streams consecutive segments of each `(game_id, seat)`
  trajectory in persistent batch lanes (`is_new_traj`) for true TBPTT.
- P0-5: Exports `is_terminal` per step for reverse-time GAE(lambda).
- P1-9: Exports `aux_target` (`[4]`) and `loss_weight` per step.
"""

from __future__ import annotations

import gzip
import json
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch.utils.data import Dataset

from sampler4 import StatefulBatchSampler4

import model4 as M
from trajectory_store4 import BST4_HDR, BST4_MAGIC, TrajectoryStore4
from structure_data4 import StructureData
import orjson

SOURCE_KIND_MAP = {
    "Net": 0,
    "House": 1,
    "Forced": 2,
    "Search": 3,
    "External": 4,
}


def padded_stack(arrays: list[np.ndarray]) -> np.ndarray:
    shape = tuple(max(a.shape[i] for a in arrays) for i in range(arrays[0].ndim))
    out = np.zeros((len(arrays), *shape), dtype=arrays[0].dtype)
    for i, arr in enumerate(arrays):
        out[(i, *(slice(0, n) for n in arr.shape))] = arr
    return out


def parse_source_kind(raw: Any) -> int:
    if isinstance(raw, int):
        return raw
    if isinstance(raw, str):
        return SOURCE_KIND_MAP.get(raw, 1)
    return 1


class TrajectoryDataset4(Dataset):
    """Dataset of unrolled `(game_id, seat)` trajectory segments of length `unroll_len`."""

    def __init__(
        self,
        data_dir: Path,
        split: str = "train",
        unroll_len: int = 16,
        max_entities: int = M.MAX_ENTITIES,
        max_edges: int = M.MAX_EDGES,
        max_cands: int = M.MAX_CANDIDATES,
        max_pairs: int = M.MAX_PAIRS,
        decisions_only: bool = False,
        max_shards: int = 0,
        encoder_version: int = M.ENCODER_VERSION,
    ):
        super().__init__()
        self.data_dir = Path(data_dir).expanduser()
        self.meta = json.loads((self.data_dir / "meta.json").read_text())
        from encoder_contract import validate_encoder
        validate_encoder(encoder_version, global_feat_width=self.meta.get('global_feat_width',80))
        if self.meta.get("encoder_version") != encoder_version:
            raise ValueError(f"Expected encoder_version={encoder_version}, got {self.meta.get('encoder_version')}")
        self.encoder_version = encoder_version
        if self.meta.get("trajectory_version") != M.TRAJECTORY_VERSION:
            raise ValueError("Reconvert records with the current complete-observation converter")
        self.structure = StructureData(self.data_dir / "card_structure.json", self.meta["structure_sha256"])
        self.unroll_len = unroll_len
        self.max_entities = max_entities
        self.max_edges = max_edges
        self.max_cands = max_cands
        self.max_pairs = max_pairs

        # Map card_id -> row index in card_dsl.bin / card_dsl_back.bin
        cards = self.meta["cards"]
        self.card_to_row: dict[int, int] = {int(cid): i for i, cid in enumerate(cards)}
        max_cid = max((k for k in self.card_to_row if k != M.UNKNOWN_CARD), default=0)
        self.cid_to_row_arr = np.full(max_cid + 1, -1, dtype=np.int32)
        for cid, row_idx in self.card_to_row.items():
            if 0 <= cid <= max_cid and cid != M.UNKNOWN_CARD:
                self.cid_to_row_arr[cid] = row_idx
        dsl_raw = np.fromfile(self.data_dir / "card_dsl.bin", dtype=np.float32)
        self.card_dsl_table = dsl_raw.reshape(len(cards), M.CARD_DSL_WIDTH)

        back_path = self.data_dir / "card_dsl_back.bin"
        if back_path.exists():
            back_raw = np.fromfile(back_path, dtype=np.float32)
            self.card_dsl_back_table = back_raw.reshape(len(cards), M.CARD_DSL_WIDTH)
        else:
            self.card_dsl_back_table = self.card_dsl_table

        tok_path = self.data_dir / "token_dsl.bin"
        if tok_path.exists():
            tok_raw = np.fromfile(tok_path, dtype=np.float32)
            self.token_dsl_table: np.ndarray | None = tok_raw.reshape(-1, M.CARD_DSL_WIDTH)
        else:
            self.token_dsl_table = None

        shard_names = self.meta["train_shards" if split == "train" else "val_shards"]
        if max_shards > 0:
            shard_names = shard_names[:max_shards]
        if decisions_only:
            raise ValueError("Recurrent training requires the complete observation stream")
        self.store = TrajectoryStore4(self.data_dir, split, shard_names)
        self.fingerprint = self.store.fingerprint
        index = self.store.index
        order = np.lexsort((index["cursor"], index["seat"], index["game"]))
        self.segments: list[np.ndarray] = []
        self.segment_is_new_traj: list[bool] = []
        self.trajectories: list[list[int]] = []
        if len(order):
            games, seats = index["game"][order], index["seat"][order]
            cuts = np.flatnonzero((games[1:] != games[:-1]) | (seats[1:] != seats[:-1])) + 1
            for rows in np.split(order, cuts):
                cursors = index["cursor"][rows]
                if not np.array_equal(cursors, np.arange(len(rows))):
                    raise ValueError("Incomplete or duplicated trajectory cursors")
                terminals = (index["flags"][rows] & 2) != 0
                if not terminals[-1] or terminals[:-1].any():
                    raise ValueError("A trajectory must have exactly one terminal, at its true end")
                traj_indices = []
                for start in range(0, len(rows), unroll_len):
                    traj_indices.append(len(self.segments))
                    self.segments.append(rows[start:start + unroll_len])
                    self.segment_is_new_traj.append(start == 0)
                self.trajectories.append(traj_indices)

    def __len__(self) -> int:
        return len(self.segments)

    def lookup_entity_dsl(self, cid: int, fid: int, tslot: int) -> np.ndarray:
        if cid != M.UNKNOWN_CARD:
            row_idx = self.card_to_row[cid]  # Missing registry entries are corruption.
            if fid == 1:
                return self.card_dsl_back_table[row_idx]
            return self.card_dsl_table[row_idx]
        if (
            tslot > 0
            and self.token_dsl_table is not None
            and tslot < len(self.token_dsl_table)
        ):
            return self.token_dsl_table[tslot]
        return np.zeros(M.CARD_DSL_WIDTH, dtype=np.float32)

    def _fill_entity_dsl(
        self,
        ent_dsl: np.ndarray,
        cids: np.ndarray,
        fids: np.ndarray,
        tids: np.ndarray,
    ) -> None:
        n = len(cids)
        if n == 0:
            return
        known = cids != M.UNKNOWN_CARD
        if known.any():
            known_cids = cids[known]
            if (known_cids < 0).any() or (known_cids >= len(self.cid_to_row_arr)).any():
                for cid in known_cids.tolist():
                    _ = self.card_to_row[int(cid)]
            rows = self.cid_to_row_arr[known_cids]
            if (rows < 0).any():
                for cid in known_cids.tolist():
                    _ = self.card_to_row[int(cid)]
            known_idx = np.flatnonzero(known)
            back_mask = fids[known] == 1
            if back_mask.any():
                ent_dsl[known_idx[back_mask]] = self.card_dsl_back_table[rows[back_mask]]
            front_mask = ~back_mask
            if front_mask.any():
                ent_dsl[known_idx[front_mask]] = self.card_dsl_table[rows[front_mask]]
        if self.token_dsl_table is not None:
            tok_mask = (~known) & (tids > 0) & (tids < len(self.token_dsl_table))
            if tok_mask.any():
                ent_dsl[tok_mask] = self.token_dsl_table[tids[tok_mask]]

    def _fill_cand_dsl(
        self,
        cand_dsl: np.ndarray,
        ent_dsl: np.ndarray,
        n_ent: int,
        ce: np.ndarray,
        ccid: np.ndarray,
        cface: np.ndarray,
    ) -> None:
        n = len(ce)
        if n == 0:
            return
        named_face = cface != 255
        from_ent = (~named_face) & (ce >= 0) & (ce < n_ent)
        from_card = (~named_face) & (~from_ent) & (ccid != M.UNKNOWN_CARD)
        if from_ent.any():
            cand_dsl[from_ent] = ent_dsl[ce[from_ent]]
        lookup_mask = (named_face & (ccid != M.UNKNOWN_CARD)) | from_card
        if lookup_mask.any():
            l_cids = ccid[lookup_mask]
            if (l_cids < 0).any() or (l_cids >= len(self.cid_to_row_arr)).any():
                for cid in l_cids.tolist():
                    _ = self.card_to_row[int(cid)]
            rows = self.cid_to_row_arr[l_cids]
            if (rows < 0).any():
                for cid in l_cids.tolist():
                    _ = self.card_to_row[int(cid)]
            l_idx = np.flatnonzero(lookup_mask)
            back = named_face[lookup_mask] & (cface[lookup_mask] == 1)
            if back.any():
                cand_dsl[l_idx[back]] = self.card_dsl_back_table[rows[back]]
            front = ~back
            if front.any():
                cand_dsl[l_idx[front]] = self.card_dsl_table[rows[front]]

    def _encode_bst4(self, buf: bytes) -> dict[str, np.ndarray]:
        (
            _magic, _ver, _game_id, _seat, _step_cursor, _turn, flags, head,
            _pending_kind, src_kind, _pad0, thinking_steps, raw_single_idx,
            loss_weight, scalar_norm, outcome, aux0, aux1, aux2, aux3,
            behavior_logprob, temp_val, n_ent_raw, ent_width, ent_rule_cols,
            seat_width, global_width, n_edg_raw, n_cand_raw, cand_width,
            cand_rule_cols, n_pair_raw, n_mm, n_fm,
        ) = BST4_HDR.unpack_from(buf, 0)
        offset = BST4_HDR.size

        E_max = max(1, min(n_ent_raw, self.max_entities))
        M_max = max(1, min(n_edg_raw, self.max_edges))
        C_max = max(1, min(n_cand_raw, self.max_cands))
        P_max = max(1, min(n_pair_raw, self.max_pairs))
        if n_ent_raw > E_max or n_cand_raw > C_max:
            raise ValueError("Semantic references would be truncated")

        raw_ef = np.frombuffer(buf, dtype="<f4", count=n_ent_raw * ent_width, offset=offset).reshape(n_ent_raw, ent_width)
        offset += n_ent_raw * ent_width * 4
        raw_cids = np.frombuffer(buf, dtype="<u4", count=n_ent_raw, offset=offset)
        offset += n_ent_raw * 4
        raw_fids = np.frombuffer(buf, dtype="<u1", count=n_ent_raw, offset=offset)
        offset += n_ent_raw
        raw_tids = np.frombuffer(buf, dtype="<u2", count=n_ent_raw, offset=offset)
        offset += n_ent_raw * 2
        raw_erules = np.frombuffer(buf, dtype="<u4", count=n_ent_raw * ent_rule_cols * 2, offset=offset).reshape(n_ent_raw, ent_rule_cols, 2)
        offset += n_ent_raw * ent_rule_cols * 8

        seat_feat = np.frombuffer(buf, dtype="<f4", count=8 * seat_width, offset=offset).reshape(8, seat_width).copy()
        offset += 8 * seat_width * 4
        global_feat = np.frombuffer(buf, dtype="<f4", count=global_width, offset=offset).copy()
        offset += global_width * 4

        raw_edges = np.frombuffer(buf, dtype="<i4", count=n_edg_raw * 3, offset=offset).reshape(n_edg_raw, 3)
        offset += n_edg_raw * 12

        raw_cf = np.frombuffer(buf, dtype="<f4", count=n_cand_raw * cand_width, offset=offset).reshape(n_cand_raw, cand_width)
        offset += n_cand_raw * cand_width * 4
        raw_ce = np.frombuffer(buf, dtype="<i2", count=n_cand_raw, offset=offset)
        offset += n_cand_raw * 2
        raw_cs = np.frombuffer(buf, dtype="<i1", count=n_cand_raw, offset=offset)
        offset += n_cand_raw
        raw_ccid = np.frombuffer(buf, dtype="<u4", count=n_cand_raw, offset=offset)
        offset += n_cand_raw * 4
        raw_cface = np.frombuffer(buf, dtype="<u1", count=n_cand_raw, offset=offset)
        offset += n_cand_raw
        raw_crules = np.frombuffer(buf, dtype="<u4", count=n_cand_raw * cand_rule_cols * 2, offset=offset).reshape(n_cand_raw, cand_rule_cols, 2)
        offset += n_cand_raw * cand_rule_cols * 8

        raw_pairs = np.frombuffer(buf, dtype="<i2", count=n_pair_raw * 2, offset=offset).reshape(n_pair_raw, 2)
        offset += n_pair_raw * 4
        raw_mm = np.frombuffer(buf, dtype="<u1", count=n_mm, offset=offset)
        offset += n_mm
        raw_fm = np.frombuffer(buf, dtype="<u1", count=n_fm, offset=offset)

        ent_feat = np.zeros((E_max, M.ENTITY_FEAT_WIDTH), dtype=np.float32)
        ent_dsl = np.zeros((E_max, M.CARD_DSL_WIDTH), dtype=np.float32)
        ent_mask = np.zeros((E_max,), dtype=bool)
        entity_rules = np.zeros((E_max, ent_rule_cols, 2), dtype=np.int64)
        n_ent = min(n_ent_raw, E_max)
        if n_ent > 0:
            ent_feat[:n_ent] = raw_ef[:n_ent]
            self._fill_entity_dsl(ent_dsl[:n_ent], raw_cids[:n_ent], raw_fids[:n_ent], raw_tids[:n_ent])
            entity_rules[:n_ent] = raw_erules[:n_ent]
            ent_mask[:n_ent] = True

        edge_index = np.zeros((M_max, 2), dtype=np.int64)
        edge_type = np.zeros((M_max,), dtype=np.int64)
        edge_mask = np.zeros((M_max,), dtype=bool)
        if n_edg_raw > 0:
            max_tok = M.RECURRENT_SLOTS + E_max
            valid_e = (raw_edges[:, 0] >= 0) & (raw_edges[:, 0] < max_tok) & (raw_edges[:, 1] >= 0) & (raw_edges[:, 1] < max_tok)
            kept = raw_edges[valid_e][:M_max]
            n_edg = len(kept)
            if n_edg > 0:
                edge_index[:n_edg] = kept[:, :2]
                edge_type[:n_edg] = kept[:, 2]
                edge_mask[:n_edg] = True

        cand_feat = np.zeros((C_max, M.CAND_FEAT_WIDTH), dtype=np.float32)
        cand_dsl = np.zeros((C_max, M.CARD_DSL_WIDTH), dtype=np.float32)
        cand_ent = np.full((C_max,), -1, dtype=np.int64)
        cand_seat = np.full((C_max,), -1, dtype=np.int64)
        cand_mask = np.zeros((C_max,), dtype=bool)
        cand_rules = np.zeros((C_max, cand_rule_cols, 2), dtype=np.int64)
        n_cand = min(n_cand_raw, C_max)
        if n_cand > 0:
            cand_feat[:n_cand] = raw_cf[:n_cand]
            ce = raw_ce[:n_cand].astype(np.int64)
            cand_ent[:n_cand] = np.where(ce < E_max, ce, -1)
            cand_seat[:n_cand] = raw_cs[:n_cand]
            self._fill_cand_dsl(cand_dsl[:n_cand], ent_dsl, n_ent, ce, raw_ccid[:n_cand], raw_cface[:n_cand])
            cand_rules[:n_cand] = raw_crules[:n_cand]
            cand_mask[:n_cand] = True
        else:
            cand_mask[0] = True

        is_decision = bool(flags & 1)
        is_terminal = bool(flags & 2)
        has_lp = bool(flags & 4)
        if is_decision and head == 0 and not 0 <= raw_single_idx < n_cand:
            raise ValueError("Decision target is outside the represented candidate set")
        single_idx = min(max(raw_single_idx, 0), max(n_cand - 1, 0))

        cand_target_mask = np.zeros((C_max,), dtype=np.float32)
        cand_free_mask = np.zeros((C_max,), dtype=np.float32)
        if head == 1:
            k = min(n_mm, n_cand)
            if k > 0:
                cand_target_mask[:k] = raw_mm[:k]
                if n_fm >= k:
                    cand_free_mask[:k] = raw_fm[:k]
                else:
                    cand_free_mask[:k] = 1.0

        pair_index = np.zeros((P_max, 2), dtype=np.int64)
        pair_mask = np.zeros((P_max,), dtype=bool)
        pair_target_mask = np.zeros((P_max,), dtype=np.float32)
        pair_free_mask = np.zeros((P_max,), dtype=np.float32)
        n_pair = 0
        if n_pair_raw > 0:
            valid_p = (raw_pairs[:, 0] >= 0) & (raw_pairs[:, 0] < E_max) & (raw_pairs[:, 1] >= 0) & (raw_pairs[:, 1] < E_max)
            orig_idx = np.flatnonzero(valid_p)[:P_max]
            n_pair = len(orig_idx)
            if n_pair > 0:
                pair_index[:n_pair] = raw_pairs[orig_idx]
                pair_mask[:n_pair] = True
                if head == 2:
                    in_mm = orig_idx < n_mm
                    if in_mm.any():
                        pair_target_mask[:n_pair][in_mm] = raw_mm[orig_idx[in_mm]]
                    in_fm = orig_idx < n_fm
                    pair_free_mask[:n_pair] = 1.0
                    if in_fm.any():
                        pair_free_mask[:n_pair][in_fm] = raw_fm[orig_idx[in_fm]]
        if n_pair == 0:
            pair_mask[0] = True

        return {
            "entity_feat": ent_feat,
            "entity_dsl": ent_dsl,
            "entity_rules": entity_rules,
            "cand_rules": cand_rules,
            "entity_mask": ent_mask,
            "seat_feat": seat_feat,
            "global_feat": global_feat,
            "edge_index": edge_index,
            "edge_type": edge_type,
            "edge_mask": edge_mask,
            "cand_feat": cand_feat,
            "cand_dsl": cand_dsl,
            "cand_entity_idx": cand_ent,
            "cand_seat_idx": cand_seat,
            "cand_mask": cand_mask,
            "pair_index": pair_index,
            "pair_mask": pair_mask,
            "is_decision": np.array(is_decision, dtype=bool),
            "loss_weight": np.array(loss_weight, dtype=np.float32),
            "is_terminal": np.array(is_terminal, dtype=bool),
            "head": np.array(head, dtype=np.int64),
            "single_idx": np.array(single_idx, dtype=np.int64),
            "cand_target_mask": cand_target_mask,
            "cand_free_mask": cand_free_mask,
            "pair_target_mask": pair_target_mask,
            "pair_free_mask": pair_free_mask,
            "scalar_target": np.array(scalar_norm, dtype=np.float32),
            "outcome": np.array(outcome, dtype=np.float32),
            "aux_target": np.array([aux0, aux1, aux2, aux3], dtype=np.float32),
            "behavior_logprob": np.array(behavior_logprob, dtype=np.float32),
            "has_logprob": np.array(has_lp, dtype=bool),
            "source_kind": np.array(src_kind, dtype=np.int64),
            "temperature": np.array(temp_val, dtype=np.float32),
            "thinking_steps": np.array(thinking_steps, dtype=np.int64),
        }

    def encode_single_step(self, step_or_row: dict[str, Any] | int) -> dict[str, np.ndarray]:
        if not isinstance(step_or_row, dict):
            buf = self.store.read_raw(int(step_or_row))
            if buf[:4] == BST4_MAGIC:
                return self._encode_bst4(buf)
            st = orjson.loads(buf)
        elif "_bst4" in step_or_row:
            return self._encode_bst4(step_or_row["_bst4"])
        else:
            st = step_or_row
        state = st["state"]
        # Allocate actual lengths, then pad only this minibatch. In particular,
        # naming any card/subtype must not require a 65k-row padded tensor at
        # every ordinary priority or observation frame.
        E_max = max(1, min(len(state["entity_feat"]), self.max_entities))
        M_max = max(1, min(len(state["edges"]), self.max_edges))
        C_max = max(1, min(len(st["cand_feat"]), self.max_cands))
        P_max = max(1, min(len(st["pairs"]), self.max_pairs))

        ent_feat = np.zeros((E_max, M.ENTITY_FEAT_WIDTH), dtype=np.float32)
        ent_dsl = np.zeros((E_max, M.CARD_DSL_WIDTH), dtype=np.float32)
        ent_mask = np.zeros((E_max,), dtype=bool)

        raw_ef = state["entity_feat"]
        raw_cids = state["entity_card_ids"]
        raw_fids = state.get("entity_face_ids")
        raw_tids = state.get("entity_token_ids")
        n_ent = min(len(raw_ef), E_max)
        if n_ent > 0:
            ent_feat[:n_ent] = np.asarray(raw_ef[:n_ent], dtype=np.float32)
            cids_arr = np.asarray(raw_cids[:n_ent], dtype=np.int64)
            fids_arr = (np.asarray(raw_fids[:n_ent], dtype=np.uint8)
                        if raw_fids is not None and len(raw_fids) >= n_ent
                        else np.zeros(n_ent, dtype=np.uint8))
            if raw_fids is not None and 0 < len(raw_fids) < n_ent:
                fids_arr[:len(raw_fids)] = np.asarray(raw_fids, dtype=np.uint8)
            tids_arr = (np.asarray(raw_tids[:n_ent], dtype=np.uint16)
                        if raw_tids is not None and len(raw_tids) >= n_ent
                        else np.zeros(n_ent, dtype=np.uint16))
            if raw_tids is not None and 0 < len(raw_tids) < n_ent:
                tids_arr[:len(raw_tids)] = np.asarray(raw_tids, dtype=np.uint16)
            self._fill_entity_dsl(ent_dsl[:n_ent], cids_arr, fids_arr, tids_arr)
            ent_mask[:n_ent] = True

        seat_feat = np.asarray(state["seat_feat"], dtype=np.float32)
        global_feat = np.asarray(state["global_feat"], dtype=np.float32)

        edge_index = np.zeros((M_max, 2), dtype=np.int64)
        edge_type = np.zeros((M_max,), dtype=np.int64)
        edge_mask = np.zeros((M_max,), dtype=bool)
        raw_edges = state["edges"]
        if len(raw_edges) > 0:
            max_tok = M.RECURRENT_SLOTS + E_max
            edges_arr = np.asarray(raw_edges, dtype=np.int64)
            valid_e = (edges_arr[:, 0] >= 0) & (edges_arr[:, 0] < max_tok) & (edges_arr[:, 1] >= 0) & (edges_arr[:, 1] < max_tok)
            kept = edges_arr[valid_e][:M_max]
            n_edg = len(kept)
            if n_edg > 0:
                edge_index[:n_edg] = kept[:, :2]
                edge_type[:n_edg] = kept[:, 2]
                edge_mask[:n_edg] = True

        cand_feat = np.zeros((C_max, M.CAND_FEAT_WIDTH), dtype=np.float32)
        cand_dsl = np.zeros((C_max, M.CARD_DSL_WIDTH), dtype=np.float32)
        cand_ent = np.full((C_max,), -1, dtype=np.int64)
        cand_seat = np.full((C_max,), -1, dtype=np.int64)
        cand_mask = np.zeros((C_max,), dtype=bool)

        raw_cf = st["cand_feat"]
        raw_ce = st["cand_entity_idx"]
        raw_cs = st["cand_rel_seat_idx"]
        n_cand = min(len(raw_cf), C_max)
        if n_cand > 0:
            cand_feat[:n_cand] = np.asarray(raw_cf[:n_cand], dtype=np.float32)
            ce_arr = np.asarray(raw_ce[:n_cand], dtype=np.int64)
            cand_ent[:n_cand] = np.where(ce_arr < E_max, ce_arr, -1)
            cand_seat[:n_cand] = np.asarray(raw_cs[:n_cand], dtype=np.int64)
            raw_ccid = st.get("cand_card_id")
            ccid_arr = (np.asarray(raw_ccid[:n_cand], dtype=np.int64)
                        if raw_ccid is not None and len(raw_ccid) >= n_cand
                        else np.full(n_cand, M.UNKNOWN_CARD, dtype=np.int64))
            raw_cface = st.get("cand_face_id")
            cface_arr = (np.asarray(raw_cface[:n_cand], dtype=np.uint8)
                         if raw_cface is not None and len(raw_cface) >= n_cand
                         else np.full(n_cand, 255, dtype=np.uint8))
            self._fill_cand_dsl(cand_dsl[:n_cand], ent_dsl, n_ent, ce_arr, ccid_arr, cface_arr)
            cand_mask[:n_cand] = True
        else:
            cand_mask[0] = True

        if len(state["entity_rule_refs"]) != len(raw_ef) or len(st["cand_rule_refs"]) != len(raw_cf):
            raise ValueError("Missing entity/action rule references; reconvert the observation stream")
        entity_rules = self.structure.pack(state["entity_rule_refs"], E_max)
        cand_rules = self.structure.pack(st["cand_rule_refs"], C_max)

        tgt = st["target"]
        raw_mm = tgt.get("multi_mask", [])
        raw_fm = tgt.get("free_mask", [])
        head = int(st["head"])
        is_decision = bool(st.get("is_decision", True))
        loss_weight = float(st.get("loss_weight", 1.0 if is_decision else 0.0))

        # Never silently relabel an out-of-range SingleChoice target to 0 (P0-2)
        raw_single_idx = int(tgt.get("single_idx", 0))
        if is_decision and head == 0 and not 0 <= raw_single_idx < n_cand:
            raise ValueError("Decision target is outside the represented candidate set")
        else:
            single_idx = min(max(raw_single_idx, 0), max(n_cand - 1, 0))

        cand_target_mask = np.zeros((C_max,), dtype=np.float32)
        cand_free_mask = np.zeros((C_max,), dtype=np.float32)
        if head == 1:  # SubsetPick
            k = min(len(raw_mm), n_cand)
            if k > 0:
                cand_target_mask[:k] = np.asarray(raw_mm[:k], dtype=np.float32)
            if len(raw_fm) >= k and k > 0:
                cand_free_mask[:k] = np.asarray(raw_fm[:k], dtype=np.float32)
            elif k > 0:
                cand_free_mask[:k] = 1.0

        pair_index = np.zeros((P_max, 2), dtype=np.int64)
        pair_mask = np.zeros((P_max,), dtype=bool)
        pair_target_mask = np.zeros((P_max,), dtype=np.float32)
        pair_free_mask = np.zeros((P_max,), dtype=np.float32)
        raw_pairs = st["pairs"]
        n_pair = 0
        for orig_i, (b_idx, a_idx) in enumerate(raw_pairs):
            if n_pair >= P_max:
                break
            if 0 <= b_idx < E_max and 0 <= a_idx < E_max:
                pair_index[n_pair, 0] = b_idx
                pair_index[n_pair, 1] = a_idx
                pair_mask[n_pair] = True
                if head == 2:
                    if orig_i < len(raw_mm):
                        pair_target_mask[n_pair] = float(raw_mm[orig_i])
                    if len(raw_fm) > orig_i:
                        pair_free_mask[n_pair] = float(raw_fm[orig_i])
                    else:
                        pair_free_mask[n_pair] = 1.0
                n_pair += 1
        if n_pair == 0:
            pair_mask[0] = True

        aux = st.get("aux", {})
        aux_vec = np.array(
            [
                float(aux.get("turns_remaining_norm", 0.0)),
                float(aux.get("my_final_life_norm", 0.0)),
                float(aux.get("opp_min_final_life_norm", 0.0)),
                float(aux.get("next_life_delta_norm", 0.0)),
            ],
            dtype=np.float32,
        )

        prov = st.get("provenance", {})
        src_kind = parse_source_kind(prov.get("source", "House"))
        has_lp = bool(prov.get("has_logprob", False))
        temp_val = float(prov.get("temperature", 0.0))

        return {
            "entity_feat": ent_feat,
            "entity_dsl": ent_dsl,
            "entity_rules": entity_rules,
            "cand_rules": cand_rules,
            "entity_mask": ent_mask,
            "seat_feat": seat_feat,
            "global_feat": global_feat,
            "edge_index": edge_index,
            "edge_type": edge_type,
            "edge_mask": edge_mask,
            "cand_feat": cand_feat,
            "cand_dsl": cand_dsl,
            "cand_entity_idx": cand_ent,
            "cand_seat_idx": cand_seat,
            "cand_mask": cand_mask,
            "pair_index": pair_index,
            "pair_mask": pair_mask,
            "is_decision": np.array(is_decision, dtype=bool),
            "loss_weight": np.array(loss_weight, dtype=np.float32),
            "is_terminal": np.array(bool(st.get("is_terminal", False)), dtype=bool),
            "head": np.array(head, dtype=np.int64),
            "single_idx": np.array(single_idx, dtype=np.int64),
            "cand_target_mask": cand_target_mask,
            "cand_free_mask": cand_free_mask,
            "pair_target_mask": pair_target_mask,
            "pair_free_mask": pair_free_mask,
            "scalar_target": np.array(float(tgt["scalar_norm"]), dtype=np.float32),
            "outcome": np.array(float(st["outcome"]), dtype=np.float32),
            "aux_target": aux_vec,
            "behavior_logprob": np.array(
                float(prov.get("behavior_logprob", 0.0)), dtype=np.float32
            ),
            "has_logprob": np.array(has_lp, dtype=bool),
            "source_kind": np.array(src_kind, dtype=np.int64),
            "temperature": np.array(temp_val, dtype=np.float32),
            "thinking_steps": np.array(
                int(prov.get("thinking_steps", 0)), dtype=np.int64
            ),
        }

    def __getitem__(self, idx: int) -> dict[str, np.ndarray]:
        if idx == -1:
            # An exhausted persistent batch lane. Never shift another game's
            # state into this lane and never drop the surviving lanes' tails.
            out = {k: np.zeros_like(v) for k, v in self[0].items()}
            out["cand_mask"][:, 0] = True
            out["pair_mask"][:, 0] = True
            out["is_new_traj"] = np.array(True, dtype=bool)
            out["segment_index"] = np.array(-1, dtype=np.int64)
            return out
        seg = self.segments[idx]
        U = self.unroll_len
        encoded = [self.encode_single_step(s) for s in seg]
        pad_count = U - len(encoded)
        if pad_count > 0:
            dummy = {k: np.zeros_like(v) for k, v in encoded[-1].items()}
            dummy["is_decision"] = np.array(False, dtype=bool)
            dummy["loss_weight"] = np.array(0.0, dtype=np.float32)
            dummy["is_terminal"] = np.array(False, dtype=bool)
            dummy["has_logprob"] = np.array(False, dtype=bool)
            dummy["cand_mask"][0] = True
            dummy["pair_mask"][0] = True
            encoded.extend([dummy] * pad_count)

        out: dict[str, np.ndarray] = {}
        for k in encoded[0].keys():
            out[k] = padded_stack([e[k] for e in encoded])
        out["step_valid"] = np.array(
            [i < len(seg) for i in range(U)], dtype=bool
        )
        out["is_new_traj"] = np.array(self.segment_is_new_traj[idx], dtype=bool)
        out["segment_index"] = np.array(idx, dtype=np.int64)
        if hasattr(self, "ppo_cache"):
            out.update({name: array[idx] for name, array in self.ppo_cache.items()})
        return out



def collate_trajectories(batch: list[dict[str, np.ndarray]]) -> dict[str, torch.Tensor]:
    out: dict[str, torch.Tensor] = {}
    for k in batch[0].keys():
        arr = padded_stack([b[k] for b in batch])
        out[k] = torch.from_numpy(arr)

    # Dynamic per-batch trimming along E, M, C, P so full caps (256/256/64/128)
    # Trim padding columns without removing encoded candidates.
    ent_any = out["entity_mask"].any(dim=(0, 1))
    e_trim = max(1, int(ent_any.nonzero()[-1].item()) + 1) if ent_any.any() else 1
    out["entity_feat"] = out["entity_feat"][:, :, :e_trim]
    out["entity_rules"] = out["entity_rules"][:, :, :e_trim]
    out["entity_dsl"] = out["entity_dsl"][:, :, :e_trim]
    out["entity_mask"] = out["entity_mask"][:, :, :e_trim]

    edg_any = out["edge_mask"].any(dim=(0, 1))
    m_trim = max(1, int(edg_any.nonzero()[-1].item()) + 1) if edg_any.any() else 1
    out["edge_index"] = out["edge_index"][:, :, :m_trim]
    out["edge_type"] = out["edge_type"][:, :, :m_trim]
    out["edge_mask"] = out["edge_mask"][:, :, :m_trim]

    cnd_any = out["cand_mask"].any(dim=(0, 1))
    c_trim = max(1, int(cnd_any.nonzero()[-1].item()) + 1) if cnd_any.any() else 1
    out["cand_feat"] = out["cand_feat"][:, :, :c_trim]
    out["cand_rules"] = out["cand_rules"][:, :, :c_trim]
    out["cand_dsl"] = out["cand_dsl"][:, :, :c_trim]
    out["cand_entity_idx"] = out["cand_entity_idx"][:, :, :c_trim]
    out["cand_seat_idx"] = out["cand_seat_idx"][:, :, :c_trim]
    out["cand_mask"] = out["cand_mask"][:, :, :c_trim]
    out["cand_target_mask"] = out["cand_target_mask"][:, :, :c_trim]
    out["cand_free_mask"] = out["cand_free_mask"][:, :, :c_trim]

    par_any = out["pair_mask"].any(dim=(0, 1))
    p_trim = max(1, int(par_any.nonzero()[-1].item()) + 1) if par_any.any() else 1
    out["pair_index"] = out["pair_index"][:, :, :p_trim]
    out["pair_mask"] = out["pair_mask"][:, :, :p_trim]
    out["pair_target_mask"] = out["pair_target_mask"][:, :, :p_trim]
    out["pair_free_mask"] = out["pair_free_mask"][:, :, :p_trim]

    return out
