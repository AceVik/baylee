//! The supported creature body is played; unsupported abilities are omitted.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn soltari_visionary() -> CardIndex {
    card_index("7971f5c2-97b7-441f-ad77-954f43ede0a1")
}

#[test]
fn soltari_visionary_keeps_its_body_without_an_inaccurate_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soltari_visionary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let body = on_battlefield(&engine, p0, soltari_visionary()).unwrap();
    lk::float(
        &mut engine,
        p0,
        &[
            (ManaColor::Black, 10),
            (ManaColor::Blue, 10),
            (ManaColor::Green, 10),
            (ManaColor::White, 10),
            (ManaColor::Red, 10),
        ],
    );
    assert!(
        kit::offered(&engine, body).is_empty(),
        "unsupported activations are unavailable"
    );
    kit::declare_attack(&mut engine, &[body], p1);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 20 - 2);
    assert!(on_battlefield(&engine, p0, soltari_visionary()).is_some());
}

#[test]
fn shadow_limits_both_blocking_and_being_blocked() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = card_index("7971f5c2-97b7-441f-ad77-954f43ede0a1");
    let mut e = lk::game(
        SEED,
        &[subject, gray_ogre()],
        &[subject, grizzly_bears()],
        &[],
        &[],
    );
    let shadow = lk::obj(&e, p0, subject);
    let normal = lk::obj(&e, p0, gray_ogre());
    let enemy_shadow = lk::obj(&e, p1, subject);
    let enemy_normal = lk::obj(&e, p1, grizzly_bears());
    let offer = lk::attack(&mut e, &[shadow, normal]);
    assert!(lk::may_block(&offer, enemy_shadow, shadow));
    assert!(lk::may_block(&offer, enemy_normal, normal));
    assert!(!lk::may_block(&offer, enemy_normal, shadow));
    assert!(!lk::may_block(&offer, enemy_shadow, normal));
}
