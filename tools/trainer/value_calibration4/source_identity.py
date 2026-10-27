"""Cache compatibility includes the implementation that creates its features."""
import hashlib
from pathlib import Path


def extraction_sources_digest():
    root = Path(__file__).resolve().parents[1]
    sources = [root / name for name in ("dataset4.py", "sampler4.py", "trajectory_store4.py",
                                        "structure_data4.py", "training4/execution.py")]
    sources += sorted((root / "model4").glob("*.py"))
    sources += sorted((root / "model5").glob("*.py"))
    sources += [Path(__file__).parent / name for name in
                ("extract.py", "cache.py", "features.py", "source_identity.py", "model_identity.py")]
    digest = hashlib.sha256()
    for path in sorted(sources):
        digest.update(str(path.relative_to(root)).encode() + b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def file_digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()
