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
import random
from pathlib import Path
from typing import Any, Iterator

import numpy as np
import torch
from torch.utils.data import Dataset, Sampler

import model4 as M
from trajectory_store4 import TrajectoryStore4

SOURCE_KIND_MAP = {
    "Net": 0,
    "House": 1,
    "Forced": 2,
    "Search": 3,
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
    ):
        super().__init__()
        self.data_dir = Path(data_dir).expanduser()
        self.meta = json.loads((self.data_dir / "meta.json").read_text())
        assert self.meta["encoder_version"] == M.ENCODER_VERSION, (
            f"Expected encoder_version={M.ENCODER_VERSION}, got {self.meta['encoder_version']}"
        )
        if self.meta.get("trajectory_version") != M.TRAJECTORY_VERSION:
            raise ValueError("Reconvert records with the current complete-observation converter")
        self.unroll_len = unroll_len
        self.max_entities = max_entities
        self.max_edges = max_edges
        self.max_cands = max_cands
        self.max_pairs = max_pairs

        # Map card_id -> row index in card_dsl.bin / card_dsl_back.bin
        cards = self.meta["cards"]
        self.card_to_row: dict[int, int] = {int(cid): i for i, cid in enumerate(cards)}
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

    def encode_single_step(self, step_or_row: dict[str, Any] | int) -> dict[str, np.ndarray]:
        st = step_or_row if isinstance(step_or_row, dict) else self.store.read(int(step_or_row))
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
        raw_fids = state.get("entity_face_ids", [0] * len(raw_cids))
        raw_tids = state.get("entity_token_ids", [0] * len(raw_cids))
        n_ent = min(len(raw_ef), E_max)
        if n_ent > 0:
            ent_feat[:n_ent] = np.asarray(raw_ef[:n_ent], dtype=np.float32)
            for i in range(n_ent):
                cid = int(raw_cids[i])
                fid = int(raw_fids[i]) if i < len(raw_fids) else 0
                tslot = int(raw_tids[i]) if i < len(raw_tids) else 0
                ent_dsl[i] = self.lookup_entity_dsl(cid, fid, tslot)
            ent_mask[:n_ent] = True

        seat_feat = np.asarray(state["seat_feat"], dtype=np.float32)
        global_feat = np.asarray(state["global_feat"], dtype=np.float32)

        edge_index = np.zeros((M_max, 2), dtype=np.int64)
        edge_type = np.zeros((M_max,), dtype=np.int64)
        edge_mask = np.zeros((M_max,), dtype=bool)
        raw_edges = state["edges"]
        n_edg = 0
        max_tok = M.RECURRENT_SLOTS + E_max
        for src, dst, rel in raw_edges:
            if n_edg >= M_max:
                break
            if 0 <= src < max_tok and 0 <= dst < max_tok:
                edge_index[n_edg, 0] = src
                edge_index[n_edg, 1] = dst
                edge_type[n_edg] = rel
                edge_mask[n_edg] = True
                n_edg += 1

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
            for i in range(n_cand):
                ce = int(raw_ce[i])
                cand_ent[i] = ce if ce < E_max else -1
                cand_seat[i] = int(raw_cs[i])
                cid = int(st.get("cand_card_id", [M.UNKNOWN_CARD] * n_cand)[i])
                face = int(st.get("cand_face_id", [255] * n_cand)[i])
                if face != 255:
                    cand_dsl[i] = self.lookup_entity_dsl(cid, face, 0)
                elif 0 <= ce < n_ent:
                    cand_dsl[i] = ent_dsl[ce]
                elif cid != M.UNKNOWN_CARD:
                    cand_dsl[i] = self.lookup_entity_dsl(cid, 0, 0)
            cand_mask[:n_cand] = True
        else:
            cand_mask[0] = True

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


class StatefulBatchSampler4(Sampler[list[int]]):
    """Streams multi-segment trajectories across `batch_size` persistent lanes
    so recurrent state `h` can be carried across consecutive segments (`P0-4`).
    """

    def __init__(
        self,
        dataset: TrajectoryDataset4,
        batch_size: int,
        shuffle: bool = True,
        seed: int = 42,
    ):
        self.dataset = dataset
        self.batch_size = max(1, batch_size)
        self.shuffle = shuffle
        self.seed = seed
        self.epoch = 0

    def _build_batches(self, advance_epoch: bool = True) -> list[list[int]]:
        trajs = [list(t) for t in self.dataset.trajectories if t]
        if not trajs:
            return []
        if self.shuffle:
            rng = random.Random(self.seed + self.epoch)
            rng.shuffle(trajs)
        if advance_epoch:
            self.epoch += 1

        B = min(self.batch_size, len(trajs))
        queue_idx = B
        lanes: list[list[int]] = [trajs[i] for i in range(B)]
        cursors: list[int] = [0] * B
        batches: list[list[int]] = []

        while True:
            batch: list[int] = []
            active_any = False
            for b in range(B):
                if cursors[b] >= len(lanes[b]):
                    if queue_idx < len(trajs):
                        lanes[b] = trajs[queue_idx]
                        queue_idx += 1
                        cursors[b] = 0
                    else:
                        continue
                active_any = True
            if not active_any:
                break
            for b in range(B):
                if cursors[b] < len(lanes[b]):
                    batch.append(lanes[b][cursors[b]])
                    cursors[b] += 1
                else:
                    batch.append(-1)
            batches.append(batch)
        return batches

    def __iter__(self) -> Iterator[list[int]]:
        yield from self._build_batches()[getattr(self, "start_batch", 0):]

    def __len__(self) -> int:
        return len(self._build_batches(advance_epoch=False))


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
