"""CPU regressions for complete learner target extraction before policy updates."""
import math
import tempfile
import json
import zlib
from pathlib import Path
from types import SimpleNamespace
import unittest
import numpy as np
import torch
from torch import nn
import model4 as M
from test_value_calibration4 import TinyDataset
from policy_training4.targets import actor_mask, prepare_policy_targets, _learner_histories


class Dataset(TinyDataset):
    def __init__(self, mismatch=False, forced=False):
        super().__init__(16)
        self.meta={'sampling_version':M.SAMPLING_VERSION, 'policy_provenance_version':2}
        index=np.zeros(6,dtype=[('game','u4'),('seat','u4'),('cursor','u8'),('flags','u4')])
        index['game']=[10]*3+[11]*3;index['cursor']=[0,1,2]*2;index['flags']=[0,0,3,0,0,2]
        self.store=SimpleNamespace(index=index,read=lambda row:{'is_decision':row==2,'head':0,
            'cand_feat':[[],[]] if not forced else [[]], 'provenance':{'source':'Net','has_logprob':True,'temperature':.6,'behavior_logprob':-math.log(3)},'target':{}})
        self.mismatch=mismatch
    def __getitem__(self,idx):
        out=super().__getitem__(idx)
        out['behavior_logprob']=np.full(2,-math.log(3)+(1 if self.mismatch else 0),np.float32)
        out['single_idx']=np.zeros(2,np.int64);out['head']=np.zeros(2,np.int64)
        out['cand_mask'][:]=True
        if hasattr(self,'ppo_cache'):
            out.update({name:value[idx] for name,value in self.ppo_cache.items()})
        return out


class Frozen(nn.Module):
    def __init__(self):
        super().__init__();self.weight=nn.Parameter(torch.ones(1),requires_grad=False)
        self.cfg=SimpleNamespace(d_model=16);self.calls=[]
    def clear_structure(self):pass
    def prepare_structure(self, active_refs=None):pass
    def forward(self,*args,**kwargs):
        h,k=args[-2:];self.calls.append((h[:,0,0].tolist(),k.tolist()))
        hn=torch.where((k==0)[:,None,None],h+1,h)
        b=len(h);cand=torch.zeros(b,args[11].shape[-1]);pair=torch.zeros(b,args[13].shape[-1])
        return hn,cand,pair,torch.zeros(b),h[:,0,0]/10


class Targets(unittest.TestCase):
    def setUp(self):torch.set_num_threads(1)
    def test_full_history_filter_mc_and_normalization(self):
        d=Dataset();net=Frozen();report=prepare_policy_targets(net,d,'cpu',batch_size=2,board_chunk=0)
        self.assertEqual(d.trajectories,[[0,1]])
        self.assertEqual(report['learner_trajectories'],1);self.assertEqual(report['frames'],3)
        self.assertEqual(report['decisions_checked'],1)
        # Before terminal decision, both passive observations have advanced memory.
        self.assertEqual(net.calls[2],([2.0],[2]))
        np.testing.assert_array_equal(d.ppo_cache['behavior_h'][1,:,0],2)
        np.testing.assert_array_equal(d.ppo_cache['returns'][0],[.5,.5])
        self.assertAlmostEqual(report['advantage_mean_before_normalization'],.3,places=6)
        self.assertEqual(float(d.ppo_cache['advantages'][1,0]),0)
        self.assertFalse(d.ppo_cache['actor_mask'][0].any())
        d._policy_targets_directory.cleanup()
    def test_archived_uncertified_or_v1_data_is_rejected_before_forward(self):
        for version in (None, 1):
            d = Dataset()
            if version is None:
                d.meta.pop('policy_provenance_version')
            else:
                d.meta['policy_provenance_version'] = version
            net = Frozen()
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, 'provenance version 2'):
                prepare_policy_targets(net, d, 'cpu')
            self.assertFalse(net.calls)
            self.assertFalse(hasattr(d, 'ppo_cache'))
            self.assertEqual(len(d.trajectories), 2)

    def test_behavior_audit_cannot_be_disabled_by_large_tolerance(self):
        for tolerance in (20., float("nan"), 0.):
            with self.assertRaisesRegex(ValueError, "tolerance"):
                prepare_policy_targets(Frozen(), Dataset(), 'cpu', tolerance=tolerance)

    def test_small_but_material_behavior_mismatch_rejected(self):
        original = Dataset.__getitem__
        class SlightMismatch(Dataset):
            def __getitem__(self, idx):
                row = original(self, idx)
                row['behavior_logprob'] += .01
                return row
        with self.assertRaisesRegex(ValueError, 'probability mismatch'):
            prepare_policy_targets(Frozen(), SlightMismatch(), 'cpu', board_chunk=0)

    def test_accepted_replay_difference_preserves_recorded_probability_in_both_learners(self):
        from policy_training4.forward import actor_outputs, batch_rows
        from policy_training4.optimization import behavior_kl
        from recurrent_ppo4.forward import episode_prefixes, segment_outputs

        class AcceptedDifference(Dataset):
            def __getitem__(self, idx):
                row = super().__getitem__(idx)
                row['behavior_logprob'] += np.float32(.0005)
                return row

        dataset = AcceptedDifference()
        net = Frozen()
        report = prepare_policy_targets(net, dataset, 'cpu', board_chunk=0)
        self.addCleanup(dataset._policy_targets_directory.cleanup)
        self.assertGreater(report['max_logprob_error'], .0004)
        self.assertLess(report['max_logprob_error'], .001)
        expected = torch.tensor([np.float32(-math.log(3)) + np.float32(.0005)])
        torch.testing.assert_close(torch.from_numpy(dataset.ppo_cache['behavior_lp'][1, :1]),
                                   expected, rtol=0, atol=0)
        head_current, head_recorded, _ = actor_outputs(net, batch_rows(dataset, [1], 'cpu'))
        recurrent = segment_outputs(net, dataset, [1], episode_prefixes(dataset),
                                    torch.device('cpu'), gradients=False)
        for current, recorded in ((head_current, head_recorded),
                                  (recurrent['logprob'], recurrent['behavior'])):
            torch.testing.assert_close(recorded, expected, rtol=0, atol=0)
            self.assertGreater(float((current-recorded).abs().max()), .0004)
            self.assertGreater(float(behavior_kl(current, recorded)), 1e-8)

    def test_mismatch_fails_before_targets_are_published(self):
        d=Dataset(mismatch=True)
        with self.assertRaisesRegex(ValueError,'probability mismatch'):
            prepare_policy_targets(Frozen(),d,'cpu',board_chunk=0)
        self.assertFalse(hasattr(d,'ppo_cache'));self.assertEqual(len(d.trajectories),2)
    def test_forced_only_is_not_a_learner_trajectory(self):
        with self.assertRaisesRegex(ValueError,'No genuine'):
            prepare_policy_targets(Frozen(),Dataset(forced=True),'cpu',board_chunk=0)
    def test_disk_header_filter_never_invents_neural_provenance(self):
        d=Dataset()
        raw=[]
        for row in range(6):
            st=d.store.read(row)
            # Last trajectory has a decision, but House metadata remains House.
            if row==5:st.update(is_decision=True,provenance={'source':'House','has_logprob':True,'temperature':.6})
            raw.append(zlib.compress(json.dumps(st).encode()))
        index=np.zeros(6,dtype=[('flags','u4'),('offset','u8'),('length','u4')])
        index['flags']=[0,0,1,0,0,1]
        offset=0
        for i,blob in enumerate(raw):
            index[i]['offset']=offset;index[i]['length']=len(blob);offset+=len(blob)
        with tempfile.TemporaryDirectory() as root:
            path=Path(root)/'data';path.write_bytes(b''.join(raw))
            d.store=SimpleNamespace(index=index,data_path=path)
            self.assertEqual(_learner_histories(d),[[0,1]])

    def test_malformed_subset_after_eligible_choice_is_rejected(self):
        d=Dataset();original=d.store.read
        # An eligible categorical row earlier in the same trajectory must not
        # hide a subsequent subset row whose free mask Dataset4 would repair.
        d.store.index[1]['flags']=1
        def read(row):
            st=original(row)
            if row==1:
                st.update(is_decision=True)
            if row==2:
                st.update(head=1,target={'multi_mask':[0,1]})
            return st
        d.store.read=read
        net=Frozen()
        with self.assertRaisesRegex(ValueError,'free_mask'):
            prepare_policy_targets(net,d,'cpu',board_chunk=0)
        self.assertFalse(net.calls)
        self.assertFalse(hasattr(d,'ppo_cache'))

    def test_nonfinite_provenance_and_nonbinary_raw_masks_are_rejected(self):
        from policy_training4.targets import _raw_choice
        base=Dataset().store.read(2)
        for bad in (float('nan'),float('inf'),0.):
            st={**base,'provenance':{**base['provenance'],'temperature':bad}}
            with self.assertRaisesRegex(ValueError,'provenance'):_raw_choice(st)
        st={**base,'provenance':{**base['provenance'],'behavior_logprob':float('nan')}}
        with self.assertRaisesRegex(ValueError,'provenance'):_raw_choice(st)
        for mask in ([0], [0,2], [0,float('nan')]):
            st={**base,'head':1,'target':{'multi_mask':[0,1],'free_mask':mask}}
            with self.assertRaisesRegex(ValueError,'free_mask'):_raw_choice(st)

    def test_low_temperature_heuristic_rows_are_not_learner_histories(self):
        from policy_training4.targets import _raw_choice
        base = Dataset().store.read(2)
        for head in (1, 2):
            for temperature in (.2, .35, .6):
                step = {**base, 'head': head, 'pairs': [[], []],
                        'target': {'multi_mask': [1, 0], 'free_mask': [1, 1]},
                        'provenance': {**base['provenance'], 'temperature': temperature}}
                self.assertEqual(_raw_choice(step), temperature > .35)
            step['provenance']['source'] = 'House'
            self.assertFalse(_raw_choice(step))

    def test_actor_masks_respect_structural_choices_and_free_masks(self):
        b={k:torch.ones(1,5,dtype=torch.bool) for k in ['step_valid','is_decision','has_logprob']}
        b.update(source_kind=torch.zeros(1,5,dtype=torch.long),temperature=torch.ones(1,5),
                 head=torch.tensor([[0,0,1,2,3]]),cand_mask=torch.tensor([[[1,0],[1,1],[1,0],[1,1],[1,1]]],dtype=torch.bool),
                 pair_mask=torch.tensor([[[1,1]]*5],dtype=torch.bool),
                 cand_free_mask=torch.tensor([[[0,0],[0,0],[0,1],[0,0],[0,0]]]),
                 pair_free_mask=torch.tensor([[[0,0],[0,0],[0,0],[1,0],[0,0]]]))
        self.assertEqual(actor_mask(b).tolist(),[[False,True,False,True,False]])

if __name__=='__main__':unittest.main()
