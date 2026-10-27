"""Process-local SHA256 receipts for explicitly immutable controller inputs.

A hit proves that the same resolved regular file has the metadata of the last
successful read. It is not a filesystem lock or protection against a privileged
writer forging metadata. Keep sealed expected digests separately and verify them
before/after consumers. Never restore this cache from JSON receipts. Writers
changing any file identity cause a miss, never an update to an expected seal.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import os
from pathlib import Path
import stat
import threading
from typing import Mapping

STRATEGY = "process_memory_stat_identity_v1"
CHUNK_BYTES = 1024 * 1024


class InputChangedError(RuntimeError):
    """An input changed/replaced while its receipt was being established."""


@dataclass(frozen=True)
class FileIdentity:
    dev: int
    ino: int
    size: int
    mtime_ns: int
    ctime_ns: int

    @classmethod
    def of(cls, value):
        if not stat.S_ISREG(value.st_mode):
            raise ValueError("Input digest requires a regular file")
        return cls(value.st_dev, value.st_ino, value.st_size,
                   value.st_mtime_ns, value.st_ctime_ns)


@dataclass(frozen=True)
class DigestReceipt:
    resolved_path: str
    identity: FileIdentity
    sha256: str

    def as_dict(self):
        return asdict(self)


def _snapshot(requested):
    resolved = requested.resolve(strict=True)
    identity = FileIdentity.of(resolved.stat())
    # Resolve again: symlink retargeting must not associate a target's receipt
    # with a different requested path, even if both targets have equal sizes.
    if (requested.resolve(strict=True) != resolved
            or FileIdentity.of(requested.stat()) != identity):
        raise InputChangedError(f"Input changed during path resolution: {requested}")
    return str(resolved), identity


def _recheck(requested, expected):
    try:
        actual = _snapshot(requested)
    except (OSError, ValueError) as error:
        raise InputChangedError(f"Input disappeared/replaced during hashing: {requested}") from error
    if actual != expected:
        raise InputChangedError(f"Input changed during hashing: {requested}")


class InputDigestCache:
    """Optional per-controller cache; no global state or persistent cache trust.

    Each lookup rechecks symlinks and the complete strict file identity. A
    miss reads bytes in bounded chunks, fstats the opened descriptor before and
    after reading, then rechecks the original path. A concurrent edit raises;
    callers must decide how to quarantine it rather than silently retrying.
    """

    def __init__(self):
        self._entries = {}
        self._lock = threading.RLock()
        self._hits = self._hashes = self._bytes_read = 0

    def statistics(self):
        with self._lock:
            return dict(hits=self._hits, hashes=self._hashes, bytes_read=self._bytes_read)

    def sha256(self, path):
        return self.receipt(path).sha256

    def receipt(self, path):
        requested = Path(path).absolute()
        with self._lock:
            snapshot = _snapshot(requested)
            resolved, identity = snapshot
            previous = self._entries.get(str(requested))
            if (previous is not None and previous.resolved_path == resolved
                    and previous.identity == identity):
                _recheck(requested, snapshot)
                self._hits += 1
                return previous
            self._hashes += 1
            # O_NOFOLLOW rejects replacement of the resolved final component by
            # a symlink. O_NONBLOCK avoids hanging if replaced with a FIFO;
            # fstat below rejects all nonregular files before reading bytes.
            flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
            try:
                descriptor = os.open(resolved, flags)
            except OSError as error:
                raise InputChangedError(f"Input replaced before open: {requested}") from error
            with os.fdopen(descriptor, "rb") as stream:
                if FileIdentity.of(os.fstat(stream.fileno())) != identity:
                    raise InputChangedError(f"Input replaced before hashing: {requested}")
                digest = hashlib.sha256()
                while chunk := stream.read(CHUNK_BYTES):
                    digest.update(chunk)
                    self._bytes_read += len(chunk)
                if FileIdentity.of(os.fstat(stream.fileno())) != identity:
                    raise InputChangedError(f"Input modified during hashing: {requested}")
            _recheck(requested, snapshot)
            result = DigestReceipt(resolved, identity, digest.hexdigest())
            self._entries[str(requested)] = result
            return result

    def verify(self, expected: Mapping[str | Path, str]):
        """Check caller-owned sealed hashes; never replace or mutate that seal."""
        for path, digest in expected.items():
            if self.sha256(path) != digest:
                raise ValueError(f"Sealed input digest mismatch: {path}")
