"""Candidate revision 3: exact transfer, learning, execution-path and export parity."""
import copy
import unittest

import torch
from torch.nn import functional as F
import model5 as V5
from export_onnx4 import make_dummy_inputs
import test_deferred4 as deferred_tests
import test_export_models as export_tests
from training4.deferred import _score_cand_rows
from training4.device_loss import compute_unroll_device_loss


class CandidateDecoderTests(unittest.TestCase):
    def setUp(self):
        torch.set_num_threads(1)
        torch.manual_seed(2718)

    def net(self, revision=3, **overrides):
        return V5.PlayerNet5(V5.Config5(d_model=32, n_layers=1, n_heads=4, d_ff=64,
                                        structure_width=16, relational_width=16, relational_revision=2,
                                        candidate_context_revision=revision, **overrides))

    def randomize_decoder(self, net, std=.1):
        with torch.no_grad():
            for name, parameter in net.named_parameters():
                if name.startswith(("cand_decoder.", "cand_context_gate.")):
                    parameter.normal_(std=std)

    def test_transfer_reproduces_source_exactly_at_every_depth(self):
        for source_revision in (1, 2):
            with self.subTest(source=source_revision):
                torch.manual_seed(19); old = self.net(source_revision).eval(); rng = torch.get_rng_state()
                torch.manual_seed(19); new = self.net().eval()
                self.assertTrue(torch.equal(rng, torch.get_rng_state()))
                new.load_state_dict(V5.warmstart_candidate_context(old.state_dict(), new), strict=True)
                inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=6, P=2))
                with torch.no_grad():
                    for k in (0, 1, 2, 4, 8):
                        lh = torch.zeros_like(inputs[-2]); rh = lh.clone()
                        for current in (0, k, 0, k):
                            inputs[-1].fill_(current)
                            inputs[-2] = lh; left = old(*inputs)
                            inputs[-2] = rh; right = new(*inputs)
                            for a, b in zip(left, right):
                                self.assertTrue(torch.equal(a, b))
                            lh, rh = left[0], right[0]
        self.assertEqual(V5.architecture_for_config(new.cfg), V5.DECODER_ARCHITECTURE)
        with self.assertRaises(ValueError):
            V5.warmstart_candidate_context(new.state_dict(), new)

    def test_zero_outputs_learn_first_and_decoded_scores_read_the_board(self):
        net = self.net()
        inputs = list(make_dummy_inputs(32, E=5, M_edges=3, C=4, P=2))
        inputs[-1].fill_(2)
        logits = net(*inputs)[1]
        F.cross_entropy(logits, torch.tensor([1])).backward()
        layer = net.cand_decoder[0]
        self.assertGreater(layer.attn.out_proj.weight.grad.abs().max().item(), 1e-6)
        self.assertGreater(layer.ff[2].weight.grad.abs().max().item(), 1e-6)
        self.assertEqual(layer.attn.q_proj.weight.grad.abs().max().item(), 0.)
        # Once learned, a candidate's score depends on an entity it does not name.
        self.randomize_decoder(net)
        net.eval()
        with torch.no_grad():
            inputs[9].fill_(-1)
            before = net(*inputs)[1]
            inputs[0][:, 4] += 3.
            after = net(*inputs)[1]
        self.assertGreater((before - after).abs().max().item(), 1e-5)

    def test_large_menus_score_in_exact_slices_with_checkpointing(self):
        net = self.net()
        self.randomize_decoder(net)
        e_ref, z_out = torch.randn(1, 3, 32), torch.randn(1, 9, 32)
        board = torch.cat([torch.randn(1, 9, 32), e_ref], dim=1)
        pad = torch.zeros(1, 12, dtype=torch.bool); pad[:, 5] = True
        keys = net.candidate_decoder_keys(z_out, board, pad)
        for count in (511, 513, 1025):
            with self.subTest(candidates=count):
                inputs = make_dummy_inputs(32, E=3, M_edges=2, C=count, P=2)
                args = [inputs[i] for i in (8, 14, 16, 9, 10, 11)] + [e_ref, z_out[:, 1:], z_out[:, 0]]
                args[5][:, -1] = False
                other = copy.deepcopy(net)
                left = _score_cand_rows(net, *args, keys)
                right = _score_cand_rows(other, *args, keys, use_ckpt=True)
                torch.testing.assert_close(left, right, atol=0, rtol=0)
                whole = net.candidate_hidden(*args[:5], *args[6:], keys)
                single = torch.cat([net.candidate_hidden(*[a[:, :1] if i < 5 else a for i, a in enumerate(args[:5] + args[6:])], keys),
                                    net.candidate_hidden(*[a[:, 1:] if i < 5 else a for i, a in enumerate(args[:5] + args[6:])], keys)], dim=1)
                torch.testing.assert_close(whole, single, atol=1e-6, rtol=1e-5)

    def test_reference_and_deferred_match_outputs_losses_and_gradients(self):
        runner = deferred_tests.DeferredReadouts()
        for k in (1, 2, 4):
            for prune in (False, True):
                with self.subTest(k=k, prune=prune):
                    batch = runner.fixture(); net = self.net(activation_checkpointing=True)
                    self.randomize_decoder(net, std=.05)
                    net.prune_unused_policy_heads = prune; other = copy.deepcopy(net)
                    left, lh, plans = runner.outputs(net, batch, 0, k)
                    right, rh, _ = runner.outputs(other, batch, 3, k)
                    torch.testing.assert_close(lh, rh, atol=0, rtol=0)
                    for t, (a, b) in enumerate(zip(left, right)):
                        for head, (x, y) in enumerate(zip(a, b)):
                            if prune and head in (1, 2, 3):
                                needed = (plans[t].heads[0] or plans[t].heads[1], plans[t].heads[2], plans[t].heads[3])[head - 1]
                                if not needed: continue
                            torch.testing.assert_close(x, y, atol=3e-6, rtol=3e-5)
                    losses = []
                    for model, outputs in ((net, left), (other, right)):
                        aux = model.aux_head(torch.stack([out[0][:, 0] for out in outputs], 1))
                        loss, _ = compute_unroll_device_loss(outputs, aux, batch, plans)
                        losses.append(loss.detach()); loss.backward()
                    torch.testing.assert_close(*losses, atol=1e-6, rtol=1e-5)
                    for (name, a), (_, b) in zip(net.named_parameters(), other.named_parameters()):
                        self.assertEqual(a.grad is None, b.grad is None, name)
                        if a.grad is not None:
                            torch.testing.assert_close(a.grad, b.grad, atol=3e-6, rtol=3e-5, msg=name)
                    self.assertGreater(net.cand_decoder[0].attn.q_proj.weight.grad.abs().max().item(), 0.)

    def test_learned_decoder_cpu_onnx_all_depths(self):
        export_tests.ExportModels.assert_parity(self, V5, V5.Config5, V5.PlayerNet5,
                                                relational_revision=2, candidate_context_revision=3)

    def test_contract_rejects_unknown_shapes(self):
        for bad in (dict(candidate_decoder_layers=0), dict(candidate_decoder_layers=5),
                    dict(relational_revision=1), dict(global_feat_width=82)):
            with self.subTest(**bad), self.assertRaises(ValueError):
                V5.Config5(candidate_context_revision=3, **{"relational_revision": 2, **bad})


if __name__ == "__main__":
    unittest.main()
