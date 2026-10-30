"""New head-only checkpoint with every other tensor checked bit-for-bit."""
from pathlib import Path
import torch
import model4 as M
from checkpoint4 import atomic_save
from .source_identity import file_digest
from .model_identity import source_model, source_identity
from .cache import metadata
from tensor_io4 import TensorPublication


def collection_plan(path, source_hash):
    return metadata('collection_plan',path=None if path is None else str(Path(path).expanduser()),source_hash=source_hash)


def publish(output, payload, report):
    """Cooperating writers lock one destination and publish all files at once."""
    with TensorPublication('directory',output) as publication:
        atomic_save(payload,publication.path/'net.pt')
        publication.commit(metadata=report)


def load_source(model):
    path = Path(model).expanduser()
    path = path / "net.pt" if path.is_dir() else path
    saved = torch.load(path, map_location="cpu", weights_only=False)
    for name in ("trajectory_version", "sampling_version"):
        if saved.get(name) != getattr(M, name.upper()):
            raise ValueError(f"Incompatible source {name}")
    net = source_model(saved)
    from encoder_contract import validate_encoder
    validate_encoder(saved.get('encoder_version'), model_version=saved.get('model_version', 4),
                     relational_revision=getattr(net.cfg, 'relational_revision', 1),
                     candidate_context_revision=getattr(net.cfg, 'candidate_context_revision', 1),
                     global_feat_width=net.cfg.global_feat_width)
    net.attach_structure(saved["structure"])
    if saved.get("structure_sha256", net.rules.digest) != net.rules.digest:
        raise ValueError("Source declared rule catalog digest differs")
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
    return {**source_identity(saved), "structure": saved["structure"],
            "state_dict": state, "provenance": provenance}
