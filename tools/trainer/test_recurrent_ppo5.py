"""Full v5 PPO: current-H replay, genuine trunk gradients and transactions."""
import copy
import unittest
import numpy as np
import torch
import model5 as M
from test_training_execution4 import batch_fixture
from test_policy_training4 import assert_state_equal
from recurrent_ppo4.forward import advance, episode_prefixes, segment_outputs, reconstruct_hidden
from recurrent_ppo4.optimization import ppo_loss, transactional_update, collection_divergence


class EpisodeDataset:
    """Two complete learner histories including passive and padded frames."""
    def __init__(self, net):
        host = batch_fixture(b=2, u=3)
        host['step_valid'].fill_(True)
        host['head'].zero_()
        host['single_idx'].zero_()
        host['is_decision'].fill_(True)
        host['is_decision'][:, 0] = False
        host['cand_mask'].fill_(True)
        host['pair_mask'].fill_(True)
        host['thinking_steps'] = host['is_decision'].long() * 2
        host['temperature'] = torch.full((2, 3), .7)
        host['is_new_traj'] = torch.ones(2, dtype=torch.bool)
        host['is_terminal'] = torch.zeros(2, 3, dtype=torch.bool)
        host['actor_mask'] = host['is_decision'].clone()
        host['advantages'] = torch.tensor([[0., 1., -.3], [0., -.5, .7]])
        host['returns'] = torch.tensor([[1., 1., 1.], [0., 0., 0.]])
        host['behavior_lp'] = torch.zeros(2, 3)
        self.rows = []
        for game in range(2):
            for segment in range(2):
                row = {name: value[game].numpy().copy() for name, value in host.items()}
                row['is_new_traj'] = np.array(segment == 0)
                row['segment_index'] = np.array(len(self.rows))
                row['is_terminal'][-1] = segment == 1
                row['global_feat'] += game * .1 + segment * .03
                self.rows.append(row)
        self.trajectories = [[0, 1], [2, 3]]
        from dataset4 import collate_trajectories
        from policy_math4 import action_logprob_and_kl
        with torch.no_grad():
            for trajectory in self.trajectories:
                h = torch.zeros(1, 9, 32)
                for idx in trajectory:
                    batch = collate_trajectories([self[idx]])
                    outputs, h = advance(net, batch, h, torch.device('cpu'))
                    for t, out in enumerate(outputs):
                        lp, _ = action_logprob_and_kl(*out[1:4], *out[1:4], batch, t)
                        self.rows[idx]['behavior_lp'][t] = float(lp[0])
    def __getitem__(self, index): return self.rows[index]
    def __len__(self): return len(self.rows)


class RecurrentPPO5(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(814)
        self.device = torch.device('cpu')
        self.net = M.PlayerNet5(M.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,
                                         relational_revision=2,candidate_context_revision=2)).eval()
        self.dataset = EpisodeDataset(self.net)
        self.prefixes = episode_prefixes(self.dataset)

    def test_prefix_recomputed_from_current_weights_and_not_behavior_h(self):
        baseline = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        self.dataset.rows[1]['behavior_h'] = np.full((9,32),999,dtype=np.float32)
        repeated = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        torch.testing.assert_close(repeated,baseline,atol=0,rtol=0)
        with torch.no_grad(): self.net.global_proj[0].weight.mul_(1.7)
        current = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        self.assertGreater(float((current-baseline).abs().max()),0)
        self.assertFalse(current.requires_grad)

    def test_full_unroll_has_graph_trunk_memory_and_thinking_gradients(self):
        output = segment_outputs(self.net,self.dataset,[1,3],self.prefixes,self.device)
        ppo_loss(output).backward()
        groups = ('relational','trunk_layers','recurrent_cell','thinking_block')
        for prefix in groups:
            total=sum(float(p.grad.abs().sum()) for name,p in self.net.named_parameters()
                      if prefix in name and p.grad is not None)
            self.assertGreater(total,0,prefix)

    def test_whole_collection_gate_restores_optimizer_and_weights(self):
        optimizer = torch.optim.AdamW(self.net.parameters(),lr=.001)
        initial = collection_divergence(self.net,self.dataset,self.prefixes,self.device,2)
        self.assertLess(initial,1e-12)
        accepted=transactional_update(self.net,optimizer,self.dataset,[1],self.prefixes,self.device,target_kl=1)
        self.assertTrue(accepted['accepted'])
        before=copy.deepcopy(self.net.state_dict());adam=copy.deepcopy(optimizer.state_dict())
        optimizer.param_groups[0]['lr']=1
        adam=copy.deepcopy(optimizer.state_dict())
        rejected=transactional_update(self.net,optimizer,self.dataset,[3],self.prefixes,self.device,target_kl=1e-12)
        self.assertFalse(rejected['accepted'])
        assert_state_equal(self,self.net.state_dict(),before)
        assert_state_equal(self,optimizer.state_dict(),adam)

    def test_strict_identity_roundtrip_encoder7_and8_candidate1_and2(self):
        from recurrent_ppo4.checkpoint import load_policy
        from test_structure4 import small_forest
        for encoder in (7,8):
            for revision in (1,2):
                cfg=M.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,relational_revision=2,
                              candidate_context_revision=revision)
                model=M.PlayerNet5(cfg)
                model.attach_structure(small_forest())
                saved=dict(config=cfg.to_dict(),model_version=5,model_family='v5',
                           architecture=M.architecture_for_config(cfg),encoder_version=encoder,
                           trajectory_version=M.TRAJECTORY_VERSION,sampling_version=M.SAMPLING_VERSION,
                           structure=small_forest(),state_dict=model.state_dict())
                loaded=load_policy(saved,self.device)
                assert_state_equal(self,model.state_dict(),loaded.state_dict())
                bad={**saved,'architecture':M.ARCHITECTURES[1]}
                with self.assertRaises(ValueError):load_policy(bad,self.device)
                with self.assertRaises(ValueError):load_policy({**saved,'encoder_version':9},self.device)

    def test_next_full_model_update_exact_after_resume(self):
        import io
        from recurrent_ppo4.checkpoint import training_payload,restore_training
        optimizer=torch.optim.AdamW(self.net.parameters(),lr=.0001)
        first=transactional_update(self.net,optimizer,self.dataset,[1],self.prefixes,self.device,target_kl=1)
        self.assertTrue(first['accepted'])
        contract={'version':2,'behavior_probability_reference':'recorded_executed_action',
                  'dataset':'fixed_fixture','source':'immutable_fixture'}
        savedsource=dict(model_version=5,config=self.net.cfg.to_dict(),encoder_version=7,
                         trajectory_version=M.TRAJECTORY_VERSION,sampling_version=M.SAMPLING_VERSION,
                         architecture=M.architecture_for_config(self.net.cfg),structure={})
        stream=io.BytesIO()
        torch.save(training_payload(self.net,optimizer,savedsource,contract,{'epoch':1,'next_batch':1}),stream)
        expected=transactional_update(self.net,optimizer,self.dataset,[3],self.prefixes,self.device,target_kl=1)
        self.assertTrue(expected['accepted'])
        weights=copy.deepcopy(self.net.state_dict());moments=copy.deepcopy(optimizer.state_dict())
        fresh=M.PlayerNet5(self.net.cfg).eval()
        freshopt=torch.optim.AdamW(fresh.parameters(),lr=.0001)
        stream.seek(0);saved=torch.load(stream,weights_only=False)
        before=copy.deepcopy(fresh.state_dict())
        from unittest.mock import patch
        for invalid in ({**contract,'version':1},
                        {k:v for k,v in contract.items() if k!='behavior_probability_reference'},
                        {**contract,'behavior_probability_reference':'recomputed_replay'}):
            with self.subTest(contract=invalid), \
                 patch.object(fresh,'load_state_dict') as load_weights, \
                 patch.object(freshopt,'load_state_dict') as load_optimizer, \
                 patch('recurrent_ppo4.checkpoint.restore_rng') as load_rng:
                with self.assertRaisesRegex(ValueError,'probability reference'):
                    restore_training({**saved,'full_ppo_contract':invalid},fresh,freshopt,contract,self.device)
                load_weights.assert_not_called()
                load_optimizer.assert_not_called()
                load_rng.assert_not_called()
        with self.assertRaises(ValueError):restore_training(saved,fresh,freshopt,{'changed':True},self.device)
        assert_state_equal(self,fresh.state_dict(),before)
        progress=restore_training(saved,fresh,freshopt,contract,self.device)
        self.assertEqual(progress,{'epoch':1,'next_batch':1})
        actual=transactional_update(fresh,freshopt,self.dataset,[3],self.prefixes,self.device,target_kl=1)
        self.assertEqual(expected,actual)
        assert_state_equal(self,fresh.state_dict(),weights)
        assert_state_equal(self,freshopt.state_dict(),moments)

    def test_legacy_resume_driver_rejects_before_model_optimizer_or_rng_setup(self):
        from unittest.mock import patch
        from recurrent_ppo4.driver import main
        args=['train_rl4','--init','unused-source','--rollout-data','unused-data',
              '--admission','unused-admission','--out','unused-output','--resume','legacy.pt']
        for contract in ({'version':1}, {'version':2},
                         {'version':2,'behavior_probability_reference':'recomputed_replay'}):
            with self.subTest(contract=contract), patch('sys.argv',args), \
                 patch('torch.load',return_value={'full_ppo_contract':contract}), \
                 patch('recurrent_ppo4.checkpoint.load_policy') as load_model, \
                 patch('torch.optim.AdamW') as optimizer, \
                 patch('torch.manual_seed') as seed:
                with self.assertRaisesRegex(ValueError,'probability reference'):
                    main()
                load_model.assert_not_called()
                optimizer.assert_not_called()
                seed.assert_not_called()

    def test_complete_disk_episode_driver_resume_matches_uninterrupted_and_excludes_opponent(self):
        import gzip,json,tempfile
        from pathlib import Path
        from unittest.mock import patch
        from contextlib import redirect_stdout
        import io
        import dataset4 as D
        from test_recurrent4 import write_dataset
        from test_structure4 import small_forest
        from value_calibration4.checkpoint import file_digest
        from policy_math4 import action_logprob_and_kl
        from recurrent_ppo4.driver import main
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);data=root/'data';data.mkdir()
            write_dataset(data,[6,5],candidates=3)
            forest=small_forest()
            self.net.attach_structure(forest)
            with gzip.open(data/'train/steps.jsonl.gz','rt') as stream:
                rows=[json.loads(line) for line in stream]
            for row in rows:
                cursor=row['step_cursor'];decision=cursor%2==1
                if row['game_id']==1:row.update(game_id=0,seat=1)
                row['is_decision']=decision
                row['outcome']=float(row['seat']==0)
                row['state'].update(entity_feat=torch.randn(2,M.ENTITY_FEAT_WIDTH).tolist(),
                                    entity_card_ids=[M.UNKNOWN_CARD]*2,entity_rule_refs=[[],[]],
                                    edges=[[9,10,0],[10,9,1]],
                                    global_feat=torch.randn(M.GLOBAL_FEAT_WIDTH).tolist())
                row.update(cand_feat=torch.randn(3,M.CAND_FEAT_WIDTH).tolist(),cand_entity_idx=[0,1,-1],
                           cand_rel_seat_idx=[-1]*3,cand_rule_refs=[[],[],[]],
                           target={'single_idx':cursor%3,'scalar_norm':0.},
                           provenance={'source':'Net','has_logprob':decision,'behavior_logprob':0.,
                                       'temperature':.7,'thinking_steps':2 if decision else 0})
            def write_rows():
                with gzip.open(data/'train/steps.jsonl.gz','wt') as stream:
                    for row in rows:stream.write(json.dumps(row)+'\n')
            write_rows()
            meta=json.loads((data/'meta.json').read_text());meta['policy_provenance_version']=2
            (data/'meta.json').write_text(json.dumps(meta))
            dataset=D.TrajectoryDataset4(data,unroll_len=3)
            bykey={(row['game_id'],row['seat'],row['step_cursor']):row for row in rows}
            self.net.eval()
            with torch.no_grad():
                self.net.clear_structure();self.net.prepare_structure()
                for trajectory in dataset.trajectories:
                    h=torch.zeros(1,9,32)
                    for index in trajectory:
                        host=D.collate_trajectories([dataset[index]])
                        outputs,h=advance(self.net,host,h,self.device)
                        for t,record in enumerate(dataset.segments[index]):
                            out=outputs[t]
                            lp,_=action_logprob_and_kl(*out[1:4],*out[1:4],host,t)
                            key=dataset.store.index[int(record)]
                            bykey[(int(key['game']),int(key['seat']),int(key['cursor']))]['provenance']['behavior_logprob']=float(lp[0])
            self.net.clear_structure();dataset.store.close();write_rows()
            dataset=D.TrajectoryDataset4(data,unroll_len=3)
            self.addCleanup(dataset.store.close)
            source=root/'source.pt'
            torch.save(dict(config=self.net.cfg.to_dict(),model_version=5,model_family='v5',
                            architecture=M.architecture_for_config(self.net.cfg),encoder_version=7,
                            trajectory_version=M.TRAJECTORY_VERSION,sampling_version=M.SAMPLING_VERSION,
                            full_ppo_contract={'version':1},
                            structure=forest,state_dict=self.net.state_dict()),source)
            admission=root/'admission.json'
            # Fixture receipt exercises the PyTorch consumer. Rust issuance is
            # tested separately; this fixture is never a real collection claim.
            admission.write_text(json.dumps(dict(version=1,admitted=True,
                source_checkpoint_sha256=file_digest(source),dataset_fingerprint=dataset.fingerprint,
                collection_contract=dict(mode='raw',actor_role='learner',precision='float32',thinking_steps=2,temperature=.7),
                admitted_learner_histories=[dict(game_id=0,seat=0)],completed_games=1,learner_histories=1,decisions=3)))
            import os
            native_test=os.environ.get('BAYLEE_NATIVE_ADMISSION_TEST')=='1'
            if native_test:
                import subprocess
                executable=os.environ['BAYLEE_TRAIN_OPS']
                names=['meta.json','card_structure.json','card_dsl.bin','card_dsl_back.bin','token_dsl.bin','train/steps.jsonl.gz']
                collection=root/'collection.json'
                source_sha=file_digest(source)
                collection.write_text(json.dumps(dict(version=1,purpose='on_policy_training',source_checkpoint_sha256=source_sha,
                    precision='float32',mode='raw',policy_provenance_version=2,sampling_version=M.SAMPLING_VERSION,
                    dataset_files={name:file_digest(data/name) for name in names},
                    games=[dict(game_id=0,source_run_sha256='2'*64,record_sha256='1'*64,source_game='fixture-table',complete=True,capped=False,fallback_answers=0,refused_answers=0,unencoded_decisions=0,
                                seats=[dict(seat=0,role='learner',checkpoint_sha256=source_sha,thinking_steps=2,temperature=.7),
                                       dict(seat=1,role='self_play',checkpoint_sha256=source_sha,thinking_steps=2,temperature=.7)])])))
                subprocess.run([executable,'admit-policy','--data',str(data),'--source-checkpoint',str(source),
                                '--collection-manifest',str(collection),'--out',str(admission)],check=True,capture_output=True)
            # The head-only worker consumes the same role receipt and genuine
            # on-disk recurrent histories. Three decisions deliberately produce
            # no 128-decision update; audit/selection still run in full.
            from policy_training4.driver import main as head_main
            head_args=['train_policy4','--model',str(source),'--rollout-data',str(data),
                       '--admission',str(admission),'--out',str(root/'head'),
                       '--batch-size','1','--unroll','3','--epochs','1','--device','cpu']
            with patch('sys.argv',head_args),redirect_stdout(io.StringIO()):
                if native_test:
                    head_main()
                else:
                    with patch('recurrent_ppo4.checkpoint.native_verify_admission') as head_native:
                        head_main()
                        self.assertEqual(head_native.call_count,2)
            head_report=json.loads((root/'head/report.json').read_text())
            self.assertEqual(head_report['audit']['decisions_checked'],3)
            self.assertEqual(head_report['audit']['learner_trajectories'],1)
            self.assertTrue(head_report['checkpoint_is_unchanged_source'])
            base=['train_rl4' ,'--init',str(source),'--rollout-data',str(data),'--admission',str(admission),
                  '--batch-size','1','--unroll','3','--device','cpu']
            def run(output,steps,resume=False):
                args=base+['--out',str(output),'--max-steps',str(steps)]
                if resume:args+=['--resume',str(output/'last.pt')]
                if native_test:
                    with patch('sys.argv',args),redirect_stdout(io.StringIO()):main()
                else:
                    with patch('sys.argv',args),redirect_stdout(io.StringIO()), \
                         patch('recurrent_ppo4.checkpoint.native_verify_admission') as native_fixture, \
                         patch('recurrent_ppo4.checkpoint.native_source_digest',side_effect=file_digest):
                        main()
                        self.assertEqual(native_fixture.call_count,2)
                return torch.load(output/'net.pt',weights_only=False)
            partial=run(root/'resumed',1)
            self.assertEqual(partial['full_ppo_progress']['accepted'],1)
            resumed=run(root/'resumed',2,True)
            uninterrupted=run(root/'full',2)
            assert_state_equal(self,resumed['state_dict'],uninterrupted['state_dict'])
            assert_state_equal(self,resumed['optimizer'],uninterrupted['optimizer'])
            self.assertEqual(resumed['full_ppo_contract']['admission']['decisions'],3)
            self.assertEqual(resumed['full_ppo_contract']['version'],2)
            self.assertEqual(resumed['full_ppo_contract']['behavior_probability_reference'],'recorded_executed_action')
            self.assertEqual(resumed['full_ppo_progress']['attempts'],2)
            from export_onnx4 import load_export_checkpoint,export_v4_onnx,make_dummy_inputs
            import onnxruntime as ort
            exported,policy,_=load_export_checkpoint(root/'resumed/net.pt')
            self.assertEqual(exported['architecture'],M.CANDIDATE_ARCHITECTURE)
            graph=root/'updated.onnx'
            export_v4_onnx(policy,graph,thinking_steps=2,fixed_depth=True)
            options=ort.SessionOptions();options.intra_op_num_threads=1
            session=ort.InferenceSession(str(graph),sess_options=options,providers=['CPUExecutionProvider'])
            inputs=make_dummy_inputs(32,E=3,M_edges=2,C=4,P=2,K=2)
            with torch.no_grad():expected=policy(*inputs)
            actual=session.run(list(M.OUTPUT_NAMES),{name:value.numpy() for name,value in zip(M.INPUT_NAMES,inputs)})
            for reference,result in zip(expected,actual):
                np.testing.assert_allclose(reference.numpy(),result,atol=2e-5,rtol=2e-4)

    def test_checkpointing_in_eval_preserves_dropout_outputs_and_gradients(self):
        from export_onnx4 import make_dummy_inputs
        cfg=M.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,dropout=.25,
                      activation_checkpointing=True,relational_revision=2,candidate_context_revision=2)
        eager=M.PlayerNet5(cfg).eval();checkpointed=copy.deepcopy(eager)
        checkpointed.checkpoint_during_eval=True
        inputs=make_dummy_inputs(32,E=3,M_edges=2,C=4,P=2,K=2)
        left=eager(*inputs);right=checkpointed(*inputs)
        self.assertFalse(checkpointed.training)
        for a,b in zip(left,right):torch.testing.assert_close(a,b,atol=0,rtol=0)
        sum(x.square().mean() for x in left).backward()
        sum(x.square().mean() for x in right).backward()
        for (name,a),(_,b) in zip(eager.named_parameters(),checkpointed.named_parameters()):
            self.assertEqual(a.grad is None,b.grad is None,name)
            if a.grad is not None:torch.testing.assert_close(a.grad,b.grad,atol=0,rtol=0,msg=name)
