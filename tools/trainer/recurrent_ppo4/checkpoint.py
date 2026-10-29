"""Strict full-policy loading and exact optimizer/RNG continuation."""
import torch
import model4 as M
from checkpoint4 import atomic_save, rng_state, restore_rng, move_optimizer
from encoder_contract import validate_encoder
from value_calibration4.model_identity import source_model, source_identity


def load_policy(saved, device):
    if saved.get('trajectory_version') != M.TRAJECTORY_VERSION or saved.get('sampling_version') != M.SAMPLING_VERSION:
        raise ValueError('Incompatible recurrent policy trajectory/sampler contract')
    net = source_model(saved)
    validate_encoder(saved.get('encoder_version'),model_version=saved.get('model_version'),
                     relational_revision=getattr(net.cfg,'relational_revision',None))
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
    import os
    import subprocess
    from pathlib import Path
    executable=os.environ.get('BAYLEE_TRAIN_OPS',str(Path(__file__).resolve().parents[3]/'target/release/train_ops'))
    result=subprocess.run([executable,'hash',str(path)],check=True,capture_output=True,text=True)
    fields=result.stdout.split()
    digest=fields[0] if fields else ""
    if len(digest)!=64 or any(character not in '0123456789abcdef' for character in digest):
        raise ValueError('Native checkpoint seal returned malformed SHA256')
    return digest


def native_verify_admission(data,source,admission):
    """Fail closed unless the native scanner verifies the actual sealed rows."""
    import os
    import subprocess
    from pathlib import Path
    executable=os.environ.get('BAYLEE_TRAIN_OPS',str(Path(__file__).resolve().parents[3]/'target/release/train_ops'))
    try:
        subprocess.run([executable,'verify-admission','--data',str(data),
                        '--source-checkpoint',str(source),'--admission',str(admission)],check=True,capture_output=True,text=True)
    except subprocess.CalledProcessError as error:
        raise ValueError('Native policy admission rejected: '+error.stderr.strip()) from error
