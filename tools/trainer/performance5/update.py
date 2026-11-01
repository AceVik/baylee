"""The production deferred supervised update, without checkpoint/validation I/O."""
import torch

from training4.deferred import deferred_readouts
from training4.device_loss import compute_unroll_device_loss
from training4.execution import encode_boards, plan_batch, transfer_batch
from training4.optimizer_groups import apply_relational_warmup


def update(net, optimizer, host, staged, hidden, saved, config, step, board_chunk, before_clip=None):
    device = hidden.device
    plans = plan_batch(host, step, net.cfg.default_thinking_steps, curriculum=False)
    batch = transfer_batch(host, device) if staged is None else staged
    hidden = torch.where(batch['is_new_traj'].to(device)[:, None, None],
                         torch.zeros_like(hidden), hidden.detach())
    optimizer.zero_grad(set_to_none=True)
    with torch.autocast(device.type, dtype=torch.bfloat16, enabled=device.type == 'cuda'):
        net.clear_structure()
        net.prepare_structure((host['entity_rules'],host['cand_rules']))
        boards = encode_boards(net, batch, board_chunk, device)
    prepared, final_hidden = deferred_readouts(
        net, batch, boards, hidden, plans, net.decision_chunk, device, device.type == 'cuda')
    with torch.autocast(device.type, dtype=torch.bfloat16, enabled=device.type == 'cuda'):
        memories = torch.stack([out[0][:, 0] for out in prepared], dim=1)
        auxiliary = net.aux_head(memories.detach() if config.detach_aux_input else memories)
    loss, _ = compute_unroll_device_loss(prepared, auxiliary.float(), batch, plans,
                                        loss_config=config)
    if not torch.isfinite(loss):
        raise FloatingPointError('Nonfinite probe loss')
    loss.backward()
    if before_clip is not None:
        before_clip(net,final_hidden,loss.detach())
    torch.nn.utils.clip_grad_norm_(net.parameters(), 1., error_if_nonfinite=True)
    apply_relational_warmup(optimizer, saved.get('optimizer_schedule'), step + 1)
    optimizer.step()
    net.clear_structure()
    return final_hidden.detach(), loss.detach()


def capture(net, hidden, loss):
    """CPU copies only in the separate correctness pass, never timed updates."""
    return dict(loss=loss.cpu().clone(), hidden=hidden.detach().cpu().clone(),
                gradients={n: p.grad.detach().cpu().clone() for n, p in net.named_parameters()
                           if p.grad is not None})


def difference(reference, candidate):
    a, b = reference['gradients'], candidate['gradients']
    if a.keys() != b.keys():
        raise AssertionError('Gradient parameter membership changed')
    delta = sum(float((a[n].double() - b[n].double()).square().sum()) for n in a)
    magnitude = sum(float(v.double().square().sum()) for v in a.values())
    return dict(loss_abs=abs(float(reference['loss']) - float(candidate['loss'])),
                hidden_max_abs=float((reference['hidden'] - candidate['hidden']).abs().max()),
                gradient_relative_l2=(delta / max(magnitude, 1e-30)) ** .5)
