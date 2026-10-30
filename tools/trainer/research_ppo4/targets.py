"""Frozen source-value targets and complete recurrent probability replay."""
from pathlib import Path
import tempfile

import numpy as np
import torch

from policy_math4 import action_logprob_and_kl
from policy_training4.targets import actor_mask
from recurrent_ppo4.collection import stream_collection_batches
from recurrent_ppo4.forward import episode_prefixes

OBJECTIVE = 'failure_terminal_utility_minus_frozen_source_value_uncentered_v1'
TARGET_CONTRACT = 'failure_terminal_research_v1'
REPLAY_TOLERANCE = .001


def frozen_advantages(utility, source_value, valid, actor):
    """A=U−V_source, without centering, scaling, bootstrapping or auxiliaries."""
    if not bool(torch.isfinite(utility[valid]).all() & torch.isfinite(source_value[valid]).all()):
        raise ValueError('Nonfinite research utility/source value')
    if bool(((utility[valid] < 0) | (utility[valid] > 1)).any()):
        raise ValueError('Research utility outside declared range')
    returns = torch.where(valid, utility, torch.zeros_like(utility))
    advantages = torch.where(valid & actor, returns - source_value, torch.zeros_like(utility))
    return advantages, returns


@torch.no_grad()
def prepare_targets(net, dataset, device, batch_size=4):
    """Visit every admitted history under unchanged source weights before updates.

    The native adapter already determines roles and exact executed masks. Check
    those masks against tensor semantics; do not select/filter histories here.
    Source values stay frozen even after optimizer updates and resume.
    """
    if hasattr(dataset, 'ppo_cache'):
        raise ValueError('Research target cache already exists')
    if any(p.dtype != torch.float32 for p in net.parameters()):
        raise ValueError('Research replay requires FP32 model parameters')
    temporary = tempfile.TemporaryDirectory(prefix='baylee-research-targets-')
    shape = (len(dataset), dataset.unroll_len)
    if not shape[0]:
        temporary.cleanup()
        raise ValueError('No admitted research histories')
    cache = {
        name: np.memmap(Path(temporary.name) / name, mode='w+', dtype=dtype, shape=shape)
        for name, dtype in [('advantages', np.float32), ('returns', np.float32),
                            ('behavior_lp', np.float32), ('actor_mask', np.bool_)]
    }
    frames = choices = 0
    max_error = total = total2 = 0.
    min_value, max_value = float('inf'), float('-inf')
    visited = set()
    try:
        for indices, batch, outputs in stream_collection_batches(
                net, dataset, episode_prefixes(dataset), device, batch_size, board_chunk=0):
            valid = batch['step_valid'].bool()
            mask = actor_mask(batch)
            if not torch.equal(mask, batch['actor_mask'].bool() & valid):
                raise ValueError('Rust and Torch research actor masks disagree')
            if bool((batch['thinking_steps'][mask] != 2).any()):
                raise ValueError('Research replay requires recorded K2')
            if not torch.allclose(batch['temperature'][mask], torch.full_like(batch['temperature'][mask], .7),
                                  atol=1e-6, rtol=0):
                raise ValueError('Research replay temperature differs')
            values, errors = [], []
            for t, out in enumerate(outputs):
                lp, _ = action_logprob_and_kl(*out[1:4], *out[1:4], batch, t)
                values.append(out[4].float().cpu())
                errors.append((lp.cpu() - batch['behavior_logprob'][:, t]).abs())
            values = torch.stack(values, 1)
            error_grid = torch.stack(errors, 1)
            errors = error_grid[mask]
            if errors.numel():
                failed = mask & (~torch.isfinite(error_grid) | (error_grid > REPLAY_TOLERANCE))
                if bool(failed.any()):
                    lane, time = failed.nonzero()[0].tolist()
                    raise ValueError(f'Research recorded probability replay failed: segment={indices[lane]} '
                                     f't={time} head={int(batch["head"][lane, time])} '
                                     f'absolute_error={float(error_grid[lane, time])} '
                                     f'tolerance={REPLAY_TOLERANCE}')
                max_error = max(max_error, float(errors.max()))
            advantages, returns = frozen_advantages(batch['training_utility'], values, valid, mask)
            if valid.any():
                min_value = min(min_value, float(values[valid].min()))
                max_value = max(max_value, float(values[valid].max()))
            frames += int(valid.sum())
            choices += int(mask.sum())
            chosen = advantages[mask].double()
            total += float(chosen.sum())
            total2 += float(chosen.square().sum())
            for lane, idx in enumerate(indices):
                if idx < 0:
                    continue
                if idx in visited:
                    raise ValueError('Duplicate research segment in numerical replay')
                visited.add(idx)
                for name, value in [('advantages', advantages), ('returns', returns),
                                    ('behavior_lp', batch['behavior_logprob']), ('actor_mask', mask)]:
                    cache[name][idx] = value[lane].numpy()
        admission = dataset.admission
        if (visited != set(range(len(dataset))) or frames != admission['frames']
                or choices != admission['genuine_choices'] or not choices
                or len(dataset.trajectories) != admission['learner_histories']):
            raise ValueError('Research numerical replay coverage differs from native admission')
        for array in cache.values():
            array.flush()
        dataset.ppo_cache = cache
        dataset._policy_targets_directory = temporary
        return dict(objective=OBJECTIVE, target_contract=TARGET_CONTRACT,
                    frames=frames, decisions_checked=choices, learner_histories=len(dataset.trajectories),
                    segments=len(visited), max_logprob_error=max_error, tolerance=REPLAY_TOLERANCE,
                    precision='float32', advantage_centering=False, bootstrap=False,
                    source_value_min=min_value, source_value_max=max_value,
                    advantage_mean=total / choices, advantage_second_moment=total2 / choices)
    except BaseException:
        cache.clear()
        temporary.cleanup()
        raise
    finally:
        net.clear_structure()
