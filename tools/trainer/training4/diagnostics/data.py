"""Opt-in diagnostic scalars and provenance; ordinary training datasets are unchanged."""
import hashlib
import json
from pathlib import Path
import numpy as np
import orjson
from dataset4 import TrajectoryDataset4
from trajectory_store4 import BST4_HDR, BST4_MAGIC

PENDING_NAMES = ('Priority','ChooseManaAbility','ChooseDamageSource','ChooseDamageEffect',
    'AllocatePrevention','DeclareAttackers','DeclareBlockers','ChooseCards','ChooseTargets','Arrange',
    'LegendChoice','Mulligan','MulliganBottom','DiscardChoice','YesNo','ChooseCastMode','ChooseNumber',
    'ChooseColor','ChooseSubtype','ChooseCardName','ChoosePlayer','ChoosePile')
PENDING_IDS = {name:index for index,name in enumerate(PENDING_NAMES)} | {'GameOver':27}


def file_hash(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


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
    """Formats come from explicit metadata/verified source-job options, never directory names."""
    def __init__(self, dataset):
        self.inputs = {}
        self.meta = dataset.meta
        root = dataset.data_dir.resolve()
        namespace = self.meta.get('source_namespace')
        if namespace is None:
            declared = Path(self.meta.get('source_dataset', root))
            source = declared.resolve() if declared.is_absolute() else root
            meta_path = source / 'meta.json'
            namespace = dict(dataset=str(source), meta_sha256=self._hash(meta_path) if meta_path.exists()
                             else self._hash(root / 'meta.json'))
        self.namespace = namespace
        self.namespace_id = hashlib.sha256(json.dumps(namespace, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.formats, self.sources = {}, {}
        entries = self.meta.get('package_sources')
        if entries is None and (root / 'source_markers.json').exists():
            self._hash(root / 'source_markers.json')
            entries = json.loads((root / 'source_markers.json').read_text())
        if entries is None:
            entries = []
            for path in sorted((root / 'ready').glob('*.json')):
                self._hash(path)
                entries.append(json.loads(path.read_text()))
        seen = {}
        run_formats = {}
        for entry in entries:
            if entry.get('split') != dataset.diagnostic_split:
                continue
            game = int(entry['game_id'])
            identity = (entry.get('source_shard'), entry.get('source_offset'), entry.get('source_len'))
            if game in seen and seen[game] != identity:
                raise ValueError('Diagnostic game ID aliases different source games')
            seen[game] = identity
            if (entry.get('structure_sha256') != dataset.structure.digest
                    or entry.get('replay_hashes_checked') is not True):
                continue
            if not entry.get('source_shard'):
                continue
            if not Path(entry['source_shard']).is_absolute():
                continue  # Relative source paths have no trustworthy base here.
            run = Path(entry['source_shard']).parent.parent
            if str(run) not in run_formats:
                run_formats[str(run)] = self._format(run)
            self.formats[game] = run_formats[str(run)]
            self.sources[game] = dict(source_shard=entry['source_shard'],
                                     source_offset=entry.get('source_offset'), source_len=entry.get('source_len'))

    def _hash(self, path):
        value = file_hash(path)
        self.inputs[str(Path(path).resolve())] = value
        return value

    def _format(self, run):
        metadata = run / 'run.json'
        if not metadata.exists():
            return 'unknown'
        self._hash(metadata)
        value = json.loads(metadata.read_text())
        shape = value.get('shape', value.get('format'))
        seats, teams = value.get('seats'), value.get('teams')
        # Teacher run.json omits shape; its completed job command records it.
        job_path = run.parent.parent / 'jobs' / run.name / 'job.json'
        if shape is None and job_path.exists():
            self._hash(job_path)
            job = json.loads(job_path.read_text())
            command = job.get('command', [])
            def option(name):
                if command.count(name) != 1:
                    return None
                index = command.index(name) + 1
                return command[index] if index < len(command) else None
            if job.get('status') == 'complete' and option('--out') and Path(option('--out')).resolve() == run.resolve():
                shape = option('--shape') or option('--shapes')
                if option('--seats') is not None and int(option('--seats')) != seats:
                    raise ValueError('Source format metadata disagrees with its completed job')
        if not isinstance(shape, str) or ',' in shape or not isinstance(seats, int) or not isinstance(teams, list):
            return 'unknown'
        return f'{shape}/seats={seats}/teams=' + (','.join(map(str, teams)) if teams else 'ffa')
