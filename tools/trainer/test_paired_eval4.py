import copy
import unittest

from paired_eval4 import compare


def fixture(utilities):
    return {"against": "sharp", "baseline": "steady", "seats": 2, "shape": "constructed", "teams": [],
            "deck_split": "val", "held_out_cards": False, "first_seed": 1, "deal_offset": 0,
            "deals": len(utilities), "policy_seed": 17, "temperature": 0.0, "thinking_steps": 1,
            "model": "test", "deal_details": [
                {"table": i//2, "seed": i//2+1, "focal_seat": i%2, "decks": ["a", "b"],
                 "net_utility": utility, "net_capped": False, "base_utility": 0.0, "base_capped": False}
                for i, utility in enumerate(utilities)]}


class PairedModelEvaluation(unittest.TestCase):
    provenance = {"engine_sha256": "engine", "deck_manifest_sha256": "decks", "max_decisions": 5000,
                  "fair_scouting": True, "holdout_tier": "", "generated": 0}

    def compare(self, a, b):
        return compare(a, b, self.provenance, self.provenance, resamples=100)

    def test_bad_deals_stay_and_wins_are_compared_pairwise(self):
        a, b = fixture([0, 1, 1, 0]), fixture([0, 0, 1, 1])
        b["deal_details"].reverse()  # Worker completion order cannot matter.
        result = self.compare(a, b)
        self.assertEqual(result["outcomes"], {"better": 1, "worse": 1, "equal": 2,
                                              "both_lose": 1, "both_win": 1, "unresolved_cap": 0})
        self.assertEqual(result["paired_uncertainty"]["deals"], 4)
        self.assertEqual(result["paired_uncertainty"]["independent_tables"], 2)
        self.assertEqual(result["paired_uncertainty"]["paired_utility_difference"], 0)
        a["deal_details"].reverse()
        self.assertEqual(self.compare(a, b), result)

    def test_caps_are_unknown_and_not_an_improvement(self):
        a, b = fixture([0.5, 1]), fixture([0, 1])
        a["deal_details"][0]["net_capped"] = True
        result = self.compare(a, b)
        self.assertEqual(result["outcomes"]["unresolved_cap"], 1)
        self.assertEqual(result["outcomes"]["better"], 0)
        self.assertIsNone(result["paired_uncertainty"]["paired_utility_difference"])

    def test_rejects_different_missing_duplicate_or_unreproduced_cases(self):
        original = fixture([0, 1])
        mutations = [lambda b: b.update(against="casual"),
                     lambda b: b["deal_details"][0].update(seed=999),
                     lambda b: b["deal_details"][0].update(decks=["a", "c"]),
                     lambda b: b["deal_details"][0].update(focal_seat=1),
                     lambda b: b["deal_details"].pop(),
                     lambda b: b["deal_details"].append(b["deal_details"][0]),
                     lambda b: b["deal_details"][0].update(base_utility=1)]
        for mutate in mutations:
            b = copy.deepcopy(original)
            mutate(b)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                self.compare(original, b)
        with self.assertRaises(ValueError):
            compare(original, original, self.provenance, {"engine_sha256": "other"})


if __name__ == "__main__":
    unittest.main()
