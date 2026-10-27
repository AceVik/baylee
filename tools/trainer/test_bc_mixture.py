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

    def test_cli_opt_in_and_default(self):
        base=['train5','--data','old','--out','out']
        with patch('sys.argv',base):args,_,_=parse_args('v5')
        self.assertEqual(args.bc_replay_data,'')
        with patch('sys.argv',base+['--bc-replay-data','new','--validation-data','val','--batch-size','8']):
            args,_,_=parse_args('v5');self.assertEqual(args.bc_replay_data,'new')
        for extra in ([],['--validation-data','val','--batch-size','3'],['--validation-data','val','--max-shards','1']):
            with patch('sys.argv',base+['--bc-replay-data','new']+extra),patch('sys.stderr'):
                with self.assertRaises(SystemExit):parse_args('v5')


if __name__=='__main__':unittest.main()
