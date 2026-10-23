//! `cards/creatures/mv_4/mirozel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mirozel() -> CardIndex {
    card_index("c35111ce-4325-4547-b954-08275a268060")
}

#[test]
fn mirozel_returns_to_hand_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mirozel(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let illusion = on_battlefield(&engine, p0, mirozel()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[illusion], &[])
    });

    assert!(on_battlefield(&engine, p0, mirozel()).is_none());
    assert!(in_hand(&engine, p0, mirozel()).is_some());
}

#[test]
fn mirozel_flies_over_ground_blockers_and_returns_for_opponents_targets() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[mirozel()],
        &[gray_ogre(), index::WIND_DRAKE],
        &[],
        &[lightning_bolt()],
    );
    let illusion = lk::obj(&e, p0, mirozel());
    let ground = lk::obj(&e, p1, gray_ogre());
    let flyer = lk::obj(&e, p1, index::WIND_DRAKE);
    let offer = lk::attack(&mut e, &[illusion]);
    assert!(!lk::may_block(&offer, ground, illusion));
    assert!(lk::may_block(&offer, flyer, illusion));
    lk::block(&mut e, &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p1, lightning_bolt(), &[illusion], &[]);
    assert!(in_hand(&e, p0, mirozel()).is_some());
    assert!(in_graveyard(&e, p0, mirozel()).is_none());
}
