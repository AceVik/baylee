"""Additive unweighted diagnostics, independent of every training loss version."""
import json
from trajectory_store4 import _native
import numpy as np
import torch
import torch.nn.functional as F

FIELDS = ('frames', 'decisions', 'zero_weight_decisions', 'forced_source_decisions',
          'single_choices', 'single_options', 'single_nll_sum', 'single_correct',
          'pass_targets', 'pass_correct', 'pass_nll_sum', 'active_targets', 'active_correct', 'active_nll_sum',
          'subset_choices', 'subset_free_entries', 'subset_bce_sum', 'subset_decision_bce_sum', 'subset_exact_free_set',
          'pair_choices', 'pair_free_entries', 'pair_bce_sum', 'pair_decision_bce_sum', 'pair_exact_free_set',
          'scalar_decisions', 'scalar_absolute_error_sum', 'scalar_squared_error_sum',
          'critic_squared_error_sum', 'critic_prediction_sum', 'outcome_sum',
          'pass_target_predicted_pass', 'pass_target_predicted_active',
          'active_target_predicted_pass', 'active_target_predicted_active',
          'pass_target_predicted_other_pass')


def step_statistics(batch, t, output, heads=None):
    """Return [B,fields] on device; no tensor-to-host reads or scalar syncs."""
    device = output[1].device
    at = lambda key: batch[key][:, t].to(device, non_blocking=True)
    sliced = lambda key, width: batch[key][:, t, :width].to(device, non_blocking=True)
    valid, decision, head = at('step_valid'), at('is_decision'), at('head')
    decision = decision & valid
    eligible = decision & (at('loss_weight') > 0) & (at('source_kind') != 2)
    cand, pair, scalar, value = (v.float() for v in output[1:])
    heads = (True,True,True,True) if heads is None else heads
    zero = torch.zeros_like(value)
    counts = at('diag_candidates')
    cmask = sliced('cand_mask',cand.shape[1]) & (torch.arange(cand.shape[1], device=device)[None] < counts[:, None])
    chosen = eligible & (head == 0) & (cmask.sum(-1) > 1)
    ce, correct, pass_target = zero, torch.zeros_like(valid), torch.zeros_like(valid)
    predicted_pass = torch.zeros_like(valid)
    if heads[0]:
        target = torch.where(chosen, at('single_idx'), 0).clamp(0, cand.shape[1] - 1)
        logits = cand.masked_fill(~cmask, -1e9)
        ce = F.cross_entropy(logits, target, reduction='none')
        predicted = logits.argmax(-1)
        correct = predicted == target
        pass_options = batch['cand_feat'][:, t, :cand.shape[1], 0].to(device, non_blocking=True) > .5
        pass_target = pass_options.gather(1, target[:, None])[:, 0]
        predicted_pass = (pass_options & cmask).gather(1, predicted[:, None])[:, 0]
    passed, active = chosen & pass_target, chosen & ~pass_target
    stats = dict(frames=valid, decisions=decision,
        zero_weight_decisions=decision & (at('loss_weight') <= 0),
        forced_source_decisions=decision & (at('source_kind') == 2),
        single_choices=chosen, single_options=cmask.sum(-1) * chosen, single_nll_sum=ce * chosen,
        single_correct=correct & chosen, pass_targets=passed, pass_correct=correct & passed,
        pass_nll_sum=ce * passed, active_targets=active, active_correct=correct & active, active_nll_sum=ce * active,
        pass_target_predicted_pass=passed & predicted_pass,
        pass_target_predicted_active=passed & ~predicted_pass,
        active_target_predicted_pass=active & predicted_pass,
        active_target_predicted_active=active & ~predicted_pass,
        pass_target_predicted_other_pass=passed & predicted_pass & ~correct)
    for kind, prefix, logits in ((1, 'cand', cand), (2, 'pair', pair)):
        label = 'subset' if kind == 1 else 'pair'
        if not heads[kind]:
            stats.update({label + suffix: zero for suffix in ('_choices','_free_entries','_bce_sum',
                          '_decision_bce_sum','_exact_free_set')})
            continue
        raw_count = at('diag_candidates' if kind == 1 else 'diag_pairs')
        width = logits.shape[1]
        mask = sliced(prefix + '_mask',width) & (torch.arange(width, device=device)[None] < raw_count[:, None])
        free = (sliced(prefix + '_free_mask',width) > 0) & mask
        truth = sliced(prefix + '_target_mask',width)
        number = free.sum(-1)
        scored = eligible & (head == kind) & (number > 0)
        safe_truth = torch.where(scored[:,None] & free, truth, torch.zeros_like(truth))
        losses = F.binary_cross_entropy_with_logits(logits.masked_fill(~mask, 0.), safe_truth, reduction='none')
        bce = (losses * free).sum(-1)
        exact = (((logits > 0) == (truth > .5)) | ~free).all(-1)
        stats.update({label + '_choices': scored, label + '_free_entries': number * scored,
            label + '_bce_sum': bce * scored,
            label + '_decision_bce_sum': bce / number.clamp_min(1) * scored,
            label + '_exact_free_set': exact & scored})
    scored = eligible & (head == 3)
    error = torch.where(scored, scalar - at('scalar_target'), torch.zeros_like(scalar))
    stats.update(scalar_decisions=scored, scalar_absolute_error_sum=error.abs() * scored,
        scalar_squared_error_sum=error.square() * scored,
        critic_squared_error_sum=(value - at('outcome')).square() * valid,
        critic_prediction_sum=value * valid, outcome_sum=at('outcome') * valid)
    return torch.stack([stats[key].double() for key in FIELDS], -1)


def _diagnostics(kind, **payload):
    return json.loads(_native(['diagnostic-metrics'],json.dumps(dict(kind=kind,fields=FIELDS,**payload),allow_nan=False).encode()))


def summarize(values):
    return _diagnostics('summary',values=np.asarray(values).tolist())


class Collector:
    def __init__(self, provenance):
        self.provenance = provenance
        self.total = np.zeros(len(FIELDS), np.float64)
        self.groups = {kind:{} for kind in ('head','pending_kind','format','game','head_pending_format')}
        self.games = {}
        self._steps = []

    def start_batch(self, host=None):
        self._heads = None
        if host is not None:
            eligible = host['step_valid'] & host['is_decision'] & (host['loss_weight'] > 0) & (host['source_kind'] != 2)
            self._heads = [tuple(bool((eligible & (host['head'] == kind))[:,t].any()) for kind in range(4))
                           for t in range(eligible.shape[1])]
        if self._steps:
            raise ValueError('Unfinished diagnostic batch')

    def step(self, batch, t, output):
        self._steps.append(step_statistics(batch, t, output, None if self._heads is None else self._heads[t]))

    def finish_batch(self, host):
        # One device-to-host transfer per complete unroll batch, never per frame.
        stats = torch.stack(self._steps, 1).detach().cpu().numpy()
        self._steps.clear()
        self.add_cpu(host, stats)

    def _state(self):
        return dict(total=self.total.tolist(),groups={kind:{
            json.dumps(key,separators=(',',':')) if isinstance(key,tuple) else key:value.tolist()
            for key,value in group.items()} for kind,group in self.groups.items()},
            games={key:{**value,'seats':sorted(value['seats'])} for key,value in self.games.items()})

    def add_cpu(self, host, stats):
        arrays={key:(host[key].numpy() if isinstance(host[key],torch.Tensor) else np.asarray(host[key])).reshape(-1).tolist()
            for key in ('diag_game','diag_seat','diag_pending','head','step_valid')}
        result=_diagnostics('add',state=self._state(),arrays=arrays,
            stats=np.asarray(stats).reshape(-1,len(FIELDS)).tolist(),
            provenance=dict(namespace=self.provenance.namespace_id,formats=self.provenance.formats,sources=self.provenance.sources))
        self.total=np.asarray(result['total'],dtype=np.float64)
        self.groups={kind:{tuple(json.loads(key)) if kind=='head_pending_format' else key:np.asarray(value,dtype=np.float64)
            for key,value in group.items()} for kind,group in result['groups'].items()}
        self.games={key:{**value,'seats':set(value['seats'])} for key,value in result['games'].items()}

    def report(self):
        if self._steps:
            raise ValueError('Cannot report a partial unroll batch')
        result=_diagnostics('report',state=self._state())
        result['definitions']=dict(pass_confusion='Masked argmax candidate feature0 >0.5 means predicted pass; exact pass_correct remains distinct from another pass option. Confusion describes teacher imitation, not whether passing is strategically wrong.',
                weighting='Unweighted labeled choices; positive loss_weight is an inclusion gate only; no outcome weights.',
                genuine_choice='Valid decision, positive weight, source != Forced, >1 categorical option or >=1 free binary entry.',
                threshold='logit >0 on free entries only; ignores forced entries. NOT a decoded action, legality score or engine-replay result.',
                game_cluster='All seats of one game share a conversion namespace/game_id cluster; additive sums support game bootstrap.',
                empty_denominator='null metrics; never invented zero accuracy or zero loss.')
        return result


def json_game_key(namespace, game):
    return f'{namespace}:{game}'
