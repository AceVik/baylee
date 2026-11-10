"""Where BayleeAI's GPU memory goes, part by part, in FP16 inference and in training.

Measurement only: no model, trainer or evaluator code changes. The GPU modes import
the frozen trainer source named by `--trainer-dir` (never this tree's copy) and run
under `train_ops pipeline` with the GPU lock:

  train    real `train5` updates through the trainer's own driver, with CUDA memory
           history. Each phase of a step is stamped as allocator metadata; parameters,
           gradients, AdamW state, the L2-SP anchor and buffers are annotated by
           address. The trace is replayed to the peak and every live block there is
           attributed to a kind (parameters, gradients, forward activations, backward
           working set, inputs, ...) and to a part of the net by its Python frames.
  infer    the FP16 graph's code path (the ONNX tracing branch: explicit attention
           scores, K unrolled thinking steps) eagerly in PyTorch FP16 on real
           validation frames, padded into calls the way `eval4 --inference-batch`
           pads them; weights per module, the live set at each call's peak, and every
           module's own working set.
  rules    the rule encoder alone on real training batches: the active closure versus
           the whole catalog, forward and backward, memory and time, and the frozen
           table a cache would hold.
  analyze  rerun the attribution of `train`/`infer` on a saved snapshot.

The trace replay and the frame classification are pure and CPU-only
(`test_vram_by_part4.py`).
"""
from __future__ import annotations

import argparse
import ast
import contextlib
import json
import math
import os
import pickle
import random
import statistics
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

MB = 1e6  # decimal, as the trainer's `gpu_peak_gb`

# --------------------------------------------------------------------------- pure

ALLOC, FREE_REQUESTED, FREE_COMPLETED, ANNOTATE = "alloc", "free_requested", "free_completed", "annotate"
RESERVED_DELTA = {"segment_alloc": 1, "segment_map": 1, "segment_free": -1, "segment_unmap": -1}


@dataclass(eq=False)
class Block:
    """One live allocation during the replay."""
    addr: int
    size: int
    frames: list
    metadata: str
    index: int  # trace index of its allocation; -1 when allocated before the trace window
    annotations: list = field(default_factory=list)


def step_of(metadata):
    """`'s12:backward'` -> 12; untagged or non-step metadata -> None."""
    if metadata and metadata[0] == "s" and ":" in metadata:
        head = metadata[1:metadata.index(":")]
        if head.isdigit():
            return int(head)
    return None


def phase_of(metadata):
    """`'s12:backward'` -> `'backward'`, `'max:forward'` -> `'forward'`."""
    return metadata.rsplit(":", 1)[-1] if metadata else ""


def initial_blocks(trace, final_blocks):
    """Blocks already allocated when the trace window opens.

    An address whose first event in the window is a free request was allocated
    before it; a block live at the end with no event in the window was too.
    `final_blocks` are the snapshot's `active_allocated` blocks.
    """
    first = {}
    for event in trace:
        if event["action"] in (ALLOC, FREE_REQUESTED, FREE_COMPLETED):
            first.setdefault(event["addr"], event)
    live = {}
    for addr, event in first.items():
        if event["action"] == FREE_REQUESTED:
            live[addr] = Block(addr, event["size"], event.get("frames") or [], event.get("user_metadata", ""), -1)
    for block in final_blocks:
        if block["address"] not in first:
            live[block["address"]] = Block(block["address"], block["size"], block.get("frames") or [], "", -1)
    return live


def replay(trace, final_blocks, capture=(), reserved_end=None):
    """Allocated bytes after every event, and the live blocks at `capture` indices.

    Allocated bytes follow the allocator's own `allocated_bytes` statistic: a block
    counts from `alloc` until `free_requested`. Annotations attach to the block live
    at their address, also after a capture, so a gradient annotated after the peak
    is still known as one in the captured set. `reserved_end` (bytes reserved when
    the snapshot was taken) gives the reserved series by walking segment events
    backward.
    """
    live = initial_blocks(trace, final_blocks)
    total = sum(block.size for block in live.values())
    start = total
    capture = set(capture)
    captured = {-1: list(live.values())} if -1 in capture else {}
    allocated = []
    for i, event in enumerate(trace):
        action = event["action"]
        if action == ALLOC:
            block = Block(event["addr"], event["size"], event.get("frames") or [], event.get("user_metadata", ""), i)
            live[event["addr"]] = block
            total += block.size
        elif action == FREE_REQUESTED:
            block = live.pop(event["addr"], None)
            if block is not None:
                total -= block.size
        elif action == ANNOTATE:
            block = live.get(event["addr"])
            if block is not None:
                block.annotations.append(event.get("user_metadata", ""))
        allocated.append(total)
        if i in capture:
            captured[i] = list(live.values())
    reserved = None
    if reserved_end is not None:
        reserved = [0] * len(trace)
        current = reserved_end
        for i in range(len(trace) - 1, -1, -1):
            reserved[i] = current
            current -= RESERVED_DELTA.get(trace[i]["action"], 0) * trace[i].get("size", 0)
    return allocated, captured, reserved, start


def argmax_in(series, indices):
    """The index among `indices` with the largest value (the first on ties)."""
    best = None
    for i in indices:
        if best is None or series[i] > series[best]:
            best = i
    return best


def step_windows(trace):
    """Trace indices of every step, from the step stamped in each event's metadata.

    Untagged events belong to the step before them.
    """
    windows = defaultdict(list)
    current = None
    for i, event in enumerate(trace):
        step = step_of(event.get("user_metadata", ""))
        if step is not None:
            current = step
        if current is not None:
            windows[current].append(i)
    return dict(windows)


def prefix_window(trace, prefixes):
    """Indices of the events whose metadata starts with one of `prefixes`."""
    return [i for i, event in enumerate(trace) if event.get("user_metadata", "").startswith(tuple(prefixes))]


# Parts of the net, from the innermost frame inside the trainer tree that names one.
# A rule ending in "." matches every method of that class; `None` looks further out.
QUALNAME_PARTS = (
    ("RuleEncoder.encode_active", "rule encoder"),
    ("RuleEncoder.encode_catalog", "rule encoder"),
    ("RuleEncoder.prepare", "rule encoder"),
    ("RuleEncoder.position", "rule encoder"),
    ("RuleEncoder.freeze", "rule encoder"),
    ("RuleEncoder.read", "rule lookup"),
    ("RuleEncoder.forward", "rule lookup"),
    ("FrozenRules.", "rule lookup"),
    ("PlayerNet4.rule_features", "rule lookup"),
    ("RelationalProcessorV2.", "relational"),
    ("RelationalProcessor.", "relational"),
    ("PlayerNet5.relational_tokens", "relational"),
    ("PreNormTransformerLayer.", "trunk layers"),
    ("CandidateDecoderLayer.", "candidate decoder"),
    ("PlayerNet5._decode_candidates", "candidate decoder"),
    ("PlayerNet5.decode_candidates", "candidate decoder"),
    ("ThinkingStepBlock.", "thinking"),
    ("dense_thinking", "thinking"),
    ("compact_thinking", "thinking"),
    ("GatedResidualUpdate.", "recurrent memory"),
    ("PlayerNet4.observe_memory", "recurrent memory"),
    ("score_candidate_hidden", "candidate head"),
    ("PlayerNet4.candidate_hidden", "candidate head"),
    ("_score_cand_rows", "candidate head"),
    ("_score_cand_slice", "candidate head"),
    ("PlayerNet4.score_pairs", "pair head"),
    ("PlayerNet4.seat_values", "value head"),
    ("PlayerNet4.value_readout", "value head"),
    ("auxiliary_readout", "aux head"),
    ("ParameterAnchor.", "anchor penalty"),
    ("prefetch_batches", "inputs"),
    ("transfer_batch", "inputs"),
    ("DynamicMultiHeadAttention.", None),
    ("symlog", None),
    ("checkpoint_activation", None),
)

# Functions that do several parts' work: the statement at the frame's line decides.
# The first substring found wins; "" is the default.
STATEMENT_PARTS = {
    "PlayerNet4.encode_board": (
        ("checkpoint_activation(", "trunk layers"), ("layer(", "trunk layers"), ("trunk_", "trunk layers"),
        ("relational_tokens", "relational"), ("", "embeddings + edge messages")),
    "PlayerNet4.forward_with_seat_logits": (
        ("encode_board", "embeddings + edge messages"), ("observe_memory", "recurrent memory"),
        ("candidate_hidden", "candidate head"), ("score_candidate_hidden", "candidate head"),
        ("cand_logits", "candidate head"), ("score_pairs", "pair head"), ("pair_logits", "pair head"),
        ("scalar", "scalar head"), ("seat_values", "value head"), ("thinking", "thinking"),
        ("h_norm", "thinking"), ("z", "thinking"), ("", "readout glue")),
    "deferred_readouts": (
        ("to(device", "inputs"), ("scal", "scalar head"), ("score_pairs", "pair head"),
        ("seat_values", "value head"), ("observe_memory", "recurrent memory"),
        ("hidden = torch.where", "recurrent memory"), ("h_norm", "thinking"), ("thinking", "thinking"),
        ("z_all", "thinking"), ("board_slots", "thinking"), ("mem_stack", "thinking"),
        ("pair", "pair head"), ("ref_p", "pair head"), ("cand", "candidate head"), ("ref_c", "candidate head"),
        ("keys", "candidate head"), ("c_logits", "candidate head"), ("", "readout glue")),
    "encode_boards": (("to(device", "inputs"), ("pad(", "inputs"), ("", "board glue")),
    "main": (("to(device", "inputs"), ("h = ", "recurrent state"), ("aux", "aux head"), ("", "driver")),
}

FILE_PARTS = (
    ("training4/device_loss.py", "loss"),
    ("training4/loss_v5.py", "loss"),
    ("training4/loss.py", "loss"),
    ("training4/objective.py", "loss"),
    ("policy_math4.py", "loss"),
    ("training4/prefetch.py", "inputs"),
)


def classify_frame(rel, qualname, statement):
    """The part one trainer frame names, or None to keep looking outward."""
    rules = STATEMENT_PARTS.get(qualname)
    if rules is None and qualname.startswith("main.<locals>."):
        rules = STATEMENT_PARTS["main"]
    if rules is not None:
        for needle, part in rules:
            if needle in statement:
                return part
    for key, part in QUALNAME_PARTS:
        if (qualname.startswith(key) if key.endswith(".") else qualname == key):
            return part
    for name, part in FILE_PARTS:
        if rel == name:
            return part
    return None


class FrameClassifier:
    """Maps allocation stacks to parts of the net through the trainer's own source."""

    def __init__(self, root):
        self.root = os.path.realpath(root) + os.sep
        self._rel = {}
        self._files = {}
        self._lines = {}

    def relpath(self, filename):
        rel = self._rel.get(filename, False)
        if rel is False:
            real = os.path.realpath(filename)
            rel = real[len(self.root):] if real.startswith(self.root) else None
            self._rel[filename] = rel
        return rel

    def _index(self, rel):
        if rel not in self._files:
            try:
                source = Path(self.root, rel).read_text()
                tree = ast.parse(source)
            except (OSError, SyntaxError, ValueError):
                self._files[rel] = None
                return None
            functions = []

            def visit(node, prefix):
                for child in ast.iter_child_nodes(node):
                    if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        name = prefix + child.name
                        functions.append((child.lineno, child.end_lineno, name, child))
                        visit(child, name + ".<locals>.")
                    elif isinstance(child, ast.ClassDef):
                        visit(child, prefix + child.name + ".")
                    else:
                        visit(child, prefix)

            visit(tree, "")
            self._files[rel] = (source.splitlines(), functions)
        return self._files[rel]

    def locate(self, rel, line):
        """`(qualname, statement text)` of `line` in `rel`; `("", "")` when unknown."""
        key = (rel, line)
        if key in self._lines:
            return self._lines[key]
        index = self._index(rel)
        result = ("", "")
        if index is not None:
            lines, functions = index
            inside = [f for f in functions if f[0] <= line <= f[1]]
            if inside:
                start, end, name, node = min(inside, key=lambda f: f[1] - f[0])
                best = None
                for stmt in ast.walk(node):
                    if (isinstance(stmt, ast.stmt) and stmt is not node and stmt.lineno <= line <= stmt.end_lineno
                            and (best is None or stmt.end_lineno - stmt.lineno < best.end_lineno - best.lineno)):
                        best = stmt
                if best is None:
                    text = lines[line - 1] if 0 < line <= len(lines) else ""
                else:
                    body = getattr(best, "body", None)
                    last = body[0].lineno - 1 if body and isinstance(body, list) else best.end_lineno
                    text = "\n".join(lines[best.lineno - 1:max(last, best.lineno)])
                result = (name, text)
        self._lines[key] = result
        return result

    def part(self, frames):
        for frame in frames:
            rel = self.relpath(frame["filename"])
            if rel is None:
                continue
            name, statement = self.locate(rel, frame["line"])
            part = classify_frame(rel, name, statement)
            if part is not None:
                return part
        return None

    def site(self, frames):
        """The innermost trainer frame as `file:line function`, else the innermost frame."""
        for frame in frames:
            rel = self.relpath(frame["filename"])
            if rel is not None:
                return f"{rel}:{frame['line']} {frame['name']}"
        if frames:
            frame = frames[0]
            return f"{os.path.basename(frame['filename'])}:{frame['line']} {frame['name']}"
        return "(no Python frame)"


def _has_file(frames, fragment):
    return any(fragment in frame["filename"] for frame in frames)


def _compiled(frames):
    return any(("torchinductor" in f["filename"] or "/torch/_inductor/" in f["filename"]
                or "/torch/_functorch/" in f["filename"] or "/torch/_dynamo/" in f["filename"]) for f in frames)


ANNOTATION_KINDS = {"param": "parameters", "grad": "gradients", "adamw": "AdamW state",
                    "anchor": "L2-SP anchor copy", "buffer": "buffers"}
FORWARD_PHASES = {"rules_encode", "encode_boards", "readouts", "aux", "loss", "anchor_penalty", "forward",
                  "input_cast"}


def kind_and_part(block, classifier):
    """What a live block is (`kind`) and which part of the net made it (`part`)."""
    for note in reversed(block.annotations):
        head, _, rest = note.partition(":")
        if head in ANNOTATION_KINDS:
            return ANNOTATION_KINDS[head], rest.split(":")[0] or "?"
    frames = block.frames
    phase = phase_of(block.metadata)
    part = classifier.part(frames) if classifier is not None else None
    if part == "inputs" or phase == "inputs":
        return "inputs", part or "batch"
    if phase in FORWARD_PHASES:
        return "activations (forward)", part or phase
    if phase in ("backward", "backward_call"):
        if _has_file(frames, "/torch/utils/checkpoint.py"):
            return "backward: checkpoint recompute", part or ("compiled region" if _compiled(frames) else "?")
        if _compiled(frames):
            return "backward: compiled region", part or "trunk layers / thinking (compiled)"
        return "backward: autograd", part or "no Python frame"
    if phase == "clip":
        return "gradient clipping", "clip_grad_norm_"
    if phase == "optimizer":
        return "optimizer step", "AdamW"
    if block.index < 0:
        return "before the trace window", part or "?"
    return "other", part or (phase or "untagged")


def breakdown(blocks, classifier, top_sites=25):
    """Bytes by (kind, part), by kind, and the largest allocation sites."""
    rows = defaultdict(int)
    kinds = defaultdict(int)
    parts = defaultdict(int)
    sites = defaultdict(int)
    counts = defaultdict(int)
    for block in blocks:
        kind, part = kind_and_part(block, classifier)
        rows[(kind, part)] += block.size
        counts[(kind, part)] += 1
        kinds[kind] += block.size
        parts[part] += block.size
        site = classifier.site(block.frames) if classifier is not None else ""
        sites[(kind, part, site, block.metadata)] += block.size
    return {
        "total": sum(block.size for block in blocks),
        "blocks": len(blocks),
        "by_kind": dict(sorted(kinds.items(), key=lambda kv: -kv[1])),
        "by_part": dict(sorted(parts.items(), key=lambda kv: -kv[1])),
        "rows": [{"kind": k, "part": p, "bytes": b, "blocks": counts[(k, p)]}
                 for (k, p), b in sorted(rows.items(), key=lambda kv: -kv[1])],
        "sites": [{"kind": k, "part": p, "site": s, "metadata": m, "bytes": b}
                  for (k, p, s, m), b in sorted(sites.items(), key=lambda kv: -kv[1])[:top_sites]],
    }


def random_call(frames, rows, seed):
    """`rows` frames drawn at random: a call takes every step queued while the previous one ran."""
    return sorted(random.Random(seed).sample(range(frames), min(rows, frames)))


def mean_call(tokens, rows):
    """`rows` frames around the median size: eval4 sorts its queue by size before cutting calls."""
    order = sorted(range(len(tokens)), key=lambda i: (tokens[i], i))
    rows = min(rows, len(order))
    start = max(0, min(len(order) - rows, len(order) // 2 - rows // 2))
    return order[start:start + rows]


def budget_call(tokens, rows, budget):
    """The largest call of `rows` frames eval4 can form: the largest frames that still fit the budget."""
    order = sorted(range(len(tokens)), key=lambda i: (tokens[i], i))
    fitting = [i for i in order if tokens[i] * rows <= budget]
    return fitting[-rows:]


def table(rows, columns):
    """Fixed-width text table: `rows` are dicts, `columns` (key, title, format) triples."""
    cells = [[title for _, title, _ in columns]]
    for row in rows:
        cells.append([fmt(row.get(key)) if row.get(key) is not None else "" for key, _, fmt in columns])
    widths = [max(len(r[c]) for r in cells) for c in range(len(columns))]
    lines = ["  ".join(cell.rjust(w) if c else cell.ljust(w) for c, (cell, w) in enumerate(zip(r, widths)))
             for r in cells]
    return "\n".join([lines[0], "-" * len(lines[0]), *lines[1:]])


def mb(value):
    return f"{value / MB:.1f}"


# --------------------------------------------------------------------------- GPU helpers

def _use_trainer(trainer_dir):
    """Import only the frozen trainer source, never this tree's copy of the packages."""
    here = os.path.realpath(os.path.dirname(os.path.abspath(__file__)))
    sys.path[:] = [p for p in sys.path if os.path.realpath(p or os.getcwd()) != here]
    sys.path.insert(0, os.path.realpath(trainer_dir))


def _top(name):
    return name.split(".", 1)[0]


def _annotate(torch, tensor, label):
    """Label the allocation behind `tensor` in the trace; empty tensors own none."""
    if tensor is not None and tensor.is_cuda and tensor.untyped_storage().data_ptr():
        torch.cuda.memory._annotate_tensor(tensor, label)


def _snapshot_parts(snapshot):
    trace = snapshot["device_traces"][0] if snapshot.get("device_traces") else []
    final = [block for segment in snapshot["segments"] for block in segment["blocks"]
             if block["state"] == "active_allocated"]
    return trace, final


def _write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=1, sort_keys=False) + "\n")


@contextlib.contextmanager
def _fp16_graph(torch):
    """FP16 everywhere, as the FP16 ONNX converter makes the graph.

    The converter turns every float32 tensor, constant and cast into float16.
    PyTorch keeps explicit `.float()` and `dtype=torch.float32`, promotes integers
    to float32, and refuses `masked_fill(mask, -1e9)` on FP16. Map each to FP16
    (the fill clamped to FP16's range); shapes, and so memory, follow the graph.
    """
    fill, to_float, zeros_like = torch.Tensor.masked_fill, torch.Tensor.float, torch.zeros_like
    default = torch.get_default_dtype()
    limit = float(torch.finfo(torch.float16).max)

    def masked_fill(self, mask, value):
        if self.dtype == torch.float16 and isinstance(value, (int, float)) and abs(value) > limit:
            value = math.copysign(limit, value)
        return fill(self, mask, value)

    def half_zeros_like(tensor, *args, **kwargs):
        if kwargs.get("dtype") == torch.float32:
            kwargs["dtype"] = torch.float16
        return zeros_like(tensor, *args, **kwargs)

    torch.Tensor.masked_fill = masked_fill
    torch.Tensor.float = lambda self, *args, **kwargs: self.to(torch.float16)
    torch.zeros_like = half_zeros_like
    torch.set_default_dtype(torch.float16)
    try:
        yield
    finally:
        torch.set_default_dtype(default)
        torch.Tensor.masked_fill, torch.Tensor.float, torch.zeros_like = fill, to_float, zeros_like


@contextlib.contextmanager
def _exported_code_path(torch):
    """Run the branch the ONNX export traced (explicit attention scores, K = the
    graph's `export_thinking_steps`, no host-side trimming) eagerly."""
    original = torch.jit.is_tracing
    torch.jit.is_tracing = lambda: True
    try:
        yield
    finally:
        torch.jit.is_tracing = original


# --------------------------------------------------------------------------- train

class StopMeasurement(Exception):
    """Raised where the driver would start its final validation."""


class TrainProbe:
    def __init__(self, torch, opts):
        self.torch = torch
        self.opts = opts
        self.step = 0
        self.steps = []
        self.rules = []
        self.shapes = {}
        self.static = None
        self.names = None
        self.anchor = None
        self.adamw_annotated = False
        self.started = time.perf_counter()
        self.tag("setup")

    def tag(self, label):
        self.torch.cuda.memory._set_memory_metadata(label)

    def phase(self, name, function):
        def wrapped(*args, **kwargs):
            self.tag(f"s{self.step}:{name}")
            return function(*args, **kwargs)
        return wrapped

    def annotate(self, tensor, label):
        _annotate(self.torch, tensor, label)

    def first_update(self, net, optimizer):
        torch = self.torch
        self.names = {p: name for name, p in net.named_parameters()}
        for name, p in net.named_parameters():
            self.annotate(p, f"param:{_top(name)}")
        for name, b in net.named_buffers():
            self.annotate(b, f"buffer:{_top(name)}")
        if self.anchor is not None:
            for p, start in self.anchor.pairs:
                self.annotate(start, f"anchor:{_top(self.names[p])}")

        def before_step(*_):
            self.tag(f"s{self.step}:optimizer")

        def after_step(opt, *_):
            if not self.adamw_annotated:
                for group in opt.param_groups:
                    for p in group["params"]:
                        for key, value in opt.state.get(p, {}).items():
                            if isinstance(value, torch.Tensor):
                                self.annotate(value, f"adamw:{_top(self.names[p])}:{key}")
                self.adamw_annotated = True
                self.static = self.static_accounting(net, opt)

        optimizer.register_step_pre_hook(before_step)
        optimizer.register_step_post_hook(after_step)

    def static_accounting(self, net, optimizer):
        """Exact bytes per top-level module: parameters, gradients, AdamW, anchor, buffers."""
        torch = self.torch
        modules = defaultdict(lambda: defaultdict(int))
        for name, p in net.named_parameters():
            row = modules[_top(name)]
            size = p.numel() * p.element_size()
            row["parameters"] += size
            row["params"] += p.numel()
            if p.requires_grad:
                row["gradients"] += size
            for value in optimizer.state.get(p, {}).values():
                if isinstance(value, torch.Tensor) and value.is_cuda:
                    row["adamw"] += value.numel() * value.element_size()
        if self.anchor is not None:
            for p, start in self.anchor.pairs:
                modules[_top(self.names[p])]["anchor"] += start.numel() * start.element_size()
        for name, b in net.named_buffers():
            if b.is_cuda:
                modules[_top(name)]["buffers"] += b.numel() * b.element_size()
        return {name: dict(row) for name, row in modules.items()}


def run_train(opts, trainer_argv):
    _use_trainer(opts.trainer_dir)
    import torch
    out = Path(opts.out)
    out.mkdir(parents=True, exist_ok=True)
    torch.cuda.memory._record_memory_history(enabled="all", context="alloc", stacks="python",
                                             max_entries=opts.trace_entries)
    probe = TrainProbe(torch, opts)

    import sampler4
    import model4.player as player
    import training4.checkpoint_async as checkpoint_async
    import training4.driver as driver
    import training4.rules_only as rules_only
    import training4.supervised_options as supervised_options

    # Start at batch `--skip-batches` of the epoch's fixed order (seed and epoch unchanged).
    original_iter = sampler4.StatefulBatchSampler4.__iter__

    def offset_iter(self):
        if self.shuffle and opts.skip_batches:
            yield from self._build_batches()[getattr(self, "start_batch", 0) + opts.skip_batches:]
        else:
            yield from original_iter(self)

    sampler4.StatefulBatchSampler4.__iter__ = offset_iter

    def stop(*_args, **_kwargs):
        raise StopMeasurement

    driver.evaluate_depths = stop
    checkpoint_async.AsyncCheckpointWriter.save = lambda self, payload, path: None

    original_encode_boards = driver.encode_boards

    def encode_boards(net, batch, chunk, device, *rest, **kw):
        probe.shapes = {name: list(batch[name].shape) for name in
                        ("entity_feat", "edge_index", "cand_feat", "pair_index", "entity_rules", "cand_rules")}
        probe.shapes["decisions"] = int((batch["is_decision"] & batch["step_valid"]).sum())
        return original_encode_boards(net, batch, chunk, device, *rest, **kw)

    driver.encode_boards = probe.phase("encode_boards", encode_boards)
    driver.deferred_readouts = probe.phase("readouts", driver.deferred_readouts)
    driver.compute_unroll_device_loss = probe.phase("loss", driver.compute_unroll_device_loss)
    supervised_options.auxiliary_readout = probe.phase("aux", supervised_options.auxiliary_readout)
    rules_only.ParameterAnchor.penalty = probe.phase("anchor_penalty", rules_only.ParameterAnchor.penalty)
    original_anchor_init = rules_only.ParameterAnchor.__init__

    def anchor_init(self, net, weight):
        original_anchor_init(self, net, weight)
        probe.anchor = self

    rules_only.ParameterAnchor.__init__ = anchor_init

    original_prepare = player.PlayerNet4.prepare_structure

    def prepare_structure(self, active_refs=None):
        probe.tag(f"s{probe.step}:rules_encode")
        torch.cuda.synchronize()
        before = torch.cuda.memory_allocated()
        started = time.perf_counter()
        original_prepare(self, active_refs)
        torch.cuda.synchronize()
        table_ = self.rules._table
        probe.rules.append({
            "step": probe.step, "seconds": time.perf_counter() - started,
            "retained_bytes": torch.cuda.memory_allocated() - before,
            "table_rows": int(table_.shape[0]), "table_bytes": table_.numel() * table_.element_size(),
            "table_dtype": str(table_.dtype)})

    player.PlayerNet4.prepare_structure = prepare_structure

    original_clip = torch.nn.utils.clip_grad_norm_

    def clip_grad_norm_(parameters, *args, **kwargs):
        parameters = list(parameters)
        probe.tag(f"s{probe.step}:clip")
        for p in parameters:
            if p.grad is not None:
                probe.annotate(p.grad, f"grad:{_top(probe.names[p])}")
        return original_clip(parameters, *args, **kwargs)

    torch.nn.utils.clip_grad_norm_ = clip_grad_norm_

    original_update = driver.update_parameters

    def update_parameters(loss, net, optimizer, *rest):
        if probe.names is None:
            probe.first_update(net, optimizer)
        step = probe.step
        probe.tag(f"s{step}:backward_call")
        if loss.requires_grad:
            # Runs on autograd's device thread, where the backward allocates.
            loss.register_hook(lambda grad: probe.tag(f"s{step}:backward"))
        updated = original_update(loss, net, optimizer, *rest)
        torch.cuda.synchronize()
        free, total = torch.cuda.mem_get_info()
        now = time.perf_counter()
        probe.steps.append({
            "step": step, "batch": opts.skip_batches + step, "updated": bool(updated),
            "seconds": now - probe.started, "peak_allocated": torch.cuda.max_memory_allocated(),
            "peak_reserved": torch.cuda.max_memory_reserved(), "allocated": torch.cuda.memory_allocated(),
            "reserved": torch.cuda.memory_reserved(), "device_used": total - free, "shapes": probe.shapes})
        print(f"[vram] step={step} batch={opts.skip_batches + step} "
              f"peak_allocated={torch.cuda.max_memory_allocated() / MB:.1f}MB "
              f"peak_reserved={torch.cuda.max_memory_reserved() / MB:.1f}MB "
              f"device_used={(total - free) / MB:.1f}MB {now - probe.started:.1f}s shapes={probe.shapes}", flush=True)
        torch.cuda.reset_peak_memory_stats()
        probe.started = now
        probe.step += 1
        probe.tag(f"s{probe.step}:between")
        return updated

    driver.update_parameters = update_parameters

    sys.argv = ["train5.py", *trainer_argv]
    try:
        driver.main(default_architecture="v5")
        raise RuntimeError("The trainer finished without reaching its final validation")
    except StopMeasurement:
        pass
    torch.cuda.synchronize()
    free, total = torch.cuda.mem_get_info()
    probe_record = {
        "mode": "train", "trainer_dir": os.path.realpath(opts.trainer_dir), "trainer_argv": trainer_argv,
        "skip_batches": opts.skip_batches, "torch": torch.__version__,
        "alloc_conf": os.environ.get("PYTORCH_CUDA_ALLOC_CONF", ""),
        "reserved_end": torch.cuda.memory_reserved(), "allocated_end": torch.cuda.memory_allocated(),
        "device_used_end": total - free, "device_total": total,
        "memory_stats": {k: v for k, v in torch.cuda.memory_stats().items()
                         if k.startswith(("num_", "allocated_bytes.all", "reserved_bytes.all"))},
        "steps": probe.steps, "rules": probe.rules, "static": probe.static,
    }
    snapshot = torch.cuda.memory._snapshot()
    torch.cuda.memory._record_memory_history(enabled=None)
    _write_json(out / "probe.json", probe_record)
    if opts.keep_snapshot:
        with open(out / "snapshot.pickle", "wb") as handle:
            pickle.dump(snapshot, handle, protocol=pickle.HIGHEST_PROTOCOL)
    report = analyze_train(snapshot, probe_record, FrameClassifier(opts.trainer_dir))
    _write_json(out / "report.json", report)
    (out / "report.txt").write_text(render_train(report))
    print(render_train(report), flush=True)


def analyze_train(snapshot, probe_record, classifier):
    trace, final = _snapshot_parts(snapshot)
    allocated, _, _, start = replay(trace, final)
    windows = step_windows(trace)
    steps = {s["step"]: s for s in probe_record["steps"]}
    peaks = {}
    for step, indices in windows.items():
        if step in steps and indices:
            peaks[step] = argmax_in(allocated, indices)
    if not peaks:
        raise ValueError("No step-tagged events in the trace")
    global_step = max(peaks, key=lambda s: allocated[peaks[s]])
    ordered = sorted(peaks, key=lambda s: allocated[peaks[s]])
    typical_step = ordered[len(ordered) // 2]
    wanted = {"peak": peaks[global_step], "typical": peaks[typical_step]}
    allocated, captured, reserved, _ = replay(trace, final, capture=set(wanted.values()),
                                              reserved_end=probe_record["reserved_end"])
    report = {"mode": "train", "trace_events": len(trace), "trace_start_bytes": start,
              "trace_window_truncated": start > 0, "static": probe_record["static"],
              "rules": probe_record["rules"], "steps": probe_record["steps"],
              "step_peaks_replayed": {str(s): allocated[i] for s, i in sorted(peaks.items())}}
    for label, index in wanted.items():
        step = global_step if label == "peak" else typical_step
        result = breakdown(captured[index], classifier)
        result.update({"step": step, "batch": steps[step]["batch"], "trace_index": index,
                       "metadata": trace[index].get("user_metadata", ""),
                       "allocated": allocated[index], "reserved_at_peak": reserved[index],
                       "fragmentation": reserved[index] - allocated[index],
                       "torch_peak_allocated": steps[step]["peak_allocated"],
                       "torch_peak_reserved": steps[step]["peak_reserved"],
                       "device_used_after_step": steps[step]["device_used"],
                       "shapes": steps[step]["shapes"],
                       "rules_encode": next((r for r in probe_record["rules"] if r["step"] == step), None)})
        report[label] = result
    return report


def render_static(static):
    rows = []
    totals = defaultdict(int)
    for name, row in sorted(static.items(), key=lambda kv: -kv[1].get("parameters", 0) - kv[1].get("buffers", 0)):
        entry = {"module": name, **row}
        entry["total"] = sum(row.get(k, 0) for k in ("parameters", "gradients", "adamw", "anchor", "buffers"))
        rows.append(entry)
        for key in ("params", "parameters", "gradients", "adamw", "anchor", "buffers", "total"):
            totals[key] += entry.get(key, 0)
    rows.append({"module": "TOTAL", **totals})
    return table(rows, [("module", "module", str), ("params", "params", lambda v: f"{v:,}"),
                        ("parameters", "weights MB", mb), ("gradients", "grads MB", mb),
                        ("adamw", "AdamW MB", mb), ("anchor", "anchor MB", mb),
                        ("buffers", "buffers MB", mb), ("total", "total MB", mb)])


def render_breakdown(result, title):
    lines = [f"## {title}: step {result['step']} (batch {result['batch']}), phase `{result['metadata']}`",
             f"replayed allocated {mb(result['allocated'])} MB (torch max_memory_allocated for the step "
             f"{mb(result['torch_peak_allocated'])} MB); reserved at that moment {mb(result['reserved_at_peak'])} MB, "
             f"fragmentation (reserved - allocated) {mb(result['fragmentation'])} MB; device used after the step "
             f"{mb(result['device_used_after_step'])} MB", f"shapes: {result['shapes']}"]
    if result.get("rules_encode"):
        r = result["rules_encode"]
        lines.append(f"rule encoder this step: {r['table_rows']} closure rows, table {mb(r['table_bytes'])} MB "
                     f"{r['table_dtype']}, retained after encode {mb(r['retained_bytes'])} MB, {r['seconds'] * 1e3:.1f} ms")
    lines.append("")
    lines.append(table([{"kind": k, "bytes": v} for k, v in result["by_kind"].items()],
                       [("kind", "kind", str), ("bytes", "MB", mb)]))
    lines.append("")
    lines.append(table(result["rows"], [("kind", "kind", str), ("part", "part", str), ("bytes", "MB", mb),
                                        ("blocks", "blocks", str)]))
    lines.append("")
    lines.append(table(result["sites"], [("kind", "kind", str), ("part", "part", str), ("site", "site", str),
                                         ("metadata", "phase", str), ("bytes", "MB", mb)]))
    return "\n".join(lines)


def render_train(report):
    parts = ["# train5 VRAM by part", ""]
    if report["static"]:
        parts += ["## Static, exact", render_static(report["static"]), ""]
    parts += ["## Steps", table(report["steps"], [("step", "step", str), ("batch", "batch", str),
                                                ("peak_allocated", "peak alloc MB", mb),
                                                ("peak_reserved", "peak reserved MB", mb),
                                                ("device_used", "device used MB", mb),
                                                ("seconds", "s", lambda v: f"{v:.2f}")]), ""]
    parts += [table(report["rules"], [("step", "step", str), ("table_rows", "closure rows", str),
                                      ("table_bytes", "table MB", mb), ("retained_bytes", "retained MB", mb),
                                      ("seconds", "encode ms", lambda v: f"{v * 1e3:.1f}")]), ""]
    for label in ("peak", "typical"):
        parts += [render_breakdown(report[label], label), ""]
    parts.append(f"trace events {report['trace_events']}, bytes live before the window {mb(report['trace_start_bytes'])} MB")
    return "\n".join(parts) + "\n"


# --------------------------------------------------------------------------- infer

class ModuleWorkingSets:
    """Every hooked module's own peak above its entry level, and its output."""

    def __init__(self, torch, net):
        self.torch = torch
        self.stats = {}
        self.stack = []
        self.handles = []
        for name, child in net.named_children():
            if isinstance(child, torch.nn.ModuleList):
                targets = [(name, layer) for layer in child]
            elif name == "cand_scorer":  # called piece by piece, never whole
                targets = [(f"{name}.{i}", layer) for i, layer in enumerate(child)]
            else:
                targets = [(name, child)]
            for key, module in targets:
                self.handles.append(module.register_forward_pre_hook(self._pre(key)))
                self.handles.append(module.register_forward_hook(self._post(key)))

    def _pre(self, key):
        def hook(module, args):
            self.torch.cuda.reset_peak_memory_stats()
            self.stack.append(self.torch.cuda.memory_allocated())
        return hook

    def _post(self, key):
        def hook(module, args, output):
            entry = self.stack.pop()
            stats = self.stats.setdefault(key, {"calls": 0, "working_set": 0, "output": 0})
            stats["calls"] += 1
            stats["working_set"] = max(stats["working_set"], self.torch.cuda.max_memory_allocated() - entry)
            stats["output"] = max(stats["output"], self.torch.cuda.memory_allocated() - entry)
        return hook

    def remove(self):
        for handle in self.handles:
            handle.remove()


def load_rows(D4, data_dir, encoder_version, limit, unroll):
    """Decision frames of the validation split, each at its own extents."""
    dataset = D4.TrajectoryDataset4(Path(data_dir), encoder_version=encoder_version, split="val", unroll_len=unroll)
    rows = []
    for segment in dataset.segments:
        for row in segment:
            frame = dataset.encode_single_step(int(row))
            if bool(frame["is_decision"]):
                rows.append(frame)
                if len(rows) >= limit:
                    return dataset, rows
    return dataset, rows


def call_inputs(torch, D4, M, frames, k, d_model):
    """Rows padded to the call's widest axes, as `eval4`'s `run_rows` pads them."""
    tensors = []
    for name in M.INPUT_NAMES[:-2]:
        tensors.append(torch.from_numpy(D4.padded_stack([frame[name] for frame in frames])))
    b = len(frames)
    tensors.append(torch.zeros(b, M.RECURRENT_SLOTS, d_model, dtype=torch.float32))
    tensors.append(torch.full((b,), k, dtype=torch.int64))
    return tensors


def run_infer(opts):
    _use_trainer(opts.trainer_dir)
    import torch
    out = Path(opts.out)
    out.mkdir(parents=True, exist_ok=True)
    torch.cuda.init()
    torch.empty(1, device="cuda")
    torch.cuda.synchronize()
    free, total = torch.cuda.mem_get_info()
    context_used = total - free
    torch.cuda.memory._record_memory_history(enabled="all", context="alloc", stacks="python",
                                             max_entries=opts.trace_entries)
    tag = torch.cuda.memory._set_memory_metadata
    import dataset4 as D4
    import model4 as M
    from export_onnx4 import load_export_checkpoint
    tag("setup")
    _, net, _ = load_export_checkpoint(Path(opts.checkpoint))
    net.rules = net.rules.freeze()
    net.export_thinking_steps = opts.thinking_steps
    net = net.half().eval()
    tag("weights")
    net.cuda()
    static = defaultdict(lambda: defaultdict(int))
    for name, p in net.named_parameters():
        _annotate(torch, p, f"param:{_top(name)}")
        static[_top(name)]["weights"] += p.numel() * p.element_size()
        static[_top(name)]["params"] += p.numel()
    for name, b in net.named_buffers():
        _annotate(torch, b, f"buffer:{_top(name)}")
        static[_top(name)]["weights"] += b.numel() * b.element_size()
        static[_top(name)]["params"] += b.numel()
    static = {k: dict(v) for k, v in static.items()}
    tag("rows")
    _, frames = load_rows(D4, opts.data, opts.encoder_version, opts.frames, opts.unroll)
    tokens = [int(f["entity_feat"].shape[0]) + int(f["cand_feat"].shape[0]) for f in frames]
    # A call holds every step queued while the previous call ran, sorted by size
    # and cut at the row and token limits: up to the limits, a random sample of
    # decision frames padded to its widest axes. `budget` is the bound: the
    # largest frames that still fit `rows x 512` tokens.
    calls = {"mean": random_call(len(frames), opts.mean_rows, 63001),
             "max": random_call(len(frames), opts.max_rows, 63002),
             "budget": budget_call(tokens, opts.max_rows, opts.token_budget)}
    record = {"mode": "infer", "trainer_dir": os.path.realpath(opts.trainer_dir), "checkpoint": opts.checkpoint,
              "torch": torch.__version__, "thinking_steps": opts.thinking_steps, "frames": len(frames),
              "frame_tokens": {"median": statistics.median(tokens), "max": max(tokens),
                               "mean": statistics.fmean(tokens)},
              "cuda_context_and_first_block": context_used, "static": static, "cases": {}}
    with _fp16_graph(torch), _exported_code_path(torch), torch.inference_mode():
        for name, picked in calls.items():
            chosen = [frames[i] for i in picked]
            inputs = call_inputs(torch, D4, M, chosen, opts.thinking_steps, net.cfg.d_model)
            shape = {"rows": len(chosen), "E": int(inputs[0].shape[1]), "edges": int(inputs[5].shape[1]),
                     "C": int(inputs[8].shape[1]), "P": int(inputs[12].shape[1]),
                     "entity_rule_refs": int(inputs[15].shape[2]), "cand_rule_refs": int(inputs[16].shape[2]),
                     "row_tokens_max": max(tokens[i] for i in picked)}
            shape["padded_tokens"] = shape["rows"] * (shape["E"] + shape["C"])
            for run in ("clean", "modules"):
                torch.cuda.synchronize()
                tag(f"{name}:inputs" if run == "clean" else f"{name}-modules:inputs")
                gpu = [t.cuda() for t in inputs]
                torch.cuda.synchronize()
                torch.cuda.reset_peak_memory_stats()
                before = torch.cuda.memory_allocated()
                hooks = ModuleWorkingSets(torch, net) if run == "modules" else None
                tag(f"{name}:input_cast" if run == "clean" else f"{name}-modules:input_cast")
                started = time.perf_counter()
                # The FP16 graph keeps FP32 inputs and casts them inside.
                cast = [t.half() if t.is_floating_point() else t for t in gpu]
                tag(f"{name}:forward" if run == "clean" else f"{name}-modules:forward")
                outputs = net(*cast)
                torch.cuda.synchronize()
                seconds = time.perf_counter() - started
                if run == "clean":
                    shape.update({"seconds": seconds, "inputs_bytes": sum(t.numel() * t.element_size() for t in gpu),
                                  "peak_allocated": torch.cuda.max_memory_allocated(),
                                  "allocated_before": before, "peak_above_before":
                                      torch.cuda.max_memory_allocated() - before,
                                  "outputs_bytes": sum(t.numel() * t.element_size() for t in outputs)})
                else:
                    hooks.remove()
                    shape["modules"] = hooks.stats
                del outputs, cast, gpu
                tag("idle")
            record["cases"][name] = shape
            print(f"[vram] infer {name}: {json.dumps(shape)}", flush=True)
    torch.cuda.synchronize()
    record["reserved_end"] = torch.cuda.memory_reserved()
    snapshot = torch.cuda.memory._snapshot()
    torch.cuda.memory._record_memory_history(enabled=None)
    _write_json(out / "probe.json", record)
    if opts.keep_snapshot:
        with open(out / "snapshot.pickle", "wb") as handle:
            pickle.dump(snapshot, handle, protocol=pickle.HIGHEST_PROTOCOL)
    report = analyze_infer(snapshot, record, FrameClassifier(opts.trainer_dir))
    _write_json(out / "report.json", report)
    (out / "report.txt").write_text(render_infer(report))
    print(render_infer(report), flush=True)


def analyze_infer(snapshot, record, classifier):
    trace, final = _snapshot_parts(snapshot)
    allocated, _, _, _ = replay(trace, final)
    wanted = {}
    for name in record["cases"]:
        indices = prefix_window(trace, (f"{name}:inputs", f"{name}:input_cast", f"{name}:forward"))
        if indices:
            wanted[name] = argmax_in(allocated, indices)
    allocated, captured, reserved, _ = replay(trace, final, capture=set(wanted.values()),
                                              reserved_end=record["reserved_end"])
    report = {"mode": "infer", "static": record["static"], "cases": {}, "frames": record["frames"],
              "frame_tokens": record["frame_tokens"], "thinking_steps": record["thinking_steps"],
              "cuda_context_and_first_block": record["cuda_context_and_first_block"]}
    for name, index in wanted.items():
        result = breakdown(captured[index], classifier)
        result.update({"call": record["cases"][name], "trace_index": index, "metadata": trace[index].get("user_metadata", ""),
                       "allocated": allocated[index], "reserved_at_peak": reserved[index]})
        report["cases"][name] = result
    return report


def render_infer(report):
    lines = ["# FP16 inference VRAM by part (PyTorch, the exported graph's code path)", ""]
    weights = [{"module": k, **v} for k, v in sorted(report["static"].items(), key=lambda kv: -kv[1]["weights"])]
    weights.append({"module": "TOTAL", "params": sum(r["params"] for r in weights),
                    "weights": sum(r["weights"] for r in weights)})
    lines += [table(weights, [("module", "module", str), ("params", "elements", lambda v: f"{v:,}"),
                              ("weights", "FP16 MB", mb)]), ""]
    for name, case in report["cases"].items():
        call = case["call"]
        lines.append(f"## {name}: {call['rows']} rows, E {call['E']}, C {call['C']}, edges {call['edges']}, "
                     f"P {call['P']}, padded tokens {call['padded_tokens']}, {call['seconds'] * 1e3:.1f} ms")
        lines.append(f"peak allocated {mb(case['allocated'])} MB (torch {mb(call['peak_allocated'])} MB), "
                     f"reserved {mb(case['reserved_at_peak'])} MB, inputs {mb(call['inputs_bytes'])} MB, "
                     f"phase at peak `{case['metadata']}`")
        lines.append(table([{"kind": k, "bytes": v} for k, v in case["by_kind"].items()],
                           [("kind", "kind", str), ("bytes", "MB", mb)]))
        lines.append("")
        lines.append(table(case["rows"], [("kind", "kind", str), ("part", "part", str), ("bytes", "MB", mb),
                                          ("blocks", "blocks", str)]))
        lines.append("")
        modules = [{"module": k, **v} for k, v in sorted(call.get("modules", {}).items(),
                                                         key=lambda kv: -kv[1]["working_set"])]
        lines.append(table(modules, [("module", "module (own working set)", str), ("calls", "calls", str),
                                     ("working_set", "peak above entry MB", mb), ("output", "output MB", mb)]))
        lines.append("")
        lines.append(table(case["sites"], [("kind", "kind", str), ("part", "part", str), ("site", "site", str),
                                           ("bytes", "MB", mb)]))
        lines.append("")
    return "\n".join(lines) + "\n"


# --------------------------------------------------------------------------- rules

def run_rules(opts):
    _use_trainer(opts.trainer_dir)
    import torch
    import dataset4 as D4
    from export_onnx4 import load_export_checkpoint
    from sampler4 import StatefulBatchSampler4
    out = Path(opts.out)
    out.mkdir(parents=True, exist_ok=True)
    _, net, _ = load_export_checkpoint(Path(opts.checkpoint))
    rules = net.rules.cuda().train()
    dataset = D4.TrajectoryDataset4(Path(opts.data), encoder_version=opts.encoder_version, split="train",
                                    unroll_len=opts.unroll)
    sampler = StatefulBatchSampler4(dataset, batch_size=opts.batch_size, shuffle=True, seed=opts.seed)
    batches = sampler._build_batches()[opts.skip_batches:opts.skip_batches + opts.batches]

    def measure(label, encode, backward=True):
        rules.clear()
        rules.zero_grad(set_to_none=True)
        torch.cuda.synchronize()
        base = torch.cuda.memory_allocated()
        torch.cuda.reset_peak_memory_stats()
        start_event, end_event = torch.cuda.Event(enable_timing=True), torch.cuda.Event(enable_timing=True)
        started = time.perf_counter()
        start_event.record()
        with torch.autocast("cuda", dtype=torch.bfloat16):
            encode()
        end_event.record()
        torch.cuda.synchronize()
        result = {"case": label, "forward_ms": (time.perf_counter() - started) * 1e3,
                  "forward_gpu_ms": start_event.elapsed_time(end_event),
                  "forward_peak_above": torch.cuda.max_memory_allocated() - base,
                  "retained": torch.cuda.memory_allocated() - base}
        table_ = rules._table
        result.update({"rows": int(table_.shape[0]), "table_bytes": table_.numel() * table_.element_size(),
                       "table_dtype": str(table_.dtype)})
        if backward and table_.requires_grad:
            grad = torch.randn_like(table_)
            torch.cuda.synchronize()
            torch.cuda.reset_peak_memory_stats()
            started = time.perf_counter()
            table_.backward(grad)
            torch.cuda.synchronize()
            result.update({"backward_ms": (time.perf_counter() - started) * 1e3,
                           "backward_peak_above": torch.cuda.max_memory_allocated() - base,
                           "grad_bytes": sum(p.grad.numel() * p.grad.element_size()
                                             for p in rules.parameters() if p.grad is not None)})
            del grad
        del table_
        rules.clear()
        rules.zero_grad(set_to_none=True)
        torch.cuda.synchronize()
        return result

    results = []
    batch_refs = []
    for i, indices in enumerate(batches):
        host = D4.collate_trajectories([dataset[j] for j in indices])
        batch_refs.append((opts.skip_batches + i, (host["entity_rules"], host["cand_rules"])))
    for repeat in range(2):  # the first pass, one batch, only warms kernels
        for batch, refs in (batch_refs if repeat else batch_refs[:1]):
            roots = int(torch.unique(torch.cat([refs[0][..., 0].reshape(-1), refs[1][..., 0].reshape(-1)])).numel())
            row = measure("active closure", lambda: rules.prepare(refs))
            row.update({"batch": batch, "repeat": repeat, "referenced_ids": roots})
            results.append(row)
            print(f"[vram] rules {json.dumps(row)}", flush=True)
        row = measure("whole catalog", lambda: rules.prepare(None))
        row["repeat"] = repeat
        results.append(row)
        print(f"[vram] rules {json.dumps(row)}", flush=True)

    def frozen():
        with torch.no_grad():
            rules._table = rules.encode_catalog()

    for repeat in range(2):
        row = measure("whole catalog, frozen (no grad)", frozen, backward=False)
        row["repeat"] = repeat
        results.append(row)
        print(f"[vram] rules {json.dumps(row)}", flush=True)
    rule_params = sum(p.numel() for p in rules.parameters())
    report = {"mode": "rules", "nodes": len(rules.data["nodes"]), "levels": rules.levels,
              "roots": len(rules.data.get("roots", {})), "width": rules.width,
              "parameters": rule_params, "parameter_bytes_fp32": rule_params * 4,
              "buffer_bytes": sum(b.numel() * b.element_size() for b in rules.buffers()),
              "frozen_table_bytes_fp32": len(rules.data["nodes"]) * rules.width * 4,
              "frozen_table_bytes_fp16": len(rules.data["nodes"]) * rules.width * 2,
              "results": results}
    _write_json(out / "report.json", report)
    measured = [r for r in results if r["repeat"] == 1]
    text = ["# Rule encoder: active closure vs whole catalog (bf16 autocast, as train5)", "",
            f"nodes {report['nodes']}, levels {report['levels']}, roots {report['roots']}, width {report['width']}, "
            f"parameters {rule_params:,} ({mb(rule_params * 4)} MB fp32), index buffers {mb(report['buffer_bytes'])} MB, "
            f"frozen table {mb(report['frozen_table_bytes_fp32'])} MB fp32 / {mb(report['frozen_table_bytes_fp16'])} MB fp16",
            "", table(measured, [("case", "case", str), ("batch", "batch", str), ("referenced_ids", "ids", str),
                                 ("rows", "rows", str), ("table_bytes", "table MB", mb),
                                 ("retained", "retained MB", mb), ("forward_peak_above", "fwd peak MB", mb),
                                 ("backward_peak_above", "bwd peak MB", mb),
                                 ("forward_ms", "fwd ms", lambda v: f"{v:.1f}"),
                                 ("forward_gpu_ms", "fwd GPU ms", lambda v: f"{v:.1f}"),
                                 ("backward_ms", "bwd ms", lambda v: f"{v:.1f}")])]
    (out / "report.txt").write_text("\n".join(text) + "\n")
    print("\n".join(text), flush=True)


# --------------------------------------------------------------------------- main

def parse(argv=None):
    """Options; for `train`, `trainer_argv` holds everything after `--`."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="mode", required=True)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--trainer-dir", required=True, help="Frozen trainer source to import")
    common.add_argument("--out", required=True)
    common.add_argument("--trace-entries", type=int, default=3_000_000)
    common.add_argument("--keep-snapshot", action="store_true")
    train = sub.add_parser("train", parents=[common], help="train5 updates; trainer arguments follow `--`")
    train.add_argument("--skip-batches", type=int, default=0)
    train.add_argument("trainer_argv", nargs=argparse.REMAINDER)
    data = argparse.ArgumentParser(add_help=False)
    data.add_argument("--checkpoint", required=True)
    data.add_argument("--data", required=True)
    data.add_argument("--encoder-version", type=int, default=10)
    data.add_argument("--unroll", type=int, default=32)
    infer = sub.add_parser("infer", parents=[common, data])
    infer.add_argument("--thinking-steps", type=int, default=2)
    infer.add_argument("--frames", type=int, default=4000)
    infer.add_argument("--mean-rows", type=int, default=17)
    infer.add_argument("--max-rows", type=int, default=96)
    infer.add_argument("--token-budget", type=int, default=96 * 512)
    rules = sub.add_parser("rules", parents=[common, data])
    rules.add_argument("--batch-size", type=int, default=8)
    rules.add_argument("--seed", type=int, default=62101)
    rules.add_argument("--skip-batches", type=int, default=0)
    rules.add_argument("--batches", type=int, default=4)
    analyze = sub.add_parser("analyze", parents=[common])
    analyze.add_argument("--snapshot", required=True)
    analyze.add_argument("--probe", required=True)
    opts = parser.parse_args(argv)
    if opts.mode == "train" and opts.trainer_argv[:1] == ["--"]:
        opts.trainer_argv = opts.trainer_argv[1:]
    return opts


def main():
    opts = parse()
    if opts.mode == "train":
        run_train(opts, opts.trainer_argv)
    elif opts.mode == "infer":
        run_infer(opts)
    elif opts.mode == "rules":
        run_rules(opts)
    else:
        with open(opts.snapshot, "rb") as handle:
            snapshot = pickle.load(handle)
        record = json.loads(Path(opts.probe).read_text())
        classifier = FrameClassifier(opts.trainer_dir)
        out = Path(opts.out)
        out.mkdir(parents=True, exist_ok=True)
        if record["mode"] == "train":
            report = analyze_train(snapshot, record, classifier)
            text = render_train(report)
        else:
            report = analyze_infer(snapshot, record, classifier)
            text = render_infer(report)
        _write_json(out / "report.json", report)
        (out / "report.txt").write_text(text)
        print(text)


if __name__ == "__main__":
    main()
