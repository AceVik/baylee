"""Exact deck-overlap checks must not leak duplicates between corpus splits."""
from collections import Counter
import random
import unittest

from import_mtgo4 import SimilarityIndex, read_cards


class DeckImportContracts(unittest.TestCase):
    def test_inverted_similarity_matches_exhaustive_multiset_comparison(self):
        rng = random.Random(91)
        index, previous = SimilarityIndex(), []
        for _ in range(100):
            cards = Counter(rng.randrange(30) for _ in range(rng.randrange(40, 81)))
            expected = max((sum((cards & other).values()) / sum((cards | other).values())
                            for other in previous), default=0.)
            self.assertEqual(index.maximum(cards), expected)
            previous.append(cards)
            index.add(cards)

    def test_a_missing_card_rejects_a_whole_mainboard(self):
        lookup = {"forest": {"id": 7, "name": "Forest", "held_out": False}}
        with self.assertRaises(KeyError):
            read_cards([{"Count": 20, "CardName": "Forest"},
                        {"Count": 4, "CardName": "unsupported"}], lookup)
        with self.assertRaises(ValueError):
            read_cards([{"Count": -1, "CardName": "Forest"}], lookup)


if __name__ == "__main__":
    unittest.main()
