"""Thin Torch sampler bridge to native source/table identity admission."""
import json
from trajectory_store4 import _native


def digest(value):
    return _native(['digest'], json.dumps(value, allow_nan=False).encode()).decode().strip()


def game_metadata(dataset, split):
    observed=[]
    for trajectory in dataset.trajectories:
        row=dataset.store.index[dataset.segments[trajectory[0]][0]]
        observed.append([int(row['game']),int(row['seat'])])
    value=json.loads(_native(['game-metadata','--data-dir',dataset.data_dir,'--split',split],
                            json.dumps(dict(meta=dataset.meta,observed=observed),allow_nan=False).encode()))
    games={int(game):{**row,'record':tuple(row['record'])} for game,row in value['games'].items()}
    return games,value['receipts']


def reject_overlap(left,right):
    _native(['reject-overlap'],json.dumps(dict(left=left,right=right),allow_nan=False).encode())
