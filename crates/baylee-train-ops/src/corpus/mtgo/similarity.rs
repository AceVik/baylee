//! Exact inverted multiset similarity for complete-deck duplicate rejection.
use std::collections::BTreeMap;
/// Exact multiset Jaccard using complete inverted postings, without approximate misses.
#[derive(Default)]
pub struct SimilarityIndex {
    postings: BTreeMap<u64, Vec<(usize, u64)>>,
    sizes: Vec<u64>,
}
impl SimilarityIndex {
    /// Include an admitted multiset in subsequent duplicate comparisons.
    pub fn add(&mut self, cards: &BTreeMap<u64, u64>) {
        let id = self.sizes.len();
        self.sizes.push(cards.values().sum());
        for (&card, &count) in cards {
            self.postings.entry(card).or_default().push((id, count));
        }
    }
    /// Largest exact multiset intersection/union ratio among all prior decks.
    #[allow(clippy::cast_precision_loss)] // Deck cardinalities are at most a few hundred.
    pub fn maximum(&self, cards: &BTreeMap<u64, u64>) -> f64 {
        let mut intersections = BTreeMap::new();
        for (card, count) in cards {
            if let Some(postings) = self.postings.get(card) {
                for (id, other) in postings {
                    *intersections.entry(*id).or_insert(0u64) += (*count).min(*other);
                }
            }
        }
        let size: u64 = cards.values().sum();
        intersections
            .into_iter()
            .map(|(id, n)| n as f64 / (size + self.sizes[id] - n) as f64)
            .fold(0.0, f64::max)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn inverted_index_matches_exhaustive_multiset_jaccard() {
        let mut state = 91u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            state
        };
        let mut index = SimilarityIndex::default();
        let mut previous: Vec<BTreeMap<u64, u64>> = Vec::new();
        for _ in 0..100 {
            let size = 40 + next() % 41;
            let mut cards = BTreeMap::new();
            for _ in 0..size {
                *cards.entry(next() % 30).or_insert(0) += 1;
            }
            let expected = previous
                .iter()
                .map(|other| {
                    let n: u64 = cards
                        .iter()
                        .map(|(card, count)| (*count).min(*other.get(card).unwrap_or(&0)))
                        .sum();
                    n as f64 / (size + other.values().sum::<u64>() - n) as f64
                })
                .fold(0.0, f64::max);
            assert_eq!(index.maximum(&cards).to_bits(), expected.to_bits());
            index.add(&cards);
            previous.push(cards);
        }
    }
}
