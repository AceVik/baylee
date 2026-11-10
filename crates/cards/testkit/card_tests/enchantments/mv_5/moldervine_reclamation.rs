//! `cards/enchantments/mv_5/moldervine_reclamation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn moldervine_reclamation() -> CardIndex {
    card_index("68639a3d-2192-4921-8298-c76bb0cd6b02")
}

/// "Whenever a creature you control dies, you gain 1 life and draw a
/// card." Mine dying pays; the opponent's does not.
#[test]
fn moldervine_reclamation_gains_life_and_draws_when_my_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[moldervine_reclamation(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let (hand, life) = (hand_len(&engine, p0), engine.state().players[0].life);
    kill(&mut engine, theirs);
    assert_eq!(engine.state().players[0].life, life, "not my creature");
    assert_eq!(hand_len(&engine, p0), hand);

    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    kill(&mut engine, mine);
    assert_eq!(engine.state().players[0].life, life + 1);
    assert_eq!(hand_len(&engine, p0), hand + 1);
}
