"""Opt-in weighted means by head over BxU; no per-frame host synchronization.

Categorical/scalar denominators count weighted decisions; set/pair denominators
count weighted legal free entries. Per-decision set diagnostics belong to the
common evaluator and are not another optimization coefficient. Explicit outcome
weighting applies to categorical/set/pair heads, as in v4, never scalar targets.
"""
import torch
import torch.nn.functional as F


def compute_loss_v5(prepared, auxiliary, batch, plans, config):
    device=auxiliary.device
    moved={}
    def get(key):
        if key not in moved:moved[key]=batch[key].to(device,non_blocking=True)
        return moved[key]
    if not any(p.valid for p in plans):
        return torch.zeros((),device=device),torch.zeros(5,device=device)
    valid=get('step_valid');decision=get('is_decision') & valid;heads=get('head')
    weight=torch.where(valid,get('loss_weight'),0.);outcome=torch.where(valid,get('outcome'),0.)
    factor=(.35+1.30*outcome) if config.outcome_weighting else torch.ones_like(outcome)
    zero=torch.zeros((),device=device);unroll=len(plans)
    numerators=[zero]*4;denominators=[zero]*4
    per_step=[];correct=zero;count=zero
    for t,plan in enumerate(plans):
        policy=zero
        for kind in range(4):
            if not plan.heads[kind]:continue
            selected=decision[:,t] & (heads[:,t]==kind)
            weights=weight[:,t] * selected
            if kind==0:
                logits=prepared[t][1].float();mask=get('cand_mask')[:,t,:logits.shape[1]]
                genuine=selected & (mask.sum(-1)>1) & (weight[:,t]>0)
                target=torch.where(genuine,get('single_idx')[:,t],0)
                # Invalid/forced/inactive targets must not reach CE.
                losses=F.cross_entropy(torch.where(genuine[:,None],logits,0.),target,reduction='none')
                weights=weight[:,t]*factor[:,t]*genuine
                correct=correct+((logits.argmax(-1)==target)&genuine).sum().float();count=count+genuine.sum().float()
            elif kind in (1,2):
                prefix='cand' if kind==1 else 'pair';logits=prepared[t][kind].float()
                mask=get(prefix+'_mask')[:,t,:logits.shape[1]]
                free=get(prefix+'_free_mask')[:,t,:logits.shape[1]]*mask.float()
                eligible=selected[:,None] & mask & (free>0) & (weight[:,t,None]>0)
                target=torch.where(eligible,get(prefix+'_target_mask')[:,t,:logits.shape[1]],0.)
                safe_logits=torch.where(eligible,logits,0.)
                losses=F.binary_cross_entropy_with_logits(safe_logits,target,reduction='none')
                free=torch.where(eligible,free,0.)
                weights=free*(weight[:,t]*factor[:,t]*selected)[:,None]
            else:
                eligible=selected & (weight[:,t]>0)
                target=torch.where(eligible,get('scalar_target')[:,t],0.)
                safe_scalar=torch.where(eligible,prepared[t][3].float(),0.)
                losses=(safe_scalar-target).square()
                weights=weight[:,t]*eligible
            numerator=(losses*weights).sum();denominator=weights.sum()
            numerators[kind]=numerators[kind]+numerator;denominators[kind]=denominators[kind]+denominator
            minimum=1. if kind in (1,2) else 1e-6
            policy=policy+numerator/denominator.clamp(min=minimum)
        per_step.append(policy)
    predictions=torch.stack([p[4].float() for p in prepared],1)
    # Inactive values/targets cannot poison loss through NaN*0.
    value_error=(torch.where(valid,predictions,0.)-torch.where(valid,outcome,0.)).square()
    aux_error=(torch.where(valid[...,None],auxiliary.float(),0.)-
               torch.where(valid[...,None],get('aux_target'),0.)).square().mean(-1)
    if config.normalization=='unroll_per_head':
        policy=sum((n/d.clamp(min=1e-6) for n,d in zip(numerators,denominators)),zero)
        n=valid.sum().clamp(min=1)
        value_loss=value_error.sum()/n;aux_loss=aux_error.sum()/n
    else:
        policy=torch.stack(per_step).sum()/unroll
        n=valid.sum(0).clamp(min=1)
        value_loss=(value_error.sum(0)/n).sum()/unroll
        aux_loss=(aux_error.sum(0)/n).sum()/unroll
    total=policy+config.value_weight*value_loss+config.auxiliary_weight*aux_loss
    stats=torch.stack((policy.detach()*unroll,value_loss.detach()*unroll,aux_loss.detach()*unroll,correct,count))
    return total,stats
