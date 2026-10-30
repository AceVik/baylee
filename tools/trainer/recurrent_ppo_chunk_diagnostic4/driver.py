"""Source-only Torch adapter; Rust owns deadlines, locks, seals and publication."""

import argparse
from contextlib import ExitStack
from pathlib import Path
import time

import torch

import dataset4 as D
from policy_training4.admission import check_audit, select_learner_histories
from policy_training4.targets import prepare_policy_targets
from recurrent_ppo4.checkpoint import load_policy, native_source_digest, native_verify_admission
from recurrent_ppo4.forward import episode_prefixes
from recurrent_ppo_probe4.comparison import HIDDEN_ATOL, HIDDEN_RTOL, LP_ATOL
from recurrent_ppo_probe4.driver import initialize_device, parse_device
from recurrent_ppo_probe4.measurement import frame_identities
from recurrent_ppo_probe4.preflight import inspect_histories
from torch_io4 import atomic_json
from .execution import diagnose


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'rollout-data', 'admission', 'out'):
        parser.add_argument('--'+name, required=True, type=Path)
    parser.add_argument('--device', type=parse_device, default='cuda')
    parser.add_argument('--preflight-only', action='store_true')
    for name in ('histories', 'frames', 'choices'):
        parser.add_argument('--expected-'+name, type=int, required=True)
    args = parser.parse_args(argv)
    if min(args.expected_histories, args.expected_frames, args.expected_choices) < 1:
        parser.error('Require positive complete-collection coverage')
    if args.out.exists():
        parser.error('Preserving existing diagnostic report; choose a new output')
    if args.preflight_only and args.device != 'cpu':
        parser.error('Preflight requires explicit --device cpu')
    return args


def validate_counts(args, histories, frames, choices):
    if (histories, frames, choices) != (args.expected_histories, args.expected_frames, args.expected_choices):
        raise ValueError('Diagnostic source coverage differs from declared complete collection')


def main(argv=None):
    args = parse_args(argv)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    report = dict(version=1, mode='recurrent_chunk_diagnostic_preflight_v1' if args.preflight_only
                  else 'recurrent_chunk_diagnostic_v1', status='running', diagnostic_only=True,
                  parity_accepted=False, optimizer_constructed=False, optimizer_steps=0,
                  parameter_updates=0, model_writes=0, promoted=False,
                  requested_device=args.device, precision='float32', tf32=False,
                  batch_size=4, unroll=64, board_chunk=4, threads=4, interop_threads=1,
                  expected=dict(histories=args.expected_histories, frames=args.expected_frames,
                                choices=args.expected_choices),
                  tolerances=dict(hidden_value_atol=HIDDEN_ATOL, hidden_value_rtol=HIDDEN_RTOL,
                                  actor_logprob_atol=LP_ATOL),
                  limitations=[
                      'Diagnostic completion never accepts chunk4 or changes the existing failed parity result.',
                      'Board error uses hidden tolerance as a descriptive scale only; the original output gates are unchanged.',
                      'The sampler compares frames in batch/time/lane order and stops at the first valid-frame violation.',
                      'Chunk boards are prefetched one unroll; visited counts include only valid frames actually compared.',
                      'Rust owns the hard deadline, group PSS limit, GPU lock and immutable source seals.'])
    started = time.perf_counter()
    atomic_json(args.out, report)
    try:
        device = torch.device('cpu') if args.preflight_only else initialize_device(args.device)
        report['device'] = str(device)
        if not args.preflight_only:
            torch.backends.cuda.matmul.allow_tf32 = False
            torch.backends.cudnn.allow_tf32 = False
            torch.use_deterministic_algorithms(True)
            if device.type == 'cuda':
                torch.cuda.reset_peak_memory_stats(device)
        with ExitStack() as resources:
            source_hash = native_source_digest(args.source)
            native_verify_admission(args.rollout_data, args.source, args.admission)
            source = torch.load(args.source, map_location='cpu', weights_only=False)
            net = load_policy(source, device).requires_grad_(False)
            resources.callback(net.clear_structure)
            before = {key: value.detach().cpu().clone() for key, value in net.state_dict().items()}
            dataset = D.TrajectoryDataset4(args.rollout_data, split='train',
                        encoder_version=source['encoder_version'], unroll_len=64, max_shards=0)
            resources.callback(dataset.store.close)
            admission = select_learner_histories(dataset, args.admission, source_hash)
            if dataset.structure.digest != net.rules.digest:
                raise ValueError('Source and dataset rule catalogs differ')
            report.update(source_checkpoint_sha256=source_hash, dataset_fingerprint=dataset.fingerprint,
                          catalog_digest=dataset.structure.digest)
            per_history = inspect_histories(dataset)
            frames = sum(row['frames'] for row in per_history)
            choices = sum(row['choices'] for row in per_history)
            validate_counts(args, len(per_history), frames, choices)
            report.update(learner_histories=len(per_history), frames=frames, choices=choices,
                          per_history=per_history, all_histories_loaded_and_collated=True,
                          source_catalog_compatible=True)
            if args.preflight_only:
                report.update(cuda_used=False, model_forward_calls=0)
            else:
                replay = prepare_policy_targets(net, dataset, device, batch_size=4, board_chunk=0)
                resources.callback(dataset._policy_targets_directory.cleanup)
                check_audit(admission, replay)
                validate_counts(args, replay['learner_trajectories'], replay['frames'], replay['decisions_checked'])
                report['probability_replay'] = replay
                atomic_json(args.out, report)
                result = diagnose(net, dataset, episode_prefixes(dataset), frame_identities(dataset), device)
                if result['full_scan']:
                    validate_counts(args, len(result['visited_histories']), result['visited_frames'], result['visited_choices'])
                elif result['first_failure'] is None:
                    raise ValueError('Diagnostic neither located a violation nor scanned the complete collection')
                if (result['visited_frames'] > args.expected_frames or result['visited_choices'] > args.expected_choices
                        or len(result['visited_histories']) > args.expected_histories):
                    raise ValueError('Diagnostic exceeded declared native coverage')
                report.update(result=result, observed_violation=result['first_failure'] is not None,
                    diagnostic_completion='first_violation_located' if result['first_failure'] is not None
                                          else 'complete_collection_scanned')
                if device.type == 'cuda':
                    report.update(cuda_peak_allocated_bytes=torch.cuda.max_memory_allocated(device),
                                  cuda_peak_reserved_bytes=torch.cuda.max_memory_reserved(device),
                                  cuda_allocator_retries=torch.cuda.memory_stats(device).get('num_alloc_retries', 0))
                    if report['cuda_allocator_retries']:
                        raise RuntimeError('Allocator retry; diagnostic does not retry or change shape')
            if (not all(torch.equal(before[key], value.detach().cpu()) for key, value in net.state_dict().items())
                    or any(parameter.grad is not None for parameter in net.parameters())):
                raise ValueError('Diagnostic mutated source state or created gradients')
            report['source_model_state_unchanged'] = True
        native_verify_admission(args.rollout_data, args.source, args.admission)
        if native_source_digest(args.source) != source_hash:
            raise ValueError('Source checkpoint changed during diagnostic')
        report.update(status='complete', elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
        print(f'[chunk-diagnostic] {report["mode"]} complete; parity_accepted=false', flush=True)
    except BaseException as error:
        report.update(status='failed', error=str(error), elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
        raise
