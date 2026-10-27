"""Additive unweighted diagnostics, independent of every training loss version."""
import numpy as np
import torch
import torch.nn.functional as F

FIELDS = ('frames', 'decisions', 'zero_weight_decisions', 'forced_source_decisions',
          'single_choices', 'single_options', 'single_nll_sum', 'single_correct',
          'pass_targets', 'pass_correct', 'pass_nll_sum', 'active_targets', 'active_correct', 'active_nll_sum',
          'subset_choices', 'subset_free_entries', 'subset_bce_sum', 'subset_decision_bce_sum', 'subset_exact_free_set',
          'pair_choices', 'pair_free_entries', 'pair_bce_sum', 'pair_decision_bce_sum', 'pair_exact_free_set',
          'scalar_decisions', 'scalar_absolute_error_sum', 'scalar_squared_error_sum',
          'critic_squared_error_sum', 'critic_prediction_sum', 'outcome_sum')


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
    if heads[0]:
        target = torch.where(chosen, at('single_idx'), 0).clamp(0, cand.shape[1] - 1)
        logits = cand.masked_fill(~cmask, -1e9)
        ce = F.cross_entropy(logits, target, reduction='none')
        correct = logits.argmax(-1) == target
        pass_target = batch['cand_feat'][:, t, :cand.shape[1], 0].to(device, non_blocking=True).gather(1, target[:, None])[:, 0] > .5
    passed, active = chosen & pass_target, chosen & ~pass_target
    stats = dict(frames=valid, decisions=decision,
        zero_weight_decisions=decision & (at('loss_weight') <= 0),
        forced_source_decisions=decision & (at('source_kind') == 2),
        single_choices=chosen, single_options=cmask.sum(-1) * chosen, single_nll_sum=ce * chosen,
        single_correct=correct & chosen, pass_targets=passed, pass_correct=correct & passed,
        pass_nll_sum=ce * passed, active_targets=active, active_correct=correct & active, active_nll_sum=ce * active)
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


def summarize(values):
    sums = dict(zip(FIELDS, map(float, values)))
    def ratio(num, den):
        return sums[num] / sums[den] if sums[den] else None
    metrics = dict(single_nll=ratio('single_nll_sum','single_choices'),
        single_accuracy=ratio('single_correct','single_choices'),
        pass_target_accuracy=ratio('pass_correct','pass_targets'), active_target_accuracy=ratio('active_correct','active_targets'),
        pass_target_nll=ratio('pass_nll_sum','pass_targets'), active_target_nll=ratio('active_nll_sum','active_targets'),
        scalar_mae=ratio('scalar_absolute_error_sum','scalar_decisions'),
        scalar_mse=ratio('scalar_squared_error_sum','scalar_decisions'),
        critic_mse=ratio('critic_squared_error_sum','frames'))
    for head in ('subset','pair'):
        metrics[head + '_free_entry_bce'] = ratio(head + '_bce_sum',head + '_free_entries')
        metrics[head + '_mean_decision_bce'] = ratio(head + '_decision_bce_sum',head + '_choices')
        metrics[head + '_threshold_exact_free_set_accuracy'] = ratio(head + '_exact_free_set',head + '_choices')
    return dict(sums=sums, metrics=metrics)


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

    def add_cpu(self, host, stats):
        arrays = {key:host[key].numpy() if isinstance(host[key], torch.Tensor) else host[key]
                  for key in ('diag_game','diag_seat','diag_pending','head','step_valid')}
        valid = arrays['step_valid']
        rows = stats[valid]
        if not np.isfinite(rows).all():
            raise ValueError('Nonfinite diagnostic statistics')
        if not len(rows):
            return
        self.total += rows.sum(0, dtype=np.float64)
        games, seats = arrays['diag_game'][valid], arrays['diag_seat'][valid]
        heads, pending = arrays['head'][valid], arrays['diag_pending'][valid]
        def add(dimension, key, selected):
            target = self.groups[dimension].setdefault(key,np.zeros(len(FIELDS),np.float64))
            target += rows[selected].sum(0, dtype=np.float64)
        game_formats = {}
        for game in np.unique(games):
            game = int(game)
            key = json_game_key(self.provenance.namespace_id, game)
            name = self.provenance.formats.get(game, 'unknown')
            game_formats[game] = name
            metadata = self.games.setdefault(key,dict(namespace=self.provenance.namespace_id,
                game_id=game,format=name,seats=set(),source=self.provenance.sources.get(game)))
            if metadata['format'] != name:
                raise ValueError('Game has conflicting format identities')
            selected = games == game
            metadata['seats'].update(map(int,np.unique(seats[selected])))
            add('game',key,selected)
        names = sorted(set(game_formats.values()))
        name_ids = {name:index for index,name in enumerate(names)}
        formats = np.array([name_ids[game_formats[int(game)]] for game in games])
        for head in np.unique(heads):
            add('head',str(int(head)),heads == head)
        for kind in np.unique(pending):
            add('pending_kind',str(int(kind)),pending == kind)
        for code,name in enumerate(names):
            add('format',name,formats == code)
        joint = np.stack((heads,pending,formats),-1)
        unique, inverse = np.unique(joint,axis=0,return_inverse=True)
        for index,(head,kind,code) in enumerate(unique):
            add('head_pending_format',(str(int(head)),str(int(kind)),names[code]),inverse == index)

    def report(self):
        if self._steps:
            raise ValueError('Cannot report a partial unroll batch')
        return dict(global_metrics=summarize(self.total),
            by_head={key:summarize(value) for key,value in sorted(self.groups['head'].items())},
            by_pending_kind={key:summarize(value) for key,value in sorted(self.groups['pending_kind'].items())},
            by_format={key:summarize(value) for key,value in sorted(self.groups['format'].items())},
            by_head_pending_format=[dict(head=int(key[0]),pending_kind=int(key[1]),format=key[2],**summarize(value))
                                   for key,value in sorted(self.groups['head_pending_format'].items())],
            per_game=[{**self.games[key], 'seats':sorted(self.games[key]['seats']), **summarize(value)}
                      for key,value in sorted(self.groups['game'].items())],
            definitions=dict(weighting='Unweighted labeled choices; positive loss_weight is an inclusion gate only; no outcome weights.',
                genuine_choice='Valid decision, positive weight, source != Forced, >1 categorical option or >=1 free binary entry.',
                threshold='logit >0 on free entries only; ignores forced entries. NOT a decoded action, legality score or engine-replay result.',
                game_cluster='All seats of one game share a conversion namespace/game_id cluster; additive sums support game bootstrap.',
                empty_denominator='null metrics; never invented zero accuracy or zero loss.'))


def json_game_key(namespace, game):
    return f'{namespace}:{game}'
