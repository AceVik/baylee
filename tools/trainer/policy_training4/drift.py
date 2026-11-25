"""How far the net's decisions outside the action-probability contract have moved.

`injective_categorical_menu_v1` certifies single categorical menus. Attackers and blockers
(subset and pair heads), numbers (scalar head), payments and aliased menus carry no behavior
probability: PPO's ratio, its regulariser and its behavior gate never read them. They share
the trunk, the memory and the candidate scorer with the certified rows and move with every
update. `SourceReadouts` keeps the frozen source's own readouts of those rows, recorded once
before any update; `Drift` compares the current policy's with them. No loss reads either and
no gradient passes through them.

`single`, `subset` and `pair` (heads 0 to 2) have a KL(source || current), the one
`policy_math4.action_logprob_and_kl` defines at each row's recorded temperature: over the
menu's entries (for aliased entries an upper bound on that of the actions), or over the
recorded draw's free entries (of the draw as recorded, not of every answer the sampler's
constraints could make of it). A number is a point mass without a KL: its drift is the
change of the normalised prediction, reported and never a reason to refuse. Only decisions
with a choice count: a menu of two or more entries, a free entry.
"""
import math

import torch

from policy_math4 import action_logprob_and_kl

KINDS = ('single', 'subset', 'pair', 'scalar')   # by policy head
GATED = KINDS[:3]
COLUMN = dict(single=1, subset=1, pair=2, scalar=3)   # of the model's outputs
CONTRACT = 'source_kl_per_head_v1'


def outside(batch, t):
    """The net's own decisions with a choice at frame t that the contract does not certify:
    {kind: [B] mask} of the kinds that have any."""
    at = lambda name: batch[name][:, t].cpu()
    own = (at('step_valid').bool() & at('is_decision').bool() & (at('source_kind') == 0)
           & ~at('actor_mask').bool())
    if not bool(own.any()):
        return {}
    head = at('head')
    choice = (at('cand_mask').sum(-1) > 1,
              ((at('cand_free_mask') > 0) & at('cand_mask').bool()).any(-1),
              ((at('pair_free_mask') > 0) & at('pair_mask').bool()).any(-1),
              torch.ones_like(own))
    found = {kind: own & (head == index) & choice[index] for index, kind in enumerate(KINDS)}
    return {kind: rows for kind, rows in found.items() if bool(rows.any())}


class SourceReadouts:
    """The frozen source's readouts of the uncertified decisions, by (segment, frame)."""

    def __init__(self):
        self.rows = {}

    def observe(self, indices, batch, t, outputs):
        for kind, rows in outside(batch, t).items():
            values = outputs[COLUMN[kind]].detach().float().cpu()
            if kind == 'scalar':
                widths = torch.ones(len(rows), dtype=torch.long)
                values = values[:, None]
            else:
                mask = batch[('pair' if kind == 'pair' else 'cand') + '_mask'][:, t].cpu().bool()
                widths = (mask * torch.arange(1, mask.shape[1] + 1)).amax(-1)
            for lane in rows.nonzero().flatten().tolist():
                key = int(indices[lane]), t
                if key in self.rows:
                    raise ValueError('An uncertified decision was recorded twice')
                self.rows[key] = values[lane, :int(widths[lane])].clone()


class Drift:
    """The current policy against `source` over the decisions observed."""

    def __init__(self, source):
        self.source = source
        self.sums = {kind: 0. for kind in KINDS}
        self.largest = {kind: 0. for kind in KINDS}
        self.counts = {kind: 0 for kind in KINDS}

    def observe(self, indices, batch, t, outputs):
        found = outside(batch, t)
        if not found:
            return
        current = [value.detach().float().cpu() for value in outputs[1:4]]
        anchor = [value.clone() for value in current]
        for kind, rows in found.items():
            for lane in rows.nonzero().flatten().tolist():
                stored = self.source.rows.get((int(indices[lane]), t))
                if stored is None:
                    raise ValueError('An uncertified decision without the source readout of it')
                target = anchor[COLUMN[kind] - 1]
                if kind == 'scalar':
                    target[lane] = stored[0]
                elif len(stored) > target.shape[1]:
                    raise ValueError('An uncertified decision has another menu than the source read')
                else:
                    target[lane, :len(stored)] = stored
        _, kl = action_logprob_and_kl(*current, *anchor, batch, t)
        for kind, rows in found.items():
            values = ((current[2] - anchor[2]).abs() if kind == 'scalar' else kl)[rows].double()
            if not bool(torch.isfinite(values).all()):
                raise FloatingPointError('Nonfinite drift of uncertified decisions')
            self.sums[kind] += float(values.sum())
            self.largest[kind] = max(self.largest[kind], float(values.max()))
            self.counts[kind] += int(rows.sum())

    def mean(self, kind):
        return self.sums[kind] / self.counts[kind] if self.counts[kind] else None

    def report(self):
        out = {kind: dict(decisions=self.counts[kind], mean_kl=self.mean(kind),
                          max_kl=self.largest[kind] if self.counts[kind] else None) for kind in GATED}
        out['scalar'] = dict(decisions=self.counts['scalar'], mean_absolute=self.mean('scalar'),
                             max_absolute=self.largest['scalar'] if self.counts['scalar'] else None)
        return out

    def refused(self, limit):
        """The heads whose mean KL exceeds `limit`, as the behavior gate refuses certified rows."""
        return [kind for kind in GATED if self.counts[kind]
                and not (math.isfinite(self.mean(kind)) and self.mean(kind) <= limit)]
