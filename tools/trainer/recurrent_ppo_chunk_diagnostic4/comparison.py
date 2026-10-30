"""Describe the unchanged probe tolerance at an exact native frame/coordinate."""

import math

import torch

from recurrent_ppo_probe4.comparison import HIDDEN_ATOL, HIDDEN_RTOL, LP_ATOL


def tensor_error(reference, current):
    if reference.shape != current.shape:
        raise ValueError('Diagnostic tensor shapes differ')
    if not bool(torch.isfinite(reference).all() & torch.isfinite(current).all()):
        raise FloatingPointError('Nonfinite diagnostic tensor')
    delta = (current.double()-reference.double()).abs()
    allowed = HIDDEN_ATOL+HIDDEN_RTOL*reference.double().abs()
    normalized = delta/allowed
    flat = int(normalized.flatten().argmax())
    coordinate, remainder = [], flat
    for size in reversed(reference.shape):
        coordinate.insert(0, remainder % size)
        remainder //= size
    ref = float(reference.flatten()[flat])
    cur = float(current.flatten()[flat])
    return dict(passed=bool(torch.allclose(current, reference, atol=HIDDEN_ATOL, rtol=HIDDEN_RTOL)),
                max_abs=float(delta.max()), max_normalized=float(normalized.max()),
                coordinate=coordinate, reference=ref, current=cur,
                allowed=HIDDEN_ATOL+HIDDEN_RTOL*abs(ref),
                normalized_error=float(normalized.flatten()[flat]))


def scalar_error(reference, current, *, actor=False):
    reference, current = float(reference), float(current)
    if not math.isfinite(reference) or not math.isfinite(current):
        raise FloatingPointError('Nonfinite diagnostic scalar')
    allowed = LP_ATOL if actor else HIDDEN_ATOL+HIDDEN_RTOL*abs(reference)
    delta = abs(current-reference)
    return dict(passed=delta <= allowed, max_abs=delta, max_normalized=delta/allowed,
                coordinate=[], reference=reference, current=current,
                allowed=allowed, normalized_error=delta/allowed)


def output_error(reference, current, reference_lp, current_lp, lane, actor):
    result = dict(hidden=tensor_error(reference[0][lane], current[0][lane]),
                  value=scalar_error(reference[4][lane], current[4][lane]))
    if actor:
        result['logprob'] = scalar_error(reference_lp[lane], current_lp[lane], actor=True)
    return result


def update_maxima(maxima, errors):
    for name, error in errors.items():
        saved = maxima.setdefault(name, dict(max_abs=0., max_normalized=0., violating_frames=0))
        saved['max_abs'] = max(saved['max_abs'], error['max_abs'])
        saved['max_normalized'] = max(saved['max_normalized'], error['max_normalized'])
        saved['violating_frames'] += int(not error['passed'])


def first_output_violation(errors):
    for name in ('hidden', 'value', 'logprob'):
        if name in errors and not errors[name]['passed']:
            return dict(kind=name, **errors[name])
    return None
