"""Cache compatibility includes the implementation that creates its features."""
from pathlib import Path
from torch_io4 import file_digest, source_digest


def extraction_sources_digest():
    root = Path(__file__).resolve().parents[1]
    return source_digest(root, ["dataset4.py", "sampler4.py", "trajectory_store4", "tensor_io4.py",
                               "structure_data4.py", "training4/execution.py", "model4", "model5",
                               *["value_calibration4/" + name for name in
                                 ("extract.py", "cache.py", "features.py", "source_identity.py", "model_identity.py")]])
