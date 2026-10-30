"""Apply Rust-certified learner histories to a PyTorch trajectory dataset."""
import json
from pathlib import Path


def select_learner_histories(dataset, receipt_path, source_hash):
    admission = json.loads(Path(receipt_path).expanduser().read_text())
    expected = dict(mode='raw', actor_role='learner', precision='float32', thinking_steps=2, temperature=.7)
    if (admission.get('version') != 1 or admission.get('admitted') is not True
            or admission.get('source_checkpoint_sha256') != source_hash
            or admission.get('dataset_fingerprint') != dataset.fingerprint
            or admission.get('collection_contract') != expected):
        raise ValueError('Missing or mismatched Rust-certified PPO data admission')
    admitted = admission.get('admitted_learner_histories')
    if not isinstance(admitted, list) or not admitted:
        raise ValueError('Admission lacks explicit learner history roles')
    allowed = set()
    for key in admitted:
        if (not isinstance(key, dict) or set(key) != {'game_id', 'seat'}
                or type(key['game_id']) is not int or key['game_id'] < 0
                or type(key['seat']) is not int or not 0 <= key['seat'] < 8):
            raise ValueError('Malformed admitted learner history key')
        allowed.add((key['game_id'], key['seat']))
    if len(allowed) != len(admitted) or admission.get('learner_histories') != len(allowed):
        raise ValueError('Admission learner history counts disagree')
    selected, found = [], set()
    for trajectory in dataset.trajectories:
        first = dataset.store.index[int(dataset.segments[trajectory[0]][0])]
        key = (int(first['game']), int(first['seat']))
        if key in allowed:
            selected.append(trajectory)
            found.add(key)
    if found != allowed or len(selected) != len(allowed):
        raise ValueError('Admitted learner history is absent or duplicated')
    if admission.get('completed_games') != len({game for game, seat in found}):
        raise ValueError('Admission completed-game count differs')
    dataset.trajectories = selected
    return admission


def check_audit(admission, audit):
    if (audit['decisions_checked'] != admission.get('decisions')
            or audit['learner_trajectories'] != admission.get('learner_histories')):
        raise ValueError('Audited learner decisions/histories disagree with Rust admission')
