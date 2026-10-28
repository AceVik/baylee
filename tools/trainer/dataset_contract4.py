"""Admission checks for counterfactual teacher labels, distinct from behavior."""
import hashlib
import json
from pathlib import Path
import re


SHA256 = re.compile(r'[0-9a-f]{64}\Z')


def reject_shadow_policy_gradient(meta):
    if any(key.startswith('llm_bc_') for key in meta):
        raise ValueError('Verified LLM labels are BC-only, never PPO behavior probabilities')
    if any(key.startswith('shadow_bc_') for key in meta) or meta.get('ppo_eligible') is False:
        raise ValueError('Shadow teacher labels are BC-only, never PPO behavior probabilities')


def validate_shadow_dataset(root, meta):
    """Fail before cache access on partial, contradictory or tampered sidecars.

    Rust verifies the original replay and teacher actions. This check retains
    that provenance across the binary cache, whose frame ABI stays unchanged.
    Historical ordinary datasets are unaffected. Source files need not remain
    mounted; their identities are preserved in the converted companion files.
    """
    if not any(key.startswith('shadow_bc_') for key in meta):
        return
    if (meta.get('shadow_bc_version') != 1
            or meta.get('shadow_bc_contract') != 'verified_view_only_teacher_v1'
            or meta.get('shadow_bc_status') != 'complete'
            or meta.get('ppo_eligible') is not False
            or meta.get('policy_provenance_version') is not None
            or meta.get('encoder_version') != 7 or meta.get('trajectory_version') != 2):
        raise ValueError('Incomplete or incompatible shadow BC dataset contract')
    sources, hashes = meta.get('shadow_bc_sources'), meta.get('shadow_bc_label_sha256')
    if not isinstance(sources, list) or not sources or not isinstance(hashes, dict):
        raise ValueError('Missing shadow BC source identities')
    names, games, identities = set(), set(), set()
    root = Path(root).resolve()
    for source in sources:
        game = source['converted_game_id']
        name = source['converted_labels']
        identity = (source['source_namespace'], source['source_game'], source['record_sha256'])
        if (not isinstance(game, int) or game < 1 or game in games or identity in identities
                or name != f'shadow-bc/game-{game:06}.jsonl' or name in names):
            raise ValueError('Duplicate or inconsistent shadow BC game identity')
        for key in ('source_namespace', 'run_sha256', 'labels_sha256', 'record_sha256'):
            if not isinstance(source.get(key), str) or not SHA256.fullmatch(source[key]):
                raise ValueError('Invalid shadow BC source digest')
        if source['source_namespace'] != source['run_sha256']:
            raise ValueError('Shadow namespace differs from source run')
        path = (root / name).resolve()
        if not path.is_relative_to(root) or path.stat().st_size > 64 * 1024**2:
            raise ValueError('Shadow BC companion path/size invalid')
        digest, header, footer, count, cursors = hashlib.sha256(), None, None, 0, set()
        with path.open('rb') as stream:
            for line in stream:
                digest.update(line)
                row = json.loads(line)
                if header is None:
                    header = row
                    continue
                if footer is not None:
                    raise ValueError('Data after completed shadow BC footer')
                if row.get('kind') == 'completed':
                    footer = row
                elif row.get('kind') == 'label':
                    cursor = (row.get('seat'), row.get('step_cursor'))
                    if (not all(type(v) is int and v >= 0 for v in cursor)
                            or cursor[0] >= 8 or cursor in cursors):
                        raise ValueError('Duplicate or invalid shadow BC frame identity')
                    cursors.add(cursor)
                    count += 1
                else:
                    raise ValueError('Unknown shadow BC companion record')
        if digest.hexdigest() != hashes.get(name):
            raise ValueError('Shadow BC companion digest mismatch')
        expected = dict(kind='header', schema=1, converted_game_id=game,
                        source_namespace=source['source_namespace'], source_game=source['source_game'],
                        run_sha256=source['run_sha256'], original_labels_sha256=source['labels_sha256'],
                        record_sha256=source['record_sha256'],
                        label_use='bc_only', ppo_eligible=False)
        if header != expected or footer != dict(kind='completed', labels=count, ppo_eligible=False):
            raise ValueError('Shadow BC companion binding/footer mismatch')
        names.add(name)
        games.add(game)
        identities.add(identity)
    if names != set(hashes):
        raise ValueError('Unbound shadow BC companion files')


def validate_bc_dataset(root, meta):
    """Validate both optional BC provenance contracts before any cached read."""
    validate_shadow_dataset(root, meta)
    from dataset_llm_contract4 import validate_llm_dataset
    validate_llm_dataset(root, meta)
