"""The drift of decisions outside the action-probability contract: which rows, which KL, which gate."""
import copy
import math
import unittest

import numpy as np
import torch
import torch.nn.functional as F

import model5 as M
from policy_training4 import drift as X
from policy_training4.forward import observe_outside
from policy_training4.optimization import transactional_step
from recurrent_ppo4.forward import episode_prefixes
from recurrent_ppo4.optimization import (collection_divergence, source_readouts, transactional_update)
from test_policy_training4 import assert_state_equal
from test_recurrent_ppo5 import EpisodeDataset

TARGET = .02


def rows_batch():
    """One frame, ten lanes: what the contract certifies and every decision it does not."""
    b = 10
    batch = dict(step_valid=torch.ones(b, 1, dtype=torch.bool), is_decision=torch.ones(b, 1, dtype=torch.bool),
                 source_kind=torch.zeros(b, 1, dtype=torch.long), actor_mask=torch.zeros(b, 1, dtype=torch.bool),
                 head=torch.tensor([[0], [0], [0], [1], [1], [2], [2], [3], [0], [0]]),
                 temperature=torch.full((b, 1), .7), single_idx=torch.zeros(b, 1, dtype=torch.long),
                 cand_mask=torch.ones(b, 1, 4, dtype=torch.bool), pair_mask=torch.ones(b, 1, 3, dtype=torch.bool),
                 cand_free_mask=torch.ones(b, 1, 4), pair_free_mask=torch.ones(b, 1, 3),
                 cand_target_mask=torch.zeros(b, 1, 4), pair_target_mask=torch.zeros(b, 1, 3))
    batch['actor_mask'][0] = True                 # a certified menu
    batch['cand_mask'][1, 0, 3] = False           # a payment over three entries
    batch['cand_mask'][2, 0, 1:] = False          # one entry: no choice
    batch['cand_free_mask'][3, 0, 2:] = 0         # an attack with two free attackers
    batch['cand_free_mask'][4, 0] = 0             # a forced attack
    batch['pair_free_mask'][5, 0, 1:] = 0         # a block with one free pair
    batch['pair_free_mask'][6, 0] = 0             # a forced block
    batch['source_kind'][8] = 1                   # the house's answer
    batch['is_decision'][9] = False               # an observation
    return batch


def outputs_of(cand, pair, scalar):
    return None, cand, pair, scalar, None


class Rows(unittest.TestCase):
    def test_only_the_nets_uncertified_choices_count_by_head(self):
        found = X.outside(rows_batch(), 0)
        self.assertEqual({kind: rows.nonzero().flatten().tolist() for kind, rows in found.items()},
                         dict(single=[1], subset=[3], pair=[5], scalar=[7]))
        for name, value in (('step_valid', False), ('is_decision', False), ('source_kind', 1), ('actor_mask', True)):
            batch = rows_batch()
            batch[name][:] = value
            self.assertEqual(X.outside(batch, 0), {}, name)

    def test_the_kl_is_the_sources_against_the_current_policys_at_the_rows_temperature(self):
        torch.manual_seed(3)
        batch = rows_batch()
        source = torch.randn(10, 4), torch.randn(10, 3), torch.rand(10)
        current = torch.randn(10, 4), torch.randn(10, 3), torch.rand(10)
        kept = X.SourceReadouts()
        kept.observe(range(100, 110), batch, 0, outputs_of(*source))
        self.assertEqual({key: len(row) for key, row in kept.rows.items()},
                         {(101, 0): 3, (103, 0): 4, (105, 0): 3, (107, 0): 1})
        drift = X.Drift(kept)
        drift.observe(range(100, 110), batch, 0, outputs_of(*current))
        categorical = lambda logits: F.log_softmax(logits[1, :3] / .7, -1)
        p, q = categorical(source[0]), categorical(current[0])
        single = (p.exp() * (p - q)).sum().item()

        def bernoulli(a, b, free):
            a, b = torch.sigmoid(a[free] / .7), torch.sigmoid(b[free] / .7)
            return (a * (a / b).log() + (1 - a) * ((1 - a) / (1 - b)).log()).sum().item()
        subset = bernoulli(source[0][3], current[0][3], slice(0, 2))
        pair = bernoulli(source[1][5], current[1][5], slice(0, 1))
        for kind, expected in (('single', single), ('subset', subset), ('pair', pair),
                               ('scalar', abs(source[2][7] - current[2][7]).item())):
            self.assertAlmostEqual(drift.mean(kind), expected, places=5, msg=kind)
            self.assertEqual(drift.counts[kind], 1)
        report = drift.report()
        self.assertAlmostEqual(report['subset']['max_kl'], subset, places=5)
        self.assertEqual(report['scalar']['decisions'], 1)
        # A head is refused on its own: the others' rows cannot hide it. A number is only reported.
        limit = sorted((single, subset, pair))[1]
        self.assertEqual(set(drift.refused(limit + 1e-4)),
                         {kind for kind, kl in (('single', single), ('subset', subset), ('pair', pair))
                          if kl > limit + 1e-4})
        self.assertEqual(drift.refused(1e9), [])
        self.assertNotIn('scalar', drift.refused(-1.))
        # The same policy has not moved at all.
        still = X.Drift(kept)
        still.observe(range(100, 110), batch, 0, outputs_of(*source))
        self.assertEqual([still.mean(kind) for kind in X.KINDS], [0.] * 4)
        self.assertEqual(still.refused(0.), [])

    def test_missing_repeated_or_nonfinite_readouts_fail_closed(self):
        batch = rows_batch()
        values = outputs_of(torch.zeros(10, 4), torch.zeros(10, 3), torch.zeros(10))
        kept = X.SourceReadouts()
        kept.observe(range(10), batch, 0, values)
        with self.assertRaisesRegex(ValueError, 'recorded twice'):
            kept.observe(range(10), batch, 0, values)
        with self.assertRaisesRegex(ValueError, 'without the source readout'):
            X.Drift(kept).observe(range(10, 20), batch, 0, values)
        with self.assertRaisesRegex(ValueError, 'another menu'):
            X.Drift(kept).observe(range(10), batch, 0, outputs_of(torch.zeros(10, 2), torch.zeros(10, 3), torch.zeros(10)))
        broken = torch.zeros(10, 4)
        broken[3, 0] = float('nan')
        with self.assertRaisesRegex(FloatingPointError, 'Nonfinite drift'):
            X.Drift(kept).observe(range(10), batch, 0, outputs_of(broken, torch.zeros(10, 3), torch.zeros(10)))
        # A nonfinite logit of an entry nobody could choose is no drift.
        unused = torch.zeros(10, 4)
        unused[3, 3] = unused[1, 3] = float('nan')
        drift = X.Drift(kept)
        drift.observe(range(10), batch, 0, outputs_of(unused, torch.zeros(10, 3), torch.zeros(10)))
        self.assertEqual(drift.mean('subset'), 0.)
        empty = X.Drift(kept)
        self.assertEqual((empty.report()['pair'], empty.refused(0.)),
                         (dict(decisions=0, mean_kl=None, max_kl=None), []))


class MixedEpisodes(EpisodeDataset):
    """Each segment's second frame is a certified menu, its third an attack, a block, a number or
    a payment."""

    def __init__(self, net):
        super().__init__(net)
        for index, row in enumerate(self.rows):
            row['source_kind'] = np.zeros(3, dtype=np.int64)
            row['head'][2] = (1, 2, 3, 0)[index]
            row['actor_mask'][2] = False
            row['behavior_h'] = np.zeros((9, 32), dtype=np.float32)


class Gate(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(814)
        self.device = torch.device('cpu')
        self.net = M.PlayerNet5(M.Config5(d_model=32, n_heads=4, n_layers=1, d_ff=64, relational_revision=2,
                                          candidate_context_revision=2)).eval()
        self.data = MixedEpisodes(self.net)
        self.prefixes = episode_prefixes(self.data)
        self.source = source_readouts(self.net, self.data, self.prefixes, self.device, 2)

    def measure(self, batch_size=2):
        drift = X.Drift(self.source)
        divergence = collection_divergence(self.net, self.data, self.prefixes, self.device, batch_size, drift)
        return divergence, drift

    def test_the_source_itself_has_not_drifted_and_every_row_is_seen_once(self):
        self.assertEqual(len(self.source.rows), 4)
        for batch_size in (1, 2):
            divergence, drift = self.measure(batch_size)
            self.assertLess(divergence, 1e-12)
            self.assertEqual(drift.counts, dict(single=1, subset=1, pair=1, scalar=1))
            # Exactly nothing in the batches it was recorded in, float rounding in others.
            for kind in X.KINDS:
                self.assertLessEqual(drift.mean(kind), 0. if batch_size == 2 else 1e-6, kind)
        self.assertTrue(all(p.grad is None for p in self.net.parameters()))

    def test_the_divergence_is_the_one_without_the_drift(self):
        with torch.no_grad():
            self.net.global_proj[0].weight.mul_(1.7)
        plain = collection_divergence(self.net, self.data, self.prefixes, self.device, 2)
        divergence, drift = self.measure()
        self.assertEqual(divergence, plain)
        self.assertTrue(all(drift.mean(kind) > 0 for kind in X.KINDS))

    def update(self, net, lr, target, source, **options):
        optimizer = torch.optim.AdamW(net.parameters(), lr=lr, weight_decay=0)
        before = copy.deepcopy(optimizer.state_dict())
        record = transactional_update(net, optimizer, self.data, [1], self.prefixes, self.device, target_kl=target,
                                      audit_batch_size=2, source=source, **options)
        return record, optimizer, before

    def test_an_accepted_update_is_bit_identical_with_and_without_the_gate(self):
        with_gate, without = copy.deepcopy(self.net), copy.deepcopy(self.net)
        gated, _, _ = self.update(with_gate, 1e-4, 1., self.source)
        plain, _, _ = self.update(without, 1e-4, 1., None)
        self.assertTrue(gated['accepted'] and plain['accepted'])
        assert_state_equal(self, with_gate.state_dict(), without.state_dict())
        self.assertNotIn('uncertified_drift', plain)
        self.assertEqual({k: gated[k] for k in plain}, plain)
        self.assertEqual({kind: cell['decisions'] for kind, cell in gated['uncertified_drift'].items()},
                         dict(single=1, subset=1, pair=1, scalar=1))

    def test_one_head_beyond_the_gate_restores_weights_and_optimizer(self):
        # The certified rows stay within the target; the block's source readout says the pair
        # head is far from where it was.
        for kind, key in (('subset', (0, 2)), ('pair', (1, 2)), ('single', (3, 2))):
            with self.subTest(kind=kind):
                net, source = copy.deepcopy(self.net), copy.deepcopy(self.source)
                source.rows[key] = source.rows[key] + torch.arange(len(source.rows[key])) * 9.
                weights = copy.deepcopy(net.state_dict())
                record, optimizer, before = self.update(net, 1e-4, TARGET, source)
                self.assertFalse(record['accepted'])
                self.assertLessEqual(record['collection_behavior_kl'], TARGET)
                self.assertEqual(record['reason'], f'uncertified_drift_rejected: {kind}')
                self.assertGreater(record['uncertified_drift'][kind]['mean_kl'], TARGET)
                assert_state_equal(self, net.state_dict(), weights)
                assert_state_equal(self, optimizer.state_dict(), before)

    def test_a_number_alone_is_reported_and_never_refuses(self):
        source = copy.deepcopy(self.source)
        source.rows[(2, 2)] = source.rows[(2, 2)] + 5.
        record, _, _ = self.update(copy.deepcopy(self.net), 1e-4, 1., source)
        self.assertTrue(record['accepted'])
        self.assertAlmostEqual(record['uncertified_drift']['scalar']['mean_absolute'], 5., delta=.1)

    def test_the_behavior_gate_still_speaks_first(self):
        record, _, _ = self.update(copy.deepcopy(self.net), 1., 1e-12, self.source)
        self.assertEqual((record['accepted'], record['reason']), (False, 'collection_behavior_kl_rejected'))

    def test_the_head_only_replay_reads_the_same_rows_from_the_stored_memory(self):
        kept = X.SourceReadouts()
        observe_outside(self.net, self.data, (0, 1, 2, 3), self.device, 0, kept.observe)
        self.assertEqual(set(kept.rows), set(self.source.rows))
        for indices in ((0, 1, 2, 3), (2,), (3, 1)):      # any grouping of segments
            drift = X.Drift(kept)
            observe_outside(self.net, self.data, indices, self.device, 0, drift.observe)
            self.assertEqual(sum(drift.counts.values()), len(indices))
            self.assertTrue(all(drift.mean(k) is None or drift.mean(k) < 1e-6 for k in X.KINDS))
        with torch.no_grad():
            self.net.cand_scorer[0].weight.mul_(1.5)       # what the head-only worker trains
        drift = X.Drift(kept)
        observe_outside(self.net, self.data, (0, 1, 2, 3), self.device, 0, drift.observe)
        self.assertGreater(drift.mean('subset'), 1e-6)
        self.assertGreater(drift.mean('single'), 1e-6)
        self.assertEqual(drift.mean('pair'), 0.)
        self.assertTrue(all(p.grad is None for p in self.net.parameters()))

    def test_a_head_only_step_rolls_back_on_the_drift_alone(self):
        class Fixed:
            def __init__(self, refused):
                self.kinds = refused

            def report(self):
                return {'subset': {'decisions': 1}}

            def refused(self, limit):
                return self.kinds if limit == TARGET else []

            def mean(self, kind):
                return .5
        for refused in ([], ['subset']):
            p = torch.nn.Parameter(torch.ones(1))
            optimizer = torch.optim.AdamW([p], lr=.1, weight_decay=0)
            before, seen = copy.deepcopy(optimizer.state_dict()), []

            def drift(microbatches):
                seen.append(microbatches)
                return Fixed(refused)
            row = transactional_step([p], optimizer, [(1, 2), (3,)], 128, lambda _: -p.sum() * 128,
                                     lambda _: torch.tensor(0.), TARGET, drift)
            self.assertEqual(seen, [[(1, 2), (3,)]])
            self.assertEqual(row['accepted'], not refused)
            if refused:
                self.assertEqual(row['reason'], 'post_step_uncertified_drift: subset=0.5')
                self.assertEqual(p.item(), 1.)
                assert_state_equal(self, optimizer.state_dict(), before)
            else:
                self.assertEqual(row['uncertified_drift'], {'subset': {'decisions': 1}})
                self.assertFalse(math.isclose(p.item(), 1.))


if __name__ == '__main__':
    unittest.main()
