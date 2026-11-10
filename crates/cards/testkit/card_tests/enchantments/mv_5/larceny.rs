//! `cards/enchantments/mv_5/larceny.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn larceny() -> CardIndex {
    card_index("f356087e-e1d2-4c25-aa1d-215d09535e7c")
}

/// "Whenever a creature you control deals combat damage to a player, that
/// player discards a card." The unblocked Ogre hits; the opponent, holding
/// two cards, discards one of them.
#[test]
fn larceny_makes_the_damaged_player_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[larceny(), gray_ogre()])
        .hand(1, &[forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    declare_attack(&mut engine, p0, &[ogre], p1);
    run_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18, "the Ogre connected");
    assert_eq!(hand_len(&engine, p1), 1, "one card discarded");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
}

/// "…a creature you control": the opponent's Ogre hitting me does not make
/// me discard.
#[test]
fn larceny_does_not_answer_the_opponents_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[larceny()])
        .hand(0, &[forest()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let before = hand_len(&engine, p0);
    declare_attack(&mut engine, p1, &[ogre], p0);
    run_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 18, "the Ogre connected");
    assert_eq!(hand_len(&engine, p0), before, "I discarded nothing");
}
