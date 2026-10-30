"""Opt-in relational learning rates; archived/default AdamW groups stay unchanged."""
import math


def build_optimizer_groups(net, main_lr, *, architecture='v4', freeze_trunk=False,
                           relational_lr=0., output_lr=0., warmup_steps=0):
    if (any(not math.isfinite(x) or x < 0 for x in (relational_lr, output_lr))
            or warmup_steps < 0):
        raise ValueError('Relational learning rates/warmup must be finite and nonnegative')
    enabled = relational_lr > 0
    if not enabled and (output_lr > 0 or warmup_steps):
        raise ValueError('Relational output rate/warmup requires --relational-lr')
    if enabled and (architecture != 'v5' or freeze_trunk):
        raise ValueError('Relational optimizer groups require v5 without --freeze-trunk')
    parameters = [(name, p) for name, p in net.named_parameters() if p.requires_grad]
    critic = [(name, p) for name, p in parameters if 'seat_value_head' in name]
    other = [(name, p) for name, p in parameters if 'seat_value_head' not in name]
    if not enabled:
        groups = [dict(params=[p for _, p in other], lr=main_lr, weight_decay=1e-2)]
        if critic:
            groups.append(dict(params=[p for _, p in critic], lr=main_lr * .2, weight_decay=5e-2))
        return groups, None
    # Modules added on top of a trained trunk (the relational block, the
    # candidate decoder) learn at the new-module rates; their zero-initialized
    # residual outputs carry no weight decay.
    def decoder_output(name):
        return name.startswith('cand_decoder.') and ('.attn.out_proj.' in name or '.ff.2.' in name)
    shared = [(n, p) for n, p in other if not n.startswith(('relational.', 'cand_decoder.'))]
    inner = [(n, p) for n, p in other if n.startswith('relational.') and n != 'relational.output.weight']
    output = [(n, p) for n, p in other if n == 'relational.output.weight']
    decoder_inner = [(n, p) for n, p in other if n.startswith('cand_decoder.') and not decoder_output(n)]
    decoder_outputs = [(n, p) for n, p in other if decoder_output(n)]
    if not inner or len(output) != 1:
        raise ValueError('Expected relational inner parameters and one output weight')
    definitions = [('shared', shared, main_lr, 1e-2), ('critic', critic, main_lr * .2, 5e-2),
                   ('relational_inner', inner, relational_lr, 1e-2),
                   ('relational_output', output, output_lr or relational_lr, 0.),
                   ('decoder_inner', decoder_inner, relational_lr, 1e-2),
                   ('decoder_output', decoder_outputs, output_lr or relational_lr, 0.)]
    groups, contract_groups = [], []
    for name, named, lr, decay in definitions:
        if not named:
            continue
        warming = name.startswith(('relational_', 'decoder_'))
        groups.append(dict(params=[p for _, p in named], name=name,
                           lr=0. if warming and warmup_steps else lr, weight_decay=decay))
        contract_groups.append(dict(name=name, parameter_names=[n for n, _ in named],
                                    target_lr=lr, weight_decay=decay, warmup=warming))
    return groups, dict(version=1, warmup_steps=warmup_steps,
                        schedule='linear_global_optimizer_update_1_based', groups=contract_groups)


def scheduled_lr(group, contract, completed_updates):
    warmup = contract['warmup_steps']
    factor = min(1., completed_updates / warmup) if group['warmup'] and warmup else 1.
    return group['target_lr'] * factor


def apply_relational_warmup(optimizer, contract, update_number):
    """Call before opt.step(); first optimizer update is numbered one."""
    if contract is None:
        return
    if update_number < 1:
        raise ValueError('Optimizer update number must be positive')
    for actual, expected in zip(optimizer.param_groups, contract['groups'], strict=True):
        if actual.get('name') != expected['name']:
            raise ValueError('Optimizer group order/name changed')
        if expected['warmup']:
            actual['lr'] = scheduled_lr(expected, contract, update_number)


def check_resume_schedule(saved, contract):
    """Reject changed contracts and corrupt/reordered restored optimizer groups."""
    if saved.get('optimizer_schedule') != contract:
        raise ValueError('Resume optimizer schedule differs; use --init for a new experiment')
    if contract is None:
        return
    actual_groups = saved['optimizer']['param_groups']
    if len(actual_groups) != len(contract['groups']):
        raise ValueError('Resume optimizer group count differs')
    for actual, expected in zip(actual_groups, contract['groups'], strict=True):
        if (actual.get('name') != expected['name']
                or actual['weight_decay'] != expected['weight_decay']
                or len(actual['params']) != len(expected['parameter_names'])
                or not math.isclose(actual['lr'], scheduled_lr(expected, contract, saved.get('optimizer_updates', saved['step_num'])),
                                    rel_tol=1e-12, abs_tol=0.)):
            raise ValueError('Resume optimizer group layout/rate differs')
