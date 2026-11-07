"""Fit a copy of `seat_value_head` on cached positions; the trunk never runs here.

The only target is who won. Each turn of each game weighs the same, so a long
game is many positions but not a louder label than its turns. `step_weight`
adds a second reading of the same label: the change of the logit from one turn's
settled position to the next says who wins, which is what a search compares.
"""
import copy

import numpy as np
import torch
import torch.nn.functional as F

from .low_precision import LowPrecisionAdamW, Numerics


def side_logit(head, tokens):
    """The duel's value as a logit: the deployed two-way softmax over seats."""
    tokens = tokens.to(next(head.parameters()).dtype)
    global_token = tokens[:, :1].expand(-1, 2, -1)
    seat_logits = head(torch.cat([tokens[:, 1:3], global_token], dim=-1)).squeeze(-1)
    return (seat_logits[:, 0] - seat_logits[:, 1]).float()


def choose(rows, per_turn, rng):
    """Per game, seat and turn: the first settled position, and others at random.

    Returns row numbers, their weights, and which of them are first settled.
    """
    key = (rows["game"].astype(np.int64) * 16 + rows["seat"]) * 4096 + np.minimum(rows["turn"], 4095)
    rank = rng.permutation(len(rows)) + 1
    settled = np.flatnonzero(rows["settled"])
    # Rows are chronological, so the first of a key is its turn's first settled position.
    rank[settled[np.unique(key[settled], return_index=True)[1]]] = 0
    order = np.lexsort((rank, key))
    sorted_keys = key[order]
    starts = np.flatnonzero(np.r_[True, sorted_keys[1:] != sorted_keys[:-1]])
    place = np.arange(len(order)) - np.repeat(starts, np.diff(np.r_[starts, len(order)]))
    picked = np.sort(order[place < per_turn])
    _, inverse, counts = np.unique(key[picked], return_inverse=True, return_counts=True)
    return picked, (1.0 / counts[inverse]).astype(np.float32), rank[picked] == 0


def steps_of(rows, first):
    """Index pairs of consecutive first settled positions of one game and seat."""
    numbers = np.flatnonzero(first)
    trajectory = rows["game"][numbers].astype(np.int64) * 16 + rows["seat"][numbers]
    numbers = numbers[np.argsort(trajectory, kind="stable")]
    trajectory = np.sort(trajectory, kind="stable")
    same = trajectory[1:] == trajectory[:-1]
    return np.stack([numbers[:-1][same], numbers[1:][same]], axis=1)


class Samples:
    """Chosen positions of several sets, resident on the device."""

    def __init__(self, sets, per_turn, seed, device, games):
        rng = np.random.default_rng(seed)
        tokens, targets, weights, pairs, domains = [], [], [], [], []
        total = 0
        for positions, set_weight, hold, domain in sets:
            rows = positions.rows
            kept = np.flatnonzero(games(rows, hold))
            picked, weight, first = choose(rows[kept], per_turn, rng)
            rows_picked = kept[picked]
            tokens.append(torch.from_numpy(positions.features[rows_picked]).to(device))
            targets.append(torch.from_numpy(rows["outcome"][rows_picked].copy()).to(device))
            weights.append(torch.from_numpy(weight * set_weight).to(device))
            pairs.append(torch.from_numpy(steps_of(rows[rows_picked], first) + total).to(device))
            domains.append(torch.full((len(rows_picked),), domain, dtype=torch.long, device=device))
            total += len(rows_picked)
        self.tokens = torch.cat(tokens)
        self.targets = torch.cat(targets)
        self.weights = torch.cat(weights)
        self.pairs = torch.cat(pairs)
        self.domains = torch.cat(domains)
        self.step_balance = self._step_balance()

    def _step_balance(self):
        """Weights that make wins and losses count alike among a domain's steps.

        With a seat that mostly loses, a plain reading of the step teaches every
        step to go down, and the head drifts: high early, low late.
        """
        outcome = self.targets[self.pairs[:, 1]]
        domain = self.domains[self.pairs[:, 1]]
        balance = torch.ones_like(outcome)
        for number in domain.unique():
            inside = domain == number
            rate = outcome[inside].mean().clamp(.05, .95)
            balance[inside] = torch.where(outcome[inside] > .5, .5 / rate, .5 / (1 - rate))
        return balance

    def __len__(self):
        return len(self.targets)


def step_loss(head, samples, numbers, gain=1.0):
    """The change of the logit between two turns, read as who wins the game."""
    pairs = samples.pairs[numbers]
    before = side_logit(head, samples.tokens[pairs[:, 0]].float())
    after = side_logit(head, samples.tokens[pairs[:, 1]].float())
    loss = F.binary_cross_entropy_with_logits(gain * (after - before), samples.targets[pairs[:, 1]], reduction="none")
    return (loss * samples.step_balance[numbers]).sum()


@torch.no_grad()
def score(head, samples, offsets, gain=1.0, batch=32768):
    head.eval()
    loss = brier = steps = 0.0
    for start in range(0, len(samples), batch):
        part = slice(start, start + batch)
        logit = side_logit(head, samples.tokens[part].float()) + offsets[samples.domains[part]]
        target, weight = samples.targets[part], samples.weights[part]
        loss += float((F.binary_cross_entropy_with_logits(logit, target, reduction="none") * weight).sum())
        brier += float(((torch.sigmoid(logit) - target).square() * weight).sum())
    for start in range(0, len(samples.pairs), batch):
        steps += float(step_loss(head, samples, slice(start, start + batch), gain))
    total = float(samples.weights.sum())
    return dict(loss=loss / total, brier=brier / total, step=steps / max(1, len(samples.pairs)))


def base_rate_offsets(train, domains):
    """Each domain's win rate against the first's, as a logit: where its intercept starts."""
    rates = []
    for number in range(domains):
        inside = train.domains == number
        weight = train.weights[inside]
        rate = (train.targets[inside] * weight).sum() / weight.sum().clamp(min=1e-6)
        rates.append(torch.logit(rate.clamp(.01, .99)) if inside.any() else torch.logit(torch.tensor(.5)))
    logits = torch.stack([rate.cpu() for rate in rates])
    return logits - logits[0]


def fit(source_head, train, validation, seed, epochs=12, lr=3e-4, weight_decay=1e-2, batch=4096, fresh=False,
        step_weight=0.0, numerics=Numerics(), learn_step_gain=False, offsets_from_base_rates=False):
    torch.manual_seed(seed)
    head = copy.deepcopy(source_head).to(device=train.tokens.device, dtype=numerics.dtype)
    if fresh:
        for layer in head:
            if hasattr(layer, "reset_parameters"):
                layer.reset_parameters()
    for parameter in head.parameters():
        parameter.requires_grad_(True)
    steps = epochs * ((len(train) + batch - 1) // batch)
    # Each domain but the first has an intercept of its own: base rates differ by
    # who sits opposite, and the head stays calibrated for the first domain alone.
    domains = int(max(train.domains.max(), validation.domains.max())) + 1
    start_offsets = base_rate_offsets(train, domains) if offsets_from_base_rates else torch.zeros(domains)
    offsets = start_offsets.to(device=train.tokens.device, dtype=numerics.dtype).requires_grad_(True)
    free = (torch.arange(domains, device=train.tokens.device) > 0).float()
    shift = lambda: offsets * free
    # The step has a scale of its own, or reading it well would stretch the logit itself.
    log_gain = torch.zeros((), device=train.tokens.device, dtype=numerics.dtype, requires_grad=learn_step_gain)
    gain = lambda: log_gain.float().exp()
    groups = [dict(params=list(head.parameters()), weight_decay=weight_decay),
              dict(params=[offsets, log_gain], weight_decay=0.0)]
    optimizer = (torch.optim.AdamW(groups, lr=lr) if numerics == Numerics() else LowPrecisionAdamW(groups, lr, numerics))
    # One scalar per domain has to travel about a logit in a few hundred steps.
    schedule = torch.optim.lr_scheduler.OneCycleLR(optimizer, [lr, 30 * lr], total_steps=max(1, steps), pct_start=.1)
    objective = lambda metrics: metrics["loss"] + step_weight * metrics["step"]
    start = score(head, validation, shift(), gain())
    best, best_state, best_epoch = objective(start), copy.deepcopy(head.state_dict()), 0
    best_offsets = shift().detach().cpu().tolist()
    history = [dict(epoch=0, **start)]
    generator = torch.Generator(device="cpu").manual_seed(seed)
    pair_batch = max(1, batch // 4)
    for epoch in range(1, epochs + 1):
        head.train()
        device = train.tokens.device
        order = torch.randperm(len(train), generator=generator).to(device)
        for begin in range(0, len(train), batch):
            part = order[begin:begin + batch]
            weight = train.weights[part]
            logit = side_logit(head, train.tokens[part].float()) + shift()[train.domains[part]]
            loss = (F.binary_cross_entropy_with_logits(logit, train.targets[part], reduction="none") * weight).sum() / weight.sum()
            if step_weight:
                drawn = torch.randint(len(train.pairs), (pair_batch,), generator=generator).to(device)
                loss = loss + step_weight * step_loss(head, train, drawn, gain()) / pair_batch
            if not torch.isfinite(loss):
                raise FloatingPointError("Nonfinite value head loss")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
            schedule.step()
        metrics = score(head, validation, shift(), gain())
        history.append(dict(epoch=epoch, **metrics))
        print(f"[value-head] seed={seed} epoch={epoch} val_loss={metrics['loss']:.5f} "
              f"val_brier={metrics['brier']:.5f} val_step={metrics['step']:.5f}", flush=True)
        if objective(metrics) < best:
            best, best_state, best_epoch = objective(metrics), copy.deepcopy(head.state_dict()), epoch
            best_offsets = shift().detach().cpu().tolist()
    head.load_state_dict(best_state)
    return head.float(), dict(selected_epoch=best_epoch, history=history, train_rows=len(train), validation_rows=len(validation),
                              numerics=vars(numerics),
                      train_steps=len(train.pairs), validation_steps=len(validation.pairs), domain_offsets=best_offsets,
                      objective="side outcome, binary cross entropy; each turn of each game weighs the same",
                      step_weight=step_weight, step_gain=float(gain()))


@torch.no_grad()
def predict(head, positions, device, batch=32768):
    """The head's value at every position, in stored order."""
    head.eval()
    result = np.empty(len(positions.rows), dtype=np.float32)
    for start in range(0, len(result), batch):
        tokens = torch.from_numpy(np.array(positions.features[start:start + batch])).to(device).float()
        result[start:start + batch] = torch.sigmoid(side_logit(head, tokens)).cpu().numpy()
    return result
