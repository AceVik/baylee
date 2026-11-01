"""Research-specific objective and exact optimizer/RNG continuation."""
import torch

from checkpoint4 import move_optimizer, restore_rng, rng_state
from value_calibration4.model_identity import source_identity
from .targets import OBJECTIVE, TARGET_CONTRACT

MEMORY = 'current_policy_complete_episode_prefix_detached_at_unroll_boundary'
PROBABILITY = 'recorded_executed_action'
VERSION = 1


def validate_resume(saved):
    contract = saved.get('research_ppo_contract')
    if (not isinstance(contract, dict) or contract.get('version') != VERSION
            or contract.get('objective') != OBJECTIVE or contract.get('target_contract') != TARGET_CONTRACT
            or contract.get('behavior_probability_reference') != PROBABILITY
            or saved.get('memory_reconstruction') != MEMORY
            or 'full_ppo_contract' in saved):
        raise ValueError('Research PPO resume objective/probability contract differs')


def training_payload(net, optimizer, source, contract, progress):
    identity = source_identity(source)
    for key in ('encoder_warmstart', 'candidate_context_warmstart'):
        if key in source:
            identity[key] = source[key]
    return {**identity, 'structure': source['structure'], 'state_dict': net.state_dict(),
            'optimizer': optimizer.state_dict(), 'rng': rng_state(),
            'research_ppo_contract': contract, 'research_ppo_progress': progress,
            'memory_reconstruction': MEMORY}


def restore_training(saved, net, optimizer, contract, device):
    validate_resume(saved)
    if saved['research_ppo_contract'] != contract:
        raise ValueError('Research PPO resume experiment contract differs')
    # Reject identities before restoring any model, optimizer or RNG state.
    if saved.get('encoder_version') != contract['encoder_version']:
        raise ValueError('Research PPO resume encoder differs')
    if any(not bool(torch.isfinite(v).all()) for v in saved['state_dict'].values() if v.is_floating_point()):
        raise ValueError('Nonfinite research resume parameters')
    net.load_state_dict(saved['state_dict'], strict=True)
    optimizer.load_state_dict(saved['optimizer'])
    move_optimizer(optimizer, device)
    restore_rng(saved['rng'])
    return dict(saved['research_ppo_progress'])
