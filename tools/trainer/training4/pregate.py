"""The offline pre-gate as its own pipeline stage, from a finished run's meta.json.

    python -m training4.pregate --model /mnt/xdata/.../round0-A2prime-s1 --min-ratio 0.8

Reads the validation metrics of the run's selected checkpoint (`meta.json`
`val_metrics`, written by a run whose validation counts R's acting chains) and
exits 3 when `training4.distill.pregate` fails, so a pipeline stops before any
play gate while the checkpoint and its export stay for diagnosis. A run from an
older source has no chain counts and fails; `training4.distill_audit` computes
them for its checkpoint on the CPU.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from .distill import pregate


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--model", required=True, help="Run directory (meta.json inside) or a meta.json")
    parser.add_argument("--min-ratio", type=float, default=0.8)
    args = parser.parse_args(argv)
    path = Path(args.model).expanduser()
    meta = json.loads((path / "meta.json" if path.is_dir() else path).read_text())
    passed, verdict = pregate(meta.get("val_metrics") or {}, args.min_ratio)
    print("[pregate] " + json.dumps(verdict), flush=True)
    return 0 if passed else 3


if __name__ == "__main__":
    sys.exit(main())
