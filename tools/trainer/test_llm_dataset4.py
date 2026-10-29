"""Executed LLM admission/cache provenance and explicit PPO exclusion."""
import copy,gzip,hashlib,json,tempfile,unittest
from pathlib import Path
import torch
import model4 as M
from dataset4 import parse_source_kind
from trajectory_store4 import validate_bc_dataset
from trajectory_store4 import validate_bc_dataset as validate_llm_dataset
from policy_training4.targets import prepare_policy_targets
from ppo_targets4 import prepare_targets
from test_policy_targets4 import Dataset,Frozen
from trajectory_store4 import BST4_HDR,TrajectoryStore4,pack_step_bst4

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def fixture(root):
 (root/'llm-bc').mkdir();(root/'train').mkdir()
 source=dict(converted_game_id=1,source_namespace='a'*64,source_game='attempt1',run_sha256='a'*64,record_sha256='b'*64,converted_labels='llm-bc/game-000001.jsonl',converted_manifest='llm-bc/game-000001.manifest.json')
 artifact=lambda role:dict(path='/original/'+role,sha256='c'*64)
 manifest=dict(schema_version=1,contract='admitted_executed_llm_v1',source_game='attempt1',record_sha256='b'*64,record_artifact_sha256='d'*64,request=artifact('request'),admitted_receipt=artifact('admitted'),technical_result=artifact('technical'),seats=[dict(seat=s,model='model'+str(s),transcript=artifact('transcript'+str(s)),origins=artifact('origins'+str(s))) for s in (0,1)])
 mp=root/source['converted_manifest'];mp.write_text(json.dumps(manifest));source['manifest_sha256']=digest(mp)
 header=dict(kind='header',schema=1,converted_game_id=1,source_namespace='a'*64,source_game='attempt1',run_sha256='a'*64,manifest_sha256=source['manifest_sha256'],record_sha256='b'*64,label_use='bc_only',ppo_eligible=False)
 rows=[header,dict(kind='label',seat=0,step_cursor=1,record_input_n=5,executed_source='llm',ambiguous=False,model='model0',executed_action={'PassPriority':{'player':0}},target={'single_idx':0}),dict(kind='completed',labels=1,ppo_eligible=False)]
 path=root/source['converted_labels'];write_rows(path,rows)
 steps=[]
 for seat in (0,1):
  for cursor in (0,1,2):
   decision=cursor==1
   steps.append(dict(game_id=1,seat=seat,step_cursor=cursor,turn=1,head=0,pending_kind=0,is_decision=decision,is_terminal=cursor==2,loss_weight=float(decision and seat==0),outcome=float(seat==0),provenance=dict(source='External' if decision else 'Forced',has_logprob=False,thinking_steps=1,temperature=0.,behavior_logprob=0.),state=dict(entity_feat=[],entity_card_ids=[],entity_face_ids=[],entity_token_ids=[],entity_rule_refs=[],seat_feat=[[0.]*M.SEAT_FEAT_WIDTH for _ in range(8)],global_feat=[0.]*M.GLOBAL_FEAT_WIDTH,edges=[]),cand_feat=[],cand_entity_idx=[],cand_rel_seat_idx=[],cand_rule_refs=[],pairs=[],target=dict(single_idx=0,multi_mask=[],free_mask=[],scalar_norm=0.)))
 shard=root/'train/a.jsonl.gz'
 with gzip.open(shard,'wt') as f:
  for step in steps:f.write(json.dumps(step)+'\n')
 (root/'card_structure.json').write_text('{"roots":{}}')
 meta=dict(llm_bc_version=1,llm_bc_contract='verified_executed_llm_v1',llm_bc_status='complete',ppo_eligible=False,policy_provenance_version=None,encoder_version=7,trajectory_version=2,llm_bc_sources=[source],llm_bc_label_sha256={source['converted_labels']:digest(path)},llm_bc_manifest_sha256={source['converted_manifest']:digest(mp)},llm_bc_frame_sha256={'train/a.jsonl.gz':digest(shard)},train_shards=['a.jsonl.gz'],val_shards=[])
 (root/'meta.json').write_text(json.dumps(meta));return meta,path,rows,steps

def write_rows(path,rows):path.write_text(''.join(json.dumps(v)+'\n' for v in rows))
def rehash(meta,path,root):meta['llm_bc_label_sha256'][str(path.relative_to(root))]=digest(path)

class LLMAdmission(unittest.TestCase):
 def test_packed_external_and_full_history_unchanged(self):
  for name,kind in [('Net',0),('House',1),('Forced',2),('Search',3),('External',4)]:
   self.assertEqual(parse_source_kind(name),kind)
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);meta,_,_,steps=fixture(root);validate_bc_dataset(root,meta)
   store=TrajectoryStore4(root,'train',meta['train_shards'])
   self.assertEqual(list(store.index['cursor']),[0,1,2,0,1,2]);self.assertEqual(list(store.index['flags']),[0,1,2,0,1,2])
   for row,step in enumerate(steps):
    header=BST4_HDR.unpack_from(store.read_raw(row));self.assertEqual(header[9],4 if step['is_decision'] else 2);self.assertFalse(header[6]&4);self.assertEqual(header[13],step['loss_weight']);self.assertEqual(store.read_raw(row),pack_step_bst4(step,{}))
   store.close()
 def test_companion_and_shard_tamper_before_existing_cache(self):
  for kind in ('companion','shard'):
   with tempfile.TemporaryDirectory() as d:
    root=Path(d);meta,path,_,_=fixture(root);TrajectoryStore4(root,'train',meta['train_shards']).close()
    p=path if kind=='companion' else root/'train/a.jsonl.gz';p.write_bytes(p.read_bytes()+b'\n')
    with self.assertRaisesRegex(ValueError,'digest mismatch'):TrajectoryStore4(root,'train',meta['train_shards'])
 def test_disallowed_policy_labels_even_with_refreshed_hash(self):
  for change in [dict(executed_source='house'),dict(executed_source='standing'),dict(ambiguous=True),dict(model='model1')]:
   with tempfile.TemporaryDirectory() as d:
    root=Path(d);meta,path,rows,_=fixture(root);rows[1].update(change);write_rows(path,rows);rehash(meta,path,root)
    with self.assertRaises(ValueError):validate_llm_dataset(root,meta)
  for kind in ('duplicate','missing-footer','header'):
   with tempfile.TemporaryDirectory() as d:
    root=Path(d);meta,path,rows,_=fixture(root)
    if kind=='duplicate':rows.insert(2,copy.deepcopy(rows[1]));rows[-1]['labels']=2
    elif kind=='missing-footer':rows.pop()
    else:rows[0]['record_sha256']='f'*64
    write_rows(path,rows);rehash(meta,path,root)
    with self.assertRaises(ValueError):validate_llm_dataset(root,meta)
 def test_unit_and_payload_player_actions_both_admitted(self):
  for action in ['MulliganKeep','MulliganTake','PassPriority',{'DeclareAttackers':{'attackers':[]}}]:
   with tempfile.TemporaryDirectory() as d:
    root=Path(d);meta,path,rows,_=fixture(root);rows[1]['executed_action']=action
    write_rows(path,rows);rehash(meta,path,root);validate_llm_dataset(root,meta)
  for action in ['',{},[],1,None]:
   with tempfile.TemporaryDirectory() as d:
    root=Path(d);meta,path,rows,_=fixture(root);rows[1]['executed_action']=action
    write_rows(path,rows);rehash(meta,path,root)
    with self.assertRaises(ValueError):validate_llm_dataset(root,meta)
 def test_incomplete_mixed_or_unbound_contract(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);meta,_,_,_=fixture(root)
   for change in [dict(llm_bc_status='in_progress'),dict(ppo_eligible=True),dict(policy_provenance_version=2),dict(shadow_bc_version=1),dict(llm_bc_frame_sha256={}),dict(llm_bc_sources=meta['llm_bc_sources']*2)]:
    with self.assertRaises(ValueError):validate_llm_dataset(root,{**meta,**change})
   validate_bc_dataset(root,{'encoder_version':7})
 def test_both_ppo_paths_reject_spoofed_net(self):
  for marker in ({'llm_bc_version':1},{'llm_bc_sources':[]}):
   dataset,model=Dataset(),Frozen();dataset.meta.update(marker);dataset.meta['ppo_eligible']=True
   with self.assertRaisesRegex(ValueError,'BC-only'):prepare_policy_targets(model,dataset,torch.device('cpu'))
   with self.assertRaisesRegex(ValueError,'BC-only'):prepare_targets(model,dataset,torch.device('cpu'),2,.99,.95,None,None)
   self.assertEqual(model.calls,[]);self.assertFalse(hasattr(dataset,'ppo_cache'))
 def test_external_source_is_not_a_neural_actor_even_with_claimed_probability(self):
  from dataset4 import collate_trajectories
  from policy_training4.targets import actor_mask,_raw_choice
  batch=collate_trajectories([Dataset()[0]])
  batch['source_kind'].fill_(4);batch['has_logprob'].fill_(True);batch['temperature'].fill_(.8)
  self.assertFalse(actor_mask(batch).any())
  self.assertFalse(_raw_choice(dict(is_decision=True,head=0,cand_feat=[[],[]],provenance=dict(source='External',has_logprob=True,temperature=.8,behavior_logprob=-1.))))
 def test_mixture_record_hash_overlap_across_namespace(self):
  from test_bc_mixture import Data
  from training4.bc_mixture.metadata import game_metadata,reject_overlap
  with tempfile.TemporaryDirectory() as d:
   data=Data(Path(d)/'new','namespace');data.meta['llm_bc_sources']=[dict(converted_game_id=g,record_sha256=f'{g:064x}') for g in range(12)]
   games,_=game_metadata(data,'train');self.assertEqual(games[0]['record_sha256'],'0'*64)
   other={0:{**games[0],'record':('different',0,1),'table':'different','source_table':'different'}}
   with self.assertRaisesRegex(ValueError,'overlaps'):reject_overlap(games,other)

if __name__=='__main__':unittest.main()
