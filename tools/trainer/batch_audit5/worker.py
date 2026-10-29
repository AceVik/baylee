"""Bounded CPU-only shape sampling of a saved recurrent training epoch (no forward)."""
import argparse
import gc
import importlib
import json
import os
from pathlib import Path
import sys
import time
import numpy as np

MIB = 1024 ** 2


def sample_indices(length, count):
    if length < 1 or count < 1:
        raise ValueError('Nonempty epoch and positive sample count required')
    return sorted(set(np.linspace(0, length - 1, min(count, length), dtype=int).tolist()))


def padded_bytes(items):
    """Exact padded_stack output bytes, given mappings of key -> (shape,dtype)."""
    total = 0
    for key in items[0]:
        shapes = [item[key][0] for item in items]
        shape = (len(items),) + tuple(max(dim) for dim in zip(*shapes))
        total += int(np.prod(shape)) * np.dtype(items[0][key][1]).itemsize
    return total


def describe(item):
    return {key:(value.shape, value.dtype) for key,value in item.items()}


def measure(batch):
    valid = batch['step_valid'].numpy()
    b,u = valid.shape
    storage={v.untyped_storage().data_ptr():v.untyped_storage().nbytes() for v in batch.values()}
    result = dict(raw_batch_bytes=sum(storage.values()), logical_batch_bytes=sum(v.numel()*v.element_size() for v in batch.values()),
                  valid_frames=int(valid.sum()),active_timesteps=int(valid.any(0).sum()),
                  thinking_decision_rows=int((batch['is_decision'].numpy() & valid).sum()))
    result['decision_head_counts']={str(int(k)):int(((batch['head'].numpy()==k)&valid&batch['is_decision'].numpy()).sum()) for k in range(4)}
    result['thinking_row_fraction_B_active_time'] = result['thinking_decision_rows']/max(1,b*result['active_timesteps'])
    for label,key in (('entities','entity_mask'),('edges','edge_mask'),('candidates','cand_mask'),('pairs','pair_mask')):
        mask=batch[key].numpy() & valid[:,:,None]
        counts=mask.sum(-1)
        width=mask.shape[-1]
        result[label]=dict(padded_width=width, max_actual=int(counts.max()), actual_entries=int(counts.sum()),
                           allocated_entries=int(mask.size),padding_fraction=1-float(counts.sum())/mask.size)
        if label=='entities':
            actual=int((counts**2).sum()); allocated=b*u*width**2
            result['entity_attention_E2']=dict(actual_entries=actual,allocated_entries=allocated,
                                               padding_fraction=1-actual/max(1,allocated))
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--job',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--samples',type=int,default=48)
    p.add_argument('--budget-mib',type=int,default=512)
    args=p.parse_args()
    if args.out.exists(): raise FileExistsError(args.out)
    if not 1 <= args.samples <= 64: p.error('samples must be 1..64')
    job=json.loads(args.job.read_text());command=job['command']
    def option(name): return command[command.index(name)+1]
    runtime=Path(command[2]).parent
    sys.path.insert(0,str(runtime))
    D=importlib.import_module('dataset4'); S=importlib.import_module('sampler4')
    import torch
    torch.set_num_threads(1)
    # Load only the opt-in provenance helper; dataset/sampler imports stay frozen.
    spec=importlib.util.spec_from_file_location('audit_provenance',Path(__file__).parent/'training4/diagnostics/data.py')
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    Provenance,file_hash=helper.Provenance,helper.file_hash
    started=time.monotonic()
    dataset=D.TrajectoryDataset4(Path(option('--data')),split='train',unroll_len=int(option('--unroll')))
    dataset.diagnostic_split='train'
    provenance=Provenance(dataset)
    sampler=S.StatefulBatchSampler4(dataset,int(option('--batch-size')),shuffle=True,seed=int(option('--seed')))
    batches=list(sampler)
    records=[]
    try:
        for index in sample_indices(len(batches),args.samples):
            ids=batches[index]; schemas=[]; identities=[]; frame_peak=0
            for segment in ids:
                rows=dataset.segments[segment if segment>=0 else 0]
                descriptions=[]
                for row in rows:
                    frame=dataset.encode_single_step(row)
                    frame_peak=max(frame_peak,sum(v.nbytes for v in frame.values()))
                    descriptions.append(describe(frame)); del frame
                schema={key:((dataset.unroll_len,)+tuple(max(d) for d in zip(*(x[key][0] for x in descriptions))),descriptions[0][key][1])
                        for key in descriptions[0]}
                schema.update(step_valid=((dataset.unroll_len,),np.dtype(bool)),is_new_traj=((),np.dtype(bool)),segment_index=((),np.dtype('int64')))
                schemas.append(schema)
                entry=dataset.store.index[int(rows[0])]
                game=int(entry['game'])
                identities.append(dict(segment_id=segment,game_id=game if segment>=0 else None,seat=int(entry['seat']) if segment>=0 else None,
                    cursor=int(entry['cursor']) if segment>=0 else None,source_format=provenance.formats.get(game,'unknown'),source=provenance.sources.get(game)))
            collated=padded_bytes(schemas)
            segment_bytes=sum(sum(int(np.prod(shape))*np.dtype(dtype).itemsize for shape,dtype in schema.values()) for schema in schemas)
            # Includes source segment arrays, padded_stack temporary, collated owner,
            # and frame decode/segment assembly scratch. Excludes fixed index/catalog/Python RSS.
            peak=segment_bytes+2*collated+2*frame_peak*dataset.unroll_len
            record=dict(batch_index=index,segments=identities,predicted_collated_bytes=collated,conservative_tensor_peak_bytes=peak)
            if peak>args.budget_mib*MIB:
                record.update(skipped=True,reason='Conservative tensor allocation estimate exceeds per-batch budget')
            else:
                items=[dataset[i] for i in ids]
                batch=D.collate_trajectories(items)
                record.update(skipped=False,**measure(batch))
                del batch,items
            records.append(record);gc.collect()
            print(f"batch {len(records)}/{min(args.samples,len(batches))} index={index} skipped={record['skipped']}",flush=True)
    finally: dataset.store.close()
    measured=[r for r in records if not r['skipped']]
    ordered=sorted(measured,key=lambda r:r['raw_batch_bytes'])
    def pick(q): return ordered[min(len(ordered)-1,int((len(ordered)-1)*q))]['batch_index'] if ordered else None
    report=dict(schema_version=1,tool_sha256=file_hash(__file__),job=str(args.job.resolve()),job_sha256=file_hash(args.job),runtime=str(runtime),
        frozen_code_sha256={name:file_hash(runtime/name) for name in ('dataset4.py','sampler4.py')},
        dataset=str(dataset.data_dir),data_fingerprint=dataset.fingerprint,namespace=provenance.namespace,
        metadata_inputs=provenance.inputs,seed=int(option('--seed')),epoch=0,sampler_version=S.StatefulBatchSampler4.VERSION,
        batch_size=int(option('--batch-size')),unroll=dataset.unroll_len,epoch_batches=len(batches),samples=len(records),measured=len(measured),
        skipped=len(records)-len(measured),budget_mib=args.budget_mib,elapsed_seconds=time.monotonic()-started,
        fractions={name:sum(test(r['raw_batch_bytes']/MIB) for r in measured)/len(measured) if measured else None
                   for name,test in [('le128MiB',lambda x:x<=128),('gt128MiB',lambda x:x>128),('gt256MiB',lambda x:x>256)]},
        benchmark_windows=dict(first=records[0]['batch_index'],median=pick(.5),high_p95=pick(.95),argmax=pick(1)),batches=records,
        limitations=['Deterministic evenly-spaced sample; sampled maximum is NOT global maximum.',
         'Shape analysis only; no model forward, training, GPU or skipping K0 observations.',
         'Benchmarking sampled mid-game windows with H0 is NOT a chronologically correct quality evaluation.',
         'Budget bounds estimated tensor allocations, not fixed index/catalog/Python RSS; dummy candidate/pair masks count as allocated entries.',
         'No full payload hash/CRC scan, no cache generation; unknown formats remain unknown.'])
    args.out.parent.mkdir(parents=True,exist_ok=True)
    temporary=args.out.with_suffix('.tmp');temporary.write_text(json.dumps(report,indent=2)+'\n');os.replace(temporary,args.out)


if __name__=='__main__': main()
