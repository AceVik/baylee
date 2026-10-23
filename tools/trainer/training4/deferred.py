"""Batch independent readouts after a strictly chronological memory pass.

Each readout receives only the state produced by its own observation and past
observations. Attention never crosses the batch axis. No frame, candidate or
gradient is discarded. This execution path requires dropout=0 so regrouping
does not change which stochastic mask belongs to an observation.
"""
import torch
import model4 as M


def readout_groups(plans, chunk, token_limit=None):
    """Bound grouping by thinking depth and candidate/pair size, not game time."""
    groups = {}
    for t, plan in enumerate(plans):
        key = (plan.thinking, (plan.candidates - 1).bit_length(), (plan.pairs - 1).bit_length())
        pending = groups.setdefault(key, [])
        pending.append(t)
        capacity = chunk
        if token_limit is not None:
            # Upper bound for the concatenated candidate/pair scorer inputs.
            capacity = min(chunk, max(1, token_limit // (4 * 2**key[1] + 3 * 2**key[2])))
        if len(pending) == capacity:
            yield tuple(pending)
            pending.clear()
    for pending in groups.values():
        if pending:
            yield tuple(pending)


def deferred_readouts(net, batch, boards, hidden, plans, chunk, device, bf16=False):
    """Return per-frame model outputs and the exact final persistent memory.

    The first pass retains the full recurrent autograd chain. The second pass
    groups independent decisions, including passive value supervision. Keeping
    autocast scoped to each memory update preserves its original precision.
    """
    if boards is None or plans is None:
        raise ValueError("Deferred readouts require pre-encoded boards and CPU execution plans")
    if chunk < 1 or net.cfg.dropout != 0:
        raise ValueError("Deferred readouts require positive chunk size and dropout=0")
    b, unroll = batch["step_valid"].shape
    memories, thinking = [], []
    for t, plan in enumerate(plans):
        k = batch["is_decision"][:, t].to(device).long() * plan.thinking
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            observed = net.observe_memory(boards[t][0], hidden, k)
        memories.append(observed)
        thinking.append(k)
        valid = batch["step_valid"][:, t].to(device)
        hidden = torch.where(valid[:, None, None], observed.float(), hidden)

    outputs = [None] * unroll
    token_limit = 128 * 1024**2 // (b * net.cfg.d_model * 4)
    for times in readout_groups(plans, chunk, token_limit):
        c = max(plans[t].candidates for t in times)
        p = max(plans[t].pairs for t in times)
        # Merge frames into the batch axis. Trim BEFORE concatenation so one
        # rare NameCard frame cannot inflate all other readouts in the unroll.
        inputs = []
        for name in M.INPUT_NAMES[:-2]:
            frames = []
            for t in times:
                frame = batch[name][:, t]
                if name.startswith("cand_"):
                    frame = frame[:, :c]
                elif name.startswith("pair_"):
                    frame = frame[:, :p]
                frames.append(frame.to(device, non_blocking=True))
            inputs.append(torch.cat(frames, dim=0))
        memory = torch.cat([memories[t] for t in times], dim=0)
        board = tuple(torch.cat([boards[t][i] for t in times], dim=0) for i in range(2))
        budgets = torch.cat([thinking[t] for t in times], dim=0)
        heads = (tuple(any(plans[t].heads[h] for t in times) for h in range(4))
                 if getattr(net, "prune_unused_policy_heads", False) else None)
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            packed = net(*inputs, memory, budgets, execution_hint=(c, p, plans[times[0]].thinking),
                         encoded_board=board, policy_heads=heads, observed_memory=memory)
        for i, t in enumerate(times):
            frame = [value[i*b:(i+1)*b] for value in packed]
            frame[1] = frame[1][:, :plans[t].candidates]
            frame[2] = frame[2][:, :plans[t].pairs]
            outputs[t] = tuple(frame)
    return outputs, hidden
