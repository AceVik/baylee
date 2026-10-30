"""Exact host-owned execution hints for recorded recurrent policy frames."""

import torch


def recorded_step_hints(batch):
    """Keep the recorded per-lane clock; do not replace it with a K curriculum.

    Model candidate/pair extents use every lane's mask, including padding, just
    like its default GPU branch. Head selection and all model tensors stay intact.
    """
    names = ('is_decision', 'thinking_steps', 'cand_mask', 'pair_mask')
    if any(batch[name].device.type != 'cpu' for name in names):
        raise ValueError('Recorded execution hints require original CPU metadata')
    k = torch.where(batch['is_decision'], batch['thinking_steps'].clamp_min(1), 0)
    if bool((k > 8).any()):
        raise ValueError('Recorded thinking depth exceeds model contract')
    candidates = batch['cand_mask'].any(dim=0)
    pairs = batch['pair_mask'].any(dim=0)
    result = []
    for t in range(k.shape[1]):
        c = torch.nonzero(candidates[t], as_tuple=False)
        p = torch.nonzero(pairs[t], as_tuple=False)
        hint = (int(c[-1, 0])+1 if c.numel() else 1,
                int(p[-1, 0])+1 if p.numel() else 1, int(k[:, t].max()))
        result.append((k[:, t], hint))
    return result
