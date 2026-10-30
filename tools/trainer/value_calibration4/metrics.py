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
