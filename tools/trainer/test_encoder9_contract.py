"""PyTorch E9 input, explicit warmstart and CPU ONNX contracts."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
import onnxruntime as ort
import torch
import model4 as M4
import model5 as M5
from encoder_contract import validate_encoder, check_initial_encoder
from export_onnx4 import make_dummy_inputs, load_export_checkpoint
from policy_training4.optimization import select_policy_parameters
from test_encoder8_contract import FOREST


class Encoder9Contract(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(909)

    def model(self, width):
        net = M5.PlayerNet5(M5.Config5(d_model=16,n_heads=4,n_layers=1,d_ff=32,
                                      relational_revision=2,global_feat_width=width)).eval()
        net.attach_structure(FOREST)
        return net

    def test_native_encoder_and_explicit_transfer_admission(self):
        for source in (7,8):
            with self.assertRaises(ValueError): check_initial_encoder(source,9)
            proof = check_initial_encoder(source,9,explicit_to9=True)
            self.assertFalse(proof['behavior_equivalence_claimed'])
        validate_encoder(9,global_feat_width=82)
        for fields in ({'global_feat_width':80}, {'global_feat_width':None},
                       {'global_feat_width':82,'model_version':3},
                       {'global_feat_width':82,'relational_revision':1},
                       {'global_feat_width':82,'candidate_context_revision':3}):
            with self.subTest(fields=fields), self.assertRaises(ValueError):
                validate_encoder(9,**fields)
        with self.assertRaises(ValueError): validate_encoder(8,global_feat_width=82)
        self.assertEqual(M4.Config4().global_feat_width,80)

    def test_warmstart_exact_columns_and_new_inputs_need_trunk_learning(self):
        old, new = self.model(80), self.model(82)
        state = M5.warmstart_encoder9(old.state_dict(),new)
        new.load_state_dict(state,strict=True)
        torch.testing.assert_close(new.global_proj[0].weight[:,:80],old.global_proj[0].weight,rtol=0,atol=0)
        self.assertEqual(int(torch.count_nonzero(new.global_proj[0].weight[:,80:])),0)
        inputs = list(make_dummy_inputs(16,E=2,M_edges=1,C=2,P=1,K=2,global_feat_width=82))
        inputs[4][:,80:] = torch.tensor([[1.,3.]])
        baseline = new(*inputs)[1]
        alternate = list(inputs); alternate[4] = inputs[4].clone(); alternate[4][:,81] = 9.
        torch.testing.assert_close(baseline,new(*alternate)[1],rtol=0,atol=0)
        baseline[0,0].backward()
        self.assertGreater(float(new.global_proj[0].weight.grad[:,80:].abs().sum()),0)
        select_policy_parameters(new)
        self.assertFalse(new.global_proj[0].weight.requires_grad)
        new.zero_grad(set_to_none=True)
        new(*inputs)[1][0,0].backward()
        self.assertIsNone(new.global_proj[0].weight.grad)

    def test_malformed_export_rejected_before_model_construction(self):
        net = self.model(82)
        saved = dict(model_version=5,architecture=M5.architecture_for_config(net.cfg),
                     trajectory_version=2,sampling_version=2,encoder_version=9,
                     config=net.cfg.to_dict(),structure=FOREST,state_dict=net.state_dict())
        for width in (None,80):
            bad = copy.deepcopy(saved)
            if width is None: bad['config'].pop('global_feat_width')
            else: bad['config']['global_feat_width'] = width
            with tempfile.TemporaryDirectory() as temporary:
                path = Path(temporary)/'net.pt';torch.save(bad,path)
                with patch.object(M5,'PlayerNet5') as constructor, self.assertRaises(ValueError):
                    load_export_checkpoint(path)
                constructor.assert_not_called()

    def test_full_ppo_resume_checks_e9_before_any_state_mutation(self):
        from recurrent_ppo4.checkpoint import restore_training
        net = self.model(82)
        optimizer = torch.optim.AdamW(net.parameters())
        contract = {'version':2,'behavior_probability_reference':'recorded_executed_action'}
        saved = dict(encoder_version=9,model_version=5,config=net.cfg.to_dict(),full_ppo_contract=contract)
        saved['config']['global_feat_width']=80
        with patch.object(net,'load_state_dict') as weights, patch.object(optimizer,'load_state_dict') as adam, self.assertRaises(ValueError):
            restore_training(saved,net,optimizer,contract,'cpu')
        weights.assert_not_called();adam.assert_not_called()

    def test_cpu_onnx_all_depths_accept82_and_match_torch(self):
        import json
        from export_onnx4 import main as export_main
        net = self.model(82)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            torch.save(dict(model_version=5,encoder_version=9,architecture=M5.architecture_for_config(net.cfg),
                            config=net.cfg.to_dict(),structure=FOREST,state_dict=net.state_dict(),
                            trajectory_version=2,sampling_version=2),root/'net.pt')
            with patch('sys.argv',['export_onnx4','--model',str(root),'--entities','2','--check','1']):
                export_main()
            metadata = json.loads((root/'model.onnx.json').read_text())
            self.assertEqual(metadata['global_feat_width'],82)
            self.assertEqual(metadata['config']['global_feat_width'],82)
            self.assertEqual(set(metadata['graphs']),{'0','1','2','4','8'})
            options = ort.SessionOptions(); options.intra_op_num_threads=1;options.inter_op_num_threads=1
            for k in (0,1,2,4,8):
                session = ort.InferenceSession(str(root/metadata['graphs'][str(k)]),options,providers=['CPUExecutionProvider'])
                self.assertEqual(next(item.shape[1] for item in session.get_inputs() if item.name=='global_feat'),82)
                inputs = make_dummy_inputs(16,E=2,M_edges=1,C=3,P=1,K=k,global_feat_width=82)
                with torch.no_grad(): expected = net(*inputs)
                feed = {name:value.numpy() for name,value in zip(M4.INPUT_NAMES,inputs)}
                actual = session.run(None,{item.name:feed[item.name] for item in session.get_inputs()})
                for left,right in zip(expected,actual):
                    np.testing.assert_allclose(left.numpy(),right,rtol=2e-4,atol=2e-5)

    def test_real_supervised_weights_only_warmstart_and_strict_resume(self):
        import contextlib, gzip, io, json
        from test_recurrent4 import write_dataset
        from training4 import driver
        main = driver.main
        original = driver.D4.TrajectoryDataset4
        def dataset(*args,**kwargs):
            result = original(*args,**kwargs)
            self.addCleanup(result.store.close)
            return result
        patcher = patch.object(driver.D4,'TrajectoryDataset4',side_effect=dataset)
        patcher.start();self.addCleanup(patcher.stop)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); data = root/'data'; data.mkdir()
            write_dataset(data,[2],candidates=2)
            path = data/'train/steps.jsonl.gz'
            with gzip.open(path,'rt') as stream: rows = [json.loads(line) for line in stream]
            for row in rows: row['state']['global_feat'] += [1.,3.]
            with gzip.open(path,'wt') as stream:
                for row in rows: stream.write(json.dumps(row)+'\n')
            (data/'val').mkdir();(data/'val/steps.jsonl.gz').write_bytes(path.read_bytes())
            meta = json.loads((data/'meta.json').read_text())
            meta.update(encoder_version=9,global_feat_width=82,val_shards=['steps.jsonl.gz'])
            (data/'meta.json').write_text(json.dumps(meta))
            old = self.model(80)
            forest = json.loads((data/'card_structure.json').read_text());old.attach_structure(forest)
            source = root/'old.pt'
            torch.save(dict(model_version=5,encoder_version=8,architecture=M5.architecture_for_config(old.cfg),
                            config=old.cfg.to_dict(),structure=forest,state_dict=old.state_dict(),
                            trajectory_version=2,sampling_version=2,optimizer='must not load',
                            h_state=torch.tensor(float('nan'))),source)
            output = root/'out'
            args = ['train5','--data',str(data),'--out',str(output),'--architecture','v5',
                    '--encoder-version','9','--d-model','16','--layers','1','--heads','4',
                    '--d-ff','32','--batch-size','1','--unroll','2','--num-workers','0',
                    '--max-steps','1','--epochs','1']
            with patch('sys.argv',args+['--init',str(source),'--encoder-warmstart-to9']),contextlib.redirect_stdout(io.StringIO()):
                main()
            saved = torch.load(output/'last.pt',weights_only=False)
            self.assertEqual(saved['encoder_version'],9)
            self.assertEqual(saved['config']['global_feat_width'],82)
            self.assertEqual(saved['encoder_warmstart']['source_encoder_version'],8)
            self.assertGreater(int(torch.count_nonzero(saved['state_dict']['global_proj.0.weight'][:,80:])),0)
            with patch('sys.argv',args+['--resume',str(output/'last.pt')]),contextlib.redirect_stdout(io.StringIO()):
                main()
            bad = copy.deepcopy(saved);bad['config']['global_feat_width']=80
            torch.save(bad,root/'bad.pt')
            with patch('sys.argv',args+['--resume',str(root/'bad.pt')]),contextlib.redirect_stdout(io.StringIO()),self.assertRaisesRegex(ValueError,'configuration'):
                main()

    def test_native_fireball_fixture_tensor_contract(self):
        import json, os
        path = os.environ.get('BAYLEE_E9_FIXTURE')
        if not path: self.skipTest('native E9 fixture emitted by Rust test required')
        fixture = json.loads(Path(path).read_text())
        self.assertEqual(fixture['global_feat_width'],82)
        prefix = fixture['cases'][0]['state']['global_feat'][:80]
        for case in fixture['cases']:
            values = torch.tensor(case['state']['global_feat'],dtype=torch.float32)
            self.assertEqual(tuple(values.shape),(82,))
            self.assertEqual(values[:80].tolist(),prefix)
            x = case['announced_x']
            expected = torch.tensor([float(x is not None),float(x or 0)],dtype=torch.float32)
            torch.testing.assert_close(values[80:],expected,rtol=0,atol=0)
        left = torch.tensor(fixture['actual_target0']['global_feat'])
        right = torch.tensor(fixture['actual_target1']['global_feat'])
        torch.testing.assert_close(left[:80],right[:80],rtol=0,atol=0)
        torch.testing.assert_close(left[80:],torch.tensor([1.,0.]),rtol=0,atol=0)
        torch.testing.assert_close(right[80:],torch.tensor([1.,1.]),rtol=0,atol=0)
if __name__=='__main__': unittest.main()
