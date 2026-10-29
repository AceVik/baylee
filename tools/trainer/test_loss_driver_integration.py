"""CPU production-route checks for opt-in objectives and detached critic/aux heads."""
import contextlib,copy,io,json,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import torch
import model4 as M
from model4.structure import structure_digest
from training4 import driver
from training4.cli import parse_args
from training4.loss_config import SupervisedLossConfig as C
from training4.supervised_options import auxiliary_readout
import test_deferred4 as deferred_tests
from test_training_execution4 import batch_fixture


class IntegrationTests(unittest.TestCase):
 def setUp(self):torch.set_num_threads(2);torch.manual_seed(941)
 def test_readout_detach_both_production_routes(self):
  helper=deferred_tests.DeferredReadouts();batch=helper.fixture()
  for family in ('v4','v5'):
   for route in (0,3):
    for detach in (False,True):
     if family == 'v4':
      net=M.PlayerNet4(M.Config4(d_model=32,n_heads=4,n_layers=1,d_ff=64))
     else:
      import model5 as V5
      net=V5.PlayerNet5(V5.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,relational_revision=2,relational_width=8))
      torch.nn.init.normal_(net.relational.output.weight,std=.1)
     net.detach_value_input=detach
     outputs,_,_=helper.outputs(net,batch,route,thinking_steps=2)
     loss=sum(o[4].square().sum() for o in outputs);loss.backward()
     trunk=[p.grad for name,p in net.named_parameters() if not name.startswith('seat_value_head.')]
     if detach:self.assertTrue(all(g is None for g in trunk))
     else:self.assertTrue(any(g is not None and g.abs().sum()>0 for g in trunk))
     self.assertTrue(any(p.grad is not None and p.grad.abs().sum()>0 for p in net.seat_value_head.parameters()))
     net.zero_grad(set_to_none=True)
     outputs,_,_=helper.outputs(net,batch,route,thinking_steps=2)
     config=C(version=5,detach_aux_input=detach)
     auxiliary_readout(net,torch.stack([o[0][:,0] for o in outputs],1),config).square().sum().backward()
     trunk=[p.grad for name,p in net.named_parameters() if not name.startswith('aux_head.')]
     if detach:self.assertTrue(all(g is None for g in trunk))
     else:self.assertTrue(any(g is not None and g.abs().sum()>0 for g in trunk))
 def test_cli_defaults_and_reference_fail_closed(self):
  base=['train5','--data','unused','--out','unused']
  with patch('sys.argv',base):args,_,_=parse_args('v5')
  self.assertEqual(args.supervised_loss,C())
  for extra in (['--outcome-weighting-off'],['--loss-version','5','--reference-execution'],['--loss-version','5','--init-state','old.pt']):
   with patch('sys.argv',base+extra),contextlib.redirect_stderr(io.StringIO()),self.assertRaises(SystemExit):parse_args('v5')
 def test_native_execution_admission_preserves_legacy_continuation(self):
  C().validate_execution(reference_execution=True,init_state='old.pt')
  C(version=5).validate_execution(reference_execution=False,init_state=None)
  with self.assertRaisesRegex(ValueError,'Loss5 requires optimized execution'):
   C(version=5).validate_execution(reference_execution=True,init_state=None)
  with self.assertRaisesRegex(ValueError,'package optimizer continuation'):
   C(version=5).validate_execution(reference_execution=False,init_state='old.pt')
 def test_one_actual_driver_update_and_checkpoint_contract(self):
  batch=batch_fixture(b=4,u=4);batch['is_new_traj']=torch.ones(4,dtype=torch.bool)
  forest=M.PlayerNet4(M.Config4(d_model=32,n_heads=4,n_layers=1,d_ff=64)).rules.data
  class Dataset:
   def __init__(self,*args,split,**kw):self.structure=SimpleNamespace(data=forest,digest=structure_digest(forest));self.fingerprint=split
   def __len__(self):return 1
  class Sampler:
   VERSION=driver.D4.StatefulBatchSampler4.VERSION
   def __init__(self,*args,**kw):self.epoch=0;self.start_batch=0
  results=[]
  with tempfile.TemporaryDirectory() as temporary:
   for chunk in (1,2):
    output=Path(temporary)/str(chunk)
    argv=['train5','--data','fake','--out',str(output),'--d-model','32','--layers','1','--heads','4','--d-ff','64','--batch-size','4','--unroll','4','--board-chunk','2','--decision-chunk',str(chunk),'--max-steps','1','--epochs','1','--loss-version','5','--loss-normalization','unroll_per_head','--outcome-weighting-off','--detach-value-input','--detach-aux-input']
    if chunk==2:argv.append('--compact-thinking-rows')
    metrics=dict(val_pol_loss=1.,val_mse=1.,val_aux_mse=1.,val_top1_acc=0.)
    with patch('sys.argv',argv),patch.object(driver.D4,'TrajectoryDataset4',Dataset),patch.object(driver.D4,'StatefulBatchSampler4',Sampler),patch.object(driver,'DataLoader',return_value=[]),patch.object(driver,'prefetch_batches',return_value=[(copy.deepcopy(batch),None)]),patch.object(driver,'evaluate_device',return_value=metrics),patch.object(driver.torch,'set_num_threads'),contextlib.redirect_stdout(io.StringIO()):
     driver.main('v5')
    saved=torch.load(output/'last.pt',map_location='cpu',weights_only=False)
    selected=torch.load(output/'net.pt',map_location='cpu',weights_only=False);meta=json.loads((output/'meta.json').read_text())
    self.assertEqual(saved['step_num'],1)
    for artifact in (saved,selected,meta):
     self.assertEqual(artifact['training_loss_version'],5)
     self.assertEqual(artifact['supervised_loss_config'],C(version=5,normalization='unroll_per_head',outcome_weighting=False,detach_value_input=True,detach_aux_input=True).to_dict())
    results.append(saved)
    # Resume rejects a changed loss contract before optimizer/lane continuation.
    bad_argv=argv+['--resume',str(output/'last.pt')]
    bad_argv.remove('--detach-value-input')
    with patch('sys.argv',bad_argv),patch.object(driver.D4,'TrajectoryDataset4',Dataset),patch.object(driver.D4,'StatefulBatchSampler4',Sampler),patch.object(driver,'DataLoader',return_value=[]),patch.object(driver.torch,'set_num_threads'),self.assertRaisesRegex(ValueError,'loss configuration'):
     driver.main('v5')
    # The old v5 contract cannot distinguish symmetric CE from the removed
    # pass heuristic. Reject before loading optimizer or restoring lane H.
    old=copy.deepcopy(saved)
    old['supervised_loss_config'].pop('categorical_weighting')
    old['optimizer']=None
    old['h']=torch.full((4,9,32),float('nan'))
    old_path=output/'ambiguous-v5.pt';torch.save(old,old_path)
    with patch('sys.argv',argv+['--resume',str(old_path)]),patch.object(driver.D4,'TrajectoryDataset4',Dataset),patch.object(driver.D4,'StatefulBatchSampler4',Sampler),patch.object(driver,'DataLoader',return_value=[]),patch.object(driver.torch,'set_num_threads'),patch.object(torch.optim.AdamW,'load_state_dict') as restore_optimizer,self.assertRaisesRegex(ValueError,'categorical_weighting'):
     driver.main('v5')
    restore_optimizer.assert_not_called()
    if chunk==1:
     transfer_argv=list(argv)
     transfer_output=Path(temporary)/'fresh-transfer'
     transfer_argv[transfer_argv.index('--out')+1]=str(transfer_output)
     transfer_argv+=['--init',str(old_path)]
     with patch('sys.argv',transfer_argv),patch.object(driver.D4,'TrajectoryDataset4',Dataset),patch.object(driver.D4,'StatefulBatchSampler4',Sampler),patch.object(driver,'DataLoader',return_value=[]),patch.object(driver,'prefetch_batches',return_value=[(copy.deepcopy(batch),None)]),patch.object(driver,'evaluate_device',return_value=metrics),patch.object(driver.torch,'set_num_threads'),patch.object(torch.optim.AdamW,'load_state_dict') as restore_optimizer,contextlib.redirect_stdout(io.StringIO()):
      driver.main('v5')
     restore_optimizer.assert_not_called()
     transferred=torch.load(transfer_output/'last.pt',map_location='cpu',weights_only=False)
     self.assertEqual(transferred['step_num'],1)
     self.assertEqual(transferred['supervised_loss_config']['categorical_weighting'],'symmetric_v1')
   for name,value in results[0]['state_dict'].items():
    torch.testing.assert_close(value,results[1]['state_dict'][name],atol=2e-5,rtol=2e-4)

if __name__=='__main__':unittest.main()
