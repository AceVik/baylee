"""CPU-only strict restore and production-equivalent disposable benchmark updates."""
import copy,tempfile,unittest,hashlib
from types import SimpleNamespace
from unittest.mock import patch
from pathlib import Path
import torch
import model4 as V4
import model5 as V5
from checkpoint4 import rng_state
from training4.loss_config import SupervisedLossConfig as C
from training4.optimizer_groups import build_optimizer_groups,apply_relational_warmup
from training4.device_loss import compute_unroll_device_loss
from training4.execution import encode_boards,plan_batch
from training4.deferred import deferred_readouts
from training4.supervised_options import auxiliary_readout
from test_training_execution4 import batch_fixture
from performance5.state import restore
from performance5.update import update,capture,difference
from performance5.runner import batches_for


class PerformanceTests(unittest.TestCase):
 def setUp(self):torch.set_num_threads(2);torch.manual_seed(77)
 def fixture(self,v5=True):
  config=C(version=5,outcome_weighting=False,normalization='unroll_per_head',detach_value_input=True,detach_aux_input=True)
  net=(V5.PlayerNet5(V5.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,relational_revision=2,relational_width=8,activation_checkpointing=True)) if v5 else V4.PlayerNet4(V4.Config4(d_model=32,n_heads=4,n_layers=1,d_ff=64,activation_checkpointing=True)))
  if v5:torch.nn.init.normal_(net.relational.output.weight,std=.1)
  groups,schedule=build_optimizer_groups(net,1e-4,architecture='v5' if v5 else 'v4',relational_lr=1e-3 if v5 else 0,warmup_steps=10 if v5 else 0)
  opt=torch.optim.AdamW(groups)
  saved=dict(config=net.cfg.to_dict(),state_dict=net.state_dict(),structure=net.rules.data,model_version=5 if v5 else 3,encoder_version=7,trajectory_version=2,sampling_version=2,training_loss_version=5,supervised_loss_config=config.to_dict(),checkpoint_selection='policy',optimizer=opt.state_dict(),optimizer_schedule=schedule,step_num=0,rng=rng_state())
  if v5:saved.update(architecture=V5.architecture_for_config(net.cfg),model_family='v5')
  return net,opt,saved,config
 def test_restore_strict_architecture_objective_optimizer(self):
  for family in (False,True):
   original,original_opt,saved,_=self.fixture(family)
   sum(p.square().sum() for p in original.parameters()).backward()
   apply_relational_warmup(original_opt,saved['optimizer_schedule'],1);original_opt.step()
   saved['state_dict']=copy.deepcopy(original.state_dict());saved['optimizer']=copy.deepcopy(original_opt.state_dict());saved['step_num']=1
   with tempfile.TemporaryDirectory() as temporary:
    path=Path(temporary)/'last.pt';torch.save(saved,path)
    _,net,opt,config=restore(path,torch.device('cpu'))
    for name,value in net.state_dict().items():self.assertTrue(torch.equal(value,saved['state_dict'][name]))
    actual=copy.deepcopy(opt.state_dict()['param_groups']);expected=copy.deepcopy(saved['optimizer']['param_groups'])
    for a,b in zip(actual,expected):a.pop('fused');b.pop('fused')
    self.assertEqual(actual,expected)
    for key,value in saved['optimizer']['state'].items():
     for field,tensor in value.items():torch.testing.assert_close(opt.state_dict()['state'][key][field],tensor,rtol=0,atol=0)
    for corrupt in ('objective','duplicate','permutation','shape'):
     bad=copy.deepcopy(saved)
     if corrupt=='objective':bad['supervised_loss_config'].pop('detach_aux_input')
     elif corrupt=='duplicate':bad['optimizer']['param_groups'][0]['params'][1]=bad['optimizer']['param_groups'][0]['params'][0]
     elif corrupt=='permutation':
      ids=bad['optimizer']['param_groups'][0]['params'];ids[0],ids[1]=ids[1],ids[0]
     else:
      identifier=bad['optimizer']['param_groups'][0]['params'][0];bad['optimizer']['state'][identifier]={'exp_avg':torch.zeros(3)}
     torch.save(bad,path)
     with self.assertRaises(ValueError):restore(path,torch.device('cpu'))
 def test_disposable_parity_restore_does_not_modify_checkpoint(self):
  _,_,saved,_=self.fixture()
  batch=batch_fixture(b=4,u=4);batch['is_new_traj']=torch.ones(4,dtype=torch.bool)
  with tempfile.TemporaryDirectory() as temporary:
   path=Path(temporary)/'last.pt';torch.save(saved,path)
   digest=hashlib.sha256(path.read_bytes()).hexdigest();snapshots=[]
   for _ in range(2):
    restored,net,opt,config=restore(path,torch.device('cpu'))
    net.decision_chunk=2;net.cache_thinking_kv=True;net.prune_unused_policy_heads=True
    update(net,opt,batch,None,torch.zeros(4,9,32),restored,config,0,2,before_clip=lambda n,h,l:snapshots.append(capture(n,h,l)))
   self.assertEqual(difference(*snapshots),dict(loss_abs=0.,hidden_max_abs=0.,gradient_relative_l2=0.))
   self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),digest)
 def test_pinned_budget_checked_before_pinning_and_lane_mode(self):
  args=SimpleNamespace(data='unused',workers=0,warmup=1,steps=1,host_limit_gib=1e-12,sampler_seed=None,resume_lanes=True)
  saved=dict(unroll=2,dataset_fingerprint='fixture',batch_size=1)
  with patch('performance5.runner.D.TrajectoryDataset4',return_value=SimpleNamespace(fingerprint='fixture')),patch('performance5.runner.checkpoint_sampler',return_value=[]) as sampler,patch('performance5.runner.DataLoader',return_value=[{'a':torch.ones(2)}]),patch.object(torch.Tensor,'pin_memory') as pin:
   with self.assertRaises(MemoryError):batches_for(saved,args)
   pin.assert_not_called();self.assertFalse(sampler.call_args.kwargs['fresh'])
 def test_update_matches_production_and_captures_raw_gradients(self):
  batch=batch_fixture(b=4,u=4);batch['is_new_traj']=torch.ones(4,dtype=torch.bool)
  net,opt,saved,config=self.fixture();reference=copy.deepcopy(net)
  refgroups,_=build_optimizer_groups(reference,1e-4,architecture='v5',relational_lr=1e-3,warmup_steps=10)
  refopt=torch.optim.AdamW(refgroups)
  for n in (net,reference):n.decision_chunk=2;n.cache_thinking_kv=True;n.prune_unused_policy_heads=True;n.detach_value_input=True
  hidden=torch.zeros(4,9,32)
  snapshots=[]
  actual_h,actual_loss=update(net,opt,batch,None,hidden,saved,config,0,2,before_clip=lambda n,h,l:snapshots.append(capture(n,h,l)))
  plans=plan_batch(batch,0,reference.cfg.default_thinking_steps)
  reference.prepare_structure((batch['entity_rules'],batch['cand_rules']))
  boards=encode_boards(reference,batch,2,torch.device('cpu'))
  outputs,expected_h=deferred_readouts(reference,batch,boards,hidden,plans,2,torch.device('cpu'))
  auxiliary=auxiliary_readout(reference,torch.stack([p[0][:,0] for p in outputs],1),config)
  loss,_=compute_unroll_device_loss(outputs,auxiliary,batch,plans,config);loss.backward()
  expected=capture(reference,expected_h,loss.detach())
  self.assertEqual(difference(expected,snapshots[0]),dict(loss_abs=0.,hidden_max_abs=0.,gradient_relative_l2=0.))
  torch.nn.utils.clip_grad_norm_(reference.parameters(),1.,error_if_nonfinite=True)
  apply_relational_warmup(refopt,saved['optimizer_schedule'],1);refopt.step()
  for name,value in net.state_dict().items():torch.testing.assert_close(value,reference.state_dict()[name],rtol=0,atol=0)
  self.assertFalse(actual_h.requires_grad)
  torch.testing.assert_close(actual_loss,loss)

if __name__=='__main__':unittest.main()
