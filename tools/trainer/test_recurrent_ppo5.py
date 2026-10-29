"""Full v5 PPO: current-H replay, genuine trunk gradients and transactions."""
import copy
import unittest
import numpy as np
import torch
import model5 as M
from test_training_execution4 import batch_fixture
from test_policy_training4 import assert_state_equal
from recurrent_ppo4.forward import advance, episode_prefixes, segment_outputs, reconstruct_hidden
from recurrent_ppo4.optimization import ppo_loss, transactional_update, collection_divergence


class EpisodeDataset:
    """Two complete learner histories including passive and padded frames."""
    def __init__(self, net):
        host = batch_fixture(b=2, u=3)
        host['step_valid'].fill_(True)
        host['head'].zero_()
        host['single_idx'].zero_()
        host['is_decision'].fill_(True)
        host['is_decision'][:, 0] = False
        host['cand_mask'].fill_(True)
        host['pair_mask'].fill_(True)
        host['thinking_steps'] = host['is_decision'].long() * 2
        host['temperature'] = torch.full((2, 3), .7)
        host['is_new_traj'] = torch.ones(2, dtype=torch.bool)
        host['is_terminal'] = torch.zeros(2, 3, dtype=torch.bool)
        host['actor_mask'] = host['is_decision'].clone()
        host['advantages'] = torch.tensor([[0., 1., -.3], [0., -.5, .7]])
        host['returns'] = torch.tensor([[1., 1., 1.], [0., 0., 0.]])
        host['behavior_lp'] = torch.zeros(2, 3)
        self.rows = []
        for game in range(2):
            for segment in range(2):
                row = {name: value[game].numpy().copy() for name, value in host.items()}
                row['is_new_traj'] = np.array(segment == 0)
                row['segment_index'] = np.array(len(self.rows))
                row['is_terminal'][-1] = segment == 1
                row['global_feat'] += game * .1 + segment * .03
                self.rows.append(row)
        self.trajectories = [[0, 1], [2, 3]]
        from dataset4 import collate_trajectories
        from policy_math4 import action_logprob_and_kl
        with torch.no_grad():
            for trajectory in self.trajectories:
                h = torch.zeros(1, 9, 32)
                for idx in trajectory:
                    batch = collate_trajectories([self[idx]])
                    outputs, h = advance(net, batch, h, torch.device('cpu'))
                    for t, out in enumerate(outputs):
                        lp, _ = action_logprob_and_kl(*out[1:4], *out[1:4], batch, t)
                        self.rows[idx]['behavior_lp'][t] = float(lp[0])
    def __getitem__(self, index): return self.rows[index]
    def __len__(self): return len(self.rows)


class RecurrentPPO5(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(814)
        self.device = torch.device('cpu')
        self.net = M.PlayerNet5(M.Config5(d_model=32,n_heads=4,n_layers=1,d_ff=64,
                                         relational_revision=2,candidate_context_revision=2)).eval()
        self.dataset = EpisodeDataset(self.net)
        self.prefixes = episode_prefixes(self.dataset)

    def test_prefix_recomputed_from_current_weights_and_not_behavior_h(self):
        baseline = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        self.dataset.rows[1]['behavior_h'] = np.full((9,32),999,dtype=np.float32)
        repeated = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        torch.testing.assert_close(repeated,baseline,atol=0,rtol=0)
        with torch.no_grad(): self.net.global_proj[0].weight.mul_(1.7)
        current = reconstruct_hidden(self.net,self.dataset,[1],self.prefixes,self.device)
        self.assertGreater(float((current-baseline).abs().max()),0)
        self.assertFalse(current.requires_grad)

    def test_full_unroll_has_graph_trunk_memory_and_thinking_gradients(self):
        output = segment_outputs(self.net,self.dataset,[1,3],self.prefixes,self.device)
        ppo_loss(output).backward()
        groups = ('relational','trunk_layers','recurrent_cell','thinking_block')
        for prefix in groups:
            total=sum(float(p.grad.abs().sum()) for name,p in self.net.named_parameters()
                      if prefix in name and p.grad is not None)
            self.assertGreater(total,0,prefix)

    def test_whole_collection_gate_restores_optimizer_and_weights(self):
        optimizer = torch.optim.AdamW(self.net.parameters(),lr=.001)
        initial = collection_divergence(self.net,self.dataset,self.prefixes,self.device,2)
        self.assertLess(initial,1e-12)
        accepted=transactional_update(self.net,optimizer,self.dataset,[1],self.prefixes,self.device,target_kl=1)
        self.assertTrue(accepted['accepted'])
        before=copy.deepcopy(self.net.state_dict());adam=copy.deepcopy(optimizer.state_dict())
        optimizer.param_groups[0]['lr']=1
        adam=copy.deepcopy(optimizer.state_dict())
        rejected=transactional_update(self.net,optimizer,self.dataset,[3],self.prefixes,self.device,target_kl=1e-12)
        self.assertFalse(rejected['accepted'])
        assert_state_equal(self,self.net.state_dict(),before)
        assert_state_equal(self,optimizer.state_dict(),adam)
