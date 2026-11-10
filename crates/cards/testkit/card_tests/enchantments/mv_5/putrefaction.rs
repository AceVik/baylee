//! `cards/enchantments/mv_5/putrefaction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn putrefaction() -> CardIndex {
    card_index("8c4545f8-10b6-4a62-9bce-8c9ffa8b8471")
}

/// "Whenever a player casts a green or white spell, that player discards a
/// card." I cast a green creature: I discard one of the two cards left in
/// hand.
#[test]
fn putrefaction_makes_me_discard_for_my_green_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[llanowar_elves(), plains(), swamp()])
        .battlefield(0, &[putrefaction(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    assert_eq!(hand_len(&engine, p0), 2, "the Elves left the hand");
    run_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, llanowar_elves()).is_some()
    });
    assert_eq!(hand_len(&engine, p0), 1, "one card discarded");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1
    );
}

/// A red spell is neither green nor white.
#[test]
fn putrefaction_ignores_a_red_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gray_ogre(), plains(), swamp()])
        .battlefield(0, &[putrefaction(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gray_ogre());
    run_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gray_ogre()).is_some()
    });
    assert_eq!(hand_len(&engine, p0), 2, "nothing discarded");
}

/// "…a player": the opponent casting a white spell discards too.
#[test]
fn putrefaction_makes_the_opponent_discard_for_their_white_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[putrefaction()])
        .hand(1, &[serra_angel(), swamp(), swamp()])
        .battlefield(1, &[plains(), plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let mine = hand_len(&engine, p0);
    cast_from_hand(&mut engine, p1, serra_angel());
    let after_cast = hand_len(&engine, p1);
    run_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p1, serra_angel()).is_some()
    });
    assert_eq!(hand_len(&engine, p1), after_cast - 1, "they discarded");
    assert_eq!(hand_len(&engine, p0), mine, "I did not");
}
