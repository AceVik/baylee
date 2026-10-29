"""Opt-in synthetic CUDA backward/optimizer probe; never gameplay training."""
import json
import os
import unittest
import torch
import model5 as M
from export_onnx4 import make_dummy_inputs
from recurrent_ppo4.optimization import ppo_loss

@unittest.skipUnless(os.environ.get('BAYLEE_FULL_PPO_CUDA')=='1','exclusive GPU slot required')
class FullPPOCapacity(unittest.TestCase):
    def test_backward_optimizer(self):
        torch.set_num_threads(4);torch.manual_seed(291014)
        torch.use_deterministic_algorithms(True)
        device=torch.device('cuda')
        total=torch.cuda.get_device_properties(device).total_memory
        torch.cuda.set_per_process_memory_fraction(min(.99,10.5*1024**3/total))
        b=int(os.environ.get('BAYLEE_PPO_PROBE_BATCH','2'))
        u=int(os.environ.get('BAYLEE_PPO_PROBE_UNROLL','16'))
        self.assertIn((b,u),((2,16),(8,32)))
        net=M.PlayerNet5(M.Config5(d_model=640,n_heads=10,n_layers=8,d_ff=2560,
            activation_checkpointing=True,relational_revision=2,candidate_context_revision=2)).to(device).eval()
        net.checkpoint_during_eval=True
        optimizer=torch.optim.AdamW(net.parameters(),lr=2e-5,weight_decay=0)
        h=torch.zeros(b,9,640,device=device)
        inputs=tuple(x.to(device) for x in make_dummy_inputs(640,B=b,E=64,M_edges=128,C=64,P=64,K=2)[:-2])
        torch.cuda.reset_peak_memory_stats()
        start,end=torch.cuda.Event(enable_timing=True),torch.cuda.Event(enable_timing=True)
        start.record();logps=[];values=[]
        for t in range(u):
            k=torch.full((b,),0 if t==0 else 2,dtype=torch.long,device=device)
            h,cl,pl,scalar,value=net(*inputs,h,k);values.append(value)
            if t:logps.append(torch.log_softmax(cl/.7,-1)[:,t%64])
        lp=torch.cat(logps);value=torch.cat(values)
        output=dict(logprob=lp,behavior=lp.detach(),advantage=torch.linspace(-1,1,len(lp),device=device),
                    value=value,returns=torch.ones_like(value))
        loss=ppo_loss(output);self.assertTrue(bool(torch.isfinite(loss)));loss.backward()
        gradients={}
        for prefix in ('relational','trunk_layers','recurrent_cell','thinking_block'):
            parts=[p.grad.abs().sum() for name,p in net.named_parameters() if prefix in name and p.grad is not None]
            gradients[prefix]=float(torch.stack(parts).sum());self.assertGreater(gradients[prefix],0,prefix)
        torch.nn.utils.clip_grad_norm_(net.parameters(),1,error_if_nonfinite=True);optimizer.step()
        end.record();end.synchronize();peak=torch.cuda.max_memory_reserved()
        self.assertLessEqual(peak,10.5*1024**3)
        print(json.dumps(dict(kind='synthetic_full_ppo_backward_optimizer_capacity',device=torch.cuda.get_device_name(),
            batch=b,unroll=u,entities=64,candidates=64,d_model=640,layers=8,attention_heads=10,catalog_scope="synthetic unknown rule only",thinking_steps=2,precision='float32',
            activation_checkpointing=True,elapsed_ms=start.elapsed_time(end),peak_reserved_bytes=peak,
            peak_allocated_bytes=torch.cuda.max_memory_allocated(),parameter_bytes=sum(p.numel()*p.element_size() for p in net.parameters()),
            gradient_l1=gradients,loss=float(loss.detach())),sort_keys=True),flush=True)

    def test_a_full_transaction_and_resume(self):
        import copy
        import io
        from test_recurrent_ppo5 import EpisodeDataset
        from test_policy_training4 import assert_state_equal
        from recurrent_ppo4.forward import episode_prefixes
        from recurrent_ppo4.optimization import transactional_update
        from recurrent_ppo4.checkpoint import training_payload,restore_training
        torch.set_num_threads(1);torch.manual_seed(72)
        torch.use_deterministic_algorithms(True)
        cfg=M.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,relational_revision=2,candidate_context_revision=2,
                      activation_checkpointing=True)
        model=M.PlayerNet5(cfg).eval();model.checkpoint_during_eval=True
        dataset=EpisodeDataset(model);prefixes=episode_prefixes(dataset)
        device=torch.device('cuda');model.to(device)
        optimizer=torch.optim.AdamW(model.parameters(),lr=.0001)
        first=transactional_update(model,optimizer,dataset,[1],prefixes,device,target_kl=1)
        self.assertTrue(first['accepted'])
        contract={'version':2,'behavior_probability_reference':'recorded_executed_action',
                  'device':'cuda','fixture':'synthetic'}
        source=dict(model_version=5,config=cfg.to_dict(),structure={})
        stream=io.BytesIO();torch.save(training_payload(model,optimizer,source,contract,{'next_batch':1}),stream)
        expected=transactional_update(model,optimizer,dataset,[3],prefixes,device,target_kl=1)
        self.assertTrue(expected['accepted'])
        weights=copy.deepcopy(model.state_dict());moments=copy.deepcopy(optimizer.state_dict())
        fresh=M.PlayerNet5(cfg).to(device).eval();fresh.checkpoint_during_eval=True
        freshopt=torch.optim.AdamW(fresh.parameters(),lr=.0001)
        stream.seek(0);saved=torch.load(stream,map_location='cpu',weights_only=False)
        restore_training(saved,fresh,freshopt,contract,device)
        actual=transactional_update(fresh,freshopt,dataset,[3],prefixes,device,target_kl=1)
        self.assertEqual(actual,expected)
        assert_state_equal(self,fresh.state_dict(),weights)
        assert_state_equal(self,freshopt.state_dict(),moments)
        print(json.dumps(dict(kind='synthetic_full_ppo_cuda_exact_resume',accepted=True,
                              collection_behavior_kl=actual['collection_behavior_kl'])),flush=True)
