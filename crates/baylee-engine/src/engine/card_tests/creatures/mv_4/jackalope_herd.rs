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

#[test]
fn jackalope_herd_ignores_an_opponents_spell_but_returns_for_your_spell() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[jackalope_herd()],
        &[],
        &[lightning_bolt()],
        &[lightning_bolt()],
    );
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::pass_once(&mut e);
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p1, lightning_bolt(), &[], &[p0]);
    assert!(on_battlefield(&e, p0, jackalope_herd()).is_some());
    lk::cast_at(&mut e, p0, lightning_bolt(), &[], &[p1]);
    assert!(in_hand(&e, p0, jackalope_herd()).is_some());
}
