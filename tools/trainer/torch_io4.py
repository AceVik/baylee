"""Thin PyTorch-worker boundary for native receipts and immutable file seals.

Filesystem administration lives in baylee-train-ops. There is no Python fallback.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess


def native(arguments, payload=None):
    executable = os.environ.get("BAYLEE_TRAIN_OPS") or shutil.which("train_ops")
    if executable is None:
        root = Path(__file__).resolve().parents[2]
        executable = next((str(root / "target" / profile / "train_ops")
                           for profile in ("release", "debug")
                           if (root / "target" / profile / "train_ops").is_file()), None)
    if executable is None:
        raise RuntimeError("Build native training operations: cargo build --release -p baylee-train-ops --bin train_ops")
    result = subprocess.run([executable, *map(str, arguments)], input=payload,
                            capture_output=True, text=True)
    if result.returncode:
        raise ValueError("Native training operation rejected: " + result.stderr.strip())
    return result.stdout


def atomic_json(path, value):
    native(["write-json", "--out", Path(path).absolute()], json.dumps(value, allow_nan=False))


def append_json(path, value):
    native(["append-json", "--out", Path(path).absolute()], json.dumps(value, allow_nan=False))


def atomic_copy(source, destination):
    native(["copy-file", "--source", Path(source).absolute(), "--out", Path(destination).absolute()])


def file_digest(path):
    result = native(["hash", path]).split()
    digest = result[0] if result else ""
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("Native checkpoint seal returned malformed SHA256")
    return digest


def source_digest(root, includes):
    args = ["source-digest", "--root", root]
    for path in includes:
        args.extend(["--include", path])
    return native(args).strip()
