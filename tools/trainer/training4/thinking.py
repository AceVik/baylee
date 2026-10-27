"""Decision scratchpad execution; persistent observation updates stay separate."""
import torch


def dense_thinking(net, z_all, boards, thinking, plans, b, device):
    active_times = [t for t, p in enumerate(plans) if p.thinking > 0]
    if active_times:
        max_k = min(max(plans[t].thinking for t in active_times), net.cfg.max_thinking_steps)
        if max_k > 0:
            act_boards = torch.cat([boards[t][0] for t in active_times], dim=0)
            act_masks = torch.cat([boards[t][1] for t in active_times], dim=0)
            if act_masks.storage_offset():
                act_masks = act_masks.clone()
            act_k = torch.cat([thinking[t] for t in active_times], dim=0)
            act_z = torch.cat([z_all[t] for t in active_times], dim=0)
            proj_kv = (net.thinking_block.prepare_board(act_boards)
                       if getattr(net, "cache_thinking_kv", False) and max_k > 1 else None)
            for k in range(max_k):
                sub_idx = [i for i, t in enumerate(active_times) if plans[t].thinking > k]
                if len(sub_idx) == len(active_times):
                    step_t = torch.full((act_z.shape[0],), k, dtype=torch.long, device=device)
                    z_next = net.thinking_block(act_z, act_boards, act_masks, step_t, proj_kv)
                    active_k = (act_k > k).view(-1, 1, 1).to(act_z.dtype)
                    act_z = act_z + active_k * (z_next - act_z)
                else:
                    row_slices = [slice(i * b, (i + 1) * b) for i in sub_idx]
                    sub_z = torch.cat([act_z[s] for s in row_slices], dim=0)
                    sub_b = torch.cat([act_boards[s] for s in row_slices], dim=0)
                    sub_m = torch.cat([act_masks[s] for s in row_slices], dim=0)
                    if sub_m.storage_offset():
                        sub_m = sub_m.clone()
                    sub_tk = torch.cat([act_k[s] for s in row_slices], dim=0)
                    sub_kv = (tuple(torch.cat([kv[s] for s in row_slices], dim=0) for kv in proj_kv)
                              if proj_kv is not None else None)
                    step_t = torch.full((sub_z.shape[0],), k, dtype=torch.long, device=device)
                    z_next = net.thinking_block(sub_z, sub_b, sub_m, step_t, sub_kv)
                    active_k = (sub_tk > k).view(-1, 1, 1).to(sub_z.dtype)
                    sub_z_out = sub_z + active_k * (z_next - sub_z)
                    new_chunks = list(act_z.split(b, dim=0))
                    for j, i in enumerate(sub_idx):
                        new_chunks[i] = sub_z_out[j * b : (j + 1) * b]
                    act_z = torch.cat(new_chunks, dim=0)
            z_list = list(z_all.unbind(0))
            for i, t in enumerate(active_times):
                z_list[t] = act_z[i * b : (i + 1) * b]
            z_all = torch.stack(z_list, dim=0)

    return z_all


def compact_thinking(net, z_all, boards, plans):
    """Think only CPU-planned valid decision lanes; observation H is untouched."""
    if net.cfg.dropout != 0:
        raise ValueError('Compact thinking requires dropout=0')
    if any(plan.decision_rows is None for plan in plans):
        raise ValueError('Compact thinking requires CPU decision-row plans')
    b=z_all.shape[1];device=z_all.device
    if any(tuple(p.decision_rows)!=tuple(sorted(set(p.decision_rows))) or
           any(type(row) is not int or not 0<=row<b for row in p.decision_rows) for p in plans):
        raise ValueError('Invalid CPU decision-row plan')
    active=[(t,tuple(plan.decision_rows)) for t,plan in enumerate(plans)
            if plan.thinking>0 and plan.decision_rows]
    if not active:return z_all
    flat_rows=[t*b+lane for t,rows in active for lane in rows]
    depths=[min(plans[t].thinking,net.cfg.max_thinking_steps) for t,rows in active for _ in rows]
    indices=torch.tensor(flat_rows,dtype=torch.long,device=device)
    selected_z=z_all.flatten(0,1).index_select(0,indices)
    lane_indices=[torch.tensor(rows,dtype=torch.long,device=device) for _,rows in active]
    selected_board=torch.cat([boards[t][0].index_select(0,rows) for (t,_),rows in zip(active,lane_indices)],0)
    selected_mask=torch.cat([boards[t][1].index_select(0,rows) for (t,_),rows in zip(active,lane_indices)],0)
    max_k=max(depths)
    kv=(net.thinking_block.prepare_board(selected_board)
        if getattr(net,'cache_thinking_kv',False) and max_k>1 else None)
    for k in range(max_k):
        rows=[i for i,depth in enumerate(depths) if depth>k]
        gather=torch.tensor(rows,dtype=torch.long,device=device)
        z=selected_z.index_select(0,gather)
        board=selected_board.index_select(0,gather)
        mask=selected_mask.index_select(0,gather)
        projections=tuple(p.index_select(0,gather) for p in kv) if kv is not None else None
        steps=torch.full((len(rows),),k,dtype=torch.long,device=device)
        updated=net.thinking_block(z,board,mask,steps,projections)
        # Retain dense-path arithmetic, including its floating-point rounding.
        selected_z=selected_z.index_copy(0,gather,z+(updated-z))
    return z_all.flatten(0,1).index_copy(0,indices,selected_z).reshape_as(z_all)
