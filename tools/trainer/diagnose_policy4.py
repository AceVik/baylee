"""Opt-in full-history policy diagnostics for strict v4/v5 checkpoints; never train."""
import argparse
import json
from pathlib import Path
import torch
from torch.utils.data import DataLoader
import model4 as M
from export_onnx4 import load_export_checkpoint
from sampler4 import StatefulBatchSampler4
from dataset4 import collate_trajectories
from torch_io4 import atomic_json
from training4.diagnostics import DiagnosticDataset, Provenance, Collector
from training4.diagnostics.data import file_hash, PENDING_IDS
from training4.validation_device import evaluate_device
from training4.objective import LOSS_VERSION


def diagnostic_loader(data, args, device):
    return DataLoader(data,batch_sampler=StatefulBatchSampler4(data,args.batch_size,shuffle=False),
                      collate_fn=collate_trajectories,num_workers=args.num_workers,
                      pin_memory=(device.type == "cuda"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model',type=Path,required=True,help='Checkpoint file or directory containing net.pt')
    parser.add_argument('--data',type=Path,required=True)
    parser.add_argument('--split',choices=('train','val'),default='val')
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--device',choices=('auto','cpu','cuda'),default='auto')
    parser.add_argument('--bf16',action='store_true',help='Match optimized CUDA validation precision; default FP32')
    parser.add_argument('--batch-size',type=int,default=8)
    parser.add_argument('--unroll',type=int,default=32)
    parser.add_argument('--thinking-steps',type=int,choices=(1,2,4,8),default=2)
    parser.add_argument('--board-chunk',type=int,default=4)
    parser.add_argument('--decision-chunk',type=int,default=4)
    parser.add_argument('--gpu-prefetch',type=int,choices=range(4),default=1)
    parser.add_argument('--num-workers',type=int,default=0)
    parser.add_argument('--cpu-threads',type=int,default=2)
    parser.add_argument('--progress-every',type=int,default=100)
    args=parser.parse_args()
    if min(args.batch_size,args.unroll,args.board_chunk,args.decision_chunk,args.cpu_threads)<1 or args.num_workers<0:
        parser.error('Positive batching/chunk/thread sizes and nonnegative workers required')
    args.model=args.model.expanduser();args.data=args.data.expanduser();args.out=args.out.expanduser()
    if args.out.exists() or args.out.is_symlink():
        raise FileExistsError('Preserving existing diagnostic report')
    checkpoint=args.model if args.model.is_file() else args.model/'net.pt'
    before=file_hash(checkpoint)
    device=torch.device('cuda' if args.device=='auto' and torch.cuda.is_available() else
                        'cpu' if args.device=='auto' else args.device)
    if args.bf16 and (device.type!='cuda' or not torch.cuda.is_bf16_supported()):
        raise ValueError('--bf16 requires supported CUDA; no silent precision change')
    torch.set_num_threads(args.cpu_threads)
    if device.type=='cuda':
        torch.backends.cuda.matmul.allow_tf32=False
        torch.backends.cudnn.allow_tf32=False
    saved,net,module=load_export_checkpoint(checkpoint)
    net.to(device).eval()
    net.decision_chunk=args.decision_chunk
    net.gpu_prefetch=args.gpu_prefetch
    net.cache_thinking_kv=True
    net.prune_unused_policy_heads=True
    data=DiagnosticDataset(args.data,split=args.split,unroll_len=args.unroll,encoder_version=saved["encoder_version"])
    if data.structure.digest != net.rules.digest:
        raise ValueError('Checkpoint and diagnostic data use different public catalogs')
    provenance=Provenance(data)
    collector=Collector(provenance)
    source_inputs={str(data.data_dir/'meta.json'):file_hash(data.data_dir/'meta.json'), **provenance.inputs}
    loader=diagnostic_loader(data,args,device)
    try:
        validation=evaluate_device(net,loader,device,args.thinking_steps,args.board_chunk,args.progress_every,
                                   diagnostic_collector=collector,use_bf16=args.bf16)
        for path,digest in source_inputs.items():
            if file_hash(path)!=digest:
                raise ValueError('Diagnostic metadata changed during evaluation: '+path)
        if file_hash(checkpoint)!=before:
            raise ValueError('Checkpoint changed during diagnosis')
        report=dict(schema_version=1,kind='complete_history_policy_diagnostics',training=False,
            checkpoint=str(checkpoint.resolve()),checkpoint_sha256=before,model_version=module.MODEL_VERSION,
            architecture=saved.get('architecture'),checkpoint_step=saved.get('step_num'),
            encoder_version=saved["encoder_version"],catalog_sha256=data.structure.digest,
            namespace=provenance.namespace,namespace_id=provenance.namespace_id,
            data=str(data.data_dir.resolve()),split=args.split,data_fingerprint=data.fingerprint,
            metadata_input_sha256=source_inputs,
            settings={**{key:value for key,value in vars(args).items() if not isinstance(value,Path)},
                      'actual_device':str(device),'precision':'bf16' if args.bf16 else 'fp32'},
            history='Every complete seat trajectory from cursor0 through its terminal; K0 observations, K>=1 decisions; persistent memory across segments.',
            head_names={'0':'SingleChoice','1':'SubsetPick','2':'PairPick','3':'ScalarValue'},pending_kind_ids=PENDING_IDS,
            validation_objective_version=LOSS_VERSION,
            data_identity_scope='Metadata/catalog hashes and store fingerprint (shard names/sizes/mtime); payload bytes not rehashed or replayed.',
            validation_metrics_existing_objective=validation,diagnostics=collector.report())
        args.out.parent.mkdir(parents=True,exist_ok=True)
        atomic_json(args.out,report)
        print(json.dumps(dict(report=str(args.out),frames=report['diagnostics']['global_metrics']['sums']['frames'],
                              games=len(report['diagnostics']['per_game']))),flush=True)
    finally:
        data.store.close()


if __name__=='__main__':
    main()
