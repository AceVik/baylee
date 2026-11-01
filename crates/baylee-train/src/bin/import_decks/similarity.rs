pub(super) use baylee_train::deckgen::strategy::{
    compute_deck_hash, compute_multiset, multiset_jaccard,
};

pub(super) fn find(p: &mut [usize], mut x: usize) -> usize {
    while p[x] != x {
        p[x] = p[p[x]];
        x = p[x];
    }
    x
}
