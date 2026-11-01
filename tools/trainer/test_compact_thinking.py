"""CPU compact-thinking equivalence without skipping any recurrent observation."""
import copy,unittest
from dataclasses import replace
from unittest.mock import patch
import torch
import model4 as M
import model5 as V5
from test_training_execution4 import batch_fixture
from training4.execution import plan_batch,encode_boards,StepPlan
from training4.deferred import deferred_readouts
from training4.device_loss import compute_unroll_device_loss
from training4.loss_config import SupervisedLossConfig


class CompactThinkingTests(unittest.TestCase):
 def setUp(self):torch.set_num_threads(2);torch.manual_seed(198)
 def run_model(self,net,batch,plans):
  boards=encode_boards(net,batch,2,torch.device('cpu'))
  with patch.object(net,'observe_memory',wraps=net.observe_memory) as observe:
   output,h=deferred_readouts(net,batch,boards,torch.zeros(4,9,32),plans,3,torch.device('cpu'))
   self.assertEqual(observe.call_count,batch['step_valid'].shape[1])
  aux=net.aux_head(torch.stack([o[0][:,0] for o in output],1))
  loss,_=compute_unroll_device_loss(output,aux,batch,plans,SupervisedLossConfig(version=5,normalization='unroll_per_head'))
  return output,h,loss
 def test_outputs_loss_and_all_gradients_v4_v5_mixed_and_empty_rows(self):
  for family in ('v4','v5'):
   for mode in ('mixed','none','all'):
    with self.subTest(family=family,mode=mode):
     batch=batch_fixture(b=4,u=6)
     batch['step_valid'][:]=True
     if mode=='mixed':
      batch['is_decision'][:]=False;batch['is_decision'][0,1::2]=True;batch['is_decision'][2,2:]=True
     elif mode=='none':batch['is_decision'][:]=False
     else:batch['is_decision'][:]=True
     plans=plan_batch(batch,0,2)
     plans=[replace(p,thinking=(1,2,4)[t%3] if p.thinking else 0) for t,p in enumerate(plans)]
     net=(M.PlayerNet4(M.Config4(d_model=32,n_heads=4,n_layers=1,d_ff=64)) if family=='v4' else V5.PlayerNet5(V5.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,relational_revision=2,relational_width=8)))
     if family=='v5':torch.nn.init.normal_(net.relational.output.weight,std=.1)
     net.cache_thinking_kv=True;net.prune_unused_policy_heads=True
     compact=copy.deepcopy(net);compact.compact_thinking_rows=True
     left,h1,l1=self.run_model(net,batch,plans);right,h2,l2=self.run_model(compact,batch,plans)
     torch.testing.assert_close(h1,h2,rtol=0,atol=0)
     for a,b in zip(left,right):
      for x,y in zip(a,b):torch.testing.assert_close(x,y,rtol=3e-5,atol=4e-6)
     torch.testing.assert_close(l1,l2,rtol=3e-5,atol=4e-6)
     l1.backward();l2.backward()
     for (name,p),(other,q) in zip(net.named_parameters(),compact.named_parameters()):
      self.assertEqual(name,other)
      if p.grad is None:self.assertIsNone(q.grad,name)
      else:torch.testing.assert_close(p.grad,q.grad,rtol=3e-4,atol=1e-5,msg=name)
 def test_padding_and_planned_cpu_lane_selection(self):
  batch=batch_fixture(b=4,u=4);batch['is_decision'][:]=True;batch['step_valid'][3,:]=False
  batch['aux_target'][3]=float('nan');batch['outcome'][3]=float('nan')
  plans=plan_batch(batch,0,2)
  self.assertTrue(all(p.decision_rows==(0,1,2) for p in plans))
  net=M.PlayerNet4(M.Config4(d_model=32,n_heads=4,n_layers=1,d_ff=64));net.compact_thinking_rows=True
  with patch.object(net.thinking_block,'forward',wraps=net.thinking_block.forward) as think:
   _,_,loss=self.run_model(net,batch,plans)
   self.assertTrue(torch.isfinite(loss))
   self.assertTrue(all(call.args[0].shape[0]==12 for call in think.call_args_list))
  old=[StepPlan(p.candidates,p.pairs,p.thinking,p.valid,p.heads) for p in plans]
  with self.assertRaisesRegex(ValueError,'CPU decision-row'):self.run_model(net,batch,old)

if __name__=='__main__':unittest.main()
