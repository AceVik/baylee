"""Objective-independent diagnostics preserve history, grouping and choice denominators."""
import copy
import gzip
import io
from contextlib import redirect_stdout
from unittest.mock import patch
import json
import math
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
import numpy as np
import torch
from torch.utils.data import DataLoader
import model4 as M
from dataset4 import collate_trajectories
from sampler4 import StatefulBatchSampler4
from test_training_execution4 import batch_fixture
from test_value_calibration4 import TinyDataset
from training4.diagnostics import Collector, DiagnosticDataset, Provenance
from training4.diagnostics.stats import FIELDS, step_statistics
from training4.validation_device import evaluate_device
from trajectory_store4 import pack_step_bst4


def provenance(namespace='source-A'):
    return SimpleNamespace(namespace_id=namespace,formats={1:'duels',2:'commander'},sources={})


def fixture():
    batch=batch_fixture(b=11,u=1)
    batch['step_valid'].fill_(True);batch['step_valid'][9]=False
    batch['is_decision'].fill_(True);batch['is_decision'][10]=False
    batch['head'][:,0]=torch.tensor([0,0,0,0,0,1,1,2,3,0,1])
    batch['loss_weight'].fill_(1);batch['loss_weight'][3]=0
    batch['source_kind']=torch.ones(11,1,dtype=torch.long);batch['source_kind'][2]=2
    batch['single_idx'].zero_();batch['single_idx'][1]=1
    batch['cand_feat'].zero_();batch['cand_feat'][:,:,0,0]=1
    batch['cand_mask'].fill_(False);batch['cand_mask'][:,:,:3]=True
    batch['pair_mask'].fill_(False);batch['pair_mask'][:,:,:2]=True
    batch['cand_free_mask'].zero_();batch['cand_free_mask'][5,0,:2]=1
    batch['cand_target_mask'].zero_();batch['cand_target_mask'][5,0,[0,2]]=1
    batch['pair_free_mask'].zero_();batch['pair_free_mask'][7,0,:2]=1
    batch['pair_target_mask'].zero_();batch['pair_target_mask'][7,0,:2]=1
    batch['scalar_target'].fill_(.5);batch['outcome'].fill_(.5)
    batch['diag_game']=torch.tensor([[1]]*5+[[2]]*3+[[1]]*3)
    batch['diag_seat']=torch.zeros(11,1,dtype=torch.long);batch['diag_seat'][8]=1
    batch['diag_pending']=batch['head'].clone()
    batch['diag_candidates']=torch.full((11,1),3);batch['diag_candidates'][4]=0
    batch['diag_pairs']=torch.full((11,1),2)
    cand=torch.tensor([[0.,1.,-1.]]*11)
    cand[5]=torch.tensor([1.,-1.,-5.])
    pair=torch.tensor([[1.,-1.]]*11)
    return batch,(torch.zeros(11,9,16),cand,pair,torch.full((11,),.75),torch.full((11,),.5))


class History(TinyDataset):
    def __getitem__(self,index):
        out=super().__getitem__(index)
        rows=self.segments[max(index,0)]
        out.update(head=np.zeros(2,np.int64),loss_weight=np.ones(2,np.float32),single_idx=np.zeros(2,np.int64),
            scalar_target=np.zeros(2,np.float32),aux_target=np.zeros((2,4),np.float32),
            diag_game=np.full(2,10 if index<2 else 11,np.int64),diag_seat=np.zeros(2,np.int64),
            diag_cursor=np.pad(self.store.index['cursor'][rows],(0,2-len(rows))),
            diag_pending=np.zeros(2,np.int64),diag_candidates=np.full(2,3,np.int64),diag_pairs=np.ones(2,np.int64))
        return out


class RecordingNet(M.PlayerNet4):
    def __init__(self):
        super().__init__(M.Config4(d_model=16,n_heads=2,n_layers=1,d_ff=32,structure_width=16))
        self.calls=[]
    def forward(self,*args,**kwargs):
        result=super().forward(*args,**kwargs)
        self.calls.append((args[-2].clone(),args[-1].clone(),result[0].clone()))
        return result


class Diagnostics(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1);torch.manual_seed(81)

    def pass_confusion_fixture(self):
        batch = batch_fixture(b=12, u=1)
        batch['step_valid'].fill_(True)
        batch['is_decision'].fill_(True)
        batch['head'].zero_()
        batch['loss_weight'].fill_(1)
        batch['source_kind'] = torch.ones(12, 1, dtype=torch.long)
        batch['single_idx'].fill_(1)
        batch['single_idx'][[0, 1, 4]] = 0
        batch['cand_feat'].zero_()
        batch['cand_feat'][:, :, 0, 0] = 1
        batch['cand_feat'][4, 0, 2, 0] = 1  # A second pass, not the target option.
        batch['cand_feat'][11, 0, 2, 0] = 1
        batch['cand_mask'].zero_()
        batch['cand_mask'][:, :, :3] = True
        batch['cand_mask'][5, 0, 0] = False  # Highest logit pass is unavailable.
        batch['diag_candidates'] = torch.full((12, 1), 3)
        batch['diag_candidates'][10] = 1  # Not a genuine choice.
        batch['diag_candidates'][11] = 2  # Raw count also excludes a pass.
        batch['diag_pairs'] = torch.full((12, 1), 2)
        batch['source_kind'][6] = 2
        batch['loss_weight'][7] = 0
        batch['step_valid'][8] = False
        batch['is_decision'][9] = False
        batch['diag_game'] = torch.tensor([[1], [2]] * 6)
        batch['diag_seat'] = torch.zeros(12, 1, dtype=torch.long)
        batch['diag_pending'] = torch.zeros(12, 1, dtype=torch.long)
        predicted = [0, 1, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2]
        logits = torch.zeros(12, 3)
        for row, option in enumerate(predicted):
            logits[row, option] = 10
        logits[5, 1] = logits[11, 1] = 5
        output = (torch.zeros(12, 9, 16), logits, torch.zeros(12, 2),
                  torch.zeros(12), torch.zeros(12))
        return batch, output

    def test_pass_confusion_masks_eligibility_and_multiple_pass_options(self):
        batch, output = self.pass_confusion_fixture()
        values = step_statistics(batch, 0, output).numpy()
        collector = Collector(provenance())
        collector.add_cpu(batch, values[:, None])
        report = collector.report()['global_metrics']
        sums, metrics = report['sums'], report['metrics']
        fields = ['pass_target_predicted_pass', 'pass_target_predicted_active',
                  'active_target_predicted_pass', 'active_target_predicted_active']
        self.assertEqual([sums[k] for k in fields], [2, 1, 1, 3])
        self.assertEqual(sums['single_choices'], 7)
        self.assertEqual(sum(sums[k] for k in fields), sums['single_choices'])
        self.assertEqual(sums['pass_targets'], 3)
        self.assertEqual(sums['active_targets'], 4)
        self.assertEqual(sums['pass_correct'], 1)
        self.assertEqual(sums['pass_target_predicted_other_pass'], 1)
        self.assertEqual(sums['active_correct'], 2)  # Wrong active is still active.
        self.assertEqual(metrics['predicted_pass_rate'], 3 / 7)
        self.assertEqual(metrics['pass_target_predicted_pass_rate'], 2 / 3)
        self.assertEqual(metrics['active_target_predicted_pass_rate'], 1 / 4)
        self.assertEqual(metrics['pass_target_predicted_other_pass_rate'], 1 / 3)
        # Masked and count-excluded pass candidates must never win argmax.
        self.assertEqual(values[5, FIELDS.index('active_target_predicted_active')], 1)
        self.assertEqual(values[11, FIELDS.index('active_target_predicted_active')], 1)
        for index in [6, 7, 8, 9, 10]:
            self.assertEqual(sum(values[index, FIELDS.index(k)] for k in fields), 0)

    def test_pass_confusion_is_additive_across_games_and_batches(self):
        batch, output = self.pass_confusion_fixture()
        values = step_statistics(batch, 0, output).numpy()[:, None]
        full, split = Collector(provenance()), Collector(provenance())
        full.add_cpu(batch, values)
        for part in [slice(0, 5), slice(5, None)]:
            split.add_cpu({k: v[part] for k, v in batch.items()}, values[part])
        self.assertEqual(full.report(), split.report())
        report = full.report()
        for key in ['pass_target_predicted_pass', 'pass_target_predicted_active',
                    'active_target_predicted_pass', 'active_target_predicted_active',
                    'pass_target_predicted_other_pass']:
            self.assertEqual(sum(g['sums'][key] for g in report['per_game']),
                             report['global_metrics']['sums'][key])

    def test_masks_denominators_and_threshold_labels(self):
        batch,output=fixture()
        stats=step_statistics(batch,0,output)
        collector=Collector(provenance());collector.add_cpu(batch,stats[:,None].numpy())
        report=collector.report();s=report['global_metrics']['sums'];m=report['global_metrics']['metrics']
        self.assertEqual([s[k] for k in ['frames','decisions','zero_weight_decisions','forced_source_decisions']], [10,9,1,1])
        self.assertEqual([s[k] for k in ['single_choices','single_options','pass_targets','active_targets']], [2,6,1,1])
        self.assertEqual(m['pass_target_accuracy'],0);self.assertEqual(m['active_target_accuracy'],1)
        self.assertAlmostEqual(s['single_nll_sum'],2*math.log(math.exp(0)+math.exp(1)+math.exp(-1))-1,places=6)
        self.assertEqual([s[k] for k in ['subset_choices','subset_free_entries','subset_exact_free_set']], [1,2,1])
        self.assertEqual(m['subset_threshold_exact_free_set_accuracy'],1)
        self.assertEqual(m['pair_threshold_exact_free_set_accuracy'],0)
        self.assertAlmostEqual(m['subset_free_entry_bce'],math.log1p(math.exp(-1)),places=6)
        self.assertEqual(m['scalar_mae'],.25);self.assertEqual(m['scalar_mse'],.0625)
        self.assertIn('NOT a decoded action',report['definitions']['threshold'])

    def test_partition_aggregation_and_conversion_namespaces_do_not_alias(self):
        batch,output=fixture();values=step_statistics(batch,0,output)[:,None].numpy()
        full=Collector(provenance());full.add_cpu(batch,values)
        split=Collector(provenance())
        for start,end in ((0,4),(4,11)):
            split.add_cpu({key:value[start:end] for key,value in batch.items()},values[start:end])
        np.testing.assert_array_equal(full.total,split.total)
        for dimension in full.groups:
            np.testing.assert_allclose(sum(full.groups[dimension].values()),full.total,rtol=1e-14,atol=1e-14)
        report=full.report()
        self.assertEqual(next(g for g in report['per_game'] if g['game_id']==1)['seats'],[0,1])
        other=Collector(provenance('different-conversion'));other.add_cpu(batch,values)
        self.assertTrue(set(full.groups['game']).isdisjoint(other.groups['game']))
        original=full.total.copy();full.groups['head']['0'][0]+=1
        np.testing.assert_array_equal(full.total,original)

    def test_host_head_pruning_matches_unpruned_stats_across_distinct_timesteps(self):
        batch,output=fixture()
        batch={key:value.repeat(1,2,*([1]*(value.ndim-2))) for key,value in batch.items()}
        batch['is_decision'][:,1]=False
        expected=np.stack([step_statistics(batch,t,output).numpy() for t in range(2)],axis=1)
        collector=Collector(provenance());collector.start_batch(batch)
        for t in range(2):collector.step(batch,t,output)
        collector.finish_batch(batch)
        reference=Collector(provenance());reference.add_cpu(batch,expected)
        np.testing.assert_allclose(collector.total,reference.total,rtol=0,atol=0)

    def test_zero_choice_denominators_are_null(self):
        batch,output=fixture();batch['loss_weight'].zero_()
        collector=Collector(provenance());collector.add_cpu(batch,step_statistics(batch,0,output)[:,None].numpy())
        metrics=collector.report()['global_metrics']['metrics']
        self.assertIsNone(metrics['single_nll']);self.assertIsNone(metrics['subset_free_entry_bce'])
        self.assertIsNone(metrics['scalar_mse'])
        for key in ('predicted_pass_rate', 'pass_target_predicted_pass_rate',
                    'pass_target_predicted_active_rate', 'active_target_predicted_pass_rate',
                    'active_target_predicted_active_rate', 'pass_target_predicted_other_pass_rate'):
            self.assertIsNone(metrics[key])

    def test_full_history_default_metrics_and_optimized_hook_agree(self):
        data=History(16)
        def loader():
            return DataLoader(data,batch_sampler=StatefulBatchSampler4(data,1,shuffle=False),collate_fn=collate_trajectories)
        net=RecordingNet().eval()
        baseline=evaluate_device(net,loader(),torch.device('cpu'),2,board_chunk=1)
        net.calls=[];collector=Collector(provenance())
        observed=evaluate_device(net,loader(),torch.device('cpu'),2,board_chunk=1,diagnostic_collector=collector)
        self.assertEqual(baseline,observed)
        self.assertEqual([int(call[1][0]) for call in net.calls],[0,0,2,0,0,0,0,0])
        torch.testing.assert_close(net.calls[1][0],net.calls[0][2],rtol=0,atol=0)
        torch.testing.assert_close(net.calls[2][0],net.calls[1][2],rtol=0,atol=0)
        self.assertTrue(torch.equal(net.calls[2][0],net.calls[2][2])) # decision doesn't advance persistent H
        self.assertTrue(torch.equal(net.calls[4][0],torch.zeros_like(net.calls[4][0])))
        optimized=copy.deepcopy(net);optimized.decision_chunk=2;optimized.prune_unused_policy_heads=True
        second=Collector(provenance())
        evaluate_device(optimized,loader(),torch.device('cpu'),2,board_chunk=2,diagnostic_collector=second)
        np.testing.assert_allclose(collector.total,second.total,rtol=2e-6,atol=2e-6)
        self.assertEqual(collector.report()['global_metrics']['sums']['frames'],6)
        self.assertEqual(collector.report()['global_metrics']['sums']['single_choices'],1)
        self.assertGreater(collector.report()['global_metrics']['sums']['single_nll_sum'],0)

    def test_binary_metadata_preserves_zero_counts_without_default_mask_invention(self):
        data=DiagnosticDataset.__new__(DiagnosticDataset)
        data.max_entities=M.MAX_ENTITIES;data.max_edges=M.MAX_EDGES;data.max_cands=M.MAX_CANDIDATES;data.max_pairs=M.MAX_PAIRS
        step=dict(game_id=123,seat=3,step_cursor=7,turn=1,head=0,pending_kind=8,is_decision=False,
            state=dict(entity_feat=[],entity_card_ids=[],entity_face_ids=[],entity_token_ids=[],entity_rule_refs=[],
                       seat_feat=np.zeros((8,M.SEAT_FEAT_WIDTH)).tolist(),global_feat=[0.]*M.GLOBAL_FEAT_WIDTH,edges=[]),
            cand_feat=[],cand_entity_idx=[],cand_rel_seat_idx=[],cand_rule_refs=[],pairs=[],
            target=dict(single_idx=0,multi_mask=[],free_mask=[],scalar_norm=0.),outcome=.5)
        encoded=data.encode_single_step({'_bst4':pack_step_bst4(step,{})})
        self.assertEqual([int(encoded[k]) for k in ['diag_game','diag_seat','diag_cursor','diag_pending','diag_candidates','diag_pairs']],
                         [123,3,7,8,0,0])

    def test_missing_raw_free_masks_fail_before_loader_repairs(self):
        data=DiagnosticDataset.__new__(DiagnosticDataset)
        step=dict(game_id=1,seat=0,step_cursor=0,turn=1,head=1,pending_kind=7,is_decision=True,
            state=dict(entity_feat=[],entity_card_ids=[],entity_rule_refs=[],
                       seat_feat=np.zeros((8,M.SEAT_FEAT_WIDTH)).tolist(),global_feat=[0.]*M.GLOBAL_FEAT_WIDTH,edges=[]),
            cand_feat=[[0.]*M.CAND_FEAT_WIDTH]*2,cand_entity_idx=[-1,-1],cand_rel_seat_idx=[-1,-1],
            cand_rule_refs=[[],[]],pairs=[],target=dict(single_idx=0,multi_mask=[1,0],scalar_norm=0.),outcome=.5)
        with self.assertRaisesRegex(ValueError,'incomplete'):
            data.encode_single_step({'_bst4':pack_step_bst4(step,{})})

    def test_loader_pins_only_cuda_without_initializing_gpu(self):
        import diagnose_policy4
        args=SimpleNamespace(batch_size=8,num_workers=0)
        data=SimpleNamespace(trajectories=[[0]])
        for kind,pinned in (("cpu",False),("cuda",True)):
            with patch.object(diagnose_policy4,"DataLoader") as loader:
                diagnose_policy4.diagnostic_loader(data,args,torch.device(kind))
            self.assertIs(loader.call_args.kwargs["pin_memory"],pinned)
            self.assertFalse(loader.call_args.kwargs["batch_sampler"].shuffle)
            self.assertIs(loader.call_args.kwargs["collate_fn"],collate_trajectories)

    def test_cli_end_to_end_v4_and_v5_full_json_histories(self):
        import diagnose_policy4
        import model5 as V5
        from model4.structure import structure_digest
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);data=root/'data';data.mkdir();(data/'val').mkdir()
            forest=dict(version=1,dsl_source_sha256='synthetic',tags=['Unknown'],
                        nodes=[dict(tag=0,bytes=[],children=[])],roots={})
            (data/'card_structure.json').write_text(json.dumps(forest))
            (data/'meta.json').write_text(json.dumps(dict(encoder_version=7,trajectory_version=2,
                structure_sha256=structure_digest(forest),cards=[0],train_shards=[],val_shards=['game.jsonl.gz'],
                source_namespace={'conversion':'synthetic'})))
            np.zeros((1,M.CARD_DSL_WIDTH),np.float32).tofile(data/'card_dsl.bin')
            with gzip.open(data/'val/game.jsonl.gz','wt') as stream:
                for cursor in range(3):
                    decision=cursor==2;count=2 if decision else 0
                    feats=[[0.]*M.CAND_FEAT_WIDTH for _ in range(count)]
                    if decision:feats[0][0]=1
                    stream.write(json.dumps(dict(game_id=1,seat=0,step_cursor=cursor,turn=1,head=0,pending_kind=0,
                        is_decision=decision,is_terminal=decision,loss_weight=float(decision),outcome=.5,
                        state=dict(entity_feat=[],entity_card_ids=[],entity_rule_refs=[],
                            seat_feat=np.zeros((8,M.SEAT_FEAT_WIDTH)).tolist(),global_feat=[0.]*M.GLOBAL_FEAT_WIDTH,edges=[]),
                        cand_feat=feats,cand_entity_idx=[-1]*count,cand_rel_seat_idx=[-1]*count,
                        cand_rule_refs=[[] for _ in range(count)],pairs=[],target=dict(single_idx=1 if decision else 0,
                            multi_mask=[],free_mask=[],scalar_norm=0.),provenance=dict(source='House')))+'\n')
            for revision in (None,2):
                cfg=M.Config4(d_model=16,n_heads=2,n_layers=1,d_ff=32,structure_width=16) if revision is None else V5.Config5(
                    d_model=16,n_heads=2,n_layers=1,d_ff=32,structure_width=16,relational_revision=revision)
                net=M.PlayerNet4(cfg) if revision is None else V5.PlayerNet5(cfg)
                net.attach_structure(forest)
                model=root/f'model-{revision}';model.mkdir()
                saved=dict(config=cfg.to_dict(),state_dict=net.state_dict(),structure=forest,
                           model_version=3 if revision is None else 5,encoder_version=7,trajectory_version=2,sampling_version=2)
                if revision is not None:saved['architecture']=V5.architecture_for_config(cfg)
                torch.save(saved,model/'net.pt')
                target=root/f'report-{revision}.json'
                with patch('sys.argv',['diagnose_policy4','--model',str(model),'--data',str(data),'--out',str(target),
                         '--device','cpu','--batch-size','1','--unroll','2','--board-chunk','2','--decision-chunk','2']), redirect_stdout(io.StringIO()):
                    diagnose_policy4.main()
                report=json.loads(target.read_text())
                self.assertEqual(report['diagnostics']['global_metrics']['sums']['frames'],3)
                self.assertEqual(report['diagnostics']['global_metrics']['sums']['single_choices'],1)
                self.assertEqual(report['diagnostics']['by_format']['unknown']['sums']['frames'],3)
                self.assertEqual(len(report['diagnostics']['per_game']),1)

    def test_relative_unresolved_source_namespaces_cannot_alias_different_conversions(self):
        with tempfile.TemporaryDirectory() as tmp:
            values=[]
            for name in ('one','two'):
                root=Path(tmp)/name;root.mkdir();(root/'meta.json').write_text('{}')
                data=SimpleNamespace(data_dir=root,meta={'source_dataset':'ambiguous-relative'},
                    diagnostic_split='val',structure=SimpleNamespace(digest='catalog'))
                values.append(Provenance(data).namespace_id)
            self.assertNotEqual(*values)

    def test_format_requires_metadata_and_completed_job_not_filename_inference(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);run=root/'runs'/'misleading-duel-name';run.mkdir(parents=True)
            (run/'run.json').write_text(json.dumps(dict(seats=4,teams=[1,2,1,2])))
            entry=dict(game_id=1,source_shard=str(run/'records/file.gz'),source_offset=0,source_len=1,
                       split='val',structure_sha256='catalog',replay_hashes_checked=True)
            dataset=SimpleNamespace(data_dir=root,diagnostic_split='val',structure=SimpleNamespace(digest='catalog'),
                meta=dict(source_namespace={'dataset':'conversion-one'},package_sources=[entry]))
            self.assertEqual(Provenance(dataset).formats[1],'unknown')
            job=root/'jobs'/run.name;job.mkdir(parents=True)
            (job/'job.json').write_text(json.dumps(dict(status='complete',command=['selfplay','--out',str(run),'--shapes','commander','--seats','4'])))
            value=Provenance(dataset)
            self.assertEqual(value.formats[1],'commander/seats=4/teams=1,2,1,2')
            dataset.meta['package_sources'].append({**entry,'source_offset':99})
            with self.assertRaisesRegex(ValueError,'aliases'):
                Provenance(dataset)


if __name__=='__main__':
    unittest.main()
