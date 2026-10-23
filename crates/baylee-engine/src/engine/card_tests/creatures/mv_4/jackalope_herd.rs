//! `cards/creatures/mv_4/jackalope_herd.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn jackalope_herd() -> CardIndex {
    card_index("bcc8ce61-a881-42be-ab2c-64351d17f9f8")
}

#[test]
fn jackalope_herd_returns_to_hand_when_you_cast_a_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[jackalope_herd(), mountain(), mountain(), mountain()])
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, jackalope_herd()).is_none());
    assert!(in_hand(&engine, p0, jackalope_herd()).is_some());
}
