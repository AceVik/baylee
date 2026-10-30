"""Bounded full-model research PPO over native, separately admitted tensors."""
import argparse
import copy
import json
import math
import os
from pathlib import Path
import random
from contextlib import ExitStack

import numpy as np
import torch

from checkpoint4 import atomic_save, rng_state, restore_rng
from recurrent_ppo4.checkpoint import load_policy, native_source_digest
from recurrent_ppo4.forward import episode_prefixes
from recurrent_ppo4.optimization import transactional_update, collection_divergence
from recurrent_ppo_probe4.driver import initialize_device
from performance5.precision import configure as configure_precision, require_unchanged
from torch_io4 import atomic_json
from .checkpoint import (VERSION, MEMORY, PROBABILITY, validate_resume,
                         training_payload, restore_training)
from .dataset import ResearchDataset
from .targets import OBJECTIVE, TARGET_CONTRACT, prepare_targets
from .timing import PhaseTimings


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('init', 'cache-dir', 'conversion-sha256', 'cache-sha256', 'pins',
                 'pins-sha256', 'admission', 'out'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--resume')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--audit-only', action='store_true')
    mode.add_argument('--preflight-only', action='store_true')
    parser.add_argument('--device', choices=('cpu', 'cuda'), default='cpu')
    parser.add_argument('--batch-size', type=int, default=4)
    parser.add_argument('--unroll', type=int, default=64)
    parser.add_argument('--epochs', type=int, default=1)
    parser.add_argument('--max-steps', type=int, default=1)
    parser.add_argument('--seed', type=int, default=31431)
    parser.add_argument('--lr', type=float, default=2e-5)
    parser.add_argument('--target-kl', type=float, default=.02)
    parser.add_argument('--clip-eps', type=float, default=.2)
    parser.add_argument('--beta-kl', type=float, default=.02)
    args = parser.parse_args()
    if args.preflight_only and args.device != 'cpu':
        parser.error('Research tensor preflight requires explicit --device cpu')
    if args.resume and (args.preflight_only or args.audit_only):
        parser.error('Diagnostic modes always use the original source, without --resume')
    if (min(args.batch_size, args.unroll, args.epochs, args.max_steps) < 1 or args.unroll > 256
            or not all(math.isfinite(v) for v in (args.lr, args.target_kl, args.clip_eps, args.beta_kl))
            or args.lr <= 0 or not 0 < args.target_kl <= .02 or not 0 < args.clip_eps < 1 or args.beta_kl < 0):
        parser.error('Invalid bounded research PPO settings')
    return args


def configure_execution(device):
    """Set reproducible FP32 math and bound CUDA before loading model tensors."""
    os.environ.setdefault('CUBLAS_WORKSPACE_CONFIG', ':4096:8')
    torch.use_deterministic_algorithms(True)
    torch.backends.cudnn.benchmark = False
    torch.backends.cudnn.deterministic = True
    precision = configure_precision('highest', False)
    return initialize_device(device), precision


def main():
    timings = PhaseTimings()
    args = arguments()
    resume = None
    if args.resume:
        with timings.measure('resume_checkpoint_load'):
            resume = torch.load(args.resume, map_location='cpu', weights_only=False)
            validate_resume(resume)
    out = Path(args.out).expanduser()
    if out.exists() and not args.resume:
        raise FileExistsError('Preserving existing research output; use a new directory or exact --resume')
    device, precision = configure_execution(args.device)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    torch.manual_seed(args.seed)
    random.seed(args.seed)
    np.random.seed(args.seed)
    source_path = Path(args.init).expanduser()
    if source_path.is_dir():
        source_path /= 'net.pt'
    with timings.measure('source_checkpoint_digest'):
        source_sha = native_source_digest(source_path)
    with timings.measure('source_checkpoint_load'):
        source = torch.load(source_path, map_location='cpu', weights_only=False)
    with timings.measure('policy_construction'):
        net = load_policy(source, device)
    with ExitStack() as resources:
        with timings.measure('native_handshake_dataset_preparation'):
            data = ResearchDataset(args.cache_dir, args.conversion_sha256, args.cache_sha256,
                                   args.pins, args.pins_sha256, args.admission, args.unroll)
        resources.callback(data.close)
        if (data.metadata['source_checkpoint_sha256'] != source_sha
                or data.metadata['encoder_version'] != source['encoder_version']
                or data.metadata['structure_sha256'] != net.rules.digest):
            raise ValueError('Native research tensors and source policy identity differ')
        if source['encoder_version'] != 7:
            raise ValueError('First research objective pilot keeps the approved E7 observation contract')
        if args.preflight_only:
            from .preflight import check_tensors
            with timings.measure('preflight_tensor_materialization_validation'):
                report = check_tensors(data, args.batch_size)
            with timings.measure('final_verification'):
                data.verify()
                require_unchanged(precision, False)
                if native_source_digest(source_path) != source_sha:
                    raise ValueError('Source checkpoint changed during research preflight')
            report.update(source_checkpoint_sha256=source_sha, encoder_version=source['encoder_version'],
                          structure_sha256=net.rules.digest, admission=data.admission,
                          catalog=data.metadata['catalog'], precision=precision,
                          automatic_promotion=False)
            out.mkdir(parents=True, exist_ok=True)
            report['timings'] = timings.snapshot()
            atomic_json(out / 'report.json', report)
            return
        with timings.measure('complete_history_replay_target_preparation'):
            audit = prepare_targets(net, data, device, args.batch_size)
        resources.callback(data._policy_targets_directory.cleanup)
        out.mkdir(parents=True, exist_ok=True)
        audit['timings'] = timings.snapshot()
        with timings.measure('publication'):
            atomic_json(out / 'probability-replay.json', audit)
        print('[research-replay] ' + json.dumps(audit, sort_keys=True), flush=True)
        if args.audit_only:
            with timings.measure('final_verification'):
                data.verify()
                require_unchanged(precision, False)
            atomic_json(out / 'report.json', dict(phase='audit-complete', training=False,
                        automatic_promotion=False, audit=audit, admission=data.admission,
                        precision=precision, timings=timings.snapshot()))
            return
        # An explicitly small fixture is useful for tests but is never a live
        # learning admission. One transaction may use a subset of these choices.
        if audit['decisions_checked'] < 128:
            raise ValueError('Research pilot needs at least 128 genuine audited choices')
        contract = dict(version=VERSION, objective=OBJECTIVE, target_contract=TARGET_CONTRACT,
                        behavior_probability_reference=PROBABILITY,
                        source_checkpoint_sha256=source_sha, dataset_fingerprint=data.fingerprint,
                        admission=data.admission, pins_sha256=args.pins_sha256,
                        encoder_version=source['encoder_version'], precision='float32',
                        precision_flags=precision, cuda_allocator_limit_bytes=11 * 2**30,
                        memory=MEMORY, bootstrap=False, advantage_centering=False,
                        deterministic_algorithms=True, activation_checkpointing=True,
                        torch_threads=torch.get_num_threads(),
                        torch_interop_threads=torch.get_num_interop_threads(),
                        torch_version=str(torch.__version__), device=str(device),
                        batch_size=args.batch_size, unroll=args.unroll, seed=args.seed,
                        epochs=args.epochs, lr=args.lr, target_kl=args.target_kl,
                        clip_eps=args.clip_eps, beta_kl=args.beta_kl,
                        collection_audit='current_policy_streaming_v1', anchor_mix_weight=0.)
        optimizer = torch.optim.AdamW(net.parameters(), lr=args.lr, weight_decay=0)
        source_optimizer = copy.deepcopy(optimizer.state_dict())
        source_rng = rng_state()
        progress = dict(epoch=0, next_batch=0, attempts=0, accepted=0, updates=[], terminal=False)
        if resume is not None:
            progress = restore_training(resume, net, optimizer, contract, device)
        prefixes = episode_prefixes(data)
        segments = [index for trajectory in data.trajectories for index in trajectory]

        def save():
            with timings.measure('publication'):
                atomic_save(training_payload(net, optimizer, source, contract, progress), out / 'last.pt')

        stopped = bool(progress['terminal'])
        for epoch in range(progress['epoch'], args.epochs):
            if stopped:
                break
            generator = torch.Generator().manual_seed(args.seed + epoch)
            order = torch.randperm(len(segments), generator=generator).tolist()
            groups = [tuple(segments[i] for i in order[start:start + args.batch_size])
                      for start in range(0, len(order), args.batch_size)]
            start_batch = progress['next_batch'] if epoch == progress['epoch'] else 0
            for batch_index in range(start_batch, len(groups)):
                if progress['attempts'] >= args.max_steps:
                    stopped = True
                    break
                with timings.measure('update_transactions'):
                    record = transactional_update(net, optimizer, data, groups[batch_index], prefixes, device,
                                                  args.target_kl, args.clip_eps, args.batch_size, args.beta_kl)
                progress['attempts'] += 1
                progress['accepted'] += int(record['accepted'])
                progress['epoch'] = epoch
                progress['next_batch'] = batch_index + 1
                progress['updates'].append(dict(epoch=epoch, batch=batch_index, **record))
                print('[research-ppo-step] ' + json.dumps(progress['updates'][-1], sort_keys=True), flush=True)
                if not record['accepted']:
                    progress['terminal'] = stopped = True
                save()
                if stopped:
                    break
            if stopped:
                break
            progress['epoch'], progress['next_batch'] = epoch + 1, 0
            save()
        try:
            with timings.measure('final_collection_replay'):
                final_kl = collection_divergence(net, data, prefixes, device, args.batch_size)
        except FloatingPointError:
            final_kl = float('inf')
        if not math.isfinite(final_kl) or final_kl > args.target_kl:
            net.load_state_dict(source['state_dict'], strict=True)
            optimizer.load_state_dict(source_optimizer)
            restore_rng(source_rng)
            progress.update(accepted=0, terminal=True, final_gate='rejected_published_unchanged_source')
        else:
            progress['final_gate'] = 'accepted'
        progress['final_collection_behavior_kl'] = final_kl if math.isfinite(final_kl) else None
        with timings.measure('final_verification'):
            data.verify()
            require_unchanged(precision, False)
            if native_source_digest(source_path) != source_sha:
                raise ValueError('Source checkpoint changed during research PPO')
        save()
        with timings.measure('publication'):
            atomic_save(training_payload(net, optimizer, source, contract, progress), out / 'net.pt')
        with timings.measure('changed_parameter_comparison'):
            changed_parameters = [name for name, parameter in net.named_parameters()
                                  if not torch.equal(parameter.detach().cpu(), source['state_dict'][name])]
        atomic_json(out / 'report.json', dict(phase='complete', automatic_promotion=False,
                    audit=audit, progress=progress, admission=data.admission,
                    precision=precision, changed_parameters=changed_parameters,
                    changed_parameter_tensors=len(changed_parameters), timings=timings.snapshot()))


if __name__ == '__main__':
    main()
