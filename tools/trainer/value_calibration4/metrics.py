"""Proper terminal Brier scores with frame and equal-trajectory aggregation."""
import numpy as np
import torch
from .features import predict


def trajectory_weights(trajectories):
    _, inverse, counts = np.unique(trajectories, return_inverse=True, return_counts=True)
    return 1.0 / (len(counts) * counts[inverse])


@torch.no_grad()
def predictions(head, cache, device, batch_size=256):
    result = np.empty(len(cache.indices), dtype=np.float32)
    head.eval()
    for start in range(0, len(result), batch_size):
        features, active, allied, _ = cache.tensor_batch(cache.indices[start:start+batch_size], device)
        result[start:start+batch_size] = predict(head, features, active, allied).cpu().numpy()
    if not np.isfinite(result).all():
        raise FloatingPointError("Nonfinite value predictions")
    return result


def summary(prediction, truth, trajectories):
    def aggregate(weights):
        weights = weights / weights.sum()
        error = (prediction - truth) ** 2
        bins = np.minimum((np.clip(prediction, 0, 1) * 10).astype(int), 9)
        calibration = []
        ece = 0.0
        for i in range(10):
            mask = bins == i
            mass = weights[mask].sum()
            if mass:
                p = float(np.average(prediction[mask], weights=weights[mask]))
                y = float(np.average(truth[mask], weights=weights[mask]))
                ece += mass * abs(p - y)
                calibration.append(dict(bin=i, samples=int(mask.sum()), mass=float(mass), prediction=p, outcome=y))
        return dict(mse=float(np.sum(error * weights)), brier=float(np.sum(error * weights)),
                    prediction_mean=float(np.sum(prediction * weights)),
                    outcome_mean=float(np.sum(truth * weights)), ece=float(ece), calibration=calibration)
    return dict(samples=len(truth), trajectories=len(np.unique(trajectories)),
                frame_weighted=aggregate(np.ones(len(truth))),
                trajectory_weighted=aggregate(trajectory_weights(trajectories)))


def training_priors(cache):
    rows = {key: cache.records[key][cache.indices] for key in ("players", "outcome", "trajectory")}
    prior = {}
    for players in np.unique(rows["players"]):
        mask = rows["players"] == players
        prior[int(players)] = float(np.average(rows["outcome"][mask],
                                        weights=trajectory_weights(rows["trajectory"][mask])))
    fallback = float(np.average(rows["outcome"], weights=trajectory_weights(rows["trajectory"])))
    return prior, fallback


def evaluate(head, cache, device, priors, fallback):
    rows = {key: cache.records[key][cache.indices] for key in
            ("active", "allied", "source_value", "players", "turn_norm", "commander", "team",
             "decision", "outcome", "trajectory")}
    actual = predictions(head, cache, device)
    baseline = ((rows["active"] & rows["allied"]).sum(-1) / rows["active"].sum(-1))
    constant = np.array([priors.get(int(n), fallback) for n in rows["players"]])
    outputs = dict(calibrated=actual, source=rows["source_value"], uniform_side=baseline,
                   training_seatcount_prior=constant)
    groups = {"all": np.ones(len(actual), dtype=bool)}
    for n in np.unique(rows["players"]):
        groups[f"players_{n}"] = rows["players"] == n
    turn = rows["turn_norm"]
    groups["turn_0_5"] = turn <= .25
    groups["turn_6_10"] = (turn > .25) & (turn <= .5)
    groups["turn_11_19"] = (turn > .5) & (turn < 1)
    groups["turn_20plus"] = turn >= 1
    groups["commander"] = rows["commander"]
    groups["noncommander"] = ~rows["commander"]
    groups["team"] = rows["team"]
    groups["decision_samples"] = rows["decision"]
    groups["passive_samples"] = ~rows["decision"]
    return {name: {model: summary(values[mask], rows["outcome"][mask], rows["trajectory"][mask])
                   for model, values in outputs.items()}
            for name, mask in groups.items() if mask.any()}


def assert_disjoint_games(train, validation):
    left = set(map(int, train.store.index["game"]))
    right = set(map(int, validation.store.index["game"]))
    if left & right:
        raise ValueError(f"Training/validation game leakage: {sorted(left & right)[:10]}")
