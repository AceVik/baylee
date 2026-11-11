"""The offline pre-gate as its own pipeline stage, from a finished run's meta.json.

    python -m training4.pregate --model /mnt/xdata/.../round0-A2prime-s1 --min-ratio 0.8

Reads the validation metrics of the run's selected checkpoint (`meta.json`
`val_metrics`, written by a run whose validation counts R's acting chains) and
exits 3 when `training4.distill.pregate` fails, so a pipeline stops before any
play gate while the checkpoint and its export stay for diagnosis. A run from an
older source has no chain counts and fails; `training4.distill_audit` computes
them for its checkpoint on the CPU.

A house-teacher run (`--distill-teacher house`) is judged by the house gate
instead (`training4.house.house_gate`: `--gate-top1` agreement with H at N != H
over every kind, the student's chain-start act rate at least `--gate-ratio`
times H's); the pre-gate against r1 is printed beside it.
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
    parser.add_argument("--gate-top1", type=float, default=0.70)
    parser.add_argument("--gate-ratio", type=float, default=0.9)
    args = parser.parse_args(argv)
    path = Path(args.model).expanduser()
    meta = json.loads((path / "meta.json" if path.is_dir() else path).read_text())
    metrics = meta.get("val_metrics") or {}
    passed, verdict = pregate(metrics, args.min_ratio)
    print("[pregate] " + json.dumps(verdict), flush=True)
    if "house" in (meta.get("bundle_distillation") or {}):
        from .house import house_gate
        passed, gate = house_gate(metrics, args.gate_top1, args.gate_ratio)
        print("[house-gate] " + json.dumps(gate), flush=True)
    return 0 if passed else 3


if __name__ == "__main__":
    sys.exit(main())
