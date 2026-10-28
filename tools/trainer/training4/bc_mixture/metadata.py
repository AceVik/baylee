"""Source-record/table identities for opt-in supervised mixtures."""
import hashlib
import json
from pathlib import Path


def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()


def game_metadata(dataset, split):
    root=dataset.data_dir.resolve();meta=dataset.meta
    namespace=meta.get('source_namespace',dict(dataset=str(root)))
    entries=meta.get('package_sources')
    if entries is None and (root/'source_markers.json').exists():
        entries=json.loads((root/'source_markers.json').read_text())
    if entries is None:
        entries=[json.loads(p.read_text()) for p in sorted((root/'ready').glob('*.json'))]
    entries=[e for e in entries if e['split']==split]
    source_hashes={e['converted_game_id']:e['record_sha256'] for e in [*meta.get('shadow_bc_sources',[]),*meta.get('llm_bc_sources',[])]}
    indices={};games={};receipts={};seen_records=set();observed_by_game={}
    # Dataset4 already groups/validates every complete seat trajectory. Inspect
    # its first row once, rather than scanning the full frame index per game.
    for trajectory in dataset.trajectories:
        row=dataset.store.index[dataset.segments[trajectory[0]][0]]
        observed_by_game.setdefault(int(row['game']),set()).add(int(row['seat']))
    for entry in entries:
        game=int(entry['game_id']);source=Path(entry['source_shard'])
        if not source.is_absolute() or entry.get('replay_hashes_checked') is not True:
            raise ValueError('BC mixture requires verified absolute source markers')
        source=source.resolve();record=(str(source),int(entry['source_offset']),int(entry['source_len']))
        if record in seen_records:raise ValueError('Duplicate converted source record')
        seen_records.add(record)
        index=source.parent.parent/'games.jsonl'
        if index not in indices:
            rows={};hasher=hashlib.sha256();run_seats=None
            run_path=index.parent/'run.json'
            if run_path.exists():
                raw=run_path.read_bytes();run_seats=json.loads(raw).get('seats')
                receipts[str(run_path)]=hashlib.sha256(raw).hexdigest()
            with index.open('rb') as stream:
                for line in stream:
                    hasher.update(line);row=json.loads(line)
                    key=(str((index.parent/'records'/row['shard']).resolve()),int(row['offset']),int(row['len']))
                    if key in rows:raise ValueError('Duplicate source-record metadata')
                    seats=len(row['decks']) if isinstance(row.get('decks'),list) else run_seats
                    if type(seats) is not int or not 2<=seats<=8:
                        raise ValueError('Source index lacks complete table seat metadata')
                    if run_seats is not None and seats!=run_seats:
                        raise ValueError('Source index and run seat metadata differ')
                    rows[key]=(int(row['seed']),seats)
            indices[index]=rows;receipts[str(index)]=hasher.hexdigest()
        if record not in indices[index]:raise ValueError('Source record missing from game index')
        seed,seats=indices[index][record]
        observed_seats=observed_by_game.get(game,set())
        if observed_seats!=set(range(seats)):
            raise ValueError('BC mixture requires all source game seats')
        value=dict(record=record,record_sha256=source_hashes.get(game),
                   table=digest(dict(namespace=namespace,seed=seed)),
                   source_table=digest(dict(run=str(index.parent),seed=seed)))
        if game in games:raise ValueError('Duplicate converted game marker')
        games[game]=value
    if set(games)!=set(observed_by_game):raise ValueError('Source markers do not cover exact dataset split')
    return games,receipts


def reject_overlap(left,right):
    records={tuple(v['record']) for v in left.values()}
    hashes={v['record_sha256'] for v in left.values() if v.get('record_sha256')}
    tables={v['table'] for v in left.values()};source_tables={v['source_table'] for v in left.values()}
    for value in right.values():
        if tuple(value['record']) in records or value['table'] in tables or value['source_table'] in source_tables or value.get('record_sha256') in hashes:
            raise ValueError('BC mixture source game/table overlaps another source or validation')
