"""BC mixtures isolate conversions while retaining complete recurrent lanes."""
import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import numpy as np
from dataset4 import collate_trajectories
from test_value_calibration4 import TinyDataset
from training4.bc_mixture import BCMixture,BCMixtureSampler,check_resume,validate_budget
from training4.bc_mixture.metadata import game_metadata
from training4.cli import parse_args


class Data(TinyDataset):
    def __init__(self,root,namespace,split='train',games=12,run=None):
        super().__init__(16)
        self.data_dir=root;root.mkdir(parents=True,exist_ok=True)
        self.fingerprint=namespace;self.structure=SimpleNamespace(digest='catalog')
        self.meta=dict(source_namespace=namespace,package_sources=[])
        self.segments=[];self.trajectories=[];rows=[]
        run=run or root/'source';(run/'records').mkdir(parents=True,exist_ok=True)
        source_rows=[]
        for g in range(games):
            # Two game records share each seed/table; all seats must stay together.
            source_rows.append(dict(shard='a.gz',offset=g*10,len=10,seed=g//2,decks=['a','b']))
            self.meta['package_sources'].append(dict(game_id=g,split=split,replay_hashes_checked=True,
                source_shard=str((run/'records/a.gz').resolve()),source_offset=g*10,source_len=10))
            for seat in (0,1):
                trajectory=[]
                for cursor in (0,2):
                    trajectory.append(len(self.segments));start=len(rows)
                    rows.extend([(g,seat,cursor),(g,seat,cursor+1)])
                    self.segments.append(np.array([start,start+1]))
                self.trajectories.append(trajectory)
        (run/'games.jsonl').write_text(''.join(json.dumps(x)+'\n' for x in source_rows))
        self.store=SimpleNamespace(index=np.array(rows,dtype=[('game','u4'),('seat','u4'),('cursor','u8')]))

    def __getitem__(self,index):
        out=super().__getitem__(0)
        if index<0:
            out['step_valid'][:]=False;out['is_new_traj']=np.array(True)
        else:
            rows=self.store.index[self.segments[index]]
            out['game']=rows['game'].copy();out['seat']=rows['seat'].copy();out['cursor']=rows['cursor'].copy()
            out['step_valid'][:]=True;out['is_new_traj']=np.array(rows['cursor'][0]==0)
            out['is_decision']=np.array([False,True])
            out['is_terminal']=np.array([False,rows['cursor'][-1]==3])
        if index<0:
            for k in ('game','seat','cursor'):out[k]=np.zeros(2,dtype=np.uint32)
        return out


class MixtureTests(unittest.TestCase):
    def test_native_cycling_crosses_boundary_once_per_complete_pass(self):
        from training4.bc_mixture import cycling
        child=SimpleNamespace(epoch=0)
        seen=[]
        def batches(advance_epoch=False):
            self.assertFalse(advance_epoch)
            seen.append(child.epoch)
            return [[i] for i in range(200)]
        child._build_batches=batches
        with patch.object(cycling,'_native',wraps=cycling._native) as native:
            rows,receipt=cycling.cycle_source(child,2,7,201)
        self.assertEqual(native.call_count,3)
        self.assertEqual(seen,[7,1_000_010])
        self.assertEqual(len(rows),201)
        self.assertEqual(rows[-1],[0,-1])
        self.assertEqual(receipt['partial_repeat_batches'],1)

    def fixture(self,root):
        return Data(root/'old','old'),Data(root/'new','new'),Data(root/'val','validation',split='val')

    def test_same_anchor_lanes_control_augmented_and_whole_tables(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));control=BCMixture(a,a,v,8);aug=BCMixture(a,b,v,8)
            cs=list(BCMixtureSampler(control,8,42));as_=list(BCMixtureSampler(aug,8,42))
            self.assertEqual([x[:4] for x in cs],[x[:4] for x in as_[:len(cs)]])
            meta,_=game_metadata(a,'train')
            left={int(a.store.index['game'][a.segments[i][0]]) for i in control.sources[0].segments}
            right={int(a.store.index['game'][a.segments[i][0]]) for i in control.sources[1].segments}
            self.assertFalse(left&right);self.assertEqual(left|right,set(meta))
            for g in left:self.assertIn(g^1,left) # paired rotations from same table
            self.assertEqual(aug.sources[0].segments,control.sources[0].segments)

    def test_colliding_local_ids_unique_segments_chronology_and_reset(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));mix=BCMixture(a,b,v,8);sampler=BCMixtureSampler(mix,8,2)
            seen=[];previous={};collisions=set();exhausted=0
            for ids in sampler:
                batch=collate_trajectories([mix[i] for i in ids])
                for lane,index in enumerate(ids):
                    if index<0:
                        exhausted+=1;self.assertFalse(batch['step_valid'][lane].any());self.assertTrue(batch['is_new_traj'][lane]);continue
                    seen.append(index);source=int(batch['bc_source'][lane]);self.assertEqual(source,int(lane>=4))
                    game=int(batch['game'][lane,0]);seat=int(batch['seat'][lane,0]);cursor=int(batch['cursor'][lane,0]);collisions.add((source,game))
                    if batch['is_new_traj'][lane]:self.assertEqual(cursor,0)
                    else:self.assertEqual(previous[lane],(source,game,seat,cursor-2))
                    previous[lane]=(source,game,seat,cursor)
            self.assertEqual(sorted(seen),list(range(len(mix))));self.assertGreater(exhausted,0)
            self.assertTrue({g for s,g in collisions if s==0}&{g for s,g in collisions if s==1})

    def test_resume_contract_and_sampler_suffix(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));mix=BCMixture(a,b,v,8);s=BCMixtureSampler(mix,8,7);s.epoch=3;all_=list(s)
            resumed=BCMixtureSampler(mix,8,7);resumed.epoch=3;resumed.start_batch=2
            self.assertEqual(list(resumed),all_[2:]);check_resume({'bc_mixture':mix.contract},mix.contract)
            for changed in (None,{**mix.contract,'lane_counts':[2,6]}):
                with self.assertRaisesRegex(ValueError,'mixture'):check_resume({'bc_mixture':mix.contract},changed)
            other=copy.deepcopy(mix.contract);other['sources'][1]['fingerprint']='changed'
            with self.assertRaises(ValueError):check_resume({'bc_mixture':mix.contract},other)
            original=mix.fingerprint;b.fingerprint='new-data-identity'
            changed=BCMixture(a,b,v,8)
            self.assertNotEqual(changed.fingerprint,original)
            with self.assertRaises(ValueError):check_resume({'bc_mixture':mix.contract},changed.contract)
            check_resume({},None)

    def test_overlap_validation_and_missing_metadata_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp))
            overlap=Data(Path(tmp)/'copy','different-namespace',run=a.data_dir/'source')
            with self.assertRaisesRegex(ValueError,'overlaps'):BCMixture(a,overlap,v,8)
            val=Data(Path(tmp)/'leakyval','different-validation',split='val',run=b.data_dir/'source')
            with self.assertRaisesRegex(ValueError,'overlaps'):BCMixture(a,b,val,8)
            saved=b.trajectories
            b.trajectories=[t for t in saved if b.store.index['seat'][b.segments[t[0]][0]]==0]
            with self.assertRaisesRegex(ValueError,'all source game seats'):BCMixture(a,b,v,8)
            b.trajectories=saved
            b.meta['package_sources'].pop()
            with self.assertRaisesRegex(ValueError,'exact dataset'):BCMixture(a,b,v,8)

    def test_eval_game_indices_resolve_seats_from_pinned_run_metadata(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));index=b.data_dir/'source/games.jsonl'
            rows=[json.loads(line) for line in index.read_text().splitlines()]
            for row in rows:row.pop('decks')
            index.write_text(''.join(json.dumps(row)+'\n' for row in rows))
            run=index.parent/'run.json';run.write_text(json.dumps({'seats':2}))
            mix=BCMixture(a,b,v,8)
            self.assertIn(str(run),mix.contract['source_metadata_sha256'])
            run.write_text(json.dumps({'seats':4}))
            with self.assertRaisesRegex(ValueError,'all source game seats'):BCMixture(a,b,v,8)

    def test_explicit_matched_budget_gate_does_not_change_tail_sampler(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));mix=BCMixture(a,b,v,8);sampler=BCMixtureSampler(mix,8,9)
            prefixes=sampler.full_lane_prefix_batches
            batches=list(sampler)
            for source,prefix in enumerate(prefixes):
                self.assertTrue(all(all(i>=0 for i in row[source*4:(source+1)*4]) for row in batches[:prefix]))
                if prefix<len(batches):self.assertIn(-1,batches[prefix][source*4:(source+1)*4])
            validate_budget(prefixes,min(prefixes),1)
            for updates,epochs in ((min(prefixes)+1,1),(1,2),(0,1)):
                with self.assertRaises(ValueError):validate_budget(prefixes,updates,epochs)
            self.assertGreater(len(batches),min(prefixes)) # general tails remain supported

    def test_default_sampler_matches_original_child_zip_and_contract(self):
        from itertools import zip_longest
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));mix=BCMixture(a,b,v,8);contract=copy.deepcopy(mix.contract)
            sampler=BCMixtureSampler(mix,8,11)
            streams=[child._build_batches(advance_epoch=False) for child in sampler.children]
            expected=[left+[i+mix.offset if i>=0 else -1 for i in right]
                      for left,right in zip_longest(*streams,fillvalue=[-1]*4)]
            self.assertEqual(list(sampler),expected)
            self.assertEqual(mix.contract,contract);self.assertIsNone(sampler.schedule_report())

    def test_cycling_shared_anchor_complete_passes_chronology_and_reproducibility(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp));control=BCMixture(a,a,v,8);aug=BCMixture(a,b,v,8)
            budget=max(len(BCMixtureSampler(m,8,42)) for m in (control,aug))+19
            cs=BCMixtureSampler(control,8,42,budget);as_=BCMixtureSampler(aug,8,42,budget)
            cb=list(cs);ab=list(as_)
            self.assertEqual(len(cb),budget);self.assertEqual(len(ab),budget)
            self.assertEqual([r[:4] for r in cb],[r[:4] for r in ab])
            repeat=BCMixtureSampler(aug,8,42,budget)
            self.assertEqual(ab,list(repeat));report=repeat.schedule_report() # epoch1 after iteration
            self.assertTrue(all(r['repeated_passes_started']>0 for r in report))
            repeat.epoch=0;report=repeat.schedule_report()
            previous={}
            for row in ab:
                for lane,index in enumerate(row):
                    item=aug[index]
                    if index<0:
                        self.assertTrue(item['is_new_traj']);previous.pop(lane,None);continue
                    key=(int(item['bc_source']),int(item['game'][0]),int(item['seat'][0]))
                    cursor=int(item['cursor'][0])
                    if item['is_new_traj']:self.assertEqual(cursor,0)
                    else:self.assertEqual(previous[lane],(key,cursor-2))
                    previous[lane]=(key,cursor)
            for source,r in enumerate(report):
                first=r['first_pass_batches'];offset=aug.offset if source else 0
                seen=[i-offset for row in ab[:first] for i in row[4*source:4*source+4] if i>=0]
                self.assertEqual(sorted(seen),list(range(len(aug.sources[source].segments))))
                self.assertEqual(r['padding_lane_batches'],sum(i<0 for row in ab for i in row[4*source:4*source+4]))
                self.assertGreaterEqual(r['complete_passes'],1)
                self.assertEqual(r['passes'][1]['sampler_epoch'],1_000_003)
            resumed=BCMixtureSampler(aug,8,42,budget);resumed.start_batch=7
            self.assertEqual(list(resumed),ab[7:])
            other=BCMixture(a,b,v,8);BCMixtureSampler(other,8,42,budget+1)
            with self.assertRaises(ValueError):check_resume({'bc_mixture':aug.contract},other.contract)
            other=BCMixture(a,b,v,8);BCMixtureSampler(other,8,43,budget)
            with self.assertRaises(ValueError):check_resume({'bc_mixture':aug.contract},other.contract)

    def test_cycling_rejects_partial_first_pass_and_malformed_budget(self):
        with tempfile.TemporaryDirectory() as tmp:
            a,b,v=self.fixture(Path(tmp))
            for budget in (-1,True,1.5,1):
                with self.assertRaises(ValueError):BCMixtureSampler(BCMixture(a,b,v,8),8,4,budget)

    def test_cli_opt_in_and_default(self):
        base=['train5','--data','old','--out','out']
        with patch('sys.argv',base):args,_,_=parse_args('v5')
        self.assertEqual(args.bc_replay_data,'');self.assertEqual(args.bc_epoch_batches,0)
        with patch('sys.argv',base+['--bc-replay-data','new','--validation-data','val','--batch-size','8']):
            args,_,_=parse_args('v5');self.assertEqual(args.bc_replay_data,'new')
        for extra in ([],['--bc-epoch-batches','-1'],['--validation-data','val','--batch-size','3'],['--validation-data','val','--max-shards','1']):
            with patch('sys.argv',base+['--bc-replay-data','new']+extra),patch('sys.stderr'):
                with self.assertRaises(SystemExit):parse_args('v5')
        with patch('sys.argv',base+['--bc-epoch-batches','100']),patch('sys.stderr'):
            with self.assertRaises(SystemExit):parse_args('v5')
        with patch('sys.argv',base+['--bc-replay-data','new','--validation-data','val','--bc-epoch-batches','100']):
            args,_,_=parse_args('v5');self.assertEqual(args.bc_epoch_batches,100)


if __name__=='__main__':unittest.main()
