"""Fixed-weight model/tensor adapter; Rust owns admission and process budgets."""

import argparse
from contextlib import ExitStack
from pathlib import Path
import time

import torch

import dataset4 as D
from policy_training4.admission import check_audit, select_learner_histories
from policy_training4.targets import prepare_policy_targets
from recurrent_ppo4.checkpoint import (load_policy, native_source_digest,
                                      native_verify_admission, validate_resume_probability)
from recurrent_ppo4.forward import episode_prefixes
from torch_io4 import atomic_json
from value_calibration4.model_identity import source_identity
from .comparison import (HIDDEN_ATOL, HIDDEN_RTOL, KL_ATOL, LP_ATOL, TARGET_KL,
                         compare, validate_coverage)
from .measurement import PhaseTimer, frame_identities, measure_pass


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'candidate', 'rollout-data', 'admission', 'out'):
        parser.add_argument('--'+name, type=Path, required=True)
    parser.add_argument('--device', choices=('cpu', 'cuda'), default='cuda')
    parser.add_argument('--preflight-only', action='store_true',
                        help='CPU checkpoint/dataset/collation validation, with no model forward')
    parser.add_argument('--unroll', type=int, choices=(64,), default=64)
    parser.add_argument('--audit-lanes', nargs='+', type=int, default=[4, 8])
    parser.add_argument('--board-chunks', nargs='+', type=int, default=[0, 4])
    parser.add_argument('--warmups', type=int, choices=(0, 1), default=0)
    parser.add_argument('--repeats', type=int, choices=(1, 2), default=1)
    for name in ('histories', 'choices', 'frames'):
        parser.add_argument('--expected-'+name, type=int, required=True)
    args = parser.parse_args(argv)
    if (not set(args.audit_lanes) <= {4, 8} or 4 not in args.audit_lanes
            or len(set(args.audit_lanes)) != len(args.audit_lanes)
            or not set(args.board_chunks) <= {0, 4} or 0 not in args.board_chunks
            or len(set(args.board_chunks)) != len(args.board_chunks)
            or min(args.expected_histories, args.expected_choices, args.expected_frames) < 1):
        parser.error('Require unique lanes 4/8 and chunks 0/4 including baseline 4/0, positive coverage')
    if args.out.exists():
        parser.error('Preserving existing report; choose a new output file')
    if args.preflight_only and args.device != 'cpu':
        parser.error('Preflight requires explicit --device cpu and never reserves CUDA')
    return args


def validate_candidate(saved, source, source_hash, fingerprint):
    validate_resume_probability(saved)
    contract = saved['full_ppo_contract']
    if (contract.get('source_checkpoint_sha256') != source_hash
            or contract.get('dataset_fingerprint') != fingerprint
            or contract.get('collection_audit') != 'current_policy_streaming_v1'
            or contract.get('precision') != 'float32'
            or contract.get('batch_size') != 4 or contract.get('unroll') != 64
            or contract.get('target_kl') != TARGET_KL
            or source_identity(saved) != source_identity(source)):
        raise ValueError('Candidate does not belong to the declared B4/U64 source/collection')
    progress = saved.get('full_ppo_progress', {})
    if progress.get('final_gate') != 'accepted':
        raise ValueError('Candidate lacks an accepted final whole-collection gate')


def timing_delta(reference, actual):
    phases = set(reference['phase_seconds']) | set(actual['phase_seconds'])
    result = dict(wall_seconds_delta=actual['wall_seconds']-reference['wall_seconds'],
                  wall_time_ratio=actual['wall_seconds']/reference['wall_seconds'],
                  phase_seconds_delta={name: actual['phase_seconds'].get(name, 0.)
                                       -reference['phase_seconds'].get(name, 0.)
                                       for name in sorted(phases)})
    for name in ('cuda_peak_allocated_bytes', 'cuda_peak_reserved_bytes'):
        if name in actual:
            result[name+'_delta'] = actual[name]-reference[name]
    return result


def main(argv=None):
    args = parse_args(argv)
    if args.preflight_only:
        from .preflight import run_preflight
        run_preflight(args)
        return
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    torch.use_deterministic_algorithms(True)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    device = torch.device(args.device)
    if device.type == 'cuda':
        total_memory = torch.cuda.get_device_properties(device).total_memory
        torch.cuda.set_per_process_memory_fraction(min(1., 11 * 2**30 / total_memory), device)
    report = dict(version=1, status='running', optimizer_constructed=False,
                  optimizer_steps=0, parameter_updates=0, promoted=False,
                  optimizer_group_contract=dict(batch_size=4, unroll=64, changed=False),
                  precision='float32', tf32=False, threads=4, interop_threads=1,
                  expected=dict(histories=args.expected_histories, frames=args.expected_frames,
                                choices=args.expected_choices),
                  tolerances=dict(logprob_atol=LP_ATOL, hidden_value_atol=HIDDEN_ATOL,
                                  hidden_value_rtol=HIDDEN_RTOL, kl_atol=KL_ATOL,
                                  unchanged_target_kl=TARGET_KL),
                  warmups=args.warmups, repeats=args.repeats, passes=[],
                  completed_configurations=0, required_configurations=2*len(args.audit_lanes)*len(args.board_chunks),
                  limitations=[
                      'Instrumented fixed-weight audits only; no optimizer/Adam/backward capacity result.',
                      'Wall time includes parity copies and phase synchronization; phase sums exclude scheduling gaps.',
                      'Order is source then candidate, lanes ascending then chunks ascending; caches/order can affect timings.',
                      'Zero warmups/single repeat is cold-inclusive evidence, not a speedup estimate.',
                      'Group PSS and hard 900s deadline are enforced/reported by the owning Rust launcher.',
                      'Torch allocator cap is 11GiB or physical capacity when smaller; allocator stats exclude other device users.'])
    atomic_json(args.out, report)
    started = time.perf_counter()
    try:
        with ExitStack() as resources:
            timer = PhaseTimer(device)
            with timer.scope('source_load'):
                source_hash = native_source_digest(args.source)
                candidate_hash = native_source_digest(args.candidate)
                source = torch.load(args.source, map_location='cpu', weights_only=False)
                net = load_policy(source, device).requires_grad_(False)
            with timer.scope('admission_and_dataset'):
                native_verify_admission(args.rollout_data, args.source, args.admission)
                dataset = D.TrajectoryDataset4(args.rollout_data, split='train',
                            encoder_version=source['encoder_version'], unroll_len=64, max_shards=0)
                resources.callback(dataset.store.close)
                if dataset.structure.digest != net.rules.digest:
                    raise ValueError('Dataset and model rule catalogs differ')
                admission = select_learner_histories(dataset, args.admission, source_hash)
            with timer.scope('original_behavior_replay_and_targets'):
                replay = prepare_policy_targets(net, dataset, device, batch_size=4, board_chunk=0)
                resources.callback(dataset._policy_targets_directory.cleanup)
                check_audit(admission, replay)
                if (replay['learner_trajectories'] != args.expected_histories
                        or replay['frames'] != args.expected_frames
                        or replay['decisions_checked'] != args.expected_choices):
                    raise ValueError('Original replay differs from declared full exposure')
            report.update(source_checkpoint_sha256=source_hash, candidate_checkpoint_sha256=candidate_hash,
                          dataset_fingerprint=dataset.fingerprint, probability_replay=replay,
                          preparation_phase_seconds=timer.seconds)
            atomic_json(args.out, report)
            identities = frame_identities(dataset)
            prefixes = episode_prefixes(dataset)
            for label, checkpoint in (('source', args.source), ('candidate', args.candidate)):
                if label == 'candidate':
                    saved = torch.load(checkpoint, map_location='cpu', weights_only=False)
                    validate_candidate(saved, source, source_hash, dataset.fingerprint)
                    net.clear_structure()
                    net.load_state_dict(saved['state_dict'], strict=True)
                    del saved
                before = {key: value.detach().cpu().clone() for key, value in net.state_dict().items()}
                reference = None
                for lanes in sorted(args.audit_lanes):
                    for chunk in sorted(args.board_chunks):
                        for iteration in range(args.warmups + args.repeats):
                            warmup = iteration < args.warmups
                            snapshot = measure_pass(net, dataset, prefixes, identities, device, lanes, chunk)
                            validate_coverage(snapshot, args.expected_histories,
                                              args.expected_frames, args.expected_choices)
                            if reference is None:
                                reference = snapshot
                            parity = compare(reference, snapshot)
                            row = dict(checkpoint=label, audit_lanes=lanes, board_chunk=chunk,
                                       warmup=warmup, repeat=iteration-args.warmups,
                                       **snapshot.report, parity=parity,
                                       baseline_delta=timing_delta(reference.report, snapshot.report))
                            if label == 'source':
                                replay_delta = max(abs(lp-old) for lp, old in snapshot.actions.values())
                                row['max_recorded_behavior_replay_error'] = replay_delta
                                if replay_delta > replay['tolerance']:
                                    parity['passed'] = False
                            report['passes'].append(row)
                            if not parity['passed']:
                                raise ValueError('Fixed-weight complete-history parity gate failed')
                            if not warmup and iteration == args.warmups+args.repeats-1:
                                report['completed_configurations'] += 1
                            report['elapsed_seconds'] = time.perf_counter()-started
                            atomic_json(args.out, report)
                            print(f'[ppo-probe] {label} lanes={lanes} chunk={chunk} warmup={warmup} '
                                  f'wall={snapshot.report["wall_seconds"]:.6f}s parity=passed', flush=True)
                        del snapshot
                if (not all(torch.equal(before[key], value.detach().cpu()) for key, value in net.state_dict().items())
                        or any(parameter.grad is not None for parameter in net.parameters())):
                    raise ValueError('Fixed-weight probe mutated model state or created gradients')
                report[label+'_model_state_unchanged'] = True
                del before, reference
            native_verify_admission(args.rollout_data, args.source, args.admission)
            if native_source_digest(args.candidate) != candidate_hash:
                raise ValueError('Candidate checkpoint changed during the probe')
        if report['completed_configurations'] != report['required_configurations']:
            raise ValueError('Incomplete configuration coverage')
        report.update(status='complete', elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
    except BaseException as error:
        report.update(status='failed', error=str(error), elapsed_seconds=time.perf_counter()-started)
        atomic_json(args.out, report)
        raise
