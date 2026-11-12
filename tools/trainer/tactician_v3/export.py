"""The tactician's ONNX graph (`tactician_head_v2`) at any number of passes.

`rows` [U, 2d + 128] (the null row first) -> `logits` [U - 1] (the final pass's
logit per unit) and `confidence` [1] (its probability that the answer is the
label). Every export is checked against PyTorch on random rows of several sizes
and windows and on recorded rows, and refused past 1e-4.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy as np
import torch

from .model import Exported, TacticianV3
from .records import ROW_TAIL

PARITY_LIMIT = 1e-4


def random_rows(d_model: int, units: int, window: int, seed: int) -> torch.Tensor:
    """Rows of one question: random state, units and tails, the window code set."""
    g = torch.Generator().manual_seed(seed)
    rows = torch.randn(units + 1, 2 * d_model + ROW_TAIL, generator=g)
    rows[0, d_model:2 * d_model] = 0.0
    code = rows[:, 2 * d_model:2 * d_model + 16]
    code[:, :4] = 0.0
    code[:, window] = 1.0
    code[0, 3] = 1.0
    code[:, 5] = 0.0
    code[:, 11] = 0.0
    return rows


def onnx_session(path: Path):
    import onnxruntime as ort
    options = ort.SessionOptions()
    options.intra_op_num_threads = 1
    return ort.InferenceSession(str(path), sess_options=options,
                                providers=["CPUExecutionProvider"])


def parity(net: TacticianV3, passes: int, path: Path, rows_list) -> float:
    """The largest difference between ONNX Runtime and PyTorch on `rows_list`."""
    session = onnx_session(path)
    wrapper = Exported(net, passes).eval()
    worst = 0.0
    with torch.no_grad():
        for rows in rows_list:
            want_logits, want_conf = wrapper(rows.float())
            got_logits, got_conf = session.run(["logits", "confidence"],
                                               {"rows": rows.float().numpy()})
            if got_logits.shape != tuple(want_logits.shape) or got_conf.shape != (1,):
                raise ValueError("ONNX outputs of another shape")
            worst = max(worst, float(np.max(np.abs(got_logits - want_logits.numpy()))),
                        float(np.max(np.abs(got_conf - want_conf.numpy()))))
    return worst


def export_onnx(net: TacticianV3, passes: int, path: Path) -> None:
    import onnx  # noqa: F401  (the exporter needs it)
    wrapper = Exported(net.cpu().eval(), passes).eval()
    dummy = random_rows(net.cfg.d_model, 3, 0, 0)
    torch.onnx.export(wrapper, (dummy,), str(path), input_names=["rows"],
                      output_names=["logits", "confidence"],
                      dynamic_axes={"rows": {0: "U"}, "logits": {0: "N"}},
                      opset_version=18, dynamo=False)


def check_rows(d_model: int, recorded=()) -> list:
    """Random questions of every window and several sizes, then `recorded`."""
    rows = [random_rows(d_model, units, window, 17 * units + window)
            for units in (1, 2, 5, 17) for window in (0, 1, 2)]
    return rows + [r.float() for r in recorded]


def export_dir(net: TacticianV3, meta: dict, passes: int, out: Path, recorded=()) -> dict:
    """Writes `tactician.onnx` and `tactician.json` at `passes` into `out`."""
    out.mkdir(parents=True, exist_ok=True)
    net = net.cpu().eval()
    path = out / "tactician.onnx"
    export_onnx(net, passes, path)
    worst = parity(net, passes, path, check_rows(net.cfg.d_model, recorded))
    if worst > PARITY_LIMIT:
        raise ValueError(f"ONNX parity failed at T={passes}: {worst:.3g}")
    thresholds = meta.get("thresholds_by_pass") or [0.5]
    threshold = thresholds[min(passes, len(thresholds)) - 1]
    written = dict(meta, passes=passes, confidence_threshold=threshold,
                   threshold_from_pass=min(passes, len(thresholds)),
                   io=dict(input=dict(name="rows", shape=["U", meta["d_model"] * 2 + ROW_TAIL],
                                      rows="null row first, then one per unit"),
                           outputs=[dict(name="logits", shape=["U - 1"]),
                                    dict(name="confidence", shape=[1])]),
                   parity_max_abs_diff=worst, parity_rows=len(check_rows(net.cfg.d_model,
                                                                         recorded)),
                   onnx_sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    (out / "tactician.json").write_text(json.dumps(written, indent=2))
    return written


def load_checkpoint(path: Path) -> tuple[TacticianV3, dict]:
    from .model import Config
    checkpoint = torch.load(path, map_location="cpu", weights_only=False)
    net = TacticianV3(Config(**checkpoint["config"]))
    net.load_state_dict(checkpoint["state_dict"])
    return net.eval(), checkpoint["meta"]
