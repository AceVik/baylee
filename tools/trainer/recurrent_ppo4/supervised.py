"""Optional labeled BC replay with current-policy prefix reconstruction."""
import torch
import dataset4 as D
from training4.loss import compute_step_loss
from .forward import reconstruct_hidden, advance


def supervised_loss(net, dataset, indices, prefixes, device):
    h = reconstruct_hidden(net,dataset,indices,prefixes,device)
    host = D.collate_trajectories([dataset[index] for index in indices])
    net.clear_structure()
    net.prepare_structure((host["entity_rules"],host["cand_rules"]))
    outputs,_ = advance(net,host,h,device)
    loss = torch.zeros((),device=device)
    for t,out in enumerate(outputs):
        auxiliary = net.aux_head(out[0][:,0])
        contribution,_ = compute_step_loss(*out[1:4],out[4],auxiliary,host,t)
        loss = loss + contribution / len(outputs)
    return loss
