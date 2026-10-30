"""CPU tensor materialization check, without model forward or target creation."""
import torch
import dataset4 as D
from policy_training4.targets import actor_mask


def check_tensors(dataset, batch_size):
    visited, shapes, dtypes = set(), {}, {}
    frames = choices = 0
    for indices in D.StatefulBatchSampler4(dataset, batch_size, shuffle=False):
        batch = D.collate_trajectories([dataset[index] for index in indices])
        for name, tensor in batch.items():
            if tensor.device.type != 'cpu':
                raise ValueError('Research preflight materialized a non-CPU tensor')
            if tensor.is_floating_point() and not bool(torch.isfinite(tensor).all()):
                raise ValueError(f'Nonfinite research tensor: {name}')
            shape = list(tensor.shape)
            if name in shapes:
                if len(shape) != len(shapes[name]) or dtypes[name] != str(tensor.dtype):
                    raise ValueError('Research tensor rank/dtype changed between unrolls')
                shape = [max(a, b) for a, b in zip(shape, shapes[name])]
            shapes[name], dtypes[name] = shape, str(tensor.dtype)
        valid = batch['step_valid'].bool()
        actor = actor_mask(batch)
        if not torch.equal(actor, batch['actor_mask'].bool() & valid):
            raise ValueError('Native and Torch preflight actor masks disagree')
        frames += int(valid.sum())
        choices += int(actor.sum())
        for index in indices:
            if index < 0:
                continue
            if index in visited:
                raise ValueError('Duplicate research preflight segment')
            visited.add(index)
    if (visited != set(range(len(dataset))) or frames != dataset.admission['frames']
            or choices != dataset.admission['genuine_choices']
            or len(dataset.trajectories) != dataset.admission['learner_histories']):
        raise ValueError('Research preflight coverage differs from native admission')
    return dict(phase='preflight-complete', training=False, optimizer_constructed=False,
                optimizer_steps=0, model_forwards=0, model_writes=0,
                frames=frames, genuine_choices=choices, segments=len(visited),
                learner_histories=len(dataset.trajectories),
                maximum_collated_shapes=shapes, dtypes=dtypes)
