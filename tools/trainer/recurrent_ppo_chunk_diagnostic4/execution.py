"""Reference-H, independent-H and contiguous-board comparisons in one history pass."""

import torch

import dataset4 as D
import model4 as M
from policy_math4 import action_logprob_and_kl
from recurrent_ppo4.forward import step_inputs
from training4.execution import encode_boards
from .comparison import first_output_violation, output_error, tensor_error, update_maxima


def board_layout(board):
    return {name: dict(shape=list(value.shape), stride=list(value.stride()),
                       dtype=str(value.dtype), contiguous=bool(value.is_contiguous()))
            for name, value in zip(('tokens', 'mask'), board)}


@torch.no_grad()
def diagnose(net, dataset, prefixes, identities, device):
    """Visit valid native frames in production sampler order, stopping on violation.

    Chunk encoding computes one U64 batch ahead. Visited counts refer only to
    valid frames actually compared, not prefetched boards or padded lanes.
    Local/contiguous paths always receive the reference's incoming H; only the
    free-running path carries its own H. No post-failure history is evaluated.
    """
    net.eval()
    reference_h = current_h = previous = None
    result = dict(visited_frames=0, visited_choices=0, visited_histories=[],
                  completed_histories=[], first_failure=None, full_scan=False,
                  board_mask_mismatches=0, board={}, shared_reference_h={},
                  free_running_h={}, contiguous_shared_reference_h={})
    seen_frames, histories, completed = set(), set(), set()
    history_key = {}
    last_frame = {}
    for history in dataset.trajectories:
        keys = [identities[index, t] for index in history for t in range(len(dataset.segments[index]))]
        key = keys[0][:2]
        for index in history:
            history_key[index] = key
        last_frame[key] = keys[-1]
    try:
        for indices in D.StatefulBatchSampler4(dataset, 4, shuffle=False):
            if reference_h is None:
                reference_h = torch.zeros(len(indices), M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
                current_h = reference_h.clone()
                previous = [None]*len(indices)
            reset = []
            for lane, index in enumerate(indices):
                prefix = () if index < 0 else prefixes[index]
                if prefix and previous[lane] != prefix[-1]:
                    raise ValueError('Diagnostic lane is not a contiguous admitted history')
                reset.append(not prefix)
                previous[lane] = index if index >= 0 else None
            reset = torch.tensor(reset, device=device)[:, None, None]
            reference_h = torch.where(reset, torch.zeros_like(reference_h), reference_h)
            current_h = torch.where(reset, torch.zeros_like(current_h), current_h)
            batch = D.collate_trajectories([dataset[index] for index in indices])
            net.clear_structure()
            net.prepare_structure((batch['entity_rules'], batch['cand_rules']))
            chunked = encode_boards(net, batch, 4, device)
            for t in range(batch['step_valid'].shape[1]):
                inputs, k = step_inputs(batch, t, device)
                reference_board = net.encode_board(*inputs[:8], inputs[M.INPUT_NAMES.index('entity_rules')])
                current_board = chunked[t]
                contiguous_board = tuple(value.contiguous() for value in current_board)
                reference = net(*inputs, reference_h, k, encoded_board=reference_board)
                shared = net(*inputs, reference_h, k, encoded_board=current_board)
                contiguous = net(*inputs, reference_h, k, encoded_board=contiguous_board)
                independent = net(*inputs, current_h, k, encoded_board=current_board)
                outputs = (reference, shared, contiguous, independent)
                lp = [action_logprob_and_kl(*output[1:4], *output[1:4], batch, t)[0]
                      for output in outputs]
                # Copy diagnostic observations once per timestep. Comparisons on
                # CPU avoid one CUDA scalar synchronization per lane/coordinate;
                # the four model executions and carried H remain on the device.
                compared_outputs = [(output[0].float().cpu(), None, None, None, output[4].float().cpu())
                                    for output in outputs]
                compared_lp = [value.float().cpu() for value in lp]
                compared_boards = [tuple(value.cpu() for value in board)
                                   for board in (reference_board, current_board)]
                for lane, index in enumerate(indices):
                    if not bool(batch['step_valid'][lane, t]):
                        continue
                    key = identities[index, t]
                    if key in seen_frames:
                        raise ValueError('Diagnostic repeated a native frame')
                    seen_frames.add(key)
                    histories.add(history_key[index])
                    if key == last_frame[history_key[index]]:
                        completed.add(history_key[index])
                    result['visited_frames'] += 1
                    actor = bool(batch['actor_mask'][lane, t])
                    result['visited_choices'] += int(actor)
                    local = output_error(compared_outputs[0], compared_outputs[1], compared_lp[0], compared_lp[1], lane, actor)
                    layout = output_error(compared_outputs[0], compared_outputs[2], compared_lp[0], compared_lp[2], lane, actor)
                    free = output_error(compared_outputs[0], compared_outputs[3], compared_lp[0], compared_lp[3], lane, actor)
                    board = tensor_error(compared_boards[0][0][lane], compared_boards[1][0][lane])
                    masks_match = torch.equal(compared_boards[0][1][lane], compared_boards[1][1][lane])
                    update_maxima(result['board'], dict(encoded_board=board))
                    update_maxima(result['shared_reference_h'], local)
                    update_maxima(result['contiguous_shared_reference_h'], layout)
                    update_maxima(result['free_running_h'], free)
                    violation = first_output_violation(free)
                    if not masks_match:
                        mismatch = torch.nonzero(compared_boards[0][1][lane] != compared_boards[1][1][lane], as_tuple=False)[0]
                        token = int(mismatch[0])
                        result['board_mask_mismatches'] += 1
                        violation = dict(kind='board_mask', coordinate=[token],
                            reference=bool(compared_boards[0][1][lane, token]),
                            current=bool(compared_boards[1][1][lane, token]), allowed=0., normalized_error=None)
                    if violation is not None:
                        result['first_failure'] = dict(game_id=key[0], seat=key[1], cursor=key[2],
                            segment_index=index, timestep=t, lane=lane, **violation,
                            board=board, board_masks_equal=masks_match,
                            shared_reference_h=local, contiguous_shared_reference_h=layout,
                            free_running_h=free,
                            board_layout=dict(reference=board_layout(reference_board),
                                              chunked=board_layout(current_board),
                                              contiguous=board_layout(contiguous_board)),
                            chunk_board_contiguous=bool(current_board[0].is_contiguous()),
                            reference_board_contiguous=bool(reference_board[0].is_contiguous()))
                        return result
                valid = batch['step_valid'][:, t].to(device)[:, None, None]
                reference_h = torch.where(valid, reference[0].float(), reference_h)
                current_h = torch.where(valid, independent[0].float(), current_h)
        if seen_frames != set(identities.values()):
            raise ValueError('Diagnostic omitted admitted native frames')
        result['full_scan'] = True
        return result
    finally:
        result['visited_histories'] = [dict(game_id=game, seat=seat) for game, seat in sorted(histories)]
        result['completed_histories'] = [dict(game_id=game, seat=seat) for game, seat in sorted(completed)]
        net.clear_structure()
