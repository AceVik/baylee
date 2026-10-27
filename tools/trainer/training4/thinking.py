"""Decision scratchpad execution; persistent observation updates stay separate."""
import torch


def dense_thinking(net, z_all, boards, thinking, plans, b):
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
