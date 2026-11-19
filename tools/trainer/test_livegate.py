"""The live-search curve's gate scores (`training4.livegate`)."""
import unittest

import numpy as np

from training4 import advantage as V
from training4 import livegate as G


def rows_of():
    """Two roots: the first's own answer is item 1 of three, item 2 unsearched; the second's is
    item 0 of two."""
    n = np.array([3, 2])
    rows = V.AdvantageRows(
        dict(game=np.array([4, 4]), seat=np.zeros(2, dtype=np.int64), cursor=np.array([10, 20]),
             kind=np.zeros(2, dtype=np.int64), own=np.array([1, 0]), choice4=np.zeros(2, dtype=np.int64),
             half_a=np.zeros(2, dtype=np.int64), half_b=np.zeros(2, dtype=np.int64), offset=np.array([0, 3]), n=n),
        np.zeros((2, 2), dtype=np.float16),
        np.array([[1., 0.], [3., 0.], [9., 0.], [2., 0.], [0., 5.]], dtype=np.float16),
        dict(A=np.zeros(5, dtype=np.float32), SE=np.zeros(5, dtype=np.float32),
             searched=np.array([True, True, False, True, True]), payable=np.zeros(5, dtype=bool),
             cls=np.zeros(5, dtype=np.int64)),
        np.array([True, True]), ("pass",))
    return rows


class Gates(unittest.TestCase):
    def test_the_policy_margin_is_the_own_logit_less_the_best_other_searched(self):
        rows = rows_of()
        weight, bias = np.array([[1., 2.]]), np.array([.5])
        margins = G.policy_margins(rows, weight, bias)
        # Root 0: logits 1.5, 3.5, (9.5 unsearched): own 3.5 - 1.5. Root 1: own 2.5, other 10.5.
        np.testing.assert_allclose(margins, [2., -8.])

    def test_a_head_gate_is_the_largest_gap_of_another_searched_candidate(self):
        rows = rows_of()
        gaps = np.array([-.3, 0., 9., 0., .1])
        np.testing.assert_allclose(G.per_root_max_abs(rows, gaps), [.3, .1])


if __name__ == "__main__":
    unittest.main()
