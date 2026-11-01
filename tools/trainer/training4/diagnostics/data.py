"""Opt-in diagnostic scalars and provenance; ordinary training datasets are unchanged."""
import json
import numpy as np
import orjson
from torch_io4 import file_digest as file_hash
from dataset4 import TrajectoryDataset4
from trajectory_store4 import BST4_HDR, BST4_MAGIC

PENDING_NAMES = ('Priority','ChooseManaAbility','ChooseDamageSource','ChooseDamageEffect',
    'AllocatePrevention','DeclareAttackers','DeclareBlockers','ChooseCards','ChooseTargets','Arrange',
    'LegendChoice','Mulligan','MulliganBottom','DiscardChoice','YesNo','ChooseCastMode','ChooseNumber',
    'ChooseColor','ChooseSubtype','ChooseCardName','ChoosePlayer','ChoosePile')
PENDING_IDS = {name:index for index,name in enumerate(PENDING_NAMES)} | {'GameOver':27}


class DiagnosticDataset(TrajectoryDataset4):
    def __init__(self, data_dir, split='val', **kwargs):
        self.diagnostic_split = split
        super().__init__(data_dir, split=split, **kwargs)

    def encode_single_step(self, step_or_row):
        raw = step_or_row if isinstance(step_or_row, dict) else self.store.read_raw(int(step_or_row))
        if isinstance(raw, dict) and '_bst4' in raw:
            raw = raw['_bst4']
        if isinstance(raw, bytes) and raw[:4] == BST4_MAGIC:
            header = BST4_HDR.unpack_from(raw)
            game, seat, cursor, pending = header[2], header[3], header[4], header[8]
            head, decision, nc, npairs = header[7], bool(header[6] & 1), header[28], header[31]
            nm, nf = header[32:34]
            # Strict diagnostics must not score loader-invented free masks.
            if decision and head in (1, 2):
                size = nc if head == 1 else npairs
                if nm != size or nf != size:
                    raise ValueError('Diagnostic subset/pair masks are incomplete')
                masks = raw[len(raw)-nm-nf:] if nm + nf else b''
                if any(value not in (0, 1) for value in masks):
                    raise ValueError('Diagnostic masks must be binary')
            encoded = super().encode_single_step({'_bst4': raw})
        else:
            step = orjson.loads(raw) if isinstance(raw, bytes) else raw
            game, seat, cursor = step['game_id'], step['seat'], step['step_cursor']
            pending = step.get('pending_kind', -1)
            if not isinstance(pending, int):
                pending = PENDING_IDS.get(pending, -1)  # Unknown stays explicit.
            head, decision = step['head'], step.get('is_decision', True)
            nc, npairs = len(step['cand_feat']), len(step['pairs'])
            if decision and head in (1, 2):
                size = nc if head == 1 else npairs
                for name in ('multi_mask', 'free_mask'):
                    mask = step['target'].get(name)
                    if (not isinstance(mask, list) or len(mask) != size
                            or any(value not in (0, 1) for value in mask)):
                        raise ValueError('Diagnostic subset/pair masks are incomplete or nonbinary')
            encoded = super().encode_single_step(step)
        if decision and head == 2 and npairs and int(encoded['pair_mask'].sum()) != npairs:
            raise ValueError('Diagnostic pair references would be discarded')
        encoded.update(diag_game=np.array(game, dtype=np.int64), diag_seat=np.array(seat, dtype=np.int64),
                       diag_cursor=np.array(cursor, dtype=np.int64), diag_pending=np.array(pending, dtype=np.int64),
                       diag_candidates=np.array(nc, dtype=np.int64), diag_pairs=np.array(npairs, dtype=np.int64))
        return encoded


class Provenance:
    """Thin tensor-diagnostics bridge to native immutable source admission."""
    def __init__(self, dataset):
        from trajectory_store4 import _native
        result=json.loads(_native(['diagnostic-provenance'],json.dumps(dict(
            root=str(dataset.data_dir),meta=dataset.meta,split=dataset.diagnostic_split,
            structure_digest=dataset.structure.digest),allow_nan=False).encode()))
        self.meta=dataset.meta
        self.inputs=result['inputs']
        self.namespace=result['namespace']
        self.namespace_id=result['namespace_id']
        self.formats={int(game):value for game,value in result['formats'].items()}
        self.sources={int(game):value for game,value in result['sources'].items()}
