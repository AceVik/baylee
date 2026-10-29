"""Strict full-policy loading and exact optimizer/RNG continuation."""
import torch
import model4 as M
from checkpoint4 import atomic_save, rng_state, restore_rng, move_optimizer
from encoder_contract import validate_encoder
from value_calibration4.model_identity import source_model, source_identity

FULL_PPO_CONTRACT_VERSION = 2
BEHAVIOR_PROBABILITY_REFERENCE = 'recorded_executed_action'


def validate_resume_probability(saved):
    """Reject ambiguous legacy optimizer continuations before any restoration."""
    contract = saved.get('full_ppo_contract')
    if (not isinstance(contract, dict)
            or contract.get('version') != FULL_PPO_CONTRACT_VERSION
            or contract.get('behavior_probability_reference') != BEHAVIOR_PROBABILITY_REFERENCE):
        raise ValueError('Full PPO resume probability reference differs; use --init for weights-only initialization')


def load_policy(saved, device):
    if saved.get('trajectory_version') != M.TRAJECTORY_VERSION or saved.get('sampling_version') != M.SAMPLING_VERSION:
        raise ValueError('Incompatible recurrent policy trajectory/sampler contract')
    net = source_model(saved)
    validate_encoder(saved.get('encoder_version'),model_version=saved.get('model_version'),
                     relational_revision=getattr(net.cfg,'relational_revision',None),
                     candidate_context_revision=getattr(net.cfg,'candidate_context_revision',1),
                     global_feat_width=net.cfg.global_feat_width)
    net.attach_structure(saved['structure'])
    if saved.get('structure_sha256',net.rules.digest) != net.rules.digest:
        raise ValueError('Source rule catalog digest differs')
    net.load_state_dict(saved['state_dict'],strict=True)
    if not all(bool(torch.isfinite(p).all()) for p in net.parameters()):
        raise ValueError('Nonfinite source policy parameters')
    net.cfg.activation_checkpointing=True
    net.checkpoint_during_eval=True
    return net.to(device).eval().requires_grad_(True)


def training_payload(net, optimizer, source, contract, progress):
    identity=source_identity(source)
    for key in ('encoder_warmstart','candidate_context_warmstart'):
        if key in source:identity[key]=source[key]
    return {**identity, 'structure': source['structure'],
            'state_dict': net.state_dict(), 'optimizer': optimizer.state_dict(),
            'rng': rng_state(), 'full_ppo_contract': contract,
            'full_ppo_progress': progress,
            'memory_reconstruction': 'current_policy_complete_episode_prefix_detached_at_unroll_boundary'}


def restore_training(saved, net, optimizer, contract, device):
    # Check the entire sealed experiment before touching parameters or optimizer.
    validate_resume_probability(saved)
    if saved.get('encoder_version') == 9 or net.cfg.global_feat_width == 82:
        values = saved.get('config', {})
        validate_encoder(saved.get('encoder_version'),model_version=saved.get('model_version'),
                         relational_revision=values.get('relational_revision',1),
                         candidate_context_revision=values.get('candidate_context_revision',1),
                         global_feat_width=values.get('global_feat_width',80))
        if saved.get('encoder_version') != 9 or net.cfg.global_feat_width != 82:
            raise ValueError('Encoder9 resume input contract differs')
    if saved.get('full_ppo_contract') != contract:
        raise ValueError('Full PPO resume experiment contract differs')
    if saved.get('memory_reconstruction') != 'current_policy_complete_episode_prefix_detached_at_unroll_boundary':
        raise ValueError('Full PPO resume memory reconstruction differs')
    net.load_state_dict(saved['state_dict'],strict=True)
    optimizer.load_state_dict(saved['optimizer'])
    move_optimizer(optimizer,device)
    restore_rng(saved['rng'])
    return dict(saved['full_ppo_progress'])


def native_source_digest(path):
    """Thin PyTorch checkpoint-I/O adapter; hashing itself belongs to Rust."""
    from torch_io4 import file_digest
    return file_digest(path)


def native_verify_admission(data,source,admission):
    """Fail closed unless the native scanner verifies the actual sealed rows."""
    from torch_io4 import native
    native(['verify-admission','--data',data,'--source-checkpoint',source,'--admission',admission])
