"""Versioned menu context: archive compatibility, learning and CPU execution."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
import onnxruntime as ort
import torch
from torch.nn import functional as F
import model4 as V4
import model5 as V5
from model4.candidate import score_candidate_hidden
from export_onnx4 import export_v4_onnx, load_export_checkpoint, make_dummy_inputs, main
from policy_training4.optimization import select_policy_parameters
import test_deferred4 as deferred_tests
import test_export_models as export_tests
from training4.deferred import _score_cand_rows, _score_cand_slice
from training4.device_loss import compute_unroll_device_loss
from value_calibration4.model_identity import source_model

class CandidateContextTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(1729)

    def net(self, revision=2, **overrides):
        return V5.PlayerNet5(V5.Config5(d_model=32,n_layers=1,n_heads=4,d_ff=64,
            structure_width=16,relational_width=16,relational_revision=2,
            candidate_context_revision=revision,**overrides))

    def test_transfer_initial_rng_weights_and_histories(self):
        torch.manual_seed(19); old=self.net(1).eval(); rng=torch.get_rng_state()
        torch.manual_seed(19); new=self.net().eval()
        self.assertTrue(torch.equal(rng,torch.get_rng_state()))
        self.assertEqual(set(new.state_dict())-set(old.state_dict()),
                         {'cand_context_gate.weight','cand_context_gate.bias'})
        new.load_state_dict(V5.warmstart_candidate_context(old.state_dict(),new),strict=True)
        for key,value in old.state_dict().items():
            self.assertTrue(torch.equal(value,new.state_dict()[key]),key)
        inputs=list(make_dummy_inputs(32,E=3,M_edges=2,C=4,P=2))
        with torch.no_grad():
            for k in (0,1,2,4,8):
                lh=torch.zeros_like(inputs[-2]); rh=lh.clone()
                for current in (0,k,0,k):
                    inputs[-1].fill_(current)
                    inputs[-2]=lh; left=old(*inputs)
                    inputs[-2]=rh; right=new(*inputs)
                    for a,b in zip(left,right): self.assertTrue(torch.equal(a,b))
                    lh,rh=left[0],right[0]
        for state,target in ((new.state_dict(),new),(old.state_dict(),old)):
            with self.assertRaises(ValueError): V5.warmstart_candidate_context(state,target)
        broken=dict(old.state_dict()); broken.pop('cand_ctx_proj.bias')
        with self.assertRaises(ValueError): V5.warmstart_candidate_context(broken,new)

    def test_categorical_gradient_subset_offset_and_optimizer_selection(self):
        net=self.net(); h=torch.randn(1,4,32); mask=torch.ones(1,4,dtype=torch.bool)
        F.cross_entropy(score_candidate_hidden(net,h,mask),torch.tensor([1])).backward()
        self.assertGreater(net.cand_context_gate.weight.grad.abs().max().item(),1e-5)
        self.assertLess(net.cand_ctx_proj.weight.grad.abs().max().item(),1e-6)
        net.zero_grad()
        F.binary_cross_entropy_with_logits(score_candidate_hidden(net,h,mask),torch.tensor([[0.,1.,0.,0.]])).backward()
        self.assertGreater(net.cand_ctx_proj.weight.grad.abs().max().item(),1e-5)
        selected={id(p) for p in select_policy_parameters(net)}
        for p in net.cand_context_gate.parameters(): self.assertIn(id(p),selected)
        self.assertFalse(net.global_proj[0].weight.requires_grad)

    def test_sensitivity_padding_permutation_and_batch_isolation(self):
        old=self.net(1); new=self.net()
        new.load_state_dict(V5.warmstart_candidate_context(old.state_dict(),new))
        with torch.no_grad(): new.cand_context_gate.weight.normal_(std=.2)
        h=torch.randn(2,4,32); changed=h.clone(); changed[0,2]+=torch.randn(32)*3
        mask=torch.tensor([[True,True,True,False],[True,False,True,False]])
        def gap(net,x):
            s=score_candidate_hidden(net,x,mask); return s[0,0]-s[0,1]
        self.assertLess(abs(float((gap(old,h)-gap(old,changed)).detach())),1e-6)
        self.assertGreater(abs(float((gap(new,h)-gap(new,changed)).detach())),1e-4)
        scores=score_candidate_hidden(new,h,mask)
        torch.testing.assert_close(scores[1],score_candidate_hidden(new,changed,mask)[1],atol=0,rtol=0)
        padded=h.clone(); padded[:,3]+=100
        torch.testing.assert_close(scores,score_candidate_hidden(new,padded,mask),atol=0,rtol=0)
        order=torch.tensor([3,1,0,2])
        torch.testing.assert_close(scores[:,order],score_candidate_hidden(new,h[:,order],mask[:,order]))
        empty=score_candidate_hidden(new,h[:,:1],torch.zeros(2,1,dtype=torch.bool))
        self.assertTrue(torch.equal(empty,torch.full_like(empty,-1e9)))

    def test_large_menu_pooling_chunk_checkpoint_and_gradients(self):
        net=self.net()
        with torch.no_grad(): net.cand_context_gate.weight.normal_(std=.1)
        for count in (511,512,513,1025):
            with self.subTest(candidates=count):
                inputs=make_dummy_inputs(32,E=3,M_edges=2,C=count,P=2)
                args=[inputs[i] for i in (8,14,16,9,10,11)]+[torch.randn(1,3,32),torch.randn(1,8,32),torch.randn(1,32)]
                args[5][:,-1]=False
                other=copy.deepcopy(net); net.zero_grad()
                left=_score_cand_rows(net,*args); right=_score_cand_rows(other,*args,use_ckpt=True)
                torch.testing.assert_close(left,right,atol=0,rtol=0)
                for output in (left,right): F.cross_entropy(output,torch.tensor([1])).backward()
                for (name,a),(_,b) in zip(net.named_parameters(),other.named_parameters()):
                    self.assertEqual(a.grad is None,b.grad is None,name)
                    if a.grad is not None: torch.testing.assert_close(a.grad,b.grad,atol=0,rtol=0,msg=name)
                h=_score_cand_slice(net,*args[:5],*args[6:])
                torch.testing.assert_close(left,score_candidate_hidden(net,h,args[5]),atol=1e-6,rtol=1e-5)

    def test_reference_deferred_pruned_losses_and_all_gradients(self):
        runner=deferred_tests.DeferredReadouts()
        for k in (0,1,2,4,8):
            for prune in (False,True):
                with self.subTest(k=k,prune=prune):
                    batch=runner.fixture(); net=self.net(activation_checkpointing=True)
                    with torch.no_grad():
                        net.cand_context_gate.weight.normal_(std=.05)
                        net.relational.output.weight.normal_(std=.05)
                    net.prune_unused_policy_heads=prune; other=copy.deepcopy(net)
                    if k == 0:
                        batch["is_decision"].zero_()
                    left,lh,plans=runner.outputs(net,batch,0,k or 2)
                    right,rh,_=runner.outputs(other,batch,3,k or 2)
                    torch.testing.assert_close(lh,rh,atol=0,rtol=0)
                    for a,b in zip(left,right):
                        for x,y in zip(a,b): torch.testing.assert_close(x,y,atol=3e-6,rtol=3e-5)
                    losses=[]
                    for model,outputs in ((net,left),(other,right)):
                        aux=model.aux_head(torch.stack([out[0][:,0] for out in outputs],1))
                        loss,_=compute_unroll_device_loss(outputs,aux,batch,plans)
                        losses.append(loss.detach()); loss.backward()
                    torch.testing.assert_close(*losses,atol=1e-6,rtol=1e-5)
                    for (name,a),(_,b) in zip(net.named_parameters(),other.named_parameters()):
                        self.assertEqual(a.grad is None,b.grad is None,name)
                        if a.grad is not None: torch.testing.assert_close(a.grad,b.grad,atol=3e-6,rtol=3e-5,msg=name)

    def test_strict_identity_archived_omissions_and_unknown_combinations(self):
        for bad in (0,4,True,2.):
            with self.assertRaises(ValueError): V5.Config5(candidate_context_revision=bad)
        with self.assertRaises(ValueError): V5.Config5(candidate_context_revision=2,relational_revision=1)
        net=self.net(); net.attach_structure(export_tests.FOREST)
        saved=export_tests.ExportModels().checkpoint(V5,net.cfg,net)
        self.assertEqual(saved['architecture'],V5.CANDIDATE_ARCHITECTURE)
        with tempfile.TemporaryDirectory() as root:
            path=Path(root)/'net.pt'
            for encoder in (7,8):
                saved['encoder_version']=encoder; torch.save(saved,path)
                _,loaded,_=load_export_checkpoint(path)
                self.assertEqual(loaded.cfg.candidate_context_revision,2)
                self.assertEqual(source_model(saved).cfg.candidate_context_revision,2)
            for change in ('architecture','config','state'):
                bad=copy.deepcopy(saved)
                if change=='architecture': bad['architecture']=V5.ARCHITECTURES[2]
                elif change=='config': bad['config'].pop('candidate_context_revision')
                else: bad['state_dict'].pop('cand_context_gate.weight')
                torch.save(bad,path)
                with self.assertRaises((ValueError,RuntimeError)): load_export_checkpoint(path)
            legacy=self.net(1); legacy.attach_structure(export_tests.FOREST)
            archived=export_tests.ExportModels().checkpoint(V5,legacy.cfg,legacy)
            archived['config'].pop('candidate_context_revision'); torch.save(archived,path)
            _,loaded,_=load_export_checkpoint(path)
            self.assertEqual(loaded.cfg.candidate_context_revision,1)
            self.assertFalse(hasattr(loaded,'cand_context_gate'))

    def test_learned_gate_cpu_onnx_all_depths(self):
        export_tests.ExportModels.assert_parity(self,V5,V5.Config5,V5.PlayerNet5,
                                  relational_revision=2,candidate_context_revision=2)

    def test_encoder8_cli_export_binds_candidate_revision_and_architecture(self):
        import json
        net=self.net().eval(); net.attach_structure(export_tests.FOREST)
        saved=export_tests.ExportModels().checkpoint(V5,net.cfg,net)
        saved['encoder_version']=8
        with tempfile.TemporaryDirectory() as root:
            path=Path(root)
            torch.save(saved,path/'net.pt')
            with patch('sys.argv',['export_onnx4.py','--model',root,'--check','0']):
                main()
            metadata=json.loads((path/'model.onnx.json').read_text())
            self.assertEqual(metadata['architecture'],V5.CANDIDATE_ARCHITECTURE)
            self.assertEqual(metadata['encoder_version'],8)
            self.assertEqual(metadata['config']['relational_revision'],2)
            self.assertEqual(metadata['config']['candidate_context_revision'],2)
            self.assertEqual(set(metadata['graphs']),{'0','1','2','4','8'})
            self.assertLess(metadata['parity_max_abs_diff'],1e-3)

    def test_cpu_onnx_empty_and_large_dynamic_menus(self):
        net=self.net().eval(); net.attach_structure(export_tests.FOREST)
        with torch.no_grad(): net.cand_context_gate.weight.normal_(std=.1)
        net.rules=net.rules.freeze()
        options=ort.SessionOptions(); options.intra_op_num_threads=1
        with tempfile.TemporaryDirectory() as root:
            path=Path(root)/'model.onnx'; export_v4_onnx(net,path,thinking_steps=2,fixed_depth=True)
            session=ort.InferenceSession(str(path),sess_options=options,providers=['CPUExecutionProvider'])
            for count in (1,513,1025):
                inputs=list(make_dummy_inputs(32,E=3,M_edges=2,C=count,P=2)); inputs[11][:,-1]=False
                with torch.no_grad(): expected=net(*inputs)
                actual=session.run(list(V4.OUTPUT_NAMES),{n:t.numpy() for n,t in zip(V4.INPUT_NAMES,inputs)})
                for index,(want,got) in enumerate(zip(expected,actual)):
                    if index==1: got=got[:,:want.shape[1]]
                    np.testing.assert_allclose(got,want.numpy(),rtol=2e-4,atol=2e-5)

if __name__=='__main__': unittest.main()
