"""Full-model FP32 recurrent PPO with exact current-policy prefix reconstruction.

Frozen behavior probabilities and complete terminal side-utility targets are
audited before updates. Every accepted update passes a whole-collection
behavior-divergence gate; rejected updates restore weights, optimizer and RNG.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

import torch

import dataset4 as D4
import model4 as M
from policy_math4 import action_logprob_and_kl
from checkpoint4 import atomic_save


def compute_reverse_gae(
    values: torch.Tensor,
    outcomes: torch.Tensor,
    step_valid: torch.Tensor,
    is_terminal: torch.Tensor,
    gamma: float,
    gae_lambda: float,
    bootstrap_value: torch.Tensor | None = None,
    bootstrap_advantage: torch.Tensor | None = None,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Computes reverse-time GAE(gamma, lambda) across `[B, U]` (`P0-5`)."""
    B, U = values.shape
    device = values.device
    gae = (torch.zeros(B, device=device, dtype=torch.float32) if bootstrap_advantage is None
           else bootstrap_advantage.detach().to(device).clone())
    adv_out = torch.zeros((B, U), device=device, dtype=torch.float32)
    ret_out = torch.zeros((B, U), device=device, dtype=torch.float32)

    for t in reversed(range(U)):
        term_t = is_terminal[:, t]
        valid_t = step_valid[:, t]
        v_t = values[:, t].detach()
        r_term = outcomes[:, t]
        at_boundary = torch.ones(B, dtype=torch.bool, device=device) if t == U - 1 else ~step_valid[:, t + 1]
        needs_bootstrap = valid_t & ~term_t & at_boundary
        if needs_bootstrap.any() and bootstrap_value is None:
            raise ValueError("Nonterminal sequence boundary requires the actual next-state value")
        boot = torch.zeros(B, device=device) if bootstrap_value is None else bootstrap_value.detach().to(device)
        v_next = boot if t == U - 1 else torch.where(at_boundary, boot, values[:, t + 1].detach())
        delta = torch.where(term_t, r_term - v_t, gamma * v_next - v_t)
        updated = delta + gamma * gae_lambda * (~term_t).float() * gae
        gae = torch.where(valid_t, updated, gae)
        adv_out[:, t] = torch.where(valid_t, gae, 0.0)
        ret_out[:, t] = torch.where(valid_t, v_t + gae, 0.0)

    return adv_out, ret_out


def main() -> None:
    import os
    from contextlib import ExitStack
    from .checkpoint import load_policy,training_payload,restore_training,native_source_digest,native_verify_admission
    from .checkpoint import validate_resume_probability,FULL_PPO_CONTRACT_VERSION,BEHAVIOR_PROBABILITY_REFERENCE
    from .forward import episode_prefixes
    from .optimization import transactional_update,collection_divergence
    from .supervised import supervised_loss
    from policy_training4.targets import prepare_policy_targets
    ap=argparse.ArgumentParser(description='FP32 full-model recurrent PPO with current-policy burn-in and collection-wide KL rollback')
    ap.add_argument('--init',required=True)
    ap.add_argument('--rollout-data','--rollout',dest='rollout_data',required=True)
    ap.add_argument('--admission',required=True,help='Rust-certified collection admission JSON bound to source and dataset')
    ap.add_argument('--out',required=True)
    ap.add_argument('--resume',default='')
    ap.add_argument('--audit-only',action='store_true')
    ap.add_argument('--anchor-data',default='')
    ap.add_argument('--anchor-mix-weight',type=float,default=.25)
    ap.add_argument('--batch-size',type=int,default=4)
    ap.add_argument('--unroll','--unroll-len',dest='unroll',type=int,default=32)
    ap.add_argument('--thinking-steps',type=int,default=2)
    ap.add_argument('--epochs',type=int,default=1)
    ap.add_argument('--max-steps',type=int,default=32)
    ap.add_argument('--max-shards',type=int,default=0)
    ap.add_argument('--lr',type=float,default=2e-5)
    ap.add_argument('--target-kl',type=float,default=.02)
    ap.add_argument('--clip-eps',type=float,default=.2)
    ap.add_argument('--beta-kl',type=float,default=.02,help='Sampled behavior-divergence regularizer')
    ap.add_argument('--gamma',type=float,default=1.)
    ap.add_argument('--gae-lambda',type=float,default=1.)
    ap.add_argument('--seed',type=int,default=31431)
    ap.add_argument('--device',choices=('cpu','cuda'),default='cpu')
    args=ap.parse_args()
    if (min(args.batch_size,args.unroll,args.epochs,args.max_steps)<1 or args.lr<=0
            or not 0<args.clip_eps<1 or args.target_kl<=0 or args.beta_kl<0 or args.anchor_mix_weight<0):
        ap.error('Invalid PPO settings')
    if args.thinking_steps!=2 or args.max_shards or args.gamma!=1 or args.gae_lambda!=1:
        ap.error('Full-model pilot requires fixed K2, all shards and complete terminal-return targets gamma=lambda=1')
    resume_saved = None
    if args.resume:
        resume_saved = torch.load(Path(args.resume).expanduser(),map_location='cpu',weights_only=False)
        validate_resume_probability(resume_saved)
    os.environ.setdefault('CUBLAS_WORKSPACE_CONFIG',':4096:8')
    torch.use_deterministic_algorithms(True)
    torch.set_num_threads(4)
    torch.manual_seed(args.seed)
    import random
    import numpy as np
    random.seed(args.seed);np.random.seed(args.seed)
    device=torch.device(args.device)
    source_path=Path(args.init).expanduser()
    if source_path.is_dir():source_path=source_path/'net.pt'
    source_hash=native_source_digest(source_path)
    source=torch.load(source_path,map_location='cpu',weights_only=False)
    net=load_policy(source,device)
    native_verify_admission(Path(args.rollout_data).expanduser(),source_path,Path(args.admission).expanduser())
    with ExitStack() as resources:
        dataset=D4.TrajectoryDataset4(Path(args.rollout_data).expanduser(),split='train',
                                     encoder_version=source['encoder_version'],unroll_len=args.unroll,max_shards=0)
        resources.callback(dataset.store.close)
        if dataset.structure.digest!=net.rules.digest:
            raise ValueError('Rollout and policy rule catalogs differ')
        from policy_training4.admission import select_learner_histories, check_audit
        admission=select_learner_histories(dataset,args.admission,source_hash)
        audit=prepare_policy_targets(net,dataset,device,batch_size=args.batch_size,board_chunk=0)
        resources.callback(dataset._policy_targets_directory.cleanup)
        check_audit(admission,audit)
        for trajectory in dataset.trajectories:
            for index in trajectory:
                row=dataset[index]
                mask=torch.from_numpy(row['actor_mask']).bool()
                if bool((torch.from_numpy(row['thinking_steps'])[mask]!=2).any()):
                    raise ValueError('Full PPO actor thinking depth differs from fixed K2')
        # Every policy likelihood is replayed before the first optimizer update.
        if args.audit_only:
            print(json.dumps(dict(admission=admission,probability_replay=audit),sort_keys=True),flush=True)
            return
        prefixes=episode_prefixes(dataset)
        segments=[idx for tr in dataset.trajectories for idx in tr]
        anchor_dataset=anchor_prefixes=None
        if args.anchor_data and args.anchor_mix_weight:
            anchor_dataset=D4.TrajectoryDataset4(Path(args.anchor_data).expanduser(),split='train',
                                                encoder_version=source['encoder_version'],unroll_len=args.unroll,max_shards=0)
            resources.callback(anchor_dataset.store.close)
            if anchor_dataset.structure.digest!=net.rules.digest:
                raise ValueError('Supervised anchor rule catalog differs')
            anchor_prefixes=episode_prefixes(anchor_dataset)
        contract=dict(version=FULL_PPO_CONTRACT_VERSION,behavior_probability_reference=BEHAVIOR_PROBABILITY_REFERENCE,
                      collection_audit='current_policy_streaming_v1',
                      source_checkpoint_sha256=source_hash,dataset_fingerprint=dataset.fingerprint,
                      admission=admission,encoder_version=source['encoder_version'],
                      target='complete terminal side outcome; gamma=lambda=1',precision='float32',
                      memory='current_policy_complete_episode_prefix_detached_at_unroll_boundary',
                      deterministic_algorithms=True,activation_checkpointing=True,torch_version=str(torch.__version__),
                      device=str(device),batch_size=args.batch_size,unroll=args.unroll,seed=args.seed,
                      lr=args.lr,target_kl=args.target_kl,clip_eps=args.clip_eps,beta_kl=args.beta_kl,
                      anchor_dataset_fingerprint=None if anchor_dataset is None else anchor_dataset.fingerprint,
                      anchor_mix_weight=args.anchor_mix_weight,epochs=args.epochs)
        optimizer=torch.optim.AdamW(net.parameters(),lr=args.lr,weight_decay=0)
        source_optimizer=copy.deepcopy(optimizer.state_dict())
        progress=dict(epoch=0,next_batch=0,attempts=0,accepted=0,updates=[],terminal=False)
        out=Path(args.out).expanduser()
        if args.resume:
            progress=restore_training(resume_saved,net,optimizer,contract,device)
        elif out.exists():
            raise FileExistsError('Preserving existing full-PPO output; use --resume or a new directory')
        out.mkdir(parents=True,exist_ok=True)
        def save():
            atomic_save(training_payload(net,optimizer,source,contract,progress),out/'last.pt')
        stopped=bool(progress['terminal'])
        for epoch in range(progress['epoch'],args.epochs):
            if stopped:break
            generator=torch.Generator().manual_seed(args.seed+epoch)
            order=torch.randperm(len(segments),generator=generator).tolist()
            groups=[tuple(segments[i] for i in order[start:start+args.batch_size])
                    for start in range(0,len(order),args.batch_size)]
            start_batch=progress['next_batch'] if epoch==progress['epoch'] else 0
            for batch_index in range(start_batch,len(groups)):
                if progress['attempts']>=args.max_steps:
                    stopped=True;break
                extra=None
                if anchor_dataset is not None:
                    anchor_ids=list(anchor_prefixes)
                    start=(progress['attempts']*args.batch_size)%len(anchor_ids)
                    indices=tuple(anchor_ids[start:start+args.batch_size])
                    extra=lambda:args.anchor_mix_weight*supervised_loss(net,anchor_dataset,indices,anchor_prefixes,device)
                record=transactional_update(net,optimizer,dataset,groups[batch_index],prefixes,device,
                                            args.target_kl,args.clip_eps,args.batch_size,args.beta_kl,extra)
                progress['attempts']+=1
                progress['accepted']+=int(record['accepted'])
                progress['epoch']=epoch;progress['next_batch']=batch_index+1
                progress['updates'].append(dict(epoch=epoch,batch=batch_index,**record))
                print('[full-ppo-step] '+json.dumps(progress['updates'][-1],sort_keys=True),flush=True)
                if not record['accepted']:
                    progress['terminal']=True;stopped=True
                save()
                if stopped:break
            if stopped:break
            progress['epoch']=epoch+1;progress['next_batch']=0
            save()
        try:
            final_kl=collection_divergence(net,dataset,prefixes,device,args.batch_size)
        except FloatingPointError:
            final_kl=float("inf")
        if not np.isfinite(final_kl) or final_kl>args.target_kl:
            net.load_state_dict(source['state_dict'],strict=True)
            optimizer.load_state_dict(source_optimizer)
            progress.update(accepted=0,terminal=True,final_gate='rejected_published_unchanged_source')
        else:
            progress['final_gate']='accepted'
        progress['final_collection_behavior_kl']=final_kl if np.isfinite(final_kl) else None
        native_verify_admission(Path(args.rollout_data).expanduser(),source_path,Path(args.admission).expanduser())
        save()
        atomic_save(training_payload(net,optimizer,source,contract,progress),out/'net.pt')
        print(json.dumps(dict(output=str(out),audit=audit,progress=progress),sort_keys=True),flush=True)


if __name__=='__main__':
    main()
