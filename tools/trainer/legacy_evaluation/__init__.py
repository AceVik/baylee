"""Thin native reduction boundary for Tensor-derived legacy evaluation arrays."""
import json
import os
from pathlib import Path
import shutil
import subprocess


def native_metrics(kind, **values):
    executable = os.environ.get('BAYLEE_LEGACY_METRICS') or shutil.which('legacy_metrics')
    if executable is None:
        root = Path(__file__).resolve().parents[3]
        executable = next((str(root / 'target' / profile / 'legacy_metrics')
                           for profile in ('release', 'debug')
                           if (root / 'target' / profile / 'legacy_metrics').is_file()), None)
    if executable is None:
        raise RuntimeError('Build native baseline metrics: cargo build --release -p baylee-train-ops --bin legacy_metrics')
    result = subprocess.run([executable], input=json.dumps(dict(kind=kind, **values), allow_nan=False),
                            capture_output=True, text=True)
    if result.returncode:
        raise ValueError('Native legacy evaluation rejected: ' + result.stderr.strip())
    return json.loads(result.stdout)
