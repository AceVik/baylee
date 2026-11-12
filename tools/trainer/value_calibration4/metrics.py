"""Torch prediction execution with batched native calibration aggregation."""
import json
import numpy as np
import torch
from trajectory_store4 import _native
from .features import predict


def _metrics(kind, **value):
    return json.loads(_native(['calibration-metrics'],json.dumps(dict(kind=kind,**value),allow_nan=False).encode()))


def trajectory_weights(trajectories):
    return np.asarray(_metrics('weights',trajectories=np.asarray(trajectories).tolist()),dtype=np.float64)


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
    return _metrics('summary',prediction=np.asarray(prediction).tolist(),truth=np.asarray(truth).tolist(),
        trajectories=np.asarray(trajectories).tolist(),float32=np.asarray(prediction).dtype==np.float32 and np.asarray(truth).dtype==np.float32)


def training_priors(cache):
    rows={key:cache.records[key][cache.indices].tolist() for key in ('players','outcome','trajectory')}
    result=_metrics('priors',rows=rows)
    return {int(key):value for key,value in result['prior'].items()},result['fallback']


def evaluate(head, cache, device, priors, fallback):
    rows={key:cache.records[key][cache.indices].tolist() for key in
        ('active','allied','source_value','players','turn_norm','commander','team','decision','outcome','trajectory')}
    return _metrics('evaluate',actual=predictions(head,cache,device).tolist(),rows=rows,priors=priors,fallback=fallback)


def assert_disjoint_games(train, validation):
    _metrics('disjoint',left=train.store.index['game'].tolist(),right=validation.store.index['game'].tolist())


def stride_indices(cache, stride):
    """The cache's ordinary samples: cursor stride, each trajectory's first and terminal frame."""
    cursor = cache.records["cursor"][cache.indices]
    keep = (cursor % stride == 0) | cache.records["terminal"][cache.indices]
    return cache.indices[keep]


def turn_start_indices(cache):
    """Each trajectory's earliest sample of every turn below 20.

    With `--turn-starts` every turn's first frame is sampled, so the earliest
    sample of a (trajectory, turn) is that turn's start. turn/20 is clipped at
    one, so turns from 20 on cannot be told apart and are left out.
    """
    trajectory = cache.records["trajectory"][cache.indices]
    turn = cache.records["turn_norm"][cache.indices]
    cursor = cache.records["cursor"][cache.indices]
    order = np.lexsort((cursor, turn, trajectory))
    first = np.ones(len(order), dtype=bool)
    first[1:] = (trajectory[order][1:] != trajectory[order][:-1]) | (turn[order][1:] != turn[order][:-1])
    return cache.indices[np.sort(order[first & (turn[order] < 1.0)])]


def game_seeds(data_dir):
    """Deal seed of every converted game, joined natively from the dataset's ready records."""
    from .cache import metadata
    return {int(game): int(seed) for game, seed in metadata('game_seeds', data_dir=str(data_dir)).items()}


def seed_halves(cache, seeds, held_out_mod):
    """Held-out games in two halves by deal seed, as the tactician probe splits them.

    convert4 holds out the seeds divisible by `held_out_mod` (`--val-every`);
    games with (seed // held_out_mod) even select the epoch, odd ones are reported.
    """
    games = cache.records["game"][cache.indices]
    seed = np.array([seeds[int(game)] for game in games], dtype=np.uint64)
    mod = np.uint64(held_out_mod)
    if (seed % mod != 0).any():
        raise ValueError("Held-out games must be the seeds divisible by --held-out-mod")
    select = (seed // mod) % np.uint64(2) == 0
    return cache.indices[select], cache.indices[~select]


def bootstrap(head, cache, indices, device, resamples, seed):
    """Native game-cluster bootstrap of r1's value (before) and the refit head (after)."""
    return _metrics('bootstrap', before=cache.records["source_value"][indices].tolist(),
                    after=predictions(head, cache.restrict(indices), device).tolist(),
                    truth=cache.records["outcome"][indices].tolist(),
                    trajectories=cache.records["trajectory"][indices].tolist(),
                    games=cache.records["game"][indices].tolist(),
                    resamples=resamples, seed=seed, float32=True)


def turn_start_report(head, cache, device):
    """Native calibration summaries of r1's own value and the refit head at turn starts."""
    indices = turn_start_indices(cache)
    subset = cache.restrict(indices)
    truth = cache.records["outcome"][indices]
    trajectories = cache.records["trajectory"][indices]
    return dict(definition="first sampled frame of each turn below 20 per held-out learner trajectory",
                source=summary(cache.records["source_value"][indices], truth, trajectories),
                calibrated=summary(predictions(head, subset, device), truth, trajectories))
