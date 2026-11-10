//! `cards/creatures/mv_7/trostani_s_summoner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trostani_s_summoner() -> CardIndex {
    card_index("874e0b54-bc24-4887-abe1-1ecfd3a3abae")
}

/// Trostani's Summoner — a 1/1 for {5}{G}{W} that makes a 2/2 white Knight
/// with vigilance, a 3/3 green Centaur and a 4/4 green Rhino with trample.
#[test]
fn trostani_s_summoner_makes_three_tokens() {
    let p0 = PlayerId::new(0);
    let board = [lots(forest(), 6), vec![plains()]].concat();
    let mut engine = table7(&board, &[trostani_s_summoner()], &[]);
    let summoner = cast_and_resolve(&mut engine, trostani_s_summoner());
    assert_eq!(pt(&engine, summoner), (1, 1));
    let mut tokens: Vec<_> = tokens_of(&engine, p0)
        .into_iter()
        .map(|t| (pt(&engine, t), keywords(&engine, t), t))
        .collect();
    tokens.sort_by_key(|(body, _, _)| *body);
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].0, (2, 2), "the Knight");
    assert!(tokens[0].1.contains(KeywordSet::VIGILANCE));
    assert_eq!(tokens[1].0, (3, 3), "the Centaur");
    assert_eq!(tokens[2].0, (4, 4), "the Rhino");
    assert!(tokens[2].1.contains(KeywordSet::TRAMPLE));
    assert!(!tokens[1].1.contains(KeywordSet::TRAMPLE));
}
