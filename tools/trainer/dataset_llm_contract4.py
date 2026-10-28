"""Strict admission of native-replayed executed LLM labels, for BC only."""
import hashlib
import json
from pathlib import Path
from dataset_contract4 import SHA256


def _require(value, message):
    if not value:
        raise ValueError('LLM BC ' + message)


def _digest(value):
    return isinstance(value, str) and SHA256.fullmatch(value) is not None


def _bound_file(root, name, expected):
    _require(isinstance(name, str) and _digest(expected), 'invalid file identity')
    path = (root / name).resolve()
    _require(path.is_relative_to(root) and path.is_file(), 'file escapes dataset or is missing')
    with path.open('rb') as stream:
        actual = hashlib.file_digest(stream, 'sha256').hexdigest()
    _require(actual == expected, 'file digest mismatch: ' + name)
    return path


def validate_llm_dataset(root, meta):
    """Validate immutable companions and full frame shards before cache reuse.

    The native producer proves replay legality and per-input attribution; Python
    preserves that proof across packing without changing observations or targets.
    This is a technical admission contract, not a teacher-quality assessment.
    """
    if not any(key.startswith('llm_bc_') for key in meta):
        return
    _require(not any(key.startswith('shadow_bc_') for key in meta), 'cannot also claim shadow labels')
    _require(meta.get('llm_bc_version') == 1
             and meta.get('llm_bc_contract') == 'verified_executed_llm_v1'
             and meta.get('llm_bc_status') == 'complete'
             and meta.get('ppo_eligible') is False
             and meta.get('policy_provenance_version') is None
             and meta.get('encoder_version') == 7 and meta.get('trajectory_version') == 2,
             'incomplete or incompatible contract')
    root = Path(root).resolve()
    sources = meta.get('llm_bc_sources')
    labels = meta.get('llm_bc_label_sha256')
    manifests = meta.get('llm_bc_manifest_sha256')
    frames = meta.get('llm_bc_frame_sha256')
    _require(isinstance(sources, list) and sources and all(isinstance(v, dict) for v in sources),
             'missing source identities')
    _require(all(isinstance(v, dict) for v in (labels, manifests, frames)), 'missing file digest maps')
    expected_frames = set()
    for split in ('train', 'val'):
        names = meta.get(split + '_shards')
        _require(isinstance(names, list) and all(isinstance(n, str) and Path(n).name == n for n in names)
                 and len(names) == len(set(names)), 'invalid shard names')
        expected_frames.update(f'{split}/{name}' for name in names)
    _require(expected_frames and set(frames) == expected_frames, 'unbound or missing full-history shards')
    for name, value in frames.items():
        _bound_file(root, name, value)
    games, identities, records, label_names, manifest_names = set(), set(), set(), set(), set()
    for source in sources:
        game, source_game = source.get('converted_game_id'), source.get('source_game')
        _require(type(game) is int and game >= 1 and game not in games, 'duplicate/invalid converted game')
        _require(isinstance(source_game, str) and 0 < len(source_game) <= 256, 'invalid source game')
        for key in ('source_namespace', 'run_sha256', 'manifest_sha256', 'record_sha256'):
            _require(_digest(source.get(key)), 'invalid source digest: ' + key)
        _require(source['source_namespace'] == source['run_sha256'], 'namespace differs from run')
        identity = (source['source_namespace'], source_game, source['record_sha256'])
        _require(identity not in identities and source['record_sha256'] not in records, 'duplicate source game identity')
        name = f'llm-bc/game-{game:06}.jsonl'
        manifest = f'llm-bc/game-{game:06}.manifest.json'
        _require(source.get('converted_labels') == name and source.get('converted_manifest') == manifest,
                 'noncanonical companion path')
        path = _bound_file(root, name, labels.get(name))
        manifest_path = _bound_file(root, manifest, manifests.get(manifest))
        _require(manifests[manifest] == source['manifest_sha256'], 'manifest source identity differs')
        _require(manifest_path.stat().st_size <= 64 * 1024**2, 'manifest too large')
        models = _manifest(json.loads(manifest_path.read_bytes()), source)
        _require(path.stat().st_size <= 64 * 1024**2, 'companion too large')
        expected_header = dict(kind='header', schema=1, converted_game_id=game,
            source_namespace=source['source_namespace'], source_game=source_game,
            run_sha256=source['run_sha256'], manifest_sha256=source['manifest_sha256'],
            record_sha256=source['record_sha256'], label_use='bc_only', ppo_eligible=False)
        _labels(path, expected_header, models)
        games.add(game);identities.add(identity);records.add(source['record_sha256']);label_names.add(name);manifest_names.add(manifest)
    _require(set(labels) == label_names and set(manifests) == manifest_names, 'unbound companion files')


def _labels(path, header, models):
    count, cursors, inputs, footer = 0, set(), set(), None
    with path.open('rb') as stream:
        first = stream.readline()
        _require(first and json.loads(first) == header, 'companion header binding differs')
        for line in stream:
            row = json.loads(line)
            _require(footer is None, 'data after completed footer')
            if row.get('kind') == 'completed':
                footer = row
                continue
            _require(set(row) == {'kind', 'seat', 'step_cursor', 'record_input_n', 'executed_source',
                                  'ambiguous', 'model', 'executed_action', 'target'}
                     and row['kind'] == 'label' and row['executed_source'] == 'llm'
                     and row['ambiguous'] is False, 'non-LLM/ambiguous policy label')
            seat, cursor, n = row['seat'], row['step_cursor'], row['record_input_n']
            _require(all(type(v) is int and v >= 0 for v in (seat, cursor, n))
                     and seat in models and (seat, cursor) not in cursors and n not in inputs,
                     'duplicate/invalid frame or executed input')
            _require(row['model'] == models[seat]
                     and isinstance(row['executed_action'], (str, dict)) and bool(row['executed_action'])
                     and isinstance(row['target'], dict) and bool(row['target']), 'invalid label payload')
            cursors.add((seat, cursor));inputs.add(n);count += 1
    _require(footer == dict(kind='completed', labels=count, ppo_eligible=False), 'completion/count differs')


def _manifest(value, source):
    _require(isinstance(value, dict) and value.get('schema_version') == 1
             and value.get('contract') == 'admitted_executed_llm_v1'
             and value.get('source_game') == source['source_game']
             and value.get('record_sha256') == source['record_sha256']
             and _digest(value.get('record_artifact_sha256')), 'manifest record binding differs')
    def artifact(row):
        _require(isinstance(row, dict) and isinstance(row.get('path'), str)
                 and Path(row['path']).is_absolute() and _digest(row.get('sha256')),
                 'manifest artifact role/digest missing')
    for name in ('request', 'admitted_receipt', 'technical_result'):
        artifact(value.get(name))
    seats = value.get('seats')
    _require(isinstance(seats, list) and 2 <= len(seats) <= 8, 'manifest seat count invalid')
    models = {}
    for row in seats:
        _require(isinstance(row, dict) and type(row.get('seat')) is int
                 and row['seat'] not in models and isinstance(row.get('model'), str)
                 and bool(row['model']), 'manifest seat/model binding invalid')
        artifact(row.get('transcript'));artifact(row.get('origins'))
        models[row['seat']] = row['model']
    _require(set(models) == set(range(len(seats))), 'manifest seats incomplete')
    return models
