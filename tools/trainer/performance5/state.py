"""Restore the declared architecture and exact optimizer parameter grouping."""
import copy

import torch

from checkpoint4 import move_optimizer
from export_onnx4 import load_export_checkpoint
from training4.loss_config import SupervisedLossConfig
from training4.objective import check_resume_objective
from training4.optimizer_groups import build_optimizer_groups, check_resume_schedule


def optimizer_groups(net, saved):
    contract = saved.get('optimizer_schedule')
    if contract is not None and (contract.get('version') != 1 or contract.get('schedule') != 'linear_global_optimizer_update_1_based'):
        raise ValueError('Unsupported optimizer schedule contract')
    if contract is None:
        groups, _ = build_optimizer_groups(net, saved['optimizer']['param_groups'][0]['lr'])
    else:
        parameters = dict(net.named_parameters())
        names = [name for group in contract['groups'] for name in group['parameter_names']]
        if len(names) != len(set(names)) or set(names) != set(parameters):
            raise ValueError('Optimizer parameter contract is incomplete or duplicated')
        groups = [dict(params=[parameters[name] for name in group['parameter_names']],
                       name=group['name']) for group in contract['groups']]
        check_resume_schedule(saved, contract)
    if [len(g['params']) for g in groups] != [len(g['params']) for g in saved['optimizer']['param_groups']]:
        raise ValueError('Optimizer grouping differs; frozen-trunk probes are unsupported')
    identifiers=[i for g in saved['optimizer']['param_groups'] for i in g['params']]
    if identifiers!=list(range(len(identifiers))) or not set(saved['optimizer']['state'])<=set(identifiers):
        raise ValueError('Invalid optimizer state parameter identity')
    for parameter,identifier in zip([p for g in groups for p in g['params']],identifiers,strict=True):
        for name,tensor in saved['optimizer']['state'].get(identifier,{}).items():
            if torch.is_tensor(tensor):
                if not torch.isfinite(tensor).all():raise ValueError('Nonfinite optimizer state')
                if name != 'step' and tensor.shape != parameter.shape:
                    raise ValueError('Optimizer tensor shape differs from parameter')
                if name == 'step' and (tensor.numel()!=1 or tensor.item()<0):
                    raise ValueError('Invalid optimizer update count')
    return groups


def restore(path, device):
    saved, net, _ = load_export_checkpoint(path)
    if saved['config']['dropout'] != 0:
        raise ValueError('Matched regrouped execution requires dropout=0')
    config = SupervisedLossConfig.from_dict(saved.get('supervised_loss_config', {}))
    check_resume_objective(saved,saved.get('checkpoint_selection','combined'),config)
    net = net.to(device).train()
    net.detach_value_input = config.detach_value_input
    opt = torch.optim.AdamW(optimizer_groups(net, saved), fused=device.type == 'cuda')
    opt.load_state_dict(copy.deepcopy(saved['optimizer']))
    for group in opt.param_groups:
        group['fused'] = device.type == 'cuda'
    move_optimizer(opt, device)
    return saved, net, opt, config
