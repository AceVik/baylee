import unittest

from summarize_eval4 import summarize


class EvaluationUncertainty(unittest.TestCase):
    def test_rotations_do_not_multiply_independent_sample_size(self):
        rows = [{"table": i, "seed": i, "decks": ["a", "b"],
                 "net_utility": float(i % 2), "base_utility": 0.5,
                 "net_capped": False, "base_capped": False} for i in range(8)]
        a = summarize({"deal_details": rows}, resamples=100)
        b = summarize({"deal_details": rows * 8}, resamples=100)
        self.assertEqual(a["independent_tables"], b["independent_tables"])
        self.assertEqual(a["bounded_independent_table_95_percent_interval"], b["bounded_independent_table_95_percent_interval"])
        self.assertEqual(a["table_bootstrap_95_percent_interval"], b["table_bootstrap_95_percent_interval"])

    def test_answer_cap_is_not_a_known_draw(self):
        row = {"table": 0, "seed": 1, "decks": ["a", "b"],
               "net_utility": 0.5, "base_utility": 1.0,
               "net_capped": True, "base_capped": False}
        a = summarize({"deal_details": [row]}, resamples=100)
        self.assertIsNone(a["paired_utility_difference"])
        self.assertEqual(a["censoring_bounds_on_observed_tables"], [-1., 0.])
        self.assertEqual(a["table_bootstrap_95_percent_interval"], [-1., 0.])


if __name__ == "__main__":
    unittest.main()
