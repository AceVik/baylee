"""Opt-in loss contracts, masks, gradients and BxU regrouping invariance."""
import copy,subprocess,types,unittest
from types import SimpleNamespace
import torch
from training4.device_loss import compute_unroll_device_loss,compute_device_loss
from training4.loss_config import SupervisedLossConfig as C
from training4.objective import check_resume_objective


def fixture(b=2,u=4):
 torch.manual_seed(31)
 batch={k:torch.ones(b,u,dtype=torch.bool) for k in ('step_valid','is_decision')}
 batch['head']=torch.arange(b*u).reshape(b,u)%4
 for k in ('loss_weight','outcome','scalar_target'):batch[k]=torch.rand(b,u)
 batch['single_idx']=torch.ones(b,u,dtype=torch.long)
 batch['aux_target']=torch.randn(b,u,6)
 for prefix,width in [('cand',3),('pair',2)]:
  batch[prefix+'_mask']=torch.ones(b,u,width,dtype=torch.bool)
  batch[prefix+'_free_mask']=torch.ones(b,u,width)
  batch[prefix+'_target_mask']=torch.randint(0,2,(b,u,width)).float()
 prepared=[(None,torch.randn(b,3,requires_grad=True),torch.randn(b,2,requires_grad=True),torch.randn(b,requires_grad=True),torch.randn(b,requires_grad=True)) for _ in range(u)]
 aux=torch.randn(b,u,6,requires_grad=True)
 plans=[SimpleNamespace(valid=True,heads=tuple(bool((batch['head'][:,t]==k).any()) for k in range(4))) for t in range(u)]
 return prepared,aux,batch,plans


class LossTests(unittest.TestCase):
 def setUp(self):torch.set_num_threads(2)
 def test_legacy_bitidentical_loss_statistics_and_gradients(self):
  # Frozen baseline function bodies, evaluated separately from modified dispatch.
  source=subprocess.check_output(['git','show','ff0c2474d00b2bb7391e11c8f4665f5d6b41a171:tools/trainer/training4/device_loss.py'],text=True)
  module=types.ModuleType('legacy');exec(source,module.__dict__)
  first=fixture();second=copy.deepcopy(first)
  a,sa=module.compute_unroll_device_loss(*first);b,sb=compute_unroll_device_loss(*second,loss_config=C())
  self.assertTrue(torch.equal(a,b));self.assertTrue(torch.equal(sa,sb));a.backward();b.backward()
  for old,new in zip([p for step in first[0] for p in step[1:]]+[first[1]],[p for step in second[0] for p in step[1:]]+[second[1]]):
   if old.grad is None:self.assertIsNone(new.grad)
   else:self.assertTrue(torch.equal(old.grad,new.grad))
 def test_global_regrouping_and_timestep_permutation(self):
  p,a,b,plans=fixture();cfg=C(version=5,normalization='unroll_per_head',outcome_weighting=False)
  expected=compute_unroll_device_loss(p,a,b,plans,loss_config=cfg)[0]
  order=[2,0,3,1]
  actual=compute_unroll_device_loss([p[i] for i in order],a[:,order],{k:v[:,order] for k,v in b.items()},[plans[i] for i in order],loss_config=cfg)[0]
  torch.testing.assert_close(expected,actual)
  flat={k:v.reshape(1,8,*v.shape[2:]) for k,v in b.items()}
  fp=[]
  for i in range(8):
   lane,t=divmod(i,4);fp.append((None,*[p[t][j][lane:lane+1] for j in range(1,5)]))
  fplans=[SimpleNamespace(valid=True,heads=tuple(int(flat['head'][0,t])==k for k in range(4))) for t in range(8)]
  regroup=compute_unroll_device_loss(fp,a.reshape(1,8,6),flat,fplans,loss_config=cfg)[0]
  torch.testing.assert_close(expected,regroup)
  tensors=[tensor for step in p for tensor in step[1:]]+[a]
  g1=torch.autograd.grad(expected,tensors,allow_unused=True,retain_graph=True)
  g2=torch.autograd.grad(regroup,tensors,allow_unused=True,retain_graph=True)
  for first,second in zip(g1,g2):
   if first is None:self.assertIsNone(second)
   else:torch.testing.assert_close(first,second)

 def test_inactive_forced_invalid_targets_and_absent_gradients(self):
  p,a,b,plans=fixture(b=1,u=2);b['head'][:]=0;b['single_idx'][:]=999;b['cand_mask'][:]=False;b['loss_weight'][:]=0
  plans=[SimpleNamespace(valid=True,heads=(True,False,False,False))]*2
  loss,_=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,normalization='unroll_per_head'))
  self.assertTrue(torch.isfinite(loss));loss.backward()
  for step in p:self.assertIsNone(step[2].grad);self.assertIsNone(step[3].grad)
 def test_masked_nan_and_variable_width_bce(self):
  p,a,b,plans=fixture(b=1,u=2);b['head'][:]=1;b['cand_mask'][:,:,2]=False;b['cand_target_mask'][:,:,2]=float('nan');b['cand_free_mask'][:,:,2]=float('nan')
  b['pair_target_mask'][:]=float('nan');b['single_idx'][:]=999
  p[1]=(p[1][0],p[1][1][:,:2],*p[1][2:]);plans=[SimpleNamespace(valid=True,heads=(False,True,False,False))]*2
  loss,_=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,normalization='unroll_per_head'))
  self.assertTrue(torch.isfinite(loss));loss.backward()
  self.assertIsNone(p[0][2].grad)
 def test_singleton_ce_cancellation_and_outcome_ablation_support(self):
  p,a,b,plans=fixture(b=1,u=2);b['head'][:]=0;b['outcome'][0]=torch.tensor([0.,1.]);b['loss_weight'][:]=1
  plans=[SimpleNamespace(valid=True,heads=(True,False,False,False))]*2
  on=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5))[1][0]
  off=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,outcome_weighting=False))[1][0]
  torch.testing.assert_close(on,off)
  globalon=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,normalization='unroll_per_head'))[1][0]
  globaloff=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,outcome_weighting=False,normalization='unroll_per_head'))[1][0]
  self.assertGreater(float((globalon-globaloff).abs()),1e-3)
 def test_inactive_nan_padding_cannot_poison_global_loss(self):
  p,a,b,plans=fixture(b=2,u=2);b['step_valid'][1]=False
  for key in ('loss_weight','outcome','scalar_target','aux_target','cand_target_mask','pair_target_mask','cand_free_mask','pair_free_mask'):b[key][1]=float('nan')
  for step in p:
   for tensor in step[1:]:
    with torch.no_grad():tensor[1]=float('nan')
  with torch.no_grad():a[1]=float('nan')
  loss,_=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,normalization='unroll_per_head'))
  self.assertTrue(torch.isfinite(loss));loss.backward()
  self.assertTrue(torch.isfinite(a.grad).all())
 def test_zero_valid_and_fractional_free_entry_denominators(self):
  p,a,b,plans=fixture(b=1,u=2)
  empty=[SimpleNamespace(valid=False,heads=(False,False,False,False))]*2
  loss,stats=compute_unroll_device_loss(p,a,b,empty,loss_config=C(version=5,normalization='unroll_per_head'))
  self.assertFalse(loss.requires_grad);self.assertEqual(stats.sum().item(),0)
  b['head'][:]=1;b['loss_weight'][:]=.1;b['outcome'][:]=.5
  b['cand_target_mask'][:]=0;b['cand_free_mask'][:]=0;b['cand_free_mask'][:,:,0]=1
  p=[(step[0],torch.zeros_like(step[1],requires_grad=True),*step[2:]) for step in p]
  plans=[SimpleNamespace(valid=True,heads=(False,True,False,False))]*2
  cfg=C(version=5,outcome_weighting=False,normalization='unroll_per_head')
  policy=compute_unroll_device_loss(p,a,b,plans,loss_config=cfg)[1][0]/2
  torch.testing.assert_close(policy,torch.log(torch.tensor(2.)))
  legacy=compute_unroll_device_loss(p,a,b,plans,loss_config=C(version=5,outcome_weighting=False))[1][0]/2
  torch.testing.assert_close(legacy,torch.log(torch.tensor(2.))*.1)
 def test_per_frame_global_rejected_and_legacy_explicit_matches(self):
  p,a,b,plans=fixture()
  with self.assertRaisesRegex(ValueError,'Unroll'):
   compute_device_loss(*p[0][1:],a[:,0],b,0,plans[0],C(version=5,normalization='unroll_per_head'))
  first=compute_device_loss(*p[0][1:],a[:,0],b,0,plans[0])
  second=compute_device_loss(*p[0][1:],a[:,0],b,0,plans[0],C())
  for x,y in zip(first,second):self.assertTrue(torch.equal(x,y))
 def test_resume_requires_exact_explicit_v5_contract(self):
  cfg=C(version=5,outcome_weighting=False,normalization='unroll_per_head');saved=dict(training_loss_version=5,checkpoint_selection='policy',supervised_loss_config=cfg.to_dict())
  check_resume_objective(saved,'policy',cfg)
  for bad in ({**saved,'supervised_loss_config':None},{**saved,'training_loss_version':4}):
   with self.assertRaises(ValueError):check_resume_objective(bad,'policy',cfg)
  with self.assertRaises(ValueError):C(outcome_weighting=False)
  with self.assertRaises(ValueError):check_resume_objective(saved,'policy')

if __name__=='__main__':unittest.main()
