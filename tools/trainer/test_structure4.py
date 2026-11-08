"""Semantic counterexamples and complete neural/export gradient contract."""
import copy
import unittest
import torch
from model4.structure import RuleEncoder


def small_forest():
    return {"version": 1, "dsl_source_sha256": "test", "roots": {},
            "tags": ["Unknown", "Creature", "Not", "Sequence", "u32"],
            "nodes": [
                {"tag": 0, "bytes": [], "children": []},
                {"tag": 1, "bytes": [], "children": []},
                {"tag": 2, "bytes": [], "children": [1]},
                {"tag": 3, "bytes": [], "children": [1, 2]},
                {"tag": 3, "bytes": [], "children": [2, 1]},
                {"tag": 4, "bytes": [12, 0, 0, 0], "children": []},
                {"tag": 4, "bytes": [13, 0, 0, 0], "children": []},
            ]}


class StructureTests(unittest.TestCase):
    def test_negation_order_and_uncapped_numbers_reach_neural_output(self):
        torch.manual_seed(7)
        net = RuleEncoder(small_forest(), 16).eval()
        table = net.encode_catalog()
        self.assertEqual(float(table[0].detach().abs().sum()), 0)
        for a, b in [(1, 2), (3, 4), (5, 6)]:
            self.assertGreater(float((table[a] - table[b]).detach().abs().max()), 1e-4)
        table[3].square().sum().backward()
        self.assertGreater(float(net.symbol.weight.grad[1].abs().sum()), 0)

    def test_shared_update_table_has_same_gradients_and_frozen_predictions(self):
        torch.manual_seed(8)
        a = RuleEncoder(small_forest(), 16).eval()
        b = copy.deepcopy(a)
        refs = torch.tensor([[[[3, 1], [5, 7]], [[4, 1], [0, 0]]]])
        expected = a(refs)
        (expected.square().sum() + a(refs).sum()).backward()
        b.prepare()
        actual = b(refs)
        (actual.square().sum() + b(refs).sum()).backward()
        torch.testing.assert_close(actual, expected)
        for p, q in zip(a.parameters(), b.parameters()):
            if p.grad is not None: torch.testing.assert_close(p.grad, q.grad)
        torch.testing.assert_close(a.freeze()(refs), expected)
        b.clear()
        self.assertIsNone(b._table)

    def test_reject_forward_references(self):
        data = small_forest()
        data["nodes"][1]["children"] = [2]
        with self.assertRaisesRegex(ValueError, "topological"):
            RuleEncoder(data, 16)

    def test_catalog_refresh_maps_symbols_by_name_and_rebuilds_buffers(self):
        from refresh_structure4 import refresh_encoder
        old = RuleEncoder(small_forest(), 16).eval()
        data = copy.deepcopy(old.data)
        data["tags"].reverse()
        for node in data["nodes"]:
            node["tag"] = len(data["tags"]) - 1 - node["tag"]
        # Insert a new combination using an existing operator. Old DAG row
        # numbers may shift; shared subtrees must still retain their meaning.
        data["nodes"].insert(3, {"tag": data["tags"].index("Not"), "bytes": [], "children": [2]})
        refreshed = refresh_encoder(old, data)
        actual = refreshed.encode_catalog()
        original = old.encode_catalog()
        torch.testing.assert_close(actual[[0, 1, 2, 4, 5, 6, 7]], original)
        self.assertNotEqual(refreshed.digest, old.digest)
        self.assertGreater(float((actual[3] - actual[2]).detach().abs().max()), 1e-4)
        data["tags"].append("FutureRule")
        with self.assertRaisesRegex(ValueError, "require training"):
            refresh_encoder(old, data)

    def test_actor_and_onnx_use_nonzero_semantic_references(self):
        import tempfile
        from pathlib import Path
        import numpy as np
        import onnxruntime as ort
        import model4 as M
        from export_onnx4 import make_dummy_inputs, export_v4_onnx
        torch.manual_seed(9)
        net = M.PlayerNet4(M.Config4(d_model=32, n_layers=1, n_heads=4, d_ff=64, structure_width=16)).eval()
        net.attach_structure(small_forest()); net.eval()
        values = list(make_dummy_inputs(32, E=2, M_edges=1, C=2, P=1, K=1))
        values[M.INPUT_NAMES.index("cand_feat")].zero_()
        values[M.INPUT_NAMES.index("cand_entity_idx")].fill_(-1)
        values[M.INPUT_NAMES.index("entity_rules")][..., 0, :] = torch.tensor([3, 1])
        values[M.INPUT_NAMES.index("cand_rules")][0, :, 0, :] = torch.tensor([[3, 7], [4, 7]])
        with torch.no_grad(): expected = net(*values)
        self.assertGreater(float((expected[1][0, 0] - expected[1][0, 1]).abs()), 1e-6)
        net.rules = net.rules.freeze()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "model.onnx"
            export_v4_onnx(net, path, thinking_steps=1, fixed_depth=True)
            options = ort.SessionOptions(); options.intra_op_num_threads = 1
            session = ort.InferenceSession(str(path), options, providers=["CPUExecutionProvider"])
            actual = session.run(list(M.OUTPUT_NAMES), {n: t.numpy() for n, t in zip(M.INPUT_NAMES, values)})
            for a, b in zip(actual, expected): np.testing.assert_allclose(a, b.numpy(), atol=1e-5, rtol=1e-5)
            # Different reference count is a real dynamic ONNX axis.
            for name in ("entity_rules", "cand_rules"):
                i = M.INPUT_NAMES.index(name)
                values[i] = torch.cat((values[i], torch.zeros_like(values[i])), 2)
            actual = session.run(list(M.OUTPUT_NAMES), {n: t.numpy() for n, t in zip(M.INPUT_NAMES, values)})
            for a, b in zip(actual, expected): np.testing.assert_allclose(a, b.numpy(), atol=1e-5, rtol=1e-5)

    def test_active_subgraph_encoding_matches_full_catalog_outputs_and_gradients(self):
        torch.manual_seed(11)
        forest = small_forest()
        # Pad with unreferenced nodes so active subgraph is a strict subset.
        for i in range(16):
            forest["nodes"].append({"tag": (i % 4) + 1, "bytes": [i, 1], "children": [1, 2]})
        a = RuleEncoder(forest, 16).eval()
        b = copy.deepcopy(a)
        refs = torch.tensor([[[[3, 1], [5, 7]], [[4, 1], [0, 0]]]])
        a.prepare()
        expected = a(refs)
        expected.square().mean().backward()
        b._table = b.encode_active((refs,))
        role_idx = torch.arange(8)
        b._gate_w = torch.sigmoid(b.role_gate(role_idx))
        b._role_w = b.role(role_idx)
        actual = b(refs)
        actual.square().mean().backward()
        torch.testing.assert_close(actual, expected, atol=1e-6, rtol=1e-6)
        for (name, p), (_, q) in zip(a.named_parameters(), b.named_parameters()):
            self.assertEqual(p.grad is None, q.grad is None, name)
            if p.grad is not None:
                torch.testing.assert_close(p.grad, q.grad, atol=1e-6, rtol=1e-6, msg=name)

    def test_blind_card_names_change_only_rows_that_read_a_name(self):
        from model4.structure import without_card_names
        from refresh_structure4 import refresh_encoder
        forest = {"version": 1, "dsl_source_sha256": "test", "roots": {},
                  "tags": ["Unknown", "String", "FaceDef.name", "CardFace", "Filter::Named", "u32"],
                  "nodes": [
                      {"tag": 0, "bytes": [], "children": []},
                      {"tag": 1, "bytes": list(b"Bolt"), "children": []},
                      {"tag": 2, "bytes": [], "children": [1]},
                      {"tag": 1, "bytes": list(b"Cat"), "children": []},
                      {"tag": 2, "bytes": [], "children": [3]},
                      {"tag": 5, "bytes": [3, 0, 0, 0], "children": []},
                      {"tag": 3, "bytes": [], "children": [2, 5]},
                      {"tag": 3, "bytes": [], "children": [4, 5]},
                      {"tag": 1, "bytes": list(b"Goblin"), "children": []},
                      {"tag": 4, "bytes": [], "children": [8]},
                      {"tag": 4, "bytes": [], "children": [1]},
                  ]}
        blind, emptied = without_card_names(forest)
        self.assertEqual(emptied, 2)
        self.assertEqual([n["bytes"] for n in blind["nodes"]][1:4], [[], [], []])
        self.assertEqual(blind["nodes"][8]["bytes"], list(b"Goblin"))
        self.assertEqual(forest["nodes"][1]["bytes"], list(b"Bolt"), "the source stays")
        self.assertEqual([(n["tag"], n["children"]) for n in blind["nodes"]],
                         [(n["tag"], n["children"]) for n in forest["nodes"]])
        torch.manual_seed(12)
        net = RuleEncoder(forest, 16).eval()
        seen = net.freeze().table
        blinded = refresh_encoder(net, blind).eval().freeze().table
        # Rows that read no name are the model's own, bit for bit.
        for row in (0, 5, 8, 9):
            torch.testing.assert_close(blinded[row], seen[row], atol=0, rtol=0)
        # Every name now reads the same (up to the order of float sums); a
        # rule naming a card loses it too.
        for a, b in ((1, 3), (2, 4), (6, 7)):
            self.assertGreater(float((seen[a] - seen[b]).abs().max()), 1e-4)
            torch.testing.assert_close(blinded[a], blinded[b], atol=1e-6, rtol=1e-5)
        self.assertGreater(float((seen[10] - blinded[10]).abs().max()), 1e-4)
        broken = copy.deepcopy(forest)
        broken["nodes"][2]["children"] = [1, 3]
        with self.assertRaisesRegex(ValueError, "one string child"):
            without_card_names(broken)


if __name__ == "__main__": unittest.main()
