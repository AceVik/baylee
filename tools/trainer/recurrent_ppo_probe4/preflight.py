"""CPU PyTorch/checkpoint/native-dataset preflight; never executes a model."""

from contextlib import ExitStack
import time

import torch

import dataset4 as D
from policy_training4.admission import select_learner_histories
from policy_training4.targets import actor_mask
from recurrent_ppo4.checkpoint import load_policy, native_source_digest, native_verify_admission
from torch_io4 import atomic_json
from .measurement import frame_identities


def inspect_histories(dataset, unroll=64):
    """Load/collate every segment of every selected history, not a sampled prefix."""
    identities = frame_identities(dataset)
    visited = set()
    result = []
    for history in dataset.trajectories:
        history_key = None
        frames = choices = 0
        for start in range(0, len(history), 4):
            indices = history[start:start+4]
            batch = D.collate_trajectories([dataset[index] for index in indices])
            if batch['step_valid'].shape != (len(indices), unroll):
                raise ValueError('Collated preflight tensor extent differs from declared unroll')
            if batch['segment_index'].tolist() != list(indices):
                raise ValueError('Collated preflight segment identities changed')
            valid = batch['step_valid'].bool()
            actor = actor_mask(batch)
            if (not bool(torch.isfinite(batch['behavior_logprob'][actor]).all())
                    or bool((batch['thinking_steps'][actor] != 2).any())
                    or not bool(torch.allclose(batch['temperature'][actor],
                                              torch.full_like(batch['temperature'][actor], .7),
                                              atol=1e-6, rtol=0))):
                raise ValueError('Admitted actor likelihood/K2/temperature contract differs')
            for lane, segment in enumerate(indices):
                if int(valid[lane].sum()) != len(dataset.segments[segment]):
                    raise ValueError('Loaded segment frame count differs from native index')
                for t in torch.nonzero(valid[lane], as_tuple=False).flatten().tolist():
                    key = identities[segment, t]
                    if key in visited:
                        raise ValueError('Preflight loaded an admitted frame more than once')
                    visited.add(key)
                    if history_key is None:
                        history_key = key[:2]
                    if history_key != key[:2]:
                        raise ValueError('One recurrent history mixes game/seat identities')
            frames += int(valid.sum())
            choices += int(actor.sum())
        if history_key is None:
            raise ValueError('Empty admitted recurrent history')
        result.append(dict(game_id=history_key[0], seat=history_key[1],
                           segments=len(history), frames=frames, choices=choices))
    if set(identities.values()) != visited:
        raise ValueError('Preflight did not load all indexed admitted frames')
    if len({(row['game_id'], row['seat']) for row in result}) != len(result):
        raise ValueError('Repeated admitted recurrent history')
    return result


def run_preflight(args):
    # This dispatch happens before the measurement driver's CUDA initialization.
    # Rust owns wall time/PSS, immutable source seals and the guarded environment.
    from .driver import validate_candidate
    if args.device != 'cpu':
        raise ValueError('Tensor preflight is CPU-only')
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    report = dict(version=1, mode='recurrent_ppo_tensor_preflight_v1', status='running',
                  device='cpu', cuda_used=False, model_forward_calls=0,
                  optimizer_constructed=False, probability_replay_performed=False,
                  threads=4, interop_threads=1, torch_version=str(torch.__version__),
                  all_histories_loaded_and_collated=False,
                  source_candidate_contract_compatible=False)
    started = time.perf_counter()
    atomic_json(args.out, report)
    try:
        source_hash = native_source_digest(args.source)
        candidate_hash = native_source_digest(args.candidate)
        native_verify_admission(args.rollout_data, args.source, args.admission)
        source = torch.load(args.source, map_location='cpu', weights_only=False)
        candidate = torch.load(args.candidate, map_location='cpu', weights_only=False)
        source_net = load_policy(source, torch.device('cpu')).requires_grad_(False)
        candidate_net = load_policy(candidate, torch.device('cpu')).requires_grad_(False)
        with ExitStack() as resources:
            dataset = D.TrajectoryDataset4(args.rollout_data, split='train',
                        encoder_version=source['encoder_version'], unroll_len=64, max_shards=0)
            resources.callback(dataset.store.close)
            select_learner_histories(dataset, args.admission, source_hash)
            validate_candidate(candidate, source, source_hash, dataset.fingerprint)
            if (dataset.structure.digest != source_net.rules.digest
                    or dataset.structure.digest != candidate_net.rules.digest):
                raise ValueError('Source/candidate/dataset rule catalogs differ')
            per_history = inspect_histories(dataset)
            frames = sum(row['frames'] for row in per_history)
            choices = sum(row['choices'] for row in per_history)
            if (len(per_history) != args.expected_histories or frames != args.expected_frames
                    or choices != args.expected_choices):
                raise ValueError('CPU preflight differs from declared complete collection coverage')
            report.update(source_checkpoint_sha256=source_hash, candidate_checkpoint_sha256=candidate_hash,
                          dataset_fingerprint=dataset.fingerprint, catalog_digest=dataset.structure.digest,
                          learner_histories=len(per_history), frames=frames, choices=choices,
                          per_history=per_history, all_histories_loaded_and_collated=True,
                          source_candidate_contract_compatible=True)
        native_verify_admission(args.rollout_data, args.source, args.admission)
        if native_source_digest(args.candidate) != candidate_hash:
            raise ValueError('Candidate checkpoint changed during CPU preflight')
        report.update(status='complete', elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
        print(f'[ppo-preflight] histories={len(per_history)} frames={frames} choices={choices} '
              'CPU tensors loaded/collated; no model forward', flush=True)
    except BaseException as error:
        report.update(status='failed', error=str(error), elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
        raise
