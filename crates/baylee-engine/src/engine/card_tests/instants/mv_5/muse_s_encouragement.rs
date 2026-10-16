//! `cards/instants/mv_5/muse_s_encouragement.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "62421084-96e6-40dd-8d34-7c8c1091d4a4"
fn muse_s_encouragement() -> CardIndex {
    card_index("62421084-96e6-40dd-8d34-7c8c1091d4a4")
}

/// Muse's Encouragement: "Create a 3/3 blue and red Elemental creature token
/// with flying. Surveil 2."
///
/// The token is read for its size and its flying, and the surveil is answered
/// by sending one of the two looked-at cards to the graveyard.
#[test]
fn muse_s_encouragement_makes_a_flier_and_surveils_two() {
    let mut engine = table(927, &[muse_s_encouragement()], &[island(); 5], &[], &[]);
    let library_before = library_len(&engine, me());
    cast_from_hand(&mut engine, me(), muse_s_encouragement());
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, me());
    assert_eq!(cards.len(), 2, "surveil 2 looks at two cards");
    let graveyard_before = graveyard_len(&engine, me());
    engine
        .apply(me(), look_answer(&cards, &cards[..1]))
        .expect("one to the graveyard, one stays");
    settle(&mut engine);

    let tokens = my_tokens(&engine);
    assert_eq!(tokens.len(), 1, "one token");
    assert_eq!(pt(&engine, tokens[0]), (3, 3));
    assert!(keywords_of(&engine, tokens[0]).contains(KeywordSet::FLYING), "flying");
    assert_eq!(graveyard_len(&engine, me()), graveyard_before + 2, "one surveilled away, and the spell itself");
    assert_eq!(library_len(&engine, me()), library_before - 1);
}
