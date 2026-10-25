"""Disk-backed compressed trajectory records; bounded RAM and atomic cache builds.

The cache is an optimization of immutable converter output, not another source
of training labels. Each index row points to one compressed JSON record. Only
the small index is resident; worker processes reopen the files after fork/spawn.
"""
from __future__ import annotations

import fcntl
import gzip
import hashlib
import json
import mmap
import os
from pathlib import Path
import struct
import zlib
import re
import shutil
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import orjson

INDEX = np.dtype([
    ("game", "<u4"), ("seat", "<u4"), ("cursor", "<u8"),
    ("offset", "<u8"), ("length", "<u4"), ("flags", "<u4"),
])
ROW = struct.Struct("<IIQQII")
BST4_MAGIC = b"BST4"
BST4_HDR = struct.Struct("<4s10I2i9f12I")
STATE_FIELD = re.compile(rb',\s*"state"\s*:')
SOURCE_KIND_MAP = {"Net": 0, "House": 1, "Forced": 2, "Search": 3}


def parse_source_kind(raw) -> int:
    if isinstance(raw, int):
        return raw
    if isinstance(raw, str):
        return SOURCE_KIND_MAP.get(raw, 1)
    return 1


def _pack_rules_u32(rows: list, roots: dict[str, int]) -> tuple[np.ndarray, int]:
    n = len(rows)
    cols = max(1, max(map(len, rows), default=0))
    out = np.zeros((n, cols, 2), dtype="<u4")
    for i, refs in enumerate(rows):
        for j, ref in enumerate(refs):
            key, role = ref["key"], int(ref["role"])
            if key not in roots or not 1 <= role <= 7:
                raise ValueError(f"Unrepresented public rule reference: {ref}")
            out[i, j, 0] = roots[key]
            out[i, j, 1] = role
    return out, cols


def pack_step_bst4(st: dict, roots: dict[str, int]) -> bytes:
    """Pack a Step4 dictionary into a contiguous little-endian BST4 binary frame."""
    state = st["state"]
    raw_ef = state["entity_feat"]
    raw_cids = state["entity_card_ids"]
    n_ent = len(raw_ef)
    if len(state["entity_rule_refs"]) != n_ent or len(st["cand_rule_refs"]) != len(st["cand_feat"]):
        raise ValueError("Missing entity/action rule references")
    ent_rules, ent_rule_cols = _pack_rules_u32(state["entity_rule_refs"], roots)
    cand_rules, cand_rule_cols = _pack_rules_u32(st["cand_rule_refs"], roots)

    ef_arr = np.asarray(raw_ef, dtype="<f4")
    ent_width = int(ef_arr.shape[1]) if n_ent > 0 else 0
    cids_arr = np.asarray(raw_cids, dtype="<u4")
    raw_fids = state.get("entity_face_ids")
    fids_arr = (np.asarray(raw_fids[:n_ent], dtype="<u1")
                if raw_fids is not None and len(raw_fids) >= n_ent
                else np.zeros(n_ent, dtype="<u1"))
    if raw_fids is not None and 0 < len(raw_fids) < n_ent:
        fids_arr[:len(raw_fids)] = np.asarray(raw_fids, dtype="<u1")
    raw_tids = state.get("entity_token_ids")
    tids_arr = (np.asarray(raw_tids[:n_ent], dtype="<u2")
                if raw_tids is not None and len(raw_tids) >= n_ent
                else np.zeros(n_ent, dtype="<u2"))
    if raw_tids is not None and 0 < len(raw_tids) < n_ent:
        tids_arr[:len(raw_tids)] = np.asarray(raw_tids, dtype="<u2")

    seat_arr = np.asarray(state["seat_feat"], dtype="<f4")
    seat_width = int(seat_arr.shape[1])
    global_arr = np.asarray(state["global_feat"], dtype="<f4")
    global_width = int(global_arr.shape[0])

    raw_edges = state["edges"]
    n_edg = len(raw_edges)
    edges_arr = np.asarray(raw_edges, dtype="<i4").reshape(n_edg, 3) if n_edg > 0 else np.empty((0, 3), dtype="<i4")

    raw_cf = st["cand_feat"]
    n_cand = len(raw_cf)
    cf_arr = np.asarray(raw_cf, dtype="<f4")
    cand_width = int(cf_arr.shape[1]) if n_cand > 0 else 0
    ce_arr = np.asarray(st["cand_entity_idx"][:n_cand], dtype="<i2")
    cs_arr = np.asarray(st["cand_rel_seat_idx"][:n_cand], dtype="<i1")
    raw_ccid = st.get("cand_card_id")
    ccid_arr = (np.asarray(raw_ccid[:n_cand], dtype="<u4")
                if raw_ccid is not None and len(raw_ccid) >= n_cand
                else np.full(n_cand, 0xFFFFFFFF, dtype="<u4"))
    raw_cface = st.get("cand_face_id")
    cface_arr = (np.asarray(raw_cface[:n_cand], dtype="<u1")
                 if raw_cface is not None and len(raw_cface) >= n_cand
                 else np.full(n_cand, 255, dtype="<u1"))

    raw_pairs = st["pairs"]
    n_pair = len(raw_pairs)
    pairs_arr = np.asarray(raw_pairs, dtype="<i2").reshape(n_pair, 2) if n_pair > 0 else np.empty((0, 2), dtype="<i2")

    tgt = st["target"]
    mm_arr = np.asarray(tgt.get("multi_mask", ()), dtype="<u1")
    fm_arr = np.asarray(tgt.get("free_mask", ()), dtype="<u1")

    is_decision = bool(st.get("is_decision", True))
    is_terminal = bool(st.get("is_terminal", False))
    prov = st.get("provenance", {})
    has_lp = bool(prov.get("has_logprob", False))
    flags = int(is_decision) | (int(is_terminal) << 1) | (int(has_lp) << 2)
    loss_weight = float(st.get("loss_weight", 1.0 if is_decision else 0.0))
    aux = st.get("aux", {})

    hdr = BST4_HDR.pack(
        BST4_MAGIC,
        1,
        int(st["game_id"]),
        int(st["seat"]),
        int(st["step_cursor"]),
        int(st.get("turn", 0)),
        flags,
        int(st["head"]),
        int(st.get("pending_kind", 0)),
        parse_source_kind(prov.get("source", "House")),
        0,
        int(prov.get("thinking_steps", 0)),
        int(tgt.get("single_idx", 0)),
        loss_weight,
        float(tgt["scalar_norm"]),
        float(st["outcome"]),
        float(aux.get("turns_remaining_norm", 0.0)),
        float(aux.get("my_final_life_norm", 0.0)),
        float(aux.get("opp_min_final_life_norm", 0.0)),
        float(aux.get("next_life_delta_norm", 0.0)),
        float(prov.get("behavior_logprob", 0.0)),
        float(prov.get("temperature", 0.0)),
        n_ent,
        ent_width,
        ent_rule_cols,
        seat_width,
        global_width,
        n_edg,
        n_cand,
        cand_width,
        cand_rule_cols,
        n_pair,
        len(mm_arr),
        len(fm_arr),
    )
    return b"".join((
        hdr,
        ef_arr.tobytes(),
        cids_arr.tobytes(),
        fids_arr.tobytes(),
        tids_arr.tobytes(),
        ent_rules.tobytes(),
        seat_arr.tobytes(),
        global_arr.tobytes(),
        edges_arr.tobytes(),
        cf_arr.tobytes(),
        ce_arr.tobytes(),
        cs_arr.tobytes(),
        ccid_arr.tobytes(),
        cface_arr.tobytes(),
        cand_rules.tobytes(),
        pairs_arr.tobytes(),
        mm_arr.tobytes(),
        fm_arr.tobytes(),
    ))


def unpack_bst4_header_dict(buf: bytes) -> dict:
    (
        _magic, _ver, game_id, seat, step_cursor, turn, flags, head, pending_kind,
        _src_kind, _pad0, _k, single_idx, loss_weight, scalar_norm, outcome,
        aux0, aux1, aux2, aux3, _blp, _temp, *_dims
    ) = BST4_HDR.unpack_from(buf, 0)
    return {
        "game_id": game_id,
        "seat": seat,
        "step_cursor": step_cursor,
        "turn": turn,
        "is_decision": bool(flags & 1),
        "is_terminal": bool(flags & 2),
        "head": head,
        "pending_kind": pending_kind,
        "loss_weight": loss_weight,
        "outcome": outcome,
        "target": {"single_idx": single_idx, "scalar_norm": scalar_norm},
        "aux": {
            "turns_remaining_norm": aux0,
            "my_final_life_norm": aux1,
            "opp_min_final_life_norm": aux2,
            "next_life_delta_norm": aux3,
        },
        "_bst4": buf,
    }


def record_header(line: bytes) -> dict:
    # serde emits the small index fields before the large state arrays. Parsing
    # that prefix avoids creating millions of unused Python floats at startup.
    # Other valid JSON key orders take the general path.
    match = STATE_FIELD.search(line)
    if match:
        try:
            header = orjson.loads(line[:match.start()] + b"}")
            if all(k in header for k in ("game_id", "seat", "step_cursor", "is_decision", "is_terminal")):
                return header
        except orjson.JSONDecodeError:
            pass
    return orjson.loads(line)


def build_part(path: Path, data_path: Path, index_path: Path) -> int:
    sidecar_dir = path.parent.parent / ".shard-cache-v3"
    sidecar_meta = sidecar_dir / f"{path.name}.meta"
    sidecar_bin = sidecar_dir / f"{path.name}.bin"
    sidecar_idx = sidecar_dir / f"{path.name}.idx"
    if sidecar_meta.exists() and sidecar_bin.exists() and sidecar_idx.exists():
        try:
            st = path.stat()
            meta = json.loads(sidecar_meta.read_text())
            if meta.get("size") == st.st_size and meta.get("mtime_ns") == st.st_mtime_ns:
                shutil.copyfile(sidecar_bin, data_path)
                shutil.copyfile(sidecar_idx, index_path)
                return int(meta["records"])
        except (OSError, ValueError, KeyError):
            pass

    roots = None
    struct_path = path.parent.parent / "card_structure.json"
    if struct_path.exists():
        try:
            roots = json.loads(struct_path.read_text()).get("roots")
        except (OSError, ValueError):
            roots = None

    count = 0
    with data_path.open("wb") as data, index_path.open("wb") as index, gzip.open(path, "rb") as inp:
        for line in inp:
            if not line.strip():
                continue
            if roots is not None:
                step = orjson.loads(line)
                try:
                    raw = pack_step_bst4(step, roots)
                except (KeyError, ValueError, TypeError):
                    raw = line
            else:
                step = record_header(line)
                raw = line
            blob = zlib.compress(raw, level=1)
            flags = int(step["is_decision"]) | (int(step.get("is_terminal", False)) << 1)
            index.write(ROW.pack(step["game_id"], step["seat"], step["step_cursor"],
                                 data.tell(), len(blob), flags))
            data.write(blob)
            count += 1
    return count


class TrajectoryStore4:
    def __init__(self, data_dir: Path, split: str, shards: list[str]):
        paths = [data_dir / split / name for name in shards]
        identity = hashlib.sha256((data_dir / "meta.json").read_bytes())
        for path in paths:
            st = path.stat()  # Missing shards are corruption, never silently skipped.
            identity.update(f"{path.name}:{st.st_size}:{st.st_mtime_ns}".encode())
        self.fingerprint = identity.hexdigest()
        cache = data_dir / ".trajectory-cache-v2"
        cache.mkdir(exist_ok=True)
        stem = f"{split}-{self.fingerprint[:24]}"
        self.data_path = cache / f"{stem}.bin"
        self.index_path = cache / f"{stem}.idx"
        done = cache / f"{stem}.json"
        with (cache / f"{stem}.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if not done.exists():
                self._build(paths, done)
        size = self.index_path.stat().st_size
        if size % INDEX.itemsize:
            raise ValueError(f"Corrupt trajectory index: {self.index_path}")
        self.index = (np.memmap(self.index_path, mode="r", dtype=INDEX)
                      if size else np.empty(0, dtype=INDEX))
        self._pid = None
        self._file = None
        self._map = None

    def _build(self, paths: list[Path], done: Path) -> None:
        data_tmp = self.data_path.with_suffix(".bin.partial")
        index_tmp = self.index_path.with_suffix(".idx.partial")
        count = 0
        parts = [(path, data_tmp.with_suffix(f".part{i}.bin"), index_tmp.with_suffix(f".part{i}.idx"))
                 for i, path in enumerate(paths)]
        max_workers = min(12, os.cpu_count() or 4, max(1, len(parts)))
        with ThreadPoolExecutor(max_workers=max_workers) as pool:
            jobs = [pool.submit(build_part, *part) for part in parts]
            for job in jobs:
                count += job.result()
        with data_tmp.open("wb") as data, index_tmp.open("wb") as index:
            for _, part_data, part_index in parts:
                rows = np.fromfile(part_index, dtype=INDEX)
                rows["offset"] += data.tell()
                index.write(rows.tobytes())
                with part_data.open("rb") as src:
                    shutil.copyfileobj(src, data, length=1 << 20)
                part_data.unlink()
                part_index.unlink()
            data.flush()
            index.flush()
            os.fsync(data.fileno())
            os.fsync(index.fileno())
        os.replace(data_tmp, self.data_path)
        os.replace(index_tmp, self.index_path)
        marker = done.with_suffix(".json.partial")
        marker.write_text(json.dumps({"fingerprint": self.fingerprint, "records": count}))
        os.replace(marker, done)

    def read_raw(self, row: int) -> bytes:
        if self._pid != os.getpid():
            if self._map is not None:
                self._map.close()
                self._file.close()
            self._file = self.data_path.open("rb")
            self._map = mmap.mmap(self._file.fileno(), 0, access=mmap.ACCESS_READ)
            self._pid = os.getpid()
        rec = self.index[row]
        start = int(rec["offset"])
        return zlib.decompress(self._map[start:start + int(rec["length"])])

    def read(self, row: int) -> dict:
        buf = self.read_raw(row)
        if buf[:4] == BST4_MAGIC:
            return unpack_bst4_header_dict(buf)
        return orjson.loads(buf)

    def __getstate__(self):
        state = dict(self.__dict__)
        state.update(_pid=None, _file=None, _map=None)
        return state

    def close(self):
        if self._map is not None:
            self._map.close()
            self._file.close()
            self._map = self._file = self._pid = None

