"""CPU latency of exported depth graphs, one decision per call (what a seat pays).

Times every `model.kK.onnx` of each export directory with ONNX Runtime on one
thread, on rows of the given board and menu sizes, and prints the median and
the 90th percentile in milliseconds. Comparing a source export with its
thinking revision 2 export gives the cost of a deliberation step.

Usage:
    python3 tools/trainer/bench_thinking5.py --model DIR [--model DIR ...] \
        [--shapes 26x5,64x8,160x12] [--repeats 40] [--threads 1]
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
import time

import onnxruntime as ort

import model4 as M
from export_onnx4 import make_dummy_inputs


def time_graphs(paths, d_model, global_width, entities, candidates, repeats, threads):
    """Median and p90 per depth graph. The graphs take turns within every
    repeat, so a change in machine load falls on all of them alike."""
    options = ort.SessionOptions()
    options.intra_op_num_threads = threads
    runs = {}
    for depth, path in paths.items():
        session = ort.InferenceSession(str(path), sess_options=options, providers=["CPUExecutionProvider"])
        inputs = make_dummy_inputs(d_model, E=entities, M_edges=2 * entities, C=candidates, P=4, K=depth,
                                   global_feat_width=global_width)
        feed = {name: tensor.numpy() for name, tensor in zip(M.INPUT_NAMES, inputs)}
        for _ in range(3):
            session.run(None, feed)
        runs[depth] = (session, feed, [])
    for _ in range(repeats):
        for session, feed, samples in runs.values():
            start = time.perf_counter()
            session.run(None, feed)
            samples.append((time.perf_counter() - start) * 1000)
    report = {}
    for depth, (_, _, samples) in runs.items():
        samples.sort()
        report[f"k{depth}"] = {"median_ms": round(statistics.median(samples), 2),
                               "p90_ms": round(samples[min(len(samples) - 1, int(0.9 * len(samples)))], 2)}
    return report


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--model", action="append", required=True)
    ap.add_argument("--shapes", default="26x5,64x8,160x12", help="ENTITIESxCANDIDATES, comma-separated")
    ap.add_argument("--repeats", type=int, default=40)
    ap.add_argument("--threads", type=int, default=1)
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    report = {}
    for directory in args.model:
        root = Path(directory).expanduser()
        meta = json.loads((root / "model.onnx.json").read_text())
        width = meta.get("config", {}).get("global_feat_width", M.GLOBAL_FEAT_WIDTH)
        rows = {}
        for shape in args.shapes.split(","):
            entities, candidates = (int(part) for part in shape.split("x"))
            paths = {int(depth): root / name for depth, name in meta["graphs"].items()}
            rows[shape] = time_graphs(dict(sorted(paths.items())), meta["d_model"], width, entities, candidates,
                                      args.repeats, args.threads)
        report[str(root)] = {"thinking": meta.get("thinking_outputs"), "precision": meta.get("precision", "float32"),
                             "shapes": rows}
    text = json.dumps(report, indent=2)
    print(text)
    if args.out:
        Path(args.out).expanduser().write_text(text + "\n")


if __name__ == "__main__":
    main()
