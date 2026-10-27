"""Real learned-branch V5 calibration preserves actor, memory and checkpoint identity."""
import copy
import tempfile
import unittest
from pathlib import Path
import torch
import model5 as V5
from export_onnx4 import make_dummy_inputs
from test_value_calibration4 import checkpoint, signal_cache, TinyDataset
from value_calibration4.checkpoint import load_source, merged_checkpoint, publish, file_digest
from value_calibration4.extract import extract
from value_calibration4.cache import FeatureCache
from value_calibration4.fit import fit


class V5Calibration(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(417)

    def saved(self, revision):
        net = V5.PlayerNet5(V5.Config5(d_model=16, n_heads=4, n_layers=1, d_ff=32,
                                      relational_width=8, relational_revision=revision)).eval()
        torch.nn.init.normal_(net.relational.output.weight, std=.1)
        saved = checkpoint(net)
        saved.update(model_version=5, model_family='v5', architecture=V5.architecture_for_config(net.cfg))
        return net, saved

    def test_real_v5_fit_reload_and_policy_are_preserved(self):
        for revision in (1, 2):
            with self.subTest(revision=revision), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);net,saved=self.saved(revision)
                torch.save(saved, root/'source.pt');digest=file_digest(root/'source.pt')
                _, loaded_saved, loaded=load_source(root/'source.pt')
                for split,seed in [('train',31),('val',32)]:
                    (root/split).mkdir();signal_cache(root/split,16,seed)
                train=FeatureCache(root/'train',16);val=FeatureCache(root/'val',16)
                head,report=fit(loaded.seat_value_head,train,val,'cpu',epochs=6,lr=.02,batch_size=20)
                self.assertGreater(report['selected_epoch'],0)
                merged=merged_checkpoint(loaded_saved,head,{'source_sha256':digest})
                self.assertEqual(merged['architecture'],saved['architecture'])
                self.assertEqual(merged['config'],saved['config'])
                for name,tensor in saved['state_dict'].items():
                    if not name.startswith('seat_value_head.'):
                        self.assertTrue(torch.equal(tensor,merged['state_dict'][name]),name)
                publish(root/'calibrated',merged,report)
                _,reloaded_saved,reloaded=load_source(root/'calibrated')
                self.assertEqual(reloaded_saved['model_version'],5)
                self.assertEqual(file_digest(root/'source.pt'),digest)
                inputs=list(make_dummy_inputs(16,E=4,M_edges=4,C=4,P=2,K=0))
                inputs[5][0]=torch.tensor([[9,0],[10,1],[9,10],[10,9]])
                inputs[7].fill_(True)
                with torch.no_grad():
                    # K0 updates once; K2 policy keeps memory and uses learned graph branch.
                    for k in (0,2):
                        inputs[-1].fill_(k)
                        before=loaded(*inputs);after=reloaded(*inputs)
                        for a,b in zip(before[:4],after[:4]):
                            torch.testing.assert_close(a,b,rtol=0,atol=0)
                    expected=copy.deepcopy(loaded)
                    expected.seat_value_head.load_state_dict(head.state_dict(),strict=True)
                    for a,b in zip(expected(*inputs),reloaded(*inputs)):
                        torch.testing.assert_close(a,b,rtol=0,atol=0)

    def test_v4_load_remains_supported(self):
        import model4 as V4
        net=V4.PlayerNet4(V4.Config4(d_model=16,n_heads=4,n_layers=1,d_ff=32))
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'net.pt';saved=checkpoint(net);torch.save(saved,path)
            _,original,loaded=load_source(path)
            self.assertIsInstance(loaded,V4.PlayerNet4)
            self.assertEqual(original['model_version'],3)
            for a,b in zip(net.parameters(),loaded.parameters()):
                self.assertTrue(torch.equal(a,b))

    def test_house_and_greedy_histories_still_fail_closed(self):
        for house in (True,False):
            class Ineligible(TinyDataset):
                def __getitem__(self,index):
                    row=super().__getitem__(index)
                    row['has_logprob'][:]=False
                    row['temperature'][:]=0
                    if house:row['source_kind'][:]=1
                    return row
            net,_=self.saved(2)
            with tempfile.TemporaryDirectory() as temporary:
                with self.assertRaisesRegex(ValueError,'stochastic neural trajectory'):
                    extract(net,Ineligible(16),temporary,stride=1,batch_size=2,board_chunk=2)

    def test_invalid_identity_and_old_v1_config(self):
        with tempfile.TemporaryDirectory() as temporary:
            p=Path(temporary)/'net.pt';_,saved=self.saved(2)
            for updates in ({'architecture':None},{'architecture':V5.ARCHITECTURES[1]},
                            {'model_family':'v4'},{'model_version':3},{'encoder_version':6},
                            {'structure_sha256':'0'*64}):
                torch.save({**saved,**updates},p)
                with self.assertRaises((ValueError,TypeError,RuntimeError)):load_source(p)
            bad=copy.deepcopy(saved);bad['config'].pop('relational_revision');torch.save(bad,p)
            with self.assertRaises(ValueError):load_source(p)
            _,old=self.saved(1);old['config'].pop('relational_revision');torch.save(old,p)
            _,original,net=load_source(p)
            self.assertEqual(net.cfg.relational_revision,1)
            self.assertNotIn('relational_revision',merged_checkpoint(original,net.seat_value_head,{})['config'])

    def test_v5_extraction_passive_observations_then_decision_readout(self):
        for revision in (1,2):
            net,_=self.saved(revision);dataset=TinyDataset(16)
            with tempfile.TemporaryDirectory() as temporary:
                meta=extract(net,dataset,temporary,stride=1,batch_size=2,board_chunk=2)
                self.assertEqual(meta['scanned_frames'],6)
                import numpy as np
                from value_calibration4.cache import record_dtype
                records=np.fromfile(Path(temporary)/'records.bin',dtype=record_dtype(16))
                learner=records[records['trajectory']==0]
                h=torch.zeros(1,9,16);captured=[]
                hook=net.seat_value_head.register_forward_pre_hook(lambda m,a:captured.append(a[0].detach().clone()))
                with torch.no_grad():
                    for cursor in range(3):
                        args=[torch.from_numpy(dataset.frame[name][None].copy()) for name in V5.INPUT_NAMES[:-2]]
                        k=torch.tensor([2 if cursor==2 else 0])
                        out=net(*args,h,k)
                        if cursor==2:
                            self.assertTrue(torch.equal(out[0],h))
                        h=out[0]
                hook.remove()
                torch.testing.assert_close(torch.from_numpy(learner[-1]['features'].copy()),captured[-1][0],rtol=2e-6,atol=2e-6)

if __name__=='__main__':unittest.main()
