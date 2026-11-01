"""Thin complete-pass callback adapter between Torch sampler and native planner."""
import json
from trajectory_store4 import _native

CYCLE_EPOCH_STRIDE = 1_000_003


def _plan(**value):
    return json.loads(_native(['bc-cycle'],json.dumps(value,allow_nan=False).encode()))


def cycle_source(child, half, epoch, budget):
    plan=_plan(half=half,epoch=epoch,budget=budget)
    while not plan['done']:
        child.epoch=plan['next_epoch']
        rows=child._build_batches(advance_epoch=False)
        plan=_plan(state=plan['state'],rows=rows)
    return plan['state']['result'],plan['receipt']
