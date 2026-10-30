"""Streaming current-policy KL must agree with full-prefix reconstruction."""

import copy
import unittest
from unittest.mock import patch

import numpy as np
import torch

import model5 as M
from recurrent_ppo4.forward import episode_prefixes, segment_outputs
from recurrent_ppo4.optimization import behavior_divergence, collection_divergence
from recurrent_ppo4.collection import stream_collection_probabilities
from test_policy_training4 import assert_state_equal
from test_recurrent_ppo5 import EpisodeDataset


class UnevenHistories(EpisodeDataset):
    """A long history, a short history, and a refill, with a padded last frame."""

    def __init__(self, net):
        super().__init__(net)
        base = copy.deepcopy(self.rows)
        self.rows = []
        self.trajectories = []
        for game, length in enumerate((4, 1, 2)):
            history = []
            for offset in range(length):
                row = copy.deepcopy(base[offset % len(base)])
                row['is_new_traj'] = np.array(offset == 0)
                row['segment_index'] = np.array(len(self.rows))
                row['global_feat'] += .07 * game + .011 * offset
                row['behavior_h'] = np.full((9, 32), 999., dtype=np.float32)
                if offset == length - 1:
                    row['step_valid'][-1] = False
                    row['actor_mask'][-1] = False
                history.append(len(self.rows))
                self.rows.append(row)
            self.trajectories.append(history)

    def __getitem__(self, index):
        if index == -1:
            row = {name: np.zeros_like(value) for name, value in self.rows[0].items()}
            row['cand_mask'][:, 0] = True
            row['pair_mask'][:, 0] = True
            row['is_new_traj'] = np.array(True)
            row['segment_index'] = np.array(-1)
            return row
        return self.rows[index]


class StreamingCollection5(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(814)
        self.net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1,
                                         d_ff=64, relational_revision=2,
                                         candidate_context_revision=2)).eval()
        self.data = UnevenHistories(self.net)
        self.prefixes = episode_prefixes(self.data)
        self.device = torch.device('cpu')

    @torch.no_grad()
    def reference(self):
        total, count = 0., 0
        for history in self.data.trajectories:
            for index in history:
                output = segment_outputs(self.net, self.data, [index], self.prefixes,
                                         self.device, gradients=False)
                total += float(behavior_divergence(output['logprob'], output['behavior']))
                count += output['logprob'].numel()
        self.net.clear_structure()
        return total / count

    def test_all_histories_once_refill_padding_and_changed_current_weights(self):
        for changed in (False, True):
            if changed:
                with torch.no_grad():
                    self.net.global_proj[0].weight.mul_(1.7)
                    self.net.recurrent_cell.gate_z.weight.mul_(1.3)
            expected = self.reference()
            before = copy.deepcopy(self.net.state_dict())
            for batch_size in (1, 2, 3):
                with self.subTest(changed=changed, batch=batch_size):
                    actual = collection_divergence(self.net, self.data, self.prefixes,
                                                   self.device, batch_size)
                    self.assertAlmostEqual(actual, expected, delta=2e-7)
                    assert_state_equal(self, before, self.net.state_dict())
                    self.assertTrue(all(p.grad is None for p in self.net.parameters()))

    def test_each_real_frame_forwarded_once_instead_of_replaying_prefixes(self):
        with patch.object(self.net, 'forward', wraps=self.net.forward) as forward:
            self.reference()
            reference_calls = forward.call_count
        with patch.object(self.net, 'forward', wraps=self.net.forward) as forward:
            collection_divergence(self.net, self.data, self.prefixes, self.device, 1)
            self.assertEqual(forward.call_count, len(self.data) * 3)
            self.assertEqual(reference_calls, (10 + 1 + 3) * 3)
            self.assertLess(forward.call_count, reference_calls)

    @torch.no_grad()
    def test_every_action_probability_matches_including_lane_reuse(self):
        # Unique negative recorded logprobs identify decisions after lane
        # interleaving. They are comparison keys, not newly admitted behavior.
        for index, row in enumerate(self.data.rows):
            row['behavior_lp'] = np.array([-1. - index * .1 - t * .01
                                           for t in range(3)], dtype=np.float32)
        expected = {}
        for history in self.data.trajectories:
            for index in history:
                out = segment_outputs(self.net, self.data, [index], self.prefixes,
                                      self.device, gradients=False)
                expected.update(zip(out['behavior'].tolist(), out['logprob'].tolist()))
        for batch_size in (1, 2, 3):
            actual = {}
            for current, behavior in stream_collection_probabilities(
                    self.net, self.data, self.prefixes, self.device, batch_size):
                for key, probability in zip(behavior.tolist(), current.tolist()):
                    self.assertNotIn(key, actual)
                    actual[key] = probability
            self.assertEqual(set(actual), set(expected))
            for key in expected:
                self.assertAlmostEqual(actual[key], expected[key], delta=2e-6)
        self.net.clear_structure()

    def test_failure_clears_structure_and_rejects_noncontiguous_history(self):
        broken = dict(self.prefixes)
        broken[1] = (1000,)
        with patch.object(self.net, 'clear_structure', wraps=self.net.clear_structure) as clear:
            with self.assertRaisesRegex(ValueError, 'contiguous history'):
                collection_divergence(self.net, self.data, broken, self.device, 1)
            self.assertGreater(clear.call_count, 0)
        for row in self.data.rows:
            row['actor_mask'].fill(False)
        with self.assertRaisesRegex(ValueError, 'No admitted policy decisions'):
            collection_divergence(self.net, self.data, self.prefixes, self.device, 2)


if __name__ == '__main__':
    unittest.main()
