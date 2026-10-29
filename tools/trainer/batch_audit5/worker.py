"""CPU PyTorch batch measurement worker; planning and report math live in Rust."""
import gc
import importlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

os.environ['CUDA_VISIBLE_DEVICES'] = ''
import torch


def describe(item):
    return {key: dict(shape=list(value.shape), element_size=int(value.itemsize))
            for key, value in item.items()}


def measure(batch):
    """Read actual Tensor storage and masked counts; no report aggregation."""
    valid = batch['step_valid'].bool()
    storage = {v.untyped_storage().data_ptr(): v.untyped_storage().nbytes()
               for v in batch.values()}
    result = dict(raw_batch_bytes=sum(storage.values()),
                  logical_batch_bytes=sum(v.numel() * v.element_size() for v in batch.values()),
                  batch_size=valid.shape[0], batch_unroll=valid.shape[1],
                  valid_frames=int(valid.sum()), active_timesteps=int(valid.any(0).sum()),
                  thinking_decision_rows=int((batch['is_decision'] & valid).sum()))
    result['decision_head_counts'] = {
        str(k): int(((batch['head'] == k) & valid & batch['is_decision']).sum())
        for k in range(4)}
    for label, key in (('entities', 'entity_mask'), ('edges', 'edge_mask'),
                       ('candidates', 'cand_mask'), ('pairs', 'pair_mask')):
        mask = batch[key] & valid[:, :, None]
        counts = mask.sum(-1)
        result[label] = dict(padded_width=mask.shape[-1], max_actual=int(counts.max()),
                             actual_entries=int(counts.sum()), allocated_entries=mask.numel())
        if label == 'entities':
            result['entity_attention_E2'] = dict(actual_entries=int(counts.square().sum()))
    return result


def request(context, kind, **value):
    result = subprocess.run([context['native_binary'], '--tensor-boundary'],
                            input=json.dumps(dict(kind=kind, **value), allow_nan=False),
                            capture_output=True, text=True)
    if result.returncode:
        raise ValueError('Native batch audit rejected: ' + result.stderr.strip())
    return json.loads(result.stdout)


def measure_plan(plan):
    runtime = Path(plan['runtime'])
    sys.path.insert(0, str(runtime))
    D = importlib.import_module('dataset4')
    S = importlib.import_module('sampler4')
    torch.set_num_threads(1)
    trainer = Path(__file__).resolve().parents[1]
    sys.path.append(str(trainer))
    spec = importlib.util.spec_from_file_location('audit_provenance', trainer / 'training4/diagnostics/data.py')
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    started = time.monotonic()
    dataset = D.TrajectoryDataset4(Path(plan['data']), split='train', unroll_len=plan['unroll'])
    dataset.diagnostic_split = 'train'
    try:
        provenance = helper.Provenance(dataset)
        sampler = S.StatefulBatchSampler4(dataset, plan['batch_size'], shuffle=True, seed=plan['seed'])
        batches = list(sampler)
        metadata = dict(epoch_batches=len(batches), dataset=str(dataset.data_dir),
                        data_fingerprint=dataset.fingerprint, namespace=provenance.namespace,
                        metadata_inputs=provenance.inputs, sampler_version=S.StatefulBatchSampler4.VERSION)
        indices = request(plan, 'indices', length=len(batches), count=plan['samples'])
        records = []
        for index in indices:
            ids = batches[index]
            segments, identities = [], []
            for segment in ids:
                rows = dataset.segments[segment if segment >= 0 else 0]
                descriptions = []
                for row in rows:
                    frame = dataset.encode_single_step(row)
                    descriptions.append(describe(frame))
                    del frame
                segments.append(descriptions)
                entry = dataset.store.index[int(rows[0])]
                game = int(entry['game'])
                identities.append(dict(segment_id=segment,
                    game_id=game if segment >= 0 else None,
                    seat=int(entry['seat']) if segment >= 0 else None,
                    cursor=int(entry['cursor']) if segment >= 0 else None,
                    source_format=provenance.formats.get(game, 'unknown'), source=provenance.sources.get(game)))
            estimate = request(plan, 'estimate', frame_schemas=segments,
                               unroll=dataset.unroll_len, budget_mib=plan['budget_mib'])
            measurement = None
            if estimate['measure']:
                items = [dataset[i] for i in ids]
                batch = D.collate_trajectories(items)
                measurement = measure(batch)
                del batch, items
            records.append(dict(batch_index=index, segments=identities,
                                estimate=estimate, measurement=measurement))
            gc.collect()
        completed = request(plan, 'finish', plan=plan, metadata=metadata, records=records,
                            elapsed_seconds=time.monotonic() - started)
        print(json.dumps(completed), flush=True)
    finally:
        dataset.store.close()


def main():
    plan = json.loads(Path(sys.argv[1]).read_text())
    try:
        measure_plan(plan)
    except Exception as error:
        request(plan, 'worker-error', diagnostic=plan['diagnostic'],
                error_type=type(error).__name__, detail=str(error))
        raise


if __name__ == '__main__':
    main()
