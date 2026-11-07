"""Fit a copy of `seat_value_head` on cached positions; the trunk never runs here.

The only target is who won. Each turn of each game weighs the same, so a long
game is many positions but not a louder label than its turns.
"""
import copy

import numpy as np
import torch
import torch.nn.functional as F


def side_logit(head, tokens):
    """The duel's value as a logit: the deployed two-way softmax over seats."""
    global_token = tokens[:, :1].expand(-1, 2, -1)
    seat_logits = head(torch.cat([tokens[:, 1:3], global_token], dim=-1)).squeeze(-1)
    return seat_logits[:, 0] - seat_logits[:, 1]


def choose(rows, per_turn, rng):
    """Row numbers: per game, seat and turn, the first settled position and others at random."""
    key = (rows["game"].astype(np.int64) * 16 + rows["seat"]) * 4096 + np.minimum(rows["turn"], 4095)
    shuffled = rng.permutation(len(rows))
    # Settled positions sort first within a turn, earliest first, then the shuffle.
    order = np.lexsort((shuffled, np.where(rows["settled"], 0, 1), key))
    sorted_keys = key[order]
    starts = np.flatnonzero(np.r_[True, sorted_keys[1:] != sorted_keys[:-1]])
    place = np.arange(len(order)) - np.repeat(starts, np.diff(np.r_[starts, len(order)]))
    picked = np.sort(order[place < per_turn])
    _, inverse, counts = np.unique(key[picked], return_inverse=True, return_counts=True)
    return picked, (1.0 / counts[inverse]).astype(np.float32)


class Samples:
    """Chosen positions of several sets, resident on the device."""

    def __init__(self, sets, per_turn, seed, device, games):
        rng = np.random.default_rng(seed)
        tokens, targets, weights = [], [], []
        for positions, set_weight, hold in sets:
            rows = positions.rows
            kept = np.flatnonzero(games(rows, hold))
            picked, weight = choose(rows[kept], per_turn, rng)
            rows_picked = kept[picked]
            tokens.append(torch.from_numpy(np.ascontiguousarray(positions.features[rows_picked])).to(device))
            targets.append(torch.from_numpy(rows["outcome"][rows_picked].copy()).to(device))
            weights.append(torch.from_numpy(weight * set_weight).to(device))
        self.tokens = torch.cat(tokens)
        self.targets = torch.cat(targets)
        self.weights = torch.cat(weights)

    def __len__(self):
        return len(self.targets)


@torch.no_grad()
def score(head, samples, batch=32768):
    head.eval()
    loss = brier = 0.0
    for start in range(0, len(samples), batch):
        part = slice(start, start + batch)
        logit = side_logit(head, samples.tokens[part].float())
        target, weight = samples.targets[part], samples.weights[part]
        loss += float((F.binary_cross_entropy_with_logits(logit, target, reduction="none") * weight).sum())
        brier += float(((torch.sigmoid(logit) - target).square() * weight).sum())
    total = float(samples.weights.sum())
    return dict(loss=loss / total, brier=brier / total)


def fit(source_head, train, validation, seed, epochs=12, lr=1e-3, weight_decay=1e-2, batch=4096, fresh=False):
    torch.manual_seed(seed)
    head = copy.deepcopy(source_head).float().to(train.tokens.device)
    if fresh:
        for layer in head:
            if hasattr(layer, "reset_parameters"):
                layer.reset_parameters()
    for parameter in head.parameters():
        parameter.requires_grad_(True)
    steps = epochs * ((len(train) + batch - 1) // batch)
    optimizer = torch.optim.AdamW(head.parameters(), lr=lr, weight_decay=weight_decay)
    schedule = torch.optim.lr_scheduler.OneCycleLR(optimizer, lr, total_steps=max(1, steps), pct_start=.1)
    start = score(head, validation)
    best, best_state, best_epoch = start["loss"], copy.deepcopy(head.state_dict()), 0
    history = [dict(epoch=0, **start)]
    generator = torch.Generator(device="cpu").manual_seed(seed)
    for epoch in range(1, epochs + 1):
        head.train()
        order = torch.randperm(len(train), generator=generator).to(train.tokens.device)
        for begin in range(0, len(train), batch):
            part = order[begin:begin + batch]
            weight = train.weights[part]
            logit = side_logit(head, train.tokens[part].float())
            loss = (F.binary_cross_entropy_with_logits(logit, train.targets[part], reduction="none") * weight).sum() / weight.sum()
            if not torch.isfinite(loss):
                raise FloatingPointError("Nonfinite value head loss")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
            schedule.step()
        metrics = score(head, validation)
        history.append(dict(epoch=epoch, **metrics))
        print(f"[value-head] seed={seed} epoch={epoch} val_loss={metrics['loss']:.5f} val_brier={metrics['brier']:.5f}", flush=True)
        if metrics["loss"] < best:
            best, best_state, best_epoch = metrics["loss"], copy.deepcopy(head.state_dict()), epoch
    head.load_state_dict(best_state)
    return head, dict(selected_epoch=best_epoch, history=history, train_rows=len(train), validation_rows=len(validation),
                      objective="side outcome, binary cross entropy; each turn of each game weighs the same")


@torch.no_grad()
def predict(head, positions, device, batch=32768):
    """The head's value at every position, in stored order."""
    head.eval()
    result = np.empty(len(positions.rows), dtype=np.float32)
    for start in range(0, len(result), batch):
        tokens = torch.from_numpy(np.ascontiguousarray(positions.features[start:start + batch])).to(device).float()
        result[start:start + batch] = torch.sigmoid(side_logit(head, tokens)).cpu().numpy()
    return result
