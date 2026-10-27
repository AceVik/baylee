"""Explicit fixed-budget child schedules; every first source pass stays complete."""
CYCLE_EPOCH_STRIDE = 1_000_003


def cycle_source(child, half, epoch, budget):
    """Append complete chronological passes, then an explicit final repeat prefix.

    Segment indices and their original trajectory-start flags are unchanged.
    Each new pass therefore starts every lane at a genuine trajectory boundary.
    Per-pass lane tail padding is retained; cycling is not a label balancer.
    """
    result=[];passes=[];cycle=0
    while len(result)<budget:
        child.epoch=epoch+cycle*CYCLE_EPOCH_STRIDE
        rows=child._build_batches(advance_epoch=False)
        if not rows:raise ValueError('BC cycling source has no batches')
        if cycle==0 and budget<len(rows):
            raise ValueError('BC epoch batches cannot truncate a source first complete pass')
        take=min(len(rows),budget-len(result))
        selected=[row+[-1]*(half-len(row)) for row in rows[:take]]
        result.extend(selected)
        passes.append(dict(cycle=cycle,sampler_epoch=child.epoch,total_batches=len(rows),
                           consumed_batches=take,complete=take==len(rows),
                           padding_lane_batches=sum(i<0 for row in selected for i in row)))
        cycle+=1
    return result,dict(first_pass_batches=passes[0]['total_batches'],
        complete_passes=sum(p['complete'] for p in passes),repeated_passes_started=len(passes)-1,
        partial_repeat_batches=passes[-1]['consumed_batches'] if not passes[-1]['complete'] else 0,
        padding_lane_batches=sum(p['padding_lane_batches'] for p in passes),passes=passes)
