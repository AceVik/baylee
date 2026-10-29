"""Complete learner histories, frozen FP32 behavior audit and Monte Carlo targets."""
from __future__ import annotations
import tempfile
import mmap
import zlib
import orjson
from pathlib import Path
import numpy as np
import torch
from torch.utils.data import DataLoader
import dataset4 as D
import model4 as M
from trajectory_store4 import BST4_HDR, BST4_MAGIC
from train_rl4 import action_logprob_and_kl
from training4.execution import encode_boards


def actor_mask(batch):
    """Only recorded stochastic neural choices with at least one free choice."""
    base = (batch['step_valid'] & batch['is_decision'] & (batch['source_kind'] == 0)
            & batch['has_logprob'] & (batch['temperature'] > 1e-4))
    choices = ((batch['head'] == 0) & (batch['cand_mask'].sum(-1) > 1))
    choices |= ((batch['head'] == 1) & (batch['temperature'] > 0.35)
                & ((batch['cand_free_mask'] > 0) & batch['cand_mask']).any(-1))
    choices |= ((batch['head'] == 2) & (batch['temperature'] > 0.35)
                & ((batch['pair_free_mask'] > 0) & batch['pair_mask']).any(-1))
    return base & choices


def _raw_choice(step):
    p = step.get('provenance', {})
    if not (step.get('is_decision') and p.get('source') in ('Net', 0) and p.get('has_logprob')):
        return False
    temperature = float(p.get('temperature', float('nan')))
    logprob = float(p.get('behavior_logprob', float('nan')))
    if not np.isfinite(temperature) or temperature <= 1e-4 or not np.isfinite(logprob):
        raise ValueError('Malformed stochastic neural provenance: temperature/logprob')
    head = int(step['head'])
    if head == 0:
        return len(step['cand_feat']) > 1
    if head in (1, 2):
        size = len(step['cand_feat'] if head == 1 else step['pairs'])
        target = step.get('target', {})
        for name in ('multi_mask', 'free_mask'):
            mask = target.get(name)
            if not isinstance(mask, list) or len(mask) != size or any(
                    not isinstance(x, (int, float)) or not np.isfinite(x) or x not in (0, 1) for x in mask):
                raise ValueError('Malformed stochastic neural ' + name)
        return temperature > 0.35 and any(target['free_mask'])
    if head == 3:
        raise ValueError('Scalar action falsely claims stochastic behavior probability')
    raise ValueError('Unknown stochastic neural action head')


def _bst4_choice(blob: bytes) -> bool:
    (
        _magic, _ver, _game_id, _seat, _step_cursor, _turn, flags, head, _pending_kind,
        src_kind, _pad0, _k, _single_idx, _loss_weight, _scalar_norm, _outcome,
        _aux0, _aux1, _aux2, _aux3, logprob, temperature,
        _n_ent, _ent_width, _ent_rule_cols, _seat_width, _global_width, _n_edg,
        n_cand, _cand_width, _cand_rule_cols, n_pair, n_mm, n_fm,
    ) = BST4_HDR.unpack_from(blob, 0)
    is_decision = bool(flags & 1)
    has_logprob = bool(flags & 4)
    if not (is_decision and src_kind == 0 and has_logprob):
        return False
    if not np.isfinite(temperature) or temperature <= 1e-4 or not np.isfinite(logprob):
        raise ValueError('Malformed stochastic neural provenance: temperature/logprob')
    if head == 0:
        return n_cand > 1
    if head in (1, 2):
        size = n_cand if head == 1 else n_pair
        if n_mm != size or n_fm != size:
            raise ValueError('Malformed stochastic neural mask length')
        mm = blob[len(blob) - n_mm - n_fm : len(blob) - n_fm]
        fm = blob[len(blob) - n_fm :]
        if any(b not in (0, 1) for b in mm) or any(b not in (0, 1) for b in fm):
            raise ValueError('Malformed stochastic neural mask values')
        return temperature > 0.35 and any(fm)
    if head == 3:
        raise ValueError('Scalar action falsely claims stochastic behavior probability')
    raise ValueError('Unknown stochastic neural action head')


def _learner_histories(dataset):
    # Provenance is the final JSON member in converter records, or packed in BST4_HDR.
    stream = mapping = None
    try:
        if hasattr(dataset.store, 'data_path'):
            stream = dataset.store.data_path.open('rb')
            mapping = mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ)
        selected = []
        for trajectory in dataset.trajectories:
            found = False
            for segment in trajectory:
                for row in dataset.segments[segment]:
                    rec = dataset.store.index[int(row)]
                    if not int(rec['flags']) & 1:
                        continue
                    if mapping is None:
                        step = dataset.store.read(int(row))
                        if '_bst4' in step:
                            if _bst4_choice(step['_bst4']):
                                found = True
                            continue
                    else:
                        start = int(rec['offset'])
                        blob = zlib.decompress(mapping[start:start+int(rec['length'])])
                        if blob[:4] == BST4_MAGIC:
                            if _bst4_choice(blob):
                                found = True
                            continue
                        pos = blob.rfind(b'"provenance"')
                        provenance = None
                        if pos >= 0:
                            try:
                                provenance = orjson.loads(blob[blob.index(b':',pos)+1:].rstrip()[:-1])
                            except orjson.JSONDecodeError:
                                pass
                        if provenance is not None and not (
                            provenance.get('source') in ('Net',0) and provenance.get('has_logprob')):
                            continue
                        step = orjson.loads(blob)
                    # Validate every stochastic row, including rows after the
                    # first eligible choice; Dataset4's legacy mask repair must
                    # never invent free PPO decisions.
                    if _raw_choice(step):
                        found = True
            if found:
                selected.append(trajectory)
        return selected
    finally:
        if mapping is not None:
            mapping.close()
            stream.close()


@torch.no_grad()
def prepare_policy_targets(net, dataset, device, batch_size=8, board_chunk=4, tolerance=.001):
    """Audit all genuine learner choices before exposing normalized advantages.

    Original segment IDs remain stable. Sparse disk arrays bound resident cache
    memory; their TemporaryDirectory is retained by the dataset for its lifetime.
    Teacher-only trajectories are removed from the in-memory sampler list only.
    """
    from trajectory_store4 import reject_shadow_policy_gradient
    reject_shadow_policy_gradient(dataset.meta)
    if not np.isfinite(tolerance) or not 0 < tolerance <= .002:
        raise ValueError('Behavior audit tolerance must be finite and at most 0.002')
    if hasattr(dataset, 'ppo_cache'):
        raise ValueError('Refusing to reuse a dataset with existing policy targets')
    # Version 2 certifies exact sample-to-action decoding; version 1 predates
    # that gate. Numerical LP parity alone cannot certify the executed action.
    if dataset.meta.get('policy_provenance_version') != 2:
        raise ValueError('PPO requires certified raw-policy provenance version 2; archived data remains BC-only')
    if dataset.meta.get('sampling_version') != M.SAMPLING_VERSION:
        raise ValueError('Incompatible recorded sampling contract')
    if any(p.dtype != torch.float32 for p in net.parameters()):
        raise ValueError('Frozen target preparation requires FP32 parameters')
    original = dataset.trajectories
    selected = _learner_histories(dataset)
    if not selected:
        raise ValueError('No genuine stochastic learner trajectories')
    dataset.trajectories = selected
    temporary = tempfile.TemporaryDirectory(prefix='baylee-policy-targets-')
    cache = {}
    n, u, width = len(dataset), dataset.unroll_len, net.cfg.d_model
    for name, dtype, shape in [('advantages', np.float32, (n,u)), ('returns', np.float32,(n,u)),
                               ('behavior_lp', np.float32, (n,u)),
                               ('actor_mask',np.bool_,(n,u)), ('behavior_h',np.float32,(n,9,width))]:
        cache[name] = np.memmap(Path(temporary.name)/name, mode='w+', dtype=dtype, shape=shape)
    loader = DataLoader(dataset, batch_sampler=D.StatefulBatchSampler4(dataset,batch_size,shuffle=False),
                        collate_fn=D.collate_trajectories)
    h = None
    checked = frames = 0
    max_error = 0.
    minimum_value, maximum_value = float('inf'),float('-inf')
    net.eval()
    try:
        net.clear_structure()
        for batch_index, host in enumerate(loader):
            net.clear_structure()
            net.prepare_structure((host['entity_rules'], host['cand_rules']))
            ids = host['segment_index'].tolist()
            b = len(ids)
            if h is None:
                h = torch.zeros(b,9,width,device=device)
            h = torch.where(host['is_new_traj'].to(device)[:,None,None],torch.zeros_like(h),h)
            entry = h.cpu().numpy()
            for lane, idx in enumerate(ids):
                if idx >= 0:
                    cache['behavior_h'][idx] = entry[lane]
            boards = encode_boards(net,host,board_chunk,device)
            mask = actor_mask(host)
            invalid_k = ((host['thinking_steps'] < 1) | (host['thinking_steps'] > 8)) & mask
            if bool(invalid_k.any()):
                raise ValueError('Genuine neural decision has invalid recorded thinking depth')
            values, errors = [], []
            for t in range(u):
                decision = host['is_decision'][:,t].to(device)
                recorded = host['thinking_steps'][:,t].to(device)
                source = host['source_kind'][:,t].to(device) == 0
                k = torch.where(decision,torch.where(source & (recorded > 0),recorded,1),0)
                heads = tuple(bool((mask[:,t] & (host['head'][:,t] == kind)).any()) for kind in range(4))
                output = net(*[host[name][:,t].to(device) for name in M.INPUT_NAMES[:-2]],h,k,
                             policy_heads=heads, encoded_board=None if boards is None else boards[t])
                valid = host['step_valid'][:,t].to(device)
                h = torch.where(valid[:,None,None],output[0],h)
                lp,_ = action_logprob_and_kl(output[1].float(),output[2].float(),output[3].float(),
                                            output[1].float(),output[2].float(),output[3].float(),host,t)
                errors.append((lp-host['behavior_logprob'][:,t].to(device)).abs())
                values.append(output[4].float())
            value = torch.stack(values,1).cpu().numpy()
            error = torch.stack(errors,1).cpu().numpy()
            keep = mask.numpy()
            observed = error[keep]
            if len(observed):
                if not np.isfinite(observed).all() or float(observed.max()) > tolerance:
                    raise ValueError(f'Frozen behavior probability mismatch before updates: max_error={observed.max()}')
                max_error = max(max_error,float(observed.max()))
                checked += len(observed)
            valid = host['step_valid'].numpy()
            if not np.isfinite(value[valid]).all():
                raise ValueError('Nonfinite frozen critic value')
            minimum_value=min(minimum_value,float(value[valid].min()))
            maximum_value=max(maximum_value,float(value[valid].max()))
            truth = host['outcome'].numpy()
            for lane,idx in enumerate(ids):
                if idx >= 0:
                    cache['actor_mask'][idx] = keep[lane]
                    # Replay certifies the behavior policy; it must not replace
                    # the sampled policy's recorded PPO denominator/KL anchor.
                    cache['behavior_lp'][idx] = np.where(keep[lane],host['behavior_logprob'][lane].numpy(),0)
                    cache['returns'][idx] = np.where(valid[lane],truth[lane],0)
                    cache['advantages'][idx] = np.where(keep[lane],truth[lane]-value[lane],0)
            frames += int(valid.sum())
            if batch_index % 100 == 0:
                print(f'[policy-targets] batches={batch_index+1}/{len(loader)} frames={frames} choices={checked}',flush=True)
        # Dataset4 already validates complete contiguous terminal trajectories;
        # additionally refuse inconsistent per-frame terminal outcome labels.
        total = total2 = 0.
        count = 0
        for tr in selected:
            target = None
            for idx in tr:
                size=len(dataset.segments[idx]);ret=cache['returns'][idx,:size]
                if target is None:target=float(ret[0])
                if not 0 <= target <= 1 or not np.all(ret == target):
                    raise ValueError('Inconsistent terminal outcome within learner trajectory')
                chosen=cache['advantages'][idx][cache['actor_mask'][idx]].astype(np.float64)
                total+=float(chosen.sum());total2+=float((chosen*chosen).sum());count+=len(chosen)
        if count != checked or not count:
            raise ValueError('No audited genuine policy choices')
        mean=total/count;std=max((max(0,total2/count-mean*mean))**.5,1e-5)
        for tr in selected:
            for idx in tr:
                mask=cache['actor_mask'][idx]
                cache['advantages'][idx,mask]=(cache['advantages'][idx,mask]-mean)/std
        for array in cache.values():array.flush()
        dataset.ppo_cache=cache
        dataset._policy_targets_directory=temporary
        return dict(learner_trajectories=len(selected),excluded_teacher_trajectories=len(original)-len(selected),
                    frames=frames,decisions_checked=checked,max_logprob_error=max_error,tolerance=tolerance,
                    precision='float32',returns='complete terminal side outcome; gamma=lambda=1',
                    advantage_mean_before_normalization=mean,advantage_std_before_normalization=std,
                    critic_value_min=minimum_value,critic_value_max=maximum_value,
                    cache_storage='sparse disk-backed arrays indexed by original segment ID')
    except BaseException:
        dataset.trajectories=original
        cache.clear()
        temporary.cleanup()
        raise
    finally:
        net.clear_structure()
