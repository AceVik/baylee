"""Bounded mmap cache for immutable trajectory shards."""
from __future__ import annotations

import fcntl
import hashlib
import json
import mmap
import os
from pathlib import Path
import shutil
import zlib
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import orjson
from .packing import INDEX, BST4_MAGIC, build_part, unpack_bst4_header_dict


class TrajectoryStore4:
    def __init__(self, data_dir: Path, split: str, shards: list[str]):
        from dataset_contract4 import validate_bc_dataset
        validate_bc_dataset(data_dir, json.loads((data_dir / 'meta.json').read_text()))
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

