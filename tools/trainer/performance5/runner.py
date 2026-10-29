"""Bounded matched v4/v5 CUDA probes; no checkpoint is written or promoted."""
import argparse
import gc
import json
from pathlib import Path
import time

import torch
from torch.utils.data import DataLoader

import dataset4 as D
from benchmark_state4 import checkpoint_sampler
from checkpoint4 import restore_rng
from torch_io4 import atomic_json, file_digest as checksum
from training4.execution import compile_regions
from training4.prefetch import prefetch_batches
from . import precision
from .state import restore
from .update import capture, difference, update


VARIANTS = {
    'baseline': dict(saved=0, prefetch=1, board=4),
    'baseline-repeat': dict(saved=0, prefetch=1, board=4),
    'compact-thinking':dict(saved=0,prefetch=1,board=4,compact=True),
    'saved1': dict(saved=1, prefetch=1, board=4),
    'saved2': dict(saved=2, prefetch=1, board=4),
    'prefetch2': dict(saved=0, prefetch=2, board=4),
    'board8': dict(saved=0, prefetch=1, board=8),
}


def configure(net, settings, compile_enabled):
    if not net.cfg.activation_checkpointing or settings['saved'] > net.cfg.n_layers:
        raise ValueError('Probe requires activation checkpointing and sufficient layers')
    net.compact_thinking_rows = settings.get('compact',False)
    net.saved_activation_layers = settings['saved']
    net.cache_thinking_kv = True
    net.prune_unused_policy_heads = True
    net.decision_chunk = 4
    if compile_enabled:
        compile_regions(net)


def batches_for(saved, args):
    ds = D.TrajectoryDataset4(Path(args.data), split='train', unroll_len=saved['unroll'])
    if ds.fingerprint != saved['dataset_fingerprint']:
        raise ValueError('Probe data differ from checkpoint')
    sampler = checkpoint_sampler(ds, saved, saved['batch_size'], fresh=not args.resume_lanes, seed=args.sampler_seed)
    loader = DataLoader(ds, batch_sampler=sampler, collate_fn=D.collate_trajectories,
                        num_workers=args.workers, pin_memory=False,
                        **({'prefetch_factor':1} if args.workers else {}),
                        generator=torch.Generator().manual_seed(19417))
    batches, waits, total_bytes = [], [], 0
    iterator = iter(loader)
    try:
        for _ in range(args.warmup + args.steps):
            begin = time.perf_counter()
            batch = next(iterator)
            total_bytes += sum(t.numel() * t.element_size() for t in batch.values())
            if total_bytes > args.host_limit_gib * 1024**3:
                raise MemoryError('Pinned input probe budget exceeded')
            batches.append({key:t.pin_memory() for key,t in batch.items()})
            waits.append(time.perf_counter()-begin)
    finally:
        # DataLoader iterator destruction joins its workers, also on failure.
        del iterator, loader
    return batches, waits


def run(args):
    destination = Path(args.out)
    if destination.exists():
        raise FileExistsError(destination)
    if args.steps < 2 or args.warmup < 2 or args.workers < 0 or args.host_limit_gib <= 0 or args.timeout_seconds <= 0:
        raise ValueError('Invalid bounded probe settings')
    names = args.variants.split(',')
    if names[0] != 'baseline' or any(n not in VARIANTS for n in names):
        raise ValueError('Baseline must be first; unknown variant')
    torch.set_num_threads(4)
    precision_policy = getattr(args, 'float32_precision', 'inherit')
    precision_flags = precision.configure(precision_policy, not args.no_compile)
    if len(names)!=len(set(names)):
        raise ValueError('Duplicate variants')
    destination.parent.mkdir(parents=True,exist_ok=True)
    device = torch.device('cuda')
    if not torch.cuda.is_bf16_supported():
        raise RuntimeError('This probe requires BF16 CUDA')
    checkpoint_sha=checksum(args.checkpoint)
    saved = torch.load(args.checkpoint, map_location='cpu', weights_only=False)
    if checksum(args.checkpoint)!=checkpoint_sha:raise RuntimeError('Checkpoint changed while loading')
    batches, waits = batches_for(saved, args)
    if args.resume_lanes and saved['next_batch'] and saved.get('h') is None:
        raise ValueError('Resumed sampler lanes require checkpoint memory')
    initial_h = (saved['h'].to(device).clone() if args.resume_lanes and saved.get('h') is not None else
                 torch.zeros(saved['batch_size'],9,saved['config']['d_model'],device=device))
    if initial_h.shape != (saved['batch_size'],9,saved['config']['d_model']):
        raise ValueError('Checkpoint memory shape differs')
    report = dict(schema=1, scope='Disposable matched updates, declared chronological lanes; no learning or promotion claim',
                  checkpoint=args.checkpoint, checkpoint_sha256=checkpoint_sha,
                  data=args.data, dataset_fingerprint=saved['dataset_fingerprint'],
                  model_version=saved['model_version'], architecture=saved.get('architecture'),
                  batch_size=saved['batch_size'], unroll=saved['unroll'],
                  thinking_steps=saved['config']['default_thinking_steps'],
                  sampler_seed=saved['sampler_seed'] if args.sampler_seed is None else args.sampler_seed,
                  fresh_trajectories=not args.resume_lanes, sampler_start_batch=saved['next_batch'] if args.resume_lanes else 0,
                  data_scope='Finite contiguous checkpoint/fresh window only; fresh lanes emphasize early-game shapes; no full-distribution claim',
                  torch=torch.__version__, gpu=torch.cuda.get_device_name(),
                  float32_precision_policy=precision_policy, precision=precision_flags,
                  parity_scope='FIRST_UNROLL_ONLY',
                  parity_scope_detail='One initial BxU update only; not full-trajectory or production numerical parity',
                  warmup_updates=args.warmup, timed_updates=args.steps,
                  loader_wait_s=waits, loader_wait_scope='Iterator wait plus explicit bounded pinning; outside update timing', timing_scope='Warm pinned-RAM inputs; disk/loader waits reported separately. Includes H2D API/staging; no additional per-update timing synchronization or gradient CPU copies. Production finite-loss and clipping guards retain their synchronization.',
                  batch_mib=[sum(t.numel()*t.element_size() for t in b.values()) / 2**20 for b in batches],
                  max_entities=[b['entity_feat'].shape[2] for b in batches],
                  max_candidates=[b['cand_mask'].shape[2] for b in batches],
                  max_pairs=[b['pair_mask'].shape[2] for b in batches],
                  decision_head_counts={str(head):sum(int((b['step_valid'] & b['is_decision'] & (b['head']==head)).sum()) for b in batches) for head in range(4)},
                  active_parameter_scope='All parameters restored; frozen-trunk checkpoints rejected; absent heads keep None gradients',
                  results=[], promoted=False)
    report['phase']='measuring'
    atomic_json(destination,report)
    reference = None
    deadline=time.monotonic()+args.timeout_seconds
    for name in names:
        settings = VARIANTS[name]
        row = dict(variant=name, settings=settings, started_unix=time.time())
        if name != 'baseline' and reference is None:
            row['rejected']='Baseline correctness reference unavailable'
            report['results'].append(row)
            atomic_json(destination,report)
            continue
        stream = None
        net = optimizer = hidden = loss = None
        losses = []
        candidate = restored = config = host = staged = None
        try:
            if time.monotonic()>deadline:raise TimeoutError('Probe wall budget exceeded between updates')
            restored, net, optimizer, config = restore(Path(args.checkpoint), device)
            if checksum(args.checkpoint)!=checkpoint_sha:raise RuntimeError('Checkpoint changed before variant')
            configure(net, settings, not args.no_compile)
            row['precision'] = precision.require_unchanged(precision_flags, not args.no_compile)
            restore_rng(restored['rng'])
            snapshots=[]
            parity_begin=time.perf_counter()
            hidden, loss = update(net, optimizer, batches[0], None, initial_h.clone(),
                                  restored, config, restored['step_num'], settings['board'],
                                  before_clip=lambda n,h,l:snapshots.append(capture(n,h,l)))
            candidate = snapshots.pop()
            row['untimed_parity_and_first_shape_compile_s']=time.perf_counter()-parity_begin
            row['gradient_parity_scope']='FIRST_UNROLL_ONLY: raw gradients before clipping; disposable parity update discarded'
            precision.require_unchanged(precision_flags, not args.no_compile)
            if reference is None:
                reference = candidate
            row['parity'] = difference(reference, candidate)
            if (row['parity']['gradient_relative_l2'] > args.gradient_tolerance
                    or row['parity']['hidden_max_abs'] > args.hidden_tolerance
                    or row['parity']['loss_abs'] > args.loss_tolerance):
                row['rejected'] = 'Numerical parity tolerance exceeded'
            else:
                # Restore both optimizer AND all weights; parity update is disposable.
                optimizer.zero_grad(set_to_none=True)
                hidden = loss = None
                net.clear_structure()
                net.load_state_dict(restored['state_dict'], strict=True)
                import copy
                from checkpoint4 import move_optimizer
                optimizer.load_state_dict(copy.deepcopy(restored['optimizer']))
                for group in optimizer.param_groups:
                    group['fused'] = True
                move_optimizer(optimizer, device)
                restore_rng(restored['rng'])
                hidden = initial_h.clone()
                stream = prefetch_batches(batches, device, settings['prefetch'])
                losses = []
                warm_begin=time.perf_counter()
                for i in range(args.warmup):
                    if time.monotonic()>deadline:raise TimeoutError('Probe wall budget exceeded between updates')
                    host, staged = next(stream)
                    hidden, loss = update(net, optimizer, host, staged, hidden, restored,
                                          config, restored['step_num'] + i, settings['board'])
                torch.cuda.synchronize()
                row['warmup_including_shape_compiles_s']=time.perf_counter()-warm_begin
                row['timed_scope']='Remaining selected shapes may compile inside timing; not a guaranteed steady state'
                torch.cuda.reset_peak_memory_stats()
                retries = torch.cuda.memory_stats()['num_alloc_retries']
                begin = time.perf_counter()
                for i in range(args.warmup, len(batches)):
                    if time.monotonic()>deadline:raise TimeoutError('Probe wall budget exceeded between updates')
                    host, staged = next(stream)
                    hidden, loss = update(net, optimizer, host, staged, hidden, restored,
                                          config, restored['step_num'] + i, settings['board'])
                    losses.append(loss)
                torch.cuda.synchronize()
                elapsed = time.perf_counter() - begin
                row['precision_after_timing'] = precision.require_unchanged(precision_flags, not args.no_compile)
                measured = batches[args.warmup:]
                row.update(elapsed_s=elapsed, updates_per_s=args.steps/elapsed,
                           valid_frames_per_s=sum(int(b['step_valid'].sum()) for b in measured)/elapsed,
                           decision_frames_per_s=sum(int((b['step_valid'] & b['is_decision']).sum()) for b in measured)/elapsed,
                           losses=torch.stack(losses).cpu().tolist(),
                           peak_allocated_gib=torch.cuda.max_memory_allocated()/2**30,
                           peak_reserved_gib=torch.cuda.max_memory_reserved()/2**30,
                           allocator_retries=torch.cuda.memory_stats()['num_alloc_retries']-retries)
                if row['allocator_retries']:
                    row['rejected'] = 'Allocator retries; insufficient memory margin'
                if args.profile and name == 'baseline':
                    # Separate diagnostic trace, never part of throughput results.
                    with torch.profiler.profile(activities=[torch.profiler.ProfilerActivity.CPU,
                                                            torch.profiler.ProfilerActivity.CUDA]) as prof:
                        hidden, loss = update(net, optimizer, batches[-1], None, hidden,
                                              restored, config, restored['step_num']+len(batches), settings['board'])
                    prof.export_chrome_trace(str(destination.with_suffix('.trace.json')))
                    torch.cuda.synchronize()
                    row['profile'] = prof.key_averages().table(sort_by='self_device_time_total', row_limit=20)
                    row['profile_scope']='Separate extra disposable update after measured updates; profiler context drains trace queues'
        except TimeoutError as error:
            row['rejected']=str(error)
        except torch.cuda.OutOfMemoryError:
            row['rejected'] = 'CUDA out of memory'
        finally:
            if stream is not None:
                stream.close()
            losses.clear()
            if optimizer is not None:optimizer.zero_grad(set_to_none=True)
            if net is not None:net.clear_structure()
            net = optimizer = hidden = loss = stream = None
            candidate = restored = config = host = staged = None
            gc.collect()
            torch.cuda.empty_cache()
        row['finished_unix'] = time.time()
        report['results'].append(row)
        atomic_json(destination, report)
        print(json.dumps({k:v for k,v in row.items() if k not in ('profile', 'losses')}), flush=True)
    if checksum(args.checkpoint) != report['checkpoint_sha256']:
        raise RuntimeError('Probe input checkpoint changed during measurement')
    report['phase'] = 'complete' if all('rejected' not in row for row in report['results']) else 'complete-with-rejections'
    report['accepted_variants']=[row['variant'] for row in report['results'] if 'rejected' not in row and 'elapsed_s' in row]
    atomic_json(destination, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checkpoint', required=True)
    parser.add_argument('--data', required=True)
    parser.add_argument('--out', required=True)
    parser.add_argument('--variants', default=','.join(VARIANTS))
    parser.add_argument('--steps', type=int, default=20)
    parser.add_argument('--warmup', type=int, default=5)
    parser.add_argument('--timeout-seconds',type=float,default=900.,help='Between-update wall budget; use an external run_guard for a hard process deadline')
    parser.add_argument('--workers', type=int, default=2)
    parser.add_argument('--host-limit-gib', type=float, default=3.)
    parser.add_argument('--sampler-seed', type=int)
    parser.add_argument('--resume-lanes', action='store_true', help='Use exact saved sampler cursor/H instead of fresh early-game lanes')
    parser.add_argument('--no-compile', action='store_true')
    parser.add_argument('--float32-precision', choices=('inherit', 'highest'), default='inherit',
                        help='Probe-wide FP32 policy: inherit preserves existing flags; highest disables matmul/cuDNN TF32')
    parser.add_argument('--profile', action='store_true')
    parser.add_argument('--gradient-tolerance', type=float, default=.02)
    parser.add_argument('--hidden-tolerance', type=float, default=.02)
    parser.add_argument('--loss-tolerance', type=float, default=.002)
    run(parser.parse_args())
