"""CPU scheduling metadata and bounded transfers for recurrent GPU training.

The host already owns masks. Reading them here avoids CUDA scalar/nonzero
synchronization without changing any encoded choices or the memory clock.
"""
from dataclasses import dataclass
import numpy as np
import torch
import model4 as M
from .depth_schedule import DepthSchedule


@dataclass(frozen=True)
class StepPlan:
    candidates: int
    pairs: int
    thinking: int
    valid: bool
    heads: tuple[bool, ...]
    decision_rows: tuple[int, ...] | None = None

    @property
    def hint(self):
        return (self.candidates, self.pairs, self.thinking)


def plan_batch(batch, step_num, thinking_steps=1, curriculum=False, schedule=None):
    """Compute exact per-frame extents, head presence and K before H2D."""
    if batch["step_valid"].device.type != "cpu":
        raise ValueError("Execution plans must be computed before GPU transfer")
    schedule = schedule or DepthSchedule.from_training(thinking_steps, curriculum)
    valid = batch["step_valid"].numpy()
    decision = batch["is_decision"].numpy()
    heads = batch["head"].numpy()
    candidates = batch["cand_mask"].numpy()
    pairs = batch["pair_mask"].numpy()

    cand_any = candidates.any(axis=0)
    pair_any = pairs.any(axis=0)
    valid_dec = valid & decision
    dec_any = decision.any(axis=0)
    val_any = valid.any(axis=0)
    head_any = tuple((valid_dec & (heads == h)).any(axis=0) for h in range(4))

    result = []
    for t in range(valid.shape[1]):
        c = np.flatnonzero(cand_any[t])
        p = np.flatnonzero(pair_any[t])
        k = schedule.depth(step_num, t)
        result.append(StepPlan(
            int(c[-1]) + 1 if len(c) else 1,
            int(p[-1]) + 1 if len(p) else 1,
            k if dec_any[t] else 0,
            bool(val_any[t]),
            (bool(head_any[0][t]), bool(head_any[1][t]), bool(head_any[2][t]), bool(head_any[3][t])),
            tuple(int(row) for row in np.flatnonzero(valid_dec[:,t])),
        ))
    return result


def transfer_batch(batch, device, limit_bytes=256 * 1024**2):
    """Keep huge candidate-set unrolls on the host; the step loop streams those."""
    size = sum(t.numel() * t.element_size() for t in batch.values())
    if size < limit_bytes:
        return {key: value.to(device, non_blocking=True) for key, value in batch.items()}
    return batch


def encode_boards(net, batch, chunk, device, entity_bucket=0):
    """Batch independent spatial encodings; recurrence still runs in time order.

    Only the batch axis is merged. There is no attention between frames or lanes.
    Chunk size bounds temporary trunk activations; checkpointing stays enabled.
    Return None for the reference per-frame path.
    """
    if chunk <= 1 and not entity_bucket:
        return None
    chunk = max(1, chunk)
    b, unroll = batch["step_valid"].shape
    # An optional token budget shrinks the chunk on large boards so one trunk
    # call never holds more tokens than the budget: small boards keep the
    # fewer, larger launches, the largest stay at the single-frame footprint.
    budget = getattr(net, "board_token_budget", 0)
    if budget:
        tokens_per_frame = b * (M.RECURRENT_SLOTS + batch["entity_feat"].shape[2])
        chunk = max(1, min(chunk, budget // tokens_per_frame))
    result = []
    for start in range(0, unroll, chunk):
        end = min(start + chunk, unroll)
        inputs = []
        for name in (*M.INPUT_NAMES[:8], "entity_rules"):
            tensor = batch[name][:, start:end]
            inputs.append(tensor.reshape(b * (end - start), *tensor.shape[2:]).to(device, non_blocking=True))
        if entity_bucket:
            # Padding happens on-device, never copies the whole batch through
            # pageable host memory. No entity or edge is removed or reordered.
            padding = (-inputs[0].shape[1]) % entity_bucket
            if padding:
                inputs[0] = torch.nn.functional.pad(inputs[0], (0, 0, 0, padding))
                inputs[1] = torch.nn.functional.pad(inputs[1], (0, 0, 0, padding))
                inputs[8] = torch.nn.functional.pad(inputs[8], (0, 0, 0, 0, 0, padding))
                inputs[2] = torch.nn.functional.pad(inputs[2], (0, padding), value=False)
        board, mask = net.encode_board(*inputs)
        board = board.reshape(b, end - start, *board.shape[1:])
        mask = mask.reshape(b, end - start, *mask.shape[1:])
        result.extend((board[:, t], mask[:, t]) for t in range(end - start))
    return result


def compile_regions(net, cuda_graphs=False, readouts=False):
    """Fuse repeated layers without changing state_dict keys or ONNX exports."""
    import torch._inductor.config
    torch._inductor.config.compile_threads = 4
    net._cuda_graphs = cuda_graphs
    mode = "reduce-overhead" if cuda_graphs else "default"
    for layer in net.trunk_layers:
        layer.compile(dynamic=True, mode=mode)
    net.thinking_block.compile(dynamic=True, mode=mode)
    if readouts:
        for layer in (net.recurrent_cell, net.cand_scorer, net.pair_scorer, net.seat_value_head):
            layer.compile(dynamic=True, mode=mode)
