//! `cards/enchantments/mv_6/forced_fruition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn forced_fruition() -> CardIndex {
    card_index("448b27a5-7c0c-4ab6-bddf-bd62b920aacc")
}

/// "Whenever an opponent casts a spell, that player draws seven cards."
/// The opponent's Ogre draws them seven; my own spell does not draw me any.
#[test]
fn forced_fruition_draws_the_opponent_seven() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forced_fruition(), mountain(), mountain(), mountain()])
        .hand(0, &[gray_ogre()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gray_ogre());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gray_ogre()).is_some()
    });
    assert_eq!(hand_len(&engine, p0), 0, "my own spell draws nothing");

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, gray_ogre());
    let after_cast = hand_len(&engine, p1);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p1, gray_ogre()).is_some()
    });
    assert_eq!(hand_len(&engine, p1), after_cast + 7);
}
