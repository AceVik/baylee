"""New head-only checkpoint with every other tensor checked bit-for-bit."""
import fcntl
import json
import os
from pathlib import Path
import shutil
import tempfile
import torch
import model4 as M
from checkpoint4 import atomic_save
from .source_identity import file_digest


def collection_plan(path, source_hash):
    if path is None:
        return None
    path = Path(path).expanduser().resolve()
    content = path.read_bytes()
    plan = json.loads(content)
    if plan.get("model_sha256") != source_hash:
        raise ValueError("Collection plan model_sha256 differs from source checkpoint")
    import hashlib
    return dict(path=str(path), sha256=hashlib.sha256(content).hexdigest(), model_sha256=source_hash)


def publish(output, payload, report):
    """Cooperating writers lock one destination and publish all files at once."""
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    with (output.parent / (output.name + ".publish.lock")).open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if output.exists() or output.is_symlink():
            raise FileExistsError("Preserving existing output; use a new directory")
        temporary = Path(tempfile.mkdtemp(prefix=output.name + ".partial-", dir=output.parent))
        try:
            atomic_save(payload, temporary / "net.pt")
            with (temporary / "report.json").open("w") as stream:
                json.dump(report, stream, indent=2, allow_nan=False)
                stream.write("\n")
                stream.flush()
                os.fsync(stream.fileno())
            os.rename(temporary, output)
        finally:
            if temporary.exists():
                shutil.rmtree(temporary)


def load_source(model):
    path = Path(model).expanduser()
    path = path / "net.pt" if path.is_dir() else path
    saved = torch.load(path, map_location="cpu", weights_only=False)
    for name in ("encoder_version", "model_version", "trajectory_version", "sampling_version"):
        if saved.get(name) != getattr(M, name.upper()):
            raise ValueError(f"Incompatible source {name}")
    net = M.PlayerNet4(M.Config4(**saved["config"]))
    net.attach_structure(saved["structure"])
    net.load_state_dict(saved["state_dict"], strict=True)
    if not all(torch.isfinite(p).all() for p in net.parameters()):
        raise ValueError("Nonfinite source weights")
    net.eval().requires_grad_(False)
    return path, saved, net


def merged_checkpoint(saved, head, provenance):
    state = {k: v.detach().cpu().clone() for k, v in saved["state_dict"].items()}
    for key, value in head.state_dict().items():
        full = "seat_value_head." + key
        if full not in state or state[full].shape != value.shape:
            raise ValueError("Calibrated head architecture differs")
        state[full] = value.detach().cpu().clone()
    for key, original in saved["state_dict"].items():
        if not key.startswith("seat_value_head.") and not torch.equal(original.cpu(), state[key]):
            raise AssertionError(f"Calibration altered frozen tensor: {key}")
    return {**{key: saved[key] for key in ("encoder_version", "model_version", "trajectory_version",
                                          "sampling_version", "config", "structure")},
            "state_dict": state, "provenance": provenance}
