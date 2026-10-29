"""Batch independent readouts after a strictly chronological memory pass.

Each readout receives only the state produced by its own observation and past
observations. Attention never crosses the batch axis. No frame, candidate or
gradient is discarded. This execution path requires dropout=0 so regrouping
does not change which stochastic mask belongs to an observation.
"""
import torch
import model4 as M
from model4.candidate import score_candidate_hidden


def readout_groups(plans, chunk, token_limit=None):
    """Bound grouping by thinking depth and candidate/pair size, not game time."""
    groups = {}
    for t, plan in enumerate(plans):
        key = (plan.thinking, (plan.candidates - 1).bit_length(), (plan.pairs - 1).bit_length())
        pending = groups.setdefault(key, [])
        pending.append(t)
        capacity = chunk
        if token_limit is not None:
            # Upper bound for the concatenated candidate/pair scorer inputs.
            capacity = min(chunk, max(1, token_limit // (4 * 2**key[1] + 3 * 2**key[2])))
        if len(pending) == capacity:
            yield tuple(pending)
            pending.clear()
    for pending in groups.values():
        if pending:
            yield tuple(pending)


def _score_cand_slice(net, c_feat, c_dsl, c_rules, c_eidx, c_sidx, e_ref_c, s_ref_c, g_ref_c):
    d = net.cfg.d_model
    c_emb = net.cand_proj(M.symlog(c_feat)) + net.dsl_proj(M.symlog(c_dsl)) + net.rule_features(c_rules)
    ent_idx_clamped = c_eidx.clamp(min=0)
    has_ent = c_eidx >= 0
    c_ent = torch.gather(e_ref_c, 1, ent_idx_clamped.unsqueeze(-1).expand(-1, -1, d)) * has_ent.unsqueeze(-1).to(e_ref_c.dtype)
    seat_idx_clamped = c_sidx.clamp(0, M.MAX_SEATS - 1)
    has_seat = (c_sidx >= 0) & (c_sidx < M.MAX_SEATS)
    c_seat = torch.gather(s_ref_c, 1, seat_idx_clamped.unsqueeze(-1).expand(-1, -1, d)) * has_seat.unsqueeze(-1).to(s_ref_c.dtype)
    g_exp = g_ref_c.unsqueeze(1).expand_as(c_emb)
    cand_in = torch.cat([c_emb, c_ent, c_seat, g_exp], dim=-1)
    return net.cand_scorer[2](net.cand_scorer[1](net.cand_scorer[0](cand_in)))


def _score_cand_rows(net, c_feat, c_dsl, c_rules, c_eidx, c_sidx, c_msk, e_ref_c, s_ref_c, g_ref_c, use_ckpt=False):
    c_len = c_feat.shape[1]
    if c_len > 512:
        slices = []
        for s in range(0, c_len, 512):
            args = (
                c_feat[:, s : s + 512],
                c_dsl[:, s : s + 512],
                c_rules[:, s : s + 512],
                c_eidx[:, s : s + 512],
                c_sidx[:, s : s + 512],
                e_ref_c,
                s_ref_c,
                g_ref_c,
            )
            if use_ckpt:
                slices.append(torch.utils.checkpoint.checkpoint(lambda *a: _score_cand_slice(net, *a), *args, use_reentrant=False))
            else:
                slices.append(_score_cand_slice(net, *args))
        h_c = torch.cat(slices, dim=1)
    else:
        h_c = _score_cand_slice(net, c_feat, c_dsl, c_rules, c_eidx, c_sidx, e_ref_c, s_ref_c, g_ref_c)
    return score_candidate_hidden(net, h_c, c_msk)


def _cand_groups(plans, cand_times, chunk, token_limit=None):
    groups = {}
    for t in cand_times:
        key = (plans[t].candidates - 1).bit_length()
        pending = groups.setdefault(key, [])
        pending.append(t)
        capacity = chunk
        if token_limit is not None:
            capacity = min(chunk, max(1, token_limit // (4 * 2**key)))
        if len(pending) == capacity:
            yield tuple(pending)
            pending.clear()
    for pending in groups.values():
        if pending:
            yield tuple(pending)


def deferred_readouts(net, batch, boards, hidden, plans, chunk, device, bf16=False):
    """Return per-frame model outputs and the exact final persistent memory.

    The first pass retains the full recurrent autograd chain. The second pass
    decouples latent thinking steps, value readouts, and active policy heads
    across the unroll so each stage runs in dense device batches with zero
    host-device synchronization. Keeping autocast scoped to each memory update
    preserves its original precision.
    """
    if boards is None or plans is None:
        raise ValueError("Deferred readouts require pre-encoded boards and CPU execution plans")
    if chunk < 1 or net.cfg.dropout != 0:
        raise ValueError("Deferred readouts require positive chunk size and dropout=0")
    b, unroll = batch["step_valid"].shape
    d = net.cfg.d_model
    valid_all = batch["step_valid"].to(device, non_blocking=True)
    decision_all = batch["is_decision"].to(device, non_blocking=True).long()
    memories, thinking = [], []
    for t, plan in enumerate(plans):
        k = decision_all[:, t] * plan.thinking
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            observed = net.observe_memory(boards[t][0], hidden, k)
        memories.append(observed)
        thinking.append(k)
        hidden = torch.where(valid_all[:, t, None, None], observed.float(), hidden)

    with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
        board_slots = torch.stack([boards[t][0][:, :M.RECURRENT_SLOTS] for t in range(unroll)], dim=0)
        mem_stack = torch.stack(memories, dim=0)
        z_all = mem_stack + board_slots

        from .thinking import dense_thinking, compact_thinking
        z_all = (compact_thinking(net,z_all,boards,plans) if getattr(net,"compact_thinking_rows",False)
                 else dense_thinking(net,z_all,boards,thinking,plans,b,device))

        z_out_all = net.h_norm(z_all)
        g_ref_all = z_out_all[:, :, 0]
        s_ref_all = z_out_all[:, :, 1:M.RECURRENT_SLOTS]

        seat_feat_all = batch["seat_feat"].to(device, non_blocking=True).transpose(0, 1)
        seat0_mask = torch.arange(M.MAX_SEATS, device=device).view(1, 1, M.MAX_SEATS) == 0
        s_present_v = seat_feat_all[:, :, :, M.SEAT_FEAT_PRESENT] > 0.5
        s_lost_v = seat_feat_all[:, :, :, M.SEAT_FEAT_LOST] > 0.5
        s_active = s_present_v & (~s_lost_v)
        any_active = s_active.to(seat_feat_all.dtype).sum(dim=-1, keepdim=True) > 0.5
        s_active = s_active | ((~any_active) & seat0_mask)
        s_allied = (
            (seat_feat_all[:, :, :, M.SEAT_FEAT_SELF] > 0.5)
            | (seat_feat_all[:, :, :, M.SEAT_FEAT_TEAMMATE] > 0.5)
            | seat0_mask
        )
        sv_in = torch.cat([s_ref_all, g_ref_all.unsqueeze(2).expand(-1, -1, M.MAX_SEATS, d)], dim=-1)
        seat_win_logits = net.value_readout(sv_in).squeeze(-1)
        seat_win_logits = seat_win_logits.masked_fill(~s_active, -1e9)
        seat_probs = torch.nn.functional.softmax(seat_win_logits, dim=-1)
        value_all = (seat_probs * s_allied.to(seat_probs.dtype)).sum(dim=-1).clamp(0.0, 1.0)

        prune = getattr(net, "prune_unused_policy_heads", False)
        cand_mask_all = batch["cand_mask"].to(device, non_blocking=True)
        pair_mask_all = batch["pair_mask"].to(device, non_blocking=True)

        cand_outs = [None] * unroll
        pair_outs = [None] * unroll
        scal_outs = [torch.zeros(b, device=device, dtype=g_ref_all.dtype) for _ in range(unroll)]

        scal_times = [t for t, p in enumerate(plans) if (not prune) or p.heads[3]]
        if scal_times:
            g_scal = torch.cat([g_ref_all[t] for t in scal_times], dim=0)
            s_pred = torch.sigmoid(net.scalar_head(g_scal)).squeeze(-1)
            for i, t in enumerate(scal_times):
                scal_outs[t] = s_pred[i * b : (i + 1) * b]

        pair_times = [t for t, p in enumerate(plans) if (not prune) or p.heads[2]]
        if pair_times:
            p_max = max(plans[t].pairs for t in pair_times)
            p_idx = torch.cat([batch["pair_index"][:, t, :p_max].to(device, non_blocking=True) for t in pair_times], dim=0)
            p_msk = torch.cat([pair_mask_all[:, t, :p_max] for t in pair_times], dim=0)
            e_ref_p = torch.cat([boards[t][0][:, M.RECURRENT_SLOTS:] for t in pair_times], dim=0)
            g_ref_p = torch.cat([g_ref_all[t] for t in pair_times], dim=0)
            blk_idx = p_idx[:, :, 0].clamp(min=0)
            atk_idx = p_idx[:, :, 1].clamp(min=0)
            blk_tok = net.pair_blk_proj(torch.gather(e_ref_p, 1, blk_idx.unsqueeze(-1).expand(-1, -1, d)))
            atk_tok = net.pair_atk_proj(torch.gather(e_ref_p, 1, atk_idx.unsqueeze(-1).expand(-1, -1, d)))
            g_pair = g_ref_p.unsqueeze(1).expand_as(blk_tok)
            pair_in = torch.cat([blk_tok, atk_tok, blk_tok * atk_tok + g_pair], dim=-1)
            p_logits = net.pair_scorer(pair_in).squeeze(-1).masked_fill(~p_msk, -1e9)
            for i, t in enumerate(pair_times):
                pair_outs[t] = p_logits[i * b : (i + 1) * b, :plans[t].pairs]
        for t in range(unroll):
            if pair_outs[t] is None:
                p_len = plans[t].pairs
                pair_outs[t] = torch.zeros((b, p_len), device=device, dtype=boards[t][0].dtype).masked_fill(~pair_mask_all[:, t, :p_len], -1e9)

        cand_times = [t for t, p in enumerate(plans) if (not prune) or p.heads[0] or p.heads[1]]
        if cand_times:
            token_limit = 128 * 1024**2 // (b * d * 4)
            use_ckpt = bool(getattr(net.cfg, "activation_checkpointing", False) and net.training and torch.is_grad_enabled())
            for times in _cand_groups(plans, cand_times, max(chunk, unroll), token_limit):
                c_max = max(plans[t].candidates for t in times)
                c_feat = torch.cat([batch["cand_feat"][:, t, :c_max].to(device, non_blocking=True) for t in times], dim=0)
                c_dsl = torch.cat([batch["cand_dsl"][:, t, :c_max].to(device, non_blocking=True) for t in times], dim=0)
                c_rules = torch.cat([batch["cand_rules"][:, t, :c_max].to(device, non_blocking=True) for t in times], dim=0)
                c_eidx = torch.cat([batch["cand_entity_idx"][:, t, :c_max].to(device, non_blocking=True) for t in times], dim=0)
                c_sidx = torch.cat([batch["cand_seat_idx"][:, t, :c_max].to(device, non_blocking=True) for t in times], dim=0)
                c_msk = torch.cat([cand_mask_all[:, t, :c_max] for t in times], dim=0)
                e_ref_c = torch.cat([boards[t][0][:, M.RECURRENT_SLOTS:] for t in times], dim=0)
                s_ref_c = torch.cat([s_ref_all[t] for t in times], dim=0)
                g_ref_c = torch.cat([g_ref_all[t] for t in times], dim=0)

                args = (c_feat, c_dsl, c_rules, c_eidx, c_sidx, c_msk, e_ref_c, s_ref_c, g_ref_c)
                if use_ckpt and c_max > 64:
                    c_logits = torch.utils.checkpoint.checkpoint(
                        lambda *a: _score_cand_rows(net, *a, use_ckpt=True), *args, use_reentrant=False
                    )
                else:
                    c_logits = _score_cand_rows(net, *args, use_ckpt=False)
                for i, t in enumerate(times):
                    cand_outs[t] = c_logits[i * b : (i + 1) * b, :plans[t].candidates]
        for t in range(unroll):
            if cand_outs[t] is None:
                c_len = plans[t].candidates
                cand_outs[t] = torch.zeros((b, c_len), device=device, dtype=boards[t][0].dtype).masked_fill(~cand_mask_all[:, t, :c_len], -1e9)

    outputs = [
        (memories[t], cand_outs[t], pair_outs[t], scal_outs[t], value_all[t])
        for t in range(unroll)
    ]
    return outputs, hidden
